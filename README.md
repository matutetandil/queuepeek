# rabbitpeek

A terminal UI for inspecting message queues, built with Rust and ratatui.

## Overview

rabbitpeek connects to message broker management APIs and lets you browse queues and read messages without consuming them. The interface follows a wizard-style drill-down flow: select a profile, pick a queue, browse messages, and inspect individual message detail — all from the terminal.

Messages are peeked using `ack_requeue_true`, so the queue state is never modified.

## Features

- Wizard-style drill-down navigation: Profiles -> Queues -> Messages -> Message Detail
- Non-destructive message peek — messages are never acknowledged or removed
- JSON and XML auto-detection with pretty-print toggle
- Clipboard support for copying payload and headers
- 5 built-in color themes with live preview picker
- Auto-refresh queue list every 5 seconds
- Queue filtering by name
- Message filtering within a queue
- Queue publish/deliver rate display
- Message position indicator in detail view
- Fetch count picker popup with presets
- Breadcrumb navigation in all screen headers
- Styled keybinding footers on all screens
- TLS support with client certificate authentication
- Multi-profile configuration for managing multiple environments
- Vhost and namespace picker
- Profile form with type selection (rabbitmq / kafka / mqtt)
- Cloud host auto-detection

## Supported Backends

| Backend  | Status         |
|----------|----------------|
| RabbitMQ | Full support   |
| Kafka    | Coming soon    |
| MQTT     | Coming soon    |

RabbitMQ uses the Management HTTP API. Kafka and MQTT backends are stubbed and will be implemented in future releases.

## Prerequisites

- Rust 1.70 or newer
- RabbitMQ with the Management Plugin enabled:
  ```bash
  rabbitmq-plugins enable rabbitmq_management
  ```

## Installation

Install with Cargo:

```bash
cargo install rabbitpeek
```

Or build from source:

```bash
git clone https://github.com/matutedenda/rabbitpeek.git
cd rabbitpeek
cargo build --release
./target/release/rabbitpeek
```

## Configuration

rabbitpeek reads its configuration from `~/.config/rabbitpeek/config.toml`.

```toml
default_profile = "local"

[profiles.local]
type     = "rabbitmq"
host     = "localhost"
port     = 15672
username = "guest"
password = "guest"
vhost    = "/"
tls      = false

[profiles.production]
type     = "rabbitmq"
host     = "rabbit.internal.company.com"
port     = 15671
username = "admin"
password = "secret"
vhost    = "/app"
tls      = true
tls_cert = "/path/to/client.crt"
tls_key  = "/path/to/client.key"
tls_ca   = "/path/to/ca.crt"
```

Each `[profiles.<name>]` block defines a named environment. The `type` field determines which backend is used (`rabbitmq`, `kafka`, or `mqtt`). The `default_profile` key sets which profile is selected on startup.

### TLS

Set `tls = true` to use HTTPS when connecting to the Management API. Provide `tls_cert`, `tls_key`, and `tls_ca` paths for mutual TLS with a client certificate.

## Usage

Start the application:

```bash
rabbitpeek
```

The wizard flow proceeds through four screens:

1. **Profile selection** — Choose a saved profile or create a new one.
2. **Queue list** — Browse queues for the active vhost. Queues auto-refresh every 5 seconds. Filter by name with `/`.
3. **Message list** — Browse fetched messages for the selected queue. Filter messages with `/`.
4. **Message detail** — View full message payload, headers, and metadata. Toggle pretty-print, copy to clipboard, scroll through the payload.

Press `Esc` or `Backspace` at any screen to go back one level.

## Keyboard Shortcuts

### Profile Screen

| Key          | Action                        |
|--------------|-------------------------------|
| `j` / Down   | Move selection down           |
| `k` / Up     | Move selection up             |
| `Enter`      | Connect with selected profile |
| `n`          | Create new profile            |
| `e`          | Edit selected profile         |
| `d`          | Delete selected profile       |
| `q` / Ctrl+C | Quit                          |

### Queue List Screen

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / Down   | Move selection down             |
| `k` / Up     | Move selection up               |
| `Enter`      | Open selected queue             |
| `/`          | Filter queues by name           |
| `r`          | Refresh queue list              |
| `t`          | Open theme picker               |
| `Esc`        | Go back to profile screen       |
| `q` / Ctrl+C | Quit                            |

### Message List Screen

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / Down   | Move selection down             |
| `k` / Up     | Move selection up               |
| `Enter`      | Open message detail             |
| `/`          | Filter messages                 |
| `n`          | Open fetch count picker         |
| `r`          | Re-fetch messages               |
| `Esc`        | Go back to queue list           |
| `q` / Ctrl+C | Quit                            |

### Message Detail Screen

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / Down   | Scroll payload down             |
| `k` / Up     | Scroll payload up               |
| `p`          | Toggle pretty-print             |
| `c`          | Copy payload to clipboard       |
| `h`          | Copy headers to clipboard       |
| `[`          | Previous message                |
| `]`          | Next message                    |
| `Esc`        | Go back to message list         |
| `q` / Ctrl+C | Quit                            |

## Tech Stack

- [ratatui](https://github.com/ratatui-org/ratatui) — Terminal UI rendering
- [crossterm](https://github.com/crossterm-rs/crossterm) — Cross-platform terminal control
- [tokio](https://github.com/tokio-rs/tokio) — Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client for Management API
- [serde / toml](https://github.com/toml-rs/toml) — Configuration parsing
- [arboard](https://github.com/1Password/arboard) — Clipboard support

## License

MIT
