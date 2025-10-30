use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrometheusData {
    #[serde(rename = "resultType")]
    pub result_type: String,
    pub result: Vec<PrometheusResult>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PrometheusResult {
    pub metric: HashMap<String, String>,
    pub value: (f64, String),
}

#[derive(Debug, Clone)]
pub struct BandwidthMetric {
    pub interface: String,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct IpMetric {
    pub ip: String,
    pub nic: String,
    pub value: f64,
}

pub struct PrometheusClient {
    client: reqwest::Client,
    base_url: String,
}

impl PrometheusClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Query Prometheus and parse response
    async fn query(&self, query: &str) -> Result<PrometheusResponse> {
        let url = format!("{}/api/v1/query", self.base_url);

        println!("[DEBUG] Prometheus Query: {}", query);

        let response = self
            .client
            .get(&url)
            .query(&[("query", query)])
            .send()
            .await
            .context("Failed to send Prometheus query")?;

        let result: PrometheusResponse = response
            .json()
            .await
            .context("Failed to parse Prometheus response")?;

        println!("[DEBUG] Result count: {}", result.data.result.len());
        if result.data.result.is_empty() {
            println!("[DEBUG] No results found for query: {}", query);
        }

        Ok(result)
    }

    /// Get TCP bandwidth average metrics
    pub async fn get_tcp_bandwidth_avg(&self) -> Result<Vec<BandwidthMetric>> {
        let query = "tcp_traffic_scan_tcp_bandwidth_avg_bps";
        let response = self.query(query).await?;

        let metrics = response
            .data
            .result
            .into_iter()
            .filter_map(|result| {
                let interface = result.metric.get("interface")?.clone();
                let value = result.value.1.parse::<f64>().ok()?;
                Some(BandwidthMetric { interface, value })
            })
            .collect();

        Ok(metrics)
    }

    /// Get network IP RX/TX total metrics for a specific NIC
    pub async fn get_network_total(&self, nic: &str, direction: &str) -> Result<f64> {
        let query = format!("network_ip_{}_bps_total{{nic=\"{}\"}}", direction, nic);
        let response = self.query(&query).await?;

        let value = response
            .data
            .result
            .first()
            .and_then(|result| result.value.1.parse::<f64>().ok())
            .unwrap_or(0.0);

        Ok(value)
    }

    /// Get network IP RX/TX metrics by IP address for a specific NIC
    pub async fn get_network_by_ip(&self, nic: &str, direction: &str) -> Result<Vec<IpMetric>> {
        let query = format!("network_ip_{}_bps{{nic=\"{}\"}}", direction, nic);
        let response = self.query(&query).await?;

        let metrics = response
            .data
            .result
            .into_iter()
            .filter_map(|result| {
                let ip = result.metric.get("ip")?.clone();
                let nic = result.metric.get("nic")?.clone();
                let value = result.value.1.parse::<f64>().ok()?;
                Some(IpMetric { ip, nic, value })
            })
            .collect();

        Ok(metrics)
    }

    /// Get all network totals for RX and TX for a list of NICs
    pub async fn get_all_network_totals(
        &self,
        nics: &[String],
    ) -> Result<HashMap<String, (f64, f64)>> {
        let mut results = HashMap::new();

        for nic in nics {
            let rx = self.get_network_total(nic, "rx").await?;
            let tx = self.get_network_total(nic, "tx").await?;
            results.insert(nic.clone(), (rx, tx));
        }

        Ok(results)
    }
}
