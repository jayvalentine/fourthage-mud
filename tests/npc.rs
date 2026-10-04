mod common;
use std::time::Duration;
use tokio::time::sleep;

use sqlx::PgPool;

use crate::common::{TestServer, create_test_account};

/// Verifies that NPCs can respond to events.
#[sqlx::test(migrations = "./migrations")]
async fn test_npc_event_response(pool: PgPool) {
    create_test_account(&pool, "player", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player", "password").await;

    let response = client.send_with_response("say test query").await;
    assert!(response.contains("You say: test query"));
    assert!(response.contains("test response"));

    let response = client.send_with_response("say other query").await;
    assert!(response.contains("You say: other query"));
    assert!(!response.contains("test response"));
}

/// Verifies that NPC scripts can use memory.
#[sqlx::test(migrations = "./migrations")]
async fn test_npc_memory(pool: PgPool) {
    create_test_account(&pool, "player", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player", "password").await;

    let response = client.send_with_response("say remember foo").await;
    assert!(response.contains("You say: remember foo"));

    let response = client.send_with_response("say recall").await;
    tracing::debug!(response);
    assert!(response.contains("You say: recall"));
    assert!(response.contains("remembered: foo"));
}

/// Verifies that NPC scripts can trigger NPC movement.
#[sqlx::test(migrations = "./migrations")]
async fn test_npc_movement(pool: PgPool) {
    create_test_account(&pool, "player", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player", "password").await;

    // Wait some time, assert that the NPC has left and returned.
    // The NPC should move once per tick.
    sleep(Duration::from_secs(5)).await;
    let messages = client.recv().await;
    assert!(messages.contains("Moving Test NPC arrived."));
    assert!(messages.contains("Moving Test NPC left."));
}

/// Verifies that NPC scripts can use pathfinding.
#[sqlx::test(migrations = "./migrations")]
async fn test_npc_pathfinding(pool: PgPool) {
    create_test_account(&pool, "player", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player", "password").await;

    // Wait some time, assert that the NPC has left and returned.
    // The NPC should move once per tick.
    sleep(Duration::from_secs(5)).await;
    let messages = client.recv().await;
    assert!(messages.contains("Pathfinding Test NPC arrived."));
    assert!(messages.contains("Pathfinding Test NPC left."));
}
