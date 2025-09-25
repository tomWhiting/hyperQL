//! Window Functions
//!
//! Window functions for time series analysis including:
//! - moving_average() for rolling average computations
//! - exponential_smoothing() for weighted averages
//! - lag() and lead() for time-shifted data access
//! - first_value() and last_value() for window boundaries
//! - rank() and row_number() for ordered rankings
//! - Integration with spatial clustering for spatio-temporal windows
//! - Memory-efficient streaming implementations

use crate::{HyperQLError, Result};
use crate::types::Value;
use std::collections::{VecDeque, HashMap};

/// Moving window for time series calculations
#[derive(Debug, Clone)]
struct MovingWindow {
    values: VecDeque<f64>,
    capacity: usize,
    sum: f64,
}

impl MovingWindow {
    fn new(capacity: usize) -> Self {
        Self {
            values: VecDeque::with_capacity(capacity),
            capacity,
            sum: 0.0,
        }
    }

    fn push(&mut self, value: f64) {
        if self.values.len() == self.capacity {
            if let Some(old_value) = self.values.pop_front() {
                self.sum -= old_value;
            }
        }
        self.values.push_back(value);
        self.sum += value;
    }

    fn average(&self) -> Option<f64> {
        if self.values.is_empty() {
            None
        } else {
            Some(self.sum / self.values.len() as f64)
        }
    }

    fn count(&self) -> usize {
        self.values.len()
    }
}

/// Compute moving average over time window
/// Parameters: (values, window_size)
pub fn moving_average(values: Vec<Value>, window_size: usize) -> Result<Value> {
    if window_size == 0 {
        return Err(HyperQLError::ValidationError {
            message: "Window size must be greater than 0".to_string(),
            field: Some("window_size".to_string()),
        });
    }

    if values.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let mut window = MovingWindow::new(window_size);
    let mut results = Vec::new();

    for (i, value) in values.iter().enumerate() {
        let numeric_value = match value {
            Value::Int(i) => *i as f64,
            Value::Float(f) => *f,
            _ => return Err(HyperQLError::TypeError {
                expected: "numeric".to_string(),
                found: format!("{:?}", value),
                context: format!("moving_average at index {}", i),
            }),
        };

        window.push(numeric_value);

        if let Some(avg) = window.average() {
            let mut result = HashMap::new();
            result.insert("index".to_string(), Value::Int(i as i64));
            result.insert("value".to_string(), Value::Float(avg));
            result.insert("window_size".to_string(), Value::Int(window.count() as i64));
            results.push(Value::Map(result));
        }
    }

    Ok(Value::List(results))
}

/// Compute moving average with Value parameters
pub fn moving_average_values(values: Value, window_size: Value) -> Result<Value> {
    let values_list = match values {
        Value::List(list) => list,
        _ => return Err(HyperQLError::TypeError {
            expected: "list".to_string(),
            found: format!("{:?}", values),
            context: "moving_average values".to_string(),
        }),
    };

    let window_size_val = match window_size {
        Value::Int(size) => size as usize,
        _ => return Err(HyperQLError::TypeError {
            expected: "integer".to_string(),
            found: format!("{:?}", window_size),
            context: "moving_average window_size".to_string(),
        }),
    };

    moving_average(values_list, window_size_val)
}

/// Compute exponentially smoothed average
/// Parameters: (values, alpha)
pub fn exponential_smoothing(values: Vec<Value>, alpha: f64) -> Result<Value> {
    if alpha < 0.0 || alpha > 1.0 {
        return Err(HyperQLError::ValidationError {
            message: "Alpha must be between 0.0 and 1.0".to_string(),
            field: Some("alpha".to_string()),
        });
    }

    if values.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let mut results = Vec::new();
    let mut smoothed_value: Option<f64> = None;

    for (i, value) in values.iter().enumerate() {
        let numeric_value = match value {
            Value::Int(i) => *i as f64,
            Value::Float(f) => *f,
            _ => return Err(HyperQLError::TypeError {
                expected: "numeric".to_string(),
                found: format!("{:?}", value),
                context: format!("exponential_smoothing at index {}", i),
            }),
        };

        smoothed_value = match smoothed_value {
            None => Some(numeric_value), // First value
            Some(prev) => Some(alpha * numeric_value + (1.0 - alpha) * prev),
        };

        if let Some(smoothed) = smoothed_value {
            let mut result = HashMap::new();
            result.insert("index".to_string(), Value::Int(i as i64));
            result.insert("value".to_string(), Value::Float(smoothed));
            result.insert("alpha".to_string(), Value::Float(alpha));
            results.push(Value::Map(result));
        }
    }

    Ok(Value::List(results))
}

