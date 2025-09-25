//! Vector Distance Functions for Named Embeddings
//!
//! This module implements vector distance functions that operate on named embeddings.
//! These functions complement similarity functions by providing distance-based metrics
//! for named vector fields that entities can have multiple of.
//!
//! ## Named Vector Distance Operations
//!
//! These functions operate on specific named vector fields:
//! - Support for user-defined vector names like "text_embedding", "code_vec"
//! - Distance-based metrics (lower values indicate more similarity)
//! - Integration with k-NN queries for nearest neighbor search
//!
//! ## Distance vs Similarity
//!
//! Distance functions return values where:
//! - 0.0 = identical vectors
//! - Higher values = more dissimilar vectors
//! - Can be converted to similarity via: sim = 1 / (1 + distance)

use crate::{HyperQLError, Value};
use crate::ast::vector::similarity::{SimilarityMetric, VectorType};
use std::collections::HashMap;

/// Compute distance between a named vector and reference vector
pub fn compute_named_vector_distance(
    entity_vectors: &HashMap<String, Value>,
    vector_name: &str,
    reference_vector: &Value,
    metric: &SimilarityMetric,
    vector_type: &VectorType,
) -> Result<f64, HyperQLError> {
    // Extract the named vector from entity_vectors
    let entity_vector = entity_vectors.get(vector_name).ok_or_else(|| {
        HyperQLError::ExecutionError {
            message: format!("Named vector '{}' not found in entity", vector_name),
            operation: "compute_named_vector_distance".to_string(),
            entity_context: Some(format!("vector_name={}", vector_name)),
        }
    })?;

    // Apply the appropriate distance metric based on vector type
    let distance = match vector_type {
        VectorType::Dense { .. } => {
            let entity_vec = extract_dense_vector(entity_vector, vector_name)?;
            let reference_vec = extract_dense_vector(reference_vector, "reference")?;

            match metric {
                SimilarityMetric::Euclidean => euclidean_distance(&entity_vec, &reference_vec, vector_name)?,
                SimilarityMetric::Manhattan => manhattan_distance(&entity_vec, &reference_vec, vector_name)?,
                SimilarityMetric::Cosine => cosine_distance(&entity_vec, &reference_vec, vector_name)?,
                _ => return Err(HyperQLError::ValidationError {
                    message: format!("Distance metric {:?} not supported for dense vectors", metric),
                    field: Some("metric".to_string()),
                }),
            }
        },
        VectorType::Sparse { .. } => {
            let (entity_indices, entity_values) = extract_sparse_vector(entity_vector, vector_name)?;
            let (ref_indices, ref_values) = extract_sparse_vector(reference_vector, "reference")?;

            match metric {
                SimilarityMetric::Jaccard => jaccard_distance(&entity_indices, &entity_values, &ref_indices, &ref_values, vector_name)?,
                _ => return Err(HyperQLError::ValidationError {
                    message: format!("Distance metric {:?} not supported for sparse vectors", metric),
                    field: Some("metric".to_string()),
                }),
            }
        },
        VectorType::ColBERT { .. } => {
            let entity_tokens = extract_colbert_vector(entity_vector, vector_name)?;
            let reference_tokens = extract_colbert_vector(reference_vector, "reference")?;

            colbert_distance(&entity_tokens, &reference_tokens, vector_name)?
        },
    };

    Ok(distance)
}

/// Compute Euclidean distance between named dense vectors
pub fn euclidean_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    // Validate dimensions match
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: format!(
                "Vector dimension mismatch for '{}': {} vs {}",
                vector_name, vector_a.len(), vector_b.len()
            ),
            field: Some("dimensions".to_string()),
        });
    }

    if vector_a.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: format!("Empty vectors not supported for '{}'", vector_name),
            field: Some("vector_length".to_string()),
        });
    }

    // Compute Euclidean distance: √(Σᵢ (aᵢ - bᵢ)²)
    let squared_distance: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| {
            let diff = a - b;
            diff * diff
        })
        .sum();

    Ok(squared_distance.sqrt())
}

