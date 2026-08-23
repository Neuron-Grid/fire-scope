# fire-scope

`fire-scope` は、国コードまたはAS番号に対応するIPv4/IPv6 CIDRを取得・集約し、TXTまたはnftables形式で出力するCLIです。国別CIDRとAS別CIDRの重複部分も計算できます。

## 特徴

- AFRINIC、LACNIC、RIPE NCC、APNIC、ARINの委任統計から国別CIDRを生成
- RIPEstatとARIN RDAPからASの発表プレフィックスを取得
- CIDRの重複排除と集約により、決定的な順序で出力
- RIR取得のリトライ、Full Jitter付き指数バックオフ、部分失敗ポリシーを設定可能
- HTTPレスポンスをストリーミング処理し、RIRは32 MiB、JSONは8 MiBに制限
- カレントディレクトリへTXTまたはnftables形式でatomic上書き

## 動作条件

- Rust 1.85以降
- 外部のRIR、RIPEstat、ARIN RDAPへHTTPS接続できる環境
- 実行時のカレントディレクトリに対する書込権限

## インストール

crates.ioからインストールする場合、任意のディレクトリで実行します。

```zsh
cargo install fire-scope
```

ソースからインストールする場合、リポジトリルートで実行します。

```zsh
cargo install --path "."
```

## CLI

バージョン0.2.0ではCLIが破壊的に変更されています。旧 `-c`、`-a`、`-o` 構文は使用できません。

```text
fire-scope [GLOBAL OPTIONS] <COMMAND>

Commands:
  list country <COUNTRY_CODE>...
  list asn <AS_NUMBER>...
  overlap --country <COUNTRY_CODE>... --asn <AS_NUMBER>...
```

国コードは2〜3文字のASCII英字だけを受け付け、内部で大文字化します。AS番号は `u32` の範囲で指定します。

### 国別リスト

インストール後、出力先にするディレクトリで実行します。

```zsh
fire-scope list country jp us
```

### AS別リスト

```zsh
fire-scope list asn 1234 65000
```

### 国別リストとAS別リストの重複

指定した国のCIDR集合とASのCIDR集合をそれぞれ結合してから、両集合の重複部分を出力します。

```zsh
fire-scope overlap --country jp us --asn 1234 65000
```

### グローバルオプション

| オプション | 説明 | 既定値 |
|---|---|---|
| `-f, --format <txt\|nft>` | 出力形式 | `txt` |
| `--http-timeout-secs <SEC>` | HTTPリクエスト全体のタイムアウト秒数。0は不可 | `20` |
| `--connect-timeout-secs <SEC>` | HTTP接続のタイムアウト秒数。0は不可 | `10` |
| `-d, --debug` | デバッグ情報をstderrへ出力 | 無効 |
| `-h, --help` | ヘルプを表示 | - |
| `-V, --version` | バージョンを表示 | - |

### RIR取得オプション

`list country` と `overlap` で使用できます。

| オプション | 説明 | 既定値 |
|---|---|---|
| `--rir-attempts <N>` | RIRごとのHTTP試行回数。0は不可 | `6` |
| `--max-backoff-secs <SEC>` | Full Jitter付き指数バックオフの上限秒数。0は不可 | `16` |
| `--continue-on-partial` | 一部のRIR取得に失敗しても成功分で続行 | 無効 |

既定では、1件でもRIR取得に失敗するとエラー終了します。`--continue-on-partial` 指定時は警告を出し、取得できたRIRだけで続行します。全RIR取得失敗時は常にエラー終了します。

```zsh
fire-scope list country jp us --rir-attempts 3 --max-backoff-secs 8 --continue-on-partial
```

### AS取得オプション

`list asn` と `overlap` で使用できます。

| オプション | 説明 | 既定値 |
|---|---|---|
| `-C, --concurrency <N>` | AS問い合わせの同時実行数。`1..=64` | `5` |

```zsh
fire-scope list asn 1234 65000 -C 10
```

`list asn` は取得に成功したASのファイルを書き、失敗したASをstderrへまとめて報告して非0で終了します。`overlap` はAS取得が1件でも失敗すると重複ファイルを書かず、非0で終了します。

## 情報の取得元

国別リストは以下のRIR委任統計を使用します。

- [AFRINIC](https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest)
- [LACNIC](https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest)
- [RIPE NCC](https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest)
- [APNIC](https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest)
- [ARIN](https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest)

AS別リストは次の規則で取得します。

- RIPEstat Announced Prefixes APIが成功した場合、ARIN RDAP OriginASの成功分もbest-effortで併合
- RIPEstatが失敗した場合、ARIN RDAP OriginASを必須fallbackとして使用
- RIPEstatとARINの両方が失敗した場合、そのASの取得を失敗として扱う

ARINのOriginASデータは地域・登録状況により網羅的でない場合があります。

## 出力

出力先は実行時のカレントディレクトリです。同名ファイルは一時ファイルへの書込後に置換します。

| コマンド | IPv4出力例 | IPv6出力例 |
|---|---|---|
| `list country jp` | `IPv4_JP.txt` | `IPv6_JP.txt` |
| `list asn 1234` | `AS_1234_IPv4.txt` | `AS_1234_IPv6.txt` |
| `overlap --country jp --asn 1234` | `overlap_JP_1234_IPv4.txt` | `overlap_JP_1234_IPv6.txt` |

`nft` 指定時は拡張子が `.nft` になり、ファイル名から生成した識別子を `define` 名として使用します。重複がないIP familyや、取得プレフィックスがないASのIP familyについてはファイルを生成しません。

各ファイルの先頭には生成日時、国コード、AS番号をコメントとして記録します。TXTは1行1CIDR、nftables形式は以下の構文です。

```zsh
fire-scope -f nft list country jp
```

```nft
define IPv4_JP = {
    192.0.2.0/24,
    198.51.100.0/24
}
```

生成ファイルを配置した後、nftables設定から読み込みます。

```nft
include "/etc/nftables/IPv4_JP.nft"
include "/etc/nftables/IPv6_JP.nft"

table inet filter {
    chain input {
        type filter hook input priority 0;
        ip saddr $IPv4_JP accept
        ip6 saddr $IPv6_JP accept
    }
}
```

## 終了コード

- `0`: 処理と必要な出力が完了
- 非0: 引数不正、ネットワーク/HTTP失敗、厳格モードでのRIR部分失敗、全RIR失敗、AS取得失敗、解析失敗、ファイル書込失敗など

## セキュリティと制限

- 国コードを入力境界で検証し、ファイル名とnftables識別子もサニタイズします。
- HTTPの総合タイムアウトと接続タイムアウトを設定します。
- `Content-Length` がない応答にもストリーミング上限を適用します。
- RPKI検証は行いません。出力は各取得元が返す委任情報・発表情報に基づきます。
- 結果は外部RIR/APIの可用性と内容に依存します。

## ライセンス

[MPL-2.0](./LICENSE.txt)
