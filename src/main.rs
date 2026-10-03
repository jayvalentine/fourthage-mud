use fourthage_mud::run_server;
use fourthage_mud::AppError;
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();
    
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        AppError::InitialisationError("DATABASE_URL not set".into())
    })?;

    let data_path = std::env::var("MUD_DATA_DIR").map_err(|e| {
       AppError::InitialisationError(format!("Error reading MUD_DATA_DIR environment variable: {e}"))
    })?;

    let listener = TcpListener::bind("0.0.0.0:8080").await.map_err(|e| {
        AppError::InitialisationError(format!("Error starting TCP listener: {e}"))
    })?;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            tracing::error!("Failed to listen for Ctrl+C: {e}");
            return;
        }
        tracing::info!("Ctrl+C received, shutting down...");
        let _ = shutdown_tx.send(());
    });

    run_server(listener, shutdown_rx, &database_url, &data_path).await
}
