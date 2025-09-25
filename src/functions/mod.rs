//! # Built-in Functions for HyperQL
//!
//! This module provides a comprehensive library of built-in functions that extend
//! HyperQL's expressive power across vector operations, geometric computations,
//! and aggregate functions. These functions are optimized for hyperbolic space
//! operations and integrate seamlessly with the query execution engine.
//!
//! ## Purpose
//!
//! The functions module serves multiple critical roles:
//! - Provides a rich library of mathematical and geometric operations
//! - Enables complex computations within query expressions
//! - Supports vector similarity and hyperbolic geometry calculations
//! - Implements aggregate functions with spatial awareness
//! - Facilitates custom function development and registration
//!
//! ## Function Categories
//!
//! HyperQL's built-in functions are organized into distinct categories:
//!
//! ### Mathematical Functions
//! Standard mathematical operations enhanced for multi-dimensional data:
//! - **Arithmetic**: Addition, subtraction, multiplication, division
//! - **Trigonometric**: Sin, cos, tan with hyperbolic variants
//! - **Exponential**: Log, exp, power functions
//! - **Statistical**: Mean, median, standard deviation, percentiles
//!
//! ### Vector Operations
//! Specialized functions for vector and embedding computations:
//! - **Similarity**: Cosine similarity, dot product, Euclidean distance
//! - **Normalization**: L1, L2, and max normalization
//! - **Dimensionality**: Vector length, dimension counting
//! - **Transformation**: Vector scaling, rotation, projection
//!
//! ### Hyperbolic Geometry
//! Functions specific to hyperbolic space operations:
//! - **Distance**: Hyperbolic distance calculation between positions
//! - **Geodesics**: Shortest path computation in hyperbolic space
//! - **Regions**: Containment testing and region queries
//! - **Transformations**: Isometries and hyperbolic transformations
//!
//! ### Spatial Analysis
//! Advanced spatial analysis functions:
//! - **Clustering**: Density-based clustering in hyperbolic space
//! - **Neighborhoods**: k-nearest neighbors with hyperbolic metrics
//! - **Hierarchies**: Hierarchy detection and tree extraction
//! - **Trajectories**: Path analysis and movement pattern detection
//!
//! ### Aggregate Functions
//! Multi-dimensional aggregate operations:
//! - **Spatial Aggregates**: Center of mass, spatial variance
//! - **Vector Aggregates**: Centroid calculation, consensus embeddings
//! - **Statistical Aggregates**: Multi-dimensional statistics
//! - **Temporal Aggregates**: Time-aware aggregation functions
//!
//! ## Function Architecture
//!
//! Functions follow a consistent architectural pattern:
//!
//! ### Type Safety
//! - **Static Typing**: Compile-time type checking for function arguments
//! - **Coercion Rules**: Automatic type conversion where appropriate
//! - **Null Handling**: Consistent null value propagation semantics
//! - **Error Propagation**: Structured error handling across function calls
//!
//! ### Performance Optimization
//! - **Vectorization**: SIMD-optimized implementations where applicable
//! - **Caching**: Intelligent caching of expensive computations
//! - **Lazy Evaluation**: Deferred computation for conditional expressions
//! - **Memory Efficiency**: Minimal allocation and zero-copy operations
//!
//! ### Extensibility
//! - **Plugin Architecture**: Support for custom function registration
//! - **UDF Support**: User-defined function integration
//! - **Operator Overloading**: Custom operators for domain-specific functions
//! - **Function Composition**: Higher-order functions and composition patterns
//!
//! ## Hyperbolic Function Specializations
//!
//! Many functions include specialized implementations for hyperbolic operations:
//!
//! ### Distance Functions
//! ```hyperql
//! hyperbolic_distance(pos1, pos2)          -- Standard hyperbolic distance
//! hyperbolic_distance_squared(pos1, pos2)  -- Squared distance (faster)
//! geodesic_distance(pos1, pos2)            -- Geodesic distance along surface
//! ```
//!
//! ### Similarity Functions
//! ```hyperql
//! spatial_similarity(entity1, entity2)     -- Position-based similarity
//! combined_similarity(e1, e2, weights)     -- Vector + spatial similarity
//! hierarchical_similarity(e1, e2)          -- Hierarchy-aware similarity
//! ```
//!
//! ### Region Functions
//! ```hyperql
//! hyperbolic_contains(region, position)    -- Point-in-region testing
//! hyperbolic_intersection(region1, region2) -- Region intersection
//! hyperbolic_ball(center, radius)          -- Create hyperbolic ball region
//! ```
//!
//! ## Module Organization
//!
//! The functions module is organized by functional category:
//!
//! - [`math`]: Core mathematical functions and operations
//! - [`vector`]: Vector and embedding operations
//! - [`geometry`]: Hyperbolic geometry and spatial functions
//! - [`similarity`]: Similarity and distance computations
//! - [`aggregate`]: Aggregation functions with spatial awareness
//! - [`temporal`]: Time-based functions and temporal analysis
//! - [`string`]: Text processing and string manipulation
//! - [`conversion`]: Type conversion and casting functions
//! - [`registry`]: Function registration and lookup system
//! - [`udf`]: User-defined function support and integration
//!
//! ## Function Registration System
//!
//! HyperQL includes a sophisticated function registration system:
//!
//! ### Built-in Registration
//! All built-in functions are automatically registered with:
//! - Function signature information
//! - Type checking rules
//! - Performance characteristics
//! - Documentation and examples
//!
//! ### Custom Functions
//! Support for registering custom functions with:
//! - Rust-based function implementations
//! - Dynamic library loading
//! - Scripting language integration
//! - Performance monitoring and profiling
//!
//! ### Function Metadata
//! Each function includes comprehensive metadata:
//! - Parameter types and constraints
//! - Return type specifications
//! - Performance complexity information
//! - Usage examples and documentation
//!
//! ## Performance Characteristics
//!
//! Functions are optimized for query execution performance:
//!
//! ### Vectorized Operations
//! - **SIMD Instructions**: Hardware-accelerated computations
//! - **Batch Processing**: Process multiple values simultaneously
//! - **Memory Locality**: Cache-friendly memory access patterns
//! - **Pipeline Optimization**: Minimize CPU pipeline stalls
//!
//! ### Caching Strategies
//! - **Result Caching**: Cache expensive computation results
//! - **Memoization**: Automatic function result memoization
//! - **Index Integration**: Leverage existing index structures
//! - **Lazy Loading**: Defer expensive operations when possible
//!
//! ### Resource Management
//! - **Memory Pools**: Pre-allocated memory for frequent operations
//! - **Thread Safety**: Lock-free implementations where possible
//! - **Error Recovery**: Graceful handling of computation failures
//! - **Resource Limits**: Configurable limits for resource usage
//!
//! ## Integration Features
//!
//! Functions integrate deeply with other HyperQL components:
//!
//! ### Query Optimizer Integration
//! - **Cost Estimation**: Accurate cost models for optimization
//! - **Predicate Pushdown**: Optimization-friendly function implementations
//! - **Index Utilization**: Functions that can leverage existing indices
//! - **Parallel Execution**: Functions designed for parallel execution
//!
//! ### Type System Integration
//! - **Static Analysis**: Compile-time function signature checking
//! - **Type Inference**: Automatic result type inference
//! - **Coercion Support**: Seamless integration with type coercion
//! - **Generic Functions**: Polymorphic function implementations
//!
//! ### Error Handling Integration
//! - **Structured Errors**: Consistent error reporting across all functions
//! - **Context Preservation**: Maintain query context in error messages
//! - **Recovery Strategies**: Automatic fallback for failed computations
//! - **Debugging Support**: Detailed execution tracing for development
//!
//! ## Usage Examples
//!
//! Functions enable powerful query expressions:
//!
//! ```hyperql
//! -- Combine vector similarity with spatial proximity
//! SELECT u.name, f.name,
//!        combined_similarity(u, f, [0.6, 0.4]) as score
//! FROM users u, users f
//! WHERE hyperbolic_distance(u, f) < 2.0
//!   AND cosine_similarity(u.interests, f.interests) > 0.7
//! ORDER BY score DESC
//! LIMIT 10;
//!
//! -- Spatial clustering with temporal constraints
//! SELECT cluster_id,
//!        spatial_centroid(positions) as center,
//!        count(*) as members
//! FROM entity_positions
//! WHERE timestamp > now() - interval '1 day'
//! GROUP BY hyperbolic_cluster(position, 0.5)
//! ORDER BY members DESC;
//! ```
//!
//! This comprehensive function library enables HyperQL to support
//! sophisticated multi-paradigm queries with high performance and
//! mathematical precision across all supported data types and operations.

pub mod graph;
pub mod vector;
pub mod timeseries;