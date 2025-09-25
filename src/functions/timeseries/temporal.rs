//! Temporal Functions
//!
//! Functions for temporal data processing including:
//! - time_bucket() for interval-based grouping
//! - time_diff() for time difference calculations
//! - extract() for timestamp component extraction
//! - date_trunc() for timestamp truncation
//! - timezone_convert() for time zone conversions
//! - Integration with hyperbolic trajectory analysis
//! - High-precision temporal arithmetic

use crate::{HyperQLError, Result};
use crate::types::Value;
use chrono::{DateTime, Utc, TimeZone, Datelike, Timelike, Duration};
use std::collections::HashMap;

/// Time bucket interval types
#[derive(Debug, Clone)]
pub enum TimeBucketInterval {
    Second(i64),
    Minute(i64),
    Hour(i64),
    Day(i64),
    Week(i64),
    Month(i64),
    Year(i64),
}

impl TimeBucketInterval {
    pub fn from_string(s: &str, value: i64) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "second" | "seconds" | "s" => Ok(TimeBucketInterval::Second(value)),
            "minute" | "minutes" | "min" | "m" => Ok(TimeBucketInterval::Minute(value)),
            "hour" | "hours" | "h" => Ok(TimeBucketInterval::Hour(value)),
            "day" | "days" | "d" => Ok(TimeBucketInterval::Day(value)),
            "week" | "weeks" | "w" => Ok(TimeBucketInterval::Week(value)),
            "month" | "months" | "mon" => Ok(TimeBucketInterval::Month(value)),
            "year" | "years" | "y" => Ok(TimeBucketInterval::Year(value)),
            _ => Err(HyperQLError::ValidationError {
                message: format!("Unknown time interval: {}", s),
                field: Some("interval".to_string()),
            }),
        }
    }

    pub fn bucket_timestamp(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            TimeBucketInterval::Second(interval) => {
                let seconds = timestamp.timestamp();
                let bucketed_seconds = (seconds / interval) * interval;
                Utc.timestamp_opt(bucketed_seconds, 0).unwrap()
            },
            TimeBucketInterval::Minute(interval) => {
                let minutes = timestamp.timestamp() / 60;
                let bucketed_minutes = (minutes / interval) * interval;
                Utc.timestamp_opt(bucketed_minutes * 60, 0).unwrap()
            },
            TimeBucketInterval::Hour(interval) => {
                let hours = timestamp.timestamp() / 3600;
                let bucketed_hours = (hours / interval) * interval;
                Utc.timestamp_opt(bucketed_hours * 3600, 0).unwrap()
            },
            TimeBucketInterval::Day(interval) => {
                let days = timestamp.timestamp() / 86400;
                let bucketed_days = (days / interval) * interval;
                Utc.timestamp_opt(bucketed_days * 86400, 0).unwrap()
            },
            TimeBucketInterval::Week(interval) => {
                let weeks = timestamp.timestamp() / (86400 * 7);
                let bucketed_weeks = (weeks / interval) * interval;
                Utc.timestamp_opt(bucketed_weeks * 86400 * 7, 0).unwrap()
            },
            TimeBucketInterval::Month(interval) => {
                let year = timestamp.year();
                let month = timestamp.month() as i32;
                let bucketed_month = ((month - 1) / (*interval as i32)) * (*interval as i32) + 1;
                Utc.with_ymd_and_hms(year, bucketed_month as u32, 1, 0, 0, 0).unwrap()
            },
            TimeBucketInterval::Year(interval) => {
                let year = timestamp.year();
                let bucketed_year = (year / (*interval as i32)) * (*interval as i32);
                Utc.with_ymd_and_hms(bucketed_year, 1, 1, 0, 0, 0).unwrap()
            },
        }
    }
}

/// Group timestamps into time buckets
/// Parameters: (timestamp, interval_value, interval_unit)
pub fn time_bucket(timestamp: i64, interval_value: i64, interval_unit: &str) -> Result<Value> {
    let ts = Utc.timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
        .single()
        .ok_or_else(|| HyperQLError::ValidationError {
            message: "Invalid timestamp".to_string(),
            field: Some("timestamp".to_string()),
        })?;

    let interval = TimeBucketInterval::from_string(interval_unit, interval_value)?;
    let bucketed = interval.bucket_timestamp(ts);

    Ok(Value::Timestamp(bucketed.timestamp_millis()))
}

