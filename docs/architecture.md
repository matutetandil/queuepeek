# Architecture

## Module structure

```
src/
├── main.rs              # Entry point (~98 lines): terminal init, event loop
├── app.rs               # Central state machine: App struct, Screen/Popup enums,
│                        # BgResult variants, background task launchers, process_bg_results
├── config.rs            # Config structs and TOML load/save
├── schema.rs            # Schema Registry client, Avro/Protobuf decode
├── filters.rs           # Advanced filter expression parser and evaluator
├── comparison.rs        # Queue diff / comparison algorithm
├── operations.rs        # Dump strategies, message serialization, x-death parsing
├── utils.rs             # Template interpolation, clipboard
├── updater.rs           # Auto-update via GitHub Releases
├── keys/                # Key handlers, one file per screen
│   ├── mod.rs           # Top-level dispatch
│   ├── profile.rs       # Profile screen handlers
│   ├── queue_list.rs    # Queue list handlers
│   ├── message_list.rs  # Message list handlers
│   ├── message_detail.rs
│   └── popup.rs         # All popup handlers (unified queue pickers, publish/edit)
├── backend/
│   ├── mod.rs           # Backend trait (~20 methods) and shared data structs
│   ├── rabbitmq.rs      # RabbitMQ Management HTTP API
│   ├── kafka.rs         # Kafka via rdkafka (librdkafka)
│   └── mqtt.rs          # MQTT via rumqttc
└── ui/
    ├── mod.rs            # Draw dispatcher per Screen
    ├── profiles.rs       # Profile list and form
    ├── queue_list.rs     # Queue list with sparklines
    ├── message_list.rs   # Message list with multi-select checkboxes
    ├── message_detail.rs # Payload display with syntax highlighting and decode
    ├── popup.rs          # ~38 popup draw functions
    └── theme.rs          # 5 themes with picker and live preview
```

## Key design patterns

### State machine
`App` holds all application state. The `Screen` enum (`ProfileSelect`, `QueueList`, `MessageList`, `MessageDetail`) drives which UI module renders. The `Popup` enum (~38 variants) overlays modal popups on any screen.

### Backend trait
The `Backend` trait abstracts all broker-specific logic. Required methods: `broker_info`, `list_namespaces`, `list_queues`, `peek_messages`, `clone_backend`. Optional methods (default to "not supported"): `publish_message`, `delete_queue`, `purge_queue`, `consume_messages`, `consumer_groups`, `list_permissions`, `list_retained_messages`, etc.

### Background I/O
All broker communication runs in spawned threads. Results are sent back via `std::sync::mpsc` channels as `BgResult` enum variants. The event loop in `run_app` polls both `crossterm` input events and the mpsc receiver on each 100ms tick.

### Concurrency model
- Queue list auto-refresh: timer-based, every 5 seconds
- Message tail mode: piggybacks on the same 5-second timer
- Benchmark: `std::thread::scope` with N threads, shared atomic counters
- Webhook alerts: checked every 30 seconds in the event loop
- Scheduled messages: checked every tick, persisted to disk

### Key handler separation
Key handlers live in `src/keys/`, separated by screen. The popup handler (`keys/popup.rs`) uses shared helpers to deduplicate:
- `handle_scroll_keys` — generic j/k/PgDn/PgUp for scrollable popups
- `handle_queue_picker_key` — unified handler for 3 queue picker variants
- `handle_publish_key` — unified handler for publish and edit popups

## Testing

79 unit tests cover all pure logic modules:
- `filters.rs` — parse, resolve, eval
- `comparison.rs` — queue diff scenarios
- `operations.rs` — serialization, x-death parsing
- `schema.rs` — varint, protobuf wire format, Avro value conversion
- `utils.rs` — template interpolation
- `config.rs` — Profile methods, Config CRUD, save/load roundtrip

Run with `cargo test`.
