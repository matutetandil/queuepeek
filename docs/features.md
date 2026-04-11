# Features

## Wizard-style navigation

The UI follows a four-screen drill-down: **Profiles -> Queue list -> Message list -> Message detail**. Press `Esc` to go back one level. Each screen uses 100% of the terminal — no split panels.

## Message operations

### Publish
Press `Shift+P` on queue list or message list. Multi-line body editor with routing key and content type fields. `Ctrl+Enter` sends, `Enter` in body adds newline.

### Copy / Move
`Shift+C` copies all messages from one queue to another (non-destructive). `m` moves (destructive consume + publish). Both show a queue picker popup with filter. In message list, `Shift+C` and `Shift+M` operate on selected messages only.

### Delete selected
`Shift+D` in message list deletes selected messages via a consume-all → filter → re-publish strategy. A temp file backup is created for recovery on failure.

### Export / Import
`e` exports selected messages to a pretty JSON file. `Shift+W` dumps the entire queue to a JSONL file (streaming, low memory). `Shift+I` imports from JSONL or JSON array files.

### Diff
Select exactly 2 messages with `Space`, press `d`. Side-by-side colored diff using LCS algorithm. Headers compared field-by-field, bodies compared line-by-line.

## Message decode

### Pretty-print
`p` toggles JSON/XML auto-detection and formatting with syntax highlighting. JSON keys in accent color, strings green, numbers bold.

### Base64 / gzip
`b` toggles auto-decode chain: base64 -> UTF-8, base64 -> gzip -> UTF-8, URL-safe base64, raw gzip. Format label shows the decode method (e.g. `[json+b64]`).

### Schema Registry (Avro / Protobuf)
`s` toggles Confluent Schema Registry decode. Requires `schema_registry` configured in the profile. Supports:
- **Avro** — full schema-based decode to JSON
- **Protobuf** — raw wire format decode (field numbers with values, like `protoc --decode_raw`)
- **JSON Schema** — pass-through with pretty-print

Wire format: magic byte `0x00` + 4-byte schema ID (big-endian) + payload. Schemas cached by ID.

## Queue management

### Queue info
`i` shows a scrollable popup with stats, rates (with horizontal bar charts), configuration, and backend-specific details. Auto-refreshes every 5 seconds.

### Purge / Delete
`x` purges all messages. `Shift+D` deletes the queue/topic entirely. Both require confirmation. Kafka purge works by deleting and recreating the topic.

### Compare
`=` picks a second queue and computes a diff — shows messages in both, only in A, only in B. Tabbed popup with summary.

### Topology (RabbitMQ)
`Shift+X` shows a tree view of exchanges -> bindings -> queues with routing keys.

## Kafka-specific

### Consumer groups
`Shift+G` shows groups with state, member count, total lag, and per-partition offset/lag table. Lag color-coded (red > 0, green = 0).

### Offset reset
`Shift+R` in consumer groups popup. Strategies: Earliest, Latest, To Timestamp, To Offset. Only for inactive (non-Stable) groups.

### Message replay
`Shift+Y` replays messages from an offset range to a destination topic.

## Monitoring

### Sparklines
Queue list shows inline unicode sparklines for publish rate history (last 60 data points, ~5 minutes).

### Tail mode
`Shift+T` enables auto-refresh on the message list (every 5 seconds). `[live]` indicator shown in header.

### Webhook alerts
`Shift+W` configures regex-based alerts. When a message matches, an HTTP POST is sent to the configured URL. Alerts deduplicated by content hash. Alert log accessible via `Shift+L` in the alert config popup.

## Benchmarking

`F5` opens benchmark config with message count and thread count. Uses `std::thread::scope` for real parallelism. Results show messages/sec, average latency, and p50/p95/p99 percentiles.

## Scheduling

`Ctrl+S` in the publish popup schedules a message with a delay (presets: 30s to 1h). Scheduled messages persist to disk and survive restarts. `Shift+S` shows pending messages with countdown timers.

## DLQ workflows

`Shift+L` parses `x-death` headers from dead-lettered messages, shows the original exchange and routing key, and re-publishes with one confirmation. Strips x-death headers from re-routed messages.

## MQTT retained messages

`Shift+H` on queue list (MQTT only) scans for retained messages via wildcard subscription. `Shift+D` clears a retained message by publishing an empty payload with the retain flag.

## ACL / Permissions

`Shift+A` shows a scrollable, color-coded permission table. RabbitMQ queries the Management API; Kafka shows broker security configs.

## Themes

5 built-in themes: Slack, Dracula, Gruvbox, Catppuccin, Tokyo Night. Press `t` to open the picker with live preview — the theme applies as you navigate.
