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

    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        tracing::error!("DATABASE_URL not set");
        AppError::InitialisationError
    })?;

    tracing::info!("Connecting to database at {database_url}");

    let data_path = std::env::var("MUD_DATA_DIR").map_err(|e| {
       tracing::error!("Error reading MUD_DATA_DIR environment variable: {e}");
       AppError::InitialisationError 
    })?;

    let rooms = data::get_rooms(&format!("{data_path}/rooms/rooms.json")).map_err(|e| {
        tracing::error!("Error loading room data: {e}");
        AppError::InitialisationError
    })?;

    let world = Arc::new(World::new(rooms));

    let listener = TcpListener::bind("0.0.0.0:8080").await.map_err(|e| {
        tracing::error!("Error starting TCP listener: {e}");
        AppError::InitialisationError
    })?;
    tracing::info!("Listening on port 8080");

    loop {
        match listener.accept().await {
            Ok((mut socket, addr)) => {
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
            },
            Err(e) => {
                tracing::error!("Error handling new connection: {e}");
                continue;
            }
        }
    }
}
