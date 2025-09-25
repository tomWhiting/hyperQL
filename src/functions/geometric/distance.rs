//! Hyperbolic Distance Functions
//!
//! This module implements distance computation functions for hyperbolic space.
//! These functions provide the core geometric operations for WITHIN, NEAR,
//! and IN_RADIUS queries in HyperQL.
//!
//! ## Mathematical Foundation
//!
//! All distance functions use the Poincaré ball model of hyperbolic geometry:
//! - Points are represented as 3D coordinates with ||p|| < 1
//! - Distance: d(x,y) = artanh(||x⊖y||) where ⊖ is Möbius subtraction
//! - Möbius subtraction: x⊖y = (x-y)/(1-⟨x,y⟩)
//!
//! ## Performance Considerations
//!
//! All implementations return "not yet implemented" with operation details to
//! facilitate integration with the actual hyperbolic geometry engine while
//! maintaining clean interfaces for future real implementations.

use crate::{HyperQLError, types::Position3D};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compute hyperbolic distance between two positions in Poincaré ball model
pub fn hyperbolic_distance(pos1: &Position3D, pos2: &Position3D) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Hyperbolic distance computation: pos1=({:.3}, {:.3}, {:.3}) to pos2=({:.3}, {:.3}, {:.3})",
        pos1.x, pos1.y, pos1.z, pos2.x, pos2.y, pos2.z
    );

    // Validate that points are within the Poincaré ball (||p|| < 1)
    let norm1_sq = pos1.x * pos1.x + pos1.y * pos1.y + pos1.z * pos1.z;
    let norm2_sq = pos2.x * pos2.x + pos2.y * pos2.y + pos2.z * pos2.z;
    
    if norm1_sq >= 1.0 {
        return Err(HyperQLError::GeometricError {
            operation: "hyperbolic_distance".to_string(),
            reason: "Position 1 is outside Poincaré ball (norm >= 1.0)".to_string(),
            positions: vec![format!("pos1=({:.3}, {:.3}, {:.3}), norm²={:.6}", pos1.x, pos1.y, pos1.z, norm1_sq)],
        });
    }
    
    if norm2_sq >= 1.0 {
        return Err(HyperQLError::GeometricError {
            operation: "hyperbolic_distance".to_string(),
            reason: "Position 2 is outside Poincaré ball (norm >= 1.0)".to_string(),
            positions: vec![format!("pos2=({:.3}, {:.3}, {:.3}), norm²={:.6}", pos2.x, pos2.y, pos2.z, norm2_sq)],
        });
    }

    // Implement hyperbolic distance computation using Poincaré ball model
    // Formula: d(x,y) = artanh(||x⊖y||) where x⊖y is Möbius subtraction
    // Möbius subtraction: x⊖y = (x-y)/(1-⟨x,y⟩)

    // Compute dot product ⟨x,y⟩
    let dot_product = pos1.x * pos2.x + pos1.y * pos2.y + pos1.z * pos2.z;

    // Compute x - y
    let diff_x = pos1.x - pos2.x;
    let diff_y = pos1.y - pos2.y;
    let diff_z = pos1.z - pos2.z;

    // Compute denominator: 1 - ⟨x,y⟩
    let denominator = 1.0 - dot_product;

    // Check for numerical issues (denominator too close to zero)
    // Use a larger epsilon threshold for better stability
    const STABILITY_EPSILON: f64 = 1e-10;
    if denominator.abs() < STABILITY_EPSILON {
        return Err(HyperQLError::GeometricError {
            operation: "hyperbolic_distance".to_string(),
            reason: "Numerical instability: positions too close to boundary or each other".to_string(),
            positions: vec![format!("denominator={:.15}, dot_product={:.15}, epsilon_threshold={:.15}", denominator, dot_product, STABILITY_EPSILON)],
        });
    }

    // Additional stability check: prevent division by very small numbers
    if denominator.abs() < STABILITY_EPSILON * 10.0 {
        // Apply defensive clamping for borderline cases
        let adjusted_denominator = if denominator >= 0.0 {
            denominator.max(STABILITY_EPSILON * 10.0)
        } else {
            denominator.min(-STABILITY_EPSILON * 10.0)
        };

        // Recompute with adjusted denominator for better stability
        let mobius_x = diff_x / adjusted_denominator;
        let mobius_y = diff_y / adjusted_denominator;
        let mobius_z = diff_z / adjusted_denominator;
        let mobius_norm_sq = mobius_x * mobius_x + mobius_y * mobius_y + mobius_z * mobius_z;
        let mobius_norm = mobius_norm_sq.sqrt();

        if mobius_norm >= 1.0 {
            let clamped_norm = 0.99999999_f64;
            return Ok(clamped_norm.atanh());
        }

        let distance = mobius_norm.atanh();
        if !distance.is_finite() || distance < 0.0 {
            return Err(HyperQLError::GeometricError {
                operation: "hyperbolic_distance".to_string(),
                reason: "Invalid distance computation result after stability adjustment".to_string(),
                positions: vec![format!("distance={:.15}, mobius_norm={:.15}, adjusted_denominator={:.15}", distance, mobius_norm, adjusted_denominator)],
            });
        }
        return Ok(distance);
    }

    // Compute Möbius subtraction: (x-y)/(1-⟨x,y⟩)
    // This point is reached only if denominator is sufficiently stable
    let mobius_x = diff_x / denominator;
    let mobius_y = diff_y / denominator;
    let mobius_z = diff_z / denominator;

    // Compute norm of Möbius subtraction result
    let mobius_norm_sq = mobius_x * mobius_x + mobius_y * mobius_y + mobius_z * mobius_z;
    let mobius_norm = mobius_norm_sq.sqrt();

    // Check that the result is within valid range for artanh
    if mobius_norm >= 1.0 {
        // Clamp to slightly less than 1 for numerical stability
        let clamped_norm = 0.99999999_f64;
        let distance = clamped_norm.atanh();
        return Ok(distance);
    }

    // Apply artanh to get hyperbolic distance
    let distance = mobius_norm.atanh();

    // Ensure distance is non-negative and finite
    if !distance.is_finite() || distance < 0.0 {
        return Err(HyperQLError::GeometricError {
            operation: "hyperbolic_distance".to_string(),
            reason: "Invalid distance computation result".to_string(),
            positions: vec![format!("distance={:.15}, mobius_norm={:.15}", distance, mobius_norm)],
        });
    }

    Ok(distance)
}