/// Compute Manhattan distance between named dense vectors
pub fn manhattan_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    // Validate dimensions match
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: format!(
                "Vector dimension mismatch for '{}': {} vs {}",
                vector_name, vector_a.len(), vector_b.len()
            ),
            field: Some("dimensions".to_string()),
        });
    }

    if vector_a.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: format!("Empty vectors not supported for '{}'", vector_name),
            field: Some("vector_length".to_string()),
        });
    }

    // Compute Manhattan distance: Σᵢ |aᵢ - bᵢ|
    let distance: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();

    Ok(distance)
}

/// Compute generalized Minkowski distance between named vectors
pub fn minkowski_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    p: f64,
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    // Validate dimensions match
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: format!(
                "Vector dimension mismatch for '{}': {} vs {}",
                vector_name, vector_a.len(), vector_b.len()
            ),
            field: Some("dimensions".to_string()),
        });
    }

    if vector_a.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: format!("Empty vectors not supported for '{}'", vector_name),
            field: Some("vector_length".to_string()),
        });
    }

    if p <= 0.0 {
        return Err(HyperQLError::ValidationError {
            message: format!("Minkowski p parameter must be positive, got: {}", p),
            field: Some("p".to_string()),
        });
    }

    // Handle special cases for efficiency
    if (p - 1.0).abs() < f64::EPSILON {
        // p = 1: Manhattan distance
        return manhattan_distance(vector_a, vector_b, vector_name);
    }
    if (p - 2.0).abs() < f64::EPSILON {
        // p = 2: Euclidean distance
        return euclidean_distance(vector_a, vector_b, vector_name);
    }
    if p.is_infinite() {
        // p = infinity: Chebyshev distance (max difference)
        let max_diff = vector_a.iter().zip(vector_b.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, |acc, x| acc.max(x));
        return Ok(max_diff);
    }

    // General case: distance = (Σᵢ |aᵢ - bᵢ|^p)^(1/p)
    let powered_sum: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| (a - b).abs().powf(p))
        .sum();

    Ok(powered_sum.powf(1.0 / p))
}

/// Compute Cosine distance between named vectors (1 - cosine_similarity)
pub fn cosine_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    // Validate dimensions match
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: format!(
                "Vector dimension mismatch for '{}': {} vs {}",
                vector_name, vector_a.len(), vector_b.len()
            ),
            field: Some("dimensions".to_string()),
        });
    }

    if vector_a.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: format!("Empty vectors not supported for '{}'", vector_name),
            field: Some("vector_length".to_string()),
        });
    }

    // Compute cosine similarity first
    let dot_product: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| a * b)
        .sum();

    let norm_a: f64 = vector_a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = vector_b.iter().map(|x| x * x).sum::<f64>().sqrt();

    // Handle zero vectors - cosine distance is 1.0
    if norm_a == 0.0 || norm_b == 0.0 {
        return Ok(1.0);
    }

    let cosine_similarity = (dot_product / (norm_a * norm_b)).clamp(-1.0, 1.0);

    // Return cosine distance = 1 - cosine_similarity
    Ok(1.0 - cosine_similarity)
}

/// Compute Hamming distance between binary named vectors
pub fn hamming_distance(
    vector_a: &[bool],
    vector_b: &[bool],
    vector_name: &str,
) -> Result<u32, HyperQLError> {
    // Validate dimensions match
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: format!(
                "Vector dimension mismatch for '{}': {} vs {}",
                vector_name, vector_a.len(), vector_b.len()
            ),
            field: Some("dimensions".to_string()),
        });
    }

    if vector_a.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: format!("Empty vectors not supported for '{}'", vector_name),
            field: Some("vector_length".to_string()),
        });
    }

    // Count positions where bits differ (XOR operation)
    let hamming_distance = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| if a != b { 1u32 } else { 0u32 })
        .sum();

    Ok(hamming_distance)
}

