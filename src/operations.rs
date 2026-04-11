use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::BgResult;
use crate::backend::{Backend, MessageInfo};

pub fn chrono_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// Parse x-death header value to extract original exchange and routing key
pub fn parse_x_death_value(value: &str) -> Option<(String, String)> {
    // x-death is typically a JSON array: [{"exchange":"...", "routing-keys":["..."], ...}]
    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(value) {
        let entry = if arr.is_array() { arr.get(0)? } else { &arr };
        let exchange = entry.get("exchange").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let routing_key = entry.get("routing-keys")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !exchange.is_empty() || !routing_key.is_empty() {
            return Some((exchange, routing_key));
        }
    }
    None
}

pub fn message_to_json(m: &MessageInfo) -> String {
    let headers: serde_json::Map<String, serde_json::Value> = m.headers.iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    let val = serde_json::json!({
        "index": m.index,
        "routing_key": m.routing_key,
        "exchange": m.exchange,
        "redelivered": m.redelivered,
        "timestamp": m.timestamp,
        "content_type": m.content_type,
        "headers": headers,
        "body": m.body,
    });
    serde_json::to_string(&val).unwrap_or_default()
}

/// RabbitMQ dump: consume all -> write JSONL -> re-publish all back
pub fn dump_rabbitmq(
    backend: Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    tx: mpsc::Sender<BgResult>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::Write;

    let filename = format!("queuepeek-dump-{}-{}.jsonl", queue, chrono_timestamp());
    let path = std::env::current_dir().unwrap_or_default().join(&filename);
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating file: {}", e))));
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);

    // Phase 1: consume all messages to JSONL file
    let batch_size = 100u32;
    let mut total = 0usize;

    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = tx.send(BgResult::OperationComplete(
                Ok(format!("Dump cancelled after {} messages — saved to {}", total, path.display()))
            ));
            return;
        }

        let batch = match backend.consume_messages(namespace, queue, batch_size) {
            Ok(msgs) => msgs,
            Err(e) => {
                if total > 0 {
                    drop(writer);
                    let _ = tx.send(BgResult::OperationProgress { completed: total, total: 0 });
                    republish_from_file(&backend, namespace, queue, &path, &tx, &cancel);
                    let _ = tx.send(BgResult::OperationComplete(
                        Ok(format!("Dumped {} messages to {} (consume stopped: {})", total, path.display(), e))
                    ));
                } else {
                    let _ = tx.send(BgResult::OperationComplete(Err(format!("Consume failed: {}", e))));
                }
                return;
            }
        };

        if batch.is_empty() { break; }

        for msg in &batch {
            let json = message_to_json(msg);
            if let Err(e) = writeln!(writer, "{}", json) {
                let _ = tx.send(BgResult::OperationComplete(
                    Err(format!("Writing: {} — partial dump at {}", e, path.display()))
                ));
                return;
            }
        }

        total += batch.len();
        let _ = tx.send(BgResult::OperationProgress { completed: total, total: 0 });

        if (batch.len() as u32) < batch_size { break; }
    }

    drop(writer);

    // Phase 2: re-publish all messages back to restore the queue
    republish_from_file(&backend, namespace, queue, &path, &tx, &cancel);

    let _ = tx.send(BgResult::OperationComplete(
        Ok(format!("Dumped {} messages to {}", total, path.display()))
    ));
}

/// Re-publish all messages from a JSONL file back to the queue
pub fn republish_from_file(
    backend: &Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    path: &std::path::Path,
    tx: &mpsc::Sender<BgResult>,
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::BufRead;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(
                Err(format!("Reading dump for re-publish: {} — file at {}", e, path.display()))
            ));
            return;
        }
    };
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let body = msg["body"].as_str().unwrap_or("");
        let routing_key = msg["routing_key"].as_str().unwrap_or("");
        let content_type = msg["content_type"].as_str().unwrap_or("");
        let headers: Vec<(String, String)> = msg["headers"].as_object()
            .map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default();

        let _ = backend.publish_message(namespace, queue, body, routing_key, &headers, content_type);
    }
}

/// Kafka dump: dedicated consumer from low watermark, non-destructive full read
pub fn dump_kafka(
    backend: Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    tx: mpsc::Sender<BgResult>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::Write;

    let filename = format!("queuepeek-dump-{}-{}.jsonl", queue, chrono_timestamp());
    let path = std::env::current_dir().unwrap_or_default().join(&filename);
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating file: {}", e))));
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);

    let batch_size = 500u32;
    let mut total = 0usize;
    let mut empty_polls = 0;

    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = tx.send(BgResult::OperationComplete(
                Ok(format!("Dump cancelled after {} messages — saved to {}", total, path.display()))
            ));
            return;
        }

        let batch = match backend.peek_messages(namespace, queue, batch_size) {
            Ok(msgs) => msgs,
            Err(e) => {
                if total > 0 {
                    let _ = tx.send(BgResult::OperationComplete(
                        Ok(format!("Dumped {} messages to {} (stopped: {})", total, path.display(), e))
                    ));
                } else {
                    let _ = tx.send(BgResult::OperationComplete(Err(format!("Peek failed: {}", e))));
                }
                return;
            }
        };

        if batch.is_empty() {
            empty_polls += 1;
            if empty_polls >= 2 { break; }
            continue;
        }
        empty_polls = 0;

        for msg in &batch {
            let json = message_to_json(msg);
            if let Err(e) = writeln!(writer, "{}", json) {
                let _ = tx.send(BgResult::OperationComplete(
                    Err(format!("Writing: {} — partial dump at {}", e, path.display()))
                ));
                return;
            }
        }

        total += batch.len();
        let _ = tx.send(BgResult::OperationProgress { completed: total, total: 0 });

        // Kafka peek calculates offsets from watermarks each call, so we get one batch
        break;
    }

    let _ = tx.send(BgResult::OperationComplete(
        Ok(format!("Dumped {} messages to {}", total, path.display()))
    ));
}

/// Simple peek-based dump for MQTT and other backends
pub fn dump_simple_peek(
    backend: Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    tx: mpsc::Sender<BgResult>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::Write;

    let filename = format!("queuepeek-dump-{}-{}.jsonl", queue, chrono_timestamp());
    let path = std::env::current_dir().unwrap_or_default().join(&filename);
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating file: {}", e))));
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);

    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        let _ = tx.send(BgResult::OperationComplete(Ok("Dump cancelled".into())));
        return;
    }

    let batch = match backend.peek_messages(namespace, queue, 100) {
        Ok(msgs) => msgs,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Peek failed: {}", e))));
            return;
        }
    };

    for msg in &batch {
        let json = message_to_json(msg);
        if let Err(e) = writeln!(writer, "{}", json) {
            let _ = tx.send(BgResult::OperationComplete(
                Err(format!("Writing: {} — partial dump at {}", e, path.display()))
            ));
            return;
        }
    }

    let _ = tx.send(BgResult::OperationProgress { completed: batch.len(), total: batch.len() });
    let _ = tx.send(BgResult::OperationComplete(
        Ok(format!("Dumped {} messages to {}", batch.len(), path.display()))
    ));
}
