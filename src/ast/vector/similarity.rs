//! Vector Similarity AST Nodes
//!
//! This module implements AST nodes for vector similarity operations with named embeddings.
//! Unlike geometric operations, vector operations work with user-defined named embeddings
//! that entities can have zero to many of, with different vector types (dense, sparse, ColBERT).
//!
//! ## Named Vector Concept
//!
//! Entities can have multiple named vectors:
//! - `text_embedding`: Dense vector from text encoder
//! - `code_embedding`: Dense vector from code encoder
//! - `bge_m3`: Dense vector from BGE-M3 model
//! - `keywords_sparse`: Sparse vector for keyword matching
//! - `colbert_tokens`: Multi-vector representation (ColBERT)
//!
//! ## Operation Semantics
//!
//! Vector similarity operations reference named vectors:
//!
//! ```hyperql
//! SELECT * FROM products WHERE text_embedding SIMILAR TO query_vector THRESHOLD 0.8
//! SELECT * FROM docs ORDER BY SIMILARITY(code_embedding, query_vector) LIMIT 10
//! ```

use serde::{Deserialize, Serialize};
use crate::ast::Expression;
use crate::HyperQLError;

/// Vector similarity expression AST node for named embeddings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarityExpressionNode {
    /// Name of the vector field (user-defined, e.g., "text_embedding", "code_vec")
    pub vector_name: String,
    /// Reference vector to compare against
    pub reference: Box<Expression>,
    /// Similarity metric to use
    pub metric: SimilarityMetric,
    /// Optional similarity threshold for filtering
    pub threshold: Option<f64>,
    /// Vector type specification
    pub vector_type: VectorType,
}

/// Similarity metric specification for named vectors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimilarityMetric {
    /// Cosine similarity: cos(θ) = ⟨a,b⟩/(||a|| ||b||)
    Cosine,
    /// Dot product similarity: ⟨a,b⟩ = Σᵢ aᵢbᵢ
    DotProduct,
    /// Euclidean distance (inverted for similarity): 1/(1 + ||a-b||₂)
    Euclidean,
    /// Manhattan distance (inverted): 1/(1 + ||a-b||₁)
    Manhattan,
    /// Jaccard similarity for sparse/binary vectors
    Jaccard,
    /// Custom user-defined similarity metric
    Custom(String),
}

/// Vector type specification for different embedding types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VectorType {
    /// Dense vector with fixed dimensions
    Dense { dimensions: u32 },
    /// Sparse vector with non-zero indices and values
    Sparse { max_dimensions: Option<u32> },
    /// ColBERT multi-vector representation
    ColBERT {
        token_dimensions: u32,
        max_tokens: Option<u32>,
    },
}

/// Similarity threshold constraint for filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarityThreshold {
    /// Minimum similarity score (inclusive)
    pub min_score: Option<f64>,
    /// Maximum similarity score (inclusive)
    pub max_score: Option<f64>,
    /// Whether to use approximate comparison (for performance)
    pub approximate: bool,
}

/// Vector reference in expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedVectorRef {
    /// Table/entity reference (optional)
    pub table: Option<String>,
    /// Name of the vector field
    pub vector_name: String,
}

impl SimilarityExpressionNode {
    /// Create a new similarity expression for a named vector
    pub fn new(
        vector_name: String,
        reference: Expression,
        metric: SimilarityMetric,
        vector_type: VectorType,
    ) -> Self {
        Self {
            vector_name,
            reference: Box::new(reference),
            metric,
            threshold: None,
            vector_type,
        }
    }

    /// Create a new similarity expression with threshold
    pub fn new_with_threshold(
        vector_name: String,
        reference: Expression,
        metric: SimilarityMetric,
        vector_type: VectorType,
        threshold: SimilarityThreshold,
    ) -> Self {
        Self {
            vector_name,
            reference: Box::new(reference),
            metric,
            threshold: threshold.min_score,
            vector_type,
        }
    }

