//! # Stream Operation Functions
//!
//! Built-in functions for creating and manipulating streams in HyperQL.

use crate::types::Value;
use crate::{HyperQLError, Result};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};

/// Stream configuration for stream creation
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub buffer_size: usize,
    pub retention_ms: Option<i64>,
    pub max_events: Option<usize>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            retention_ms: None,
            max_events: None,
        }
    }
}

/// Stream event with timestamp
#[derive(Debug, Clone)]
pub struct StreamEvent {
    pub data: Value,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: u64,
}

/// In-memory stream storage
type StreamStorage = Arc<Mutex<HashMap<String, VecDeque<StreamEvent>>>>;
type StreamSenders = Arc<Mutex<HashMap<String, broadcast::Sender<StreamEvent>>>>;

lazy_static::lazy_static! {
    /// Global stream registry - in production this would be managed differently
    pub static ref STREAM_STORAGE: StreamStorage = Arc::new(Mutex::new(HashMap::new()));
    pub static ref STREAM_SENDERS: StreamSenders = Arc::new(Mutex::new(HashMap::new()));
    pub static ref STREAM_CONFIGS: Arc<Mutex<HashMap<String, StreamConfig>>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref SEQUENCE_COUNTERS: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Stream operation function trait
pub trait StreamOperationFunction {
    /// Execute the stream operation
    fn execute(&self, args: Vec<Value>) -> Result<Value>;

    /// Get function name
    fn name(&self) -> &str;

    /// Get function signature
    fn signature(&self) -> &str;
}

/// CREATE STREAM function
pub struct CreateStreamFunction;

impl StreamOperationFunction for CreateStreamFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 1 {
            return Err(HyperQLError::ValidationError {
                message: "CREATE STREAM requires at least a stream name".to_string(),
                field: Some("name".to_string()),
            });
        }

        let stream_name = match &args[0] {
            Value::String(name) => name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[0]),
                context: "stream name".to_string(),
            }),
        };

        // Parse configuration if provided
        let mut config = StreamConfig::default();
        if args.len() > 1 {
            if let Value::Map(config_map) = &args[1] {
                if let Some(Value::Int(buffer_size)) = config_map.get("buffer_size") {
                    config.buffer_size = *buffer_size as usize;
                }
                if let Some(Value::Int(retention_ms)) = config_map.get("retention_ms") {
                    config.retention_ms = Some(*retention_ms);
                }
                if let Some(Value::Int(max_events)) = config_map.get("max_events") {
                    config.max_events = Some(*max_events as usize);
                }
            }
        }

        // Create the stream
        {
            let mut storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "CreateStreamFunction::execute".to_string(),
            })?;

            let mut senders = STREAM_SENDERS.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream senders lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "CreateStreamFunction::execute".to_string(),
            })?;

            let mut configs = STREAM_CONFIGS.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream configs lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "CreateStreamFunction::execute".to_string(),
            })?;

            let mut counters = SEQUENCE_COUNTERS.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire sequence counters lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "CreateStreamFunction::execute".to_string(),
            })?;

            // Check if stream already exists
            if storage.contains_key(&stream_name) {
                return Err(HyperQLError::ValidationError {
                    message: format!("Stream '{}' already exists", stream_name),
                    field: Some("name".to_string()),
                });
            }

            // Create broadcast channel for the stream
            let (tx, _rx) = broadcast::channel(config.buffer_size);

            // Initialize stream storage
            storage.insert(stream_name.clone(), VecDeque::new());
            senders.insert(stream_name.clone(), tx);
            configs.insert(stream_name.clone(), config);
            counters.insert(stream_name.clone(), 0);
        }

        Ok(Value::Bool(true))
    }
    
    fn name(&self) -> &str {
        "stream_create"
    }
    
    fn signature(&self) -> &str {
        "stream_create(name: string, config: object) -> boolean"
    }
}

/// PRODUCE EVENT function
pub struct ProduceEventFunction;

impl StreamOperationFunction for ProduceEventFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 2 {
            return Err(HyperQLError::ValidationError {
                message: "PRODUCE EVENT requires stream name and event data".to_string(),
                field: Some("args".to_string()),
            });
        }

        let stream_name = match &args[0] {
            Value::String(name) => name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[0]),
                context: "stream name".to_string(),
            }),
        };

        let event_data = args[1].clone();

        // Get sequence number and create event
        let event = {
            let mut counters = SEQUENCE_COUNTERS.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire sequence counters lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "ProduceEventFunction::execute".to_string(),
            })?;

            let sequence_number = counters.entry(stream_name.clone()).and_modify(|e| *e += 1).or_insert(1);

            StreamEvent {
                data: event_data,
                timestamp: Utc::now(),
                sequence_number: *sequence_number,
            }
        };

        // Store event and broadcast
        {
            let mut storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "ProduceEventFunction::execute".to_string(),
            })?;

            let senders = STREAM_SENDERS.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream senders lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "ProduceEventFunction::execute".to_string(),
            })?;

            let configs = STREAM_CONFIGS.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream configs lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "ProduceEventFunction::execute".to_string(),
            })?;

            let stream_events = storage.get_mut(&stream_name).ok_or_else(|| HyperQLError::ValidationError {
                message: format!("Stream '{}' does not exist", stream_name),
                field: Some("stream".to_string()),
            })?;

            let config = configs.get(&stream_name).ok_or_else(|| HyperQLError::InternalError {
                message: format!("Stream config for '{}' not found", stream_name),
                component: "stream_operations".to_string(),
                debug_info: "ProduceEventFunction::execute".to_string(),
            })?;

            // Add event to storage
            stream_events.push_back(event.clone());

            // Apply retention policies
            if let Some(max_events) = config.max_events {
                while stream_events.len() > max_events {
                    stream_events.pop_front();
                }
            }

            if let Some(retention_ms) = config.retention_ms {
                let cutoff = Utc::now() - chrono::Duration::milliseconds(retention_ms);
                while let Some(front_event) = stream_events.front() {
                    if front_event.timestamp < cutoff {
                        stream_events.pop_front();
                    } else {
                        break;
                    }
                }
            }

            // Broadcast event to subscribers
            if let Some(sender) = senders.get(&stream_name) {
                let _ = sender.send(event); // Ignore send errors (no active receivers)
            }
        }

        Ok(Value::Bool(true))
    }
    
    fn name(&self) -> &str {
        "stream_produce"
    }
    
    fn signature(&self) -> &str {
        "stream_produce(stream: string, event_data: object) -> boolean"
    }
}

