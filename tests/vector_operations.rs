//! Integration Tests for Vector Operations with Named Embeddings
//!
//! This module tests the vector operations (SIMILAR TO, SIMILARITY function, k-NN)
//! that work with user-defined named embeddings. These are separate from geometric
//! operations and work with named vector fields that entities can have multiple of.

use hyperQL::*;
use hyperQL::ast::vector::similarity::*;
use hyperQL::ast::vector::knn::*;
use hyperQL::ast::{Expression, VectorExpression};
use hyperQL::types::Value;
use std::collections::HashMap;

#[test]
fn test_similarity_expression_creation() {
    let vector_name = "text_embedding".to_string();
    let reference = Expression::Literal(ast::Literal::String("query_vector".to_string()));
    let metric = SimilarityMetric::Cosine;
    let vector_type = VectorType::Dense { dimensions: 768 };
    
    let similarity_expr = SimilarityExpressionNode::new(
        vector_name.clone(),
        reference,
        metric,
        vector_type,
    );
    
    assert_eq!(similarity_expr.vector_name, vector_name);
    assert!(matches!(similarity_expr.metric, SimilarityMetric::Cosine));
    assert!(similarity_expr.threshold.is_none());
    assert!(matches!(similarity_expr.vector_type, VectorType::Dense { dimensions: 768 }));
}

#[test]
fn test_similarity_with_threshold() {
    let vector_name = "code_embedding".to_string();
    let reference = Expression::Literal(ast::Literal::String("target_vec".to_string()));
    let metric = SimilarityMetric::DotProduct;
    let vector_type = VectorType::Dense { dimensions: 512 };
    let threshold = SimilarityThreshold {
        min_score: Some(0.8),
        max_score: None,
        approximate: false,
    };
    
    let similarity_expr = SimilarityExpressionNode::new_with_threshold(
        vector_name,
        reference,
        metric,
        vector_type,
        threshold,
    );
    
    assert_eq!(similarity_expr.threshold, Some(0.8));
    assert!(matches!(similarity_expr.metric, SimilarityMetric::DotProduct));
}

#[test]
fn test_vector_type_specifications() {
    let reference = Expression::Literal(ast::Literal::String("ref".to_string()));
    
    // Dense vector
    let dense_expr = SimilarityExpressionNode::new(
        "dense_vec".to_string(),
        reference.clone(),
        SimilarityMetric::Cosine,
        VectorType::Dense { dimensions: 1024 },
    );
    assert!(matches!(dense_expr.vector_type, VectorType::Dense { dimensions: 1024 }));
    
    // Sparse vector
    let sparse_expr = SimilarityExpressionNode::new(
        "sparse_vec".to_string(),
        reference.clone(),
        SimilarityMetric::Jaccard,
        VectorType::Sparse { max_dimensions: Some(10000) },
    );
    assert!(matches!(sparse_expr.vector_type, VectorType::Sparse { max_dimensions: Some(10000) }));
    
    // ColBERT vector
    let colbert_expr = SimilarityExpressionNode::new(
        "colbert_tokens".to_string(),
        reference,
        SimilarityMetric::DotProduct,
        VectorType::ColBERT {
            token_dimensions: 128,
            max_tokens: Some(64),
        },
    );
    assert!(matches!(colbert_expr.vector_type, VectorType::ColBERT { token_dimensions: 128, max_tokens: Some(64) }));
}

#[test]
fn test_similarity_validation() {
    // Test empty vector name validation
    let invalid_expr = SimilarityExpressionNode {
        vector_name: "".to_string(),
        reference: Box::new(Expression::Literal(ast::Literal::String("test".to_string()))),
        metric: SimilarityMetric::Cosine,
        threshold: None,
        vector_type: VectorType::Dense { dimensions: 100 },
    };
    
    let result = invalid_expr.validate();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Vector name cannot be empty"));
    
    // Test metric-vector type compatibility
    let incompatible_expr = SimilarityExpressionNode {
        vector_name: "test_vec".to_string(),
        reference: Box::new(Expression::Literal(ast::Literal::String("ref".to_string()))),
        metric: SimilarityMetric::Jaccard,
        threshold: None,
        vector_type: VectorType::Dense { dimensions: 100 },
    };
    
    let result = incompatible_expr.validate();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Jaccard similarity not supported for dense vectors"));
}

