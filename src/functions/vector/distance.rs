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
    let operation_details = format!(
        "Named vector distance: vector_name='{}', metric={:?}, vector_type={:?}",
        vector_name, metric, vector_type
    );

    // TODO: Implement actual named vector distance computation
    // This would involve:
    // 1. Extract the named vector from entity_vectors
    // 2. Validate vector compatibility with reference_vector
    // 3. Apply the appropriate distance metric
    // 4. Handle different vector types (Dense, Sparse, ColBERT)
    // 5. Return distance value (lower = more similar)

    Err(HyperQLError::ExecutionError {
        message: format!("Named vector distance not yet implemented. {}", operation_details),
        operation: "compute_named_vector_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute Euclidean distance between named dense vectors
pub fn euclidean_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Euclidean distance computation for named vector: '{}', dimensions: {} x {}",
        vector_name, vector_a.len(), vector_b.len()
    );

    // TODO: Implement actual Euclidean distance computation
    // distance = √(Σᵢ (aᵢ - bᵢ)²)
    // This would involve:
    // 1. Compute squared differences: (aᵢ - bᵢ)²
    // 2. Sum squared differences
    // 3. Take square root
    // 4. Validate input dimensions match

    Err(HyperQLError::ExecutionError {
        message: format!("Euclidean distance not yet implemented. {}", operation_details),
        operation: "euclidean_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute Manhattan distance between named dense vectors
pub fn manhattan_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Manhattan distance computation for named vector: '{}', dimensions: {} x {}",
        vector_name, vector_a.len(), vector_b.len()
    );

    // TODO: Implement actual Manhattan distance computation
    // distance = Σᵢ |aᵢ - bᵢ|
    // This would involve:
    // 1. Compute absolute differences: |aᵢ - bᵢ|
    // 2. Sum absolute differences
    // 3. Validate input dimensions match

    Err(HyperQLError::ExecutionError {
        message: format!("Manhattan distance not yet implemented. {}", operation_details),
        operation: "manhattan_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute generalized Minkowski distance between named vectors
pub fn minkowski_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    p: f64,
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Minkowski distance (p={}) computation for named vector: '{}', dimensions: {} x {}",
        p, vector_name, vector_a.len(), vector_b.len()
    );

    // TODO: Implement actual Minkowski distance computation
    // distance = (Σᵢ |aᵢ - bᵢ|^p)^(1/p)
    // This would involve:
    // 1. Compute powered absolute differences: |aᵢ - bᵢ|^p
    // 2. Sum powered differences
    // 3. Take p-th root
    // 4. Handle special cases (p=1: Manhattan, p=2: Euclidean, p=∞: Chebyshev)

    Err(HyperQLError::ExecutionError {
        message: format!("Minkowski distance not yet implemented. {}", operation_details),
        operation: "minkowski_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute Cosine distance between named vectors (1 - cosine_similarity)
pub fn cosine_distance(
    vector_a: &[f64],
    vector_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Cosine distance computation for named vector: '{}', dimensions: {} x {}",
        vector_name, vector_a.len(), vector_b.len()
    );

    // TODO: Implement actual cosine distance computation
    // cosine_distance = 1 - cosine_similarity
    // cosine_similarity = ⟨a,b⟩ / (||a|| ||b||)
    // This would involve:
    // 1. Compute cosine similarity
    // 2. Return 1 - similarity for distance semantics
    // 3. Handle zero vectors gracefully

    Err(HyperQLError::ExecutionError {
        message: format!("Cosine distance not yet implemented. {}", operation_details),
        operation: "cosine_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute Hamming distance between binary named vectors
pub fn hamming_distance(
    vector_a: &[bool],
    vector_b: &[bool],
    vector_name: &str,
) -> Result<u32, HyperQLError> {
    let operation_details = format!(
        "Hamming distance computation for binary named vector: '{}', dimensions: {} x {}",
        vector_name, vector_a.len(), vector_b.len()
    );

    // TODO: Implement actual Hamming distance computation
    // distance = number of positions where bits differ
    // This would involve:
    // 1. XOR vectors element-wise
    // 2. Count number of true values
    // 3. Return count as distance

    Err(HyperQLError::ExecutionError {
        message: format!("Hamming distance not yet implemented. {}", operation_details),
        operation: "hamming_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute Jaccard distance between sparse named vectors (1 - jaccard_similarity)
pub fn jaccard_distance(
    indices_a: &[u32],
    values_a: &[f64],
    indices_b: &[u32],
    values_b: &[f64],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "Jaccard distance computation for sparse named vector: '{}', nnz: {} x {}",
        vector_name, indices_a.len(), indices_b.len()
    );

    // TODO: Implement actual Jaccard distance computation
    // jaccard_distance = 1 - jaccard_similarity
    // jaccard_similarity = |A ∩ B| / |A ∪ B|
    // This would involve:
    // 1. Compute Jaccard similarity for sparse vectors
    // 2. Return 1 - similarity for distance semantics

    Err(HyperQLError::ExecutionError {
        message: format!("Jaccard distance not yet implemented. {}", operation_details),
        operation: "jaccard_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

/// Compute ColBERT-style multi-vector distance (max-sim converted to distance)
pub fn colbert_distance(
    tokens_a: &[Vec<f64>],
    tokens_b: &[Vec<f64>],
    vector_name: &str,
) -> Result<f64, HyperQLError> {
    let operation_details = format!(
        "ColBERT distance computation for named vector: '{}', tokens: {} x {}",
        vector_name, tokens_a.len(), tokens_b.len()
    );

    // TODO: Implement actual ColBERT distance computation
    // Convert ColBERT max-sim similarity to distance measure
    // This would involve:
    // 1. Compute ColBERT similarity
    // 2. Convert to distance: distance = max_possible_sim - actual_sim
    // 3. Normalize appropriately for distance semantics

    Err(HyperQLError::ExecutionError {
        message: format!("ColBERT distance not yet implemented. {}", operation_details),
        operation: "colbert_distance".to_string(),
        entity_context: Some(format!("vector_name={}", vector_name)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_functions_return_descriptive_errors() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![0.5, 1.5, 2.5];
        let vector_name = "test_embedding";

        // Test Euclidean distance
        let result = euclidean_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Euclidean distance"));
        assert!(error_msg.contains("test_embedding"));
        assert!(error_msg.contains("not yet implemented"));

        // Test Manhattan distance
        let result = manhattan_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Manhattan distance"));
        assert!(error_msg.contains("test_embedding"));

        // Test Cosine distance
        let result = cosine_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cosine distance"));
        assert!(error_msg.contains("test_embedding"));
    }

    #[test]
    fn test_minkowski_distance_with_parameter() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![0.5, 1.5, 2.5];
        let vector_name = "test_embedding";
        let p = 3.0;

        let result = minkowski_distance(&vec_a, &vec_b, p, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Minkowski distance"));
        assert!(error_msg.contains("p=3"));
        assert!(error_msg.contains("test_embedding"));
    }

    #[test]
    fn test_hamming_distance() {
        let vec_a = vec![true, false, true, false];
        let vec_b = vec![true, true, false, false];
        let vector_name = "binary_features";

        let result = hamming_distance(&vec_a, &vec_b, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Hamming distance"));
        assert!(error_msg.contains("binary_features"));
    }

    #[test]
    fn test_sparse_vector_distance() {
        let indices_a = vec![0, 2, 5];
        let values_a = vec![1.0, 0.5, 2.0];
        let indices_b = vec![1, 2, 3, 5];
        let values_b = vec![0.8, 0.6, 1.2, 1.8];
        let vector_name = "sparse_keywords";

        let result = jaccard_distance(&indices_a, &values_a, &indices_b, &values_b, vector_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Jaccard distance"));
        assert!(error_msg.contains("sparse_keywords"));
        assert!(error_msg.contains("nnz: 3 x 4"));
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
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("ColBERT distance"));
        assert!(error_msg.contains("colbert_tokens"));
        assert!(error_msg.contains("tokens: 2 x 2"));
    }
}