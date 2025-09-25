//! Geometric Operations and Utilities
//!
//! This module provides utility functions and compound operations for
//! hyperbolic geometry computations. These functions support complex
//! geometric queries that combine multiple primitive operations.
//!
//! ## Compound Operations
//!
//! These functions combine multiple geometric primitives:
//! - Finding multiple positions meeting criteria
//! - Sorting by geometric properties
//! - Complex region queries
//! - Optimization utilities

use crate::{HyperQLError, types::Position3D};
use super::{distance, regions};

/// Find all positions near a reference point within max distance
pub fn find_near_positions(
    positions: &[Position3D],
    reference: &Position3D,
    max_distance: f64,
) -> Result<Vec<usize>, HyperQLError> {
    let operation_details = format!(
        "Find near positions: {} candidates, reference=({:.3}, {:.3}, {:.3}), max_distance={:.3}",
        positions.len(), reference.x, reference.y, reference.z, max_distance
    );

    if max_distance <= 0.0 {
        return Err(HyperQLError::ValidationError {
            message: "Max distance must be positive".to_string(),
            field: Some("max_distance".to_string()),
        });
    }

    // TODO: Implement finding near positions
    // This would involve:
    // 1. Iterate through all positions
    // 2. Compute hyperbolic distance to reference for each
    // 3. Collect indices where distance <= max_distance
    // 4. Could be optimized with spatial indexing
    // 5. Could return results sorted by distance

    Err(HyperQLError::ExecutionError {
        message: format!("Find near positions not yet implemented. {}", operation_details),
        operation: "find_near_positions".to_string(),
        entity_context: Some(format!("search_{}_positions", positions.len())),
    })
}

/// Find positions sorted by distance from reference point
pub fn positions_sorted_by_distance(
    positions: &[Position3D],
    reference: &Position3D,
    limit: Option<usize>,
) -> Result<Vec<(usize, f64)>, HyperQLError> {
    let operation_details = format!(
        "Sort positions by distance: {} positions, reference=({:.3}, {:.3}, {:.3}), limit={:?}",
        positions.len(), reference.x, reference.y, reference.z, limit
    );

    // TODO: Implement sorting positions by distance
    // This would involve:
    // 1. Compute distances from reference to all positions
    // 2. Create (index, distance) pairs
    // 3. Sort by distance (ascending order)
    // 4. Apply limit if specified
    // 5. Return indices with their distances for efficiency

    Err(HyperQLError::ExecutionError {
        message: format!("Positions sorted by distance not yet implemented. {}", operation_details),
        operation: "positions_sorted_by_distance".to_string(),
        entity_context: Some(format!("sort_{}_positions", positions.len())),
    })
}

/// Find k nearest neighbors to reference point
pub fn k_nearest_neighbors(
    positions: &[Position3D],
    reference: &Position3D,
    k: usize,
) -> Result<Vec<(usize, f64)>, HyperQLError> {
    let operation_details = format!(
        "K-nearest neighbors: k={}, {} candidates, reference=({:.3}, {:.3}, {:.3})",
        k, positions.len(), reference.x, reference.y, reference.z
    );

    if k == 0 {
        return Err(HyperQLError::ValidationError {
            message: "k must be greater than 0".to_string(),
            field: Some("k".to_string()),
        });
    }

    if k > positions.len() {
        return Err(HyperQLError::ValidationError {
            message: format!("k ({}) cannot be greater than number of positions ({})", k, positions.len()),
            field: Some("k".to_string()),
        });
    }

    // TODO: Implement k-nearest neighbors
    // This would involve:
    // 1. Compute distances to all positions
    // 2. Use partial sorting or heap to find k smallest
    // 3. Return (index, distance) pairs for k nearest
    // 4. Could be optimized with spatial indexing for large k

    Err(HyperQLError::ExecutionError {
        message: format!("K-nearest neighbors not yet implemented. {}", operation_details),
        operation: "k_nearest_neighbors".to_string(),
        entity_context: Some(format!("knn_k{}_from_{}", k, positions.len())),
    })
}

/// Check if any positions in a set are within radius of center
pub fn any_within_radius(
    positions: &[Position3D],
    center: &Position3D,
    radius: f64,
) -> Result<bool, HyperQLError> {
    let operation_details = format!(
        "Any within radius: {} positions, center=({:.3}, {:.3}, {:.3}), radius={:.3}",
        positions.len(), center.x, center.y, center.z, radius
    );

    // TODO: Implement any within radius check
    // This would involve:
    // 1. Iterate through positions
    // 2. Check each against radius constraint
    // 3. Return true on first match (early termination)
    // 4. Return false if none match
    // 5. Could be optimized with spatial indexing

    Err(HyperQLError::ExecutionError {
        message: format!("Any within radius not yet implemented. {}", operation_details),
        operation: "any_within_radius".to_string(),
        entity_context: Some(format!("check_{}_positions", positions.len())),
    })
}

