//! # Schema Operation AST Nodes
//!
//! AST nodes for schema DDL operations including CREATE SCHEMA, ALTER SCHEMA,
//! and DROP SCHEMA statements.

use serde::{Deserialize, Serialize};

use super::field_spec::FieldDefinition;

/// Schema operation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchemaOperation {
    /// CREATE SCHEMA statement
    Create(CreateSchemaStatement),
    
    /// ALTER SCHEMA statement
    Alter(AlterSchemaStatement),
    
    /// DROP SCHEMA statement
    Drop(DropSchemaStatement),
    
    /// DESCRIBE SCHEMA statement
    Describe(DescribeSchemaStatement),
}

/// CREATE SCHEMA statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSchemaStatement {
    /// Collection name for this schema
    pub collection_name: String,
    
    /// Field definitions
    pub fields: Vec<FieldDefinition>,
    
    /// Extensibility mode
    pub extensibility: ExtensibilityMode,
    
    /// Optional schema description
    pub description: Option<String>,
    
    /// If NOT EXISTS flag
    pub if_not_exists: bool,
}

/// ALTER SCHEMA statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterSchemaStatement {
    /// Collection name
    pub collection_name: String,
    
    /// Alteration operation
    pub operation: AlterOperation,
}

/// Alteration operations for ALTER SCHEMA
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterOperation {
    /// Add a new field
    AddField(FieldDefinition),
    
    /// Drop an existing field
    DropField(String),
    
    /// Modify an existing field
    ModifyField {
        /// Field name
        name: String,
        /// New field definition
        definition: FieldDefinition,
    },
    
    /// Change extensibility mode
    SetExtensibility(ExtensibilityMode),
}

/// DROP SCHEMA statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropSchemaStatement {
    /// Collection name
    pub collection_name: String,
    
    /// IF EXISTS flag
    pub if_exists: bool,
}

/// DESCRIBE SCHEMA statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeSchemaStatement {
    /// Collection name
    pub collection_name: String,
    
    /// Include detailed information flag
    pub detailed: bool,
}

/// Extensibility mode for schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExtensibilityMode {
    /// Closed - only defined fields allowed
    Closed,
    
    /// Open - any fields allowed beyond defined ones
    Open,
    
    /// Typed - additional fields allowed with explicit type hints
    Typed,
}
