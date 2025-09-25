//! # Stream Operation AST Nodes
//!
//! AST nodes for stream operations including CREATE STREAM, PRODUCE, and CONSUME statements.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stream operations in HyperQL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreamOperation {
    /// CREATE STREAM statement
    CreateStream(CreateStreamStatement),
    
    /// PRODUCE statement for generating events
    Produce(ProduceStatement),
    
    /// CONSUME statement for processing events
    Consume(ConsumeStatement),
    
    /// DROP STREAM statement
    DropStream(DropStreamStatement),
    
    /// DESCRIBE STREAM statement
    DescribeStream(DescribeStreamStatement),
}

/// CREATE STREAM statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateStreamStatement {
    /// Stream name
    pub name: String,
    
    /// Stream configuration
    pub config: StreamConfig,
    
    /// Optional source query
    pub source_query: Option<String>,
    
    /// Stream schema definition
    pub schema: Option<StreamSchema>,
}

/// Stream configuration options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Number of partitions
    pub partitions: Option<u32>,
    
    /// Retention period in hours
    pub retention_hours: Option<u64>,
    
    /// Enable hyperbolic positioning
    pub hyperbolic_positioning: Option<bool>,
    
    /// Compression type
    pub compression: Option<String>,
    
    /// Custom properties
    pub properties: HashMap<String, String>,
}

/// Stream schema definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamSchema {
    /// Event fields
    pub fields: Vec<StreamField>,
    
    /// Primary key fields
    pub primary_key: Option<Vec<String>>,
}

/// Stream field definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamField {
    /// Field name
    pub name: String,
    
    /// Field type
    pub field_type: String,
    
    /// Optional flag
    pub optional: bool,
    
    /// Default value
    pub default: Option<String>,
}

/// PRODUCE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProduceStatement {
    /// Target stream name
    pub stream: String,
    
    /// Event data
    pub event_data: EventData,
    
    /// Optional partition key
    pub partition_key: Option<String>,
    
    /// Optional timestamp
    pub timestamp: Option<String>,
}

/// Event data specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventData {
    /// Direct values
    Values(HashMap<String, ValueExpression>),
    
    /// From query result
    FromQuery(String),
    
    /// From entity changes
    FromEntity {
        entity_id: String,
        operation: String,
    },
}

/// Value expressions for event data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueExpression {
    /// Literal value
    Literal(serde_json::Value),
    
    /// Function call
    Function {
        name: String,
        args: Vec<ValueExpression>,
    },
    
    /// Entity property reference
    EntityProperty {
        entity_id: String,
        property: String,
    },
    
    /// Measure value reference
    MeasureValue {
        entity_id: String,
        measure: String,
    },
}

/// CONSUME statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumeStatement {
    /// Source stream name
    pub stream: String,
    
    /// Consumer configuration
    pub config: ConsumerConfig,
    
    /// Processing action
    pub action: ConsumerAction,
    
    /// Optional filter condition
    pub filter: Option<String>,
}

/// Consumer configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Consumer group ID
    pub group_id: String,
    
    /// Consumer ID
    pub consumer_id: Option<String>,
    
    /// Auto-commit interval
    pub auto_commit_interval_ms: Option<u64>,
    
    /// Batch size
    pub batch_size: Option<u32>,
    
    /// Starting position
    pub start_position: Option<String>,
}

/// Consumer actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsumerAction {
    /// Execute function for each event
    ExecuteFunction {
        function_name: String,
        parameters: HashMap<String, String>,
    },
    
    /// Insert events into table
    InsertIntoTable {
        table_name: String,
        field_mapping: HashMap<String, String>,
    },
    
    /// Forward to another stream
    ForwardToStream {
        target_stream: String,
        transformation: Option<String>,
    },
    
    /// Call webhook
    CallWebhook {
        url: String,
        method: String,
        headers: HashMap<String, String>,
    },
    
    /// Custom action
    Custom {
        action_type: String,
        parameters: HashMap<String, String>,
    },
}

/// DROP STREAM statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropStreamStatement {
    /// Stream name to drop
    pub name: String,
    
    /// If exists flag
    pub if_exists: bool,
    
    /// Cascade deletion
    pub cascade: bool,
}

/// DESCRIBE STREAM statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeStreamStatement {
    /// Stream name to describe
    pub name: String,
    
    /// Include detailed statistics
    pub detailed: bool,
}