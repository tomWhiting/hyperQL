//! # Field Specification AST
//!
//! AST nodes for field definitions in schema DDL statements.

use crate::ast::Expression;
use serde::{Deserialize, Serialize};

use super::cascade::CascadeConfiguration;

/// Field definition in schema DDL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field name
    pub name: String,
    
    /// Field type
    pub field_type: FieldType,
    
    /// Whether field is required (NOT NULL)
    pub required: bool,
    
    /// Property scope for similarity comparison
    pub scope: Option<PropertyScope>,
    
    /// Field kind determining value source
    pub kind: FieldKind,
    
    /// Optional description
    pub description: Option<String>,
}

/// Field type enumeration (17 fundamental types)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// String type
    String,
    
    /// Integer type
    Integer,
    
    /// Float type
    Float,
    
    /// Boolean type
    Boolean,
    
    /// Timestamp type
    Timestamp,
    
    /// Duration type
    Duration,
    
    /// Date type
    Date,
    
    /// JSON type
    Json,
    
    /// Array type with element type
    Array(Box<FieldType>),
    
    /// Map type with value type
    Map(Box<FieldType>),
    
    /// Reference to another entity
    Reference,
    
    /// Edge reference type
    Edge,
    
    /// Embedding vector with dimension
    Embedding(usize),
    
    /// 3D position in hyperbolic space
    Position3D,
    
    /// Raw bytes
    Bytes,
}

/// Property scope for similarity comparison
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyScope {
    /// Excluded from similarity comparison (e.g., created_at, id)
    Metadata,
    
    /// Only compared within same collection (e.g., status with collection-specific values)
    CollectionSpecific,
    
    /// Globally comparable across collections (e.g., sentiment, price)
    DomainShared,
}

/// Field kind determining value source
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldKind {
    /// Regular field - direct user input
    Regular,
    
    /// Cascaded field - aggregated from edges
    Cascaded(CascadeConfiguration),
    
    /// Calculated field - computed from formula
    Calculated(Expression),
    
    /// Computed field - WASM/Lua function
    Computed(String),
}
