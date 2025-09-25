//! # Geometric-Specific Functions
//!
//! This module provides a comprehensive collection of geometric functions for
//! hyperbolic space operations within HyperQL queries. These functions enable
//! spatial computations, distance calculations, and region-based operations
//! that leverage the hyperbolic positioning system.
//!
//! ## Purpose
//!
//! The geometric functions module serves multiple purposes:
//! - Provides hyperbolic distance and region calculations
//! - Implements WITHIN, NEAR, and IN_RADIUS operations
//! - Enables spatial filtering and proximity queries
//! - Supports hyperbolic geometry computations
//! - Integrates with the Hyperspatial positioning system
//!
//! ## Function Categories
//!
//! Geometric functions are organized into specialized categories:
//!
//! ### Distance Functions
//! Functions for hyperbolic distance calculations:
//! - **hyperbolic_distance()**: Core distance computation in Poincaré ball model
//! - **geodesic_distance()**: Shortest path distance in hyperbolic space
//! - **approximate_distance()**: Fast approximate distance for large datasets
//!
//! ### Region Functions
//! Functions for spatial region operations:
//! - **within_radius()**: Check if point is within hyperbolic radius
//! - **near_point()**: Proximity queries with distance constraints
//! - **in_region()**: Region containment testing
//!
//! ## Mathematical Foundations
//!
//! Geometric functions are grounded in hyperbolic geometry:
//!
//! ### Poincaré Ball Model
//! Mathematical foundations for hyperbolic operations:
//! - **Hyperbolic Distance**: d(x,y) = artanh(||x⊖y||) where ⊖ is Möbius subtraction
//! - **Möbius Operations**: Specialized arithmetic for hyperbolic space
//! - **Geodesics**: Shortest paths represented as circular arcs
//! - **Isometries**: Distance-preserving transformations in hyperbolic space
//!
//! ### Spatial Operations
//! Core spatial computation concepts:
//! - **Hyperbolic Balls**: Regions of constant hyperbolic distance
//! - **Boundary Operations**: Intersection and containment tests
//! - **Hierarchical Structure**: Natural clustering in hyperbolic space
//!
//! ## Module Organization
//!
//! The geometric functions module is organized by operation type:
//!
//! - [`distance`]: Hyperbolic distance computation functions
//! - [`regions`]: Spatial region and containment functions
//! - [`operations`]: Core geometric operations and utilities
//!
//! This organization enables efficient spatial operations while maintaining
//! clear separation of concerns and supporting various geometric paradigms.

pub mod distance;
pub mod regions;
pub mod operations;

use crate::HyperQLError;
use crate::types::Position3D;

/// Core geometric computation interface
pub trait GeometricComputation {
    /// Compute hyperbolic distance between two positions
    fn hyperbolic_distance(&self, pos1: &Position3D, pos2: &Position3D) -> Result<f64, HyperQLError>;
    
    /// Check if position is within radius of reference point
    fn within_radius(&self, position: &Position3D, center: &Position3D, radius: f64) -> Result<bool, HyperQLError>;
    
    /// Find all positions near a reference point within max_distance
    fn near_positions(&self, positions: &[Position3D], reference: &Position3D, max_distance: f64) -> Result<Vec<usize>, HyperQLError>;
}

/// Default geometric computation implementation
pub struct DefaultGeometricComputation;

impl GeometricComputation for DefaultGeometricComputation {
    fn hyperbolic_distance(&self, pos1: &Position3D, pos2: &Position3D) -> Result<f64, HyperQLError> {
        distance::hyperbolic_distance(pos1, pos2)
    }
    
    fn within_radius(&self, position: &Position3D, center: &Position3D, radius: f64) -> Result<bool, HyperQLError> {
        regions::within_radius(position, center, radius)
    }
    
    fn near_positions(&self, positions: &[Position3D], reference: &Position3D, max_distance: f64) -> Result<Vec<usize>, HyperQLError> {
        operations::find_near_positions(positions, reference, max_distance)
    }
}