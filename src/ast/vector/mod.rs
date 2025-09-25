//! # Vector-Specific AST Nodes
//!
//! This module defines AST nodes specifically for vector operations and similarity
//! searches within HyperQL. These nodes represent vector computations, similarity
//! expressions, and high-dimensional operations that integrate seamlessly with
//! the hyperbolic positioning system for enhanced performance.
//!
//! ## Purpose
//!
//! The vector AST nodes enable sophisticated vector query representations:
//! - Vector similarity searches with multiple distance metrics
//! - k-nearest neighbor queries with configurable parameters
//! - Vector arithmetic operations and transformations
//! - High-dimensional aggregations and statistics
//! - Integration with hyperbolic space for spatial-semantic queries
//!
//! ## Design Principles
//!
//! Vector AST nodes follow specific design principles:
//!
//! ### Performance Optimization
//! Nodes are designed for high-performance vector operations:
//! - SIMD-friendly representation of vector operations
//! - Index-aware similarity search specifications
//! - Batch processing support for multiple vectors
//! - Memory-efficient storage of vector expressions
//!
//! ### Mathematical Precision
//! Vector operations maintain numerical accuracy:
//! - IEEE 754 compliant floating-point operations
//! - Configurable precision levels for trade-offs
//! - Overflow and underflow protection
//! - Consistent handling of edge cases and special values
//!
//! ### Hyperbolic Integration
//! Vector nodes integrate with spatial positioning:
//! - Combined vector-spatial similarity measures
//! - Hyperbolic distance weighting of vector operations
//! - Spatial clustering of vector search results
//! - Geometric constraints on vector spaces
//!
//! ## Mathematical Foundations
//!
//! Vector AST nodes are grounded in linear algebra and metric geometry:
//!
//! ### Vector Space Theory
//! Classical vector space concepts:
//! - **Vector Addition**: v + w element-wise addition
//! - **Scalar Multiplication**: αv scaling by scalar α
//! - **Inner Product**: ⟨v,w⟩ = Σᵢ vᵢwᵢ
//! - **Norms**: ||v||_p = (Σᵢ |vᵢ|^p)^(1/p)
//!
//! ### Similarity Metrics
//! Distance and similarity functions:
//! - **Cosine Similarity**: cos(θ) = ⟨v,w⟩/(||v|| ||w||)
//! - **Euclidean Distance**: ||v - w||₂
//! - **Manhattan Distance**: ||v - w||₁ = Σᵢ |vᵢ - wᵢ|
//! - **Hamming Distance**: Number of differing components
//!
//! ### High-Dimensional Geometry
//! Concepts specific to high-dimensional spaces:
//! - **Curse of Dimensionality**: Distance concentration effects
//! - **Hub Phenomena**: Emergence of universal nearest neighbors
//! - **Intrinsic Dimensionality**: Effective dimensionality of data
//! - **Manifold Learning**: Low-dimensional structure discovery
//!
//! ## Core AST Node Types
//!
//! The vector AST includes several categories of nodes:
//!
//! ### Similarity Nodes
//! Nodes for similarity computations:
//! - **SimilaritySearchNode**: General similarity search expressions
//! - **CosineSimilarityNode**: Cosine similarity computations
//! - **EuclideanDistanceNode**: Euclidean distance calculations
//! - **CustomMetricNode**: User-defined similarity metrics
//!
//! ### k-NN Nodes
//! Nodes for nearest neighbor queries:
//! - **KNearestNeighborsNode**: Standard k-NN queries
//! - **ApproximateKNNNode**: Approximate nearest neighbor search
//! - **RangeKNNNode**: k-NN within distance thresholds
//! - **DiverseKNNNode**: Diverse nearest neighbor selection
//!
//! ### Operation Nodes
//! Nodes for vector arithmetic:
//! - **VectorAdditionNode**: Vector addition operations
//! - **VectorScalingNode**: Scalar multiplication
//! - **DotProductNode**: Inner product computation
//! - **NormalizationNode**: Vector normalization
//!
//! ### Aggregation Nodes
//! Nodes for vector aggregations:
//! - **VectorMeanNode**: Centroid computation
//! - **VectorMedianNode**: Geometric median calculation
//! - **VectorVarianceNode**: Covariance matrix computation
//! - **ClusterCentroidNode**: Cluster center calculation
//!
//! ## Performance Features
//!
//! Vector AST nodes include advanced performance optimizations:
//!
//! ### Vectorization Support
//! - SIMD instruction utilization for parallel computation
//! - Batch processing of multiple vector operations
//! - Cache-friendly memory access patterns
//! - Hardware-specific optimizations
//!
//! ### Index Integration
//! - Automatic index selection for similarity searches
//! - Index-aware query planning and optimization
//! - Dynamic index selection based on query patterns
//! - Multi-index query execution strategies
//!
//! ### Memory Management
//! - Lazy evaluation for large vector operations
//! - Streaming processing for memory-bounded queries
//! - Efficient representation of sparse vectors
//! - Memory pool utilization for frequent operations
//!
//! ## Integration Features
//!
//! Vector AST nodes integrate with other system components:
//!
//! ### Spatial Integration
//! - Combined vector-spatial similarity queries
//! - Hyperbolic distance weighting in vector operations
//! - Spatial constraints on vector search spaces
//! - Geometric clustering of vector results
//!
//! ### Type System Integration
//! - Strong typing for vector dimensions and types
//! - Automatic type coercion for compatible operations
//! - Generic vector operations with type parameters
//! - Runtime dimension checking and validation
//!
//! ### Query Optimization
//! - Cost-based optimization of vector operations
//! - Predicate pushdown for filtered similarity searches
//! - Join optimization for vector-relational queries
//! - Parallel execution planning for vector operations
//!
//! ## Module Organization
//!
//! The vector AST module is organized by functionality:
//!
//! - [`similarity`]: Similarity search and distance calculation nodes
//! - [`knn`]: k-nearest neighbor query nodes
//!
//! ## Usage Examples
//!
//! Vector AST nodes enable sophisticated vector queries:
//!
//! ```hyperql
//! -- Find similar documents with spatial constraints
//! SELECT d.title, similarity_score, hyperbolic_distance(d, query_doc)
//! FROM documents d
//! WHERE cosine_similarity(d.embedding, @query_embedding) > 0.8
//!   AND hyperbolic_distance(d, @query_doc) < 3.0
//! ORDER BY similarity_score DESC
//! LIMIT 10;
//!
//! -- k-NN search with diverse results
//! SELECT item, vector_distance, spatial_cluster
//! FROM products
//! ORDER BY euclidean_distance(features, @target_features) ASC
//! LIMIT 20 DIVERSE BY spatial_cluster(position, 1.5);
//!
//! -- Vector aggregation with spatial grouping
//! SELECT cluster_id, 
//!        vector_centroid(embeddings) as center,
//!        count(*) as size
//! FROM entities
//! GROUP BY hyperbolic_cluster(position, 2.0)
//! HAVING size > 5
//! ORDER BY size DESC;
//! ```
//!
//! This comprehensive vector AST enables HyperQL to express complex
//! vector operations while leveraging hyperbolic positioning for
//! enhanced performance and natural integration with spatial queries.

pub mod similarity;
pub mod knn;