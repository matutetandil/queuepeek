# Changelog

## [0.3.0] - 2026-04-08

### Changed
- Renamed project from rabbitpeek to queuepeek
- Config path changed from `~/.config/rabbitpeek/` to `~/.config/queuepeek/`
- Default port auto-updates when switching backend type in profile form (RabbitMQ: 15672, Kafka: 9092, MQTT: 1883)
- Message list Esc behavior is now two-stage: first press clears selection, second press goes back to queue list

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
- Publish message operation (`P` key on queue list)
  - Multi-line body editor with routing key and content type fields
  - RabbitMQ: publishes via default exchange using queue name as routing key
  - Kafka: publishes via BaseProducer
  - MQTT: publishes with QoS 1
- Purge queue operation (`x` key on queue list, RabbitMQ only)
  - Removes all messages from the selected queue
  - Requires confirmation before executing
- Delete queue/topic operation (`D` key on queue list)
  - Supported on RabbitMQ and Kafka
  - Requires confirmation before executing
- Copy messages operation (`C` key on queue list)
  - Non-destructive copy of all messages from one queue to another
  - Preserves message order
  - Uses peek (ack_requeue_true) + publish internally
  - Progress bar with cancellation support
- Move messages operation (`m` key on queue list)
  - Destructive move: consumes from source, publishes to destination
  - Preserves message order
  - Progress bar with cancellation support
- Queue picker popup for selecting copy/move destination
  - Filter queues with `/`
  - Keyboard navigation with `j`/`k`
- Multi-select in message list
  - Space toggles selection on the focused message; visual checkbox prefix (☑/☐) on each row
  - `a` selects all messages; pressing `a` again when all are selected deselects all
  - Selection count shown in the screen header and footer
- Copy selected messages to another queue (`C` key in message list)
  - Opens the queue picker popup to choose the destination
  - Only the selected messages are copied, preserving their order
  - RabbitMQ only
- Delete selected messages (`D` key in message list)
  - Uses a consume-all-and-requeue approach: fetches all messages, discards the selected ones, and requeues the rest
  - Destructive operation; requires confirmation before executing
  - RabbitMQ only
- Export selected messages to JSON (`e` key in message list)
  - Writes selected messages to a `.json` file in the current working directory
  - File is named after the queue and a timestamp (e.g. `my-queue-20260408-153012.json`)
  - Available on all backends
- Re-publish selected messages (`R` key in message list)
  - Re-publishes each selected message back to the same queue
  - Useful for retry workflows or testing message processing
  - RabbitMQ and Kafka supported
- Dump entire queue to JSONL file (`W` key in message list)
  - Non-destructive peek-based dump, streaming with low memory usage
  - Output file: `queuepeek-dump-{queue}-{timestamp}.jsonl`
- Stream-based delete implementation
  - Consumes messages in batches of 100, writing to a temp JSONL backup
  - Re-publishes non-selected messages by reading the backup line by line
  - Backup file persists on failure for manual recovery
  - Constant memory usage regardless of queue size
- Auto-update system via GitHub Releases
  - Checks for new versions on startup and every hour
  - Non-intrusive footer notification when an update is available
  - Press `U` to self-update the binary
- Backend type picker popup with descriptions in profile form
- Cloud host auto-detection (sets port 443 and TLS for known providers)
- Dynamic footer hints that change based on focused profile form field
- Optional vhost field (shows namespace picker when empty)

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
