mod common;
use sqlx::PgPool;

use crate::common::{TestServer, create_test_account};

#[sqlx::test(migrations = "./migrations")]
async fn test_persist_location(pool: PgPool) {
    create_test_account(&pool, "player1", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player1", "password").await;

    // Change position.
    let response = client.send_with_response("go north").await;
    assert!(response.contains("North Room"));

    // Disconnect and restart the server.
    drop(client);
    let server = server.restart().await;

    // Verify that the position is the same after restart.
    let mut client = server.connect_as("player1", "password").await;
    let response = client.send_with_response("look").await;
    assert!(response.contains("North Room"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_persist_item(pool: PgPool) {
    create_test_account(&pool, "player1", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player1", "password").await;

    // Pick up the test item.
    let response = client.send_with_response("take test item").await;
    assert!(response.contains("took 'Test Item'"));

    // Disconnect and restart the server.
    drop(client);
    let server = server.restart().await;

    // Verify that the item is still in the player's inventory after restart.
    let mut client = server.connect_as("player1", "password").await;
    let response = client.send_with_response("inventory").await;
    assert!(response.contains("Test Item"));

    // Verify that the item is not in the room.
    let response = client.send_with_response("look").await;
    assert!(!response.contains("Test Item"));
}
