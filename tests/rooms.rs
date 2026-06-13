mod common;
use sqlx::PgPool;

use crate::common::TestServer;

#[sqlx::test(migrations = "./migrations", fixtures("accounts"))]
async fn test_room_description(pool: PgPool) {
    let server = TestServer::start(&pool).await;
    let mut client = server.connect_as("player1", "password").await;

    let response = client.send_with_response("look")
                         .await.expect("Failed to send command");

    assert!(response.contains("Starting Room"));
    assert!(response.contains("The room that you start in."));
}
