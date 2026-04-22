mod app;
mod cli_args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
