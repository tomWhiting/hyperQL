//! NEAR Operation AST Nodes
//!
//! This module implements AST nodes for NEAR operations in hyperbolic space.
//! NEAR operations enable proximity queries that find entities close to a
//! reference position with configurable distance constraints.
//!
//! ## Operation Semantics
//!
//! The NEAR operation finds entities within a specified maximum distance
//! of a reference position, with optional ordering by distance:
//!
//! ```hyperql
//! SELECT * FROM entities WHERE position NEAR reference_point DISTANCE max_distance
//! ```
//!
//! This translates to the mathematical constraint:
//! hyperbolic_distance(position, reference_point) ≤ max_distance
//!
//! ## Mathematical Foundation
//!
//! NEAR operations use hyperbolic distance with proximity semantics:
//! - Distance function: d(x,y) = artanh(||x⊖y||) in Poincaré ball model
//! - Natural ordering by increasing hyperbolic distance
//! - Support for approximate nearest neighbor search
//! - Integration with spatial indices for efficient query processing

use serde::{Deserialize, Serialize};
use crate::ast::Expression;
use crate::HyperQLError;

/// AST node for NEAR operations in hyperbolic space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NearNode {
    /// The target position expression to test
    pub target: Box<Expression>,
    /// The reference position for proximity measurement
    pub reference: Box<Expression>,
    /// The maximum hyperbolic distance constraint
    pub max_distance: f64,
    /// Whether to order results by distance (nearest first)
    pub order_by_distance: bool,
}

/// Configuration parameters for NEAR operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NearConfig {
    /// Precision tolerance for distance comparisons
    pub epsilon: f64,
    /// Whether to use approximate distance calculations
    pub approximate: bool,
    /// Maximum number of candidates to consider
    pub max_candidates: Option<u64>,
    /// Whether to use spatial index acceleration
    pub use_spatial_index: bool,
}

impl NearNode {
    /// Create a new NEAR operation node
    pub fn new(
        target: Expression,
        reference: Expression,
        max_distance: f64,
    ) -> Self {
        Self {
            target: Box::new(target),
            reference: Box::new(reference),
            max_distance,
            order_by_distance: true, // Default to ordering by distance
        }
    }
    
    /// Create a new NEAR operation node with distance ordering control
    pub fn new_with_ordering(
        target: Expression,
        reference: Expression,
        max_distance: f64,
        order_by_distance: bool,
    ) -> Self {
        Self {
            target: Box::new(target),
            reference: Box::new(reference),
            max_distance,
            order_by_distance,
        }
    }
    
    /// Validate the NEAR operation parameters
    pub fn validate(&self) -> Result<(), HyperQLError> {
        if self.max_distance <= 0.0 {
            return Err(HyperQLError::ValidationError {
                message: "NEAR max_distance must be positive".to_string(),
                field: Some("max_distance".to_string()),
            });
        }
        
        if self.max_distance.is_infinite() || self.max_distance.is_nan() {
            return Err(HyperQLError::ValidationError {
                message: "NEAR max_distance must be a finite positive number".to_string(),
                field: Some("max_distance".to_string()),
            });
        }
        
        // TODO: Validate that target and reference expressions are position-valued
        
        Ok(())
    }
    
    /// Get the estimated cost of this NEAR operation
    /// Higher values indicate more expensive operations
    pub fn estimate_cost(&self) -> f64 {
        let base_cost = 1.0;
        let distance_factor = if self.max_distance > 5.0 { 2.0 } else { 1.0 };
        let ordering_factor = if self.order_by_distance { 1.5 } else { 1.0 };
        
        base_cost * distance_factor * ordering_factor
    }
    
    /// Check if this NEAR operation would benefit from spatial indexing
    pub fn should_use_spatial_index(&self) -> bool {
        // Large distances or operations that need ordering benefit from indexing
        self.max_distance > 3.0 || self.order_by_distance
    }
}

impl Default for NearConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-10,
            approximate: false,
            max_candidates: None,
            use_spatial_index: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Expression};
    
    #[test]
    fn test_near_node_creation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("ref_pos".to_string()));
        let max_distance = 3.0;
        
        let near_node = NearNode::new(target, reference, max_distance);
        
        assert_eq!(near_node.max_distance, 3.0);
        assert_eq!(near_node.order_by_distance, true);
    }
    
    #[test]
    fn test_near_node_with_ordering() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("ref_pos".to_string()));
        let max_distance = 2.5;
        
        let near_node = NearNode::new_with_ordering(target, reference, max_distance, false);
        
        assert_eq!(near_node.max_distance, 2.5);
        assert_eq!(near_node.order_by_distance, false);
    }
    
    #[test]
    fn test_near_validation_positive_distance() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("ref_pos".to_string()));
        
        let valid_node = NearNode::new(target.clone(), reference.clone(), 1.0);
        assert!(valid_node.validate().is_ok());
        
        let invalid_node = NearNode::new(target, reference, -1.0);
        assert!(invalid_node.validate().is_err());
    }
    
    #[test]
    fn test_near_validation_finite_distance() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("ref_pos".to_string()));
        
        let infinite_node = NearNode::new(target.clone(), reference.clone(), f64::INFINITY);
        assert!(infinite_node.validate().is_err());
        
        let nan_node = NearNode::new(target, reference, f64::NAN);
        assert!(nan_node.validate().is_err());
    }
    
    #[test]
    fn test_cost_estimation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("ref_pos".to_string()));
        
        let small_distance_node = NearNode::new_with_ordering(
            target.clone(), reference.clone(), 1.0, false
        );
        let large_distance_ordered_node = NearNode::new_with_ordering(
            target, reference, 10.0, true
        );
        
        let small_cost = small_distance_node.estimate_cost();
        let large_cost = large_distance_ordered_node.estimate_cost();
        
        assert!(small_cost < large_cost);
    }
    
    #[test]
    fn test_spatial_index_recommendation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("ref_pos".to_string()));
        
        let small_unordered = NearNode::new_with_ordering(
            target.clone(), reference.clone(), 1.0, false
        );
        let large_unordered = NearNode::new_with_ordering(
            target.clone(), reference.clone(), 5.0, false
        );
        let small_ordered = NearNode::new_with_ordering(
            target, reference, 1.0, true
        );
        
        assert!(!small_unordered.should_use_spatial_index());
        assert!(large_unordered.should_use_spatial_index());
        assert!(small_ordered.should_use_spatial_index());
    }
}