mod common;
use sqlx::PgPool;

use crate::common::{TestServer, create_test_account};

#[sqlx::test(migrations = "./migrations")]
async fn test_say_message(pool: PgPool) {
    create_test_account(&pool, "player1", "password1", false).await.expect("player1 account creation failed");
    create_test_account(&pool, "player2", "password2", false).await.expect("player2 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client1 = server.connect_as("player1", "password1").await;
    let mut client2 = server.connect_as("player2", "password2").await;

    let response = client1.send_with_response("say hello").await;

    assert!(response.contains("You say: hello"));

    let message = client2.recv().await;
    assert!(message.contains("player1 says: hello"));
}