/// Group a list of timestamps into time buckets
pub fn time_bucket_batch(timestamps: Vec<Value>, interval_value: i64, interval_unit: &str) -> Result<Value> {
    let interval = TimeBucketInterval::from_string(interval_unit, interval_value)?;
    let mut buckets: HashMap<i64, Vec<Value>> = HashMap::new();

    for ts_value in timestamps {
        match ts_value {
            Value::Timestamp(ts) => {
                let timestamp = Utc.timestamp_opt(ts / 1000, ((ts % 1000) * 1_000_000) as u32)
                    .single()
                    .ok_or_else(|| HyperQLError::ValidationError {
                        message: "Invalid timestamp".to_string(),
                        field: Some("timestamp".to_string()),
                    })?;

                let bucketed = interval.bucket_timestamp(timestamp);
                let bucket_key = bucketed.timestamp_millis();
                buckets.entry(bucket_key).or_insert_with(Vec::new).push(Value::Timestamp(ts));
            },
            _ => return Err(HyperQLError::TypeError {
                expected: "timestamp".to_string(),
                found: format!("{:?}", ts_value),
                context: "time_bucket_batch".to_string(),
            }),
        }
    }

    let mut result = Vec::new();
    for (bucket_ts, values) in buckets {
        let mut bucket_result = HashMap::new();
        bucket_result.insert("bucket".to_string(), Value::Timestamp(bucket_ts));
        let count = values.len() as i64;
        bucket_result.insert("values".to_string(), Value::List(values));
        bucket_result.insert("count".to_string(), Value::Int(count));
        result.push(Value::Map(bucket_result));
    }

    Ok(Value::List(result))
}

/// Calculate time differences
/// Parameters: (timestamp1, timestamp2, unit)
pub fn time_diff(timestamp1: i64, timestamp2: i64, unit: Option<&str>) -> Result<Value> {
    let ts1 = Utc.timestamp_opt(timestamp1 / 1000, ((timestamp1 % 1000) * 1_000_000) as u32)
        .single()
        .ok_or_else(|| HyperQLError::ValidationError {
            message: "Invalid timestamp1".to_string(),
            field: Some("timestamp1".to_string()),
        })?;

    let ts2 = Utc.timestamp_opt(timestamp2 / 1000, ((timestamp2 % 1000) * 1_000_000) as u32)
        .single()
        .ok_or_else(|| HyperQLError::ValidationError {
            message: "Invalid timestamp2".to_string(),
            field: Some("timestamp2".to_string()),
        })?;

    let duration = ts2.signed_duration_since(ts1);

    let result = match unit.unwrap_or("milliseconds").to_lowercase().as_str() {
        "milliseconds" | "millis" | "ms" => duration.num_milliseconds(),
        "seconds" | "s" => duration.num_seconds(),
        "minutes" | "min" | "m" => duration.num_minutes(),
        "hours" | "h" => duration.num_hours(),
        "days" | "d" => duration.num_days(),
        "weeks" | "w" => duration.num_weeks(),
        _ => return Err(HyperQLError::ValidationError {
            message: format!("Unknown time unit: {}", unit.unwrap_or("milliseconds")),
            field: Some("unit".to_string()),
        }),
    };

    Ok(Value::Int(result))
}

/// Calculate time difference with Value parameters
pub fn time_diff_values(ts1: Value, ts2: Value, unit: Option<String>) -> Result<Value> {
    let timestamp1 = match ts1 {
        Value::Timestamp(ts) => ts,
        _ => return Err(HyperQLError::TypeError {
            expected: "timestamp".to_string(),
            found: format!("{:?}", ts1),
            context: "time_diff timestamp1".to_string(),
        }),
    };

    let timestamp2 = match ts2 {
        Value::Timestamp(ts) => ts,
        _ => return Err(HyperQLError::TypeError {
            expected: "timestamp".to_string(),
            found: format!("{:?}", ts2),
            context: "time_diff timestamp2".to_string(),
        }),
    };

    time_diff(timestamp1, timestamp2, unit.as_deref())
}

/// Extract components from timestamps
/// Parameters: (timestamp, component)
pub fn extract(timestamp: i64, component: &str) -> Result<Value> {
    let ts = Utc.timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
        .single()
        .ok_or_else(|| HyperQLError::ValidationError {
            message: "Invalid timestamp".to_string(),
            field: Some("timestamp".to_string()),
        })?;

    let result = match component.to_lowercase().as_str() {
        "year" | "y" => ts.year() as i64,
        "month" | "mon" => ts.month() as i64,
        "day" | "d" => ts.day() as i64,
        "hour" | "h" => ts.hour() as i64,
        "minute" | "min" | "m" => ts.minute() as i64,
        "second" | "s" => ts.second() as i64,
        "millisecond" | "millis" | "ms" => (ts.nanosecond() / 1_000_000) as i64,
        "microsecond" | "micros" | "us" => (ts.nanosecond() / 1_000) as i64,
        "nanosecond" | "nanos" | "ns" => ts.nanosecond() as i64,
        "weekday" | "dow" => ts.weekday().num_days_from_monday() as i64,
        "yearday" | "doy" => ts.ordinal() as i64,
        "week" | "w" => {
            let start_of_year = Utc.with_ymd_and_hms(ts.year(), 1, 1, 0, 0, 0).unwrap();
            let days_since_start = ts.signed_duration_since(start_of_year).num_days();
            (days_since_start / 7) + 1
        },
        "quarter" | "q" => ((ts.month() - 1) / 3 + 1) as i64,
        "epoch" => ts.timestamp(),
        "epoch_ms" => ts.timestamp_millis(),
        _ => return Err(HyperQLError::ValidationError {
            message: format!("Unknown timestamp component: {}", component),
            field: Some("component".to_string()),
        }),
    };

    Ok(Value::Int(result))
}

