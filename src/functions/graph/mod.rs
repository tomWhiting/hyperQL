//! # Graph-Specific Functions
//!
//! This module provides a comprehensive collection of graph-specific functions
//! for HyperQL queries. These functions enable sophisticated graph analysis,
//! traversal operations, and network metrics while leveraging hyperbolic
//! positioning for enhanced performance and natural hierarchy representation.
//!
//! ## Purpose
//!
//! The graph functions module serves multiple purposes:
//! - Provides graph traversal and navigation functions
//! - Implements centrality measures and network metrics
//! - Enables structural analysis and pattern detection
//! - Supports path analysis and reachability queries
//! - Integrates graph operations with hyperbolic geometry
//!
//! ## Function Categories
//!
//! Graph functions are organized into specialized categories:
//!
//! ### Traversal Functions
//! Functions for graph navigation and exploration:
//! - **traverse()**: Basic relationship traversal with constraints
//! - **shortest_path()**: Shortest path computation between nodes
//! - **reachable()**: Reachability analysis with depth limits
//! - **connected_components()**: Component identification and analysis
//! - **bidirectional_search()**: Meet-in-the-middle path finding
//!
//! ### Algorithm Functions
//! Implementations of classic graph algorithms:
//! - **pagerank()**: PageRank centrality computation
//! - **betweenness_centrality()**: Betweenness centrality calculation
//! - **community_detection()**: Community structure identification
//! - **minimum_spanning_tree()**: MST computation with custom weights
//! - **maximum_flow()**: Network flow analysis
//!
//! ### Metric Functions
//! Graph property and metric calculations:
//! - **clustering_coefficient()**: Local and global clustering measures
//! - **degree_distribution()**: Degree sequence analysis
//! - **graph_diameter()**: Maximum shortest path length
//! - **assortativity()**: Degree correlation analysis
//! - **modularity()**: Community structure quality measure
//!
//! ### Pattern Functions
//! Structural pattern detection and analysis:
//! - **motif_count()**: Subgraph motif enumeration
//! - **triangle_count()**: Triangle enumeration and analysis
//! - **clique_detection()**: Maximal clique identification
//! - **pattern_match()**: General pattern matching functions
//! - **isomorphism()**: Graph and subgraph isomorphism testing
//!
//! ## Hyperbolic Enhancements
//!
//! All graph functions include hyperbolic space optimizations:
//!
//! ### Spatial Acceleration
//! - **Distance Pruning**: Use hyperbolic distance for early termination
//! - **Locality-Aware Processing**: Process spatially nearby nodes together
//! - **Hierarchical Traversal**: Leverage natural hierarchy for efficiency
//! - **Geometric Constraints**: Apply spatial constraints to graph operations
//!
//! ### Performance Optimization
//! - **Spatial Indexing**: Integration with hyperbolic spatial indices
//! - **Cache Locality**: Optimize memory access patterns using position
//! - **Parallel Processing**: Partition work based on spatial clusters
//! - **Approximation**: Trade accuracy for speed using geometric approximation
//!
//! ## Mathematical Foundations
//!
//! Graph functions are grounded in graph theory and hyperbolic geometry:
//!
//! ### Graph Theory
//! Classical graph theoretical foundations:
//! - **Adjacency Relations**: Formal definition of graph structure
//! - **Walk and Path Theory**: Sequences of vertices and edges
//! - **Connectivity**: Strong and weak connectivity concepts
//! - **Spectral Graph Theory**: Eigenvalue analysis of graph matrices
//!
//! ### Network Analysis
//! Modern network analysis techniques:
//! - **Centrality Measures**: Formal definitions of node importance
//! - **Community Structure**: Mathematical foundations of clustering
//! - **Random Walks**: Stochastic processes on graphs
//! - **Network Flows**: Flow theory and optimization
//!
//! ### Hyperbolic Graph Theory
//! Integration with hyperbolic geometry:
//! - **Hyperbolic Embeddings**: Natural representation of tree-like structures
//! - **Geometric Graph Models**: Probabilistic geometric graph generation
//! - **Distance Correlation**: Relationship between graph and geometric distance
//! - **Hierarchical Structure**: Emergence of hierarchy in hyperbolic space
//!
//! ## Performance Characteristics
//!
//! Graph functions are optimized for various performance profiles:
//!
//! ### Computational Complexity
//! - **Traversal Functions**: O(V + E) for basic traversal operations
//! - **Shortest Paths**: O(V log V + E) for single-source, O(V³) for all-pairs
//! - **Centrality Measures**: O(VE) for betweenness, O(V³) for eigenvector
//! - **Community Detection**: O(E) for label propagation, O(V³) for spectral
//!
//! ### Scalability Features
//! - **Streaming Processing**: Handle graphs larger than memory
//! - **Approximation Algorithms**: Trade accuracy for speed on large graphs
//! - **Parallel Execution**: Multi-threaded implementations for suitable algorithms
//! - **Incremental Updates**: Efficient updates for dynamic graphs
//!
//! ## Module Organization
//!
//! The graph functions module is organized by functionality:
//!
//! - [`traversal`]: Graph traversal and navigation functions
//! - [`algorithms`]: Classic graph algorithm implementations
//!
//! ## Usage Examples
//!
//! Graph functions enable sophisticated graph analysis in queries:
//!
//! ```hyperql
//! -- Find influential users in social network
//! SELECT user_id, 
//!        pagerank(user_network, damping := 0.85) as influence,
//!        betweenness_centrality(user_network) as broker_score
//! FROM social_graph
//! WHERE hyperbolic_distance(user_position, @center) < 3.0
//! ORDER BY influence DESC
//! LIMIT 10;
//!
//! -- Detect communities with spatial constraints
//! SELECT community_id,
//!        count(*) as size,
//!        spatial_centroid(positions) as center,
//!        modularity(subgraph) as quality
//! FROM (
//!   SELECT community_detection(graph, method := 'louvain') as community_id,
//!          node_id,
//!          position
//!   FROM network_data
//!   WHERE component_id = largest_component(graph)
//! ) WITH ROLLUP
//! GROUP BY community_id
//! HAVING size > 5 AND quality > 0.3
//! ORDER BY size DESC;
//! ```
//!
//! This comprehensive function library enables sophisticated graph
//! analysis while leveraging hyperbolic positioning for enhanced
//! performance and natural representation of hierarchical structures.

pub mod traversal;
pub mod algorithms;