/// Compute exponentially smoothed average with Value parameters
pub fn exponential_smoothing_values(values: Value, alpha: Value) -> Result<Value> {
    let values_list = match values {
        Value::List(list) => list,
        _ => return Err(HyperQLError::TypeError {
            expected: "list".to_string(),
            found: format!("{:?}", values),
            context: "exponential_smoothing values".to_string(),
        }),
    };

    let alpha_val = match alpha {
        Value::Float(a) => a,
        Value::Int(i) => i as f64,
        _ => return Err(HyperQLError::TypeError {
            expected: "number".to_string(),
            found: format!("{:?}", alpha),
            context: "exponential_smoothing alpha".to_string(),
        }),
    };

    exponential_smoothing(values_list, alpha_val)
}

/// Access lagged values in time series
/// Parameters: (values, lag_periods, default_value)
pub fn lag(values: Vec<Value>, lag_periods: i64, default_value: Option<Value>) -> Result<Value> {
    if lag_periods < 0 {
        return Err(HyperQLError::ValidationError {
            message: "Lag periods must be non-negative".to_string(),
            field: Some("lag_periods".to_string()),
        });
    }

    let lag_size = lag_periods as usize;
    let default_val = default_value.unwrap_or(Value::Null);
    let mut results = Vec::new();

    for (i, value) in values.iter().enumerate() {
        let lagged_value = if i < lag_size {
            default_val.clone()
        } else {
            values[i - lag_size].clone()
        };

        let mut result = HashMap::new();
        result.insert("index".to_string(), Value::Int(i as i64));
        result.insert("current_value".to_string(), value.clone());
        result.insert("lagged_value".to_string(), lagged_value);
        result.insert("lag_periods".to_string(), Value::Int(lag_periods));
        results.push(Value::Map(result));
    }

    Ok(Value::List(results))
}

/// Access lagged values with Value parameters
pub fn lag_values(values: Value, lag_periods: Value, default_value: Option<Value>) -> Result<Value> {
    let values_list = match values {
        Value::List(list) => list,
        _ => return Err(HyperQLError::TypeError {
            expected: "list".to_string(),
            found: format!("{:?}", values),
            context: "lag values".to_string(),
        }),
    };

    let lag_val = match lag_periods {
        Value::Int(lag) => lag,
        _ => return Err(HyperQLError::TypeError {
            expected: "integer".to_string(),
            found: format!("{:?}", lag_periods),
            context: "lag periods".to_string(),
        }),
    };

    lag(values_list, lag_val, default_value)
}

/// Access lead values in time series
/// Parameters: (values, lead_periods, default_value)
pub fn lead(values: Vec<Value>, lead_periods: i64, default_value: Option<Value>) -> Result<Value> {
    if lead_periods < 0 {
        return Err(HyperQLError::ValidationError {
            message: "Lead periods must be non-negative".to_string(),
            field: Some("lead_periods".to_string()),
        });
    }

    let lead_size = lead_periods as usize;
    let default_val = default_value.unwrap_or(Value::Null);
    let mut results = Vec::new();

    for (i, value) in values.iter().enumerate() {
        let lead_value = if i + lead_size >= values.len() {
            default_val.clone()
        } else {
            values[i + lead_size].clone()
        };

        let mut result = HashMap::new();
        result.insert("index".to_string(), Value::Int(i as i64));
        result.insert("current_value".to_string(), value.clone());
        result.insert("lead_value".to_string(), lead_value);
        result.insert("lead_periods".to_string(), Value::Int(lead_periods));
        results.push(Value::Map(result));
    }

    Ok(Value::List(results))
}

/// Access lead values with Value parameters
pub fn lead_values(values: Value, lead_periods: Value, default_value: Option<Value>) -> Result<Value> {
    let values_list = match values {
        Value::List(list) => list,
        _ => return Err(HyperQLError::TypeError {
            expected: "list".to_string(),
            found: format!("{:?}", values),
            context: "lead values".to_string(),
        }),
    };

    let lead_val = match lead_periods {
        Value::Int(lead) => lead,
        _ => return Err(HyperQLError::TypeError {
            expected: "integer".to_string(),
            found: format!("{:?}", lead_periods),
            context: "lead periods".to_string(),
        }),
    };

    lead(values_list, lead_val, default_value)
}

/// Get the first value in a window
pub fn first_value(values: Vec<Value>) -> Result<Value> {
    values.first().cloned().ok_or_else(|| HyperQLError::ValidationError {
        message: "Cannot get first value from empty list".to_string(),
        field: Some("values".to_string()),
    })
}