    /// Set the similarity threshold for filtering
    pub fn set_threshold(&mut self, threshold: SimilarityThreshold) -> Result<(), HyperQLError> {
        if let Some(min_score) = threshold.min_score {
            if !(-1.0..=1.0).contains(&min_score) {
                return Err(HyperQLError::ValidationError {
                    message: "Similarity threshold must be between -1.0 and 1.0".to_string(),
                    field: Some("min_score".to_string()),
                });
            }
            self.threshold = Some(min_score);
        }

        if let Some(max_score) = threshold.max_score {
            if max_score < -1.0 || max_score > 1.0 {
                return Err(HyperQLError::ValidationError {
                    message: "Similarity threshold must be between -1.0 and 1.0".to_string(),
                    field: Some("max_score".to_string()),
                });
            }
        }

        Ok(())
    }

    /// Validate the similarity expression
    pub fn validate(&self) -> Result<(), HyperQLError> {
        if self.vector_name.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Vector name cannot be empty".to_string(),
                field: Some("vector_name".to_string()),
            });
        }

        // Validate metric compatibility with vector type
        match (&self.metric, &self.vector_type) {
            (SimilarityMetric::Jaccard, VectorType::Dense { .. }) => {
                return Err(HyperQLError::ValidationError {
                    message: "Jaccard similarity not supported for dense vectors".to_string(),
                    field: Some("metric".to_string()),
                });
            }
            (SimilarityMetric::Cosine, VectorType::ColBERT { .. }) => {
                return Err(HyperQLError::ValidationError {
                    message: "Cosine similarity requires special handling for ColBERT vectors".to_string(),
                    field: Some("metric".to_string()),
                });
            }
            _ => {} // Other combinations are valid
        }

        // Validate threshold if present
        if let Some(threshold) = self.threshold {
            if threshold < -1.0 || threshold > 1.0 {
                return Err(HyperQLError::ValidationError {
                    message: "Similarity threshold must be between -1.0 and 1.0".to_string(),
                    field: Some("threshold".to_string()),
                });
            }
        }

        Ok(())
    }

    /// Get the expected output type for this similarity operation
    pub fn output_type(&self) -> &'static str {
        match self.metric {
            SimilarityMetric::Cosine |
            SimilarityMetric::DotProduct |
            SimilarityMetric::Jaccard => "similarity_score",
            SimilarityMetric::Euclidean |
            SimilarityMetric::Manhattan => "distance_score",
            SimilarityMetric::Custom(_) => "custom_score",
        }
    }

    /// Estimate the computational cost of this similarity operation
    pub fn estimate_cost(&self) -> f64 {
        let base_cost = match self.metric {
            SimilarityMetric::Cosine => 1.5, // Normalization overhead
            SimilarityMetric::DotProduct => 1.0,
            SimilarityMetric::Euclidean => 1.2,
            SimilarityMetric::Manhattan => 1.1,
            SimilarityMetric::Jaccard => 2.0, // Sparse vector overhead
            SimilarityMetric::Custom(_) => 3.0, // User function overhead
        };

        let vector_type_factor = match self.vector_type {
            VectorType::Dense { dimensions } => (dimensions as f64).log2() / 10.0,
            VectorType::Sparse { .. } => 1.5, // Sparse vector overhead
            VectorType::ColBERT { token_dimensions, max_tokens } => {
                let tokens = max_tokens.unwrap_or(128) as f64;
                let dims = token_dimensions as f64;
                (tokens * dims).log2() / 8.0
            }
        };

        base_cost * (1.0 + vector_type_factor)
    }
}

