use std::io::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;

async fn handle_new_connection(socket: &mut TcpStream, addr: &SocketAddr) -> Result<(), Error> {
    socket.write_all(b"Welcome!\n").await?;
    tracing::debug!("Sent welcome message to {addr}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("Listening on port 8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        tracing::info!("Handling connection from {addr}");

        tokio::spawn(async move {
            handle_new_connection(&mut socket, &addr).await.unwrap_or_else(|error| {
                tracing::error!("Failed to handle new connection from {addr}: {error}");
            });

            tracing::info!("Connection from {addr} closed");
        });
    }
}
