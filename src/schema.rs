use std::collections::HashMap;

use crate::config::SchemaRegistryConfig;

pub struct SchemaRegistryClient {
    client: reqwest::blocking::Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
    cache: HashMap<i32, CachedSchema>,
}

enum CachedSchema {
    Avro(apache_avro::Schema),
    Protobuf(String), // .proto text (for info display)
    Json(String),
    RawJson(String),
}

pub struct DecodedMessage {
    pub schema_id: i32,
    pub schema_type: String,
    pub decoded_body: String,
}

impl SchemaRegistryClient {
    pub fn new(config: &SchemaRegistryConfig) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client: {}", e))?;

        Ok(Self {
            client,
            base_url: config.url.trim_end_matches('/').to_string(),
            username: config.username.clone(),
            password: config.password.clone(),
            cache: HashMap::new(),
        })
    }

    fn fetch_schema(&mut self, id: i32) -> Result<(), String> {
        if self.cache.contains_key(&id) {
            return Ok(());
        }

        let url = format!("{}/schemas/ids/{}", self.base_url, id);
        let mut req = self.client.get(&url);
        if let (Some(ref u), Some(ref p)) = (&self.username, &self.password) {
            req = req.basic_auth(u, Some(p));
        }

        let resp: serde_json::Value = req
            .send().map_err(|e| format!("Registry HTTP: {}", e))?
            .json().map_err(|e| format!("Registry JSON: {}", e))?;

        let schema_str = resp["schema"].as_str()
            .ok_or_else(|| "No 'schema' field in registry response".to_string())?;
        let schema_type = resp["schemaType"].as_str().unwrap_or("AVRO");

        let cached = match schema_type {
            "AVRO" => {
                let avro_schema = apache_avro::Schema::parse_str(schema_str)
                    .map_err(|e| format!("Avro schema parse: {}", e))?;
                CachedSchema::Avro(avro_schema)
            }
            "PROTOBUF" => {
                CachedSchema::Protobuf(schema_str.to_string())
            }
            "JSON" => {
                CachedSchema::Json(schema_str.to_string())
            }
            _ => {
                CachedSchema::RawJson(schema_str.to_string())
            }
        };

        self.cache.insert(id, cached);
        Ok(())
    }

    /// Attempt to decode a message body using the Confluent wire format.
    /// Wire format: byte 0 = 0x00 (magic), bytes 1-4 = schema ID (big-endian), rest = payload
    pub fn decode_message(&mut self, raw_body: &[u8]) -> Result<DecodedMessage, String> {
        if raw_body.len() < 5 {
            return Err("Body too short for Confluent wire format".into());
        }

        if raw_body[0] != 0x00 {
            return Err("No Confluent magic byte (0x00)".into());
        }

        let schema_id = i32::from_be_bytes([raw_body[1], raw_body[2], raw_body[3], raw_body[4]]);
        let payload = &raw_body[5..];

        self.fetch_schema(schema_id)?;

        let cached = self.cache.get(&schema_id)
            .ok_or_else(|| "Schema not in cache".to_string())?;

        match cached {
            CachedSchema::Avro(schema) => {
                let reader = apache_avro::Reader::with_schema(schema, &payload[..])
                    .map_err(|e| format!("Avro reader: {}", e));

                match reader {
                    Ok(mut r) => {
                        if let Some(Ok(value)) = r.next() {
                            let json = avro_value_to_json(&value);
                            let pretty = serde_json::to_string_pretty(&json)
                                .unwrap_or_else(|_| format!("{:?}", value));
                            Ok(DecodedMessage {
                                schema_id,
                                schema_type: "avro".to_string(),
                                decoded_body: pretty,
                            })
                        } else {
                            // Try single-object decoding without container
                            decode_avro_single(schema, payload, schema_id)
                        }
                    }
                    Err(_) => {
                        // Fallback: try single-object decode (no container header)
                        decode_avro_single(schema, payload, schema_id)
                    }
                }
            }
            CachedSchema::Protobuf(_proto_text) => {
                // Confluent Protobuf wire format: after the 5-byte header,
                // there is a varint-encoded array length followed by varint message indexes.
                // Skip the message index array to get to the actual protobuf payload.
                let proto_payload = skip_protobuf_message_indexes(payload)?;
                let decoded = decode_protobuf_raw(proto_payload);
                let pretty = serde_json::to_string_pretty(&decoded)
                    .unwrap_or_else(|_| format!("{:?}", decoded));
                Ok(DecodedMessage {
                    schema_id,
                    schema_type: "protobuf".to_string(),
                    decoded_body: pretty,
                })
            }
            CachedSchema::Json(schema_str) => {
                // JSON schema — the payload is JSON, just validate and pretty-print
                let val: serde_json::Value = serde_json::from_slice(payload)
                    .map_err(|e| format!("JSON decode: {}", e))?;
                let pretty = serde_json::to_string_pretty(&val)
                    .unwrap_or_else(|_| String::from_utf8_lossy(payload).to_string());
                Ok(DecodedMessage {
                    schema_id,
                    schema_type: "json-schema".to_string(),
                    decoded_body: pretty,
                })
            }
            CachedSchema::RawJson(_) => {
                // Unknown schema type — show raw payload as string
                let text = String::from_utf8_lossy(payload).to_string();
                Ok(DecodedMessage {
                    schema_id,
                    schema_type: "unknown".to_string(),
                    decoded_body: text,
                })
            }
        }
    }

    /// Try to decode a body string (may be binary-escaped or UTF-8 lossy)
    pub fn decode_body_string(&mut self, body: &str) -> Result<DecodedMessage, String> {
        let bytes = body.as_bytes();
        self.decode_message(bytes)
    }
}

