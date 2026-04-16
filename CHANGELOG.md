# Changelog

## [0.9.0] - 2026-04-16

### Added
- Live pulse indicator in header bar
  - Queue list: pulsing green dot shows auto-refresh is active
  - Message list: `live ●` appears when tail mode is on (`T`)
- Activity bar in message list during tail mode
  - Shows message count, publish/deliver rates, consumer count, and sparkline
  - Appears between header and message list only when tail mode is active
  - Refreshes queue stats alongside messages every 5 seconds

---

## [0.8.6] - 2026-04-16

### Improved
- Update popup flow with animated spinner and proper cleanup
  - Spinner animation during download with version info visible
  - All update popups use the same size to prevent visual artifacts
  - Successful update closes the app on Enter for clean restart
  - Failed update only closes the popup so the app remains usable
- Parallelized crates.io publish with binary builds in release workflow

---

## [0.8.5] - 2026-04-16

### Improved
- Queue list columns now use fixed widths with consistent spacing
  - Proper gaps between message count, rates, consumers, state, and sparkline
  - Columns stay stable as data refreshes — no more shifting layout
- Marquee scrolling for long queue names that don't fit
  - Names scroll horizontally every 2 seconds with seamless looping
  - Short names display normally without scrolling

---

## [0.8.4] - 2026-04-16

### Improved
- Live filter feedback in header and status bar
  - Queue list header shows "X of Y queues" while filtering
  - Message list header shows "X of Y msgs" while filtering
  - Status bar (green line) updates in real time: "12 of 153 queues match"
  - Restores to "153 queues loaded" when filter is cleared

---

## [0.8.3] - 2026-04-16

### Added
- Version displayed in UI
  - Profile screen: centered below logo
  - All other screens: right-aligned in header bar (`queuepeek vX.Y.Z`)
  - Auto-hides when terminal is too narrow to avoid overlapping header content
- Update flow with visual feedback
  - "Updating..." popup blocks input during download
  - Completion popup shows result and prompts restart
  - No more TUI corruption or frozen state during updates

### Fixed
- Update popup no longer closes immediately leaving no feedback

---

## [0.8.2] - 2026-04-16

### Added
- Pretty export (`Shift+E`) with fully indented JSON including nested body objects
  - JSON message bodies are parsed and exported as nested objects instead of escaped strings
  - Non-JSON bodies fall back to string representation
  - Compact export (`e`) unchanged

### Fixed
- Self-update no longer corrupts the TUI by redirecting stdout during download
- Windows build fixed with platform-specific stdout suppression

---

## [0.8.0] - 2026-04-15

### Added
- Vim-style payload search in message detail (`/` to search, `n`/`N` to navigate matches)
  - Case-insensitive search with highlighted matches in the payload
  - Match counter in search bar, adapts to current display mode (pretty-print, decode, schema)
- File browser popup for export and import operations
  - Directory navigation with `j`/`k`, `Enter` to open, `Backspace` to go up, `..` for parent
  - Export mode shows directories only; import mode shows files and directories
  - `Tab` to edit filename, `Ctrl+H` to toggle hidden files
  - Pre-populated filename with timestamp
- Consumer details in RabbitMQ queue info popup (`i`)
  - Shows each consumer's IP address, port, tag, connection, channel, prefetch, and ack mode
- Update confirmation popup with version comparison
  - `Shift+U` opens a confirm dialog instead of updating inline (prevents TUI corruption)
  - Async download with completion popup prompting restart
- Two-line status bar across all screens
  - Top line: keyboard shortcuts
  - Bottom line: notifications, update hints, status messages
- Help popup (`?`) available on all screens including profile select
  - Contextual shortcuts adapt to current screen, backend, and state (e.g., search active)

### Changed
- Export (`e`) now opens file picker instead of saving to current directory
- Import (`I`) now opens file picker with file browsing instead of text path input
- Footer shortcuts in message detail are contextual (search-mode shows `n`/`N`, normal shows all shortcuts)
- `Esc` in message detail is now three-stage: clear search → clear query → go back

### Fixed
- Panic on entering message list when messages hadn't loaded yet (index out of bounds)
- Help popup not rendering in message detail screen (missing popup draw call)
- Search state persisting across message detail entries

---

## [0.7.0] - 2026-04-10

### Changed
- Split `app.rs` (2368 lines) and `main.rs` (2045 lines) into focused modules
  - New `src/keys/` directory with one handler file per screen, eliminating deeply nested match arms
  - New `src/filters.rs` for filter expression parsing and evaluation
  - New `src/comparison.rs` for queue diff logic
  - New `src/operations.rs` for background operation helpers
  - New `src/utils.rs` for shared utilities
  - Deduplicated three queue picker key handler variants and two publish/edit popup handlers
  - `main.rs` reduced to ~98 lines (entry point only); `app.rs` reduced to ~1908 lines
- Real concurrent benchmark using `std::thread::scope`-based parallelism
  - Configurable thread count replaces the previous sequential flood loop
  - Results now include latency percentiles: p50, p95, p99 in addition to average latency
