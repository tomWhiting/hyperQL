//! Vector Similarity Functions for Named Embeddings
//!
//! This module implements vector similarity functions that operate on named embeddings.
//! Unlike geometric operations, these functions work with user-defined vector fields
//! that entities can have multiple of, supporting various vector types and metrics.
//!
//! ## Named Vector Operations
//!
//! These functions operate on specific named vector fields:
//! - Entity can have multiple vectors: "text_embedding", "code_vec", "bge_m3"
//! - Each function specifies which named vector to operate on
//! - Support for dense, sparse, and ColBERT vector types
//!
//! ## Performance Considerations
//!
//! All implementations return "not yet implemented" with operation details to
//! facilitate integration with the actual vector processing engine while
//! maintaining clean interfaces for future real implementations.

use crate::{HyperQLError, Value};
use crate::ast::vector::similarity::{SimilarityMetric, VectorType};
use std::collections::HashMap;

/// Compute similarity between a named vector and reference vector
pub fn compute_named_vector_similarity(
    entity_vectors: &HashMap<String, Value>,
    vector_name: &str,
    reference_vector: &Value,
    metric: &SimilarityMetric,
    vector_type: &VectorType,
    threshold: Option<f64>,
) -> Result<f64, HyperQLError> {
    // Extract the named vector from entity_vectors
    let entity_vector = entity_vectors.get(vector_name).ok_or_else(|| {
        HyperQLError::ExecutionError {
            message: format!("Named vector '{}' not found in entity", vector_name),
            operation: "compute_named_vector_similarity".to_string(),
            entity_context: Some(format!("vector_name={}", vector_name)),
        }
    })?;

    // Apply the appropriate similarity metric based on vector type
    let similarity = match vector_type {
        VectorType::Dense { .. } => {
            let entity_vec = extract_dense_vector(entity_vector, vector_name)?;
            let reference_vec = extract_dense_vector(reference_vector, "reference")?;

            match metric {
                SimilarityMetric::Cosine => cosine_similarity(&entity_vec, &reference_vec, vector_name)?,
                SimilarityMetric::DotProduct => dot_product_similarity(&entity_vec, &reference_vec, vector_name)?,
                SimilarityMetric::Euclidean => euclidean_similarity(&entity_vec, &reference_vec, vector_name)?,
                SimilarityMetric::Manhattan => {
                    // Manhattan distance converted to similarity
                    let distance = manhattan_distance_internal(&entity_vec, &reference_vec)?;
                    1.0 / (1.0 + distance)
                }
                _ => return Err(HyperQLError::ValidationError {
                    message: format!("Similarity metric {:?} not supported for dense vectors", metric),
                    field: Some("metric".to_string()),
                }),
            }
        },
        VectorType::Sparse { .. } => {
            let (entity_indices, entity_values) = extract_sparse_vector(entity_vector, vector_name)?;
            let (ref_indices, ref_values) = extract_sparse_vector(reference_vector, "reference")?;

            match metric {
                SimilarityMetric::Jaccard => jaccard_similarity(&entity_indices, &entity_values, &ref_indices, &ref_values, vector_name)?,
                SimilarityMetric::DotProduct => dot_product_sparse(&entity_indices, &entity_values, &ref_indices, &ref_values)?,
                _ => return Err(HyperQLError::ValidationError {
                    message: format!("Similarity metric {:?} not supported for sparse vectors", metric),
                    field: Some("metric".to_string()),
                }),
            }
        },
        VectorType::ColBERT { .. } => {
            let entity_tokens = extract_colbert_vector(entity_vector, vector_name)?;
            let reference_tokens = extract_colbert_vector(reference_vector, "reference")?;

            colbert_similarity(&entity_tokens, &reference_tokens, vector_name)?
        },
    };

    // Apply threshold filtering if specified
    if let Some(threshold_value) = threshold {
        apply_similarity_threshold(similarity, Some(threshold_value), vector_name)?;
    }

    Ok(similarity)
}

