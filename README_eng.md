# fire-scope

`fire-scope` is a CLI that retrieves and aggregates IPv4/IPv6 CIDRs for country codes or AS numbers and writes them in TXT or nftables format. It can also calculate the overlap between country and AS CIDR sets.

## Features

- Builds country CIDR lists from the delegated statistics published by AFRINIC, LACNIC, RIPE NCC, APNIC, and ARIN
- Fetches announced AS prefixes from RIPEstat and ARIN RDAP
- Deduplicates and aggregates CIDRs and writes them in deterministic order
- Configurable RIR retries, exponential backoff with Full Jitter, and partial-failure policy
- Streams HTTP responses with a 32 MiB limit for RIR files and an 8 MiB limit for JSON
- Atomically overwrites TXT or nftables output in the current working directory

## Requirements

- Rust 1.85 or later
- HTTPS access to the external RIR, RIPEstat, and ARIN RDAP services
- Write permission for the current working directory at runtime

## Installation

Run this from any directory to install from crates.io.

```zsh
cargo install fire-scope
```

Run this from the repository root to install from source.

```zsh
cargo install --path "."
```

## CLI

Version 0.2.0 introduces a breaking CLI change. The old `-c`, `-a`, and `-o` syntax is not supported.

```text
fire-scope [GLOBAL OPTIONS] <COMMAND>

Commands:
  list country <COUNTRY_CODE>...
  list asn <AS_NUMBER>...
  overlap --country <COUNTRY_CODE>... --asn <AS_NUMBER>...
```

Country codes must contain two or three ASCII letters and are normalized to uppercase. AS numbers must fit in a `u32`.

### Country lists

After installation, run the command from the directory where the files should be written.

```zsh
fire-scope list country jp us
```

### AS lists

```zsh
fire-scope list asn 1234 65000
```

### Country and AS overlap

The command unions the selected country CIDRs, unions the selected AS CIDRs, and writes the overlap of the two sets.

```zsh
fire-scope overlap --country jp us --asn 1234 65000
```

### Global options

| Option | Description | Default |
|---|---|---|
| `-f, --format <txt\|nft>` | Output format | `txt` |
| `--http-timeout-secs <SEC>` | Overall HTTP request timeout in seconds; zero is rejected | `20` |
| `--connect-timeout-secs <SEC>` | HTTP connection timeout in seconds; zero is rejected | `10` |
| `-d, --debug` | Write debug diagnostics to stderr | disabled |
| `-h, --help` | Display help | - |
| `-V, --version` | Display the version | - |

### RIR options

These options apply to `list country` and `overlap`.

| Option | Description | Default |
|---|---|---|
| `--rir-attempts <N>` | Number of HTTP attempts per RIR; zero is rejected | `6` |
| `--max-backoff-secs <SEC>` | Maximum exponential-backoff delay with Full Jitter; zero is rejected | `16` |
| `--continue-on-partial` | Continue with the successfully downloaded RIR files | disabled |

By default, any RIR download failure causes the command to fail. With `--continue-on-partial`, the command warns and continues with the RIR files it obtained. A failure to download every RIR always causes the command to fail.

```zsh
fire-scope list country jp us --rir-attempts 3 --max-backoff-secs 8 --continue-on-partial
```

### AS options

This option applies to `list asn` and `overlap`.

| Option | Description | Default |
|---|---|---|
| `-C, --concurrency <N>` | Concurrent AS queries; accepted range is `1..=64` | `5` |

```zsh
fire-scope list asn 1234 65000 -C 10
```

`list asn` writes files for successful AS queries, reports all failed AS numbers to stderr, and then exits non-zero. `overlap` writes no overlap file and exits non-zero if any AS query fails.

## Data sources

Country lists use the following RIR delegated statistics files:

- [AFRINIC](https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest)
- [LACNIC](https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest)
- [RIPE NCC](https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest)
- [APNIC](https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest)
- [ARIN](https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest)

AS lists use these rules:

- When the RIPEstat Announced Prefixes API succeeds, any successful ARIN RDAP OriginAS result is also merged on a best-effort basis
- When RIPEstat fails, ARIN RDAP OriginAS is the required fallback
- When both RIPEstat and ARIN fail, the AS query fails

ARIN OriginAS data may not be comprehensive for every region or registration.

## Output

Files are written to the current working directory. An existing file is replaced only after its temporary replacement has been written.

| Command | Example IPv4 output | Example IPv6 output |
|---|---|---|
| `list country jp` | `IPv4_JP.txt` | `IPv6_JP.txt` |
| `list asn 1234` | `AS_1234_IPv4.txt` | `AS_1234_IPv6.txt` |
| `overlap --country jp --asn 1234` | `overlap_JP_1234_IPv4.txt` | `overlap_JP_1234_IPv6.txt` |

With `nft`, the extension is `.nft`, and the sanitized file stem is used as the `define` name. No file is created for an IP family with no overlap or for an AS IP family with no retrieved prefixes.

Each file starts with comments containing its generation time, country code, and AS number. TXT output contains one CIDR per line. nftables output uses this form:

```zsh
fire-scope -f nft list country jp
```

```nft
define IPv4_JP = {
    192.0.2.0/24,
    198.51.100.0/24
}
```

After placing the generated files at the required path, include them from the nftables configuration.

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

## Exit codes

- `0`: Processing and all required output completed
- Non-zero: Invalid arguments, network or HTTP failure, partial RIR failure in strict mode, total RIR failure, AS query failure, parse failure, file-write failure, or another fatal error

## Security and limitations

- Country codes are validated at the input boundary, and file names and nftables identifiers are sanitized.
- The HTTP client enforces overall and connection timeouts.
- Streaming size limits also apply when a response has no `Content-Length` header.
- RPKI validation is not performed. Output is based on the delegation and announcement data returned by the configured sources.
- Results depend on the availability and contents of external RIR files and APIs.

## License

[MPL-2.0](./LICENSE.txt)
