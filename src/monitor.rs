use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::config::{Config, StatusResponse};
use crate::prometheus::PrometheusClient;

#[derive(Debug)]
pub struct BandwidthComparison {
    pub nic: String,
    pub interface: String,
    pub estimated_bandwidth: f64,
    pub actual_rx: f64,
    pub actual_tx: f64,
    pub actual_total: f64,
    pub exceeded: bool,
}

#[derive(Debug)]
pub struct TopIpReport {
    pub nic: String,
    pub interface: String,
    pub direction: String,
    pub ip: String,
    pub bandwidth: f64,
}

pub struct BandwidthMonitor {
    config: Config,
    prometheus_client: PrometheusClient,
    http_client: reqwest::Client,
}

impl BandwidthMonitor {
    pub fn new(config: Config) -> Self {
        let prometheus_client = PrometheusClient::new(config.prometheus_url.clone());
        let http_client = reqwest::Client::new();

        Self {
            config,
            prometheus_client,
            http_client,
        }
    }

    /// Fetch status from the routing service
    pub async fn fetch_status(&self) -> Result<StatusResponse> {
        let response = self
            .http_client
            .get(&self.config.status_url)
            .send()
            .await
            .context("Failed to fetch status")?;

        let status: StatusResponse = response
            .json()
            .await
            .context("Failed to parse status response")?;

        Ok(status)
    }

    /// Compare bandwidth and identify exceeded NICs
    pub async fn compare_bandwidth(&self) -> Result<Vec<BandwidthComparison>> {
        // Get TCP bandwidth estimates
        let tcp_bandwidth = self.prometheus_client.get_tcp_bandwidth_avg().await?;

        // Build a map of interface -> estimated bandwidth
        let mut bandwidth_map: HashMap<String, f64> = HashMap::new();
        for metric in tcp_bandwidth {
            bandwidth_map.insert(metric.interface.clone(), metric.value);
        }

        // Get actual network usage for all NICs
        let nics = vec![
            self.config.nic_config.wan0.clone(),
            self.config.nic_config.wan1.clone(),
        ];

        let network_totals = self.prometheus_client.get_all_network_totals(&nics).await?;

        // Compare and build results
        let mut comparisons = Vec::new();

        for (nic, (rx, tx)) in network_totals {
            let estimated = bandwidth_map.get(&nic).copied().unwrap_or(0.0);
            let actual_total = rx + tx;
            let exceeded = actual_total > estimated;

            comparisons.push(BandwidthComparison {
                nic: nic.clone(),
                interface: nic.clone(),
                estimated_bandwidth: estimated,
                actual_rx: rx,
                actual_tx: tx,
                actual_total,
                exceeded,
            });
        }

        Ok(comparisons)
    }

    /// Find top IP addresses consuming bandwidth for a specific NIC
    pub async fn find_top_ips(&self, nic: &str) -> Result<Vec<TopIpReport>> {
        let mut reports = Vec::new();

        // Get RX metrics
        let rx_metrics = self.prometheus_client.get_network_by_ip(nic, "rx").await?;
        if let Some(top_rx) = rx_metrics.iter().max_by(|a, b| {
            a.value
                .partial_cmp(&b.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            reports.push(TopIpReport {
                nic: nic.to_string(),
                interface: nic.to_string(),
                direction: "RX".to_string(),
                ip: top_rx.ip.clone(),
                bandwidth: top_rx.value,
            });
        }

        // Get TX metrics
        let tx_metrics = self.prometheus_client.get_network_by_ip(nic, "tx").await?;
        if let Some(top_tx) = tx_metrics.iter().max_by(|a, b| {
            a.value
                .partial_cmp(&b.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            reports.push(TopIpReport {
                nic: nic.to_string(),
                interface: nic.to_string(),
                direction: "TX".to_string(),
                ip: top_tx.ip.clone(),
                bandwidth: top_tx.value,
            });
        }

        Ok(reports)
    }

    /// Run full monitoring cycle
    pub async fn run_monitor(&self) -> Result<()> {
        println!("=== Bandwidth Monitoring Report ===\n");

        // Fetch status
        let status = self.fetch_status().await?;
        println!("Network Configuration:");
        println!("  LAN: {}", status.config.lan);
        println!("  WAN0: {}", status.config.wan0);
        println!("  WAN1: {}", status.config.wan1);
        println!("\nIP Mappings:");
        for (ip, wan) in &status.mappings {
            println!("  {} -> {}", ip, wan);
        }
        println!();

        // Compare bandwidth
        let comparisons = self.compare_bandwidth().await?;

        println!("Bandwidth Comparison:");
        for comparison in &comparisons {
            println!("\n  Interface: {}", comparison.interface);
            println!(
                "    Estimated Bandwidth: {:.2} bps",
                comparison.estimated_bandwidth
            );
            println!("    Actual RX: {:.2} bps", comparison.actual_rx);
            println!("    Actual TX: {:.2} bps", comparison.actual_tx);
            println!("    Actual Total: {:.2} bps", comparison.actual_total);
            println!(
                "    Exceeded: {}",
                if comparison.exceeded {
                    "YES ⚠️"
                } else {
                    "NO ✓"
                }
            );

            // If exceeded, find top IPs
            if comparison.exceeded {
                println!("\n    Finding top IP addresses...");
                match self.find_top_ips(&comparison.nic).await {
                    Ok(top_ips) => {
                        for report in top_ips {
                            println!(
                                "      Top {} IP: {} ({:.2} bps)",
                                report.direction, report.ip, report.bandwidth
                            );
                        }
                    }
                    Err(e) => {
                        println!("      Error finding top IPs: {}", e);
                    }
                }
            }
        }

        println!("\n=== End of Report ===");

        Ok(())
    }
}