/// Compute cosine similarity between named dense vectors
pub fn cosine_similarity(
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

    // Compute dot product: Σᵢ aᵢbᵢ
    let dot_product: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| a * b)
        .sum();

    // Compute norms: ||a|| = √(Σᵢ aᵢ²)
    let norm_a: f64 = vector_a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = vector_b.iter().map(|x| x * x).sum::<f64>().sqrt();

    // Handle zero vectors
    if norm_a == 0.0 || norm_b == 0.0 {
        return Ok(0.0); // Cosine similarity of zero vector with any vector is 0
    }

    // Return cos(θ) = ⟨a,b⟩ / (||a|| ||b||)
    let similarity = dot_product / (norm_a * norm_b);

    // Clamp to [-1, 1] to handle numerical precision issues
    Ok(similarity.clamp(-1.0, 1.0))
}

/// Compute Jaccard similarity for sparse named vectors
pub fn jaccard_similarity(
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
        .map(|(&idx, &val)| (idx, val.abs())) // Use absolute values for Jaccard
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
        return Ok(if intersection_sum == 0.0 { 1.0 } else { 0.0 });
    }

    // Return Jaccard similarity: |A ∩ B| / |A ∪ B|
    Ok(intersection_sum / union_sum)
}

/// Compute Euclidean distance between named vectors (inverted for similarity)
pub fn euclidean_similarity(
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

    let distance = squared_distance.sqrt();

    // Convert distance to similarity: 1 / (1 + distance)
    Ok(1.0 / (1.0 + distance))
}

/// Compute dot product similarity between named vectors
pub fn dot_product_similarity(
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

    // Compute dot product: Σᵢ aᵢbᵢ
    let dot_product: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| a * b)
        .sum();

    Ok(dot_product)
}

/// Compute ColBERT-style multi-vector similarity
pub fn colbert_similarity(
    tokens_a: &[Vec<f64>],  // Multiple token vectors for entity A
    tokens_b: &[Vec<f64>],  // Multiple token vectors for entity B
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

    // ColBERT max-sim operation: Σᵢ max_j(sim(tokᵢᴬ, tokⱼᴮ))
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

    // Normalize by the number of tokens in A
    Ok(total_similarity / tokens_a.len() as f64)
}