/// Count positions within radius of center
pub fn count_within_radius(
    positions: &[Position3D],
    center: &Position3D,
    radius: f64,
) -> Result<usize, HyperQLError> {
    let operation_details = format!(
        "Count within radius: {} positions, center=({:.3}, {:.3}, {:.3}), radius={:.3}",
        positions.len(), center.x, center.y, center.z, radius
    );

    // TODO: Implement counting positions within radius
    // This would involve:
    // 1. Iterate through all positions
    // 2. Check each against radius constraint
    // 3. Count matches
    // 4. Could be optimized with spatial indexing

    Err(HyperQLError::ExecutionError {
        message: format!("Count within radius not yet implemented. {}", operation_details),
        operation: "count_within_radius".to_string(),
        entity_context: Some(format!("count_{}_positions", positions.len())),
    })
}

/// Find the centroid (geometric center) of a set of positions
pub fn hyperbolic_centroid(positions: &[Position3D]) -> Result<Position3D, HyperQLError> {
    let operation_details = format!(
        "Hyperbolic centroid computation: {} positions",
        positions.len()
    );

    if positions.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: "Cannot compute centroid of empty position set".to_string(),
            field: Some("positions".to_string()),
        });
    }

    // TODO: Implement hyperbolic centroid computation
    // This is more complex than Euclidean centroid due to hyperbolic geometry
    // 1. Could use iterative algorithms (e.g., Weiszfeld algorithm adapted for hyperbolic space)
    // 2. Could use exponential/logarithmic maps
    // 3. Need to handle numerical stability in hyperbolic space
    // 4. Result should minimize sum of hyperbolic distances to all points

    Err(HyperQLError::ExecutionError {
        message: format!("Hyperbolic centroid not yet implemented. {}", operation_details),
        operation: "hyperbolic_centroid".to_string(),
        entity_context: Some(format!("centroid_{}_positions", positions.len())),
    })
}

/// Compute bounding hyperbolic ball for a set of positions
pub fn bounding_hyperbolic_ball(positions: &[Position3D]) -> Result<(Position3D, f64), HyperQLError> {
    let operation_details = format!(
        "Bounding hyperbolic ball: {} positions",
        positions.len()
    );

    if positions.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: "Cannot compute bounding ball of empty position set".to_string(),
            field: Some("positions".to_string()),
        });
    }

    // TODO: Implement bounding hyperbolic ball computation
    // This would involve:
    // 1. Find center that minimizes maximum distance to any point
    // 2. This is the hyperbolic version of smallest enclosing circle problem
    // 3. Could use iterative algorithms or approximate solutions
    // 4. Return (center, radius) pair

    Err(HyperQLError::ExecutionError {
        message: format!("Bounding hyperbolic ball not yet implemented. {}", operation_details),
        operation: "bounding_hyperbolic_ball".to_string(),
        entity_context: Some(format!("bounding_{}_positions", positions.len())),
    })
}

/// Validate that a position is within the Poincaré ball
pub fn validate_position(position: &Position3D) -> Result<(), HyperQLError> {
    let norm_sq = position.x * position.x + position.y * position.y + position.z * position.z;
    
    if norm_sq >= 1.0 {
        return Err(HyperQLError::GeometricError {
            operation: "position_validation".to_string(),
            reason: "Position is outside Poincaré ball (norm >= 1.0)".to_string(),
            positions: vec![format!("pos=({:.6}, {:.6}, {:.6}), norm²={:.9}", position.x, position.y, position.z, norm_sq)],
        });
    }
    
    Ok(())
}