/// Get the last value in a window
pub fn last_value(values: Vec<Value>) -> Result<Value> {
    values.last().cloned().ok_or_else(|| HyperQLError::ValidationError {
        message: "Cannot get last value from empty list".to_string(),
        field: Some("values".to_string()),
    })
}

/// Compute rolling statistics over a window
pub fn rolling_stats(values: Vec<Value>, window_size: usize, stat_type: &str) -> Result<Value> {
    if window_size == 0 {
        return Err(HyperQLError::ValidationError {
            message: "Window size must be greater than 0".to_string(),
            field: Some("window_size".to_string()),
        });
    }

    if values.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let mut results = Vec::new();

    for i in 0..values.len() {
        let window_start = if i + 1 < window_size { 0 } else { i + 1 - window_size };
        let window_end = i + 1;
        let window_values = &values[window_start..window_end];

        let stat_result = match stat_type.to_lowercase().as_str() {
            "sum" => compute_sum(window_values)?,
            "avg" | "mean" => compute_average(window_values)?,
            "min" => compute_min(window_values)?,
            "max" => compute_max(window_values)?,
            "count" => Value::Int(window_values.len() as i64),
            "std" | "stddev" => compute_stddev(window_values)?,
            "var" | "variance" => compute_variance(window_values)?,
            _ => return Err(HyperQLError::ValidationError {
                message: format!("Unknown statistic type: {}", stat_type),
                field: Some("stat_type".to_string()),
            }),
        };

        let mut result = HashMap::new();
        result.insert("index".to_string(), Value::Int(i as i64));
        result.insert("value".to_string(), stat_result);
        result.insert("window_size".to_string(), Value::Int(window_values.len() as i64));
        results.push(Value::Map(result));
    }

    Ok(Value::List(results))
}

// Helper functions for statistical computations
fn compute_sum(values: &[Value]) -> Result<Value> {
    let mut sum = 0.0;
    for value in values {
        match value {
            Value::Int(i) => sum += *i as f64,
            Value::Float(f) => sum += f,
            _ => return Err(HyperQLError::TypeError {
                expected: "numeric".to_string(),
                found: format!("{:?}", value),
                context: "sum computation".to_string(),
            }),
        }
    }
    Ok(Value::Float(sum))
}

fn compute_average(values: &[Value]) -> Result<Value> {
    if values.is_empty() {
        return Ok(Value::Null);
    }
    let sum = compute_sum(values)?;
    if let Value::Float(s) = sum {
        Ok(Value::Float(s / values.len() as f64))
    } else {
        Err(HyperQLError::InternalError {
            message: "Sum computation returned non-float".to_string(),
            component: "window".to_string(),
            debug_info: "compute_average".to_string(),
        })
    }
}

fn compute_min(values: &[Value]) -> Result<Value> {
    if values.is_empty() {
        return Ok(Value::Null);
    }
    let mut min_val = &values[0];
    for value in values.iter().skip(1) {
        if compare_values(value, min_val)? {
            min_val = value;
        }
    }
    Ok(min_val.clone())
}

fn compute_max(values: &[Value]) -> Result<Value> {
    if values.is_empty() {
        return Ok(Value::Null);
    }
    let mut max_val = &values[0];
    for value in values.iter().skip(1) {
        if !compare_values(value, max_val)? {
            max_val = value;
        }
    }
    Ok(max_val.clone())
}

fn compute_variance(values: &[Value]) -> Result<Value> {
    if values.len() <= 1 {
        return Ok(Value::Float(0.0));
    }

    let mean = compute_average(values)?;
    if let Value::Float(m) = mean {
        let mut sum_sq_diff = 0.0;
        for value in values {
            let numeric_value = match value {
                Value::Int(i) => *i as f64,
                Value::Float(f) => *f,
                _ => return Err(HyperQLError::TypeError {
                    expected: "numeric".to_string(),
                    found: format!("{:?}", value),
                    context: "variance computation".to_string(),
                }),
            };
            let diff = numeric_value - m;
            sum_sq_diff += diff * diff;
        }
        Ok(Value::Float(sum_sq_diff / (values.len() - 1) as f64))
    } else {
        Err(HyperQLError::InternalError {
            message: "Mean computation returned non-float".to_string(),
            component: "window".to_string(),
            debug_info: "compute_variance".to_string(),
        })
    }
}

fn compute_stddev(values: &[Value]) -> Result<Value> {
    let variance = compute_variance(values)?;
    if let Value::Float(var) = variance {
        Ok(Value::Float(var.sqrt()))
    } else {
        Err(HyperQLError::InternalError {
            message: "Variance computation returned non-float".to_string(),
            component: "window".to_string(),
            debug_info: "compute_stddev".to_string(),
        })
    }
}

fn compare_values(a: &Value, b: &Value) -> Result<bool> {
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