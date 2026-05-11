use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::io::BufReader;

mod model;
mod command;
mod session;
mod data;

use model::world::World;

#[derive(Debug)]
enum AppError {
    InitialisationError
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();

    let rooms = data::get_rooms("data/rooms/rooms.json").map_err(|e| {
        tracing::error!("Error loading room data: {e}");
        AppError::InitialisationError
    })?;

    let world = Arc::new(World::new(rooms));

    let listener = TcpListener::bind("127.0.0.1:8080").await.map_err(|e| {
        tracing::error!("Error starting TCP listener: {e}");
        AppError::InitialisationError
    })?;
    tracing::info!("Listening on port 8080");

    loop {
        let result = listener.accept().await;
        if result.is_err() {
            let e = result.unwrap_err();
            tracing::error!("Error handling new connection: {e}");
            continue;
        }

        let (mut socket, addr) = result.unwrap();
        tracing::info!("Handling connection from {addr}");

        let world = world.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);

            session::run(&mut writer, &mut reader, world).await.unwrap_or_else(|e| {
                tracing::error!("Error during session from {addr}: {e}");
            });

            tracing::info!("Connection from {addr} closed");
        });
    }
}
