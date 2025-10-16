//! # Abstract Syntax Tree for HyperQL
//!
//! This module defines the complete abstract syntax tree (AST) representation for
//! HyperQL queries. The AST captures the full semantic structure of queries after
//! parsing, providing a clean interface between parsing and compilation phases.
//!
//! ## Purpose
//!
//! The AST serves as the central intermediate representation for HyperQL queries:
//! - Provides a structured representation of all query constructs
//! - Enables semantic analysis and type checking
//! - Supports query transformation and optimization
//! - Facilitates code generation for different execution engines
//! - Enables query introspection and analysis tools
//!
//! ## Design Principles
//!
//! The AST design follows several key principles:
//!
//! ### Completeness
//! Every syntactic construct in HyperQL has a corresponding AST node, ensuring
//! that no information is lost during parsing and that the full query semantics
//! are preserved.
//!
//! ### Composability
//! AST nodes are designed to compose naturally, allowing complex queries to be
//! built from simpler components through a clean hierarchical structure.
//!
//! ### Type Safety
//! The AST uses Rust's type system to prevent malformed query structures and
//! ensure that only valid query combinations can be represented.
//!
//! ### Performance
//! AST nodes use efficient representations and avoid unnecessary allocations,
//! with reference counting for shared subexpressions.
//!
//! ## Query Structure
//!
//! HyperQL queries are represented by a hierarchical AST structure:
//!
//! ### Statement Types
//! HyperQL supports multiple statement types:
//! - **SELECT**: Data retrieval with projections, filtering, grouping, and ordering
//! - **INSERT**: Data insertion with column specifications and value lists
//! - **UPDATE**: Data modification with assignments and optional WHERE clauses
//! - **DELETE**: Data removal with optional WHERE clauses
//!
//! ### SELECT Query Structure
//! SELECT statements support comprehensive SQL functionality:
//! - SELECT clause with expressions, aggregate functions, and aliases
//! - FROM clause with table sources and optional aliases
//! - WHERE clause with complex filter predicates and boolean logic
//! - GROUP BY clause for data aggregation by expressions
//! - HAVING clause for filtering grouped results
//! - ORDER BY clause for result sorting (ASC/DESC)
//! - LIMIT and OFFSET clauses for pagination
//! - DISTINCT flag for duplicate elimination
//!
//! ### Expression Trees
//! All expressions are represented as typed expression trees supporting:
//! - **Arithmetic Operations**: Addition, subtraction, multiplication, division, modulo
//! - **Logical Operations**: AND, OR, NOT with proper precedence handling
//! - **Comparison Operations**: Equality, inequality, relational comparisons
//! - **Function Calls**: Built-in functions (string, math, date) and aggregate functions
//! - **Aggregate Functions**: COUNT, SUM, AVG, MIN, MAX with proper grouping semantics
//! - **Column References**: Table-qualified and unqualified column access
//! - **Literal Values**: Numbers, strings, booleans, NULL, entity IDs
//!
//! ### Type System Integration
//! The AST includes rich type information that enables:
//! - Compile-time type checking
//! - Automatic type coercion where appropriate
//! - Type-aware optimization decisions
//! - Runtime type safety guarantees
//!
//! ## Module Organization
//!
//! The AST module is organized into focused submodules:
//!
//! - [`query`]: Core query structure and top-level constructs
//! - [`expression`]: Expression trees and operators
//! - [`literal`]: Literal values and constants
//! - [`identifier`]: Entity and property identifiers
//! - [`function`]: Function calls and built-in operations
//! - [`predicate`]: Boolean expressions and filters
//! - [`traverse`]: Graph traversal and navigation
//! - [`cascade`]: Measure propagation and cascade operations
//! - [`aggregate`]: Aggregation functions and grouping
//! - [`temporal`]: Time-based operations and trajectories
//!
//! ## Geometric Extensions
//!
//! The AST includes specialized nodes for hyperbolic operations:
//! - Distance calculations and comparisons
//! - Spatial region queries and containment tests
//! - Position-based filtering and sorting
//! - Trajectory analysis and temporal patterns
//!
//! ## Validation and Analysis
//!
//! The AST supports comprehensive validation:
//! - Semantic consistency checking
//! - Type compatibility verification
//! - Reference resolution and scope analysis
//! - Performance impact estimation
//!
//! ## Integration
//!
//! The AST integrates with other HyperQL components:
//! - **Parser**: Creates AST nodes from query text
//! - **Compiler**: Transforms AST into execution plans
//! - **Optimizer**: Analyzes and transforms AST nodes
//! - **Type System**: Provides type information for all nodes
//!
//! This comprehensive AST enables HyperQL to support complex multi-paradigm
//! queries while maintaining clean separation between parsing, analysis, and
//! execution phases.

pub mod geometric;
pub mod graph;
pub mod vector;
pub mod timeseries;
pub mod schema;
pub mod streams;

// Core query structures
pub use crate::types::*;
use serde::{Deserialize, Serialize};

/// Root statement types in HyperQL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// SELECT statement for data retrieval
    Select(SelectStatement),
    /// INSERT statement for data insertion
    Insert(InsertStatement),
    /// UPDATE statement for data modification
    Update(UpdateStatement),
    /// DELETE statement for data removal
    Delete(DeleteStatement),
    /// Schema DDL statements
    Schema(schema::SchemaOperation),
    /// Stream DDL statements
    Stream(streams::StreamOperation),
}