/// Apply similarity threshold filtering
pub fn apply_similarity_threshold(
    similarity_score: f64,
    threshold: Option<f64>,
    _vector_name: &str, // Keep for API consistency but not used in logic
) -> Result<bool, HyperQLError> {
    match threshold {
        Some(min_threshold) => {
            // Validate threshold is in reasonable range
            if min_threshold < -1.0 || min_threshold > 1.0 {
                return Err(HyperQLError::ValidationError {
                    message: "Similarity threshold must be between -1.0 and 1.0".to_string(),
                    field: Some("threshold".to_string()),
                });
            }

            Ok(similarity_score >= min_threshold)
        }
        None => {
            // No threshold - always pass
            Ok(true)
        }
    }
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

/// Compute cosine similarity between two vectors (raw computation without validation)
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

/// Compute dot product between sparse vectors
fn dot_product_sparse(indices_a: &[u32], values_a: &[f64], indices_b: &[u32], values_b: &[f64]) -> Result<f64, HyperQLError> {
    let map_a: HashMap<u32, f64> = indices_a.iter().zip(values_a.iter())
        .map(|(&idx, &val)| (idx, val))
        .collect();

    let map_b: HashMap<u32, f64> = indices_b.iter().zip(values_b.iter())
        .map(|(&idx, &val)| (idx, val))
        .collect();

    let mut dot_product = 0.0;
    for (&idx, &val_a) in &map_a {
        if let Some(&val_b) = map_b.get(&idx) {
            dot_product += val_a * val_b;
        }
    }

    Ok(dot_product)
}

/// Compute Manhattan distance between two vectors (internal helper)
fn manhattan_distance_internal(vector_a: &[f64], vector_b: &[f64]) -> Result<f64, HyperQLError> {
    if vector_a.len() != vector_b.len() {
        return Err(HyperQLError::ValidationError {
            message: "Vector dimension mismatch in Manhattan distance".to_string(),
            field: Some("dimensions".to_string()),
        });
    }

    let distance: f64 = vector_a.iter().zip(vector_b.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();

    Ok(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![0.5, 1.5, 2.5];
        let vector_name = "test_embedding";

        let result = cosine_similarity(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let similarity = result.unwrap();
        assert!(similarity > 0.0 && similarity <= 1.0);
    }

    #[test]
    fn test_dot_product_similarity() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![2.0, 3.0, 4.0];
        let vector_name = "test_embedding";

        let result = dot_product_similarity(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let dot_product = result.unwrap();
        // 1*2 + 2*3 + 3*4 = 2 + 6 + 12 = 20
        assert!((dot_product - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_euclidean_similarity() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![1.0, 2.0, 3.0];  // Identical vectors
        let vector_name = "test_embedding";

        let result = euclidean_similarity(&vec_a, &vec_b, vector_name);
        assert!(result.is_ok());
        let similarity = result.unwrap();
        // Distance is 0 for identical vectors, so similarity should be 1.0
        assert!((similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![1.0, 2.0];  // Different dimensions
        let vector_name = "test_embedding";

        let result = cosine_similarity(&vec_a, &vec_b, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("dimension mismatch"));
    }

    #[test]
    fn test_jaccard_similarity() {
        let indices_a = vec![0, 2, 5];
        let values_a = vec![1.0, 0.5, 2.0];
        let indices_b = vec![1, 2, 3, 5];
        let values_b = vec![0.8, 0.6, 1.2, 1.8];
        let vector_name = "sparse_keywords";

        let result = jaccard_similarity(&indices_a, &values_a, &indices_b, &values_b, vector_name);
        assert!(result.is_ok());
        let similarity = result.unwrap();
        assert!(similarity >= 0.0 && similarity <= 1.0);

        // Test identical sparse vectors
        let result_identical = jaccard_similarity(&indices_a, &values_a, &indices_a, &values_a, vector_name);
        assert!(result_identical.is_ok());
        let identical_similarity = result_identical.unwrap();
        assert!((identical_similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_colbert_similarity() {
        let tokens_a = vec![
            vec![0.1, 0.2, 0.3],
            vec![0.4, 0.5, 0.6],
        ];
        let tokens_b = vec![
            vec![0.15, 0.25, 0.35],
            vec![0.2, 0.3, 0.4],
            vec![0.45, 0.55, 0.65],
        ];
        let vector_name = "colbert_tokens";

        let result = colbert_similarity(&tokens_a, &tokens_b, vector_name);
        assert!(result.is_ok());
        let similarity = result.unwrap();
        assert!(similarity >= -1.0 && similarity <= 1.0);

        // Test identical ColBERT vectors
        let result_identical = colbert_similarity(&tokens_a, &tokens_a, vector_name);
        assert!(result_identical.is_ok());
        let identical_similarity = result_identical.unwrap();
        assert!(identical_similarity > 0.9);  // Should be very close to 1.0
    }

    #[test]
    fn test_threshold_filtering() {
        let vector_name = "text_embedding";

        // Test score below threshold
        let result = apply_similarity_threshold(0.75, Some(0.8), vector_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        // Test score above threshold
        let result = apply_similarity_threshold(0.85, Some(0.8), vector_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Test no threshold
        let result = apply_similarity_threshold(0.5, None, vector_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Test invalid threshold
        let result = apply_similarity_threshold(0.5, Some(1.5), vector_name);  // > 1.0
        assert!(result.is_err());
    }
}