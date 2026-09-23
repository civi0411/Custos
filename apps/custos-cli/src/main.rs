#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    custos_cli::run().await
}
