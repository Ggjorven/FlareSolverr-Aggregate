# FlareSolverr Aggregate

A very simple docker container that can have multiple cloudflare solvers and aggregates them into 1 endpoint.

## Support for

- [FlareSolverr](https://github.com/Flaresolverr/Flaresolverr) (obviously)
- [Byparr](https://github.com/ThePhaseless/Byparr/)
- [CloudflareBypassForScraping](https://github.com/sarperavci/CloudflareBypassForScraping) // TODO

## Installation

Pre-built images are published to the GitHub Container Registry at
[`ghcr.io/ggjorven/flaresolverr-aggregate`](https://github.com/Ggjorven/FlareSolverr-Aggregate/pkgs/container/flaresolverr-aggregate).
Available tags: `latest` (newest release), `nightly` (latest `dev` build), and per-version tags (e.g. `0.1.0-alpha2`).

The recommended way to run is with Docker Compose. Create a `compose.yaml`:

```yaml
services:
  flaresolverr:
    image: ghcr.io/flaresolverr/flaresolverr:latest
    container_name: flaresolverr
    environment:
      - LOG_LEVEL=info
      - PORT=8192
    restart: unless-stopped

  byparr:
    image: ghcr.io/thephaseless/byparr:latest
    container_name: byparr
    environment:
      - LOG_LEVEL=info
      - PORT=8193
    restart: unless-stopped

  flaresolverr-aggregate:
    image: ghcr.io/ggjorven/flaresolverr-aggregate:latest
    container_name: flaresolverr-aggregate
    environment:
      - LOG_LEVEL=info
      - FLARESOLVERR_URL=http://flaresolverr:8192/v1
      - BYPARR_URL=http://byparr:8193/v1
    ports:
      - 8191:8191
    depends_on:
      - flaresolverr
      - byparr
    restart: unless-stopped

```

Then start it:

```sh
docker compose up -d
```

## Configuration

The container is configured through environment variables:

| Variable | Default | Description |
|---|---|---|
| `LOG_LEVEL` | `info` | Log verbosity: `debug` / `info` / `warning` / `error` |
| `PORT` | `8191` | Port the API is exposed on |
| `FLARESOLVERR_URL` | - | FlareSolverr endpoint (may be empty) |
| `BYPARR_URL` | - | Byparr endpoint (may be empty) |

## Usage

This container follows the same **FlareSolverr** standard that [FlareSolverr](https://github.com/Flaresolverr/Flaresolverr) and [Byparr](https://github.com/ThePhaseless/Byparr/) use.  
For a more complete reference see [FlareSolverr's README](https://github.com/Flaresolverr/Flaresolverr#commands).

## Contributing

Contributions are highly appreciated, please follow the [CONTRIBUTING GUIDELINES](./CONTRIBUTING.md) to make a quality contribution.

## License

This project is licensed under the **MIT LICENSE**. See [LICENSE](LICENSE.txt) for the full license text.
