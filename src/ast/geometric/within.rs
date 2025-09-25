//! WITHIN Operation AST Nodes
//!
//! This module implements AST nodes for WITHIN operations in hyperbolic space.
//! WITHIN operations enable radius-based filtering of entities based on their
//! hyperbolic distance from a reference position.
//!
//! ## Operation Semantics
//!
//! The WITHIN operation checks if entities are within a specified hyperbolic
//! radius of a reference position:
//!
//! ```hyperql
//! SELECT * FROM entities WHERE position WITHIN radius OF reference_position
//! ```
//!
//! This translates to the mathematical constraint:
//! hyperbolic_distance(position, reference_position) ≤ radius
//!
//! ## Mathematical Foundation
//!
//! WITHIN operations use hyperbolic distance in the Poincaré ball model:
//! - Distance function: d(x,y) = artanh(||x⊖y||)
//! - Möbius subtraction: x⊖y = (x-y)/(1-⟨x,y⟩) for ||x||, ||y|| < 1
//! - Efficient boundary checking and early termination

use serde::{Deserialize, Serialize};
use crate::ast::Expression;
use crate::HyperQLError;

/// AST node for WITHIN operations in hyperbolic space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithinNode {
    /// The target position expression to test
    pub target: Box<Expression>,
    /// The hyperbolic radius constraint
    pub radius: f64,
    /// The reference position for distance measurement
    pub reference: Box<Expression>,
}

/// Configuration parameters for WITHIN operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithinConfig {
    /// Precision tolerance for distance comparisons
    pub epsilon: f64,
    /// Whether to use approximate distance calculations for performance
    pub approximate: bool,
    /// Maximum number of entities to examine (for performance bounds)
    pub max_candidates: Option<u64>,
}

impl WithinNode {
    /// Create a new WITHIN operation node
    pub fn new(target: Expression, radius: f64, reference: Expression) -> Self {
        Self {
            target: Box::new(target),
            radius,
            reference: Box::new(reference),
        }
    }
    
    /// Validate the WITHIN operation parameters
    pub fn validate(&self) -> Result<(), HyperQLError> {
        if self.radius <= 0.0 {
            return Err(HyperQLError::ValidationError {
                message: "WITHIN radius must be positive".to_string(),
                field: Some("radius".to_string()),
            });
        }
        
        if self.radius.is_infinite() || self.radius.is_nan() {
            return Err(HyperQLError::ValidationError {
                message: "WITHIN radius must be a finite positive number".to_string(), 
                field: Some("radius".to_string()),
            });
        }
        
        // TODO: Validate that target and reference expressions are position-valued
        
        Ok(())
    }
    
    /// Get the estimated selectivity of this WITHIN operation
    /// Returns a value between 0.0 (very selective) and 1.0 (not selective)
    pub fn estimate_selectivity(&self) -> f64 {
        // Simple heuristic: larger radius = less selective
        // In hyperbolic space, area grows exponentially with radius
        let normalized_radius = self.radius.min(10.0) / 10.0;
        normalized_radius * normalized_radius
    }
}

impl Default for WithinConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-10,
            approximate: false,
            max_candidates: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Expression};
    
    #[test]
    fn test_within_node_creation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("target_pos".to_string()));
        let radius = 2.5;
        
        let within_node = WithinNode::new(target, radius, reference);
        
        assert_eq!(within_node.radius, 2.5);
    }
    
    #[test]
    fn test_within_validation_positive_radius() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("target_pos".to_string()));
        
        let valid_node = WithinNode::new(target.clone(), 1.0, reference.clone());
        assert!(valid_node.validate().is_ok());
        
        let invalid_node = WithinNode::new(target, -1.0, reference);
        assert!(invalid_node.validate().is_err());
    }
    
    #[test]
    fn test_within_validation_finite_radius() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("target_pos".to_string()));
        
        let infinite_node = WithinNode::new(target.clone(), f64::INFINITY, reference.clone());
        assert!(infinite_node.validate().is_err());
        
        let nan_node = WithinNode::new(target, f64::NAN, reference);
        assert!(nan_node.validate().is_err());
    }
    
    #[test]
    fn test_selectivity_estimation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let reference = Expression::Literal(Literal::String("target_pos".to_string()));
        
        let small_radius_node = WithinNode::new(target.clone(), 1.0, reference.clone());
        let large_radius_node = WithinNode::new(target, 5.0, reference);
        
        let small_selectivity = small_radius_node.estimate_selectivity();
        let large_selectivity = large_radius_node.estimate_selectivity();
        
        assert!(small_selectivity < large_selectivity);
        assert!(small_selectivity >= 0.0 && small_selectivity <= 1.0);
        assert!(large_selectivity >= 0.0 && large_selectivity <= 1.0);
    }
}