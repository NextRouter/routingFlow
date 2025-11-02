# Prometheus NIC Monitor

Prometheus から複数のソースのメトリクスを収集し、NIC ごとに統計情報を表示する Rust プログラムです。

## 機能

1. **TCP Traffic Scan データの収集**

   - `{job="tcp-traffic-scan",__name__=~"tcp_traffic_scan_tcp_bandwidth_avg_bps"}`
   - NIC の`interface`ラベルでグループ化

2. **Local Packet Dump データの収集と集計**
   - `{job="lcoalpacketdump",__name__=~"network_ip_tx_bps|network_ip_rx_bps"}`
   - `localhost:32599/status` から IP-NIC マッピングを取得
   - 各 IP のトラフィックを NIC ごとに集計

## 前提条件

- Rust 1.70 以上
- Prometheus (localhost:9090)
- Status API (localhost:32599)

## ビルドと実行

```bash
# ビルド
cargo build --release

# 実行
cargo run
```

## 出力例

```
Fetching status mappings from localhost:32599...

NIC Configuration:
  LAN: eth2
  WAN0: eth0 (wan0)
  WAN1: eth1 (wan1)

Fetching TCP bandwidth data from Prometheus...
Fetching network traffic data from Prometheus...

=== NIC Statistics ===

Interface: eth0
  TCP Bandwidth (avg): 1234567.00 bps (1.23 Mbps)
  TX (total): 2345678.00 bps (2.35 Mbps)
  RX (total): 3456789.00 bps (3.46 Mbps)
  Total Traffic: 5802467.00 bps (5.80 Mbps)

Interface: eth1
  TCP Bandwidth (avg): 987654.00 bps (0.99 Mbps)
  TX (total): 1876543.00 bps (1.88 Mbps)
  RX (total): 2765432.00 bps (2.77 Mbps)
  Total Traffic: 4641975.00 bps (4.64 Mbps)

Interface: eth2
  TCP Bandwidth (avg): 456789.00 bps (0.46 Mbps)
  TX (total): 567890.00 bps (0.57 Mbps)
  RX (total): 678901.00 bps (0.68 Mbps)
  Total Traffic: 1246791.00 bps (1.25 Mbps)
```

## 実装の詳細

### データフロー

1. Status API から NIC 設定と IP マッピングを取得
2. Prometheus から TCP bandwidth metrics を取得（interface ラベル付き）
3. Prometheus から network traffic metrics を取得（IP ラベル付き）
4. IP を NIC にマッピングしてトラフィックを集計
5. NIC ごとに統計情報を表示

### 依存クレート

- `tokio`: 非同期ランタイム
- `reqwest`: HTTP クライアント
- `serde`: JSON シリアライゼーション
- `anyhow`: エラーハンドリング
- `urlencoding`: URL エンコーディング
