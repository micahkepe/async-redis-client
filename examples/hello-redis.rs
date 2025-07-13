//! Hello world server.
use mini_redis::{Result, client};

#[tokio::main]
async fn main() -> Result<()> {
    // Open a connection to mini-redis
    let mut client = client::connect("127.0.0.1:6379").await?;

    // Set the key "hello"
    client.set("hello", "world".into()).await?;

    // get key
    let result = client.get("hello").await?;

    println!("got value from server; result={:?}", result);

    Ok(())
}
