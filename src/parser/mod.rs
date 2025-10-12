//! # HyperQL Query Parser
//!
//! This module implements a comprehensive parser for the HyperQL query language.
//! It supports SELECT, INSERT, UPDATE, DELETE statements with expressions,
//! aggregate functions, GROUP BY, HAVING, ORDER BY, and LIMIT clauses.
//!
//! ## Module Organization
//!
//! The parser is organized into focused submodules:
//! - [`statement`]: Top-level statement parsing dispatch
//! - [`select`]: SELECT statement parsing
//! - [`expression`]: Expression and literal parsing
//! - [`clause`]: SQL clause parsing (GROUP BY, ORDER BY, etc.)
//! - [`traverse`]: TRAVERSE clause parsing for graph patterns
//! - [`utils`]: Parser utility functions

mod statement;
mod select;
mod expression;
mod clause;
mod traverse;
mod utils;
mod geometric;


/// Parse a complete HyperQL statement
pub use statement::parse_statement;

/// Parse geometric expressions (NEAR, WITHIN, DISTANCE)
pub use geometric::{parse_near_expression, parse_distance_expression};