/// SELECT statement structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStatement {
    /// Columns/expressions to select
    pub select_list: Vec<SelectItem>,
    /// FROM clause with table/entity sources
    pub from: Option<FromClause>,
    /// TRAVERSE clause for graph patterns
    pub traverse_clause: Option<TraverseClause>,
    /// WHERE clause for filtering
    pub where_clause: Option<Expression>,
    /// GROUP BY expressions
    pub group_by: Vec<Expression>,
    /// HAVING clause for grouped filtering
    pub having: Option<Expression>,
    /// ORDER BY expressions
    pub order_by: Vec<OrderByItem>,
    /// LIMIT for result count restriction
    pub limit: Option<u64>,
    /// OFFSET for pagination
    pub offset: Option<u64>,
    /// DISTINCT flag
    pub distinct: bool,
}

/// Items in the SELECT list
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectItem {
    /// Wildcard (*) to select all columns
    Wildcard,
    /// Expression with optional alias
    Expression {
        expr: Expression,
        alias: Option<String>,
    },
}

/// FROM clause sources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FromClause {
    /// Simple table/entity reference
    Table {
        name: String,
        alias: Option<String>,
    },
    /// Subquery as source
    Subquery {
        query: Box<SelectStatement>,
        alias: String,
    },
}

/// ORDER BY items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expr: Expression,
    pub direction: OrderDirection,
}

/// Sort direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// Expression types in HyperQL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Literal values
    Literal(Literal),
    /// Column references
    Column(ColumnRef),
    /// Binary operations
    Binary {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    /// Unary operations
    Unary {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    /// BETWEEN operation (value BETWEEN min AND max)
    Between {
        expr: Box<Expression>,
        lower: Box<Expression>,
        upper: Box<Expression>,
        negated: bool,
    },
    /// Function calls
    Function {
        name: String,
        args: Vec<Expression>,
    },
    /// Geometric operations in hyperbolic space
    Geometric(geometric::GeometricExpression),
    /// Vector operations with named embeddings
    Vector(VectorExpression),
}

/// Literal value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    EntityId(EntityId),
}

/// Column reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnRef {
    pub table: Option<String>,
    pub name: String,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,

    // Logical
    And,
    Or,

    // String
    Like,
    NotLike,

    // Membership
    In,
    NotIn,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Minus,
    Plus,
    IsNull,
    IsNotNull,
}

/// Vector expression types for named embeddings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VectorExpression {
    /// Similarity search with named vector
    Similarity {
        vector_name: String,
        reference: Box<Expression>,
        metric: vector::similarity::SimilarityMetric,
        threshold: Option<f64>,
        vector_type: vector::similarity::VectorType,
    },
    /// k-nearest neighbors with named vector
    KNN {
        vector_name: String,
        reference: Box<Expression>,
        k: u32,
        metric: vector::similarity::SimilarityMetric,
        vector_type: vector::similarity::VectorType,
    },
}

/// INSERT statement structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStatement {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Expression>>,
}

/// UPDATE statement structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStatement {
    pub table: String,
    pub assignments: Vec<Assignment>,
    pub where_clause: Option<Expression>,
}

/// Assignment for UPDATE statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub column: String,
    pub value: Expression,
}

/// DELETE statement structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStatement {
    pub table: String,
    pub where_clause: Option<Expression>,
}

/// TRAVERSE clause for graph pattern matching
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraverseClause {
    /// List of graph patterns to match
    pub patterns: Vec<TraversePattern>,
}

/// Individual graph pattern in Cypher-style syntax
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversePattern {
    /// Start node specification
    pub start_node: NodePattern,
    /// Relationship specification
    pub relationship: RelationshipPattern,
    /// End node specification
    pub end_node: NodePattern,
}

/// Node pattern in graph traversal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodePattern {
    /// Variable name to bind the node to (optional)
    pub variable: Option<String>,
    /// Node label/type constraint (optional)
    pub label: Option<String>,
    /// Property constraints on the node
    pub properties: Option<Expression>,
}

/// Relationship pattern in graph traversal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipPattern {
    /// Variable name to bind the relationship to (optional)
    pub variable: Option<String>,
    /// Relationship type constraint (optional)
    pub rel_type: Option<String>,
    /// Direction of relationship
    pub direction: RelationshipDirection,
    /// Variable length specification (optional)
    pub variable_length: Option<VariableLength>,
    /// Optional relationship flag
    pub optional: bool,
    /// Property constraints on the relationship
    pub properties: Option<Expression>,
}

/// Direction of relationship in pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationshipDirection {
    /// Outgoing relationship (->)
    Outgoing,
    /// Incoming relationship (<-)
    Incoming,
    /// Undirected relationship (--)
    Undirected,
}

/// Variable length specification for relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableLength {
    /// Minimum number of hops (optional, defaults to 1)
    pub min_hops: Option<u32>,
    /// Maximum number of hops (optional, unlimited if None)
    pub max_hops: Option<u32>,
}