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
use std::fmt;

/// Result type alias for HyperQL operations
pub type Result<T> = std::result::Result<T, HyperQLError>;

/// Comprehensive error types for HyperQL query processing
#[derive(Error, Debug)]
pub enum HyperQLError {
    /// Parse errors from query text analysis with rich context
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        message: String,
        line: usize,
        column: usize,
        source_text: Option<String>,
        /// Error code for documentation lookup
        error_code: Option<String>,
        /// Suggestions for fixing the error
        suggestions: Vec<String>,
        /// Valid examples demonstrating correct syntax
        examples: Vec<String>,
        /// Position in the source text (byte offset)
        position: Option<usize>,
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

    /// Undefined variable or function references with suggestions
    #[error("Undefined {ref_type}: '{name}'")]
    UndefinedReference {
        name: String,
        ref_type: String,
        available: Vec<String>,
        /// Suggested similar names based on edit distance
        suggestions: Vec<String>,
        /// Context where the reference was used
        context: Option<String>,
        /// Error code for documentation
        error_code: Option<String>,
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

    /// Query builder errors
    #[error("Builder error: {}", errors.join("; "))]
    BuilderError {
        errors: Vec<String>,
    },
}

/// Error context for rich error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The original query text
    pub query_text: String,
    /// Position in the query where error occurred (byte offset)
    pub position: Option<usize>,
    /// Line number (1-based)
    pub line: Option<usize>,
    /// Column number (1-based)
    pub column: Option<usize>,
    /// The specific part of the query that caused the error
    pub problematic_text: Option<String>,
    /// Suggested fixes for the error
    pub suggestions: Vec<String>,
    /// Similar valid examples
    pub examples: Vec<String>,
    /// Error code for programmatic handling
    pub error_code: Option<String>,
}

impl ErrorContext {
    /// Create a new error context from a query and position
    pub fn new(query_text: String, position: Option<usize>) -> Self {
        let (line, column) = if let Some(pos) = position {
            Self::calculate_line_column(&query_text, pos)
        } else {
            (None, None)
        };

        Self {
            query_text,
            position,
            line,
            column,
            problematic_text: None,
            suggestions: Vec::new(),
            examples: Vec::new(),
            error_code: None,
        }
    }

    /// Calculate line and column from byte position
    fn calculate_line_column(text: &str, position: usize) -> (Option<usize>, Option<usize>) {
        let mut line = 1;
        let mut column = 1;
        let mut current_pos = 0;

        for ch in text.chars() {
            if current_pos >= position {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            current_pos += ch.len_utf8();
        }

        (Some(line), Some(column))
    }

    /// Extract the problematic part of the query around the error position
    pub fn with_problematic_text(mut self) -> Self {
        if let Some(pos) = self.position {
            // Extract context around the error (30 chars before and after)
            let start = pos.saturating_sub(30);
            let end = (pos + 30).min(self.query_text.len());

            if let Some(text) = self.query_text.get(start..end) {
                self.problematic_text = Some(text.to_string());
            }
        }
        self
    }

    /// Add a suggestion for fixing the error
    pub fn add_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Add an example of valid syntax
    pub fn add_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Set the error code
    pub fn with_error_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self
    }

    /// Get a specific line from the query text
    pub fn get_line_at(&self, line_num: usize) -> Option<String> {
        self.query_text
            .lines()
            .nth(line_num - 1)
            .map(|s| s.to_string())
    }

    /// Estimate the length of the token at the error position
    pub fn get_token_length_at_position(&self) -> Option<usize> {
        if let Some(pos) = self.position {
            // Find the end of the current token
            let remaining = &self.query_text[pos..];
            let token_end = remaining
                .chars()
                .position(|c| c.is_whitespace() || "()[]{},.;:".contains(c))
                .unwrap_or(remaining.chars().count());

            // Find the start of the current token
            let prefix = &self.query_text[..pos];
            let token_start_offset = prefix
                .chars()
                .rev()
                .position(|c| c.is_whitespace() || "()[]{},.;:".contains(c))
                .unwrap_or(prefix.chars().count());

            Some(token_start_offset + token_end)
        } else {
            None
        }
    }
}

/// Trait for adding rich context to errors
pub trait WithContext<T> {
    /// Add context information to this error
    fn with_context(self, query: &str, position: Option<usize>) -> Result<T>;
}

/// Implement WithContext for Results
impl<T, E> WithContext<T> for std::result::Result<T, E>
where
    E: fmt::Debug,
{
    fn with_context(self, query: &str, position: Option<usize>) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(err) => {
                let error_msg = format!("{:?}", err);

                // Create error context with suggestions based on error content
                let mut context = ErrorContext::new(query.to_string(), position)
                    .with_problematic_text()
                    .with_error_code("E001");

                // Add suggestions based on error pattern analysis
                if error_msg.to_lowercase().contains("expected") {
                    context = context.add_suggestion("Check syntax documentation for correct format");
                }

                Err(HyperQLError::ParseError {
                    message: error_msg,
                    line: context.line.unwrap_or(1),
                    column: context.column.unwrap_or(1),
                    source_text: Some(query.to_string()),
                    error_code: context.error_code.clone(),
                    suggestions: context.suggestions.clone(),
                    examples: context.examples.clone(),
                    position: context.position,
                })
            }
        }
    }
}

impl HyperQLError {
    /// Create a new parse error with rich context
    pub fn parse_error_with_context(
        message: impl Into<String>,
        query: &str,
        position: Option<usize>,
    ) -> Self {
        let context = ErrorContext::new(query.to_string(), position);

        HyperQLError::ParseError {
            message: message.into(),
            line: context.line.unwrap_or(1),
            column: context.column.unwrap_or(1),
            source_text: Some(query.to_string()),
            error_code: None,
            suggestions: Vec::new(),
            examples: Vec::new(),
            position: context.position,
        }
    }

    /// Create an undefined reference error with suggestions
    pub fn undefined_reference_with_suggestions(
        name: impl Into<String>,
        ref_type: impl Into<String>,
        available: Vec<String>,
        context: Option<String>,
    ) -> Self {
        let name = name.into();
        let suggestions = crate::error_context::SuggestionGenerator::find_similar_names(
            &name,
            &available,
        );

        HyperQLError::UndefinedReference {
            name,
            ref_type: ref_type.into(),
            available,
            suggestions,
            context,
            error_code: Some("E0102".to_string()),
        }
    }

    /// Create a simple parse error (for migration from old error format)
    pub fn simple_parse_error(
        message: impl Into<String>,
        query: &str,
        line: usize,
        column: usize,
    ) -> Self {
        HyperQLError::ParseError {
            message: message.into(),
            line,
            column,
            source_text: Some(query.to_string()),
            error_code: None,
            suggestions: Vec::new(),
            examples: Vec::new(),
            position: None,
        }
    }
}