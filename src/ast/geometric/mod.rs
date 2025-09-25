//! # Geometric-Specific AST Nodes
//!
//! This module defines AST nodes specifically for geometric operations in hyperbolic space.
//! These nodes represent spatial computations, distance-based filtering, and region-based
//! queries that operate on the hyperbolic positioning system within Hyperspatial.
//!
//! ## Purpose
//!
//! The geometric AST nodes enable sophisticated spatial query representations:
//! - WITHIN operations for radius-based filtering in hyperbolic space
//! - NEAR operations for proximity queries with distance constraints
//! - IN_RADIUS operations for region-based entity selection
//! - Integration with the hyperbolic positioning system for optimal performance
//!
//! ## Design Principles
//!
//! Geometric AST nodes follow specific design principles:
//!
//! ### Mathematical Precision
//! Nodes maintain accuracy in hyperbolic geometry:
//! - IEEE 754 compliant hyperbolic distance calculations
//! - Proper handling of hyperbolic metric properties
//! - Numerical stability for extreme distance values
//! - Consistent coordinate system transformations
//!
//! ### Performance Optimization
//! Nodes are designed for high-performance spatial operations:
//! - Index-aware spatial query specifications
//! - Batch processing support for multiple geometric operations
//! - Memory-efficient storage of geometric expressions
//! - SIMD-friendly representation where applicable
//!
//! ### Hyperbolic Integration
//! Geometric nodes leverage the hyperbolic positioning system:
//! - Native hyperbolic distance computations
//! - Hyperbolic region and boundary operations
//! - Integration with learned entity positions
//! - Spatial clustering and hierarchy awareness
//!
//! ## Hyperbolic Space Theory
//!
//! Geometric AST nodes are grounded in hyperbolic geometry:
//!
//! ### Hyperbolic Distance
//! Distance computations in the Poincaré ball model:
//! - **Hyperbolic Distance**: d(x,y) = artanh(||x⊖y||) where ⊖ is Möbius subtraction
//! - **Möbius Operations**: Specialized arithmetic for hyperbolic space
//! - **Geodesics**: Shortest paths in hyperbolic geometry
//! - **Isometries**: Distance-preserving transformations
//!
//! ### Spatial Regions
//! Region-based operations in hyperbolic space:
//! - **Hyperbolic Circles**: Sets of points at fixed hyperbolic distance
//! - **Hyperbolic Balls**: Interior regions bounded by hyperbolic circles
//! - **Hierarchical Regions**: Nested spatial structures reflecting data organization
//! - **Boundary Operations**: Intersection and containment tests
//!
//! ## Core AST Node Types
//!
//! The geometric AST includes several categories of nodes:
//!
//! ### Distance Nodes
//! Nodes for distance-based operations:
//! - **WithinNode**: Radius-based filtering (WITHIN operation)
//! - **NearNode**: Proximity queries with distance constraints (NEAR operation) 
//! - **InRadiusNode**: Region-based selection (IN_RADIUS operation)
//!
//! ### Position Nodes
//! Nodes for position operations:
//! - **PositionRef**: References to entity positions in hyperbolic space
//! - **CoordinateExpression**: Explicit coordinate specifications
//! - **CenterPoint**: Reference points for geometric operations
//!
//! ## Module Organization
//!
//! The geometric AST module is organized by operation type:
//!
//! - [`within`]: WITHIN operation AST nodes and processing
//! - [`near`]: NEAR operation AST nodes and processing
//! - [`radius`]: IN_RADIUS operation AST nodes and processing
//!
//! ## Usage Examples
//!
//! Geometric AST nodes enable sophisticated spatial queries:
//!
//! ```hyperql
//! -- Find entities within hyperbolic radius
//! SELECT * FROM entities WHERE position WITHIN 2.5 OF target_position;
//!
//! -- Find entities near a reference point
//! SELECT * FROM entities WHERE position NEAR reference_point DISTANCE 3.0;
//!
//! -- Check if entities are in a hyperbolic region
//! SELECT * FROM entities WHERE position IN_RADIUS(center_point, 5.0);
//! ```
//!
//! This comprehensive geometric AST enables HyperQL to express complex
//! hyperbolic space operations while leveraging the positioning system for
//! enhanced performance and natural spatial query semantics.

pub mod within;
pub mod near;
pub mod radius;

use serde::{Deserialize, Serialize};
use crate::ast::Expression;

/// Geometric expressions for hyperbolic space operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeometricExpression {
    /// WITHIN operation for radius-based filtering
    Within {
        target: Box<Expression>,
        radius: f64,
        reference: Box<Expression>,
    },
    /// NEAR operation for proximity queries
    Near {
        target: Box<Expression>,
        reference: Box<Expression>,
        max_distance: f64,
    },
    /// IN_RADIUS operation for region-based selection
    InRadius {
        target: Box<Expression>,
        center: Box<Expression>,
        radius: f64,
    },
}

/// Reference to a position in hyperbolic space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionRef {
    /// Table or entity reference
    pub table: Option<String>,
    /// Position field name (typically "position")
    pub field: String,
}

/// Explicit coordinate specification in hyperbolic space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinateExpression {
    /// X coordinate in Poincaré ball model
    pub x: f64,
    /// Y coordinate in Poincaré ball model  
    pub y: f64,
    /// Z coordinate in Poincaré ball model
    pub z: f64,
}