mod common;
use sqlx::PgPool;

use crate::common::TestServer;

#[sqlx::test(migrations = "./migrations", fixtures("accounts"))]
async fn test_persist_location(pool: PgPool) {
    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player1", "password").await;

    // Change position.
    let response = client.send_with_response("go north")
                         .await.expect("Failed to send command");
    assert!(response.contains("North Room"));

    // Disconnect and restart the server.
    drop(client);
    let server = server.restart().await;

    // Verify that the position is the same after restart.
    let mut client = server.connect_as("player1", "password").await;
    let response = client.send_with_response("look")
                         .await.expect("Failed to send command");
    assert!(response.contains("North Room"));
}

#[sqlx::test(migrations = "./migrations", fixtures("accounts"))]
async fn test_persist_item(pool: PgPool) {
    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player1", "password").await;

    // Pick up the test item.
    let response = client.send_with_response("take test item")
                         .await.expect("Failed to send command");
    assert!(response.contains("took 'Test Item'"));

    // Disconnect and restart the server.
    drop(client);
    let server = server.restart().await;

    // Verify that the item is still in the player's inventory after restart.
    let mut client = server.connect_as("player1", "password").await;
    let response = client.send_with_response("inventory")
                         .await.expect("Failed to send command");
    assert!(response.contains("Test Item"));

    // Verify that the item is not in the room.
    let response = client.send_with_response("look")
                         .await.expect("Failed to send command");
    assert!(!response.contains("Test Item"));
}
