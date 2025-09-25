//! # Window Functions for Streams
//!
//! Window functions for time-based and hyperbolic distance-based stream processing.

use crate::types::{Value, Position3D};
use crate::{HyperQLError, Result};
use std::collections::{HashMap, BTreeMap};
use chrono::Duration;
use super::operations::STREAM_STORAGE;

/// Aggregation function types
#[derive(Debug, Clone)]
enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    First,
    Last,
}

impl AggregateFunction {
    fn from_string(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "count" => Ok(AggregateFunction::Count),
            "sum" => Ok(AggregateFunction::Sum),
            "avg" | "average" => Ok(AggregateFunction::Avg),
            "min" => Ok(AggregateFunction::Min),
            "max" => Ok(AggregateFunction::Max),
            "first" => Ok(AggregateFunction::First),
            "last" => Ok(AggregateFunction::Last),
            _ => Err(HyperQLError::ValidationError {
                message: format!("Unknown aggregate function: {}", s),
                field: Some("aggregate_fn".to_string()),
            }),
        }
    }

    fn apply(&self, values: &[Value]) -> Result<Value> {
        if values.is_empty() {
            return Ok(Value::Null);
        }

        match self {
            AggregateFunction::Count => Ok(Value::Int(values.len() as i64)),
            AggregateFunction::Sum => {
                let mut sum = 0.0;
                for value in values {
                    match value {
                        Value::Int(i) => sum += *i as f64,
                        Value::Float(f) => sum += f,
                        _ => return Err(HyperQLError::TypeError {
                            expected: "numeric".to_string(),
                            found: format!("{:?}", value),
                            context: "sum aggregation".to_string(),
                        }),
                    }
                }
                Ok(Value::Float(sum))
            },
            AggregateFunction::Avg => {
                let sum = self.apply(values)?;
                if let Value::Float(s) = sum {
                    Ok(Value::Float(s / values.len() as f64))
                } else {
                    Err(HyperQLError::InternalError {
                        message: "Sum aggregation returned non-float".to_string(),
                        component: "windowing".to_string(),
                        debug_info: "AggregateFunction::apply".to_string(),
                    })
                }
            },
            AggregateFunction::Min => {
                let mut min_val = values[0].clone();
                for value in values.iter().skip(1) {
                    if self.compare_values(value, &min_val)? {
                        min_val = value.clone();
                    }
                }
                Ok(min_val)
            },
            AggregateFunction::Max => {
                let mut max_val = values[0].clone();
                for value in values.iter().skip(1) {
                    if !self.compare_values(value, &max_val)? {
                        max_val = value.clone();
                    }
                }
                Ok(max_val)
            },
            AggregateFunction::First => Ok(values[0].clone()),
            AggregateFunction::Last => Ok(values[values.len() - 1].clone()),
        }
    }

    fn compare_values(&self, a: &Value, b: &Value) -> Result<bool> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(a < b),
            (Value::Float(a), Value::Float(b)) => Ok(a < b),
            (Value::Int(a), Value::Float(b)) => Ok((*a as f64) < *b),
            (Value::Float(a), Value::Int(b)) => Ok(*a < (*b as f64)),
            (Value::String(a), Value::String(b)) => Ok(a < b),
            (Value::Timestamp(a), Value::Timestamp(b)) => Ok(a < b),
            _ => Err(HyperQLError::TypeError {
                expected: "comparable types".to_string(),
                found: format!("{:?} and {:?}", a, b),
                context: "value comparison".to_string(),
            }),
        }
    }
}

/// Window function trait
pub trait WindowFunction {
    /// Execute the window function
    fn execute(&self, args: Vec<Value>) -> Result<Value>;

    /// Get function name
    fn name(&self) -> &str;

    /// Get function signature
    fn signature(&self) -> &str;
}

/// Tumbling window function
pub struct TumblingWindowFunction;

