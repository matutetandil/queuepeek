# Keyboard shortcuts

Global: `Ctrl+C` quits from any screen. `U` triggers auto-update when available.

## Profile screen

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `Enter` | Connect with selected profile |
| `a` | Create new profile |
| `e` | Edit selected profile |
| `d` | Delete selected profile |
| `t` | Open theme picker |
| `q` | Quit |

## Queue list

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `Enter` | Open selected queue |
| `/` | Filter queues by name |
| `r` | Refresh queue list |
| `f` | Fetch count picker |
| `+` / `-` | Adjust fetch count by 10 |
| `v` | Switch namespace/vhost |
| `p` | Switch profile |
| `P` | Publish message |
| `x` | Purge queue (with confirmation) |
| `D` | Delete queue/topic (with confirmation) |
| `C` | Copy all messages to another queue |
| `m` | Move all messages to another queue |
| `=` | Compare with another queue (diff) |
| `i` | Queue info popup (stats, config, rates) |
| `G` | Consumer groups (Kafka) |
| `X` | Topology view (RabbitMQ) |
| `F5` | Benchmark / load test |
| `S` | View scheduled messages |
| `H` | Retained messages (MQTT only) |
| `A` | ACL / permissions |
| `W` | Webhook alert config |
| `t` | Theme picker |
| `Esc` | Back to profiles |
| `q` | Quit |

## Message list

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `Enter` | Open message detail |
| `/` | Filter messages |
| `Tab` | Toggle simple/advanced filter (in filter mode) |
| `B` | Load saved filter |
| `Ctrl+B` | Save current filter |
| `Space` | Toggle selection on current message |
| `a` | Select/deselect all |
| `r` | Refresh messages |
| `f` | Fetch count picker |
| `P` | Publish message |
| `C` | Copy selected to another queue |
| `M` | Move selected to another queue |
| `D` | Delete selected (with confirmation) |
| `d` | Diff two selected messages side-by-side |
| `e` | Export selected to JSON file |
| `R` | Re-publish selected to same queue |
| `W` | Dump entire queue to JSONL |
| `I` | Import from JSONL/JSON file |
| `L` | DLQ re-route (x-death header) |
| `T` | Toggle tail mode (auto-refresh) |
| `Y` | Replay from offset range (Kafka) |
| `S` | View scheduled messages |
| `Esc` | Clear selection / back to queue list |
| `q` | Quit |

## Message detail

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll payload |
| `PgDn` / `PgUp` | Scroll 10 lines |
| `p` | Toggle pretty-print |
| `b` | Toggle base64/gzip decode |
| `s` | Toggle Schema Registry decode |
| `c` | Copy payload to clipboard |
| `h` | Copy headers to clipboard |
| `E` | Edit & re-publish |
| `L` | DLQ re-route |
| `Esc` | Back to message list |
| `q` | Quit |

## Popup-specific shortcuts

### Publish popup
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Navigate fields |
| `Enter` | Send (newline in body field) |
| `Ctrl+Enter` | Force send |
| `Ctrl+S` | Schedule with delay |
| `Ctrl+T` | Load template |
| `Ctrl+W` | Save as template |

### Consumer groups (Kafka)
| Key | Action |
|-----|--------|
| `j` / `k` | Navigate groups |
| `R` | Reset offsets for selected group |

### Queue comparison results
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch tabs (Summary / Only in A / Only in B) |
| `j` / `k` | Scroll |

### Webhook alert config
| Key | Action |
|-----|--------|
| `a` | Add new alert |
| `Enter` | Toggle enabled/disabled |
| `d` | Delete alert |
| `L` | View alert log |

### Retained messages (MQTT)
| Key | Action |
|-----|--------|
| `j` / `k` | Navigate |
| `D` | Clear retained message |
| `c` | Copy body to clipboard |
