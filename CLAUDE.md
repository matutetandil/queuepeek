# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

queuepeek — a TUI application for inspecting message queues from the terminal. It uses a wizard-style drill-down flow (Profiles -> Queues -> Messages -> Message Detail) and supports multiple broker backends. RabbitMQ is fully implemented using the Management HTTP API with `ack_requeue_true` for non-destructive peeking. Kafka uses `rdkafka` (librdkafka) for native protocol access. MQTT uses `rumqttc` for subscription-based reading.

## Build and Run

```bash
cargo build                  # debug build (requires cmake for librdkafka)
cargo run                    # run in debug mode
cargo build --release        # optimized release build
./target/release/queuepeek   # run release binary
```

## Architecture

- `src/main.rs` — Entry point. Initializes the terminal, sets up crossterm raw mode, and runs the main event loop.
- `src/app.rs` — Central state machine. Holds current screen, selected profile, queue, message, and UI state. All screen transitions go through here.
- `src/config.rs` — Loads and saves `~/.config/queuepeek/config.toml`. Defines the `Profile` struct with `type` field (rabbitmq / kafka / mqtt), host, port, credentials, vhost, TLS settings, and optional `topics` list (for MQTT).
- `src/backend/mod.rs` — Defines the `Backend` trait. All broker implementations must satisfy this trait.
- `src/backend/rabbitmq.rs` — RabbitMQ implementation. Uses the Management HTTP API. Implements queue listing, message fetching, and vhost enumeration.
- `src/backend/kafka.rs` — Kafka implementation. Uses rdkafka for topic listing (via cluster metadata), message consumption (ephemeral consumer groups), and broker info.
- `src/backend/mqtt.rs` — MQTT implementation. Uses rumqttc for topic discovery (wildcard `#` subscription or pre-configured topics list) and message reading via subscriptions. Note: MQTT consumes messages on read (no peek).
- `src/ui/` — One module per screen. Each module owns rendering logic for that screen and returns keybinding hints for the footer.
  - `profiles.rs` — Profile selection list and profile form (create/edit).
  - `queues.rs` — Queue list with filter input, auto-refresh indicator, and rate columns.
  - `messages.rs` — Message list with filter input and fetch count picker popup.
  - `detail.rs` — Message detail with scrollable payload, headers panel, pretty-print toggle, and clipboard actions.
  - `theme.rs` — Theme picker popup with color swatches and live preview.

## Key Patterns

- Screen navigation follows a wizard flow. The `App` state machine holds a `Screen` enum with variants for each step. `Esc` or `Backspace` pops one level back.
- The `Backend` trait abstracts broker-specific logic. New backends implement `list_queues`, `peek_messages`, and `list_namespaces` (vhosts for RabbitMQ, single "default" for Kafka/MQTT).
- Background I/O runs in threads that communicate back to the main loop via `std::sync::mpsc` channels. The event loop polls both crossterm input events and the mpsc receiver on each tick.
- Auto-refresh for the queue list is driven by a timer tracked in `App`. Every 5 seconds the app dispatches a background fetch and updates queue data when results arrive.
- Pretty-print state is per-message and toggled with `p`. Detection runs on first open: if the payload parses as valid JSON or XML it is formatted automatically.
- The theme is stored in `App` and passed to all UI modules on each render. The theme picker shows a live preview by re-rendering with the candidate theme before the user confirms.

## Config Path

```
~/.config/queuepeek/config.toml
```

Example structure:

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
```
