use sqlx::{ConnectOptions, PgPool};
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use uuid::uuid;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::{net::TcpStream, time::sleep};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

use fourthage_mud::run_server;


async fn wait_for_port(addr: &str, timeout: Duration) -> std::io::Result<()> {
    let deadline = Instant::now() + timeout;

    loop {
        match TcpStream::connect(addr).await {
            Ok(_) => return Ok(()),
            Err(err) if Instant::now() < deadline => {
                // Only retry on transient errors
                if err.kind() == std::io::ErrorKind::ConnectionRefused
                    || err.kind() == std::io::ErrorKind::ConnectionAborted
                    || err.kind() == std::io::ErrorKind::NotConnected
                {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                }
                return Err(err);
            }
            Err(err) => return Err(err),
        }
    }
}

pub struct TestClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf
}

impl TestClient {
    pub async fn connect(addr: &SocketAddr) -> Self {
        let stream = TcpStream::connect(addr).await.expect("Failed to connect to server");
        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);
        TestClient { reader, writer }
    }

    pub async fn send_with_response(&mut self, message: &str) -> Result<String, std::io::Error> {
        self.writer.write_all(message.as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;

        let mut response: [u8;1024] = [0;1024];
        self.reader.read(&mut response).await?;
        let response = String::from_utf8_lossy(&response);
        Ok(response.trim().to_string())
    }

    pub async fn recv(&mut self) -> Result<String, std::io::Error> {
        let mut response: [u8;1024] = [0;1024];
        self.reader.read(&mut response).await?;
        let response = String::from_utf8_lossy(&response);
        Ok(response.trim().to_string())
    }

    pub async fn send(&mut self, message: &str) -> Result<(), std::io::Error> {
        self.writer.write_all(message.as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;
        Ok(())
    }
}

pub struct TestServer {
    addr: SocketAddr,
}

impl TestServer {
    pub async fn start(pool: &PgPool) -> Self {
        let db_url = pool.connect_options().to_url_lossy().to_string();

        let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_path.push("tests/data");


        let listener = TcpListener::bind("0.0.0.0:0").await.expect("Failed to bind TCP listener");
        let addr = listener.local_addr().expect("Failed to get local address");

        tokio::spawn(async move {
            run_server(listener, &db_url, data_path.to_str().unwrap_or_default(), uuid!("00000000-0000-0000-0000-000000000001")).await
        });

        wait_for_port(&addr.to_string(), Duration::from_secs(10)).await
            .expect("Server did not start within timeout");

        TestServer { addr }
    }

    /// Connect without logging in.
    pub async fn connect(&self) -> TestClient {
        TestClient::connect(&self.addr).await
    }

    /// Connect as an existing player with the given credentials.
    pub async fn connect_as(&self, username: &str, password: &str) -> TestClient {
        let mut client = self.connect().await;
        
        let response = client.recv().await.expect("Failed to receive prompt");
        assert!(response.contains("username:"));

        let response = client.send_with_response(username).await.expect("Failed to send username");
        assert!(response.contains("password:"));

        let response = client.send_with_response(password).await.expect("Failed to send password");
        assert!(response.contains(&format!("Welcome {}", username)));

        client
    }
}