impl WindowFunction for TumblingWindowFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 3 {
            return Err(HyperQLError::ValidationError {
                message: "Tumbling window requires stream name, duration, and aggregate function".to_string(),
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

        let window_duration_ms = match &args[1] {
            Value::Int(duration) => *duration,
            _ => return Err(HyperQLError::TypeError {
                expected: "integer".to_string(),
                found: format!("{:?}", args[1]),
                context: "window duration".to_string(),
            }),
        };

        let aggregate_fn_name = match &args[2] {
            Value::String(fn_name) => fn_name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[2]),
                context: "aggregate function".to_string(),
            }),
        };

        let aggregate_fn = AggregateFunction::from_string(&aggregate_fn_name)?;

        // Get stream events
        let events = {
            let storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "windowing".to_string(),
                debug_info: "TumblingWindowFunction::execute".to_string(),
            })?;

            let stream_events = storage.get(&stream_name).ok_or_else(|| HyperQLError::ValidationError {
                message: format!("Stream '{}' does not exist", stream_name),
                field: Some("stream".to_string()),
            })?;

            stream_events.iter().cloned().collect::<Vec<_>>()
        };

        if events.is_empty() {
            return Ok(Value::List(vec![]));
        }

        // Group events into tumbling windows
        let window_duration = Duration::milliseconds(window_duration_ms);
        let start_time = events[0].timestamp;
        let mut windows: BTreeMap<i64, Vec<Value>> = BTreeMap::new();

        for event in events {
            let elapsed = event.timestamp.signed_duration_since(start_time);
            let window_id = elapsed.num_milliseconds() / window_duration_ms;
            windows.entry(window_id).or_insert_with(Vec::new).push(event.data);
        }

        // Apply aggregation to each window
        let mut results = Vec::new();
        for (window_id, values) in windows {
            let window_start = start_time + Duration::milliseconds(window_id * window_duration_ms);
            let window_end = window_start + window_duration;
            let aggregated = aggregate_fn.apply(&values)?;

            let mut window_result = HashMap::new();
            window_result.insert("window_start".to_string(), Value::Timestamp(window_start.timestamp_millis()));
            window_result.insert("window_end".to_string(), Value::Timestamp(window_end.timestamp_millis()));
            window_result.insert("value".to_string(), aggregated);
            window_result.insert("count".to_string(), Value::Int(values.len() as i64));

            results.push(Value::Map(window_result));
        }

        Ok(Value::List(results))
    }
    
    fn name(&self) -> &str {
        "window_tumbling"
    }
    
    fn signature(&self) -> &str {
        "window_tumbling(stream: string, duration: interval, aggregate_fn: string) -> stream"
    }
}

/// Sliding window function
pub struct SlidingWindowFunction;

