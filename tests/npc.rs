mod common;
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
    assert!(response.contains("You say: recall"));
    assert!(response.contains("remembered: foo"));
}
