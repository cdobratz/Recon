use anyhow::Result;

/// Compatibility binary. `recon` runs the same CLI as `tracelamp`.
#[tokio::main]
async fn main() -> Result<()> {
    tracelamp::run().await
}