/// Compute Jaccard distance between sparse named vectors (1 - jaccard_similarity)
pub fn jaccard_distance(
    indices_a: &[u32],
    values_a: &[f64],
    indices_b: &[u32],
    values_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    // Validate input consistency
    if indices_a.len() != values_a.len() {
        return Err(HyperQLError::ValidationError {
            message: format!("Inconsistent sparse vector A for '{}': indices={}, values={}",
                vector_name, indices_a.len(), values_a.len()),
            field: Some("vector_a".to_string()),
        });
    }

    if indices_b.len() != values_b.len() {
        return Err(HyperQLError::ValidationError {
            message: format!("Inconsistent sparse vector B for '{}': indices={}, values={}",
                vector_name, indices_b.len(), values_b.len()),
            field: Some("vector_b".to_string()),
        });
    }

    // Convert to HashMaps for efficient lookups
    let map_a: HashMap<u32, f64> = indices_a.iter().zip(values_a.iter())
        .map(|(&idx, &val)| (idx, val.abs()))
        .collect();

    let map_b: HashMap<u32, f64> = indices_b.iter().zip(values_b.iter())
        .map(|(&idx, &val)| (idx, val.abs()))
        .collect();

    let mut intersection_sum = 0.0;
    let mut union_sum = 0.0;

    // Get all unique indices from both vectors
    let all_indices: std::collections::HashSet<u32> = map_a.keys()
        .chain(map_b.keys())
        .copied()
        .collect();

    // For each index, compute min (intersection) and max (union) values
    for &idx in &all_indices {
        let val_a = map_a.get(&idx).copied().unwrap_or(0.0);
        let val_b = map_b.get(&idx).copied().unwrap_or(0.0);

        intersection_sum += val_a.min(val_b);
        union_sum += val_a.max(val_b);
    }

    // Handle empty vectors
    if union_sum == 0.0 {
        return Ok(if intersection_sum == 0.0 { 0.0 } else { 1.0 });
    }

    // Compute Jaccard similarity and return distance = 1 - similarity
    let jaccard_similarity = intersection_sum / union_sum;
    Ok(1.0 - jaccard_similarity)
}

/// Compute ColBERT-style multi-vector distance (max-sim converted to distance)
pub fn colbert_distance(
    tokens_a: &[Vec<f64>],
    tokens_b: &[Vec<f64>],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return Err(HyperQLError::ValidationError {
            message: format!("Empty token sequences not supported for ColBERT vector '{}'", vector_name),
            field: Some("token_count".to_string()),
        });
    }

    // Validate that all token vectors have the same dimensions
    let expected_dim = tokens_a[0].len();
    for (i, token) in tokens_a.iter().enumerate() {
        if token.len() != expected_dim {
            return Err(HyperQLError::ValidationError {
                message: format!("Token dimension mismatch in tokens_a[{}] for '{}': expected {}, got {}",
                    i, vector_name, expected_dim, token.len()),
                field: Some("token_dimensions".to_string()),
            });
        }
    }

    for (i, token) in tokens_b.iter().enumerate() {
        if token.len() != expected_dim {
            return Err(HyperQLError::ValidationError {
                message: format!("Token dimension mismatch in tokens_b[{}] for '{}': expected {}, got {}",
                    i, vector_name, expected_dim, token.len()),
                field: Some("token_dimensions".to_string()),
            });
        }
    }

    // Compute ColBERT similarity first
    let mut total_similarity = 0.0;

    for token_a in tokens_a {
        let mut max_similarity = f64::NEG_INFINITY;

        // Find the maximum cosine similarity between this token in A and all tokens in B
        for token_b in tokens_b {
            let similarity = cosine_similarity_raw(token_a, token_b)?;
            if similarity > max_similarity {
                max_similarity = similarity;
            }
        }

        total_similarity += max_similarity;
    }

    // Normalize by the number of tokens in A to get average max similarity
    let colbert_similarity = total_similarity / tokens_a.len() as f64;

    // Convert similarity to distance. Since cosine similarity is in [-1, 1],
    // the maximum possible similarity is 1.0, so distance = 1 - similarity
    // This gives us a distance in [0, 2] where 0 = identical, 2 = opposite
    Ok(1.0 - colbert_similarity)
}

