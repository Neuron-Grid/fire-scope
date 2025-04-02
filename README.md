## fire-scope
本ツールは、各地域インターネットレジストリ (RIR) が提供する最新のアドレス割り当てファイルを取得し、指定された国コードに合致するIPv4/v6アドレスブロックをテキストファイルにまとめて出力するためのCLIアプリです。

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
  - `whois.radb.net`

## 使い方
### インストール
```bash
$ cargo install fire-scope
```

### 実行方法
- `-c`か`-a`の引数がない場合はエラーが発生します。
```bash
$ fire-scope -c jp us
```

```bash
$ fire-scope -a AS0000 AS1234
```

### オプション
- `-c` : 国コードを指定します。複数指定可能です。
- `-a` : AS番号を指定します。複数指定可能です。
- `-h` : ヘルプを表示します。
- `-v` : バージョンを表示します。
- `-m` : ファイル出力モードの選択できます。「追記」または「上書き」を選択できます。指定しなかった場合は「上書き」が選択されます。
  - `append` : 追記モード
  - `overwrite` : 上書きモード

- **注意事項**<br>
`-c`か`-a`のどちらか一方は必ず指定してください。
指定しなかった場合はエラーが発生します。

## ライセンス
[MIT License](./LICENSE)