# Routing Flow Monitor - 開発メモ

## 実装の詳細

### データフロー

1. **設定の読み込み** (`config.rs`)

   - `nic.json` から NIC 設定を読み込む
   - Prometheus と Status API の URL を設定

2. **ステータス取得** (`monitor.rs::fetch_status`)

   - `http://localhost:32599/status` から IP マッピング情報を取得
   - IP アドレスと WAN インターフェースの対応関係を取得

3. **帯域推測値の取得** (`prometheus.rs::get_tcp_bandwidth_avg`)

   - `tcp_traffic_scan_tcp_bandwidth_avg_bps` メトリクスを取得
   - 各インターフェースの推測帯域を取得

4. **実際の帯域取得** (`prometheus.rs::get_all_network_totals`)

   - `network_ip_rx_bps_total{nic="wan0"}` - WAN0 の受信帯域
   - `network_ip_rx_bps_total{nic="wan1"}` - WAN1 の受信帯域
   - `network_ip_tx_bps_total{nic="wan0"}` - WAN0 の送信帯域
   - `network_ip_tx_bps_total{nic="wan1"}` - WAN1 の送信帯域

5. **帯域比較** (`monitor.rs::compare_bandwidth`)

   - 推測帯域 vs 実際の帯域（RX + TX）
   - 実際の帯域が推測値を超えているかチェック

6. **トップ IP の特定** (`monitor.rs::find_top_ips`)
   - 帯域超過時のみ実行
   - `network_ip_rx_bps{nic="wanX"}` から受信トップ IP を取得
   - `network_ip_tx_bps{nic="wanX"}` から送信トップ IP を取得

### Prometheus クエリ例

```promql
# 帯域推測値
tcp_traffic_scan_tcp_bandwidth_avg_bps{instance="localhost:59121",interface="eth0",job="tcp-traffic-scan"}

# 実際の総帯域
network_ip_rx_bps_total{instance="localhost:59122",job="lcoalpacketdump",nic="wan0"}
network_ip_tx_bps_total{instance="localhost:59122",job="lcoalpacketdump",nic="wan0"}

# IP ごとの帯域
network_ip_rx_bps{nic="wan0"}
network_ip_tx_bps{nic="wan0"}
```

### デフォルトルール

- IP マッピングに記載されていない IP アドレスは `wan0` に割り当てられていると仮定
- NIC の数と割り当ては `nic.json` で定義

## テスト環境のセットアップ

### 前提条件

以下のサービスが起動している必要があります：

1. **Prometheus** (`localhost:9090`)

   - TCP トラフィックスキャン メトリクス
   - ネットワーク IP 帯域メトリクス

2. **ルーティングステータス API** (`localhost:32599`)
   - `/status` エンドポイントで IP マッピングを提供

### 動作確認

```bash
# Prometheus が起動しているか確認
curl http://localhost:9090/api/v1/query?query=up

# Status API が起動しているか確認
curl http://localhost:32599/status

# アプリケーションを実行
./run.sh
```

## 拡張可能性

### 追加機能のアイデア

1. **継続的モニタリング**

   - 定期的に実行するループの追加
   - アラート機能

2. **メトリクスのエクスポート**

   - 結果を Prometheus にエクスポート
   - Grafana ダッシュボード連携

3. **詳細なレポート**

   - 時系列データの記録
   - CSV/JSON エクスポート

4. **アラート通知**
   - 帯域超過時のメール/Slack 通知
   - しきい値の設定

## トラブルシューティング

### Prometheus に接続できない

```bash
# Prometheus が起動しているか確認
curl http://localhost:9090/-/healthy

# メトリクスが存在するか確認
curl "http://localhost:9090/api/v1/query?query=tcp_traffic_scan_tcp_bandwidth_avg_bps"
```

### Status API に接続できない

```bash
# API が起動しているか確認
curl http://localhost:32599/status

# ポートが使用されているか確認
lsof -i :32599
```

### ビルドエラー

```bash
# 依存関係を更新
cargo update

# クリーンビルド
cargo clean
cargo build --release
```