- Benchmark config popup updated with a Concurrency field (thread count)

### Added
- Persist scheduled messages to disk (`~/.config/queuepeek/scheduled.json`)
  - Scheduled messages survive app restarts
  - Messages whose scheduled time has already passed fire immediately on startup
  - File is written on every change (add or cancel) and read on startup
- MQTT retained message management (`H` key on queue list, MQTT only)
  - Scan retained messages via a timed wildcard subscription
  - View retained message list in a popup with `j`/`k` navigation
  - Clear individual retained messages by publishing an empty payload to the same topic
  - New Backend trait methods: `list_retained_messages`, `clear_retained_message`
- ACL / permission viewer (`A` key on queue list)
  - RabbitMQ: fetches user permission records from the Management API
  - Displays a color-coded table with configure, write, and read permission patterns per user and vhost
  - Other backends show "not supported"
- Webhook alerts on message pattern match (`W` key on queue list for configuration)
  - Define a regex pattern and an HTTP POST webhook URL per queue
  - Alert fires when an incoming message body matches the pattern during auto-refresh
  - Deduplicates alerts by content hash to avoid repeated notifications for the same message
  - Alert log visible in a popup; entries show timestamp, matched pattern, and message preview
  - Alert configuration is persisted in `config.toml`
  - New dependency: `regex = "1"`
- Schema Registry / Avro / Protobuf decode (`s` key toggle in message detail)
  - Confluent Schema Registry integration; URL configured per profile via `schema_registry` field
  - Auto-decodes Avro messages using the Confluent wire format (magic byte `0x00` + 4-byte schema ID + Avro payload)
  - Protobuf raw wire format decode (like `protoc --decode_raw`) — shows field numbers with values, handles nested messages, repeated fields, and all wire types
  - Handles Confluent Protobuf wire format (varint message index array after schema ID)
  - JSON Schema pass-through with pretty-printing
  - Decoded records rendered as pretty-printed JSON
  - Falls back gracefully if the Schema Registry is unreachable or the schema ID is unknown
  - New dependency: `apache-avro = "0.17"`
- Kafka security and permission info viewer (`A` key)
  - Shows connection security context (protocol, SASL mechanism, authenticated user)
  - Displays broker-level security configs (authorizer, ACL settings, super.users) via describe_configs

---

## [0.6.0] - 2026-04-09

### Added
- Sparklines in queue list — inline unicode block-character sparklines showing the publish rate history
  (last 60 data points, covering approximately 5 minutes) displayed next to each queue row
  - History accumulates automatically from the queue auto-refresh timer
  - Provides at-a-glance trend visibility without opening the queue info popup
- Base64 and gzip auto-decode in message detail (`b` key)
  - Toggle to auto-detect and decode base64, URL-safe base64, gzip, or base64+gzip payloads
  - Detected decode method is shown in the format label (e.g. `[json+b64]`, `[text+gz]`)
  - Combines with pretty-print and syntax highlighting when the decoded content is valid JSON or XML
- Message diff side-by-side (`d` key with exactly 2 messages selected in message list)
  - Opens a full-screen diff view using the `similar` crate
  - Shows header field differences and colored line-by-line body diff
  - Red lines indicate content removed from the first message; green lines indicate additions in the second
  - Scroll the diff with `j`/`k`; close with `Esc`
- Advanced message filtering (Tab in filter input to toggle between simple and advanced mode)
  - Field-based filter expressions:
    - `routing_key = "value"` — exact match on routing key
    - `header.key contains "text"` — substring match on a specific header
    - `body.field = "value"` — JSON path match on the message body
    - `field != "value"` — negation variant for any supported field
  - Simple mode retains the existing substring filter behavior
- Saved filters / bookmarks per queue (`B` to open load popup, `Ctrl+B` to save current filter)
  - Named filters are saved to `config.toml` under the current queue
  - Load popup shows the list of saved filters with `j`/`k` navigation; press `d` to delete, `Enter` to apply
- Payload templates with variable interpolation in the publish popup (`Ctrl+T` to load, `Ctrl+W` to save)
  - Templates are saved to `config.toml` with a user-provided name
  - Supported template variables: `{{timestamp}}`, `{{uuid}}`, `{{random_int}}`, `{{counter}}`, `{{env.VAR}}`
  - Variables are substituted at publish time
- Message replay for Kafka (`Y` key in message list)
  - Opens a config popup to specify start offset, end offset, and destination topic
  - Replays messages from the selected offset range to the destination topic
  - Kafka only; shows "not supported" message on other backends
- Exchange and binding topology view (`X` key on queue list)
  - Tree view showing exchanges -> bindings -> destination queues
  - RabbitMQ: fetches data from the Management API
  - Kafka and MQTT: shows "not supported" message
  - Scroll with `j`/`k`, close with `Esc`
- Benchmark / load testing (`F5` key on queue list)
  - Configuration popup for total message count and concurrency level
  - Uses the current publish form body as the message template
  - Live progress gauge during the flood publish
  - Results summary shows messages/sec throughput, average latency, and error count
  - Cancel at any time with `Esc`