// Helper functions for vector extraction and operations

/// Extract dense vector from Value
fn extract_dense_vector(value: &Value, context: &str) -> Result<Vec<f64>, HyperQLError> {
    match value {
        Value::Vector(vector) => Ok(vector.dimensions.clone()),
        Value::List(values) => {
            let mut result = Vec::with_capacity(values.len());
            for (i, v) in values.iter().enumerate() {
                match v {
                    Value::Float(f) => result.push(*f),
                    Value::Int(i) => result.push(*i as f64),
                    _ => return Err(HyperQLError::TypeError {
                        expected: "numeric value".to_string(),
                        found: format!("{:?}", v),
                        context: format!("dense vector {} at index {}", context, i),
                    }),
                }
            }
            Ok(result)
        }
        _ => Err(HyperQLError::TypeError {
            expected: "Vector or List".to_string(),
            found: format!("{:?}", value),
            context: format!("dense vector {}", context),
        }),
    }
}

/// Extract sparse vector from Value (returns indices and values)
fn extract_sparse_vector(value: &Value, context: &str) -> Result<(Vec<u32>, Vec<f64>), HyperQLError> {
    match value {
        Value::Map(map) => {
            let mut indices = Vec::new();
            let mut values = Vec::new();

            for (key, val) in map {
                let index: u32 = key.parse().map_err(|_| HyperQLError::TypeError {
                    expected: "numeric index".to_string(),
                    found: key.clone(),
                    context: format!("sparse vector {} key", context),
                })?;

                let value: f64 = match val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return Err(HyperQLError::TypeError {
                        expected: "numeric value".to_string(),
                        found: format!("{:?}", val),
                        context: format!("sparse vector {} value", context),
                    }),
                };

                indices.push(index);
                values.push(value);
            }

            Ok((indices, values))
        }
        _ => Err(HyperQLError::TypeError {
            expected: "Map".to_string(),
            found: format!("{:?}", value),
            context: format!("sparse vector {}", context),
        }),
    }
}

/// Extract ColBERT multi-vector from Value
fn extract_colbert_vector(value: &Value, context: &str) -> Result<Vec<Vec<f64>>, HyperQLError> {
    match value {
        Value::List(outer_list) => {
            let mut result = Vec::with_capacity(outer_list.len());
            for (i, token_value) in outer_list.iter().enumerate() {
                let token_vector = extract_dense_vector(token_value, &format!("{}_token_{}", context, i))?;
                result.push(token_vector);
            }
            Ok(result)
        }
        _ => Err(HyperQLError::TypeError {
            expected: "List of vectors".to_string(),
            found: format!("{:?}", value),
            context: format!("ColBERT vector {}", context),
        }),
    }
}

