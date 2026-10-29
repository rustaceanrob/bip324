// SPDX-License-Identifier: CC0-1.0

const PORT: u16 = 18444;

#[test]
fn regtest_handshake() {
    use std::{
        io::{Read, Write},
        net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bip324::{
        GarbageResult, Handshake, Initialized, PacketType, ReceivedKey, VersionResult,
        NUM_LENGTH_BYTES,
    };
    use bitcoin::consensus;
    use p2p::{
        message::{NetworkMessage, V2NetworkMessage},
        message_network::{UserAgent, VersionMessage},
        Address, ProtocolVersion, ServiceFlags,
    };
    let bitcoind = regtest_process(TransportVersion::V2);

    let mut stream = TcpStream::connect(bitcoind.params.p2p_socket.unwrap()).unwrap();

    // Initialize handshake
    let handshake =
        Handshake::<Initialized>::new(p2p::Magic::REGTEST, bip324::Role::Initiator).unwrap();

    // Send our public key
    let mut public_key = vec![0u8; Handshake::<Initialized>::send_key_len(None)];
    let handshake = handshake.send_key(None, &mut public_key).unwrap();
    println!("Writing public key to the remote node");
    stream.write_all(&public_key).unwrap();
    stream.flush().unwrap();

    // Read remote public key
    let mut remote_public_key = [0u8; 64];
    println!("Reading the remote node public key");
    stream.read_exact(&mut remote_public_key).unwrap();

    // Process remote key
    let handshake = handshake.receive_key(remote_public_key).unwrap();

    // Send garbage terminator and version
    let mut local_garbage_terminator_message =
        vec![0u8; Handshake::<ReceivedKey>::send_version_len(None)];
    println!("Sending our garbage terminator");
    let handshake = handshake
        .send_version(&mut local_garbage_terminator_message, None)
        .unwrap();
    stream.write_all(&local_garbage_terminator_message).unwrap();
    stream.flush().unwrap();

    // Read and authenticate remote response
    let mut max_response = [0; 4096];
    println!("Reading the response buffer");
    let size = stream.read(&mut max_response).unwrap();
    let response = &max_response[..size];
    println!("Authenticating the handshake");

    // First receive garbage
    let (mut handshake, consumed) = match handshake.receive_garbage(response).unwrap() {
        GarbageResult::FoundGarbage {
            handshake,
            consumed_bytes,
        } => {
            println!("Found garbage terminator after {consumed_bytes} bytes");
            (handshake, consumed_bytes)
        }
        GarbageResult::NeedMoreData(_) => panic!("Should have found garbage"),
    };

    // Then receive version - properly handle packet length and potential decoys.
    let mut remaining = &response[consumed..];
    let cipher_session = loop {
        // Check if we have enough data for packet length
        if remaining.len() < NUM_LENGTH_BYTES {
            panic!("Not enough data for packet length");
        }

        let packet_len = handshake
            .decrypt_packet_len(remaining[..NUM_LENGTH_BYTES].try_into().unwrap())
            .unwrap();

        if remaining.len() < NUM_LENGTH_BYTES + packet_len {
            panic!("Not enough data for full packet");
        }

        let mut version_packet =
            remaining[NUM_LENGTH_BYTES..NUM_LENGTH_BYTES + packet_len].to_vec();

        match handshake.receive_version(&mut version_packet).unwrap() {
            VersionResult::Complete { cipher } => {
                println!("Finalizing the handshake");
                break cipher;
            }
            VersionResult::Decoy(h) => {
                println!("Received decoy packet, continuing...");
                handshake = h;
                remaining = &remaining[NUM_LENGTH_BYTES + packet_len..];
                continue;
            }
        }
    };

    let (mut decrypter, mut encrypter) = cipher_session.into_split();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();
    let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), PORT);
    let from_and_recv = Address::new(&ip, ServiceFlags::NONE);
    let msg = VersionMessage {
        version: ProtocolVersion::INVALID_CB_NO_BAN_VERSION,
        services: ServiceFlags::NONE,
        timestamp: now as i64,
        receiver: from_and_recv.clone(),
        sender: from_and_recv,
        nonce: 1,
        user_agent: UserAgent::from_nonstandard("BIP-324 Client"),
        start_height: 0,
        relay: false,
    };
    let message = consensus::serialize(&V2NetworkMessage::new(NetworkMessage::Version(msg)));
    let packet_len = bip324::OutboundCipher::encryption_buffer_len(message.len());
    let mut packet = vec![0u8; packet_len];
    encrypter
        .encrypt(&message, &mut packet, PacketType::Genuine, None)
        .unwrap();
    println!("Serializing and writing version message");
    stream.write_all(&packet).unwrap();
    println!("Reading the response length buffer");
    let mut response_len = [0; NUM_LENGTH_BYTES];
    stream.read_exact(&mut response_len).unwrap();
    let message_len = decrypter.decrypt_packet_len(response_len);
    let mut response_message = vec![0; message_len];
    stream.read_exact(&mut response_message).unwrap();
    let mut decrypted_message =
        vec![0u8; bip324::InboundCipher::decryption_buffer_len(response_message.len())];
    let _ = decrypter
        .decrypt(&response_message, &mut decrypted_message, None)
        .unwrap();
    let message = consensus::deserialize::<V2NetworkMessage>(&decrypted_message[1..]).unwrap(); // Skip header byte
    assert_eq!(message.cmd(), "version");
}

