//! IN_RADIUS Operation AST Nodes
//!
//! This module implements AST nodes for IN_RADIUS operations in hyperbolic space.
//! IN_RADIUS operations enable region-based queries that test whether entities
//! fall within a specified hyperbolic ball or circular region.
//!
//! ## Operation Semantics
//!
//! The IN_RADIUS operation tests containment within a hyperbolic region:
//!
//! ```hyperql
//! SELECT * FROM entities WHERE position IN_RADIUS(center_point, radius)
//! ```
//!
//! This translates to the mathematical constraint:
//! hyperbolic_distance(position, center_point) < radius
//!
//! ## Mathematical Foundation
//!
//! IN_RADIUS operations define hyperbolic balls (open sets):
//! - Ball definition: B(center, r) = {x : d(x, center) < r}
//! - Boundary points: ∂B(center, r) = {x : d(x, center) = r}
//! - Proper containment (excludes boundary by default)
//! - Support for closed ball variants (includes boundary)

use serde::{Deserialize, Serialize};
use crate::ast::Expression;
use crate::HyperQLError;

/// AST node for IN_RADIUS operations in hyperbolic space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InRadiusNode {
    /// The target position expression to test
    pub target: Box<Expression>,
    /// The center position of the hyperbolic ball
    pub center: Box<Expression>,
    /// The radius of the hyperbolic ball
    pub radius: f64,
    /// Whether to include the boundary (closed ball vs open ball)
    pub include_boundary: bool,
}

/// Configuration parameters for IN_RADIUS operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InRadiusConfig {
    /// Precision tolerance for distance comparisons
    pub epsilon: f64,
    /// Whether to use approximate distance calculations
    pub approximate: bool,
    /// Whether to use spatial index for acceleration
    pub use_spatial_index: bool,
    /// Maximum number of candidates to examine
    pub max_candidates: Option<u64>,
}

/// Type of radius test (open vs closed ball)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RadiusTestType {
    /// Open ball: distance < radius (excludes boundary)
    OpenBall,
    /// Closed ball: distance ≤ radius (includes boundary) 
    ClosedBall,
}

impl InRadiusNode {
    /// Create a new IN_RADIUS operation node (open ball by default)
    pub fn new(target: Expression, center: Expression, radius: f64) -> Self {
        Self {
            target: Box::new(target),
            center: Box::new(center),
            radius,
            include_boundary: false,
        }
    }
    
    /// Create a new IN_RADIUS operation node with boundary control
    pub fn new_with_boundary(
        target: Expression,
        center: Expression,
        radius: f64,
        include_boundary: bool,
    ) -> Self {
        Self {
            target: Box::new(target),
            center: Box::new(center),
            radius,
            include_boundary,
        }
    }
    
    /// Validate the IN_RADIUS operation parameters
    pub fn validate(&self) -> Result<(), HyperQLError> {
        if self.radius <= 0.0 {
            return Err(HyperQLError::ValidationError {
                message: "IN_RADIUS radius must be positive".to_string(),
                field: Some("radius".to_string()),
            });
        }
        
        if self.radius.is_infinite() || self.radius.is_nan() {
            return Err(HyperQLError::ValidationError {
                message: "IN_RADIUS radius must be a finite positive number".to_string(),
                field: Some("radius".to_string()),
            });
        }
        
        // TODO: Validate that target and center expressions are position-valued
        
        Ok(())
    }
    
    /// Get the test type for this IN_RADIUS operation
    pub fn test_type(&self) -> RadiusTestType {
        if self.include_boundary {
            RadiusTestType::ClosedBall
        } else {
            RadiusTestType::OpenBall
        }
    }
    
    /// Get the estimated selectivity of this IN_RADIUS operation
    /// Returns a value between 0.0 (very selective) and 1.0 (not selective)
    pub fn estimate_selectivity(&self) -> f64 {
        // In hyperbolic space, area grows exponentially with radius
        // This is a simple heuristic for selectivity estimation
        let normalized_radius = self.radius.min(8.0) / 8.0;
        let exponential_factor = (normalized_radius * 2.0).exp() - 1.0;
        let max_exponential = (2.0_f64 * 2.0).exp() - 1.0;
        (exponential_factor / max_exponential).min(1.0)
    }
    