/// Compute approximate hyperbolic distance for performance
pub fn approximate_hyperbolic_distance(pos1: &Position3D, pos2: &Position3D, epsilon: f64) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Approximate hyperbolic distance: pos1=({:.3}, {:.3}, {:.3}) to pos2=({:.3}, {:.3}, {:.3}), epsilon={:.6}",
        pos1.x, pos1.y, pos1.z, pos2.x, pos2.y, pos2.z, epsilon
    );

    // For very small epsilon, use exact computation
    if epsilon < 1e-10 {
        return hyperbolic_distance(pos1, pos2);
    }

    // Use approximation based on Euclidean distance for nearby points
    let dx = pos2.x - pos1.x;
    let dy = pos2.y - pos1.y;
    let dz = pos2.z - pos1.z;
    let euclidean_dist_sq = dx * dx + dy * dy + dz * dz;

    // If points are very close, use first-order Taylor approximation
    if euclidean_dist_sq < 0.01 {
        // For small distances in Poincaré ball: d_h ≈ 2 * d_e / (1 - ||x||²)
        let euclidean_dist = euclidean_dist_sq.sqrt();
        let norm1_sq = pos1.x * pos1.x + pos1.y * pos1.y + pos1.z * pos1.z;
        let norm2_sq = pos2.x * pos2.x + pos2.y * pos2.y + pos2.z * pos2.z;
        let avg_norm_sq = (norm1_sq + norm2_sq) / 2.0;

        // Correction factor based on distance from origin
        let correction = 2.0 / (1.0 - avg_norm_sq).max(0.01);
        return Ok(euclidean_dist * correction);
    }

    // For larger distances, use full computation (already optimized)
    hyperbolic_distance(pos1, pos2)
}

/// Compute geodesic path distance in hyperbolic space
pub fn geodesic_distance(pos1: &Position3D, pos2: &Position3D) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Geodesic distance computation: pos1=({:.3}, {:.3}, {:.3}) to pos2=({:.3}, {:.3}, {:.3})",
        pos1.x, pos1.y, pos1.z, pos2.x, pos2.y, pos2.z
    );

    // In the Poincaré ball model, the geodesic distance equals the hyperbolic distance
    // Geodesics are circular arcs orthogonal to the boundary sphere
    // The shortest path (geodesic) between two points gives the hyperbolic distance

    // This function exists for semantic clarity when working with geodesics explicitly
    // For example, when computing paths along geodesics or studying geodesic flows
    hyperbolic_distance(pos1, pos2)
}

