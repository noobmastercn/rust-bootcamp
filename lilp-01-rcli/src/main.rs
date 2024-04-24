// rcli csv -i input.csv -o output.json --header -d ','

use clap::Parser;
use rcli::{CmdExector, Opts};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // 设置日志级别为INFO
        .init();

    let opts = Opts::parse();
    opts.cmd.execute().await?;

    Ok(())
}
