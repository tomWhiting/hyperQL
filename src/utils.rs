use crate::ast::{
    Statement, SelectStatement, SelectItem, FromClause, Expression, Literal, ColumnRef,
    BinaryOperator, VectorExpression, TraverseClause, TraversePattern, NodePattern,
    RelationshipPattern, RelationshipDirection
};
use crate::ast::vector::similarity::{SimilarityMetric, VectorType};
use crate::ast::geometric::GeometricExpression;
use crate::builder::HyperQLBuilder;
use crate::error::{HyperQLError, Result};
use crate::types::Value;

pub mod query_utils {
    use super::*;

    pub fn entity_by_label(label: &str) -> Result<Statement> {
        if label.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Label cannot be empty".to_string(),
                field: Some("label".to_string()),
            });
        }

        HyperQLBuilder::new()
            .select(["*"])
            .from("entities")
            .where_eq("label", Value::String(label.to_string()))
            .build()
    }

    pub fn entities_near_point(x: f64, y: f64, z: f64, radius: f64) -> Result<Statement> {
        if radius <= 0.0 {
            return Err(HyperQLError::ValidationError {
                message: "Radius must be greater than 0".to_string(),
                field: Some("radius".to_string()),
            });
        }

        if !radius.is_finite() {
            return Err(HyperQLError::ValidationError {
                message: "Radius must be a finite number".to_string(),
                field: Some("radius".to_string()),
            });
        }

        let reference_position = Expression::Function {
            name: "POSITION".to_string(),
            args: vec![
                Expression::Literal(Literal::Float(x)),
                Expression::Literal(Literal::Float(y)),
                Expression::Literal(Literal::Float(z)),
            ],
        };

        let within_expr = Expression::Geometric(GeometricExpression::Within {
            target: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: "position".to_string(),
            })),
            radius,
            reference: Box::new(reference_position),
        });

        HyperQLBuilder::new()
            .select(["*"])
            .from("entities")
            .where_expr(within_expr)
            .build()
    }

    pub fn entities_within_radius(
        center_col: &str,
        x: f64,
        y: f64,
        z: f64,
        radius: f64,
    ) -> Result<Statement> {
        if center_col.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Center column name cannot be empty".to_string(),
                field: Some("center_col".to_string()),
            });
        }

        if radius <= 0.0 {
            return Err(HyperQLError::ValidationError {
                message: "Radius must be greater than 0".to_string(),
                field: Some("radius".to_string()),
            });
        }

        if !radius.is_finite() {
            return Err(HyperQLError::ValidationError {
                message: "Radius must be a finite number".to_string(),
                field: Some("radius".to_string()),
            });
        }

        let reference_position = Expression::Function {
            name: "POSITION".to_string(),
            args: vec![
                Expression::Literal(Literal::Float(x)),
                Expression::Literal(Literal::Float(y)),
                Expression::Literal(Literal::Float(z)),
            ],
        };

        let within_expr = Expression::Geometric(GeometricExpression::Within {
            target: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: center_col.to_string(),
            })),
            radius,
            reference: Box::new(reference_position),
        });

        HyperQLBuilder::new()
            .select(["*"])
            .from("entities")
            .where_expr(within_expr)
            .build()
    }

    pub fn vector_similarity_search(
        vector_col: &str,
        query_vector: Vec<f64>,
        k: usize,
    ) -> Result<Statement> {
        if vector_col.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Vector column name cannot be empty".to_string(),
                field: Some("vector_col".to_string()),
            });
        }

        if query_vector.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Query vector cannot be empty".to_string(),
                field: Some("query_vector".to_string()),
            });
        }

        if k == 0 {
            return Err(HyperQLError::ValidationError {
                message: "K must be greater than 0".to_string(),
                field: Some("k".to_string()),
            });
        }

        let dimensions = query_vector.len();
        let vector_literals: Vec<Expression> = query_vector
            .into_iter()
            .map(|v| Expression::Literal(Literal::Float(v)))
            .collect();

        let vector_expr = Expression::Function {
            name: "VECTOR".to_string(),
            args: vector_literals,
        };

        let knn_expr = Expression::Vector(VectorExpression::KNN {
            vector_name: vector_col.to_string(),
            reference: Box::new(vector_expr),
            k: k as u32,
            metric: SimilarityMetric::Cosine,
            vector_type: VectorType::Dense { dimensions: dimensions as u32 },
        });

        HyperQLBuilder::new()
            .select(["*"])
            .from("entities")
            .where_expr(knn_expr)
            .build()
    }

    pub fn traverse_relationship(
        start: &str,
        relationship: &str,
        end: Option<&str>,
    ) -> Result<Statement> {
        if start.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Start node name cannot be empty".to_string(),
                field: Some("start".to_string()),
            });
        }

        if relationship.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Relationship type cannot be empty".to_string(),
                field: Some("relationship".to_string()),
            });
        }

        let start_pattern = NodePattern {
            variable: Some(start.to_string()),
            label: None,
            properties: None,
        };

        let rel_pattern = RelationshipPattern {
            variable: None,
            rel_type: Some(relationship.to_string()),
            direction: RelationshipDirection::Outgoing,
            variable_length: None,
            optional: false,
            properties: None,
        };

        let end_pattern = NodePattern {
            variable: end.map(|s| s.to_string()),
            label: None,
            properties: None,
        };

        let traverse_pattern = TraversePattern {
            start_node: start_pattern,
            relationship: rel_pattern,
            end_node: end_pattern,
        };

        let select_stmt = SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: Some(FromClause::Table {
                name: "entities".to_string(),
                alias: Some(start.to_string()),
            }),
            traverse_clause: Some(TraverseClause {
                patterns: vec![traverse_pattern],
            }),
            where_clause: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        };

        Ok(Statement::Select(select_stmt))
    }

    pub fn similarity_search_with_threshold(
        vector_col: &str,
        query_vector: Vec<f64>,
        threshold: f64,
        metric: SimilarityMetric,
    ) -> Result<Statement> {
        if vector_col.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Vector column name cannot be empty".to_string(),
                field: Some("vector_col".to_string()),
            });
        }

        if query_vector.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Query vector cannot be empty".to_string(),
                field: Some("query_vector".to_string()),
            });
        }

        if !(0.0..=1.0).contains(&threshold) {
            return Err(HyperQLError::ValidationError {
                message: "Threshold must be between 0.0 and 1.0".to_string(),
                field: Some("threshold".to_string()),
            });
        }

        let dimensions = query_vector.len();
        let vector_literals: Vec<Expression> = query_vector
            .into_iter()
            .map(|v| Expression::Literal(Literal::Float(v)))
            .collect();

        let vector_expr = Expression::Function {
            name: "VECTOR".to_string(),
            args: vector_literals,
        };

        let similarity_expr = Expression::Vector(VectorExpression::Similarity {
            vector_name: vector_col.to_string(),
            reference: Box::new(vector_expr),
            metric,
            threshold: Some(threshold),
            vector_type: VectorType::Dense { dimensions: dimensions as u32 },
        });

        HyperQLBuilder::new()
            .select(["*"])
            .from("entities")
            .where_expr(similarity_expr)
            .build()
    }

    pub fn entities_by_property_range<T>(
        table: &str,
        property: &str,
        min_value: T,
        max_value: T,
    ) -> Result<Statement>
    where
        T: Into<Value>,
    {
        if table.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Table name cannot be empty".to_string(),
                field: Some("table".to_string()),
            });
        }

        if property.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Property name cannot be empty".to_string(),
                field: Some("property".to_string()),
            });
        }

        let min_val = min_value.into();
        let max_val = max_value.into();

        let min_literal = match min_val {
            Value::Int(i) => Literal::Int(i),
            Value::Float(f) => Literal::Float(f),
            Value::String(s) => Literal::String(s),
            _ => {
                return Err(HyperQLError::ValidationError {
                    message: "Unsupported value type for range comparison".to_string(),
                    field: Some("min_value".to_string()),
                });
            }
        };

        let max_literal = match max_val {
            Value::Int(i) => Literal::Int(i),
            Value::Float(f) => Literal::Float(f),
            Value::String(s) => Literal::String(s),
            _ => {
                return Err(HyperQLError::ValidationError {
                    message: "Unsupported value type for range comparison".to_string(),
                    field: Some("max_value".to_string()),
                });
            }
        };

        let column_ref = Expression::Column(ColumnRef {
            table: None,
            name: property.to_string(),
        });

        let min_condition = Expression::Binary {
            left: Box::new(column_ref.clone()),
            op: BinaryOperator::GreaterThanOrEqual,
            right: Box::new(Expression::Literal(min_literal)),
        };

        let max_condition = Expression::Binary {
            left: Box::new(column_ref),
            op: BinaryOperator::LessThanOrEqual,
            right: Box::new(Expression::Literal(max_literal)),
        };

        let range_condition = Expression::Binary {
            left: Box::new(min_condition),
            op: BinaryOperator::And,
            right: Box::new(max_condition),
        };

        HyperQLBuilder::new()
            .select(["*"])
            .from(table)
            .where_expr(range_condition)
            .build()
    }

    pub fn aggregate_by_property(
        table: &str,
        group_by_property: &str,
        aggregate_property: &str,
        aggregate_function: &str,
    ) -> Result<Statement> {
        if table.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Table name cannot be empty".to_string(),
                field: Some("table".to_string()),
            });
        }

        if group_by_property.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Group by property cannot be empty".to_string(),
                field: Some("group_by_property".to_string()),
            });
        }

        if aggregate_property.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Aggregate property cannot be empty".to_string(),
                field: Some("aggregate_property".to_string()),
            });
        }

        let valid_functions = ["COUNT", "SUM", "AVG", "MIN", "MAX"];
        let function_upper = aggregate_function.to_uppercase();
        if !valid_functions.contains(&function_upper.as_str()) {
            return Err(HyperQLError::ValidationError {
                message: format!(
                    "Invalid aggregate function '{}'. Valid functions are: {}",
                    aggregate_function,
                    valid_functions.join(", ")
                ),
                field: Some("aggregate_function".to_string()),
            });
        }

        let select_items = vec![
            SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: group_by_property.to_string(),
                }),
                alias: None,
            },
            SelectItem::Expression {
                expr: Expression::Function {
                    name: function_upper,
                    args: vec![Expression::Column(ColumnRef {
                        table: None,
                        name: aggregate_property.to_string(),
                    })],
                },
                alias: Some(format!(
                    "{}_{}",
                    aggregate_function.to_lowercase(),
                    aggregate_property
                )),
            },
        ];

        let select_stmt = SelectStatement {
            select_list: select_items,
            from: Some(FromClause::Table {
                name: table.to_string(),
                alias: None,
            }),
            traverse_clause: None,
            where_clause: None,
            group_by: vec![Expression::Column(ColumnRef {
                table: None,
                name: group_by_property.to_string(),
            })],
            having: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        };

        Ok(Statement::Select(select_stmt))
    }
}