/// Extract component from timestamp Value
pub fn extract_value(ts: Value, component: String) -> Result<Value> {
    let timestamp = match ts {
        Value::Timestamp(ts) => ts,
        _ => return Err(HyperQLError::TypeError {
            expected: "timestamp".to_string(),
            found: format!("{:?}", ts),
            context: "extract timestamp".to_string(),
        }),
    };

    extract(timestamp, &component)
}

/// Truncate timestamp to specified precision
pub fn date_trunc(timestamp: i64, precision: &str) -> Result<Value> {
    let ts = Utc.timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
        .single()
        .ok_or_else(|| HyperQLError::ValidationError {
            message: "Invalid timestamp".to_string(),
            field: Some("timestamp".to_string()),
        })?;

    let truncated = match precision.to_lowercase().as_str() {
        "second" | "s" => ts.with_nanosecond(0).unwrap(),
        "minute" | "min" | "m" => ts.with_second(0).unwrap().with_nanosecond(0).unwrap(),
        "hour" | "h" => ts.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap(),
        "day" | "d" => ts.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap(),
        "week" | "w" => {
            let days_to_subtract = ts.weekday().num_days_from_monday();
            let start_of_week = ts - Duration::days(days_to_subtract as i64);
            start_of_week.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap()
        },
        "month" | "mon" => ts.with_day(1).unwrap().with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap(),
        "quarter" | "q" => {
            let quarter_start_month = ((ts.month() - 1) / 3) * 3 + 1;
            Utc.with_ymd_and_hms(ts.year(), quarter_start_month, 1, 0, 0, 0).unwrap()
        },
        "year" | "y" => Utc.with_ymd_and_hms(ts.year(), 1, 1, 0, 0, 0).unwrap(),
        _ => return Err(HyperQLError::ValidationError {
            message: format!("Unknown truncation precision: {}", precision),
            field: Some("precision".to_string()),
        }),
    };

    Ok(Value::Timestamp(truncated.timestamp_millis()))
}

/// Truncate timestamp Value to specified precision
pub fn date_trunc_value(ts: Value, precision: String) -> Result<Value> {
    let timestamp = match ts {
        Value::Timestamp(ts) => ts,
        _ => return Err(HyperQLError::TypeError {
            expected: "timestamp".to_string(),
            found: format!("{:?}", ts),
            context: "date_trunc timestamp".to_string(),
        }),
    };

    date_trunc(timestamp, &precision)
}

/// Get current timestamp
pub fn now() -> Value {
    Value::Timestamp(Utc::now().timestamp_millis())
}

/// Add duration to timestamp
pub fn timestamp_add(timestamp: i64, amount: i64, unit: &str) -> Result<Value> {
    let ts = Utc.timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
        .single()
        .ok_or_else(|| HyperQLError::ValidationError {
            message: "Invalid timestamp".to_string(),
            field: Some("timestamp".to_string()),
        })?;

    let new_ts = match unit.to_lowercase().as_str() {
        "milliseconds" | "millis" | "ms" => ts + Duration::milliseconds(amount),
        "seconds" | "s" => ts + Duration::seconds(amount),
        "minutes" | "min" | "m" => ts + Duration::minutes(amount),
        "hours" | "h" => ts + Duration::hours(amount),
        "days" | "d" => ts + Duration::days(amount),
        "weeks" | "w" => ts + Duration::weeks(amount),
        _ => return Err(HyperQLError::ValidationError {
            message: format!("Unknown time unit: {}", unit),
            field: Some("unit".to_string()),
        }),
    };

    Ok(Value::Timestamp(new_ts.timestamp_millis()))
}

/// Add duration to timestamp Value
pub fn timestamp_add_value(ts: Value, amount: Value, unit: String) -> Result<Value> {
    let timestamp = match ts {
        Value::Timestamp(ts) => ts,
        _ => return Err(HyperQLError::TypeError {
            expected: "timestamp".to_string(),
            found: format!("{:?}", ts),
            context: "timestamp_add timestamp".to_string(),
        }),
    };

    let amount_val = match amount {
        Value::Int(i) => i,
        _ => return Err(HyperQLError::TypeError {
            expected: "integer".to_string(),
            found: format!("{:?}", amount),
            context: "timestamp_add amount".to_string(),
        }),
    };

    timestamp_add(timestamp, amount_val, &unit)
}