//! # Query Validator for HyperQL
//!
//! This module provides comprehensive semantic validation for HyperQL queries beyond
//! type checking. The validator performs logical consistency checks, semantic correctness
//! validation, and structural analysis to catch errors before compilation.
//!
//! ## Purpose
//!
//! HyperQL's multi-paradigm nature requires sophisticated semantic validation that goes
//! beyond type checking to ensure queries are logically sound and executable:
//!
//! - **Structural Validation**: Validates query structure and clause relationships
//! - **Semantic Consistency**: Ensures logical consistency across query components
//! - **Schema Awareness**: Validates column and table references where schema is available
//! - **Expression Validation**: Validates expression semantics beyond type compatibility
//! - **Cross-Reference Validation**: Validates relationships between different query parts
//! - **Multi-Paradigm Support**: Validates across SQL, graph, geometric, vector, and temporal constructs
//!
//! ## Architecture
//!
//! The validator is organized into focused modules that handle different aspects of validation:
//!
//! ### Core Validation Engine
//! - [`QueryValidator`]: Main validation coordinator that orchestrates all validation phases
//! - [`ValidationResult`]: Comprehensive result type that captures errors, warnings, and suggestions
//! - [`ValidationConfig`]: Configuration options for controlling validation behavior
//!
//! ### Specialized Validators
//! - [`semantic`]: Core semantic validation for query structure and logic
//! - [`schema`]: Schema-aware validation for table and column references
//! - [`expression`]: Expression-level semantic validation beyond type checking
//! - [`statement`]: Statement-level validation for different query types
//!
//! ## Validation Process
//!
//! The validation process follows a systematic approach:
//!
//! 1. **Structural Analysis**: Validates basic query structure and required clauses
//! 2. **Semantic Validation**: Checks logical consistency and semantic correctness
//! 3. **Schema Validation**: Validates references against available schema information
//! 4. **Expression Validation**: Validates expression semantics and usage patterns
//! 5. **Cross-Reference Validation**: Validates relationships between query components
//! 6. **Warning Generation**: Identifies suspicious but valid patterns
//!
//! ## Error Categories
//!
//! The validator categorizes validation issues into distinct types:
//!
//! ### Validation Errors (Query Cannot Execute)
//! - Structural violations (e.g., HAVING without GROUP BY)
//! - Invalid aggregation usage (e.g., nested aggregates)
//! - Malformed expressions (e.g., invalid function argument counts)
//! - Invalid column references in context
//! - Semantic inconsistencies (e.g., conflicting conditions)
//!
//! ### Validation Warnings (Query May Execute Unexpectedly)
//! - Ambiguous column references
//! - OFFSET without LIMIT
//! - Unused column references
//! - Potentially expensive operations
//! - Suboptimal query patterns
//!
//! ## Multi-Paradigm Support
//!
//! The validator supports validation across all HyperQL paradigms:
//!
//! ### SQL Validation
//! - Standard SQL clause validation
//! - Aggregate function usage validation
//! - GROUP BY/HAVING relationship validation
//! - ORDER BY column reference validation
//!
//! ### Graph Validation
//! - TRAVERSE clause validation
//! - Relationship pattern validation
//! - Path expression validation
//! - Cycle detection hints
//!
//! ### Geometric Validation
//! - Point expression validation
//! - Distance parameter validation
//! - Spatial operation validation
//! - Hyperbolic space constraints
//!
//! ### Vector Validation
//! - Vector dimension consistency
//! - Similarity metric validation
//! - K-NN parameter validation
//! - Embedding reference validation
//!
//! ### Temporal Validation
//! - Time expression validation
//! - Window function validation
//! - Temporal ordering validation
//! - Stream operation validation
//!
//! ## Integration
//!
//! The validator integrates with the existing HyperQL infrastructure:
//!
//! - **Type Checker**: Builds on existing type validation
//! - **Error System**: Uses existing error types and context system
//! - **Compiler**: Optional validation step in compilation pipeline
//! - **AST**: Validates against complete AST representation
//!
//! ## Usage Examples
//!
//! ### Basic Validation
//! ```rust
//! use hyperql::validator::{QueryValidator, ValidationConfig};
//! use hyperql::parser::parse_statement;
//!
//! let validator = QueryValidator::new();
//! let statement = parse_statement("SELECT * FROM users WHERE age > 25")?;
//!
//! let result = validator.validate(&statement)?;
//! if !result.valid {
//!     for error in result.errors {
//!         eprintln!("Validation error: {}", error.message);
//!     }
//! }
//! ```
//!
//! ### Custom Configuration
//! ```rust
//! let config = ValidationConfig {
//!     strict_mode: true,
//!     allow_ambiguous_columns: false,
//!     require_explicit_aliases: true,
//!     validate_schema: true,
//! };
//!
//! let validator = QueryValidator::with_config(config);
//! let result = validator.validate(&statement)?;
//! ```
//!
//! ### Integration with Compiler
//! ```rust
//! use hyperql::compiler::Compiler;
//!
//! let compiler = Compiler::new();
//! let validator = QueryValidator::new();
//!
//! // Validate before compilation
//! let validation_result = validator.validate(&statement)?;
//! if validation_result.valid {
//!     let compiled = compiler.compile(statement)?;
//! }
//! ```
//!
//! ## Performance Considerations
//!
//! The validator is designed for efficiency:
//! - Single-pass validation where possible
//! - Early termination on critical errors
//! - Minimal memory allocation
//! - Configurable validation depth
//! - Schema caching for repeated queries
//!
//! ## Extension Points
//!
//! The validator can be extended with custom validation rules:
//! - Custom semantic validators
//! - Domain-specific validation logic
//! - Organization-specific constraints
//! - Integration with external schema systems

