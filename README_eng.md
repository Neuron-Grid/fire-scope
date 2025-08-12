## fire-scope
This CLI application is used to retrieve the latest address allocation files provided by each Regional Internet Registry (RIR) and output the IPv4/v6 address blocks corresponding to the specified country code to a text file.
It can also retrieve the specified AS number and also output it to a text file.

## Features
- Download latest data from multiple RIRs (AFRINIC, LACNIC, RIPE, APNIC, ARIN)
- Up to 10 retries and exponential backoff + random sleep to attempt stable acquisition
- Filter by country code and output unduplicated subnet lists (IPv4 / IPv6)
- Fast download and processing using Tokio
- Output files are automatically generated as IPv4_XX.txt / IPv6_XX.txt (where XX is the country code)

## Output format.
- IPv4_XX.txt / IPv6_XX.txt
  - XX is an optional country code.
- One subnet is listed per line.
- The first line contains the date and time of execution.

## Information Sources
- When specifying the `-c` option, data is retrieved from the following
  - [AFRINIC](https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest)
  - [LACNIC](https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest)
  - [RIPE NCC](https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest)
  - [APNIC](https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest)
  - [ARIN](https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest)

- When specifying the `-a` option, data is retrieved from the following
  - RIPEstat Announced Prefixes API (primary)
  - ARIN RDAP OriginAS networks (fallback)


## Usage
### Installation
```bash
$ cargo install fire-scope
```
### Example
- **Note**
  - Either `-c` or `-a` must be specified.
  - If not specified, an error occurs.
```bash
$ fire-scope -c jp us
```

```bash
$ fire-scope -a 0000 1234
```

```bash
$ fire-scope -c jp us -a 0000 1234 -o
```

### Options
- `-c`: Specify one or more country codes.
- `-a`: Specify one or more AS numbers.
- `-h`: Display help.
- `-v`: Display version.
- `-o`: Output the overlapping IP addresses among the IPv4/v6 addresses of the specified country code(s) and AS number(s).
  - By design, both `-c` and `-a` must be specified.

- Tuning options
  - `--format {txt|nft}`: Output format (default: `txt`).
  - `--max-retries <N>`: HTTP retry attempts (default: 6).
  - `--max-backoff-sec <SEC>`: Cap for exponential backoff per retry (default: 16).
  - `--http-timeout-secs <SEC>`: Overall HTTP timeout (default: 20).
  - `--connect-timeout-secs <SEC>`: Connect timeout (default: 10).
  - `--concurrency <N>`: Max concurrent AS queries (default: 5).
  - `--continue-on-partial`: Continue processing with successfully downloaded RIR files even if some fail (default: off = strict).

### Partial failure behavior
- By default, the command fails if any RIR file download fails.
- With `--continue-on-partial`, it proceeds using successfully downloaded files (and prints warnings).
- If all downloads fail, it always exits with an error.

### nftables usage
1) Generate nft format files
```bash
fire-scope -c jp --format nft
```
This creates `IPv4_JP.nft` / `IPv6_JP.nft` with
`define IPv4_JP = { ... }` / `define IPv6_JP = { ... }`.

2) Include and reference in nftables
```nft
include "/etc/nftables/IPv4_JP.nft"
include "/etc/nftables/IPv6_JP.nft"

table inet filter {
  chain input {
    type filter hook input priority 0;
    ip  saddr $IPv4_JP accept
    ip6 saddr $IPv6_JP accept
  }
}
```
Place the generated files in an appropriate path and `include` them.

### Exit codes
- 0: Success
- Non-zero: Invalid input (e.g., missing `-c`/`-a`), network/HTTP failure (strict mode), no usable RIR files, file write error, etc.

## Notes
- Output files are always overwritten if they already exist.
- If neither `-c` nor `-a` are specified, the command exits with a non-zero code.

## Security
- Filenames and nft define names are sanitized to alphanumerics/underscore to avoid path traversal and injection.
- `-c/--country` accepts only alphabetic ISO-like codes (length 2–3).
- HTTP client enforces overall and connect timeouts and sets a descriptive User-Agent.
- RIR downloads are read in streaming mode and rejected once size exceeds 32 MiB (even if `Content-Length` is missing).
- RIPEstat/ARIN JSON responses are streamed and limited to 8 MiB.

## Known limitations
- AS prefixes are fetched primarily from RIPEstat, with ARIN RDAP as a fallback.
- RPKI validation is not enabled by default (internal code exists, CLI not exposed yet).
- Availability depends on external APIs/files; tune with `--max-retries` and `--max-backoff-sec` if needed.

## Requirements
- Use the latest stable Rust toolchain with Edition 2024 support. `rustup update stable` is recommended.

## Recommended options
- Faster yet stable fetch:
  - `fire-scope -c jp us --max-retries 3 --max-backoff-sec 8 --continue-on-partial`
- Speed up AS queries with concurrency:
  - `fire-scope -a 1234 65000 -C 10`

## License
[MPL-2.0](./LICENSE.txt)
