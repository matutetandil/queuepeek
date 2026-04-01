# rabbitpeek

A terminal UI for inspecting RabbitMQ queues, built with Go and the Charmbracelet TUI stack.

## Overview

rabbitpeek connects to RabbitMQ's Management HTTP API and presents a split-view interface for browsing queues and reading messages without consuming them. Messages are peeked using `ack_requeue_true`, leaving the queue state unchanged.

The interface follows a sidebar + message panel layout: the left panel lists queues for the active vhost, and the right panel displays the selected queue's messages with JSON pretty-printing and horizontal scrolling.

## Features

- Dark terminal theme with clear visual hierarchy
- Multi-profile configuration for managing multiple RabbitMQ environments
- Non-destructive message peek — messages are never acknowledged or removed
- JSON pretty-printing with syntax-aware formatting
- Regex-capable search to filter queues by name
- TLS support with client certificate authentication
- Keyboard-driven navigation throughout
- Adjustable fetch count per session
- Profile switching without restarting

## Prerequisites

- Go 1.23 or newer (for building from source)
- RabbitMQ with the Management Plugin enabled (`rabbitmq-plugins enable rabbitmq_management`)

## Installation

Install directly with Go:

```bash
go install github.com/matutedenda/rabbitpeek@latest
```

Or build from source:

```bash
git clone https://github.com/matutedenda/rabbitpeek.git
cd rabbitpeek
go build -o rabbitpeek .
```

## Configuration

rabbitpeek reads its configuration from `~/.config/rabbitpeek/config.toml`. A template is provided at `config.example.toml` in the repository root.

```toml
# rabbitpeek configuration
# Copy this file to ~/.config/rabbitpeek/config.toml

[profiles.local]
host     = "localhost"
port     = 15672
username = "guest"
password = "guest"
vhost    = "/"
tls      = false

[profiles.production]
host     = "rabbit.internal.company.com"
port     = 15671
username = "admin"
password = "secret"
vhost    = "/app"
tls      = true
tls_cert = "/path/to/client.crt"
tls_key  = "/path/to/client.key"
tls_ca   = "/path/to/ca.crt"

default_profile = "local"
```

Each `[profiles.<name>]` block defines a named environment. The `default_profile` key sets which profile is loaded on startup when no `-p` flag is provided.

### TLS

Set `tls = true` to use HTTPS when connecting to the Management API. Provide `tls_cert`, `tls_key`, and `tls_ca` paths for mutual TLS authentication with a client certificate.

## Usage

Start with the default profile:

```bash
rabbitpeek
```

Start with a specific profile:

```bash
rabbitpeek -p production
```

Use a custom config file path:

```bash
rabbitpeek -c /path/to/config.toml
```

## Keyboard Shortcuts

| Key             | Action                                  |
|-----------------|-----------------------------------------|
| `?` / `F1`      | Toggle help overlay                     |
| `Tab`           | Switch focus between queue list and message panel |
| `j` / `Down`    | Move selection down                     |
| `k` / `Up`      | Move selection up                       |
| `Enter`         | Select queue / confirm                  |
| `r`             | Reload messages for the current queue   |
| `R`             | Reload the queue list                   |
| `/`             | Open search to filter queues by name    |
| `Esc`           | Clear search / close overlay            |
| `p`             | Switch active profile                   |
| `n`             | Fetch messages for the selected queue   |
| `+`             | Increase message fetch count            |
| `-`             | Decrease message fetch count            |
| `h` / `Left`    | Scroll message panel left               |
| `l` / `Right`   | Scroll message panel right              |
| `q` / `Ctrl+C`  | Quit                                    |

## Tech Stack

- [charmbracelet/bubbletea](https://github.com/charmbracelet/bubbletea) — TUI framework (Elm architecture)
- [charmbracelet/bubbles](https://github.com/charmbracelet/bubbles) — UI components (list, viewport, text input)
- [charmbracelet/lipgloss](https://github.com/charmbracelet/lipgloss) — Styling and layout
- [spf13/cobra](https://github.com/spf13/cobra) — CLI argument parsing
- [spf13/viper](https://github.com/spf13/viper) — Configuration loading
- [tidwall/pretty](https://github.com/tidwall/pretty) — JSON formatting

## License

MIT
