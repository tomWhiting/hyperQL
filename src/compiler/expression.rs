use crate::ast::*;
use crate::error::*;
use std::collections::HashMap;

pub use super::{CompiledExpression, ValueType};

#[derive(Debug, Clone)]
pub struct ExpressionCompiler {
    #[allow(dead_code)]
    schemas: HashMap<String, EntitySchema>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EntitySchema {
    pub name: String,
    pub properties: HashMap<String, ValueType>,
}

impl ExpressionCompiler {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn compile_expression(&self, expr: Expression) -> Result<CompiledExpression> {
        match expr {
            Expression::Literal(literal) => {
                let (value, _value_type) = self.compile_literal(literal)?;
                Ok(CompiledExpression::Literal(value))
            }
            Expression::Column(col_ref) => {
                let value_type = self.infer_column_type(&col_ref)?;
                Ok(CompiledExpression::Column {
                    table: col_ref.table,
                    name: col_ref.name,
                    value_type,
                })
            }
            Expression::Binary { left, op, right } => {
                let compiled_left = self.compile_expression(*left)?;
                let compiled_right = self.compile_expression(*right)?;
                let result_type = self.infer_binary_result_type(&compiled_left, &op, &compiled_right)?;

                Ok(CompiledExpression::Binary {
                    left: Box::new(compiled_left),
                    op,
                    right: Box::new(compiled_right),
                    result_type,
                })
            }
            Expression::Unary { op, expr } => {
                let compiled_expr = self.compile_expression(*expr)?;
                let result_type = self.infer_unary_result_type(&op, &compiled_expr)?;

                Ok(CompiledExpression::Unary {
                    op,
                    expr: Box::new(compiled_expr),
                    result_type,
                })
            }
            Expression::Between { expr, lower, upper, negated } => {
                let compiled_expr = self.compile_expression(*expr)?;
                let compiled_lower = self.compile_expression(*lower)?;
                let compiled_upper = self.compile_expression(*upper)?;

                let ge_expr = CompiledExpression::Binary {
                    left: Box::new(compiled_expr.clone()),
                    op: BinaryOperator::GreaterThanOrEqual,
                    right: Box::new(compiled_lower),
                    result_type: ValueType::Bool,
                };

                let le_expr = CompiledExpression::Binary {
                    left: Box::new(compiled_expr),
                    op: BinaryOperator::LessThanOrEqual,
                    right: Box::new(compiled_upper),
                    result_type: ValueType::Bool,
                };

                let between_expr = CompiledExpression::Binary {
                    left: Box::new(ge_expr),
                    op: BinaryOperator::And,
                    right: Box::new(le_expr),
                    result_type: ValueType::Bool,
                };

                if negated {
                    Ok(CompiledExpression::Unary {
                        op: UnaryOperator::Not,
                        expr: Box::new(between_expr),
                        result_type: ValueType::Bool,
                    })
                } else {
                    Ok(between_expr)
                }
            }
            Expression::Function { name, args } => {
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.compile_expression(arg)?);
                }
                let result_type = self.infer_function_result_type(&name, &compiled_args)?;

