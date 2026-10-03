mod common;
use std::time::Duration;
use tokio::time::sleep;

use sqlx::PgPool;

use crate::common::{TestServer, create_test_account};

/// Verifies that schedules can trigger on a periodic cycle.
#[sqlx::test(migrations = "./migrations")]
async fn test_cycle_phases(pool: PgPool) {
    create_test_account(&pool, "player", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player", "password").await;

    // At least two cycle phase changes should pass in this time.
    sleep(Duration::from_secs(5)).await;

    let message = client.recv().await;
    assert!(message.contains("tick"));
    assert!(message.contains("tock"));
}