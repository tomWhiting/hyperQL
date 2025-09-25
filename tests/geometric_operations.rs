//! Integration Tests for Geometric Operations
//!
//! This module tests the geometric operations (WITHIN, NEAR, IN_RADIUS)
//! that work with hyperbolic space positions. These are separate from
//! vector operations and work with the hyperbolic positioning system.

use hyperQL::*;
use hyperQL::ast::geometric::*;
use hyperQL::ast::Expression;
use hyperQL::types::{Position3D, Value};
use std::collections::HashMap;

#[test]
fn test_geometric_ast_creation() {
    // Test creating geometric AST nodes
    let target = Expression::Column(ast::ColumnRef {
        table: Some("entities".to_string()),
        name: "position".to_string(),
    });
    
    let center = Expression::Literal(ast::Literal::String("center_pos".to_string()));
    
    // Test WITHIN operation
    let within_expr = GeometricExpression::Within {
        target: Box::new(target.clone()),
        radius: 2.5,
        reference: Box::new(center.clone()),
    };
    
    match within_expr {
        GeometricExpression::Within { radius, .. } => {
            assert_eq!(radius, 2.5);
        }
        _ => panic!("Expected Within expression"),
    }
    
    // Test NEAR operation
    let near_expr = GeometricExpression::Near {
        target: Box::new(target.clone()),
        reference: Box::new(center.clone()),
        max_distance: 3.0,
    };
    
    match near_expr {
        GeometricExpression::Near { max_distance, .. } => {
            assert_eq!(max_distance, 3.0);
        }
        _ => panic!("Expected Near expression"),
    }
    
    // Test IN_RADIUS operation
    let in_radius_expr = GeometricExpression::InRadius {
        target: Box::new(target),
        center: Box::new(center),
        radius: 4.0,
    };
    
    match in_radius_expr {
        GeometricExpression::InRadius { radius, .. } => {
            assert_eq!(radius, 4.0);
        }
        _ => panic!("Expected InRadius expression"),
    }
}

#[test]
fn test_within_node_validation() {
    use hyperQL::ast::geometric::within::*;
    
    let target = Expression::Literal(ast::Literal::String("position".to_string()));
    let reference = Expression::Literal(ast::Literal::String("center".to_string()));
    
    // Valid WITHIN node
    let valid_node = WithinNode::new(target.clone(), 2.5, reference.clone());
    assert!(valid_node.validate().is_ok());
    
    // Invalid WITHIN node (negative radius)
    let invalid_node = WithinNode::new(target, -1.0, reference);
    assert!(invalid_node.validate().is_err());
    let error = invalid_node.validate().unwrap_err();
    assert!(error.to_string().contains("radius must be positive"));
}

#[test]
fn test_near_node_validation() {
    use hyperQL::ast::geometric::near::*;
    
    let target = Expression::Literal(ast::Literal::String("position".to_string()));
    let reference = Expression::Literal(ast::Literal::String("ref_pos".to_string()));
    
    // Valid NEAR node
    let valid_node = NearNode::new(target.clone(), reference.clone(), 3.0);
    assert!(valid_node.validate().is_ok());
    
    // Test ordering recommendation
    let ordered_node = NearNode::new_with_ordering(target.clone(), reference.clone(), 1.0, true);
    assert!(ordered_node.should_use_spatial_index());
    
    let unordered_small_node = NearNode::new_with_ordering(target, reference, 1.0, false);
    assert!(!unordered_small_node.should_use_spatial_index());
}

#[test]
fn test_in_radius_node_features() {
    use hyperQL::ast::geometric::radius::*;
    
    let target = Expression::Literal(ast::Literal::String("position".to_string()));
    let center = Expression::Literal(ast::Literal::String("center_pos".to_string()));
    
    // Test open ball (default)
    let open_ball = InRadiusNode::new(target.clone(), center.clone(), 2.0);
    assert_eq!(open_ball.test_type(), RadiusTestType::OpenBall);
    assert!(!open_ball.include_boundary);
    
    // Test closed ball
    let closed_ball = InRadiusNode::new_with_boundary(target, center, 2.0, true);
    assert_eq!(closed_ball.test_type(), RadiusTestType::ClosedBall);
    assert!(closed_ball.include_boundary);
    
    // Test selectivity estimation
    let selectivity = closed_ball.estimate_selectivity();
    assert!(selectivity >= 0.0 && selectivity <= 1.0);
}

