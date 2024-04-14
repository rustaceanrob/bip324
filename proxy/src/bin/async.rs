use bip324::{Handshake, Network, Role};
use bip324_proxy::{read_v1, read_v2, write_v1, write_v2};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;

/// Validate and bootstrap proxy connection.
#[allow(clippy::unused_io_amount)]
async fn proxy_conn(mut client: TcpStream) -> Result<(), bip324_proxy::Error> {
    let remote_ip = bip324_proxy::peek_addr(&client).await?;

    println!("Reaching out to {}.", remote_ip);
    let mut remote = TcpStream::connect(remote_ip).await?;

    println!("Initiating handshake.");
    let mut local_material_message = vec![0u8; 64];
    let mut handshake = Handshake::new(
        Network::Mainnet,
        Role::Initiator,
        None,
        &mut local_material_message,
    )
    .unwrap();
    remote.write_all(&local_material_message).await?;
    println!("Sent handshake to remote.");

    // 64 bytes ES.
    let mut remote_material_message = [0u8; 64];
    println!("Reading handshake response from remote.");
    remote.read_exact(&mut remote_material_message).await?;

    println!("Completing materials.");
    let mut local_garbage_terminator_message = [0u8; 36];
    handshake
        .complete_materials(
            remote_material_message,
            &mut local_garbage_terminator_message,
        )
        .unwrap();

    println!("Sending garbage terminator and version packet.");
    remote.write_all(&local_garbage_terminator_message).await?;

    println!("Authenticating garbage and version packet.");
    // TODO: Make this robust.
    let mut remote_garbage_and_version = vec![0u8; 5000];
    remote.read(&mut remote_garbage_and_version).await?;
    let packet_handler = handshake
        .authenticate_garbage_and_version(&remote_garbage_and_version)
        .expect("authenticated garbage");
    println!("Channel authenticated.");

    println!("Splitting channels.");
    let (mut client_reader, mut client_writer) = client.split();
    let (mut remote_reader, mut remote_writer) = remote.split();
    let (mut decrypter, mut encrypter) = packet_handler.split();

    println!("Setting up proxy loop.");
    loop {
        select! {
            res = read_v1(&mut client_reader) => {
                match res {
                    Ok(msg) => {
                         println!("Read {} message from client, writing to remote.", msg.cmd);
                         write_v2(&mut remote_writer, &mut encrypter, msg).await?;
                    },
                    Err(e) => {
                         return Err(e);
                    },
                }
            },
            res = read_v2(&mut remote_reader, &mut decrypter) => {
                match res {
                    Ok(msg) => {
                         println!("Read {} message from remote, writing to client.", msg.cmd);
                         write_v1(&mut client_writer, msg).await?;
                    },
                    Err(e) => {
                         return Err(e);
                    },
                }
            },
        }
    }
}

#[tokio::main]
async fn main() {
    let proxy = TcpListener::bind(bip324_proxy::DEFAULT_PROXY)
        .await
        .expect("Failed to bind to proxy port.");
    println!(
        "Listening for connections on {}",
        bip324_proxy::DEFAULT_PROXY
    );
    loop {
        let (stream, _) = proxy
            .accept()
            .await
            .expect("Failed to accept inbound connection.");
        // Spawn a new task per connection.
        tokio::spawn(async move {
            match proxy_conn(stream).await {
                Ok(_) => {
                    println!("Ended connection with no errors.");
                }
                Err(e) => {
                    println!("Ended connection with error: {e}.");
                }
            };
        });
    }
}