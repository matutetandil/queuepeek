# queuepeek

A terminal UI for inspecting message queues, built with Rust and ratatui.

## Overview

queuepeek connects to message broker management APIs and lets you browse queues/topics and read messages from the terminal. The interface follows a wizard-style drill-down flow: select a profile, pick a queue, browse messages, and inspect individual message detail.

## Features

- Wizard-style drill-down navigation: Profiles -> Queues -> Messages -> Message Detail
- Multi-broker support: RabbitMQ, Kafka, and MQTT
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

| Backend  | Status       | Notes                                                    |
|----------|--------------|----------------------------------------------------------|
| RabbitMQ | Full support | Management HTTP API, non-destructive peek (ack_requeue)  |
| Kafka    | Full support | Topic listing via metadata, consumer-based message fetch |
| MQTT     | Full support | Wildcard topic discovery, subscription-based reading     |

### Backend Details

**RabbitMQ** uses the Management HTTP API. Messages are peeked using `ack_requeue_true`, so the queue state is never modified.

**Kafka** connects via the native protocol using `librdkafka`. Topics are discovered from cluster metadata, and messages are consumed from tail offsets using ephemeral consumer groups with auto-commit disabled.

**MQTT** connects via the MQTT protocol. Topics are discovered by subscribing to the wildcard topic `#`, or you can pre-configure specific topics in your profile. Note: MQTT subscriptions consume messages — there is no non-destructive peek. A warning is displayed in the UI.

## Prerequisites

- Rust 1.70 or newer
- CMake (for building the bundled librdkafka used by the Kafka backend)
- For RabbitMQ: Management Plugin enabled (`rabbitmq-plugins enable rabbitmq_management`)

## Installation

Install with Cargo:

```bash
cargo install queuepeek
```

Or build from source:

```bash
git clone https://github.com/matutedenda/queuepeek.git
cd queuepeek
cargo build --release
./target/release/queuepeek
```

## Configuration

queuepeek reads its configuration from `~/.config/queuepeek/config.toml`.

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

[profiles.my-kafka]
type     = "kafka"
host     = "kafka.example.com"
port     = 9092
username = "admin"
password = "secret"

[profiles.my-mqtt]
type     = "mqtt"
host     = "mqtt.example.com"
port     = 1883
username = ""
password = ""
topics   = ["sensors/#", "devices/status"]
```

Each `[profiles.<name>]` block defines a named environment. The `type` field determines which backend is used (`rabbitmq`, `kafka`, or `mqtt`). The `default_profile` key sets which profile is selected on startup.

### MQTT Topics

For MQTT profiles, you can optionally specify a `topics` array to pre-configure which topics to monitor. If omitted, queuepeek subscribes to the wildcard topic `#` and discovers topics automatically (limited to a 3-second discovery window).

### TLS

Set `tls = true` to enable TLS. Provide `tls_cert`, `tls_key`, and `tls_ca` paths for mutual TLS with a client certificate.

- **RabbitMQ**: Uses HTTPS for the Management API
- **Kafka**: Uses SASL_SSL with PLAIN mechanism
- **MQTT**: Uses mqtts:// (MQTT over TLS)

## Usage

Start the application:

```bash
queuepeek
```

The wizard flow proceeds through four screens:

1. **Profile selection** — Choose a saved profile or create a new one.
2. **Queue list** — Browse queues/topics for the active namespace. Queues auto-refresh every 5 seconds. Filter by name with `/`.
3. **Message list** — Browse fetched messages for the selected queue/topic. Filter messages with `/`.
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
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client for RabbitMQ Management API
- [rdkafka](https://github.com/fede1024/rust-rdkafka) — Kafka client (librdkafka wrapper)
- [rumqttc](https://github.com/bytebeamio/rumqtt) — MQTT client
- [serde / toml](https://github.com/toml-rs/toml) — Configuration parsing
- [arboard](https://github.com/1Password/arboard) — Clipboard support

## License

MIT