#[test]
fn test_geometric_functions_integration() {
    use hyperQL::functions::geometric::distance::*;
    use hyperQL::functions::geometric::regions::*;
    
    let pos1 = Position3D { x: 0.1, y: 0.2, z: 0.3 };
    let pos2 = Position3D { x: 0.4, y: 0.5, z: 0.6 };
    
    // Test hyperbolic distance function works correctly
    let distance_result = hyperbolic_distance(&pos1, &pos2);
    assert!(distance_result.is_ok());
    let distance = distance_result.unwrap();
    assert!(distance > 0.0);
    assert!(distance.is_finite());
    
    // Test within radius function
    let within_result = within_radius(&pos1, &pos2, 1.0);
    assert!(within_result.is_err());
    let within_error = within_result.unwrap_err().to_string();
    assert!(within_error.contains("Within radius check"));
    assert!(within_error.contains("radius=1.000"));
}

#[test]
fn test_position_validation() {
    use hyperQL::functions::geometric::operations::*;
    
    // Valid position within Poincaré ball
    let valid_pos = Position3D { x: 0.5, y: 0.3, z: 0.2 };
    assert!(validate_position(&valid_pos).is_ok());
    
    // Invalid position outside Poincaré ball
    let invalid_pos = Position3D { x: 1.5, y: 0.0, z: 0.0 };
    let result = validate_position(&invalid_pos);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("outside Poincaré ball"));
    
    // Position exactly on boundary (should be invalid)
    let boundary_pos = Position3D { x: 1.0, y: 0.0, z: 0.0 };
    let boundary_result = validate_position(&boundary_pos);
    assert!(boundary_result.is_err());
}

#[test]
fn test_geometric_batch_operations() {
    use hyperQL::functions::geometric::operations::*;
    
    let positions = vec![
        Position3D { x: 0.1, y: 0.0, z: 0.0 },
        Position3D { x: 0.0, y: 0.2, z: 0.0 },
        Position3D { x: 0.0, y: 0.0, z: 0.3 },
    ];
    let reference = Position3D { x: 0.0, y: 0.0, z: 0.0 };
    
    // Test k-nearest neighbors
    let knn_result = k_nearest_neighbors(&positions, &reference, 2);
    assert!(knn_result.is_ok());
    let knn_neighbors = knn_result.unwrap();
    assert_eq!(knn_neighbors.len(), 2);
    // Should return the 2 nearest neighbors with their distances
    assert!(knn_neighbors[0].1 <= knn_neighbors[1].1); // Sorted by distance

    // Test finding near positions
    let near_result = find_near_positions(&positions, &reference, 0.25);
    assert!(near_result.is_ok());
    let near_indices = near_result.unwrap();
    // Should find positions within distance 0.25 from origin
    assert!(near_indices.len() <= 3); // At most all 3 positions
    assert!(near_indices.len() >= 1); // At least some positions should be within 0.25
}

#[test]
fn test_geometric_expression_in_main_ast() {
    // Test that geometric expressions integrate with the main Expression enum
    let within_geom = GeometricExpression::Within {
        target: Box::new(Expression::Column(ast::ColumnRef {
            table: None,
            name: "position".to_string(),
        })),
        radius: 1.5,
        reference: Box::new(Expression::Literal(ast::Literal::String("center".to_string()))),
    };
    
    let geom_expr = Expression::Geometric(within_geom);
    
    match geom_expr {
        Expression::Geometric(GeometricExpression::Within { radius, .. }) => {
            assert_eq!(radius, 1.5);
        }
        _ => panic!("Expected geometric WITHIN expression"),
    }
}

#[test]
fn test_geometric_cost_estimation() {
    use hyperQL::ast::geometric::within::WithinNode;
    use hyperQL::ast::geometric::near::NearNode;
    
    let target = Expression::Literal(ast::Literal::String("position".to_string()));
    let reference = Expression::Literal(ast::Literal::String("center".to_string()));
    
    // Test selectivity estimation for different radii
    let small_within = WithinNode::new(target.clone(), 0.5, reference.clone());
    let large_within = WithinNode::new(target.clone(), 5.0, reference.clone());
    
    let small_selectivity = small_within.estimate_selectivity();
    let large_selectivity = large_within.estimate_selectivity();
    
    assert!(small_selectivity < large_selectivity);
    assert!(small_selectivity >= 0.0 && small_selectivity <= 1.0);
    assert!(large_selectivity >= 0.0 && large_selectivity <= 1.0);
    
    // Test cost estimation for NEAR operations
    let simple_near = NearNode::new(target.clone(), reference.clone(), 1.0);
    let complex_near = NearNode::new_with_ordering(target, reference, 10.0, true);
    
    let simple_cost = simple_near.estimate_cost();
    let complex_cost = complex_near.estimate_cost();
    
    assert!(simple_cost < complex_cost);
    assert!(simple_cost > 0.0);
    assert!(complex_cost > 0.0);
}