impl WindowFunction for SlidingWindowFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 4 {
            return Err(HyperQLError::ValidationError {
                message: "Sliding window requires stream name, duration, slide interval, and aggregate function".to_string(),
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

        let window_duration_ms = match &args[1] {
            Value::Int(duration) => *duration,
            _ => return Err(HyperQLError::TypeError {
                expected: "integer".to_string(),
                found: format!("{:?}", args[1]),
                context: "window duration".to_string(),
            }),
        };

        let slide_interval_ms = match &args[2] {
            Value::Int(slide) => *slide,
            _ => return Err(HyperQLError::TypeError {
                expected: "integer".to_string(),
                found: format!("{:?}", args[2]),
                context: "slide interval".to_string(),
            }),
        };

        let aggregate_fn_name = match &args[3] {
            Value::String(fn_name) => fn_name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[3]),
                context: "aggregate function".to_string(),
            }),
        };

        let aggregate_fn = AggregateFunction::from_string(&aggregate_fn_name)?;

        // Get stream events
        let events = {
            let storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "windowing".to_string(),
                debug_info: "SlidingWindowFunction::execute".to_string(),
            })?;

            let stream_events = storage.get(&stream_name).ok_or_else(|| HyperQLError::ValidationError {
                message: format!("Stream '{}' does not exist", stream_name),
                field: Some("stream".to_string()),
            })?;

            stream_events.iter().cloned().collect::<Vec<_>>()
        };

        if events.is_empty() {
            return Ok(Value::List(vec![]));
        }

        // Create sliding windows
        let window_duration = Duration::milliseconds(window_duration_ms);
        let slide_interval = Duration::milliseconds(slide_interval_ms);
        let start_time = events[0].timestamp;
        let end_time = events[events.len() - 1].timestamp;

        let mut results = Vec::new();
        let mut current_window_start = start_time;

        while current_window_start <= end_time {
            let window_end = current_window_start + window_duration;

            // Collect events in this window
            let window_events: Vec<Value> = events
                .iter()
                .filter(|e| e.timestamp >= current_window_start && e.timestamp < window_end)
                .map(|e| e.data.clone())
                .collect();

            if !window_events.is_empty() {
                let aggregated = aggregate_fn.apply(&window_events)?;

                let mut window_result = HashMap::new();
                window_result.insert("window_start".to_string(), Value::Timestamp(current_window_start.timestamp_millis()));
                window_result.insert("window_end".to_string(), Value::Timestamp(window_end.timestamp_millis()));
                window_result.insert("value".to_string(), aggregated);
                window_result.insert("count".to_string(), Value::Int(window_events.len() as i64));

                results.push(Value::Map(window_result));
            }

            current_window_start = current_window_start + slide_interval;
        }

        Ok(Value::List(results))
    }
    
    fn name(&self) -> &str {
        "window_sliding"
    }
    
    fn signature(&self) -> &str {
        "window_sliding(stream: string, duration: interval, slide: interval, aggregate_fn: string) -> stream"
    }
}

/// Hyperbolic window function
pub struct HyperbolicWindowFunction;

