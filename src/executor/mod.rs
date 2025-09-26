//! # HyperQL Execution Plan Executor
//!
//! This module implements the execution plan handling for HyperQL.
//! Note: In the refactored architecture, this executor should primarily return execution
//! plans to Hyperspatial rather than performing actual computations. The current
//! implementation maintains backward compatibility while the transition is completed.
//!
//! ## Module Organization
//!
//! The executor is organized into focused submodules:
//! - [`plan_executor`]: Main execution plan handling
//! - [`expression_eval`]: Expression evaluation engine
//! - [`aggregation`]: Aggregation and GROUP BY logic
//! - [`data_source`]: DataSource trait implementations

mod plan_executor;
mod expression_eval;
mod aggregation;
mod data_source;

use crate::compiler::{CompiledExpression, CompiledAssignment};
use crate::types::Entity;
use crate::error::Result;
use std::collections::HashMap;

// Re-export main types and implementations
pub use plan_executor::PlanExecutor as Executor;
pub use data_source::MemoryDataSource;

/// Abstract data source trait
pub trait DataSource: Send + Sync {
    /// Scan entities from a table
    fn scan(&self, table: &str) -> Result<Vec<Entity>>;

    /// Insert entities into a table
    fn insert(&mut self, table: &str, entities: Vec<Entity>) -> Result<u64>;

    /// Update entities in a table
    fn update(&mut self, table: &str, filter: Option<&CompiledExpression>, assignments: &[CompiledAssignment]) -> Result<u64>;

    /// Delete entities from a table
    fn delete(&mut self, table: &str, filter: Option<&CompiledExpression>) -> Result<u64>;

    /// Get schema information for a table
    fn get_schema(&self, table: &str) -> Result<TableSchema>;
}

/// Table schema definition
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: HashMap<String, ColumnType>,
}

/// Column type information
#[derive(Debug, Clone)]
pub enum ColumnType {
    String,
    Integer,
    Float,
    Boolean,
    EntityId,
    Position,
    Vector,
}

/// Statistics collector for execution metrics
pub struct StatsCollector {
    pub entities_scanned: u64,
    pub relationships_traversed: u64,
    pub hyperbolic_operations: u64,
    pub cascade_propagations: u64,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            entities_scanned: 0,
            relationships_traversed: 0,
            hyperbolic_operations: 0,
            cascade_propagations: 0,
        }
    }

    pub fn reset(&mut self) {
        self.entities_scanned = 0;
        self.relationships_traversed = 0;
        self.hyperbolic_operations = 0;
        self.cascade_propagations = 0;
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}
