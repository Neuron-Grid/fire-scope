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

## Notes
- Output files are always overwritten if they already exist.
- If neither `-c` nor `-a` are specified, the command exits with a non-zero code.

## Security
- Filenames and nft define names are sanitized to alphanumerics/underscore to avoid path traversal and injection.
- `-c/--country` accepts only alphabetic ISO-like codes (length 2–3).
- HTTP client enforces overall and connect timeouts and sets a descriptive User-Agent.
- RIR downloads are rejected if `Content-Length` exceeds 32 MiB.

## License
[MPL-2.0](./LICENSE.txt)
