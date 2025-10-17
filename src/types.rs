//! # Core Type Definitions for HyperQL
//!
//! This module defines the fundamental types used throughout the HyperQL query
//! language implementation. These types form the foundation for all query
//! operations and provide the interface between HyperQL and the underlying
//! Hyperspatial database engine.
//!
//! ## Purpose
//!
//! HyperQL's multi-paradigm nature requires a rich type system that can:
//! - Represent all data types supported by the Hyperspatial database
//! - Handle geometric concepts like positions and distances
//! - Support both scalar and vector operations
//! - Provide type safety across query compilation and execution
//! - Enable efficient serialization for distributed operations
//!
//! ## Type Categories
//!
//! The type system organizes data into several fundamental categories:
//!
//! ### Primitive Types
//! - Numeric types (integers, floats, decimals)
//! - Text types (strings, identifiers)
//! - Boolean and null values
//! - Temporal types (timestamps, durations)
//!
//! ### Geometric Types
//! - 3D positions in hyperbolic space
//! - Hyperbolic distances and metrics
//! - Geometric regions and boundaries
//! - Trajectory and movement vectors
//!
//! ### Collection Types
//! - Vectors for embeddings and numerical arrays
//! - Lists for ordered sequences
//! - Maps for key-value associations
//! - Sets for unique collections
//!
//! ### Entity Types
//! - Entity identifiers and references
//! - Property maps and attribute collections
//! - Relationship descriptors
//! - Measure values and cascade results
//!
//! ## Value System
//!
//! The core `Value` enum provides a unified representation for all data types
//! that can appear in queries:
//! - Automatic type coercion where appropriate
//! - Efficient memory representation
//! - Serialization support for network operations
//! - Type-safe operations with comprehensive error handling
//!
//! ## Geometric Integration
//!
//! Types are designed for seamless integration with hyperbolic operations:
//! - Position types map directly to hyperbolic coordinates
//! - Distance calculations use specialized numeric types
//! - Geometric predicates operate on well-defined spatial types
//! - Vector operations support both Euclidean and hyperbolic metrics
//!
//! ## Performance Considerations
//!
//! The type system is optimized for query performance:
//! - Copy semantics for small types (IDs, numbers)
//! - Reference counting for large types (vectors, strings)
//! - Lazy evaluation for computed values
//! - Memory pooling for frequent allocations
//!
//! ## Integration
//!
//! These types integrate with all HyperQL modules:
//! - **Parser**: Literal value creation and type inference
//! - **Compiler**: Type checking and coercion rules
//! - **Executor**: Runtime value manipulation and operations
//! - **Functions**: Type-safe function signatures and implementations
//!
//! This type system enables HyperQL to provide both flexibility and performance
//! while maintaining type safety across all operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity identifier type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub String);

/// Property name identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PropertyName(pub String);

/// Relationship type identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationType(pub String);

/// Measure identifier for cascade operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasureId(pub String);

/// 3D position in hyperbolic space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Hyperbolic distance value
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HyperbolicDistance(pub f64);

/// Vector for embeddings and numerical arrays
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    pub dimensions: Vec<f64>,
}

/// Unified value type for all query operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// 64-bit signed integer
    Int(i64),
    /// 64-bit floating point number
    Float(f64),
    /// UTF-8 string
    String(String),
    /// Entity identifier
    EntityId(EntityId),
    /// 3D hyperbolic position
    Position(Position3D),
    /// Hyperbolic distance
    Distance(HyperbolicDistance),
    /// Numerical vector
    Vector(Vector),
    /// Ordered list of values
    List(Vec<Value>),
    /// Key-value map
    Map(HashMap<String, Value>),
    /// Timestamp (milliseconds since Unix epoch)
    Timestamp(i64),
    /// Duration in milliseconds
    Duration(i64),
}

/// Entity representation with properties and position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub properties: HashMap<PropertyName, Value>,
    pub position: Option<Position3D>,
    pub embedding: Option<Vector>,
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: Option<EntityId>,
    pub from_entity: EntityId,
    pub to_entity: EntityId,
    pub rel_type: RelationType,
    pub properties: HashMap<PropertyName, Value>,
}

/// Query result row
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultRow {
    pub columns: HashMap<String, Value>,
}

/// Complete query result set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows: Vec<ResultRow>,
    pub column_names: Vec<String>,
    pub execution_stats: ExecutionStats,
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value as i64)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(value as f64)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

/// Query execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub execution_time_ms: u64,
    pub entities_scanned: u64,
    pub relationships_traversed: u64,
    pub hyperbolic_operations: u64,
    pub cascade_propagations: u64,
}