use azero_tools_rs::research;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    research().await?;
    Ok(())
}
