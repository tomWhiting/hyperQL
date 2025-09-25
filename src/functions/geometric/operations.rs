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

    // Validate inputs
    batch_validate_positions(positions)?;
    validate_position(reference)?;

    let mut near_positions = Vec::new();

    // Iterate through all positions and check distances
    for (index, position) in positions.iter().enumerate() {
        match distance::hyperbolic_distance(reference, position) {
            Ok(dist) => {
                if dist <= max_distance {
                    near_positions.push(index);
                }
            }
            Err(e) => {
                return Err(HyperQLError::ExecutionError {
                    message: format!("Failed to compute distance for position[{}]: {}", index, e),
                    operation: "find_near_positions".to_string(),
                    entity_context: Some(format!("position_{}", index)),
                });
            }
        }
    }

    Ok(near_positions)
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

    // Validate inputs
    batch_validate_positions(positions)?;
    validate_position(reference)?;

    if positions.is_empty() {
        return Ok(Vec::new());
    }

    // Compute distances to all positions using batch operation
    let distances = distance::batch_hyperbolic_distances(reference, positions)?;

    // Create (index, distance) pairs
    let mut index_distance_pairs: Vec<(usize, f64)> = distances
        .into_iter()
        .enumerate()
        .collect();

    // Sort by distance (ascending order)
    index_distance_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Apply limit if specified
    if let Some(limit_count) = limit {
        if limit_count > 0 {
            index_distance_pairs.truncate(limit_count);
        }
    }

    Ok(index_distance_pairs)
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

    // Use positions_sorted_by_distance with limit = k
    let sorted_positions = positions_sorted_by_distance(positions, reference, Some(k))?;

    Ok(sorted_positions)
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

    // Validate inputs
    batch_validate_positions(positions)?;
    validate_position(center)?;

    if radius <= 0.0 {
        return Err(HyperQLError::ValidationError {
            message: "Radius must be positive".to_string(),
            field: Some("radius".to_string()),
        });
    }

    // Iterate through positions and check each against radius constraint
    // Return true on first match (early termination for efficiency)
    for position in positions {
        match distance::hyperbolic_distance(center, position) {
            Ok(dist) => {
                if dist <= radius {
                    return Ok(true);
                }
            }
            Err(e) => {
                return Err(HyperQLError::ExecutionError {
                    message: format!("Failed to compute distance: {}", e),
                    operation: "any_within_radius".to_string(),
                    entity_context: Some("distance_computation".to_string()),
                });
            }
        }
    }

    // No positions found within radius
    Ok(false)
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

    // Validate inputs
    batch_validate_positions(positions)?;
    validate_position(center)?;

    if radius <= 0.0 {
        return Err(HyperQLError::ValidationError {
            message: "Radius must be positive".to_string(),
            field: Some("radius".to_string()),
        });
    }

    let mut count = 0;

    // Iterate through all positions and count matches
    for position in positions {
        match distance::hyperbolic_distance(center, position) {
            Ok(dist) => {
                if dist <= radius {
                    count += 1;
                }
            }
            Err(e) => {
                return Err(HyperQLError::ExecutionError {
                    message: format!("Failed to compute distance: {}", e),
                    operation: "count_within_radius".to_string(),
                    entity_context: Some("distance_computation".to_string()),
                });
            }
        }
    }

    Ok(count)
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

    // Validate all positions
    batch_validate_positions(positions)?;

    // For single position, return it as centroid
    if positions.len() == 1 {
        return Ok(positions[0].clone());
    }

    // For hyperbolic centroid, we use an iterative algorithm
    // Start with the Euclidean centroid as initial guess
    let mut centroid = Position3D {
        x: positions.iter().map(|p| p.x).sum::<f64>() / positions.len() as f64,
        y: positions.iter().map(|p| p.y).sum::<f64>() / positions.len() as f64,
        z: positions.iter().map(|p| p.z).sum::<f64>() / positions.len() as f64,
    };

    // Ensure initial guess is inside Poincare ball
    let initial_norm_sq = centroid.x * centroid.x + centroid.y * centroid.y + centroid.z * centroid.z;
    if initial_norm_sq >= 1.0 {
        // Scale down to be inside the ball
        let scale = 0.5 / initial_norm_sq.sqrt();
        centroid.x *= scale;
        centroid.y *= scale;
        centroid.z *= scale;
    }

    // Iteratively improve the centroid using a simplified Weiszfeld-like algorithm
    // adapted for hyperbolic space
    const MAX_ITERATIONS: usize = 50;
    const CONVERGENCE_THRESHOLD: f64 = 1e-10;

    for _iteration in 0..MAX_ITERATIONS {
        let mut weighted_sum_x = 0.0;
        let mut weighted_sum_y = 0.0;
        let mut weighted_sum_z = 0.0;
        let mut total_weight = 0.0;

        for position in positions {
            // Compute distance from current centroid to this position
            let dist = distance::hyperbolic_distance(&centroid, position)
                .map_err(|e| HyperQLError::ExecutionError {
                    message: format!("Failed to compute distance during centroid iteration: {}", e),
                    operation: "hyperbolic_centroid".to_string(),
                    entity_context: Some("iterative_computation".to_string()),
                })?;

            // Use inverse distance weighting (avoiding division by zero)
            let weight = if dist < 1e-12 {
                1e12 // Very large weight for very close points
            } else {
                1.0 / dist
            };

            weighted_sum_x += weight * position.x;
            weighted_sum_y += weight * position.y;
            weighted_sum_z += weight * position.z;
            total_weight += weight;
        }

        // Compute new centroid estimate
        let new_centroid = Position3D {
            x: weighted_sum_x / total_weight,
            y: weighted_sum_y / total_weight,
            z: weighted_sum_z / total_weight,
        };

        // Ensure new centroid is inside Poincare ball
        let new_norm_sq = new_centroid.x * new_centroid.x + new_centroid.y * new_centroid.y + new_centroid.z * new_centroid.z;
        let final_new_centroid = if new_norm_sq >= 1.0 {
            let scale = 0.99 / new_norm_sq.sqrt();
            Position3D {
                x: new_centroid.x * scale,
                y: new_centroid.y * scale,
                z: new_centroid.z * scale,
            }
        } else {
            new_centroid
        };

        // Check for convergence
        let change = ((final_new_centroid.x - centroid.x).powi(2) +
                     (final_new_centroid.y - centroid.y).powi(2) +
                     (final_new_centroid.z - centroid.z).powi(2)).sqrt();

        centroid = final_new_centroid;

        if change < CONVERGENCE_THRESHOLD {
            break;
        }
    }

    Ok(centroid)
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

    // Validate all positions
    batch_validate_positions(positions)?;

    // For single position, return it as center with radius 0
    if positions.len() == 1 {
        return Ok((positions[0].clone(), 0.0));
    }

    // Start with the hyperbolic centroid as the center
    let mut center = hyperbolic_centroid(positions)?;

    // Iteratively improve the center to minimize maximum distance
    const MAX_ITERATIONS: usize = 20;
    const IMPROVEMENT_THRESHOLD: f64 = 1e-8;

    for _iteration in 0..MAX_ITERATIONS {
        // Compute current radius (maximum distance from center to any point)
        let mut max_distance = 0.0;
        let mut farthest_point_index = 0;

        for (i, position) in positions.iter().enumerate() {
            let dist = distance::hyperbolic_distance(&center, position)
                .map_err(|e| HyperQLError::ExecutionError {
                    message: format!("Failed to compute distance during bounding ball iteration: {}", e),
                    operation: "bounding_hyperbolic_ball".to_string(),
                    entity_context: Some("iterative_computation".to_string()),
                })?;

            if dist > max_distance {
                max_distance = dist;
                farthest_point_index = i;
            }
        }

        // Try to improve the center by moving it slightly toward the farthest point
        let farthest_point = &positions[farthest_point_index];

        // Compute direction from center to farthest point
        let direction_x = farthest_point.x - center.x;
        let direction_y = farthest_point.y - center.y;
        let direction_z = farthest_point.z - center.z;

        let direction_length = (direction_x * direction_x + direction_y * direction_y + direction_z * direction_z).sqrt();

        if direction_length < 1e-12 {
            break; // Center is already at the farthest point
        }

        // Normalize direction
        let unit_dir_x = direction_x / direction_length;
        let unit_dir_y = direction_y / direction_length;
        let unit_dir_z = direction_z / direction_length;

        // Try moving center a small step in that direction
        let step_size = 0.01;
        let new_center = Position3D {
            x: center.x + step_size * unit_dir_x,
            y: center.y + step_size * unit_dir_y,
            z: center.z + step_size * unit_dir_z,
        };

        // Ensure new center is inside Poincare ball
        let new_center_norm_sq = new_center.x * new_center.x + new_center.y * new_center.y + new_center.z * new_center.z;
        let final_new_center = if new_center_norm_sq >= 1.0 {
            // Keep the old center if the new one would be outside the ball
            center.clone()
        } else {
            new_center
        };

        // Check if this improves the maximum distance
        let mut new_max_distance = 0.0;
        let mut improvement_found = true;

        for position in positions {
            match distance::hyperbolic_distance(&final_new_center, position) {
                Ok(dist) => {
                    if dist > new_max_distance {
                        new_max_distance = dist;
                    }
                }
                Err(_) => {
                    improvement_found = false;
                    break;
                }
            }
        }

        // If the new center improves the max distance, use it
        if improvement_found && new_max_distance < max_distance - IMPROVEMENT_THRESHOLD {
            center = final_new_center;
        } else {
            // No significant improvement, stop iterating
            break;
        }
    }

    // Compute final radius
    let mut final_radius = 0.0;
    for position in positions {
        let dist = distance::hyperbolic_distance(&center, position)?;
        if dist > final_radius {
            final_radius = dist;
        }
    }

    Ok((center, final_radius))
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
        assert!(result.is_ok());
        let near_indices = result.unwrap();
        // The first two positions should be within distance 0.25 from origin
        // pos[0]: (0.1, 0.0, 0.0) - distance ≈ 0.1
        // pos[1]: (0.0, 0.2, 0.0) - distance ≈ 0.2
        // pos[2]: (0.0, 0.0, 0.3) - distance ≈ 0.3 (should be excluded)
        assert_eq!(near_indices.len(), 2);
        assert!(near_indices.contains(&0));
        assert!(near_indices.contains(&1));
        assert!(!near_indices.contains(&2));

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
        assert!(result.is_ok());
        let sorted_positions = result.unwrap();
        // Should return 2 closest positions
        assert_eq!(sorted_positions.len(), 2);
        // Position 1 (index 1) at (0.1, 0.0, 0.0) should be closest
        assert_eq!(sorted_positions[0].0, 1);
        // Position 0 (index 0) at (0.3, 0.0, 0.0) should be second
        assert_eq!(sorted_positions[1].0, 0);
        // Distances should be in ascending order
        assert!(sorted_positions[0].1 <= sorted_positions[1].1);
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
        assert!(result.is_ok());
        let knn_result = result.unwrap();
        assert_eq!(knn_result.len(), 2);
        // Should return 2 nearest neighbors in distance order
        assert!(knn_result[0].1 <= knn_result[1].1);

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
        assert!(result_any.is_ok());
        let has_any = result_any.unwrap();
        // Position (0.1, 0.0, 0.0) has distance ≈ 0.1, which is < 0.15
        assert!(has_any);

        // Count within radius
        let result_count = count_within_radius(&positions, &center, radius);
        assert!(result_count.is_ok());
        let count = result_count.unwrap();
        // Position (0.1, 0.0, 0.0) has distance ≈ 0.1 < 0.15 (included)
        // Position (0.0, 0.2, 0.0) has distance ≈ 0.2 > 0.15 (excluded)
        assert_eq!(count, 1);
    }

    #[test]
    fn test_geometric_computations() {
        let positions = vec![
            Position3D { x: 0.1, y: 0.1, z: 0.1 },
            Position3D { x: 0.2, y: 0.2, z: 0.2 },
        ];

        // Hyperbolic centroid
        let result_centroid = hyperbolic_centroid(&positions);
        assert!(result_centroid.is_ok());
        let centroid = result_centroid.unwrap();
        // Centroid should be inside Poincaré ball
        let centroid_norm_sq = centroid.x * centroid.x + centroid.y * centroid.y + centroid.z * centroid.z;
        assert!(centroid_norm_sq < 1.0);
        // Should be somewhere between the two points
        assert!(centroid.x > 0.0 && centroid.x < 0.3);
        assert!(centroid.y > 0.0 && centroid.y < 0.3);
        assert!(centroid.z > 0.0 && centroid.z < 0.3);

        // Bounding ball
        let result_bounding = bounding_hyperbolic_ball(&positions);
        assert!(result_bounding.is_ok());
        let (center, radius) = result_bounding.unwrap();
        // Center should be inside Poincaré ball
        let center_norm_sq = center.x * center.x + center.y * center.y + center.z * center.z;
        assert!(center_norm_sq < 1.0);
        // Radius should be positive
        assert!(radius > 0.0);

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