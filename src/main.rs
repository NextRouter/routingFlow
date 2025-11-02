use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct PrometheusResponse {
    data: PrometheusData,
}

#[derive(Debug, Deserialize)]
struct PrometheusData {
    result: Vec<PrometheusResult>,
}

#[derive(Debug, Deserialize)]
struct PrometheusResult {
    metric: HashMap<String, String>,
    value: (f64, String),
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    config: ConfigInfo,
    mappings: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ConfigInfo {
    lan: String,
    wan0: String,
    wan1: String,
}

#[derive(Debug, Default)]
struct NicStats {
    tcp_bandwidth: f64,
    tx_bps: f64,
    rx_bps: f64,
}

#[derive(Debug, Clone)]
struct SwitchRecord {
    ip: String,
    target_wan: String,
    timestamp: u64,
}

async fn query_prometheus(client: &Client, query: &str) -> Result<Vec<PrometheusResult>> {
    let url = format!(
        "http://localhost:9090/api/v1/query?query={}",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to query Prometheus")?;

    let prom_response: PrometheusResponse = response
        .json()
        .await
        .context("Failed to parse Prometheus response")?;

    Ok(prom_response.data.result)
}

async fn get_status_mappings(client: &Client) -> Result<StatusResponse> {
    let response = client
        .get("http://localhost:32599/status")
        .send()
        .await
        .context("Failed to get status from localhost:32599")?;

    let status: StatusResponse = response
        .json()
        .await
        .context("Failed to parse status response")?;

    Ok(status)
}

fn build_wan_to_nic_map(config: &ConfigInfo) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("wan0".to_string(), config.wan0.clone());
    map.insert("wan1".to_string(), config.wan1.clone());
    map
}

fn build_ip_to_nic_map(
    status: &StatusResponse,
    wan_to_nic: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut ip_to_nic = HashMap::new();

    for (ip, wan) in &status.mappings {
        if let Some(nic) = wan_to_nic.get(wan) {
            ip_to_nic.insert(ip.clone(), nic.clone());
        }
    }

    ip_to_nic
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    let mut switch_history: Vec<SwitchRecord> = Vec::new();

    loop {
        // Step 1: Get status mappings
        println!("Fetching status mappings from localhost:32599...");
        let status = get_status_mappings(&client).await?;

        let wan_to_nic = build_wan_to_nic_map(&status.config);
        let ip_to_nic = build_ip_to_nic_map(&status, &wan_to_nic);

        println!("\nNIC Configuration:");
        println!("  LAN: {}", status.config.lan);
        println!("  WAN0: {} ({})", wan_to_nic.get("wan0").unwrap(), "wan0");
        println!("  WAN1: {} ({})", wan_to_nic.get("wan1").unwrap(), "wan1");
        println!();

        // Step 2: Query tcp_traffic_scan data
        println!("Fetching TCP bandwidth data from Prometheus...");
        let tcp_query =
            r#"{job="tcp-traffic-scan",__name__=~"tcp_traffic_scan_tcp_bandwidth_avg_bps"}"#;
        let tcp_results = query_prometheus(&client, tcp_query).await?;

        let mut nic_stats: HashMap<String, NicStats> = HashMap::new();

        // Process TCP bandwidth data (grouped by interface)
        for result in tcp_results {
            if let Some(interface) = result.metric.get("interface") {
                let value: f64 = result.value.1.parse().unwrap_or(0.0);
                nic_stats
                    .entry(interface.clone())
                    .or_default()
                    .tcp_bandwidth += value;
            }
        }

        // Step 3: Query localpacketdump data
        println!("Fetching network traffic data from Prometheus...");
        let network_query =
            r#"{job="lcoalpacketdump",__name__=~"network_ip_tx_bps|network_ip_rx_bps"}"#;
        let network_results = query_prometheus(&client, network_query).await?;

        // Process network data (aggregate by NIC using IP mappings)
        for result in &network_results {
            if let (Some(metric_name), Some(ip)) = (
                result.metric.get("__name__"),
                result.metric.get("ip_address"),
            ) {
                if let Some(nic) = ip_to_nic.get(ip) {
                    let value: f64 = result.value.1.parse().unwrap_or(0.0);

                    let stats = nic_stats.entry(nic.clone()).or_default();

                    if metric_name == "network_ip_tx_bps" {
                        stats.tx_bps += value;
                    } else if metric_name == "network_ip_rx_bps" {
                        stats.rx_bps += value;
                    }
                }
            }
        }

        // Display results
        println!("\n=== NIC Statistics ===\n");

        let mut nics: Vec<_> = nic_stats.keys().collect();
        nics.sort();

        for nic in nics {
            if let Some(stats) = nic_stats.get(nic) {
                println!("Interface: {}", nic);
                println!(
                    "  TCP Bandwidth (avg): {:.2} bps ({:.2} Mbps)",
                    stats.tcp_bandwidth,
                    stats.tcp_bandwidth / 1_000_000.0
                );
                println!(
                    "  TX (total): {:.2} bps ({:.2} Mbps)",
                    stats.tx_bps,
                    stats.tx_bps / 1_000_000.0
                );
                println!(
                    "  RX (total): {:.2} bps ({:.2} Mbps)",
                    stats.rx_bps,
                    stats.rx_bps / 1_000_000.0
                );
                println!(
                    "  Total Traffic: {:.2} bps ({:.2} Mbps)",
                    stats.tx_bps + stats.rx_bps,
                    (stats.tx_bps + stats.rx_bps) / 1_000_000.0
                );
                // Find all IPs mapped to this NIC and their RX traffic
                let mut ip_rx_list: Vec<(String, f64)> = Vec::new();

                for result in &network_results {
                    if let (Some(metric_name), Some(ip)) = (
                        result.metric.get("__name__"),
                        result.metric.get("ip_address"),
                    ) {
                        if metric_name == "network_ip_rx_bps" {
                            if let Some(mapped_nic) = ip_to_nic.get(ip) {
                                if mapped_nic == nic {
                                    let value: f64 = result.value.1.parse().unwrap_or(0.0);
                                    ip_rx_list.push((ip.clone(), value));
                                }
                            }
                        }
                    }
                }

                // Sort by RX traffic (descending)
                ip_rx_list
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                println!("  Top IPs by RX traffic:");

                // Get current timestamp for checking recent switches
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                for (ip, rx) in ip_rx_list[0..1].iter() {
                    println!("    {} - {:.2} bps ({:.2} Mbps)", ip, rx, rx / 1_000_000.0);

                    // Check if this IP was recently switched (within 30 seconds)
                    let recently_switched = switch_history
                        .iter()
                        .any(|record| &record.ip == ip && (now - record.timestamp) <= 30);

                    if recently_switched {
                        println!(
                            "    ⏭ Skipping {} - already switched within last 30 seconds",
                            ip
                        );
                        continue;
                    }

                    // Find the NIC with the highest TCP bandwidth
                    let target_nic = nic_stats
                        .iter()
                        .filter(|(n, _)| *n != nic) // Exclude current NIC
                        .max_by(|(_, a), (_, b)| {
                            a.tcp_bandwidth
                                .partial_cmp(&b.tcp_bandwidth)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(n, _)| n.clone())
                        .unwrap_or_else(|| nic.clone());

                    let target_wan = wan_to_nic
                        .iter()
                        .find(|(_wan, nic_name)| *nic_name == &target_nic)
                        .map(|(wan, _)| wan.clone())
                        .unwrap_or_else(|| "wan0".to_string());

                    let switch_url =
                        format!("http://localhost:32599/switch?ip={}&nic={}", ip, target_wan);
                    println!(
                        "    Attempting to switch {} to {} via: {}",
                        ip, target_wan, switch_url
                    );
                    match client.get(&switch_url).send().await {
                        Ok(response) => {
                            let status = response.status();
                            println!("    API Response Status: {}", status);
                            if status.is_success() {
                                println!("    ✓ Successfully switched {} to {}", ip, target_wan);

                                // Record the switch with timestamp
                                switch_history.push(SwitchRecord {
                                    ip: ip.clone(),
                                    target_wan: target_wan.clone(),
                                    timestamp: now,
                                });
                            } else {
                                eprintln!("    ✗ API returned error status: {}", status);
                            }
                        }
                        Err(e) => {
                            eprintln!("    ✗ Failed to reach API for IP {}: {}", ip, e);
                        }
                    }
                }

                println!();
                println!("  History of IPs switched on this NIC:");

                // Track recently switched IPs (within last 30 seconds)
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Filter history for this NIC and last 30 seconds
                let recent_switches: Vec<_> = switch_history
                    .iter()
                    .filter(|record| {
                        // Find the NIC for this switch target
                        if let Some(switched_nic) = wan_to_nic.get(&record.target_wan) {
                            switched_nic == nic && (now - record.timestamp) <= 30
                        } else {
                            false
                        }
                    })
                    .collect();

                if recent_switches.is_empty() {
                    println!("    (No recent switches in the last 30 seconds)");
                } else {
                    for record in recent_switches {
                        let age = now - record.timestamp;
                        println!("    {} → {} - {}s ago", record.ip, record.target_wan, age);
                    }
                }
            }
        }

        // Clean up old records (older than 30 seconds)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        switch_history.retain(|record| (now - record.timestamp) <= 30);

        println!("\n=== Waiting 0.1 seconds before next scan ===\n");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