#[test]
fn test_knn_query_creation() {
    let vector_name = "text_embedding".to_string();
    let reference = Expression::Literal(ast::Literal::String("query_vec".to_string()));
    let k = 10;
    let metric = SimilarityMetric::Cosine;
    let vector_type = VectorType::Dense { dimensions: 768 };
    
    let knn_query = KNNQueryNode::new(vector_name.clone(), reference, k, metric, vector_type);
    
    assert_eq!(knn_query.vector_name, vector_name);
    assert_eq!(knn_query.k, k);
    assert!(matches!(knn_query.metric, SimilarityMetric::Cosine));
    assert!(knn_query.diversity_constraints.is_empty());
}

#[test]
fn test_knn_with_diversity_constraints() {
    let mut knn_query = KNNQueryNode::new(
        "vec".to_string(),
        Expression::Literal(ast::Literal::String("ref".to_string())),
        10,
        SimilarityMetric::Cosine,
        VectorType::Dense { dimensions: 100 },
    );
    
    let constraint = DiversityConstraint {
        constraint_type: DiversityType::Spatial,
        diversity_field: "position".to_string(),
        min_distance: 1.0,
    };
    
    assert!(knn_query.add_diversity_constraint(constraint).is_ok());
    assert_eq!(knn_query.diversity_constraints.len(), 1);
    
    // Test invalid constraint
    let invalid_constraint = DiversityConstraint {
        constraint_type: DiversityType::Feature,
        diversity_field: "".to_string(), // Empty field name
        min_distance: 0.5,
    };
    
    let result = knn_query.add_diversity_constraint(invalid_constraint);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Diversity field name cannot be empty"));
}

#[test]
fn test_knn_validation() {
    // Test k = 0 validation
    let invalid_knn = KNNQueryNode {
        vector_name: "vec".to_string(),
        reference: Box::new(Expression::Literal(ast::Literal::String("ref".to_string()))),
        k: 0, // Invalid
        metric: SimilarityMetric::Cosine,
        vector_type: VectorType::Dense { dimensions: 100 },
        config: KNNConfig::default(),
        diversity_constraints: Vec::new(),
    };
    
    let result = invalid_knn.validate();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("k must be greater than 0"));
    
    // Test k too large validation
    let oversized_knn = KNNQueryNode {
        vector_name: "vec".to_string(),
        reference: Box::new(Expression::Literal(ast::Literal::String("ref".to_string()))),
        k: 20000, // Invalid: > 10,000
        metric: SimilarityMetric::Cosine,
        vector_type: VectorType::Dense { dimensions: 100 },
        config: KNNConfig::default(),
        diversity_constraints: Vec::new(),
    };
    
    let result = oversized_knn.validate();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("k cannot exceed 10,000"));
}

#[test]
fn test_knn_ordering_determination() {
    let reference = Expression::Literal(ast::Literal::String("ref".to_string()));
    let vector_type = VectorType::Dense { dimensions: 100 };
    
    // Cosine similarity should order by similarity (descending)
    let cosine_knn = KNNQueryNode::new(
        "vec1".to_string(),
        reference.clone(),
        10,
        SimilarityMetric::Cosine,
        vector_type.clone(),
    );
    assert!(matches!(cosine_knn.get_ordering(), KNNOrdering::BySimilarity));
    
    // Euclidean distance should order by distance (ascending)
    let euclidean_knn = KNNQueryNode::new(
        "vec2".to_string(),
        reference,
        10,
        SimilarityMetric::Euclidean,
        vector_type,
    );
    assert!(matches!(euclidean_knn.get_ordering(), KNNOrdering::ByDistance));
}