fn decode_avro_single(schema: &apache_avro::Schema, payload: &[u8], schema_id: i32) -> Result<DecodedMessage, String> {
    let value = apache_avro::from_avro_datum(schema, &mut &payload[..], None)
        .map_err(|e| format!("Avro datum decode: {}", e))?;
    let json = avro_value_to_json(&value);
    let pretty = serde_json::to_string_pretty(&json)
        .unwrap_or_else(|_| format!("{:?}", value));
    Ok(DecodedMessage {
        schema_id,
        schema_type: "avro".to_string(),
        decoded_body: pretty,
    })
}

/// Skip the Confluent Protobuf message index array.
/// Format: varint count, then `count` varint indexes. If count == 0, the payload starts immediately.
fn skip_protobuf_message_indexes(data: &[u8]) -> Result<&[u8], String> {
    let (count, mut offset) = decode_varint(data)?;
    // count == 0 means the default (first) message type, no indexes to skip
    if count == 0 {
        return Ok(&data[offset..]);
    }
    for _ in 0..count {
        let (_, new_offset) = decode_varint(&data[offset..])?;
        offset += new_offset;
    }
    Ok(&data[offset..])
}

fn decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
    let mut result: u64 = 0;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err("Varint too long".into());
        }
    }
    Err("Unexpected end of varint".into())
}