#[test]
fn regtest_handshake_std() {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bip324::io::{Payload, Protocol};
    use bitcoin::consensus;
    use p2p::{
        message::{NetworkMessage, V2NetworkMessage},
        message_network::{UserAgent, VersionMessage},
        Address, ProtocolVersion, ServiceFlags,
    };

    let bitcoind = regtest_process(TransportVersion::V2);

    let stream = TcpStream::connect(bitcoind.params.p2p_socket.unwrap()).unwrap();
    let reader = stream.try_clone().unwrap();
    let writer = stream;

    // Initialize high-level protocol with handshake
    println!("Starting BIP-324 handshake");
    let mut protocol = Protocol::new(
        p2p::Magic::REGTEST,
        bip324::Role::Initiator,
        None, // no garbage
        None, // no decoys
        reader,
        writer,
    )
    .unwrap();

    println!("Handshake completed successfully!");

    // Create version message.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();
    let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), PORT);
    let from_and_recv = Address::new(&ip, ServiceFlags::NONE);
    let msg = VersionMessage {
        version: ProtocolVersion::INVALID_CB_NO_BAN_VERSION,
        services: ServiceFlags::NONE,
        timestamp: now as i64,
        receiver: from_and_recv.clone(),
        sender: from_and_recv,
        nonce: 1,
        user_agent: UserAgent::from_nonstandard("BIP-324 Client"),
        start_height: 0,
        relay: false,
    };

    let message = consensus::serialize(&V2NetworkMessage::new(NetworkMessage::Version(msg)));
    println!("Sending version message using Protocol::write()");
    protocol.write(&Payload::genuine(message)).unwrap();

    println!("Reading version response using Protocol::read()");
    let payload = protocol.read().unwrap();

    let response_message = consensus::deserialize::<V2NetworkMessage>(payload.contents()).unwrap();
    assert_eq!(response_message.cmd(), "version");

    println!("Successfully exchanged version messages using Protocol API!");
}

#[tokio::test]
async fn regtest_handshake_async() {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bip324::{futures::Protocol, io::Payload};
    use bitcoin::consensus;
    use p2p::{
        message::{NetworkMessage, V2NetworkMessage},
        message_network::{UserAgent, VersionMessage},
        Address, ProtocolVersion, ServiceFlags,
    };
    use tokio::net::TcpStream;

    let bitcoind = regtest_process(TransportVersion::V2);

    let stream = TcpStream::connect(bitcoind.params.p2p_socket.unwrap())
        .await
        .unwrap();

    let (reader, writer) = stream.into_split();

    // Initialize high-level async protocol with handshake
    println!("Starting async BIP-324 handshake");
    let mut protocol = Protocol::new(
        p2p::Magic::REGTEST,
        bip324::Role::Initiator,
        None, // no garbage
        None, // no decoys
        reader,
        writer,
    )
    .await
    .unwrap();

    println!("Async handshake completed successfully!");

    // Create version message.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();
    let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), PORT);
    let from_and_recv = Address::new(&ip, ServiceFlags::NONE);
    let msg = VersionMessage {
        version: ProtocolVersion::INVALID_CB_NO_BAN_VERSION,
        services: ServiceFlags::NONE,
        timestamp: now as i64,
        receiver: from_and_recv.clone(),
        sender: from_and_recv,
        nonce: 1,
        user_agent: UserAgent::from_nonstandard("BIP-324 Client"),
        start_height: 0,
        relay: false,
    };

    let message = consensus::serialize(&V2NetworkMessage::new(NetworkMessage::Version(msg)));
    println!("Sending version message using async Protocol::write()");
    protocol.write(&Payload::genuine(message)).await.unwrap();

    println!("Reading version response using async Protocol::read()");
    let payload = protocol.read().await.unwrap();

    let response_message = consensus::deserialize::<V2NetworkMessage>(payload.contents()).unwrap();
    assert_eq!(response_message.cmd(), "version");

    println!("Successfully exchanged version messages using async Protocol API!");
}

#[test]
#[should_panic]
fn regtest_handshake_v1_only() {
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };

    use bip324::{Handshake, Initialized};
    let bitcoind = regtest_process(TransportVersion::V1);

    let mut stream = TcpStream::connect(bitcoind.params.p2p_socket.unwrap()).unwrap();

    let handshake =
        Handshake::<Initialized>::new(p2p::Magic::REGTEST, bip324::Role::Initiator).unwrap();
    let mut public_key = vec![0u8; Handshake::<Initialized>::send_key_len(None)];
    let _handshake = handshake.send_key(None, &mut public_key).unwrap();
    println!("Writing public key to the remote node");
    stream.write_all(&public_key).unwrap();
    stream.flush().unwrap();
    let mut remote_public_key = [0u8; 64];
    println!("Reading the remote node public key");
    stream.read_exact(&mut remote_public_key).unwrap();
}

/// Bitcoind transport versions.
enum TransportVersion {
    V1,
    V2,
}

/// Fire up a managed regtest bitcoind process.
fn regtest_process(transport: TransportVersion) -> bitcoind::BitcoinD {
    // Pull executable from auto-downloaded location, unless
    // environment variable override is present. Some operating
    // systems (e.g. NixOS) don't like the downloaded executable
    // so the environment variable must be used.
    let exe_path = bitcoind::exe_path().unwrap();
    println!("Using bitcoind at {exe_path}");
    let mut conf = bitcoind::Conf::default();

    // Enable V2 if requested, otherwise disable.
    match transport {
        TransportVersion::V2 => conf.args.push("-v2transport=1"),
        TransportVersion::V1 => conf.args.push("-v2transport=0"),
    }

    // Enable p2p port for tests.
    conf.p2p = bitcoind::P2P::Yes;
    bitcoind::BitcoinD::with_conf(exe_path, &conf).unwrap()
}