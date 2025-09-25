//! # Trigger AST Nodes
//!
//! AST nodes for trigger definitions in HyperQL.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CREATE TRIGGER statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTriggerStatement {
    /// Trigger name
    pub name: String,
    
    /// Trigger timing
    pub timing: TriggerTiming,
    
    /// Trigger event
    pub event: TriggerEvent,
    
    /// Trigger condition
    pub condition: Option<TriggerCondition>,
    
    /// Trigger actions
    pub actions: Vec<TriggerAction>,
    
    /// Trigger configuration
    pub config: TriggerConfig,
}

/// Trigger timing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerTiming {
    /// Before the triggering event
    Before,
    
    /// After the triggering event
    After,
    
    /// Instead of the triggering event
    InsteadOf,
    
    /// On schedule
    Scheduled(ScheduleExpression),
}

/// Schedule expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduleExpression {
    /// Cron expression
    Cron(String),
    
    /// Interval in seconds
    Interval(u64),
    
    /// At specific time
    At(String),
}

/// Trigger events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerEvent {
    /// Entity operations
    EntityOperation {
        entity_type: Option<String>,
        operations: Vec<String>,
    },
    
    /// Measure updates
    MeasureUpdate {
        measure_name: String,
        threshold: Option<TriggerThreshold>,
    },
    
    /// Stream events
    StreamEvent {
        stream_name: String,
        event_pattern: Option<String>,
    },
    
    /// Query result changes
    QueryChange {
        query: String,
        change_type: QueryChangeType,
    },
    
    /// Cascade events
    CascadeEvent {
        cascade_type: String,
    },
    
    /// Position changes
    PositionChange {
        region: Option<HyperbolicRegionSpec>,
    },
    
    /// Custom events
    CustomEvent {
        event_type: String,
        parameters: HashMap<String, String>,
    },
}

/// Trigger thresholds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerThreshold {
    /// Greater than
    GreaterThan(f64),
    
    /// Less than
    LessThan(f64),
    
    /// Between values
    Between(f64, f64),
    
    /// Percentage change
    PercentChange(f64),
    
    /// Absolute change
    AbsoluteChange(f64),
}

/// Query change types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryChangeType {
    /// Result set size changed
    ResultSetSize,
    
    /// New results appeared
    NewResults,
    
    /// Results were removed
    RemovedResults,
    
    /// Aggregate value changed
    AggregateChange,
}

/// Hyperbolic region specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HyperbolicRegionSpec {
    /// Center coordinates
    pub center: Vec<f64>,
    
    /// Radius
    pub radius: f64,
}

/// Trigger conditions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Boolean expression
    Expression(String),
    
    /// HyperQL query
    Query(String),
    
    /// Hyperbolic distance condition
    HyperbolicDistance {
        center: Vec<f64>,
        radius: f64,
        comparison: String,
    },
    
    /// Composite conditions
    And(Vec<TriggerCondition>),
    Or(Vec<TriggerCondition>),
    Not(Box<TriggerCondition>),
}

/// Trigger actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerAction {
    /// Execute function
    ExecuteFunction {
        name: String,
        parameters: HashMap<String, String>,
    },
    
    /// Generate stream event
    GenerateEvent {
        stream: String,
        event_data: HashMap<String, String>,
    },
    
    /// Start cascade
    StartCascade {
        measure_name: String,
        initial_value: f64,
    },
    
    /// Call webhook
    CallWebhook {
        url: String,
        method: String,
        headers: HashMap<String, String>,
    },
    
    /// Execute query
    ExecuteQuery(String),
    
    /// Send notification
    SendNotification {
        recipient: String,
        message: String,
        channel: String,
    },
}

/// Trigger configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Execution priority
    pub priority: Option<String>,
    
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
    
    /// Max retry attempts
    pub max_retries: Option<u32>,
    
    /// Debounce period
    pub debounce_ms: Option<u64>,
    
    /// Cooldown period
    pub cooldown_ms: Option<u64>,
    
    /// Enable/disable flag
    pub enabled: Option<bool>,
}