/// Decode raw protobuf wire format into a JSON object with field numbers as keys.
/// This is equivalent to `protoc --decode_raw`.
fn decode_protobuf_raw(data: &[u8]) -> serde_json::Value {
    let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut pos = 0;

    while pos < data.len() {
        let (tag, consumed) = match decode_varint(&data[pos..]) {
            Ok(v) => v,
            Err(_) => break,
        };
        pos += consumed;

        let field_number = tag >> 3;
        let wire_type = tag & 0x07;
        let key = format!("field_{}", field_number);

        let value = match wire_type {
            0 => {
                // Varint
                match decode_varint(&data[pos..]) {
                    Ok((val, consumed)) => {
                        pos += consumed;
                        serde_json::json!(val)
                    }
                    Err(_) => break,
                }
            }
            1 => {
                // 64-bit (fixed64, double)
                if pos + 8 > data.len() { break; }
                let bytes: [u8; 8] = data[pos..pos + 8].try_into().unwrap();
                pos += 8;
                let int_val = u64::from_le_bytes(bytes);
                let float_val = f64::from_le_bytes(bytes);
                // Heuristic: if it looks like a reasonable float, show as float
                if float_val.is_finite() && float_val.abs() < 1e15 && float_val.abs() > 1e-10 {
                    serde_json::json!(float_val)
                } else {
                    serde_json::json!(int_val)
                }
            }
            2 => {
                // Length-delimited (string, bytes, embedded message)
                match decode_varint(&data[pos..]) {
                    Ok((len, consumed)) => {
                        pos += consumed;
                        let len = len as usize;
                        if pos + len > data.len() { break; }
                        let field_data = &data[pos..pos + len];
                        pos += len;

                        // Try to decode as UTF-8 string first
                        if let Ok(s) = std::str::from_utf8(field_data) {
                            if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
                                serde_json::Value::String(s.to_string())
                            } else {
                                // Try as embedded message
                                let nested = decode_protobuf_raw(field_data);
                                if nested.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                                    nested
                                } else {
                                    serde_json::json!(base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD, field_data))
                                }
                            }
                        } else {
                            // Try as embedded message
                            let nested = decode_protobuf_raw(field_data);
                            if nested.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                                nested
                            } else {
                                serde_json::json!(base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD, field_data))
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            5 => {
                // 32-bit (fixed32, float)
                if pos + 4 > data.len() { break; }
                let bytes: [u8; 4] = data[pos..pos + 4].try_into().unwrap();
                pos += 4;
                let int_val = u32::from_le_bytes(bytes);
                let float_val = f32::from_le_bytes(bytes);
                if float_val.is_finite() && float_val.abs() < 1e10 && float_val.abs() > 1e-6 {
                    serde_json::json!(float_val)
                } else {
                    serde_json::json!(int_val)
                }
            }
            _ => break, // Unknown wire type
        };

        // Handle repeated fields by converting to array
        if let Some(existing) = fields.get_mut(&key) {
            if let serde_json::Value::Array(arr) = existing {
                arr.push(value);
            } else {
                let prev = existing.clone();
                *existing = serde_json::json!([prev, value]);
            }
        } else {
            fields.insert(key, value);
        }
    }

    serde_json::Value::Object(fields)
}

fn avro_value_to_json(value: &apache_avro::types::Value) -> serde_json::Value {
    use apache_avro::types::Value;
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::json!(*i),
        Value::Long(l) => serde_json::json!(*l),
        Value::Float(f) => serde_json::json!(*f),
        Value::Double(d) => serde_json::json!(*d),
        Value::Bytes(b) => serde_json::json!(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b)),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Fixed(_, b) => serde_json::json!(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b)),
        Value::Enum(_, s) => serde_json::Value::String(s.clone()),
        Value::Union(_, v) => avro_value_to_json(v),
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(avro_value_to_json).collect())
        }
        Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map.iter()
                .map(|(k, v)| (k.clone(), avro_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Record(fields) => {
            let obj: serde_json::Map<String, serde_json::Value> = fields.iter()
                .map(|(k, v)| (k.clone(), avro_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Date(d) => serde_json::json!(*d),
        Value::TimeMillis(t) => serde_json::json!(*t),
        Value::TimeMicros(t) => serde_json::json!(*t),
        Value::TimestampMillis(t) => serde_json::json!(*t),
        Value::TimestampMicros(t) => serde_json::json!(*t),
        Value::Decimal(d) => serde_json::json!(format!("{:?}", d)),
        Value::Uuid(u) => serde_json::Value::String(u.to_string()),
        _ => serde_json::json!(format!("{:?}", value)),
    }
}
