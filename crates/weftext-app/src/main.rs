#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    weftext_app::launch::run().await
}
