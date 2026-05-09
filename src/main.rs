use std::io::Error;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::io::BufReader;

mod model;
mod command;
mod session;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let world = Arc::new(model::world::get_world());

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("Listening on port 8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
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
