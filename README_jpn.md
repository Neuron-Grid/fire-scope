## fire-scope
各地域インターネットレジストリ (RIR) が提供する最新のアドレス割り当てファイルを取得し、指定された国コードに合致するIPv4/v6アドレスブロックをテキストファイルにまとめて出力するためのCLIアプリです。
また、指定されたAS番号も取得し、同様にテキストファイルに出力できます。

## 特徴
- 複数のRIR(AFRINIC, LACNIC, RIPE, APNIC, ARIN)から最新のデータをダウンロード
- 最大10回までのリトライと指数バックオフ+ランダムスリープで安定取得を試行
- 国コードごとにフィルタし、重複のないサブネットリスト(IPv4 / IPv6)を出力
- Tokioを使用した高速なダウンロード・処理
- 出力ファイルはIPv4_XX.txt / IPv6_XX.txt (XX は国コード)として自動生成

## 出力形式
- IPv4_XX.txt / IPv6_XX.txt
  - XXは任意の国コードです。
- 1行に1つのサブネットが記載されています。
- 最初の1行目には実行日時が記載されます。

## 情報の取得元
- `-c`を指定した場合の取得元
  - [AFRINIC](https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest)
  - [LACNIC](https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest)
  - [RIPE NCC](https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest)
  - [APNIC](https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest)
  - [ARIN](https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest)

- `-a`を指定した場合の取得元
  - RIPEstat Announced Prefixes API（優先）
  - ARIN RDAP OriginAS networks（フォールバック）

## 使い方
### インストール
```bash
$ cargo install fire-scope
```

### 実行例
- **注意**
  - `-c`か`-a`のどちらか一方は必ず指定してください。
  - 指定しなかった場合はエラーが発生します。
```bash
$ fire-scope -c jp us
```

```bash
$ fire-scope -a 0000 1234
```

```bash
$ fire-scope -c jp us -a 0000 1234 -o
```

### オプション
- `-c` : 国コードを指定します。複数指定可能です。
- `-a` : AS番号を指定します。複数指定可能です。
- `-h` : ヘルプを表示します。
- `-v` : バージョンを表示します。
- `-o` : 指定された国コードとAS番号のIPv4/v6アドレスのうち、重複している部分のIPアドレスを出力します。
  - 性質上、`-c`と`-a`の両方の指定が必須事項です。

- 取得/実行の調整用オプション
  - `--format {txt|nft}`: 出力形式（既定: `txt`）
  - `--max-retries <N>`: HTTPリトライ回数（既定: 6）
  - `--max-backoff-sec <SEC>`: 指数バックオフの最大秒数（既定: 16）
  - `--http-timeout-secs <SEC>`: HTTPの総合タイムアウト秒（既定: 20）
  - `--connect-timeout-secs <SEC>`: 接続タイムアウト秒（既定: 10）
  - `--concurrency <N>`: ASクエリの同時実行数（既定: 5）
  - `--continue-on-partial`: RIRダウンロードに一部失敗しても成功分で処理を続行します（既定: 無効＝厳格）

### 一部失敗時の挙動（重要）
- 既定では、RIRファイルのダウンロードに1つでも失敗するとエラー終了します。
- `--continue-on-partial`を付けると、成功したRIRファイルのみで処理を続行します（警告を表示）。
- どちらのモードでも「全て失敗」の場合はエラー終了します。

### nftablesでの利用例
1) nft形式で出力
```bash
fire-scope -c jp --format nft
```
`IPv4_JP.nft` / `IPv6_JP.nft` が生成され、それぞれ
`define IPv4_JP = { ... }` / `define IPv6_JP = { ... }` が含まれます。

2) nftables設定へ取り込み（例）
```nft
include "/etc/nftables/IPv4_JP.nft"
include "/etc/nftables/IPv6_JP.nft"

table inet filter {
  chain input {
    type filter hook input priority 0;
    ip  saddr $IPv4_JP accept   # IPv4定義の参照
    ip6 saddr $IPv6_JP accept   # IPv6定義の参照
  }
}
```
生成ファイルを適切なパスに配置してから`include`してください。

### 終了コード
- 0: 正常終了
- 非0: 無効な引数（`-c`/`-a`未指定など）、ネットワーク/HTTP失敗（厳格モード）、RIRファイルが1つも利用不可、ファイル書込失敗 など

- **注意事項**<br>
`-c`か`-a`のどちらか一方は必ず指定してください。
指定しなかった場合はエラーで非0終了します。

- 既存の出力ファイルがある場合は常に上書きします。

## セキュリティ補足
- RIRのダウンロードはストリーミングで読み込むため、`Content-Length`ヘッダが無い場合でも32MiB超で即中断します。
- RIPEstat/ARINのJSON応答もストリーミングで読み込み、8MiBを上限に制限します。

## 既知の制限
- ASの発表プレフィックスはRIPEstatを優先し、失敗時はARIN RDAPへフォールバックします。
- 現時点ではRPKI検証はデフォルト無効です（内部コードはありますがCLI未公開）。
- 外部API/ファイルの可用性に依存します。`--max-retries`/`--max-backoff-sec`で調整可能です。

## 動作条件
- 最新の安定版Rust（Edition 2024対応）を推奨します。`rustup update stable`で更新してください。

## 推奨オプション例
- 取得安定性を保ちつつ迅速化:
  - `fire-scope -c jp us --max-retries 3 --max-backoff-sec 8 --continue-on-partial`
- AS問い合わせを並列に高速化:
  - `fire-scope -a 1234 65000 -C 10`

## ライセンス
[MPL-2.0](./LICENSE.txt)
