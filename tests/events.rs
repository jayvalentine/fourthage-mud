mod common;
use sqlx::PgPool;

use crate::common::TestServer;

#[sqlx::test(migrations = "./migrations", fixtures("accounts"))]
async fn test_say_message(pool: PgPool) {
    let server = TestServer::start(&pool).await;

    let mut client1 = server.connect_as("player1", "password").await;
    let mut client2 = server.connect_as("player2", "password").await;

    let response = client1.send_with_response("say hello").await;

    assert!(response.contains("You say: hello"));

    let message = client2.recv().await;
    assert!(message.contains("player1 says: hello"));
}
