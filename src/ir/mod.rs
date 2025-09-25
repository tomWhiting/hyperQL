//! # Intermediate Representation (IR) - Compiled Query Plans
//!
//! The IR module provides a low-level, optimized intermediate representation for HyperQL queries
//! after compilation from the abstract syntax tree. This IR is designed for efficient execution
//! in hyperbolic space while maintaining the multi-paradigm capabilities of HyperQL.
//!
//! ## Purpose
//!
//! Traditional query engines compile high-level query languages directly to execution plans,
//! often losing optimization opportunities and making it difficult to apply cross-paradigm
//! optimizations. The HyperQL IR solves this by providing:
//!
//! - **Unified Representation**: Single IR that captures relational, graph, vector, and geometric operations
//! - **Optimization Target**: Common format for applying hyperbolic-aware optimizations
//! - **Execution Efficiency**: Low-overhead representation optimized for runtime performance
//! - **Serialization**: Persistent storage and transmission of compiled queries
//! - **Analysis Framework**: Foundation for query analysis and performance prediction
//!
//! ## Mathematical Foundations
//!
//! The IR operates within the mathematical framework of hyperbolic geometry:
//!
//! ### Hyperbolic Query Algebra
//! Operations in the IR respect hyperbolic distance and geometric properties:
//! - **Selection**: σ_P(R) where P includes hyperbolic distance predicates
//! - **Join**: R ⋈_θ S where θ may include geometric proximity conditions
//! - **Projection**: π_A(R) with position-aware attribute selection
//! - **Traversal**: Navigate(R, E, P) for graph operations in hyperbolic space
//!
//! ### Geometric Optimization Properties
//! The IR enables optimizations based on hyperbolic geometry:
//! - **Spatial Locality**: Operations clustered by hyperbolic distance
//! - **Hierarchical Structure**: Queries leverage learned hierarchies
//! - **Vector Proximity**: Similarity operations use geometric positioning
//! - **Cascade Efficiency**: Measure propagation follows geometric paths
//!
//! ### Complexity Analysis
//! IR operations have complexity bounds in hyperbolic space:
//! - **Point Queries**: O(log N) using hyperbolic indexing
//! - **Range Queries**: O(log N + K) where K is result size
//! - **Traversal Queries**: O(E·log V) for E edges and V vertices
//! - **Similarity Queries**: O(N·D) reduced to O(log N + K) with geometric indexing
//!
//! ## IR Design Principles
//!
//! ### Unified Operation Set
//! The IR provides a comprehensive set of operations that handle all HyperQL paradigms:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    HyperQL IR Operations                            │
//! ├─────────────────┬─────────────────┬─────────────────┬─────────────────┤
//! │   Relational    │      Graph      │     Vector      │   Geometric     │
//! │                 │                 │                 │                 │
//! │ • Scan          │ • Traverse      │ • Similarity    │ • Distance      │
//! │ • Filter        │ • PathFind      │ • KNN           │ • Within        │
//! │ • Project       │ • Connected     │ • Clustering    │ • Transform     │
//! │ • Join          │ • Community     │ • Embedding     │ • Intersect     │
//! │ • Aggregate     │ • Centrality    │ • Dimension     │ • Union         │
//! │ • Sort          │ • Flow          │ • Reduction     │ • Buffer        │
//! │ • Group         │ • Matching      │ • Comparison    │ • Convex Hull   │
//! │ • Union         │ • Subgraph      │ • Search        │ • Voronoi       │
//! └─────────────────┴─────────────────┴─────────────────┴─────────────────┘
//! ```
//!
//! ### Execution Model
//! The IR follows a data-flow execution model optimized for hyperbolic operations:
//!
//! 1. **Streaming Processing**: Results flow through operators without full materialization
//! 2. **Lazy Evaluation**: Operations execute only when results are needed
//! 3. **Parallel Execution**: Independent operations execute concurrently
//! 4. **Memory Management**: Automatic cleanup of intermediate results
//! 5. **Error Propagation**: Structured error handling through the execution pipeline
//!
//! ### Type System
//! The IR includes a rich type system that captures hyperbolic-specific data:
//!
//! - **Primitive Types**: Integers, floats, strings, booleans
//! - **Collection Types**: Arrays, sets, maps with geometric ordering
//! - **Hyperbolic Types**: Positions, distances, vectors, embeddings
//! - **Graph Types**: Vertices, edges, paths, trees
//! - **Measure Types**: Values with source attribution and decay factors
//!
//! ## Architecture Overview
//!
//! The IR system is structured around three main components:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                          IR Architecture                            │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                    │
//!                ┌───────────────────┼───────────────────┐
//!                │                   │                   │
//!         ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
//!         │  Operators  │     │    Plans     │     │Serialization│
//!         │             │     │             │     │             │
//!         │• Scan       │     │• Physical   │     │• Binary     │
//!         │• Filter     │     │• Logical    │     │• JSON       │
//!         │• Join       │     │• Optimizer  │     │• Schema     │
//!         │• Traverse   │     │• Executor   │     │• Version    │
//!         │• Aggregate  │     │• Metrics    │     │• Compress   │
//!         │• Transform  │     │• Analysis   │     │• Validate   │
//!         └─────────────┘     └─────────────┘     └─────────────┘
//! ```
//!
//! ## Integration with HyperQL Components
//!
//! ### Compilation Pipeline
//! The IR sits between parsing and execution in the HyperQL pipeline:
//!
//! 1. **AST → IR**: Compiler converts parsed queries to IR representation
//! 2. **IR Optimization**: Optimizer applies hyperbolic-aware transformations
//! 3. **IR → Execution**: Executor interprets IR operations against data
//!
//! ### Runtime Integration
//! The IR seamlessly integrates with runtime components:
//!
//! - **UDF Calls**: IR includes runtime function invocation operations
//! - **Memory Management**: Coordinates with runtime garbage collection
//! - **Error Handling**: Propagates runtime errors through IR execution
//! - **Performance Monitoring**: Tracks execution metrics at IR level
//!
//! ### Storage Integration
//! The IR works efficiently with hyperspatial storage systems:
//!
//! - **Index Utilization**: IR operations map to optimized index scans
//! - **Data Locality**: Operations scheduled to maximize spatial locality
//! - **Caching Strategy**: Intermediate results cached based on geometric properties
//! - **Persistence Layer**: Direct integration with hyperbolic storage engines
//!
//! ## Performance Characteristics
//!
//! ### Compilation Performance
//! - **AST to IR**: O(N) linear conversion where N is AST node count
//! - **IR Optimization**: O(N log N) for most optimization passes
//! - **IR Serialization**: O(N) with optional compression
//!
//! ### Execution Performance
//! - **Operator Startup**: Minimal overhead for operator initialization
//! - **Data Flow**: Streaming execution with constant memory overhead
//! - **Parallel Execution**: Near-linear scaling with available cores
//! - **Memory Usage**: Bounded by operator-specific requirements
//!
//! ### Storage Performance
//! - **Plan Caching**: Compiled IR plans cached for reuse
//! - **Incremental Compilation**: Only recompile modified query parts
//! - **Version Management**: Efficient storage of multiple IR versions
//!
//! ## Module Organization
//!
//! The IR module is organized into focused submodules:
//!
//! - [`operators`]: IR operator definitions and implementations
//! - [`plans`]: Execution plan structures and optimization
//! - [`serialization`]: Binary and text serialization formats
//!
//! Each submodule provides comprehensive functionality for its domain while
//! maintaining clean interfaces for composition and extension.
//!
//! ## Usage Examples
//!
//! ### Simple Selection Query
//! ```hyperql
//! SELECT name FROM users WHERE age > 25
//! ```
//! Compiles to IR:
//! ```ignore
//! Project(["name"],
//!   Filter(GT(Column("age"), Literal(25)),
//!     Scan("users")))
//! ```
//!
//! ### Geometric Proximity Query
//! ```hyperql  
//! SELECT entity FROM locations
//! WHERE hyperbolic_distance(position, @center) < 2.0
//! ```
//! Compiles to IR:
//! ```ignore
//! Project(["entity"],
//!   GeometricFilter(HyperbolicDistance(Column("position"), Parameter("center")), LT(2.0),
//!     Scan("locations")))
//! ```
//!
//! ### Graph Traversal with Aggregation
//! ```hyperql
//! SELECT COUNT(*) FROM users u
//! TRAVERSE follows -> followers
//! WHERE followers.active = true
//! ```
//! Compiles to IR:
//! ```ignore
//! Aggregate([Count(*)],
//!   Filter(EQ(Column("active"), Literal(true)),
//!     Traverse("follows", "followers",
//!       Scan("users"))))
//! ```