                Ok(CompiledExpression::Function {
                    name,
                    args: compiled_args,
                    result_type,
                })
            }
            Expression::Geometric(geom_expr) => {
                self.compile_geometric_expression(geom_expr)
            }
            Expression::Vector(vector_expr) => {
                self.compile_vector_expression(vector_expr)
            }
        }
    }

    fn compile_literal(&self, literal: Literal) -> Result<(Value, ValueType)> {
        match literal {
            Literal::Null => Ok((Value::Null, ValueType::Null)),
            Literal::Bool(b) => Ok((Value::Bool(b), ValueType::Bool)),
            Literal::Int(i) => Ok((Value::Int(i), ValueType::Int)),
            Literal::Float(f) => Ok((Value::Float(f), ValueType::Float)),
            Literal::String(s) => Ok((Value::String(s), ValueType::String)),
            Literal::EntityId(id) => Ok((Value::EntityId(id), ValueType::EntityId)),
        }
    }

    fn infer_column_type(&self, _col_ref: &ColumnRef) -> Result<ValueType> {
        Ok(ValueType::String)
    }

    fn infer_binary_result_type(&self, _left: &CompiledExpression, op: &BinaryOperator, _right: &CompiledExpression) -> Result<ValueType> {
        match op {
            BinaryOperator::Equal | BinaryOperator::NotEqual |
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual |
            BinaryOperator::And | BinaryOperator::Or => Ok(ValueType::Bool),

            BinaryOperator::Add | BinaryOperator::Subtract |
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                Ok(ValueType::Float)
            }

            BinaryOperator::Like | BinaryOperator::NotLike => Ok(ValueType::Bool),
            BinaryOperator::In | BinaryOperator::NotIn => Ok(ValueType::Bool),
        }
    }

    fn infer_unary_result_type(&self, op: &UnaryOperator, _expr: &CompiledExpression) -> Result<ValueType> {
        match op {
            UnaryOperator::Not => Ok(ValueType::Bool),
            UnaryOperator::Minus | UnaryOperator::Plus => {
                Ok(ValueType::Float)
            }
            UnaryOperator::IsNull | UnaryOperator::IsNotNull => Ok(ValueType::Bool),
        }
    }

    fn infer_function_result_type(&self, name: &str, _args: &[CompiledExpression]) -> Result<ValueType> {
        match name.to_uppercase().as_str() {
            "COUNT" => Ok(ValueType::Int),
            "SUM" => Ok(ValueType::Float),
            "AVG" => Ok(ValueType::Float),
            "MIN" | "MAX" => Ok(ValueType::Float),

            "UPPER" | "LOWER" | "TRIM" => Ok(ValueType::String),
            "LENGTH" => Ok(ValueType::Int),

            "ABS" | "ROUND" | "FLOOR" | "CEIL" => Ok(ValueType::Float),
            "SQRT" | "POW" | "EXP" | "LOG" => Ok(ValueType::Float),

            "NOW" | "CURRENT_TIMESTAMP" => Ok(ValueType::Timestamp),

            "HYPERBOLIC_DISTANCE" | "GEODESIC_DISTANCE" => Ok(ValueType::Distance),
            "WITHIN_RADIUS" | "CONTAINS" | "INTERSECTS" => Ok(ValueType::Bool),

            "COSINE_SIMILARITY" | "DOT_PRODUCT" => Ok(ValueType::Float),
            "EUCLIDEAN_DISTANCE" => Ok(ValueType::Distance),
            "NORMALIZE" => Ok(ValueType::Vector),
            "KNN" | "SIMILARITY_SEARCH" => Ok(ValueType::List(Box::new(ValueType::EntityId))),

            _ => Ok(ValueType::String),
        }
    }

    fn compile_geometric_expression(&self, geom_expr: crate::ast::geometric::GeometricExpression) -> Result<CompiledExpression> {
        match geom_expr {
            crate::ast::geometric::GeometricExpression::Within { target, radius, reference } => {
                let compiled_target = self.compile_expression(*target)?;
                let compiled_reference = self.compile_expression(*reference)?;
                let compiled_radius = CompiledExpression::Literal(Value::Float(radius));

                Ok(CompiledExpression::Function {
                    name: "WITHIN_RADIUS".to_string(),
                    args: vec![compiled_target, compiled_reference, compiled_radius],
                    result_type: ValueType::Bool,
                })
            }
            crate::ast::geometric::GeometricExpression::Near { target, reference, max_distance } => {
                let compiled_target = self.compile_expression(*target)?;
                let compiled_reference = self.compile_expression(*reference)?;
                let compiled_max_distance = CompiledExpression::Literal(Value::Float(max_distance));

                Ok(CompiledExpression::Function {
                    name: "HYPERBOLIC_DISTANCE".to_string(),
                    args: vec![compiled_target, compiled_reference, compiled_max_distance],
                    result_type: ValueType::Distance,
                })
            }
            crate::ast::geometric::GeometricExpression::InRadius { target, center, radius } => {
                let compiled_target = self.compile_expression(*target)?;
                let compiled_center = self.compile_expression(*center)?;
                let compiled_radius = CompiledExpression::Literal(Value::Float(radius));

                Ok(CompiledExpression::Function {
                    name: "WITHIN_RADIUS".to_string(),
                    args: vec![compiled_target, compiled_center, compiled_radius],
                    result_type: ValueType::Bool,
                })
            }
        }
    }

    fn compile_vector_expression(&self, vector_expr: VectorExpression) -> Result<CompiledExpression> {
        match vector_expr {
            VectorExpression::Similarity { vector_name, reference, metric, threshold, vector_type } => {
                self.validate_vector_name(&vector_name)?;

                if let Some(thresh) = threshold {
                    self.validate_threshold(thresh)?;
                }

                let compiled_vector_name = CompiledExpression::Literal(Value::String(vector_name));
                let compiled_reference = self.compile_expression(*reference)?;
                let compiled_metric = CompiledExpression::Literal(Value::String(self.metric_to_string(&metric)));
                let compiled_threshold = match threshold {
                    Some(t) => CompiledExpression::Literal(Value::Float(t)),
                    None => CompiledExpression::Literal(Value::Null),
                };
                let compiled_vector_type = CompiledExpression::Literal(Value::String(self.vector_type_to_string(&vector_type)));

                let op_type = self.metric_to_op_type(&metric);
                let function_name = self.op_type_to_function_name(&op_type);

                Ok(CompiledExpression::Function {
                    name: function_name,
                    args: vec![compiled_vector_name, compiled_reference, compiled_metric, compiled_threshold, compiled_vector_type],
                    result_type: ValueType::Float,
                })
            }
            VectorExpression::KNN { vector_name, reference, k, metric, vector_type } => {
                self.validate_vector_name(&vector_name)?;
                self.validate_k(k)?;

                let compiled_vector_name = CompiledExpression::Literal(Value::String(vector_name));
                let compiled_reference = self.compile_expression(*reference)?;
                let compiled_k = CompiledExpression::Literal(Value::Int(k as i64));
                let compiled_metric = CompiledExpression::Literal(Value::String(self.metric_to_string(&metric)));
                let compiled_vector_type = CompiledExpression::Literal(Value::String(self.vector_type_to_string(&vector_type)));

                Ok(CompiledExpression::Function {
                    name: "KNN".to_string(),
                    args: vec![compiled_vector_name, compiled_reference, compiled_k, compiled_metric, compiled_vector_type],
                    result_type: ValueType::List(Box::new(ValueType::EntityId)),
                })
            }
        }
    }

    fn validate_vector_name(&self, vector_name: &str) -> Result<()> {
        if vector_name.is_empty() {
            return Err(HyperQLError::ValidationError {
                message: "Vector name cannot be empty".to_string(),
                field: Some("vector_name".to_string()),
            });
        }
        Ok(())
    }

    fn validate_threshold(&self, threshold: f64) -> Result<()> {
        if !(-1.0..=1.0).contains(&threshold) {
            return Err(HyperQLError::ValidationError {
                message: format!("Similarity threshold must be between -1.0 and 1.0, got {}", threshold),
                field: Some("threshold".to_string()),
            });
        }
        Ok(())
    }

    fn validate_k(&self, k: u32) -> Result<()> {
        if k == 0 {
            return Err(HyperQLError::ValidationError {
                message: "k must be greater than 0 for KNN queries".to_string(),
                field: Some("k".to_string()),
            });
        }
        Ok(())
    }

    fn metric_to_op_type(&self, metric: &crate::ast::vector::similarity::SimilarityMetric) -> super::VectorOpType {
        match metric {
            crate::ast::vector::similarity::SimilarityMetric::Cosine => super::VectorOpType::CosineSimilarity,
            crate::ast::vector::similarity::SimilarityMetric::Euclidean => super::VectorOpType::EuclideanDistance,
            crate::ast::vector::similarity::SimilarityMetric::DotProduct => super::VectorOpType::DotProduct,
            crate::ast::vector::similarity::SimilarityMetric::Manhattan => super::VectorOpType::EuclideanDistance,
            crate::ast::vector::similarity::SimilarityMetric::Jaccard => super::VectorOpType::CosineSimilarity,
            crate::ast::vector::similarity::SimilarityMetric::Custom(_) => super::VectorOpType::CosineSimilarity,
        }
    }

    fn op_type_to_function_name(&self, op_type: &super::VectorOpType) -> String {
        match op_type {
            super::VectorOpType::CosineSimilarity => "COSINE_SIMILARITY".to_string(),
            super::VectorOpType::EuclideanDistance => "EUCLIDEAN_DISTANCE".to_string(),
            super::VectorOpType::DotProduct => "DOT_PRODUCT".to_string(),
            super::VectorOpType::Normalize => "NORMALIZE".to_string(),
            super::VectorOpType::KNN => "KNN".to_string(),
            super::VectorOpType::SimilaritySearch => "SIMILARITY_SEARCH".to_string(),
        }
    }

    fn metric_to_string(&self, metric: &crate::ast::vector::similarity::SimilarityMetric) -> String {
        match metric {
            crate::ast::vector::similarity::SimilarityMetric::Cosine => "cosine".to_string(),
            crate::ast::vector::similarity::SimilarityMetric::Euclidean => "euclidean".to_string(),
            crate::ast::vector::similarity::SimilarityMetric::DotProduct => "dotproduct".to_string(),
            crate::ast::vector::similarity::SimilarityMetric::Manhattan => "manhattan".to_string(),
            crate::ast::vector::similarity::SimilarityMetric::Jaccard => "jaccard".to_string(),
            crate::ast::vector::similarity::SimilarityMetric::Custom(name) => name.clone(),
        }
    }

    fn vector_type_to_string(&self, vector_type: &crate::ast::vector::similarity::VectorType) -> String {
        match vector_type {
            crate::ast::vector::similarity::VectorType::Dense { dimensions } => {
                format!("Dense({})", dimensions)
            }
            crate::ast::vector::similarity::VectorType::Sparse { max_dimensions } => {
                match max_dimensions {
                    Some(max) => format!("Sparse({})", max),
                    None => "Sparse".to_string(),
                }
            }
            crate::ast::vector::similarity::VectorType::ColBERT { token_dimensions, max_tokens } => {
                match max_tokens {
                    Some(max) => format!("ColBERT({},{})", token_dimensions, max),
                    None => format!("ColBERT({})", token_dimensions),
                }
            }
        }
    }
}