// Module declarations - focused implementation modules
pub mod semantic;
pub mod schema;
pub mod expression;
pub mod statement;

// Re-export core types for public API
pub use self::semantic::SemanticValidator;
pub use self::schema::SchemaValidator;
pub use self::expression::ExpressionValidator;
pub use self::statement::StatementValidator;

use crate::ast::Statement;
use crate::error::Result;
use crate::type_checker::TypeChecker;
use std::collections::HashMap;

/// Configuration options for query validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable strict validation mode with additional checks
    pub strict_mode: bool,
    /// Allow ambiguous column references (multiple tables with same column)
    pub allow_ambiguous_columns: bool,
    /// Require explicit table aliases for multi-table queries
    pub require_explicit_aliases: bool,
    /// Validate against available schema information
    pub validate_schema: bool,
    /// Maximum validation depth for nested expressions
    pub max_validation_depth: usize,
    /// Enable performance-related warnings
    pub warn_performance_issues: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            allow_ambiguous_columns: true,
            require_explicit_aliases: false,
            validate_schema: false,
            max_validation_depth: 100,
            warn_performance_issues: true,
        }
    }
}

/// Main query validator that coordinates all validation phases
pub struct QueryValidator {
    config: ValidationConfig,
    type_checker: TypeChecker,
    semantic_validator: SemanticValidator,
    schema_validator: SchemaValidator,
    expression_validator: ExpressionValidator,
    statement_validator: StatementValidator,
}

impl QueryValidator {
    /// Create a new validator with default configuration
    pub fn new() -> Self {
        let config = ValidationConfig::default();
        Self::with_config(config)
    }

    /// Create a new validator with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        let type_checker = TypeChecker::new();
        Self {
            semantic_validator: SemanticValidator::new(&config),
            schema_validator: SchemaValidator::new(&config),
            expression_validator: ExpressionValidator::new(&config),
            statement_validator: StatementValidator::new(&config),
            config,
            type_checker,
        }
    }

    /// Validate a complete query statement
    pub fn validate(&mut self, statement: &Statement) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Phase 1: Statement-level validation
        self.statement_validator.validate(statement, &mut result)?;

        // Phase 2: Semantic validation
        self.semantic_validator.validate(statement, &mut result)?;

        // Phase 3: Expression validation (builds on type checking)
        self.expression_validator.validate(statement, &mut self.type_checker, &mut result)?;

        // Phase 4: Schema validation (if enabled and schema available)
        if self.config.validate_schema {
            self.schema_validator.validate(statement, &mut result)?;
        }

        // Determine overall validity
        result.valid = result.errors.is_empty();

        Ok(result)
    }

    /// Validate with custom schema information
    pub fn validate_with_schema(
        &mut self,
        statement: &Statement,
        schema: &HashMap<String, Vec<String>>,
    ) -> Result<ValidationResult> {
        self.schema_validator.set_schema(schema);
        self.validate(statement)
    }

    /// Get the current validation configuration
    pub fn config(&self) -> &ValidationConfig {
        &self.config
    }

    /// Update the validation configuration
    pub fn set_config(&mut self, config: ValidationConfig) {
        self.config = config;
        // Update all sub-validators with new config
        self.semantic_validator.update_config(&self.config);
        self.schema_validator.update_config(&self.config);
        self.expression_validator.update_config(&self.config);
        self.statement_validator.update_config(&self.config);
    }
}

