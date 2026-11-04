use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracelamp::run().await
}