impl WindowFunction for HyperbolicWindowFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 4 {
            return Err(HyperQLError::ValidationError {
                message: "Hyperbolic window requires stream name, center position, radius, and aggregate function".to_string(),
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

        let center = match &args[1] {
            Value::List(coords) => {
                if coords.len() != 3 {
                    return Err(HyperQLError::ValidationError {
                        message: "Center position must be a 3D coordinate [x, y, z]".to_string(),
                        field: Some("center".to_string()),
                    });
                }
                let x = match &coords[0] {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return Err(HyperQLError::TypeError {
                        expected: "number".to_string(),
                        found: format!("{:?}", coords[0]),
                        context: "x coordinate".to_string(),
                    }),
                };
                let y = match &coords[1] {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return Err(HyperQLError::TypeError {
                        expected: "number".to_string(),
                        found: format!("{:?}", coords[1]),
                        context: "y coordinate".to_string(),
                    }),
                };
                let z = match &coords[2] {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return Err(HyperQLError::TypeError {
                        expected: "number".to_string(),
                        found: format!("{:?}", coords[2]),
                        context: "z coordinate".to_string(),
                    }),
                };
                Position3D { x, y, z }
            },
            _ => return Err(HyperQLError::TypeError {
                expected: "list of coordinates".to_string(),
                found: format!("{:?}", args[1]),
                context: "center position".to_string(),
            }),
        };

        let radius = match &args[2] {
            Value::Float(r) => *r,
            Value::Int(i) => *i as f64,
            _ => return Err(HyperQLError::TypeError {
                expected: "number".to_string(),
                found: format!("{:?}", args[2]),
                context: "radius".to_string(),
            }),
        };

        let aggregate_fn_name = match &args[3] {
            Value::String(fn_name) => fn_name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[3]),
                context: "aggregate function".to_string(),
            }),
        };

        let aggregate_fn = AggregateFunction::from_string(&aggregate_fn_name)?;

        // Get stream events
        let events = {
            let storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "windowing".to_string(),
                debug_info: "HyperbolicWindowFunction::execute".to_string(),
            })?;

            let stream_events = storage.get(&stream_name).ok_or_else(|| HyperQLError::ValidationError {
                message: format!("Stream '{}' does not exist", stream_name),
                field: Some("stream".to_string()),
            })?;

            stream_events.iter().cloned().collect::<Vec<_>>()
        };

        // Filter events within hyperbolic distance
        let mut window_events = Vec::new();
        for event in events {
            // Check if event data contains position information
            if let Value::Map(event_map) = &event.data {
                if let Some(Value::Position(pos)) = event_map.get("position") {
                    match hyperbolic_distance(&center, pos) {
                        Ok(distance) if distance <= radius => {
                            window_events.push(event.data.clone());
                        }
                        Ok(_) => {}, // Distance exceeds radius
                        Err(_) => {}, // Skip invalid positions (maintain metric consistency)
                    }
                } else if let (Some(Value::Float(x)), Some(Value::Float(y)), Some(Value::Float(z))) =
                    (event_map.get("x"), event_map.get("y"), event_map.get("z")) {
                    let pos = Position3D { x: *x, y: *y, z: *z };
                    match hyperbolic_distance(&center, &pos) {
                        Ok(distance) if distance <= radius => {
                            window_events.push(event.data.clone());
                        }
                        Ok(_) => {}, // Distance exceeds radius
                        Err(_) => {}, // Skip invalid positions (maintain metric consistency)
                    }
                }
            }
        }

        if window_events.is_empty() {
            let mut result = HashMap::new();
            result.insert("center".to_string(), Value::List(vec![Value::Float(center.x), Value::Float(center.y), Value::Float(center.z)]));
            result.insert("radius".to_string(), Value::Float(radius));
            result.insert("value".to_string(), Value::Null);
            result.insert("count".to_string(), Value::Int(0));
            return Ok(Value::Map(result));
        }

        // Apply aggregation
        let aggregated = aggregate_fn.apply(&window_events)?;

        let mut result = HashMap::new();
        result.insert("center".to_string(), Value::List(vec![Value::Float(center.x), Value::Float(center.y), Value::Float(center.z)]));
        result.insert("radius".to_string(), Value::Float(radius));
        result.insert("value".to_string(), aggregated);
        result.insert("count".to_string(), Value::Int(window_events.len() as i64));

        Ok(Value::Map(result))
    }
    
    fn name(&self) -> &str {
        "window_hyperbolic"
    }
    
    fn signature(&self) -> &str {
        "window_hyperbolic(stream: string, center: position, radius: float, aggregate_fn: string) -> stream"
    }
}

/// Session window function
pub struct SessionWindowFunction;

