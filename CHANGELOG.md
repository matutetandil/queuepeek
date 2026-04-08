# Changelog

## [0.3.0] - 2026-04-08

### Changed
- Renamed project from rabbitpeek to queuepeek
- Config path changed from `~/.config/rabbitpeek/` to `~/.config/queuepeek/`
- Default port auto-updates when switching backend type in profile form (RabbitMQ: 15672, Kafka: 9092, MQTT: 1883)

### Added
- Kafka backend implementation using rdkafka (librdkafka)
  - Topic listing via cluster metadata with message count from watermarks
  - Message consumption using ephemeral consumer groups (no auto-commit)
  - SASL/SSL authentication support
  - Kafka message headers, keys, partition, and offset displayed
- MQTT backend implementation using rumqttc
  - Topic discovery via wildcard `#` subscription (3-second scan)
  - Pre-configured topic list support via `topics` field in profile config
  - Message reading via topic subscription
  - TLS support with CA and client certificates
  - Note: MQTT consumes messages on read (no non-destructive peek)
- `topics` field in profile configuration for MQTT pre-configured topic monitoring

---

## [0.2.0] - 2026-04-02

### Changed
- Complete rewrite from Go to Rust using ratatui for precise terminal rendering
- Wizard-style drill-down flow replacing sidebar layout
- Backend trait architecture for multi-broker support

### Added
- Kafka and MQTT backend stubs (coming soon)
- Profile type field (rabbitmq/kafka/mqtt)
- Auto-refresh queue list every 5 seconds
- Message detail screen with headers and scrollable payload
- JSON and XML auto-detection with pretty-print toggle
- Clipboard support for copying payload and headers
- Breadcrumb navigation in all screen headers
- Queue publish/deliver rates in queue list
- Message position indicator in detail view
- Fetch count picker popup with presets
- Theme picker popup with live preview and color swatches
- Styled keybinding footers on all screens

### Removed
- Go implementation (archived then removed)
- Sidebar/IDE layout (replaced by wizard flow)

---

## [0.1.0] - 2026-04-01

### Added
- Initial project structure with Go modules
- RabbitMQ Management API client with TLS support
- Split-view TUI with sidebar (queue list) and message panel
- Dark theme inspired by Slack using lipgloss
- TOML-based configuration with multi-profile support
- Non-destructive message peeking (ack_requeue_true)
- Real-time regex search/filter for messages
- JSON pretty-printing for message bodies
- Keyboard-driven navigation (vim-style + arrows)
- Profile switcher overlay
- Help overlay with keyboard shortcuts
- Configurable fetch count per session (+/- keys)
- Horizontal scrolling for wide payloads
- Loading spinner for async operations
- Connection error handling with status bar messages
- Terminal resize support