#[test]
#[ignore] // TODO: Re-enable when functions module is implemented
fn test_vector_functions_integration() {
    /* TODO: Re-enable when functions module is implemented
    use hyperQL::functions::vector::similarity::*;
    use hyperQL::functions::vector::distance::*;

    let vec_a = vec![1.0, 2.0, 3.0];
    let vec_b = vec![0.5, 1.5, 2.5];
    let vector_name = "test_embedding";

    // Test cosine similarity function
    let cosine_result = cosine_similarity(&vec_a, &vec_b, vector_name);
    assert!(cosine_result.is_ok());
    let similarity = cosine_result.unwrap();
    assert!(similarity > 0.0 && similarity <= 1.0);

    // Test Euclidean distance function
    let euclidean_result = euclidean_distance(&vec_a, &vec_b, vector_name);
    assert!(euclidean_result.is_ok());
    let distance = euclidean_result.unwrap();
    assert!(distance > 0.0);

    // Test sparse vector functions
    let indices_a = vec![0, 2, 5];
    let values_a = vec![1.0, 0.5, 2.0];
    let indices_b = vec![1, 2, 3, 5];
    let values_b = vec![0.8, 0.6, 1.2, 1.8];
    let sparse_vector_name = "sparse_keywords";

    let jaccard_result = jaccard_similarity(&indices_a, &values_a, &indices_b, &values_b, sparse_vector_name);
    assert!(jaccard_result.is_ok());
    let jaccard_sim = jaccard_result.unwrap();
    assert!(jaccard_sim >= 0.0 && jaccard_sim <= 1.0);
    */
}

#[test]
#[ignore] // TODO: Re-enable when functions module is implemented
fn test_colbert_vector_operations() {
    /* TODO: Re-enable when functions module is implemented
    use hyperQL::functions::vector::similarity::colbert_similarity;
    use hyperQL::functions::vector::distance::colbert_distance;

    let tokens_a = vec![
        vec![0.1, 0.2, 0.3],
        vec![0.4, 0.5, 0.6],
    ];
    let tokens_b = vec![
        vec![0.15, 0.25, 0.35],
        vec![0.45, 0.55, 0.65],
    ];
    let vector_name = "colbert_tokens";

    // Test ColBERT similarity
    let similarity_result = colbert_similarity(&tokens_a, &tokens_b, vector_name);
    assert!(similarity_result.is_ok());
    let similarity = similarity_result.unwrap();
    assert!(similarity >= -1.0 && similarity <= 1.0);

    // Test ColBERT distance
    let distance_result = colbert_distance(&tokens_a, &tokens_b, vector_name);
    assert!(distance_result.is_ok());
    let distance = distance_result.unwrap();
    assert!(distance >= 0.0 && distance <= 2.0);
    */
}

#[test]
fn test_vector_expression_in_main_ast() {
    // Test that vector expressions integrate with the main Expression enum
    let similarity_vector = VectorExpression::Similarity {
        vector_name: "text_embedding".to_string(),
        reference: Box::new(Expression::Literal(ast::Literal::String("query_vec".to_string()))),
        metric: SimilarityMetric::Cosine,
        threshold: Some(0.8),
        vector_type: VectorType::Dense { dimensions: 512 },
    };
    
    let vector_expr = Expression::Vector(similarity_vector);
    
    match vector_expr {
        Expression::Vector(VectorExpression::Similarity { vector_name, threshold, .. }) => {
            assert_eq!(vector_name, "text_embedding");
            assert_eq!(threshold, Some(0.8));
        }
        _ => panic!("Expected vector similarity expression"),
    }
    
    // Test KNN vector expression
    let knn_vector = VectorExpression::KNN {
        vector_name: "code_embedding".to_string(),
        reference: Box::new(Expression::Literal(ast::Literal::String("target_vec".to_string()))),
        k: 5,
        metric: SimilarityMetric::DotProduct,
        vector_type: VectorType::Dense { dimensions: 256 },
    };
    
    let knn_expr = Expression::Vector(knn_vector);
    
    match knn_expr {
        Expression::Vector(VectorExpression::KNN { vector_name, k, .. }) => {
            assert_eq!(vector_name, "code_embedding");
            assert_eq!(k, 5);
        }
        _ => panic!("Expected vector KNN expression"),
    }
}