pub mod operators;
pub mod plans;
pub mod serialization;

// Re-export commonly used types
pub use operators::*;
pub use plans::*;

/// Result type for IR operations
pub type IRResult<T> = Result<T, IRError>;

/// Errors that can occur during IR operations
#[derive(Debug, thiserror::Error)]
pub enum IRError {
    #[error("Invalid operator configuration: {0}")]
    InvalidOperator(String),
    
    #[error("Type mismatch in IR operation: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: String,
        actual: String,
    },
    
    #[error("Optimization error: {0}")]
    OptimizationError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Execution error: {0}")]
    ExecutionError(String),
    
    #[error("Resource error: {0}")]
    ResourceError(String),
}

/// IR node identifier for reference and optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// IR execution statistics
#[derive(Debug, Clone, Default)]
pub struct IRStats {
    pub nodes_created: u32,
    pub nodes_optimized: u32,
    pub execution_time_ms: u64,
    pub memory_used_bytes: usize,
    pub rows_processed: u64,
}

// TODO: Implement IR validation framework
// TODO: Add support for custom operator plugins
// TODO: Implement IR-level debugging and tracing
// TODO: Add cost-based optimization framework
// TODO: Implement IR-level caching strategies
// TODO: Add support for distributed IR execution
// TODO: Implement IR profiling and performance analysis
// TODO: Add IR-level security and access control