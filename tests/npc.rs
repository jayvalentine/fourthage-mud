mod common;
use sqlx::PgPool;

use crate::common::{TestServer, create_test_account};

#[sqlx::test(migrations = "./migrations")]
async fn test_npc_event_response(pool: PgPool) {
    create_test_account(&pool, "player", "password", false).await.expect("player1 account creation failed");

    let server = TestServer::start(&pool).await;

    let mut client = server.connect_as("player", "password").await;

    let response = client.send_with_response("say hello").await;
    assert!(response.contains("You say: hello"));
    assert!(response.contains("Woof!"));
}