#[cfg(test)]
mod tests {
    use super::query_utils::*;
    use crate::ast::vector::similarity::SimilarityMetric;

    #[test]
    fn test_entity_by_label() {
        let stmt = entity_by_label("Person").expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert_eq!(select_stmt.select_list.len(), 1);
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_entities_near_point() {
        let stmt = entities_near_point(1.0, 2.0, 3.0, 5.0).expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_entities_near_point_validation() {
        let result = entities_near_point(1.0, 2.0, 3.0, -1.0);
        assert!(result.is_err(), "Should reject negative radius");
        
        let result = entities_near_point(1.0, 2.0, 3.0, f64::INFINITY);
        assert!(result.is_err(), "Should reject infinite radius");
        
        let result = entities_near_point(1.0, 2.0, 3.0, 0.0);
        assert!(result.is_err(), "Should reject zero radius");
    }

    #[test]
    fn test_vector_similarity_search() {
        let query_vector = vec![0.1, 0.2, 0.3, 0.4];
        let stmt = vector_similarity_search("embeddings", query_vector, 5)
            .expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_vector_similarity_search_validation() {
        let result = vector_similarity_search("", vec![0.1, 0.2], 5);
        assert!(result.is_err(), "Should reject empty vector column name");
        
        let result = vector_similarity_search("embeddings", vec![], 5);
        assert!(result.is_err(), "Should reject empty query vector");
        
        let result = vector_similarity_search("embeddings", vec![0.1, 0.2], 0);
        assert!(result.is_err(), "Should reject zero k");
    }

    #[test]
    fn test_traverse_relationship() {
        let stmt = traverse_relationship("user", "FRIENDS", Some("friend"))
            .expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert!(select_stmt.traverse_clause.is_some());
            assert!(matches!(select_stmt.from, Some(crate::ast::FromClause::Table { .. })));
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_traverse_relationship_validation() {
        let result = traverse_relationship("", "FRIENDS", Some("friend"));
        assert!(result.is_err(), "Should reject empty start node");
        
        let result = traverse_relationship("user", "", Some("friend"));
        assert!(result.is_err(), "Should reject empty relationship type");
    }

    #[test]
    fn test_similarity_search_with_threshold() {
        let query_vector = vec![0.1, 0.2, 0.3];
        let stmt = similarity_search_with_threshold(
            "embeddings",
            query_vector,
            0.8,
            SimilarityMetric::Cosine,
        ).expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_similarity_search_threshold_validation() {
        let query_vector = vec![0.1, 0.2, 0.3];
        
        let result = similarity_search_with_threshold(
            "embeddings",
            query_vector.clone(),
            1.5,
            SimilarityMetric::Cosine,
        );
        assert!(result.is_err(), "Should reject threshold > 1.0");
        
        let result = similarity_search_with_threshold(
            "embeddings",
            query_vector,
            -0.1,
            SimilarityMetric::Cosine,
        );
        assert!(result.is_err(), "Should reject negative threshold");
    }

    #[test]
    fn test_entities_by_property_range() {
        let stmt = entities_by_property_range(
            "users",
            "age",
            18,
            65,
        ).expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_aggregate_by_property() {
        let stmt = aggregate_by_property(
            "employees",
            "department",
            "salary",
            "AVG",
        ).expect("Should create valid statement");
        
        if let crate::ast::Statement::Select(select_stmt) = stmt {
            assert_eq!(select_stmt.select_list.len(), 2);
            assert_eq!(select_stmt.group_by.len(), 1);
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_aggregate_function_validation() {
        let result = aggregate_by_property(
            "employees",
            "department",
            "salary",
            "INVALID",
        );
        assert!(result.is_err(), "Should reject invalid aggregate function");
    }
}
