//! # Query Execution Operators
//!
//! This module contains the core execution operators that implement the runtime
//! behavior of HyperQL queries. These operators form the building blocks of the
//! execution engine, providing specialized implementations for different query
//! paradigms while leveraging hyperbolic positioning for optimal performance.
//!
//! ## Purpose
//!
//! The operators module serves as the execution foundation:
//! - Implements the physical operators for query execution
//! - Provides paradigm-specific optimization strategies
//! - Enables efficient data processing across different query types
//! - Supports vectorized and parallel execution models
//! - Integrates with hyperbolic spatial indices and structures
//!
//! ## Operator Architecture
//!
//! All operators follow a consistent architectural pattern:
//!
//! ### Iterator Pattern
//! Operators implement a pull-based iterator interface:
//! - **next()**: Retrieve the next batch of results
//! - **reset()**: Reset operator to initial state
//! - **close()**: Clean up resources and finalize execution
//! - **statistics()**: Provide execution statistics and metrics
//!
//! ### Vectorized Execution
//! Operators process data in batches for efficiency:
//! - **Batch Processing**: Process multiple rows simultaneously
//! - **SIMD Operations**: Leverage hardware vectorization
//! - **Cache Optimization**: Optimize memory access patterns
//! - **Pipeline Processing**: Enable efficient operator composition
//!
//! ### Resource Management
//! Operators include comprehensive resource management:
//! - **Memory Pools**: Efficient memory allocation and reuse
//! - **Spill-to-Disk**: Handle datasets larger than memory
//! - **Parallel Execution**: Multi-threaded processing support
//! - **Resource Limits**: Configurable resource consumption bounds
//!
//! ## Operator Categories
//!
//! Execution operators are organized by paradigm and functionality:
//!
//! ### Graph Operators
//! Specialized operators for graph query execution:
//! - **TraversalOperator**: Graph traversal with constraints
//! - **PatternMatchOperator**: Subgraph pattern matching
//! - **PathOperator**: Path finding and analysis
//! - **CentralityOperator**: Centrality measure computation
//! - **ClusteringOperator**: Community detection and clustering
//!
//! ### Vector Operators
//! Operators for vector operations and similarity search:
//! - **SimilaritySearchOperator**: Vector similarity queries
//! - **KNNOperator**: k-nearest neighbor search
//! - **VectorAggregateOperator**: Vector aggregation operations
//! - **IndexScanOperator**: Vector index-based scanning
//! - **QuantizationOperator**: Vector compression and decompression
//!
//! ### Timeseries Operators
//! Operators for temporal data processing:
//! - **WindowOperator**: Window function evaluation
//! - **TemporalJoinOperator**: Time-based join operations
//! - **AggregateOperator**: Temporal aggregation functions
//! - **FilterOperator**: Time-based filtering and selection
//! - **SortOperator**: Temporal ordering and ranking
//!
//! ### Relational Operators
//! Traditional relational operators with spatial enhancements:
//! - **ScanOperator**: Table scanning with spatial optimization
//! - **JoinOperator**: Join operations with spatial acceleration
//! - **AggregateOperator**: Grouping and aggregation with spatial clustering
//! - **SortOperator**: Sorting with hyperbolic distance ordering
//! - **FilterOperator**: Selection with geometric constraints
//!
//! ## Performance Features
//!
//! Operators implement advanced performance optimizations:
//!
//! ### Parallel Processing
//! - **Operator Parallelism**: Multiple threads per operator
//! - **Pipeline Parallelism**: Concurrent operator execution
//! - **Data Parallelism**: Partitioned data processing
//! - **Work Stealing**: Dynamic load balancing
//!
//! ### Memory Management
//! - **Batch Allocation**: Efficient batch memory management
//! - **Memory Pools**: Reusable memory allocation strategies
//! - **Spill Handling**: Transparent spill-to-disk operations
//! - **Cache Management**: Intelligent data caching strategies
//!
//! ### Index Integration
//! - **Spatial Indices**: Native hyperbolic index utilization
//! - **Vector Indices**: Integration with similarity search indices
//! - **Temporal Indices**: Time-based index optimization
//! - **Composite Indices**: Multi-dimensional index support
//!
//! ## Hyperbolic Optimizations
//!
//! All operators include hyperbolic space optimizations:
//!
//! ### Spatial Acceleration
//! - **Distance Pruning**: Early termination based on geometric constraints
//! - **Locality Optimization**: Process spatially nearby data together
//! - **Hierarchical Processing**: Leverage natural hierarchy for efficiency
//! - **Geometric Filtering**: Apply spatial constraints during execution
//!
//! ### Cross-Paradigm Integration
//! - **Spatial Joins**: Join operations based on hyperbolic proximity
//! - **Geometric Grouping**: Clustering-based aggregation strategies
//! - **Position-Aware Sorting**: Order results by spatial relationships
//! - **Trajectory Analysis**: Path-based filtering and analysis
//!
//! ## Integration Features
//!
//! Operators integrate with the broader query execution system:
//!
//! ### Query Planning Integration
//! - **Cost Estimation**: Accurate cost models for optimization
//! - **Cardinality Estimation**: Result set size prediction
//! - **Selectivity Analysis**: Filter effectiveness estimation
//! - **Resource Planning**: Memory and CPU requirement prediction
//!
//! ### Monitoring Integration
//! - **Execution Metrics**: Real-time performance monitoring
//! - **Resource Usage**: CPU, memory, and I/O utilization tracking
//! - **Progress Reporting**: Query execution progress updates
//! - **Error Handling**: Comprehensive error detection and recovery
//!
//! ## Module Organization
//!
//! The operators module is organized by paradigm:
//!
//! - [`graph`]: Graph query execution operators
//! - [`vector`]: Vector operation execution operators
//! - [`timeseries`]: Temporal data processing operators
//! - [`relational`]: Traditional relational operators
//!
//! ## Usage Examples
//!
//! Operators are composed to form execution plans:
//!
//! ```ignore
//! use hyperql::executor::operators::{
//!     graph::TraversalOperator,
//!     vector::SimilaritySearchOperator,
//!     relational::JoinOperator,
//! };
//!
//! // Create a complex execution plan
//! let traversal = TraversalOperator::new(traversal_config);
//! let similarity = SimilaritySearchOperator::new(vector_config);
//! let join = JoinOperator::new(join_config);
//!
//! // Compose operators in execution pipeline
//! let plan = join
//!     .with_left_input(traversal)
//!     .with_right_input(similarity)
//!     .build();
//!
//! // Execute the plan
//! let results = plan.execute(execution_context)?;
//! ```
//!
//! This comprehensive operator framework enables HyperQL to deliver
//! high-performance execution across all supported query paradigms
//! while leveraging hyperbolic positioning for enhanced optimization
//! and cross-paradigm integration capabilities.

pub mod graph;
pub mod vector;
pub mod timeseries;
pub mod relational;