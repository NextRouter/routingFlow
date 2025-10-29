mod config;
mod monitor;
mod prometheus;

use anyhow::Result;
use config::Config;
use monitor::BandwidthMonitor;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Create monitor
    let monitor = BandwidthMonitor::new(config);

    // Monitoring interval (10 seconds)
    let interval = Duration::from_secs(10);

    println!(
        "Starting bandwidth monitoring loop (interval: {:?})...\n",
        interval
    );
    println!("Press Ctrl+C to stop\n");

    // Run monitoring loop
    loop {
        match monitor.run_monitor().await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error during monitoring: {}", e);
            }
        }

        // Wait before next check
        tokio::time::sleep(interval).await;
        println!("\n{}\n", "=".repeat(50));
    }
}
