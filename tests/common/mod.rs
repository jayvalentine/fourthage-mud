#![allow(dead_code)]

use sqlx::{ConnectOptions, PgPool};
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::task::JoinHandle;
use uuid::uuid;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::{net::TcpStream, time::sleep};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use fourthage_mud::{run_server, test_hash_password};

pub async fn create_test_account(pool: &PgPool, username: &str, password: &str, is_admin: bool) -> Result<(), sqlx::Error> {
    let hash = test_hash_password(password);
    sqlx::query(
        "INSERT INTO accounts (username, password_hash, is_admin) VALUES ($1, $2, $3)"
    )
    .bind(username)
    .bind(&hash)
    .bind(is_admin)
    .execute(pool)
    .await?;
    Ok(())
}

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

    pub async fn send_with_response(&mut self, message: &str) -> String {
        self.send(message).await;
        self.recv().await
    }

    pub async fn recv(&mut self) -> String {
        let mut response = String::new();
        loop {
            let mut line = String::new();
            match tokio::time::timeout(
                Duration::from_millis(1000),
                self.reader.read_line(&mut line)
            ).await {
                Ok(Ok(0)) => break,  // connection closed
                Ok(Ok(_)) => response.push_str(&line),
                Ok(Err(e)) => panic!("Error reading response: {e}"),
                Err(_) => break,  // timeout - assume response complete
            }
        }
        response.trim().to_string()
    }

    pub async fn send(&mut self, message: &str) {
        self.writer.write_all(message.as_bytes()).await
            .expect("Failed to send message");
        self.writer.write_all(b"\r\n").await
            .expect("Failed to send newline");
    }
}

pub struct TestServer {
    pool: PgPool,
    addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl TestServer {
    pub async fn start(pool: &PgPool) -> Self {
        let _ = tracing_subscriber::fmt::try_init();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let db_url = pool.connect_options().to_url_lossy().to_string();

        let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_path.push("tests/data");


        let listener = TcpListener::bind("0.0.0.0:0").await.expect("Failed to bind TCP listener");
        let addr = listener.local_addr().expect("Failed to get local address");
        tracing::info!("Test server listening on {}", addr);

        let task = tokio::spawn(async move {
            run_server(listener, shutdown_rx, &db_url, data_path.to_str().unwrap_or_default(), uuid!("00000000-0000-0000-0000-000000000001"))
                .await.expect("Server error.");
        });

        wait_for_port(&addr.to_string(), Duration::from_secs(10)).await
            .expect("Server did not start within timeout");

        TestServer { pool: pool.clone(), addr, shutdown_tx: Some(shutdown_tx), task: Some(task) }
    }

    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.task.take() {
            let _ = t.await;
        }
    }

    pub async fn restart(self) -> TestServer {
        let pool = self.pool.clone();
        self.stop().await;
        tracing::info!("Restarting server...");
        TestServer::start(&pool).await
    }

    /// Connect without logging in.
    pub async fn connect(&self) -> TestClient {
        TestClient::connect(&self.addr).await
    }

    /// Connect as an existing player with the given credentials.
    pub async fn connect_as(&self, username: &str, password: &str) -> TestClient {
        let mut client = self.connect().await;
        
        let response = client.recv().await;
        assert!(response.contains("username:"));

        let response = client.send_with_response(username).await;
        assert!(response.contains("password:"));

        let response = client.send_with_response(password).await;
        assert!(response.contains(&format!("Welcome {}", username)));

        client
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