impl Default for QueryValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive validation result containing errors, warnings, and suggestions
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the query is valid (no errors)
    pub valid: bool,
    /// Critical errors that prevent query execution
    pub errors: Vec<ValidationError>,
    /// Warnings about potentially problematic patterns
    pub warnings: Vec<ValidationWarning>,
    /// Performance-related suggestions
    pub suggestions: Vec<String>,
    /// Validation metadata
    pub metadata: ValidationMetadata,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
            metadata: ValidationMetadata::default(),
        }
    }

    /// Add a validation error
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.valid = false;
    }

    /// Add a validation warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Add a suggestion
    pub fn add_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }

    /// Check if there are any issues (errors or warnings)
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation error representing a critical issue that prevents execution
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error type category
    pub kind: ValidationErrorKind,
    /// Human-readable error message
    pub message: String,
    /// Location in query where error occurred
    pub location: Option<String>,
    /// Context information about the error
    pub context: Vec<String>,
    /// Suggested fixes for the error
    pub suggestions: Vec<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(kind: ValidationErrorKind, message: String) -> Self {
        Self {
            kind,
            message,
            location: None,
            context: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add location information
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    /// Add context information
    pub fn with_context(mut self, context: Vec<String>) -> Self {
        self.context = context;
        self
    }

    /// Add suggestions
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// Validation warning representing a potentially problematic pattern
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning type category
    pub kind: ValidationWarningKind,
    /// Human-readable warning message
    pub message: String,
    /// Location in query where warning occurred
    pub location: Option<String>,
    /// Suggestion for addressing the warning
    pub suggestion: Option<String>,
}

impl ValidationWarning {
    /// Create a new validation warning
    pub fn new(kind: ValidationWarningKind, message: String) -> Self {
        Self {
            kind,
            message,
            location: None,
            suggestion: None,
        }
    }

    /// Add location information
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// Categories of validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorKind {
    /// Structural query issues
    StructuralError,
    /// Invalid aggregation usage
    AggregationError,
    /// Invalid expression usage
    ExpressionError,
    /// Column reference issues
    ColumnReferenceError,
    /// Table reference issues
    TableReferenceError,
    /// Function usage issues
    FunctionError,
    /// Graph traversal issues
    GraphTraversalError,
    /// Geometric operation issues
    GeometricError,
    /// Vector operation issues
    VectorError,
    /// Temporal operation issues
    TemporalError,
    /// Schema validation issues
    SchemaError,
    /// Semantic consistency issues
    SemanticError,
}

/// Categories of validation warnings
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarningKind {
    /// Potentially ambiguous references
    AmbiguousReference,
    /// Performance concerns
    PerformanceWarning,
    /// Unusual but valid patterns
    UnusualPattern,
    /// Missing optimizations
    OptimizationOpportunity,
    /// Potentially unsafe operations
    SafetyWarning,
}

/// Metadata about the validation process
#[derive(Debug, Clone, Default)]
pub struct ValidationMetadata {
    /// Tables referenced in the query
    pub tables_referenced: Vec<String>,
    /// Columns referenced in the query
    pub columns_referenced: Vec<String>,
    /// Functions used in the query
    pub functions_used: Vec<String>,
    /// Validation phases completed
    pub phases_completed: Vec<String>,
    /// Validation time in milliseconds
    pub validation_time_ms: f64,
}