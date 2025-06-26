use mango_launcher::Result;

#[tokio::main]
async fn main() -> Result<()> {
    mango_launcher::run().await
} 