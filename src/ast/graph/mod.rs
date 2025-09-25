//! # Graph-Specific AST Nodes
//!
//! This module defines AST nodes specifically for graph query operations within
//! HyperQL. These nodes represent graph traversal patterns, relationship navigation,
//! and graph-specific expressions that leverage the hyperbolic positioning system
//! for optimal performance.
//!
//! ## Purpose
//!
//! The graph AST nodes enable sophisticated graph query representations:
//! - Graph traversal patterns with constraints and filters
//! - Relationship navigation with property-based selection
//! - Path expressions for complex route specifications
//! - Graph pattern matching with structural requirements
//! - Integration with hyperbolic distance calculations
//!
//! ## Design Principles
//!
//! Graph AST nodes follow specific design principles:
//!
//! ### Hyperbolic Integration
//! All graph nodes include spatial awareness:
//! - Position-based traversal optimizations
//! - Distance-constrained relationship following
//! - Spatial clustering for efficient pattern matching
//! - Geometric filters on graph operations
//!
//! ### Performance Optimization
//! Nodes are designed for query optimization:
//! - Early termination conditions for traversals
//! - Index-friendly representation of patterns
//! - Parallelizable operation specifications
//! - Cache-aware data access patterns
//!
//! ### Composability
//! Graph nodes compose naturally with other query constructs:
//! - Integration with vector similarity searches
//! - Compatibility with temporal constraints
//! - Seamless embedding in larger query structures
//! - Cross-paradigm operation support
//!
//! ## Mathematical Foundations
//!
//! Graph AST nodes are grounded in graph theory and hyperbolic geometry:
//!
//! ### Graph Theory
//! Classical graph concepts represented in AST:
//! - **Vertices and Edges**: Basic graph structure elements
//! - **Paths and Walks**: Sequences of connected vertices
//! - **Cycles and Trees**: Structural patterns in graphs
//! - **Subgraphs**: Portions of larger graph structures
//!
//! ### Hyperbolic Graph Theory
//! Hyperbolic space concepts for graph operations:
//! - **Hyperbolic Distance**: d_H(u,v) for vertex pairs
//! - **Spatial Neighborhoods**: Vertices within distance thresholds
//! - **Geometric Clustering**: Natural groupings in hyperbolic space
//! - **Hierarchical Structure**: Tree-like organization emergence
//!
//! ### Pattern Matching Theory
//! Mathematical foundations for pattern detection:
//! - **Subgraph Isomorphism**: Pattern matching in graphs
//! - **Approximate Matching**: Fuzzy pattern recognition
//! - **Regular Path Queries**: Pattern specification languages
//! - **Structural Similarity**: Graph comparison metrics
//!
//! ## Core AST Node Types
//!
//! The graph AST includes several categories of nodes:
//!
//! ### Traversal Nodes
//! Nodes representing graph navigation:
//! - **TraverseNode**: Basic relationship following
//! - **MultiHopTraverseNode**: Multi-step traversals
//! - **ConditionalTraverseNode**: Conditional path following
//! - **BidirectionalTraverseNode**: Bidirectional exploration
//!
//! ### Pattern Nodes
//! Nodes for structural pattern matching:
//! - **GraphPatternNode**: Complete pattern specifications
//! - **SubgraphPatternNode**: Partial pattern matching
//! - **PathPatternNode**: Linear pattern sequences
//! - **CyclePatternNode**: Circular pattern detection
//!
//! ### Constraint Nodes
//! Nodes for filtering and constraints:
//! - **RelationshipFilterNode**: Edge-based filtering
//! - **VertexFilterNode**: Node-based constraints
//! - **DistanceConstraintNode**: Spatial distance limits
//! - **PropertyConstraintNode**: Attribute-based filtering
//!
//! ### Aggregation Nodes
//! Nodes for graph-based aggregations:
//! - **PathAggregateNode**: Aggregation along paths
//! - **ClusterAggregateNode**: Spatial cluster aggregation
//! - **CentralityNode**: Centrality measure computation
//! - **ConnectivityNode**: Graph connectivity analysis
//!
//! ## Integration Features
//!
//! Graph AST nodes integrate with other system components:
//!
//! ### Spatial Integration
//! - Hyperbolic distance calculations in traversals
//! - Spatial constraints on relationship following
//! - Position-aware pattern matching
//! - Geometric optimization of graph operations
//!
//! ### Index Integration
//! - Graph index utilization for fast traversals
//! - Spatial index integration for distance queries
//! - Pattern index usage for frequent patterns
//! - Composite index support for complex queries
//!
//! ### Type System Integration
//! - Strong typing for graph elements
//! - Type checking for relationship compatibility
//! - Generic patterns with type parameters
//! - Runtime type validation and coercion
//!
//! ## Performance Characteristics
//!
//! Graph AST nodes are designed for optimal performance:
//!
//! ### Complexity Analysis
//! - **Traversal Nodes**: O(d^k) where d is degree and k is depth
//! - **Pattern Nodes**: O(n^p) where n is graph size and p is pattern size
//! - **Constraint Nodes**: O(n) for linear constraints
//! - **Aggregation Nodes**: O(n log n) for sorted aggregations
//!
//! ### Memory Efficiency
//! - Lazy evaluation for large result sets
//! - Streaming processing for memory-bounded operations
//! - Efficient representation of sparse patterns
//! - Reference sharing for common subexpressions
//!
//! ### Optimization Opportunities
//! - Early termination for bounded searches
//! - Index pushdown for selective operations
//! - Parallel execution for independent traversals
//! - Cache utilization for repeated patterns
//!
//! ## Module Organization
//!
//! The graph AST module is organized by functionality:
//!
//! - [`patterns`]: Graph pattern matching AST nodes
//! - [`paths`]: Path expression and traversal nodes
//!
//! ## Usage Examples
//!
//! Graph AST nodes enable sophisticated graph queries:
//!
//! ```hyperql
//! -- Find triangular patterns in social network
//! SELECT a.name, b.name, c.name
//! FROM users a
//! TRAVERSE friend -> users b
//! TRAVERSE friend -> users c
//! WHERE EXISTS(TRAVERSE friend FROM c TO a)
//!   AND hyperbolic_distance(a, b) < 2.0
//! ORDER BY spatial_clustering_score(a, b, c) DESC;
//!
//! -- Multi-hop traversal with distance constraints
//! SELECT destination, path_length, total_distance
//! FROM locations start
//! TRAVERSE connected_to{1,5} -> locations destination
//! WHERE hyperbolic_distance(start, destination) < 10.0
//!   AND path_length <= 5
//! GROUP BY spatial_cluster(destination, 1.0)
//! ORDER BY avg(total_distance) ASC;
//! ```
//!
//! This comprehensive graph AST enables HyperQL to express complex
//! graph queries while leveraging hyperbolic positioning for optimal
//! performance and natural integration with other query paradigms.

pub mod patterns;
pub mod paths;