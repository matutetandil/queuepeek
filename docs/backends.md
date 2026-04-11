# Backend details

queuepeek supports three message broker backends through a unified `Backend` trait.

## RabbitMQ

Uses the **Management HTTP API**. Requires the management plugin enabled:
```bash
rabbitmq-plugins enable rabbitmq_management
```

- Messages are peeked using `ack_requeue_true` — the queue state is never modified during inspection
- Publish uses the default exchange with queue name as routing key
- Publish to exchange supported for DLQ re-routing
- Purge and delete via Management API
- Topology view queries `/api/exchanges` and `/api/bindings`
- Permissions query `/api/permissions/{vhost}`
- Queue detail includes message stats, rates, memory, consumers, policy, and arguments

## Kafka

Connects via the **native protocol** using `librdkafka` (rdkafka crate). Requires `cmake` for building.

- Topics discovered from cluster metadata with watermark-based message counts
- Messages consumed via ephemeral consumer groups (no auto-commit, no side effects)
- Publishing via `BaseProducer`
- Consumer group inspection with per-partition lag calculation
- Offset reset (Earliest, Latest, To Timestamp, To Offset) for inactive groups
- Message replay from offset range to destination topic
- Topic deletion and purge (delete + recreate)
- Topic config via `describe_configs`
- Security info via broker `describe_configs` (authorizer, SASL, ACL settings)
- SASL/SSL authentication supported

## MQTT

Connects via the **MQTT protocol** using `rumqttc`.

- Topics discovered by subscribing to wildcard `#` (3-second scan) or from pre-configured topic list
- **Note: MQTT subscriptions consume messages** — there is no non-destructive peek
- Publishing with QoS 1
- Retained message management: scan, view, and clear (publish empty payload with retain flag)
- TLS with CA and client certificates
- Connection info displayed in queue detail

## Operation support matrix

| Operation | RabbitMQ | Kafka | MQTT |
|-----------|----------|-------|------|
| List queues/topics | Yes | Yes | Yes |
| Fetch messages | Yes | Yes | Yes* |
| Publish | Yes | Yes | Yes |
| Purge | Yes | Yes | No |
| Delete queue/topic | Yes | Yes | No |
| Copy messages | Yes | No | Yes* |
| Move messages | Yes | No | Yes* |
| Multi-select operations | Yes | Yes | Yes |
| Export to JSON | Yes | Yes | Yes |
| Import from JSONL/JSON | Yes | Yes | Yes |
| Dump queue to JSONL | Yes | Yes | Yes |
| Compare queues | Yes | Yes | Yes |
| Schedule messages | Yes | Yes | Yes |
| Consumer groups | No | Yes | No |
| Offset reset | No | Yes | No |
| Message replay | No | Yes | No |
| Topology view | Yes | No | No |
| Permissions/ACL | Yes | Yes** | No |
| Retained messages | No | No | Yes |
| Benchmark | Yes | Yes | Yes |
| Webhook alerts | Yes | Yes | Yes |
| Schema Registry decode | Yes | Yes | No |

\* MQTT subscriptions are inherently destructive.
\** Kafka shows broker security configs; full ACL listing requires rdkafka ACL API (not yet available in rdkafka 0.36).