/// Compute hyperbolic distance with caching for repeated calculations
pub fn cached_hyperbolic_distance(pos1: &Position3D, pos2: &Position3D, cache_key: Option<String>) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Cached hyperbolic distance: pos1=({:.3}, {:.3}, {:.3}) to pos2=({:.3}, {:.3}, {:.3}), cache_key={:?}",
        pos1.x, pos1.y, pos1.z, pos2.x, pos2.y, pos2.z, cache_key
    );

    #[derive(Debug, Clone)]
    struct CacheEntry {
        distance: f64,
        last_accessed: u64, // timestamp in milliseconds
        access_count: u64,
    }

    lazy_static::lazy_static! {
        static ref DISTANCE_CACHE: Mutex<HashMap<String, CacheEntry>> = Mutex::new(HashMap::new());
    }

    fn get_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // Generate cache key if not provided
    let key = cache_key.unwrap_or_else(|| {
        format!("{:.6},{:.6},{:.6}:{:.6},{:.6},{:.6}",
                pos1.x, pos1.y, pos1.z, pos2.x, pos2.y, pos2.z)
    });

    // Check cache first
    {
        let mut cache = DISTANCE_CACHE.lock().map_err(|_| HyperQLError::InternalError {
            message: "Failed to acquire cache lock".to_string(),
            component: "cached_hyperbolic_distance".to_string(),
            debug_info: "cache_read".to_string(),
        })?;

        if let Some(entry) = cache.get_mut(&key) {
            // Update access statistics for LRU
            entry.last_accessed = get_timestamp_millis();
            entry.access_count += 1;
            return Ok(entry.distance);
        }
    }

    // Compute distance if not in cache
    let distance = hyperbolic_distance(pos1, pos2)?;

    // Store in cache
    {
        let mut cache = DISTANCE_CACHE.lock().map_err(|_| HyperQLError::InternalError {
            message: "Failed to acquire cache lock".to_string(),
            component: "cached_hyperbolic_distance".to_string(),
            debug_info: "cache_write".to_string(),
        })?;

        // LRU cache eviction: remove least recently used entries when cache is full
        const MAX_CACHE_SIZE: usize = 10000;
        const EVICTION_BATCH_SIZE: usize = 2000; // Remove 20% when full

        if cache.len() >= MAX_CACHE_SIZE {
            // Collect entries with their keys and access info for LRU eviction
            let mut entries_for_eviction: Vec<(String, u64, u64)> = cache
                .iter()
                .map(|(k, v)| (k.clone(), v.last_accessed, v.access_count))
                .collect();

            // Sort by last accessed time (oldest first), then by access count (least used first)
            entries_for_eviction.sort_by(|a, b| {
                a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2))
            });

            // Remove the least recently used entries
            let to_remove = entries_for_eviction
                .into_iter()
                .take(EVICTION_BATCH_SIZE)
                .map(|(key, _, _)| key)
                .collect::<Vec<_>>();

            for key_to_remove in to_remove {
                cache.remove(&key_to_remove);
            }
        }

        // Insert new entry with current timestamp and initial access count
        cache.insert(key, CacheEntry {
            distance,
            last_accessed: get_timestamp_millis(),
            access_count: 1,
        });
    }

    Ok(distance)
}

