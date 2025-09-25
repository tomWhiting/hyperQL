//! # Error Handling for HyperQL
//!
//! This module provides comprehensive error types for all aspects of HyperQL query
//! processing, from parsing through execution. The error system is designed to
//! provide detailed context for debugging while maintaining performance in
//! production scenarios.
//!
//! ## Purpose
//!
//! HyperQL's multi-paradigm nature requires sophisticated error handling that can:
//! - Distinguish between syntax errors, semantic errors, and runtime failures
//! - Provide meaningful error messages for complex geometric operations
//! - Handle errors from multiple underlying systems (parser, hyperbolic engine, persistence)
//! - Maintain error context across query compilation and execution phases
//! - Support both development-friendly detailed errors and production-safe sanitized versions
//!
//! ## Error Categories
//!
//! The error system categorizes failures into distinct types:
//!
//! ### Parse Errors
//! - Syntax violations in query text
//! - Malformed expressions and operators
//! - Invalid identifier usage
//! - Bracket/parenthesis mismatches
//!
//! ### Semantic Errors
//! - Type mismatches in expressions
//! - Undefined variables and functions
//! - Invalid entity or property references
//! - Cascade operation conflicts
//!
//! ### Execution Errors
//! - Hyperbolic space operation failures
//! - Persistence layer access issues
//! - Resource exhaustion and timeouts
//! - Constraint violations
//!
//! ### System Errors
//! - Database connection failures
//! - Index corruption or unavailability
//! - Memory allocation failures
//! - Internal consistency violations
//!
//! ## Error Context
//!
//! Each error type includes rich context information:
//! - Source location (line/column for parse errors)
//! - Query context (active variables, current operation)
//! - Suggested fixes where applicable
//! - Related error chains for complex failures
//!
//! ## Integration
//!
//! The error system integrates with:
//! - **Parser**: Provides detailed syntax error reporting with source locations
//! - **Compiler**: Semantic validation and type checking error messages
//! - **Executor**: Runtime error handling with query execution context
//! - **Hyperspatial Engine**: Geometric operation error translation and context
//!
//! This unified error handling enables comprehensive debugging support while
//! maintaining clean separation between different failure modes.

use thiserror::Error;

/// Result type alias for HyperQL operations
pub type Result<T> = std::result::Result<T, HyperQLError>;

/// Comprehensive error types for HyperQL query processing
#[derive(Error, Debug)]
pub enum HyperQLError {
    /// Parse errors from query text analysis
    #[error("Parse error: {message} at line {line}, column {column}")]
    ParseError {
        message: String,
        line: usize,
        column: usize,
        source_text: Option<String>,
    },

    /// Semantic validation errors
    #[error("Semantic error: {message}")]
    SemanticError {
        message: String,
        context: Vec<String>,
    },

    /// Validation errors for parameters and constraints
    #[error("Validation error: {message}{}", field.as_ref().map(|f| format!(" in field '{}'", f)).unwrap_or_default())]
    ValidationError {
        message: String,
        field: Option<String>,
    },

    /// Type system violations
    #[error("Type error: expected {expected}, found {found} in {context}")]
    TypeError {
        expected: String,
        found: String,
        context: String,
    },

    /// Undefined variable or function references
    #[error("Undefined reference: {name} of type {ref_type}")]
    UndefinedReference {
        name: String,
        ref_type: String,
        available: Vec<String>,
    },

    /// Query execution failures
    #[error("Execution error: {message}")]
    ExecutionError {
        message: String,
        operation: String,
        entity_context: Option<String>,
    },

    /// Hyperbolic geometry operation errors
    #[error("Geometric error: {operation} failed - {reason}")]
    GeometricError {
        operation: String,
        reason: String,
        positions: Vec<String>,
    },

    /// Cascade system errors
    #[error("Cascade error: {message} in measure {measure_name}")]
    CascadeError {
        message: String,
        measure_name: String,
        propagation_path: Vec<String>,
    },

    /// Database system errors
    #[error("Database error: {message}")]
    DatabaseError {
        message: String,
        operation: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Resource constraint violations
    #[error("Resource error: {resource} {constraint}")]
    ResourceError {
        resource: String,
        constraint: String,
        current_usage: Option<String>,
    },

    /// Internal consistency violations
    #[error("Internal error: {message} - this indicates a bug")]
    InternalError {
        message: String,
        component: String,
        debug_info: String,
    },
}