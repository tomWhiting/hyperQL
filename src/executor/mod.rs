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
pub mod expression_eval;
mod aggregation;
mod data_source;
mod geometric;
mod vector;
mod join;

use crate::compiler::{CompiledExpression, CompiledAssignment};
use crate::types::Entity;
use crate::error::Result;
use std::collections::HashMap;

// Re-export main types and implementations
pub use plan_executor::PlanExecutor as Executor;
pub use data_source::MemoryDataSource;

/// Abstract data source trait
pub trait DataSource: Send + Sync {
    /// Scan entities from a collection of specific type
    ///
    /// # Parameters
    ///
    /// * `table` - Collection name
    /// * `entity_type` - Entity type to scan within collection
    ///
    /// # Returns
    ///
    /// Vector of entities matching the specified type
    fn scan(&self, table: &str, entity_type: &str) -> Result<Vec<Entity>>;

    /// Scan entities from a table with early termination after limit
    ///
    /// This is a critical optimization for queries with LIMIT clauses. Instead of loading
    /// all entities and their properties, this method stops after loading exactly `limit` entities.
    ///
    /// # Performance Impact
    ///
    /// For queries like `SELECT * FROM collection.type LIMIT 5` on a 10K entity collection:
    /// - Without optimization: Loads 10K entities + 10K property lookups (~60-110ms)
    /// - With optimization: Loads 5 entities + 5 property lookups (~1ms)
    /// - Speedup: 60-110x
    ///
    /// # Parameters
    ///
    /// * `table` - Collection name to scan
    /// * `entity_type` - Entity type to scan within collection
    /// * `limit` - Maximum number of entities to return
    ///
    /// # Returns
    ///
    /// Vector of at most `limit` entities with properties loaded
    ///
    /// # Default Implementation
    ///
    /// The default implementation uses `scan().into_iter().take(limit)` which is simple
    /// but NOT optimized (still loads all entities). Implementations should override this
    /// with database-level early termination for optimal performance.
    fn scan_with_limit(&self, table: &str, entity_type: &str, limit: usize) -> Result<Vec<Entity>> {
        // Default: scan all then take limit (slow but correct)
        // Real implementations (RouterDataSource) override with early termination
        Ok(self.scan(table, entity_type)?.into_iter().take(limit).collect())
    }

    /// Insert entities into a table
    fn insert(&mut self, table: &str, entities: Vec<Entity>) -> Result<u64>;

    /// Update entities in a table
    fn update(&mut self, table: &str, filter: Option<&CompiledExpression>, assignments: &[CompiledAssignment]) -> Result<u64>;

    /// Delete entities from a table
    fn delete(&mut self, table: &str, filter: Option<&CompiledExpression>) -> Result<u64>;

    /// Get schema information for a table
    fn get_schema(&self, table: &str) -> Result<TableSchema>;

    /// Traverse graph from start entity up to max depth
    ///
    /// Returns entities reachable from start entity with their depth (hop count).
    /// Depth 0 = start entity itself, depth 1 = direct neighbors, etc.
    ///
    /// # Parameters
    ///
    /// * `start_entity_id` - ID of entity to start traversal from
    /// * `max_depth` - Maximum number of hops from start (0 = start only)
    /// * `edge_type_filter` - Optional edge type filter (None = all edge types)
    ///
    /// # Returns
    ///
    /// Vector of (Entity, depth) tuples for all reachable entities up to max_depth
    fn traverse_graph(
        &self,
        start_entity_id: &crate::types::EntityId,
        max_depth: usize,
        edge_type_filter: Option<&str>,
    ) -> Result<Vec<(Entity, usize)>>;

    /// Fast count of entities in a table without loading data
    ///
    /// Optimized path for COUNT(*) queries. Default implementation uses scan().len()
    /// which is slow. Implementations should override this with database-level counting.
    ///
    /// # Parameters
    ///
    /// * `table` - Collection name to count entities in
    /// * `entity_type` - Entity type to count within collection
    ///
    /// # Returns
    ///
    /// Number of entities of the specified type in the collection
    fn count_entities_fast(&self, table: &str, entity_type: &str) -> Result<usize> {
        // Default implementation: fall back to scan (slow but correct)
        // Real implementations (RouterDataSource) override this with fast database-level counting
        Ok(self.scan(table, entity_type)?.len())
    }

    /// Create a schema for a collection
    ///
    /// # Parameters
    ///
    /// * `operation` - CREATE SCHEMA operation with field definitions
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    fn create_schema(&mut self, operation: &crate::ast::schema::CreateSchemaStatement) -> Result<()>;

    /// Drop a schema from a collection
    ///
    /// # Parameters
    ///
    /// * `operation` - DROP SCHEMA operation with collection name
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    fn drop_schema(&mut self, operation: &crate::ast::schema::DropSchemaStatement) -> Result<()>;

    /// Alter an existing schema
    ///
    /// # Parameters
    ///
    /// * `operation` - ALTER SCHEMA operation with modifications
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    fn alter_schema(&mut self, operation: &crate::ast::schema::AlterSchemaStatement) -> Result<()>;

    /// Describe a schema (retrieve schema information)
    ///
    /// # Parameters
    ///
    /// * `operation` - DESCRIBE SCHEMA operation with collection name
    ///
    /// # Returns
    ///
    /// Schema information as a formatted string or error
    fn describe_schema(&self, operation: &crate::ast::schema::DescribeSchemaStatement) -> Result<String>;

    /// Execute a Lua function
    ///
    /// Lua runtime is provided by the embedding application (e.g., Hyperspatial).
    /// When HyperQL is embedded in Hyperspatial, this executes via LuaComputeEngine.
    ///
    /// # Parameters
    ///
    /// * `function_name` - Name of the Lua function to execute
    /// * `args` - Function arguments as HyperQL Values
    ///
    /// # Returns
    ///
    /// Result value from function execution, or error if execution fails
    ///
    /// # Default Implementation
    ///
    /// Returns an error indicating Lua runtime is not available.
    /// Implementations (e.g., RouterDataSource) should override this with actual compute integration.
    fn execute_lua_function(
        &mut self,
        function_name: &str,
        _args: Vec<crate::types::Value>,
    ) -> Result<crate::types::Value> {
        Err(crate::error::HyperQLError::ExecutionError {
            message: format!(
                "Lua runtime not available. Function '{}' requires Hyperspatial RouterDataSource.",
                function_name
            ),
            operation: "execute_lua_function".to_string(),
            entity_context: None,
        })
    }

    /// Execute a WASM function
    ///
    /// WASM runtime is provided by the embedding application (e.g., Hyperspatial).
    /// When HyperQL is embedded in Hyperspatial, this executes via WasmComputeEngine.
    ///
    /// # Parameters
    ///
    /// * `module_name` - Name of the WASM module containing the function
    /// * `function_name` - Name of the function to execute within the module
    /// * `args` - Function arguments as HyperQL Values
    ///
    /// # Returns
    ///
    /// Result value from function execution, or error if execution fails
    ///
    /// # Default Implementation
    ///
    /// Returns an error indicating WASM runtime is not available.
    /// Implementations (e.g., RouterDataSource) should override this with actual compute integration.
    fn execute_wasm_function(
        &mut self,
        module_name: &str,
        function_name: &str,
        args: Vec<crate::types::Value>,
    ) -> Result<crate::types::Value> {
        Err(crate::error::HyperQLError::ExecutionError {
            message: format!(
                "WASM runtime not available. Function '{}/{}' requires Hyperspatial RouterDataSource.",
                module_name, function_name
            ),
            operation: "execute_wasm_function".to_string(),
            entity_context: None,
        })
    }
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
