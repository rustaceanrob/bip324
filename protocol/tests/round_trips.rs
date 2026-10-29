// SPDX-License-Identifier: CC0-1.0

#[cfg(feature = "std")]
const MAGIC: [u8; 4] = [0xF9, 0xBE, 0xB4, 0xD9];

#[test]
#[cfg(feature = "std")]
fn hello_world_happy_path() {
    use bip324::{
        GarbageResult, Handshake, Initialized, PacketType, ReceivedKey, Role, VersionResult,
        NUM_LENGTH_BYTES,
    };

    // Create initiator handshake
    let init_handshake = Handshake::<Initialized>::new(MAGIC, Role::Initiator).unwrap();

    // Send initiator key
    let mut init_key_buffer = vec![0u8; Handshake::<Initialized>::send_key_len(None)];
    let init_handshake = init_handshake.send_key(None, &mut init_key_buffer).unwrap();

    // Create responder handshake
    let resp_handshake = Handshake::<Initialized>::new(MAGIC, Role::Responder).unwrap();

    // Send responder key
    let mut resp_key_buffer = vec![0u8; Handshake::<Initialized>::send_key_len(None)];
    let resp_handshake = resp_handshake.send_key(None, &mut resp_key_buffer).unwrap();

    // Initiator receives responder's key
    let init_handshake = init_handshake
        .receive_key(resp_key_buffer[..64].try_into().unwrap())
        .unwrap();

    // Responder receives initiator's key
    let resp_handshake = resp_handshake
        .receive_key(init_key_buffer[..64].try_into().unwrap())
        .unwrap();

    // Initiator sends version
    let mut init_version_buffer = vec![0u8; Handshake::<ReceivedKey>::send_version_len(None)];
    let init_handshake = init_handshake
        .send_version(&mut init_version_buffer, None)
        .unwrap();

    // Responder sends version
    let mut resp_version_buffer = vec![0u8; Handshake::<ReceivedKey>::send_version_len(None)];
    let resp_handshake = resp_handshake
        .send_version(&mut resp_version_buffer, None)
        .unwrap();

    // Initiator receives responder's garbage and version
    let (mut init_handshake, consumed) = match init_handshake
        .receive_garbage(&resp_version_buffer)
        .unwrap()
    {
        GarbageResult::FoundGarbage {
            handshake,
            consumed_bytes,
        } => (handshake, consumed_bytes),
        GarbageResult::NeedMoreData(_) => panic!("Should have found garbage"),
    };

    // Process the version packet properly
    let remaining = &resp_version_buffer[consumed..];
    let packet_len = init_handshake
        .decrypt_packet_len(remaining[..NUM_LENGTH_BYTES].try_into().unwrap())
        .unwrap();
    let mut version_packet = remaining[NUM_LENGTH_BYTES..NUM_LENGTH_BYTES + packet_len].to_vec();
    let mut alice = match init_handshake.receive_version(&mut version_packet).unwrap() {
        VersionResult::Complete { cipher } => cipher,
        VersionResult::Decoy(_) => panic!("Should have completed"),
    };

    // Responder receives initiator's garbage and version
    let (mut resp_handshake, consumed) = match resp_handshake
        .receive_garbage(&init_version_buffer)
        .unwrap()
    {
        GarbageResult::FoundGarbage {
            handshake,
            consumed_bytes,
        } => (handshake, consumed_bytes),
        GarbageResult::NeedMoreData(_) => panic!("Should have found garbage"),
    };

    // Process the version packet properly
    let remaining = &init_version_buffer[consumed..];
    let packet_len = resp_handshake
        .decrypt_packet_len(remaining[..NUM_LENGTH_BYTES].try_into().unwrap())
        .unwrap();
    let mut version_packet = remaining[NUM_LENGTH_BYTES..NUM_LENGTH_BYTES + packet_len].to_vec();
    let mut bob = match resp_handshake.receive_version(&mut version_packet).unwrap() {
        VersionResult::Complete { cipher } => cipher,
        VersionResult::Decoy(_) => panic!("Should have completed"),
    };

    // Alice and Bob can freely exchange encrypted messages using the packet handler returned by each handshake.
    let message = b"Hello world".to_vec();
    let packet_len = bip324::OutboundCipher::encryption_buffer_len(message.len());
    let mut encrypted_message_to_alice = vec![0u8; packet_len];
    bob.outbound()
        .encrypt(
            &message,
            &mut encrypted_message_to_alice,
            PacketType::Genuine,
            None,
        )
        .unwrap();

    let alice_message_len = alice.inbound().decrypt_packet_len(
        encrypted_message_to_alice[..NUM_LENGTH_BYTES]
            .try_into()
            .unwrap(),
    );
    let mut decrypted_message =
        vec![0u8; bip324::InboundCipher::decryption_buffer_len(alice_message_len)];
    alice
        .inbound()
        .decrypt(
            &encrypted_message_to_alice[NUM_LENGTH_BYTES..],
            &mut decrypted_message,
            None,
        )
        .unwrap();
    assert_eq!(message, decrypted_message[1..].to_vec()); // Skip header byte

    let message = b"Goodbye!".to_vec();
    let packet_len = bip324::OutboundCipher::encryption_buffer_len(message.len());
    let mut encrypted_message_to_bob = vec![0u8; packet_len];
    alice
        .outbound()
        .encrypt(
            &message,
            &mut encrypted_message_to_bob,
            PacketType::Genuine,
            None,
        )
        .unwrap();

    let bob_message_len = bob.inbound().decrypt_packet_len(
        encrypted_message_to_bob[..NUM_LENGTH_BYTES]
            .try_into()
            .unwrap(),
    );
    let mut decrypted_message =
        vec![0u8; bip324::InboundCipher::decryption_buffer_len(bob_message_len)];
    bob.inbound()
        .decrypt(
            &encrypted_message_to_bob[NUM_LENGTH_BYTES..],
            &mut decrypted_message,
            None,
        )
        .unwrap();
    assert_eq!(message, decrypted_message[1..].to_vec()); // Skip header byte
}

