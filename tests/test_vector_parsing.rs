//! Integration tests for vector query parsing
//!
//! These tests verify that vector expressions work correctly within complete SQL queries.

use hyperQL::parser::parse_statement;
use hyperQL::ast::*;
use hyperQL::ast::vector::similarity::SimilarityMetric;

#[test]
fn test_select_with_similarity_in_where() {
    let query = "SELECT * FROM documents WHERE SIMILARITY(text_embedding, query_vec, 'cosine') > 0.8";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert!(select.where_clause.is_some());

            // Verify the WHERE clause contains a binary comparison with similarity
            match &select.where_clause.unwrap() {
                Expression::Binary { left, op, .. } => {
                    match op {
                        BinaryOperator::GreaterThan => {},
                        _ => panic!("Expected GreaterThan operator"),
                    }

                    match left.as_ref() {
                        Expression::Vector(VectorExpression::Similarity { vector_name, metric, .. }) => {
                            assert_eq!(vector_name, "text_embedding");
                            assert!(matches!(metric, SimilarityMetric::Cosine));
                        }
                        _ => panic!("Expected Vector::Similarity in WHERE clause"),
                    }
                }
                _ => panic!("Expected Binary expression in WHERE clause"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_select_with_distance_in_where() {
    let query = "SELECT * FROM products WHERE DISTANCE(feature_vec, target, 'euclidean') < 2.0";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert!(select.where_clause.is_some());

            match &select.where_clause.unwrap() {
                Expression::Binary { left, op, .. } => {
                    match op {
                        BinaryOperator::LessThan => {},
                        _ => panic!("Expected LessThan operator"),
                    }

                    match left.as_ref() {
                        Expression::Vector(VectorExpression::Similarity { vector_name, metric, .. }) => {
                            assert_eq!(vector_name, "feature_vec");
                            assert!(matches!(metric, SimilarityMetric::Euclidean));
                        }
                        _ => panic!("Expected Vector::Similarity expression"),
                    }
                }
                _ => panic!("Expected Binary expression"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_select_with_similar_to_operator() {
    let query = "SELECT * FROM articles WHERE text_embedding SIMILAR TO query_vec THRESHOLD 0.75";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert!(select.where_clause.is_some());

            match &select.where_clause.unwrap() {
                Expression::Vector(VectorExpression::Similarity {
                    vector_name,
                    threshold,
                    metric,
                    ..
                }) => {
                    assert_eq!(vector_name, "text_embedding");
                    assert_eq!(threshold, &Some(0.75));
                    assert!(matches!(metric, SimilarityMetric::Cosine));
                }
                _ => panic!("Expected Vector::Similarity expression with threshold"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_order_by_similarity_desc() {
    let query = "SELECT * FROM docs ORDER BY SIMILARITY(embedding, query, 'cosine') DESC LIMIT 10";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert_eq!(select.order_by.len(), 1);

            let order_item = &select.order_by[0];
            assert!(matches!(order_item.direction, OrderDirection::Desc));

            match &order_item.expr {
                Expression::Vector(VectorExpression::Similarity { vector_name, metric, .. }) => {
                    assert_eq!(vector_name, "embedding");
                    assert!(matches!(metric, SimilarityMetric::Cosine));
                }
                _ => panic!("Expected Vector::Similarity in ORDER BY"),
            }

            assert_eq!(select.limit, Some(10));
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_order_by_distance_asc() {
    let query = "SELECT * FROM items ORDER BY DISTANCE(vec, target, 'euclidean') ASC LIMIT 5";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert_eq!(select.order_by.len(), 1);

            let order_item = &select.order_by[0];
            assert!(matches!(order_item.direction, OrderDirection::Asc));

            match &order_item.expr {
                Expression::Vector(VectorExpression::Similarity { vector_name, metric, .. }) => {
                    assert_eq!(vector_name, "vec");
                    assert!(matches!(metric, SimilarityMetric::Euclidean));
                }
                _ => panic!("Expected Vector::Similarity in ORDER BY"),
            }

            assert_eq!(select.limit, Some(5));
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_order_by_similarity_default_asc() {
    let query = "SELECT * FROM docs ORDER BY SIMILARITY(embedding, query, 'dotproduct')";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert_eq!(select.order_by.len(), 1);

            let order_item = &select.order_by[0];
            assert!(matches!(order_item.direction, OrderDirection::Asc));

            match &order_item.expr {
                Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                    assert!(matches!(metric, SimilarityMetric::DotProduct));
                }
                _ => panic!("Expected Vector::Similarity in ORDER BY"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_knn_query_with_cosine() {
    let query = "SELECT id, title FROM articles ORDER BY SIMILARITY(text_embedding, query_vector, 'cosine') DESC LIMIT 20";
    let result = parse_statement(query);

    assert!(result.is_ok(), "KNN query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert_eq!(select.select_list.len(), 2);
            assert_eq!(select.order_by.len(), 1);
            assert_eq!(select.limit, Some(20));

            match &select.order_by[0].expr {
                Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                    assert!(matches!(metric, SimilarityMetric::Cosine));
                }
                _ => panic!("Expected Vector::Similarity"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_combined_vector_and_scalar_filters() {
    let query = "SELECT * FROM products WHERE category = 'electronics' AND SIMILARITY(feature_vec, target, 'cosine') > 0.7";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Combined query should parse successfully: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert!(select.where_clause.is_some());

            match &select.where_clause.unwrap() {
                Expression::Binary { left, op, right } => {
                    assert!(matches!(op, BinaryOperator::And));

                    // Left should be category comparison
                    match left.as_ref() {
                        Expression::Binary { op, .. } => {
                            assert!(matches!(op, BinaryOperator::Equal));
                        }
                        _ => panic!("Expected equality comparison"),
                    }

                    // Right should be similarity comparison
                    match right.as_ref() {
                        Expression::Binary { left, op, .. } => {
                            assert!(matches!(op, BinaryOperator::GreaterThan));
                            match left.as_ref() {
                                Expression::Vector(_) => {},
                                _ => panic!("Expected Vector expression"),
                            }
                        }
                        _ => panic!("Expected similarity comparison"),
                    }
                }
                _ => panic!("Expected AND expression"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_similarity_with_all_metrics() {
    let metrics = vec![
        ("'cosine'", SimilarityMetric::Cosine),
        ("'euclidean'", SimilarityMetric::Euclidean),
        ("'manhattan'", SimilarityMetric::Manhattan),
        ("'jaccard'", SimilarityMetric::Jaccard),
        ("'dotproduct'", SimilarityMetric::DotProduct),
        ("'dot'", SimilarityMetric::DotProduct),
    ];

    for (metric_str, expected_metric) in metrics {
        let query = format!("SELECT * FROM docs WHERE SIMILARITY(vec, ref, {}) > 0.5", metric_str);
        let result = parse_statement(&query);

        assert!(result.is_ok(), "Query with {} should parse: {:?}", metric_str, result);

        let stmt = result.unwrap();
        match stmt {
            Statement::Select(select) => {
                match &select.where_clause.unwrap() {
                    Expression::Binary { left, .. } => {
                        match left.as_ref() {
                            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                                assert_eq!(
                                    std::mem::discriminant(metric),
                                    std::mem::discriminant(&expected_metric),
                                    "Metric mismatch for {}",
                                    metric_str
                                );
                            }
                            _ => panic!("Expected Vector::Similarity"),
                        }
                    }
                    _ => panic!("Expected Binary expression"),
                }
            }
            _ => panic!("Expected SELECT statement"),
        }
    }
}

#[test]
fn test_custom_metric() {
    let query = "SELECT * FROM docs WHERE SIMILARITY(vec, ref, 'my_custom_metric') > 0.5";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Custom metric query should parse: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            match &select.where_clause.unwrap() {
                Expression::Binary { left, .. } => {
                    match left.as_ref() {
                        Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                            match metric {
                                SimilarityMetric::Custom(name) => {
                                    assert_eq!(name, "my_custom_metric");
                                }
                                _ => panic!("Expected Custom metric"),
                            }
                        }
                        _ => panic!("Expected Vector::Similarity"),
                    }
                }
                _ => panic!("Expected Binary expression"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_vector_expression_in_select_list() {
    let query = "SELECT id, SIMILARITY(embedding, query, 'cosine') as score FROM docs";
    let result = parse_statement(query);

    assert!(result.is_ok(), "Query with SIMILARITY in SELECT should parse: {:?}", result);

    let stmt = result.unwrap();
    match stmt {
        Statement::Select(select) => {
            assert_eq!(select.select_list.len(), 2);

            match &select.select_list[1] {
                SelectItem::Expression { expr, alias } => {
                    assert_eq!(alias.as_ref().unwrap(), "score");

                    match expr {
                        Expression::Vector(VectorExpression::Similarity { .. }) => {},
                        _ => panic!("Expected Vector::Similarity in SELECT list"),
                    }
                }
                _ => panic!("Expected Expression in SELECT list"),
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}