impl Default for SimilarityThreshold {
    fn default() -> Self {
        Self {
            min_score: None,
            max_score: None,
            approximate: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Expression};

    #[test]
    fn test_similarity_expression_creation() {
        let vector_name = "text_embedding".to_string();
        let reference = Expression::Literal(Literal::String("query_vector".to_string()));
        let metric = SimilarityMetric::Cosine;
        let vector_type = VectorType::Dense { dimensions: 768 };

        let expr = SimilarityExpressionNode::new(vector_name.clone(), reference, metric, vector_type);

        assert_eq!(expr.vector_name, vector_name);
        assert!(matches!(expr.metric, SimilarityMetric::Cosine));
        assert!(expr.threshold.is_none());
    }

    #[test]
    fn test_similarity_with_threshold() {
        let vector_name = "code_embedding".to_string();
        let reference = Expression::Literal(Literal::String("target_vec".to_string()));
        let metric = SimilarityMetric::DotProduct;
        let vector_type = VectorType::Dense { dimensions: 512 };
        let threshold = SimilarityThreshold {
            min_score: Some(0.8),
            max_score: None,
            approximate: false,
        };

        let expr = SimilarityExpressionNode::new_with_threshold(
            vector_name, reference, metric, vector_type, threshold
        );

        assert_eq!(expr.threshold, Some(0.8));
    }

    #[test]
    fn test_validation_empty_vector_name() {
        let expr = SimilarityExpressionNode {
            vector_name: "".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("test".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: None,
            vector_type: VectorType::Dense { dimensions: 100 },
        };

        assert!(expr.validate().is_err());
    }

    #[test]
    fn test_validation_metric_vector_type_compatibility() {
        // Jaccard with dense should fail
        let expr1 = SimilarityExpressionNode {
            vector_name: "test_vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("ref".to_string()))),
            metric: SimilarityMetric::Jaccard,
            threshold: None,
            vector_type: VectorType::Dense { dimensions: 100 },
        };
        assert!(expr1.validate().is_err());

        // Cosine with ColBERT should fail (needs special handling)
        let expr2 = SimilarityExpressionNode {
            vector_name: "colbert_vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("ref".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: None,
            vector_type: VectorType::ColBERT { token_dimensions: 128, max_tokens: Some(64) },
        };
        assert!(expr2.validate().is_err());
    }

    #[test]
    fn test_validation_threshold_range() {
        let mut expr = SimilarityExpressionNode {
            vector_name: "test_vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("ref".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: Some(1.5), // Invalid: > 1.0
            vector_type: VectorType::Dense { dimensions: 100 },
        };
        assert!(expr.validate().is_err());

        expr.threshold = Some(0.8); // Valid
        assert!(expr.validate().is_ok());
    }

    #[test]
    fn test_output_type() {
        let cosine_expr = SimilarityExpressionNode::new(
            "vec1".to_string(),
            Expression::Literal(Literal::String("ref".to_string())),
            SimilarityMetric::Cosine,
            VectorType::Dense { dimensions: 100 },
        );
        assert_eq!(cosine_expr.output_type(), "similarity_score");

        let euclidean_expr = SimilarityExpressionNode::new(
            "vec2".to_string(),
            Expression::Literal(Literal::String("ref".to_string())),
            SimilarityMetric::Euclidean,
            VectorType::Dense { dimensions: 100 },
        );
        assert_eq!(euclidean_expr.output_type(), "distance_score");
    }

    #[test]
    fn test_cost_estimation() {
        let dense_expr = SimilarityExpressionNode::new(
            "dense_vec".to_string(),
            Expression::Literal(Literal::String("ref".to_string())),
            SimilarityMetric::Cosine,
            VectorType::Dense { dimensions: 768 },
        );

        let sparse_expr = SimilarityExpressionNode::new(
            "sparse_vec".to_string(),
            Expression::Literal(Literal::String("ref".to_string())),
            SimilarityMetric::Jaccard,
            VectorType::Sparse { max_dimensions: Some(10000) },
        );

        let dense_cost = dense_expr.estimate_cost();
        let sparse_cost = sparse_expr.estimate_cost();

        assert!(dense_cost > 0.0);
        assert!(sparse_cost > dense_cost); // Sparse should be more expensive due to Jaccard
    }
}