impl WindowFunction for SessionWindowFunction {
    fn execute(&self, args: Vec<Value>) -> Result<Value> {
        if args.len() < 3 {
            return Err(HyperQLError::ValidationError {
                message: "Session window requires stream name, timeout, and aggregate function".to_string(),
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

        let timeout_ms = match &args[1] {
            Value::Int(timeout) => *timeout,
            _ => return Err(HyperQLError::TypeError {
                expected: "integer".to_string(),
                found: format!("{:?}", args[1]),
                context: "timeout".to_string(),
            }),
        };

        let aggregate_fn_name = match &args[2] {
            Value::String(fn_name) => fn_name.clone(),
            _ => return Err(HyperQLError::TypeError {
                expected: "string".to_string(),
                found: format!("{:?}", args[2]),
                context: "aggregate function".to_string(),
            }),
        };

        let aggregate_fn = AggregateFunction::from_string(&aggregate_fn_name)?;

        // Get stream events
        let events = {
            let storage = STREAM_STORAGE.lock().map_err(|_| HyperQLError::InternalError {
                message: "Failed to acquire stream storage lock".to_string(),
                component: "windowing".to_string(),
                debug_info: "SessionWindowFunction::execute".to_string(),
            })?;

            let stream_events = storage.get(&stream_name).ok_or_else(|| HyperQLError::ValidationError {
                message: format!("Stream '{}' does not exist", stream_name),
                field: Some("stream".to_string()),
            })?;

            let mut evts = stream_events.iter().cloned().collect::<Vec<_>>();
            evts.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            evts
        };

        if events.is_empty() {
            return Ok(Value::List(vec![]));
        }

        // Group events into sessions based on timeout
        let timeout_duration = Duration::milliseconds(timeout_ms);
        let mut sessions = Vec::new();
        let mut current_session = Vec::new();
        let mut last_event_time = events[0].timestamp;

        for event in events {
            let time_gap = event.timestamp.signed_duration_since(last_event_time);

            if time_gap > timeout_duration && !current_session.is_empty() {
                // End current session and start new one
                sessions.push(std::mem::take(&mut current_session));
            }

            current_session.push(event.clone());
            last_event_time = event.timestamp;
        }

        // Add final session
        if !current_session.is_empty() {
            sessions.push(current_session);
        }

        // Apply aggregation to each session
        let mut results = Vec::new();
        for (session_id, session_events) in sessions.into_iter().enumerate() {
            if session_events.is_empty() {
                continue;
            }

            let session_start = session_events[0].timestamp;
            let session_end = session_events[session_events.len() - 1].timestamp;
            let session_values: Vec<Value> = session_events.into_iter().map(|e| e.data).collect();

            let aggregated = aggregate_fn.apply(&session_values)?;

            let mut session_result = HashMap::new();
            session_result.insert("session_id".to_string(), Value::Int(session_id as i64));
            session_result.insert("session_start".to_string(), Value::Timestamp(session_start.timestamp_millis()));
            session_result.insert("session_end".to_string(), Value::Timestamp(session_end.timestamp_millis()));
            session_result.insert("value".to_string(), aggregated);
            session_result.insert("count".to_string(), Value::Int(session_values.len() as i64));

            results.push(Value::Map(session_result));
        }

        Ok(Value::List(results))
    }
    
    fn name(&self) -> &str {
        "window_session"
    }
    
    fn signature(&self) -> &str {
        "window_session(stream: string, timeout: interval, aggregate_fn: string) -> stream"
    }
}

// Convenience functions for window operations

// Helper function for hyperbolic distance calculation
fn hyperbolic_distance(pos1: &Position3D, pos2: &Position3D) -> Result<f64> {
    // Use the proper hyperbolic distance implementation from geometric module
    // No fallback to maintain metric consistency
    crate::functions::geometric::distance::hyperbolic_distance(pos1, pos2)
}

/// Create a tumbling window
pub fn window_tumbling(stream: String, duration: i64, aggregate_fn: String) -> Result<Value> {
    let function = TumblingWindowFunction;
    let args = vec![
        Value::String(stream),
        Value::Int(duration),
        Value::String(aggregate_fn),
    ];
    function.execute(args)
}

/// Create a sliding window
pub fn window_sliding(
    stream: String,
    duration: i64,
    slide: i64,
    aggregate_fn: String,
) -> Result<Value> {
    let function = SlidingWindowFunction;
    let args = vec![
        Value::String(stream),
        Value::Int(duration),
        Value::Int(slide),
        Value::String(aggregate_fn),
    ];
    function.execute(args)
}

/// Create a hyperbolic window
pub fn window_hyperbolic(
    stream: String,
    center: Vec<f64>,
    radius: f64,
    aggregate_fn: String,
) -> Result<Value> {
    let function = HyperbolicWindowFunction;
    let args = vec![
        Value::String(stream),
        Value::List(center.into_iter().map(Value::Float).collect()),
        Value::Float(radius),
        Value::String(aggregate_fn),
    ];
    function.execute(args)
}