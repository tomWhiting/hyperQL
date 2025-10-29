//! # Function Registry - UDF Management and Versioning
//!
//! The function registry provides metadata types and interfaces for user-defined functions.
//!
//! ## Registry Provided by Embedding Application
//!
//! HyperQL is designed as an embeddable query language. Function registry functionality
//! is provided by the embedding application (e.g., Hyperspatial), not by HyperQL itself.
//!
//! This module defines the data structures and types that the embedding application uses
//! to manage UDFs. The actual storage, versioning, and lookup is handled externally.

use super::RuntimeResult;
use std::collections::HashMap;
use std::time::SystemTime;

/// Central registry for managing user-defined functions
///
/// This is a metadata structure. Actual function registration and lookup
/// is handled by the embedding application.
#[allow(dead_code)]
pub struct FunctionRegistry {
    /// Function storage by name and version
    functions: HashMap<String, Vec<FunctionVersion>>,
    /// Default version mapping
    default_versions: HashMap<String, u32>,
    /// Function metadata cache
    metadata_cache: HashMap<FunctionKey, FunctionMetadata>,
}

/// Unique key for identifying function versions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionKey {
    pub name: String,
    pub version: u32,
}

/// Versioned function entry
#[derive(Debug, Clone)]
pub struct FunctionVersion {
    /// Version number (monotonically increasing)
    pub version: u32,
    /// Function implementation
    pub implementation: FunctionImplementation,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Function signature and metadata
    pub metadata: FunctionMetadata,
    /// Whether this version is marked as deprecated
    pub deprecated: bool,
}

/// Function implementation variants
#[derive(Debug, Clone)]
pub enum FunctionImplementation {
    /// Lua function code
    Lua(String),
    /// WebAssembly bytecode
    Wasm(Vec<u8>),
    /// Built-in native function
    Native(String),
}

/// Function metadata and signature information
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Function name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Parameter definitions
    pub parameters: Vec<ParameterDefinition>,
    /// Return type information
    pub return_type: TypeDefinition,
    /// Function category (similarity, geometric, business_logic, etc.)
    pub category: FunctionCategory,
    /// Performance characteristics
    pub performance_hints: PerformanceHints,
    /// Security classification
    pub security_level: SecurityLevel,
    /// Author information
    pub author: String,
    /// Function tags for discovery
    pub tags: Vec<String>,
}

/// Parameter definition with type and constraints
#[derive(Debug, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub parameter_type: TypeDefinition,
    pub optional: bool,
    pub default_value: Option<String>,
    pub description: String,
}

/// Type system for function parameters and returns
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefinition {
    /// Primitive types
    Boolean,
    Integer,
    Float,
    String,
    /// Collection types
    Array(Box<TypeDefinition>),
    Object(HashMap<String, TypeDefinition>),
    /// Hyperbolic-specific types
    Position,
    Vector(Option<usize>),
    Distance,
    /// Generic types
    Any,
    /// Union types
    Union(Vec<TypeDefinition>),
}

/// Function categorization for organization
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionCategory {
    Similarity,
    Geometric,
    Mathematical,
    StringProcessing,
    BusinessLogic,
    DataTransformation,
    Aggregation,
    Utility,
}

/// Performance characteristics and hints
#[derive(Debug, Clone)]
pub struct PerformanceHints {
    /// Expected computational complexity
    pub complexity: ComputationalComplexity,
    /// Whether function is deterministic
    pub deterministic: bool,
    /// Whether function is safe for parallelization
    pub parallel_safe: bool,
    /// Estimated memory usage
    pub memory_usage: MemoryUsage,
    /// Whether function benefits from vectorization
    pub vectorizable: bool,
}

/// Computational complexity classification
#[derive(Debug, Clone, PartialEq)]
pub enum ComputationalComplexity {
    Constant,
    Logarithmic,
    Linear,
    Linearithmic,
    Quadratic,
    Exponential,
    Unknown,
}

/// Memory usage classification
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryUsage {
    Minimal,
    Low,
    Moderate,
    High,
    VeryHigh,
}

/// Security level for function execution
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    /// Completely safe, no external dependencies
    Safe,
    /// Generally safe but may consume significant resources
    Trusted,
    /// Requires careful review, limited privileges
    Restricted,
    /// Requires administrator approval
    Administrative,
}

impl FunctionRegistry {
    /// Create a new function registry
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            default_versions: HashMap::new(),
            metadata_cache: HashMap::new(),
        }
    }

    /// Register a new function or version
    ///
    /// Function registration is handled by the embedding application.
    /// This returns an error indicating external registration is required.
    pub fn register_function(
        &mut self,
        _implementation: FunctionImplementation,
        metadata: FunctionMetadata,
    ) -> RuntimeResult<u32> {
        use super::RuntimeError;

        Err(RuntimeError::FunctionNotFound(format!(
            "Function registration is handled by embedding application. \
             Function '{}' should be registered via DataSource API.",
            metadata.name
        )))
    }

    /// Get function by name (uses default version)
    ///
    /// Function lookup is handled by the embedding application.
    pub fn get_function(&self, name: &str) -> RuntimeResult<&FunctionVersion> {
        use super::RuntimeError;

        Err(RuntimeError::FunctionNotFound(format!(
            "Function lookup is handled by embedding application. \
             Function '{}' should be accessed via DataSource API.",
            name
        )))
    }

    /// Get specific function version
    pub fn get_function_version(&self, name: &str, version: u32) -> RuntimeResult<&FunctionVersion> {
        use super::RuntimeError;

        Err(RuntimeError::FunctionNotFound(format!(
            "Function lookup is handled by embedding application. \
             Function '{}' version {} should be accessed via DataSource API.",
            name, version
        )))
    }

    /// List all available functions
    pub fn list_functions(&self) -> Vec<FunctionSummary> {
        Vec::new()
    }

    /// Search functions by category or tags
    pub fn search_functions(
        &self,
        _category: Option<FunctionCategory>,
        _tags: Option<&[String]>,
    ) -> Vec<FunctionSummary> {
        Vec::new()
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary information for function listing
#[derive(Debug, Clone)]
pub struct FunctionSummary {
    pub name: String,
    pub current_version: u32,
    pub available_versions: Vec<u32>,
    pub category: FunctionCategory,
    pub description: String,
    pub tags: Vec<String>,
    pub author: String,
}

/// Function usage and performance statistics
#[derive(Debug, Clone)]
pub struct FunctionStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub average_execution_time_ms: f64,
    pub total_execution_time_ms: u64,
    pub memory_usage_stats: MemoryStats,
    pub last_used: Option<SystemTime>,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub peak_memory_bytes: usize,
    pub average_memory_bytes: usize,
    pub total_allocations: u64,
}