#[test]
#[cfg(feature = "std")]
fn pingpong_with_closed_connection_sync() {
    use bip324::io::{Payload, Protocol};
    use bitcoin::consensus;
    use p2p::message::{NetworkMessage, V2NetworkMessage};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let reader = stream.try_clone().unwrap();
        let writer = stream;

        let mut protocol = Protocol::new(
            p2p::Magic::REGTEST,
            bip324::Role::Responder,
            None,
            None,
            reader,
            writer,
        )
        .expect("Failed to create protocol");

        let session_id = *protocol.session_id();

        // Read one message
        let payload = protocol.read().expect("Failed to read payload");
        let received_message = consensus::deserialize::<V2NetworkMessage>(payload.contents())
            .expect("Failed to deserialize");

        if let NetworkMessage::Ping(x) = received_message.payload() {
            let pong = V2NetworkMessage::new(NetworkMessage::Pong(*x));
            let message = consensus::serialize(&pong);
            protocol
                .write(&Payload::genuine(message))
                .expect("Failed to write pong");
            println!("Pong sent, stopping server.");
        } else {
            panic!("Expected Ping, but received: {received_message:?}");
        }
        session_id
    });

    let stream = TcpStream::connect(addr).unwrap();
    let reader = stream.try_clone().unwrap();
    let writer = stream;

    println!("Starting sync BIP-324 handshake");
    let mut protocol = Protocol::new(
        p2p::Magic::REGTEST,
        bip324::Role::Initiator,
        None,
        None,
        reader,
        writer,
    )
    .expect("Failed to create protocol");

    let session_id = *protocol.session_id();

    println!("Sending Ping using sync Protocol::write()");
    let ping = V2NetworkMessage::new(NetworkMessage::Ping(45324));
    let message = consensus::serialize(&ping);
    protocol
        .write(&Payload::genuine(message))
        .expect("Failed to write ping");

    println!("Reading response using sync Protocol::read()");
    let payload = protocol.read().expect("Failed to read response");
    let response_message = consensus::deserialize::<V2NetworkMessage>(payload.contents())
        .expect("Failed to deserialize response");

    assert_eq!(NetworkMessage::Pong(45324), *response_message.payload());

    println!("Successfully ping-pong'ed using sync Protocol API!");
    let server_session_id = server.join().unwrap();

    assert_eq!(session_id, server_session_id);

    println!(
        "Trying to read another message from the server, while the connection is already closed."
    );
    assert!(protocol.read().is_err());
}

#[tokio::test]
#[cfg(feature = "tokio")]
async fn pingpong_with_closed_connection_async() {
    use bip324::{futures::Protocol, io::Payload};
    use bitcoin::consensus;
    use p2p::message::{NetworkMessage, V2NetworkMessage};
    use tokio::net::TcpListener;
    use tokio::net::TcpStream;

    // Start a server that responds to exactly one Ping(x) message with a
    // Pong(x) message and then stops. This allows testing to read from a closed stream.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, writer) = stream.into_split();
        let mut protocol = Protocol::new(
            p2p::Magic::REGTEST,
            bip324::Role::Responder,
            None, // no garbage
            None, // no decoys
            reader,
            writer,
        )
        .await
        .unwrap();

        let session_id = *protocol.session_id();

        let payload = protocol.read().await.unwrap();
        let received_message =
            consensus::deserialize::<V2NetworkMessage>(payload.contents()).unwrap();
        if let NetworkMessage::Ping(x) = received_message.payload() {
            let pong = V2NetworkMessage::new(NetworkMessage::Pong(*x));
            let message = consensus::serialize(&pong);
            protocol.write(&Payload::genuine(message)).await.unwrap();
            println!("Pong sent, stopping server.")
        } else {
            panic!("Expected Ping, but received: {received_message:?}");
        }

        session_id
    });

    let stream = TcpStream::connect(addr).await.unwrap();

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

    let session_id = *protocol.session_id();

    println!("Sending Ping using async Protocol::write()");
    let ping = V2NetworkMessage::new(NetworkMessage::Ping(45324));
    let message = consensus::serialize(&ping);
    protocol.write(&Payload::genuine(message)).await.unwrap();

    println!("Reading response using async Protocol::read()");
    let payload = protocol.read().await.unwrap();
    let response_message = consensus::deserialize::<V2NetworkMessage>(payload.contents()).unwrap();

    assert_eq!(NetworkMessage::Pong(45324), *response_message.payload());

    println!("Successfully ping-pong message using async Protocol API!");
    let server_session_id = server.await.unwrap();

    assert_eq!(session_id, server_session_id);

    println!(
        "Trying to read another message from the server, while the connection is already closed."
    );
    assert!(protocol.read().await.is_err());
}