/// Compute cosine similarity between two vectors (raw computation)
fn cosine_similarity_raw(vector_a: &[f64], vector_b: &[f64]) -> Result<f64, HyperQLError> {
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: "Vector dimension mismatch in cosine similarity".to_string(),
            field: Some("dimensions".to_string()),
        });
    }

    let dot_product: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| a * b)
        .sum();

    let norm_a: f64 = vector_a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = vector_b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        Ok(0.0)
    } else {
        Ok((dot_product / (norm_a * norm_b)).clamp(-1.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_distance() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![1.0, 2.0, 3.0];  // Identical vectors
        let vector_name = "test_embedding";

        let result = euclidean_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        assert!((distance - 0.0).abs() < f64::EPSILON);

        // Test non-identical vectors
        let vec_c = vec![4.0, 5.0, 6.0];
        let result2 = euclidean_distance(&vec_a, &vec_c, vector_name);
        assert!(result2.is_ok());
        let distance2 = result2.unwrap();
        assert!(distance2 > 0.0);
    }

    #[test]
    fn test_manhattan_distance() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![2.0, 3.0, 4.0];
        let vector_name = "test_embedding";

        let result = manhattan_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        // |1-2| + |2-3| + |3-4| = 1 + 1 + 1 = 3
        assert!((distance - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cosine_distance() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![1.0, 2.0, 3.0];  // Identical vectors
        let vector_name = "test_embedding";

        let result = cosine_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        // Distance should be 0 for identical vectors (1 - 1)
        assert!((distance - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_minkowski_distance_with_parameter() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![0.5, 1.5, 2.5];
        let vector_name = "test_embedding";
        let p = 3.0;

        let result = minkowski_distance(&vec_a, &vec_b, p, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        assert!(distance > 0.0);

        // Test p=1 (should match Manhattan)
        let manhattan_result = minkowski_distance(&vec_a, &vec_b, 1.0, vector_name);
        let manhattan_direct = manhattan_distance(&vec_a, &vec_b, vector_name);
        assert!(manhattan_result.is_ok() && manhattan_direct.is_ok());
        assert!((manhattan_result.unwrap() - manhattan_direct.unwrap()).abs() < f64::EPSILON);

        // Test p=2 (should match Euclidean)
        let euclidean_result = minkowski_distance(&vec_a, &vec_b, 2.0, vector_name);
        let euclidean_direct = euclidean_distance(&vec_a, &vec_b, vector_name);
        assert!(euclidean_result.is_ok() && euclidean_direct.is_ok());
        assert!((euclidean_result.unwrap() - euclidean_direct.unwrap()).abs() < f64::EPSILON);

        // Test invalid p
        let invalid_result = minkowski_distance(&vec_a, &vec_b, -1.0, vector_name);
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_hamming_distance() {
        let vec_a = vec![true, false, true, false];
        let vec_b = vec![true, true, false, false];
        let vector_name = "binary_features";

        let result = hamming_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        // Positions 1 and 2 differ: false!=true, true!=false
        assert_eq!(distance, 2);

        // Test identical vectors
        let result_identical = hamming_distance(&vec_a, &vec_a, vector_name);
        assert!(result_identical.is_ok());
        assert_eq!(result_identical.unwrap(), 0);
    }

    #[test]
    fn test_sparse_vector_distance() {
        let indices_a = vec![0, 2, 5];
        let values_a = vec![1.0, 0.5, 2.0];
        let indices_b = vec![1, 2, 3, 5];
        let values_b = vec![0.8, 0.6, 1.2, 1.8];
        let vector_name = "sparse_keywords";

        let result = jaccard_distance(&indices_a, &values_a, &indices_b, &values_b, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        assert!(distance >= 0.0 && distance <= 1.0);

        // Test identical vectors (distance should be 0)
        let result_identical = jaccard_distance(&indices_a, &values_a, &indices_a, &values_a, vector_name);
        assert!(result_identical.is_ok());
        assert!((result_identical.unwrap() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_colbert_distance() {
        let tokens_a = vec![
            vec![0.1, 0.2, 0.3],
            vec![0.4, 0.5, 0.6],
        ];
        let tokens_b = vec![
            vec![0.15, 0.25, 0.35],
            vec![0.45, 0.55, 0.65],
        ];
        let vector_name = "colbert_tokens";

        let result = colbert_distance(&tokens_a, &tokens_b, vector_name);
        assert!(result.is_ok());
        let distance = result.unwrap();
        assert!(distance >= 0.0 && distance <= 2.0);

        // Test identical tokens (distance should be 0)
        let result_identical = colbert_distance(&tokens_a, &tokens_a, vector_name);
        assert!(result_identical.is_ok());
        let identical_distance = result_identical.unwrap();
        assert!(identical_distance.abs() < f64::EPSILON);
    }
}