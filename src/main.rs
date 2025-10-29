mod config;
mod monitor;
mod prometheus;

use anyhow::Result;
use config::Config;
use monitor::BandwidthMonitor;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Create monitor
    let monitor = BandwidthMonitor::new(config);

    // Run monitoring
    monitor.run_monitor().await?;

    Ok(())
}