---

## [0.5.0] - 2026-04-09

### Added
- Message scheduling with in-app timer (works on all backends)
  - Press `Ctrl+S` from the publish popup (`P`) or edit popup (`E`) to schedule instead of sending immediately
  - Schedule delay picker with presets: 30s, 1m, 5m, 10m, 30m, 1h
  - Scheduled messages are held in memory with a countdown timer
  - View and cancel pending scheduled messages with `S` from queue list or message list
  - Cancel individual scheduled messages with `d` from the scheduled messages popup
  - Footer indicator shows count of pending scheduled messages (`⏱N`)
  - When the timer expires, the message is published via the normal `backend.publish_message()` path
  - Note: scheduled messages are lost on app restart (in-memory only)
- Queue comparison / diff between two queues (`=` key on queue list)
  - Opens a queue picker to select the second queue (filter supported)
  - Fetches messages from both queues using the current fetch count
  - Computes diff by hashing message bodies
  - Results shown in a tabbed popup with three tabs:
    - Summary: counts for "in both", "only in A", "only in B", identical indicator
    - Only in A: list of messages unique to the first queue
    - Only in B: list of messages unique to the second queue
  - Navigate tabs with `Tab`/`Shift+Tab`, scroll with `j`/`k`
  - Useful for comparing a queue with its DLQ or verifying copy/move success
- Kafka consumer group offset reset (`R` in consumer groups popup)
  - Consumer groups popup (`G`) is now selectable with `j`/`k` navigation and a selection highlight (`▸`)
  - Press `R` on a selected group to reset its offsets
  - Only available for Kafka; refuses to reset if the group is in `Stable` (active) state
  - Strategy picker: Earliest (beginning of topic), Latest (end of topic), To Timestamp (unix ms), To Offset (specific offset)
  - For timestamp and offset strategies, a text input popup accepts the target value
  - Confirmation popup shows group name and strategy before executing
  - Uses rdkafka committed offsets commit to apply the reset
  - Consumer groups popup footer now shows `R:reset` hint

---

## [0.4.0] - 2026-04-08

### Added
- Message list auto-refresh / tail mode (`T` to toggle, `r` for manual refresh)
  - Auto-refreshes messages every 5 seconds when enabled
  - `[live ⟳]` indicator in header bar when active
- JSON and XML syntax highlighting in message detail view
  - JSON: keys in accent color, strings in green, numbers/bools bold, brackets muted
  - XML: tags in accent, attributes muted, text content in primary color
  - Only applies when pretty-print is ON; raw mode stays monochrome
- Edit & re-publish message (`E` in message detail)
  - Opens publish form pre-filled with current message body, routing key, and content type
  - Modify any field and re-publish to the same queue
- DLQ detection and re-routing (`L` in message list or detail)
  - Parses `x-death` header to extract original exchange and routing key
  - Confirmation popup showing the re-route destination
  - Publishes to original exchange with original routing key
  - Strips x-death headers from re-routed messages
  - RabbitMQ only (uses publish_to_exchange API)
- Kafka consumer groups popup (`G` on queue list)
  - Lists all consumer groups with committed offsets on the selected topic
  - Shows per-partition offset, high watermark, and lag
  - Color-coded lag (red > 0, green = 0)
  - Group state and member count displayed
  - Scrollable with j/k

---

## [0.3.1] - 2026-04-08

### Added
- Import messages from JSONL or JSON file (`I` key in message list)
  - Supports both JSONL format (from dump) and JSON array format (from export)
  - File path input popup with auto-detection of file format
  - Streaming publish with progress bar and cancellation support
- Kafka `purge_queue()` support (`x` key on queue list)
  - Deletes and recreates the topic with the same partition count
- Kafka `consume_messages()` implementation
  - Reads from low watermark using ephemeral consumer groups
  - Enables copy/move operations on Kafka topics
- `backend_type()` method on Backend trait for backend-specific behavior
- MQTT `consume_messages()` as alias for `peek_messages()` (subscriptions are inherently destructive)
  - Enables move, delete selected, copy, and re-publish operations on MQTT
- Queue info popup (`i` key on queue list) showing detailed queue/topic stats
  - RabbitMQ: type, state, node, messages (ready/unacked/total), rates with bar charts, memory, consumers, policy, arguments
  - Kafka: partition details (leader, replicas, ISR, offsets), topic configuration (retention, cleanup policy, compression, etc.)
  - MQTT: connection info and topic notes
  - Scrollable with `j`/`k`, close with `Esc`

### Changed
- Improved dump for large queues with per-backend strategies:
  - RabbitMQ: consume all → dump to JSONL → re-publish all back (gets entire queue, not just first batch)
  - Kafka: uses peek with large batch size for non-destructive dump
  - MQTT: single peek batch (no message history available)
- Updated help popup with all message list keyboard shortcuts
- Added `I:import` hint to message list footer

---

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
