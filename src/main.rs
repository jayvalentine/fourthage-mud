use fourthage_mud::run_server;
use fourthage_mud::AppError;
use tokio::net::TcpListener;
use uuid::uuid;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();
    
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        tracing::error!("DATABASE_URL not set");
        AppError::InitialisationError
    })?;

    let data_path = std::env::var("MUD_DATA_DIR").map_err(|e| {
       tracing::error!("Error reading MUD_DATA_DIR environment variable: {e}");
       AppError::InitialisationError 
    })?;


    let listener = TcpListener::bind("0.0.0.0:8080").await.map_err(|e| {
        tracing::error!("Error starting TCP listener: {e}");
        AppError::InitialisationError
    })?;

    let (_, shutdown_rx) = tokio::sync::oneshot::channel();

    run_server(listener, shutdown_rx, &database_url, &data_path, uuid!("019e5690-0757-7256-97c1-a403f4d347ca")).await
}