/// CONSUME FROM STREAM function
pub struct ConsumeStreamFunction;

impl StreamOperationFunction for ConsumeStreamFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 1 {
            return Err(HyperQLError::ValidationError {
                message: "CONSUME STREAM requires at least a stream name".to_string(),
                field: Some("stream".to_string()),
            });
        }

        let stream_name = match &args[0] {
            Value::String(name) => name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[0]),
                context: "stream name".to_string(),
            }),
        };

        // Parse consume configuration
        let mut limit: Option<usize> = None;
        let mut from_sequence: Option<u64> = None;
        let mut consume_all = false;

        if args.len() > 1 {
            if let Value::Map(config_map) = &args[1] {
                if let Some(Value::Int(l)) = config_map.get("limit") {
                    limit = Some(*l as usize);
                }
                if let Some(Value::Int(seq)) = config_map.get("from_sequence") {
                    from_sequence = Some(*seq as u64);
                }
                if let Some(Value::Bool(all)) = config_map.get("consume_all") {
                    consume_all = *all;
                }
            }
        }

        // Consume events from storage
        let events = {
            let mut storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "stream_operations".to_string(),
                debug_info: "ConsumeStreamFunction::execute".to_string(),
            })?;

            let stream_events = storage.get_mut(&stream_name).ok_or_else(|| HyperQLError::ValidationError {
                message: format!("Stream '{}' does not exist", stream_name),
                field: Some("stream".to_string()),
            })?;

            let mut result_events = Vec::new();
            let mut collected = 0;

            // Filter events based on sequence number if specified
            let filtered_events: Vec<_> = if let Some(from_seq) = from_sequence {
                stream_events.iter().filter(|e| e.sequence_number >= from_seq).cloned().collect()
            } else {
                stream_events.iter().cloned().collect()
            };

            for event in filtered_events {
                if let Some(max_limit) = limit {
                    if collected >= max_limit {
                        break;
                    }
                }

                // Convert event to Value
                let mut event_map = HashMap::new();
                event_map.insert("data".to_string(), event.data.clone());
                event_map.insert("timestamp".to_string(), Value::Timestamp(event.timestamp.timestamp_millis()));
                event_map.insert("sequence_number".to_string(), Value::Int(event.sequence_number as i64));

                result_events.push(Value::Map(event_map));
                collected += 1;
            }

            // Optionally remove consumed events
            if consume_all {
                let remove_count = if let Some(max_limit) = limit {
                    std::cmp::min(max_limit, result_events.len())
                } else {
                    result_events.len()
                };

                if let Some(from_seq) = from_sequence {
                    // Remove only events >= from_sequence that were consumed
                    let mut remaining_events = VecDeque::new();
                    let mut consumed_count = 0;

                    for event in stream_events.iter() {
                        if event.sequence_number >= from_seq && consumed_count < remove_count {
                            consumed_count += 1;
                        } else {
                            remaining_events.push_back(event.clone());
                        }
                    }
                    *stream_events = remaining_events;
                } else {
                    // Remove from front
                    for _ in 0..remove_count {
                        stream_events.pop_front();
                    }
                }
            }

            result_events
        };

        Ok(Value::List(events))
    }
    
    fn name(&self) -> &str {
        "stream_consume"
    }
    
    fn signature(&self) -> &str {
        "stream_consume(stream: string, config: object) -> array"
    }
}

// Convenience functions for common operations

/// Create a new stream
pub fn stream_create(name: String, config: HashMap<String, Value>) -> Result<Value> {
    let function = CreateStreamFunction;
    let args = vec![
        Value::String(name),
        Value::Map(config),
    ];
    function.execute(args)
}

/// Produce an event to a stream
pub fn stream_produce(stream: String, event_data: HashMap<String, Value>) -> Result<Value> {
    let function = ProduceEventFunction;
    let args = vec![
        Value::String(stream),
        Value::Map(event_data),
    ];
    function.execute(args)
}

/// Consume events from a stream
pub fn stream_consume(stream: String, config: HashMap<String, Value>) -> Result<Value> {
    let function = ConsumeStreamFunction;
    let args = vec![
        Value::String(stream),
        Value::Map(config),
    ];
    function.execute(args)
}