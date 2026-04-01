# Changelog

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
