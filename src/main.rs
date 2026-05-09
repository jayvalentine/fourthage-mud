use std::io::Error;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

mod model;
mod command;

async fn handle_new_connection(socket: &mut TcpStream, addr: &SocketAddr) -> Result<(), Error> {
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let world = Arc::new(model::world::get_world());

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("Listening on port 8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        tracing::info!("Handling connection from {addr}");

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();

            match writer.write_all(b"Welcome!\n").await {
                Ok(_) => tracing::debug!("Sent welcome message to {addr}"),
                Err(e) => tracing::error!("Error sending message to {addr}")
            };

            let mut player = model::player::Player::new(model::world::RoomId::new(0));
            let mut reader = BufReader::new(reader);

            let mut line = String::new();

            loop {
                line.clear();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => {
                        tracing::info!("Client {addr} disconnected");
                        break;
                    }
                    Ok(_) => {
                        let input = line.trim();
                        match command::Command::parse(input) {
                            Ok(command::Command::Go(d)) => format!("You go {d}"),
                            Err(command::CommandParseError::UnknownCommand(s)) => format!("Unknown command: '{s}'"),
                            Err(command::CommandParseError::InvalidSyntax(s)) => s
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from {addr}: {e}");
                        break;
                    }
                };

                match writer.write_all(format!("{response}\n").as_bytes()).await {
                    Ok(_) => tracing::debug!("Sent response to {addr}: '{response}'"),
                    Err(e) => tracing::error!("Error sending response to {addr}")
                }
            }

            tracing::info!("Connection from {addr} closed");
        });
    }
}