#[test]
fn test_cost_estimation() {
    let reference = Expression::Literal(ast::Literal::String("ref".to_string()));
    
    // Test similarity expression cost estimation
    let dense_similarity = SimilarityExpressionNode::new(
        "dense_vec".to_string(),
        reference.clone(),
        SimilarityMetric::Cosine,
        VectorType::Dense { dimensions: 768 },
    );
    
    let sparse_similarity = SimilarityExpressionNode::new(
        "sparse_vec".to_string(),
        reference.clone(),
        SimilarityMetric::Jaccard,
        VectorType::Sparse { max_dimensions: Some(10000) },
    );
    
    let dense_cost = dense_similarity.estimate_cost();
    let sparse_cost = sparse_similarity.estimate_cost();
    
    assert!(dense_cost > 0.0);
    assert!(sparse_cost > 0.0);
    assert!(sparse_cost > dense_cost); // Sparse should be more expensive due to Jaccard
    
    // Test KNN cost estimation
    let small_knn = KNNQueryNode::new(
        "vec1".to_string(),
        reference.clone(),
        5,
        SimilarityMetric::Cosine,
        VectorType::Dense { dimensions: 100 },
    );
    
    let large_knn = KNNQueryNode::new(
        "vec2".to_string(),
        reference,
        50,
        SimilarityMetric::Cosine,
        VectorType::Dense { dimensions: 100 },
    );
    
    let small_knn_cost = small_knn.estimate_cost();
    let large_knn_cost = large_knn.estimate_cost();
    
    assert!(small_knn_cost > 0.0);
    assert!(large_knn_cost > small_knn_cost); // Larger k should be more expensive
}

#[test]
fn test_named_vector_concepts() {
    // Test that vector operations work with user-defined names
    let user_defined_names = vec![
        "text_embedding",
        "code_vec", 
        "bge_m3",
        "sparse_keywords",
        "colbert_tokens",
        "custom_embedding_v2",
    ];
    
    for name in user_defined_names {
        let similarity_expr = SimilarityExpressionNode::new(
            name.to_string(),
            Expression::Literal(ast::Literal::String("query".to_string())),
            SimilarityMetric::Cosine,
            VectorType::Dense { dimensions: 512 },
        );
        
        assert_eq!(similarity_expr.vector_name, name);
        assert!(similarity_expr.validate().is_ok());
        
        let knn_query = KNNQueryNode::new(
            name.to_string(),
            Expression::Literal(ast::Literal::String("reference".to_string())),
            10,
            SimilarityMetric::Cosine,
            VectorType::Dense { dimensions: 512 },
        );
        
        assert_eq!(knn_query.vector_name, name);
        assert!(knn_query.validate().is_ok());
    }
}

#[test]
#[ignore] // TODO: Re-enable when functions module is implemented
fn test_threshold_operations() {
    /* TODO: Re-enable when functions module is implemented
    use hyperQL::functions::vector::similarity::apply_similarity_threshold;

    let vector_name = "test_vec";

    // Test threshold filtering - score below threshold
    let result = apply_similarity_threshold(0.75, Some(0.8), vector_name);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);  // Score 0.75 < threshold 0.8

    // Test threshold filtering - score above threshold
    let result_pass = apply_similarity_threshold(0.85, Some(0.8), vector_name);
    assert!(result_pass.is_ok());
    assert_eq!(result_pass.unwrap(), true);  // Score 0.85 > threshold 0.8

    // Test no threshold case
    let result_no_threshold = apply_similarity_threshold(0.75, None, vector_name);
    assert!(result_no_threshold.is_ok());
    assert_eq!(result_no_threshold.unwrap(), true);  // Always passes when no threshold
    */
}