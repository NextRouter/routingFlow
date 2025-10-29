# Routing Flow - Network Bandwidth Monitor

A Rust application that monitors network bandwidth usage by comparing Prometheus metrics for estimated and actual bandwidth, and identifies the top IP addresses consuming bandwidth when thresholds are exceeded.

## 概要

このプロジェクトは以下の機能を提供します：

1. **Prometheus メトリクスの取得**

   - `tcp_traffic_scan_tcp_bandwidth_avg_bps` - TCP 帯域の推測値
   - `network_ip_rx_bps_total` / `network_ip_tx_bps_total` - 実際の受信/送信帯域
   - `network_ip_rx_bps` / `network_ip_tx_bps` - IP アドレスごとの帯域

2. **帯域比較**

   - 推測された帯域と実際の帯域（RX + TX）を比較
   - 実際の帯域が推測値を超えた場合に検出

3. **IP アドレス分析**

   - 帯域が超過した場合、その NIC で最も多くの帯域を使用している IP アドレスを特定
   - 受信（RX）と送信（TX）の両方向で最上位の IP を報告

4. **ルーティング情報の統合**
   - `/status` エンドポイントから IP アドレスと WAN インターフェースのマッピングを取得
   - `nic.json`から NIC 設定を読み込み

## プロジェクト構造

```
.
├── Cargo.toml          # プロジェクト依存関係
├── nic.json            # NIC設定ファイル
├── run.sh              # ビルド・実行スクリプト
└── src/
    ├── main.rs         # エントリーポイント
    ├── config.rs       # 設定管理
    ├── prometheus.rs   # Prometheusクライアント
    └── monitor.rs      # 帯域監視ロジック
```

## 設定

### nic.json

NIC の設定を定義します：

```json
{
  "lan": "eth2",
  "wan0": "eth0",
  "wan1": "eth1"
}
```

### エンドポイント

デフォルトで以下のエンドポイントを使用します：

- Prometheus: `http://localhost:9090`
- Status API: `http://localhost:32599/status`

これらは `src/config.rs` で変更できます。

## 使用方法

### ビルドと実行

```bash
./run.sh
```

または手動で：

```bash
cargo build --release
./target/release/routing_flow
```

### 出力例

```
=== Bandwidth Monitoring Report ===

Network Configuration:
  LAN: eth2
  WAN0: eth0
  WAN1: eth1

IP Mappings:
  10.40.0.2 -> wan1
  10.40.0.3 -> wan1
  10.40.0.4 -> wan1

Bandwidth Comparison:

  Interface: eth0
    Estimated Bandwidth: 1000000000.00 bps
    Actual RX: 450000000.00 bps
    Actual TX: 320000000.00 bps
    Actual Total: 770000000.00 bps
    Exceeded: NO ✓

  Interface: eth1
    Estimated Bandwidth: 500000000.00 bps
    Actual RX: 380000000.00 bps
    Actual TX: 240000000.00 bps
    Actual Total: 620000000.00 bps
    Exceeded: YES ⚠️

    Finding top IP addresses...
      Top RX IP: 10.40.0.2 (250000000.00 bps)
      Top TX IP: 10.40.0.2 (180000000.00 bps)

=== End of Report ===
```

## 依存関係

- `tokio` - 非同期ランタイム
- `reqwest` - HTTP クライアント
- `serde` / `serde_json` - JSON シリアライゼーション
- `anyhow` - エラーハンドリング
- `chrono` - 日時処理

## アーキテクチャ

### モジュール

1. **config.rs**

   - NIC 設定の読み込み
   - ステータス API レスポンスの定義
   - IP アドレスと WAN のマッピング

2. **prometheus.rs**

   - Prometheus API クライアント
   - メトリクスクエリとパース
   - 帯域データの取得

3. **monitor.rs**

   - 帯域比較ロジック
   - 超過検出
   - トップ IP アドレスの特定

4. **main.rs**
   - アプリケーションのエントリーポイント
   - モジュールの統合

## ライセンス

MIT
