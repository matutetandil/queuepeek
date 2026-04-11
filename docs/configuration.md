# Configuration

queuepeek reads its configuration from `~/.config/queuepeek/config.toml`.

## Profile basics

Each `[profiles.<name>]` block defines a named environment. The `type` field determines which backend is used.

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

Supported types: `rabbitmq`, `kafka`, `mqtt`.

## TLS

Set `tls = true` to enable TLS. For mutual TLS with client certificates:

```toml
[profiles.secure]
type     = "rabbitmq"
host     = "rabbit.example.com"
port     = 15671
username = "admin"
password = "secret"
tls      = true
tls_cert = "/path/to/client.crt"
tls_key  = "/path/to/client.key"
tls_ca   = "/path/to/ca.crt"
```

- **RabbitMQ** — HTTPS for Management API
- **Kafka** — SASL_SSL with PLAIN mechanism
- **MQTT** — mqtts:// (MQTT over TLS)

Cloud hosts (CloudAMQP, AWS, Azure) are auto-detected and default to port 443 with TLS.

## MQTT topics

For MQTT profiles, you can pre-configure specific topics. If omitted, queuepeek subscribes to `#` and discovers topics via a 3-second scan.

```toml
[profiles.mqtt-dev]
type   = "mqtt"
host   = "mqtt.example.com"
port   = 1883
topics = ["sensors/#", "devices/status"]
```

## Schema Registry

Point to a Confluent-compatible Schema Registry for auto-decoding Avro and Protobuf messages.

```toml
[profiles.kafka-prod.schema_registry]
url      = "http://schema-registry.example.com:8081"
username = "sr-user"
password = "sr-pass"
```

Toggle decode with `s` in the message detail view. Messages must use the Confluent wire format (magic byte `0x00` + 4-byte schema ID).

## Webhook alerts

Define regex-based alerts that fire HTTP POST requests when messages match a pattern.

```toml
[[webhook_alerts]]
name        = "error-monitor"
pattern     = "(?i)error|exception"
webhook_url = "https://hooks.slack.com/services/xxx"
enabled     = true
queues      = []  # empty = all queues
```

Manage alerts with `Shift+W` on the queue list. Alerts check every 30 seconds with hash-based deduplication.

The webhook POST body:
```json
{
  "alert": "error-monitor",
  "queue": "my-queue",
  "matched_preview": "first 100 chars of matching message...",
  "timestamp": "1700000000"
}
```

## Saved filters

Named filter expressions are saved per queue. Use `Ctrl+B` in the message list to save, `Shift+B` to load. Stored in `config.toml`.

## Payload templates

Message body templates with variable interpolation. Use `Ctrl+W` to save, `Ctrl+T` to load in the publish popup.

Supported variables:
- `{{timestamp}}` — Unix epoch seconds
- `{{uuid}}` — UUID v4
- `{{random_int}}` — Random number (0-999999)
- `{{counter}}` — Incremental session counter
- `{{env.VAR}}` — Environment variable

## Scheduled messages

Scheduled messages persist to `~/.config/queuepeek/scheduled.json`. They survive restarts — past-due messages fire immediately on startup.

Use `Ctrl+S` in the publish popup to schedule with a delay, `Shift+S` on queue/message list to view pending.