/// Batch compute hyperbolic distances from one point to many
pub fn batch_hyperbolic_distances(
    reference: &Position3D,
    positions: &[Position3D],
) -> Result<Vec<f64>, HyperQLError> {
    let operation_details = format!(
        "Batch hyperbolic distances from reference=({:.3}, {:.3}, {:.3}) to {} positions",
        reference.x, reference.y, reference.z, positions.len()
    );

    // Implement batch hyperbolic distance computation
    // Optimize by precomputing reference point properties

    if positions.is_empty() {
        return Ok(Vec::new());
    }

    // Validate reference position
    let ref_norm_sq = reference.x * reference.x + reference.y * reference.y + reference.z * reference.z;
    if ref_norm_sq >= 1.0 {
        return Err(HyperQLError::GeometricError {
            operation: "batch_hyperbolic_distances".to_string(),
            reason: "Reference position is outside Poincaré ball (norm >= 1.0)".to_string(),
            positions: vec![format!("reference=({:.3}, {:.3}, {:.3}), norm²={:.6}", reference.x, reference.y, reference.z, ref_norm_sq)],
        });
    }

    let mut distances = Vec::with_capacity(positions.len());

    for (index, position) in positions.iter().enumerate() {
        match hyperbolic_distance(reference, position) {
            Ok(distance) => distances.push(distance),
            Err(e) => {
                return Err(HyperQLError::ExecutionError {
                    message: format!("Failed to compute distance to position[{}]: {}", index, e),
                    operation: "batch_hyperbolic_distances".to_string(),
                    entity_context: Some(format!("batch_item_{}", index)),
                });
            }
        }
    }

    Ok(distances)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperbolic_distance_validation() {
        // Valid points within Poincaré ball
        let pos1 = Position3D { x: 0.1, y: 0.2, z: 0.3 };
        let pos2 = Position3D { x: 0.4, y: 0.5, z: 0.6 };
        
        let result = hyperbolic_distance(&pos1, &pos2);
        assert!(result.is_ok());
        let distance = result.unwrap();
        // Distance should be positive and finite
        assert!(distance > 0.0);
        assert!(distance.is_finite());

        // Invalid point outside Poincaré ball
        let pos_invalid = Position3D { x: 1.1, y: 0.0, z: 0.0 };
        let result_invalid = hyperbolic_distance(&pos1, &pos_invalid);
        assert!(result_invalid.is_err());
        let error_msg_invalid = result_invalid.unwrap_err().to_string();
        assert!(error_msg_invalid.contains("outside Poincaré ball"));
    }

    #[test]
    fn test_approximate_distance() {
        let pos1 = Position3D { x: 0.1, y: 0.1, z: 0.1 };
        let pos2 = Position3D { x: 0.2, y: 0.2, z: 0.2 };
        let epsilon = 0.001;

        // Test that approximate distance works and is close to exact
        let approx_result = approximate_hyperbolic_distance(&pos1, &pos2, epsilon);
        assert!(approx_result.is_ok());
        let approx_dist = approx_result.unwrap();

        let exact_result = hyperbolic_distance(&pos1, &pos2);
        assert!(exact_result.is_ok());
        let exact_dist = exact_result.unwrap();

        // Approximate should be close to exact
        assert!((approx_dist - exact_dist).abs() < 0.01);
    }

    #[test]
    fn test_geodesic_distance() {
        let pos1 = Position3D { x: 0.0, y: 0.0, z: 0.0 }; // Origin
        let pos2 = Position3D { x: 0.5, y: 0.0, z: 0.0 }; // Along x-axis

        // Geodesic distance should equal hyperbolic distance
        let geodesic_result = geodesic_distance(&pos1, &pos2);
        assert!(geodesic_result.is_ok());
        let geodesic_dist = geodesic_result.unwrap();

        let hyperbolic_result = hyperbolic_distance(&pos1, &pos2);
        assert!(hyperbolic_result.is_ok());
        let hyperbolic_dist = hyperbolic_result.unwrap();

        // They should be identical
        assert_eq!(geodesic_dist, hyperbolic_dist);
    }

    #[test]
    fn test_cached_distance() {
        let pos1 = Position3D { x: 0.1, y: 0.2, z: 0.3 };
        let pos2 = Position3D { x: 0.4, y: 0.5, z: 0.6 };
        let cache_key = Some("test_cache_key".to_string());

        // First call should compute and cache
        let result1 = cached_hyperbolic_distance(&pos1, &pos2, cache_key.clone());
        assert!(result1.is_ok());
        let dist1 = result1.unwrap();

        // Second call with same key should return cached value
        let result2 = cached_hyperbolic_distance(&pos1, &pos2, cache_key.clone());
        assert!(result2.is_ok());
        let dist2 = result2.unwrap();

        // Both should be identical
        assert_eq!(dist1, dist2);

        // Should match hyperbolic distance
        let exact = hyperbolic_distance(&pos1, &pos2).unwrap();
        assert_eq!(dist1, exact);
    }

    #[test]
    fn test_batch_distances() {
        let reference = Position3D { x: 0.0, y: 0.0, z: 0.0 };
        let positions = vec![
            Position3D { x: 0.1, y: 0.0, z: 0.0 },
            Position3D { x: 0.0, y: 0.2, z: 0.0 },
            Position3D { x: 0.0, y: 0.0, z: 0.3 },
        ];

        let result = batch_hyperbolic_distances(&reference, &positions);
        assert!(result.is_ok());
        let distances = result.unwrap();
        // Should return 3 distances
        assert_eq!(distances.len(), 3);
        // All distances should be positive and finite
        for distance in &distances {
            assert!(distance > &0.0);
            assert!(distance.is_finite());
        }
    }

    #[test]
    fn test_position_validation_edge_cases() {
        // Point exactly on boundary (should fail)
        let pos_boundary = Position3D { x: 1.0, y: 0.0, z: 0.0 };
        let pos_valid = Position3D { x: 0.0, y: 0.0, z: 0.0 };

        let result = hyperbolic_distance(&pos_boundary, &pos_valid);
        assert!(result.is_err());
        
        // Very close to boundary but valid
        let pos_close = Position3D { x: 0.999, y: 0.0, z: 0.0 };
        let result_close = hyperbolic_distance(&pos_close, &pos_valid);
        assert!(result_close.is_ok()); // Should work for valid positions
        let distance = result_close.unwrap();
        // Should be a large finite distance since pos_close is near the boundary
        assert!(distance > 0.0);
        assert!(distance.is_finite());
    }
}