//! # Lua Runtime Implementation
//!
//! This module provides the Lua runtime interface for HyperQL user-defined functions.
//!
//! ## Runtime Provided by Embedding Application
//!
//! HyperQL is designed as an embeddable query language. Lua runtime functionality is provided
//! by the embedding application (e.g., Hyperspatial) through the DataSource trait, not
//! by HyperQL itself.
//!
//! When HyperQL is embedded in Hyperspatial:
//! - Lua functions are executed via RouterDataSource.execute_lua_function()
//! - The Hyperspatial LuaComputeEngine (515 lines) provides production-ready Lua execution
//! - Full database API access, hyperbolic operations, and sandboxing are available
//!
//! This design keeps HyperQL lightweight and delegates compute to the embedding application.

use super::{RuntimeEngine, RuntimeResult, RuntimeValue, RuntimeStats, RuntimeConfig};
use std::collections::HashMap;

/// Lua runtime engine implementation
///
/// This is a stub implementation. Actual Lua runtime functionality is provided by the
/// embedding application (e.g., Hyperspatial) via the DataSource trait.
pub struct LuaRuntime {
    #[allow(dead_code)]
    config: RuntimeConfig,
    stats: RuntimeStats,
    functions: HashMap<String, String>,
}

impl LuaRuntime {
    /// Create a new Lua runtime with the given configuration
    ///
    /// Returns an error indicating that Lua runtime is provided externally.
    pub fn new(config: RuntimeConfig) -> RuntimeResult<Self> {
        Ok(Self {
            config,
            stats: RuntimeStats {
                memory_used: 0,
                functions_executed: 0,
                total_execution_time_ms: 0,
                gc_collections: 0,
            },
            functions: HashMap::new(),
        })
    }
}

impl RuntimeEngine for LuaRuntime {
    fn execute(&mut self, function_name: &str, _args: &[RuntimeValue]) -> RuntimeResult<RuntimeValue> {
        use super::RuntimeError;

        Err(RuntimeError::LuaError(format!(
            "Lua runtime is provided by embedding application. \
             Function '{}' should be executed via DataSource.execute_lua_function(). \
             When using HyperQL in Hyperspatial, compute functions are available through RouterDataSource.",
            function_name
        )))
    }

    fn load_function(&mut self, name: &str, code: &str) -> RuntimeResult<()> {
        // Store function code for later execution by embedding application
        self.functions.insert(name.to_string(), code.to_string());
        Ok(())
    }

    fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    fn get_stats(&self) -> RuntimeStats {
        self.stats.clone()
    }
}

/// Lua-specific configuration options
#[derive(Debug, Clone)]
pub struct LuaConfig {
    /// Whether to enable LuaJIT compilation
    pub enable_jit: bool,
    /// GC pause multiplier (default: 200)
    pub gc_pause: i32,
    /// GC step multiplier (default: 200)
    pub gc_stepmul: i32,
    /// Maximum number of Lua instructions per execution
    pub max_instructions: u64,
}

impl Default for LuaConfig {
    fn default() -> Self {
        Self {
            enable_jit: true,
            gc_pause: 200,
            gc_stepmul: 200,
            max_instructions: 1_000_000,
        }
    }
}