impl Default for ExpressionCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::vector::similarity::{SimilarityMetric, VectorType};

    #[test]
    fn test_compile_cosine_similarity_expression() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "text_embedding".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query_vec".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: Some(0.8),
            vector_type: VectorType::Dense { dimensions: 768 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "Cosine similarity compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { name, args, result_type } => {
                assert_eq!(name, "COSINE_SIMILARITY");
                assert_eq!(args.len(), 5);
                assert_eq!(result_type, ValueType::Float);
            }
            _ => panic!("Expected Function expression, got: {:?}", compiled),
        }
    }

    #[test]
    fn test_compile_euclidean_distance_expression() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "feature_vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("target".to_string()))),
            metric: SimilarityMetric::Euclidean,
            threshold: None,
            vector_type: VectorType::Dense { dimensions: 512 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "Euclidean distance compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { name, result_type, .. } => {
                assert_eq!(name, "EUCLIDEAN_DISTANCE");
                assert_eq!(result_type, ValueType::Float);
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_compile_dot_product_expression() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "embedding".to_string(),
            reference: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: "ref_vec".to_string(),
            })),
            metric: SimilarityMetric::DotProduct,
            threshold: Some(0.5),
            vector_type: VectorType::Dense { dimensions: 256 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "Dot product compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { name, result_type, .. } => {
                assert_eq!(name, "DOT_PRODUCT");
                assert_eq!(result_type, ValueType::Float);
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_compile_knn_expression() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::KNN {
            vector_name: "text_embedding".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            k: 10,
            metric: SimilarityMetric::Cosine,
            vector_type: VectorType::Dense { dimensions: 768 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "KNN compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { name, args, result_type } => {
                assert_eq!(name, "KNN");
                assert_eq!(args.len(), 5);
                assert_eq!(result_type, ValueType::List(Box::new(ValueType::EntityId)));

                if let CompiledExpression::Literal(Value::Int(k_val)) = &args[2] {
                    assert_eq!(*k_val, 10);
                } else {
                    panic!("Expected k to be compiled as Int literal");
                }
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_validate_empty_vector_name() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: None,
            vector_type: VectorType::Dense { dimensions: 100 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_err(), "Empty vector name should fail validation");

        match result.unwrap_err() {
            HyperQLError::ValidationError { message, .. } => {
                assert!(message.contains("Vector name cannot be empty"));
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_threshold_out_of_range() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: Some(1.5),
            vector_type: VectorType::Dense { dimensions: 100 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_err(), "Threshold > 1.0 should fail validation");

        match result.unwrap_err() {
            HyperQLError::ValidationError { message, .. } => {
                assert!(message.contains("threshold must be between -1.0 and 1.0"));
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_threshold_negative_out_of_range() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: SimilarityMetric::Cosine,
            threshold: Some(-1.5),
            vector_type: VectorType::Dense { dimensions: 100 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_err(), "Threshold < -1.0 should fail validation");
    }

    #[test]
    fn test_validate_k_zero() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::KNN {
            vector_name: "vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            k: 0,
            metric: SimilarityMetric::Cosine,
            vector_type: VectorType::Dense { dimensions: 100 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_err(), "k = 0 should fail validation");

        match result.unwrap_err() {
            HyperQLError::ValidationError { message, .. } => {
                assert!(message.contains("k must be greater than 0"));
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_compile_sparse_vector() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "sparse_keywords".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: SimilarityMetric::Jaccard,
            threshold: None,
            vector_type: VectorType::Sparse { max_dimensions: Some(10000) },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "Sparse vector compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { name, args, .. } => {
                assert_eq!(name, "COSINE_SIMILARITY");

                if let CompiledExpression::Literal(Value::String(vec_type)) = &args[4] {
                    assert!(vec_type.contains("Sparse"));
                } else {
                    panic!("Expected vector_type to be String literal");
                }
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_compile_colbert_vector() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "colbert_tokens".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: SimilarityMetric::DotProduct,
            threshold: None,
            vector_type: VectorType::ColBERT {
                token_dimensions: 128,
                max_tokens: Some(64),
            },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "ColBERT vector compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { name, args, .. } => {
                assert_eq!(name, "DOT_PRODUCT");

                if let CompiledExpression::Literal(Value::String(vec_type)) = &args[4] {
                    assert!(vec_type.contains("ColBERT"));
                    assert!(vec_type.contains("128"));
                } else {
                    panic!("Expected vector_type to be String literal");
                }
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_compile_custom_metric() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::Similarity {
            vector_name: "custom_vec".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: SimilarityMetric::Custom("my_custom_metric".to_string()),
            threshold: None,
            vector_type: VectorType::Dense { dimensions: 512 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "Custom metric compilation failed: {:?}", result.err());

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { args, .. } => {
                if let CompiledExpression::Literal(Value::String(metric_name)) = &args[2] {
                    assert_eq!(metric_name, "my_custom_metric");
                } else {
                    panic!("Expected metric to be String literal");
                }
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_metric_to_op_type_mapping() {
        let compiler = ExpressionCompiler::new();

        assert!(matches!(
            compiler.metric_to_op_type(&SimilarityMetric::Cosine),
            super::super::VectorOpType::CosineSimilarity
        ));

        assert!(matches!(
            compiler.metric_to_op_type(&SimilarityMetric::Euclidean),
            super::super::VectorOpType::EuclideanDistance
        ));

        assert!(matches!(
            compiler.metric_to_op_type(&SimilarityMetric::DotProduct),
            super::super::VectorOpType::DotProduct
        ));
    }

    #[test]
    fn test_metric_to_string_conversion() {
        let compiler = ExpressionCompiler::new();

        assert_eq!(compiler.metric_to_string(&SimilarityMetric::Cosine), "cosine");
        assert_eq!(compiler.metric_to_string(&SimilarityMetric::Euclidean), "euclidean");
        assert_eq!(compiler.metric_to_string(&SimilarityMetric::DotProduct), "dotproduct");
        assert_eq!(compiler.metric_to_string(&SimilarityMetric::Manhattan), "manhattan");
        assert_eq!(compiler.metric_to_string(&SimilarityMetric::Jaccard), "jaccard");

        let custom_metric = SimilarityMetric::Custom("test_metric".to_string());
        assert_eq!(compiler.metric_to_string(&custom_metric), "test_metric");
    }

    #[test]
    fn test_vector_type_to_string_conversion() {
        let compiler = ExpressionCompiler::new();

        assert_eq!(
            compiler.vector_type_to_string(&VectorType::Dense { dimensions: 768 }),
            "Dense(768)"
        );

        assert_eq!(
            compiler.vector_type_to_string(&VectorType::Sparse { max_dimensions: Some(10000) }),
            "Sparse(10000)"
        );

        assert_eq!(
            compiler.vector_type_to_string(&VectorType::Sparse { max_dimensions: None }),
            "Sparse"
        );

        assert_eq!(
            compiler.vector_type_to_string(&VectorType::ColBERT {
                token_dimensions: 128,
                max_tokens: Some(64),
            }),
            "ColBERT(128,64)"
        );

        assert_eq!(
            compiler.vector_type_to_string(&VectorType::ColBERT {
                token_dimensions: 256,
                max_tokens: None,
            }),
            "ColBERT(256)"
        );
    }

    #[test]
    fn test_compile_expression_with_vector() {
        let compiler = ExpressionCompiler::new();

        let expr = Expression::Vector(VectorExpression::Similarity {
            vector_name: "embedding".to_string(),
            reference: Box::new(Expression::Literal(Literal::Float(0.5))),
            metric: SimilarityMetric::Cosine,
            threshold: Some(0.7),
            vector_type: VectorType::Dense { dimensions: 384 },
        });

        let result = compiler.compile_expression(expr);
        assert!(result.is_ok(), "Vector expression compilation failed: {:?}", result.err());

        match result.unwrap() {
            CompiledExpression::Function { name, result_type, .. } => {
                assert_eq!(name, "COSINE_SIMILARITY");
                assert_eq!(result_type, ValueType::Float);
            }
            other => panic!("Expected Function expression, got: {:?}", other),
        }
    }

    #[test]
    fn test_knn_with_large_k() {
        let compiler = ExpressionCompiler::new();

        let vector_expr = VectorExpression::KNN {
            vector_name: "text_embedding".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            k: 1000,
            metric: SimilarityMetric::Cosine,
            vector_type: VectorType::Dense { dimensions: 768 },
        };

        let result = compiler.compile_vector_expression(vector_expr);
        assert!(result.is_ok(), "KNN with large k should compile successfully");

        let compiled = result.unwrap();
        match compiled {
            CompiledExpression::Function { args, .. } => {
                if let CompiledExpression::Literal(Value::Int(k_val)) = &args[2] {
                    assert_eq!(*k_val, 1000);
                } else {
                    panic!("Expected k to be compiled as Int literal");
                }
            }
            _ => panic!("Expected Function expression"),
        }
    }
}