    /// Check if this IN_RADIUS operation would benefit from spatial indexing
    pub fn should_use_spatial_index(&self) -> bool {
        // Large radii benefit from spatial indexing
        // Very small radii might be better with linear scan
        self.radius > 1.0 && self.radius < 10.0
    }
    
    /// Convert to equivalent WITHIN operation for optimization
    pub fn to_within_operation(&self) -> crate::ast::geometric::GeometricExpression {
        use crate::ast::geometric::GeometricExpression;
        
        GeometricExpression::Within {
            target: self.target.clone(),
            radius: self.radius,
            reference: self.center.clone(),
        }
    }
}

impl Default for InRadiusConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-10,
            approximate: false,
            use_spatial_index: true,
            max_candidates: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Expression};
    
    #[test]
    fn test_in_radius_node_creation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        let radius = 4.0;
        
        let in_radius_node = InRadiusNode::new(target, center, radius);
        
        assert_eq!(in_radius_node.radius, 4.0);
        assert_eq!(in_radius_node.include_boundary, false);
        assert_eq!(in_radius_node.test_type(), RadiusTestType::OpenBall);
    }
    
    #[test]
    fn test_in_radius_node_with_boundary() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        let radius = 3.5;
        
        let closed_ball_node = InRadiusNode::new_with_boundary(target, center, radius, true);
        
        assert_eq!(closed_ball_node.radius, 3.5);
        assert_eq!(closed_ball_node.include_boundary, true);
        assert_eq!(closed_ball_node.test_type(), RadiusTestType::ClosedBall);
    }
    
    #[test]
    fn test_in_radius_validation_positive_radius() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        
        let valid_node = InRadiusNode::new(target.clone(), center.clone(), 1.0);
        assert!(valid_node.validate().is_ok());
        
        let invalid_node = InRadiusNode::new(target, center, -1.0);
        assert!(invalid_node.validate().is_err());
    }
    
    #[test]
    fn test_in_radius_validation_finite_radius() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        
        let infinite_node = InRadiusNode::new(target.clone(), center.clone(), f64::INFINITY);
        assert!(infinite_node.validate().is_err());
        
        let nan_node = InRadiusNode::new(target, center, f64::NAN);
        assert!(nan_node.validate().is_err());
    }
    
    #[test]
    fn test_selectivity_estimation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        
        let small_radius_node = InRadiusNode::new(target.clone(), center.clone(), 1.0);
        let large_radius_node = InRadiusNode::new(target, center, 6.0);
        
        let small_selectivity = small_radius_node.estimate_selectivity();
        let large_selectivity = large_radius_node.estimate_selectivity();
        
        assert!(small_selectivity < large_selectivity);
        assert!(small_selectivity >= 0.0 && small_selectivity <= 1.0);
        assert!(large_selectivity >= 0.0 && large_selectivity <= 1.0);
    }
    
    #[test]
    fn test_spatial_index_recommendation() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        
        let very_small_radius = InRadiusNode::new(target.clone(), center.clone(), 0.5);
        let optimal_radius = InRadiusNode::new(target.clone(), center.clone(), 3.0);
        let very_large_radius = InRadiusNode::new(target, center, 15.0);
        
        assert!(!very_small_radius.should_use_spatial_index());
        assert!(optimal_radius.should_use_spatial_index());
        assert!(!very_large_radius.should_use_spatial_index());
    }
    
    #[test]
    fn test_to_within_conversion() {
        let target = Expression::Literal(Literal::String("position".to_string()));
        let center = Expression::Literal(Literal::String("center_pos".to_string()));
        let radius = 2.0;
        
        let in_radius_node = InRadiusNode::new(target.clone(), center.clone(), radius);
        let within_op = in_radius_node.to_within_operation();
        
        match within_op {
            crate::ast::geometric::GeometricExpression::Within { target: within_target, radius: within_radius, reference } => {
                assert_eq!(*within_target, target);
                assert_eq!(within_radius, radius);
                assert_eq!(*reference, center);
            }
            _ => panic!("Expected Within operation"),
        }
    }
}