/// Batch validate multiple positions
pub fn batch_validate_positions(positions: &[Position3D]) -> Result<(), HyperQLError> {
    for (index, position) in positions.iter().enumerate() {
        validate_position(position).map_err(|mut error| {
            // Add context about which position failed validation
            if let HyperQLError::GeometricError { ref mut positions, .. } = error {
                positions[0] = format!("position[{}]: {}", index, positions[0]);
            }
            error
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_near_positions() {
        let positions = vec![
            Position3D { x: 0.1, y: 0.0, z: 0.0 },
            Position3D { x: 0.0, y: 0.2, z: 0.0 },
            Position3D { x: 0.0, y: 0.0, z: 0.3 },
        ];
        let reference = Position3D { x: 0.0, y: 0.0, z: 0.0 };
        let max_distance = 0.25;

        let result = find_near_positions(&positions, &reference, max_distance);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Find near positions"));
        assert!(error_msg.contains("3 candidates"));

        // Test validation
        let result_invalid = find_near_positions(&positions, &reference, -1.0);
        assert!(result_invalid.is_err());
        let error_msg_invalid = result_invalid.unwrap_err().to_string();
        assert!(error_msg_invalid.contains("Max distance must be positive"));
    }

    #[test]
    fn test_positions_sorted_by_distance() {
        let positions = vec![
            Position3D { x: 0.3, y: 0.0, z: 0.0 },
            Position3D { x: 0.1, y: 0.0, z: 0.0 },
            Position3D { x: 0.5, y: 0.0, z: 0.0 },
        ];
        let reference = Position3D { x: 0.0, y: 0.0, z: 0.0 };

        let result = positions_sorted_by_distance(&positions, &reference, Some(2));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Sort positions by distance"));
        assert!(error_msg.contains("limit=Some(2)"));
    }

    #[test]
    fn test_k_nearest_neighbors() {
        let positions = vec![
            Position3D { x: 0.1, y: 0.0, z: 0.0 },
            Position3D { x: 0.0, y: 0.2, z: 0.0 },
            Position3D { x: 0.0, y: 0.0, z: 0.3 },
        ];
        let reference = Position3D { x: 0.0, y: 0.0, z: 0.0 };

        // Valid k
        let result = k_nearest_neighbors(&positions, &reference, 2);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("K-nearest neighbors"));
        assert!(error_msg.contains("k=2"));

        // Invalid k (zero)
        let result_zero = k_nearest_neighbors(&positions, &reference, 0);
        assert!(result_zero.is_err());
        let error_msg_zero = result_zero.unwrap_err().to_string();
        assert!(error_msg_zero.contains("k must be greater than 0"));

        // Invalid k (too large)
        let result_large = k_nearest_neighbors(&positions, &reference, 5);
        assert!(result_large.is_err());
        let error_msg_large = result_large.unwrap_err().to_string();
        assert!(error_msg_large.contains("cannot be greater than number of positions"));
    }

    #[test]
    fn test_counting_operations() {
        let positions = vec![
            Position3D { x: 0.1, y: 0.0, z: 0.0 },
            Position3D { x: 0.0, y: 0.2, z: 0.0 },
        ];
        let center = Position3D { x: 0.0, y: 0.0, z: 0.0 };
        let radius = 0.15;

        // Any within radius
        let result_any = any_within_radius(&positions, &center, radius);
        assert!(result_any.is_err());
        let error_msg_any = result_any.unwrap_err().to_string();
        assert!(error_msg_any.contains("Any within radius"));

        // Count within radius
        let result_count = count_within_radius(&positions, &center, radius);
        assert!(result_count.is_err());
        let error_msg_count = result_count.unwrap_err().to_string();
        assert!(error_msg_count.contains("Count within radius"));
    }

    #[test]
    fn test_geometric_computations() {
        let positions = vec![
            Position3D { x: 0.1, y: 0.1, z: 0.1 },
            Position3D { x: 0.2, y: 0.2, z: 0.2 },
        ];

        // Hyperbolic centroid
        let result_centroid = hyperbolic_centroid(&positions);
        assert!(result_centroid.is_err());
        let error_msg_centroid = result_centroid.unwrap_err().to_string();
        assert!(error_msg_centroid.contains("Hyperbolic centroid"));

        // Bounding ball
        let result_bounding = bounding_hyperbolic_ball(&positions);
        assert!(result_bounding.is_err());
        let error_msg_bounding = result_bounding.unwrap_err().to_string();
        assert!(error_msg_bounding.contains("Bounding hyperbolic ball"));

        // Empty set validation
        let empty_positions: Vec<Position3D> = vec![];
        let result_empty_centroid = hyperbolic_centroid(&empty_positions);
        assert!(result_empty_centroid.is_err());
        let error_msg_empty = result_empty_centroid.unwrap_err().to_string();
        assert!(error_msg_empty.contains("Cannot compute centroid of empty"));
    }

    #[test]
    fn test_position_validation() {
        // Valid position
        let valid_pos = Position3D { x: 0.5, y: 0.3, z: 0.2 };
        assert!(validate_position(&valid_pos).is_ok());

        // Invalid position (on boundary)
        let boundary_pos = Position3D { x: 1.0, y: 0.0, z: 0.0 };
        let result_boundary = validate_position(&boundary_pos);
        assert!(result_boundary.is_err());
        let error_msg_boundary = result_boundary.unwrap_err().to_string();
        assert!(error_msg_boundary.contains("outside Poincaré ball"));

        // Invalid position (outside)
        let outside_pos = Position3D { x: 1.5, y: 0.0, z: 0.0 };
        assert!(validate_position(&outside_pos).is_err());
    }

    #[test]
    fn test_batch_validation() {
        let mixed_positions = vec![
            Position3D { x: 0.1, y: 0.2, z: 0.3 }, // Valid
            Position3D { x: 1.1, y: 0.0, z: 0.0 }, // Invalid
            Position3D { x: 0.4, y: 0.5, z: 0.6 }, // Valid
        ];

        let result = batch_validate_positions(&mixed_positions);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // The error should mention the invalid position (position[1]) in the context
        assert!(error_msg.contains("position[1]:") || error_msg.contains("outside Poincaré ball"));
    }
}