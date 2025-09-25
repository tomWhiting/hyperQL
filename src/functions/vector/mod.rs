//! # Vector-Specific Functions
//!
//! This module provides a comprehensive collection of vector-specific functions
//! for HyperQL queries. These functions enable high-dimensional vector operations,
//! similarity searches, and embedding manipulations while integrating seamlessly
//! with hyperbolic positioning for enhanced spatio-semantic analysis.
//!
//! ## Purpose
//!
//! The vector functions module serves multiple purposes:
//! - Provides vector similarity and distance computations
//! - Implements k-nearest neighbor search functions
//! - Enables vector arithmetic and transformation operations
//! - Supports high-dimensional statistical analysis
//! - Integrates vector operations with hyperbolic geometry
//!
//! ## Function Categories
//!
//! Vector functions are organized into specialized categories:
//!
//! ### Similarity Functions
//! Functions for vector similarity and distance:
//! - **cosine_similarity()**: Cosine similarity computation
//! - **euclidean_distance()**: L2 norm distance calculation
//! - **manhattan_distance()**: L1 norm distance calculation
//! - **hamming_distance()**: Binary vector distance
//! - **custom_metric()**: User-defined similarity functions
//!
//! ### Distance Functions
//! Specialized distance metric implementations:
//! - **minkowski_distance()**: Generalized Lp norm distances
//! - **mahalanobis_distance()**: Covariance-weighted distance
//! - **jaccard_similarity()**: Set similarity for sparse vectors
//! - **pearson_correlation()**: Statistical correlation measure
//! - **spearman_correlation()**: Rank-based correlation
//!
//! ## Mathematical Foundations
//!
//! Vector functions are grounded in linear algebra and metric geometry:
//!
//! ### Vector Space Theory
//! Mathematical foundations for vector operations:
//! - **Inner Products**: ⟨u,v⟩ = Σᵢ uᵢvᵢ
//! - **Norms**: ||v||_p = (Σᵢ |vᵢ|^p)^(1/p)
//! - **Orthogonality**: Perpendicular vector relationships
//! - **Linear Independence**: Basis and spanning concepts
//!
//! ### Metric Geometry
//! Distance and similarity theory:
//! - **Metric Properties**: Non-negativity, symmetry, triangle inequality
//! - **Similarity Measures**: Equivalence to distance through transformation
//! - **Curse of Dimensionality**: High-dimensional distance behavior
//! - **Concentration Phenomena**: Distance concentration in high dimensions
//!
//! ## Module Organization
//!
//! The vector functions module is organized by functionality:
//!
//! - [`similarity`]: Vector similarity and correlation functions
//! - [`distance`]: Distance metric implementations
//!
//! This organization enables efficient vector operations while maintaining
//! clear separation of concerns and supporting various similarity paradigms.

pub mod similarity;
pub mod distance;