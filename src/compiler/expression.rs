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
                let compiled_vector_name = CompiledExpression::Literal(Value::String(vector_name));
                let compiled_reference = self.compile_expression(*reference)?;
                let compiled_metric = CompiledExpression::Literal(Value::String(format!("{:?}", metric)));
                let compiled_threshold = match threshold {
                    Some(t) => CompiledExpression::Literal(Value::Float(t)),
                    None => CompiledExpression::Literal(Value::Null),
                };
                let compiled_vector_type = CompiledExpression::Literal(Value::String(format!("{:?}", vector_type)));

                Ok(CompiledExpression::Function {
                    name: "COSINE_SIMILARITY".to_string(),
                    args: vec![compiled_vector_name, compiled_reference, compiled_metric, compiled_threshold, compiled_vector_type],
                    result_type: ValueType::Float,
                })
            }
            VectorExpression::KNN { vector_name, reference, k, metric, vector_type } => {
                let compiled_vector_name = CompiledExpression::Literal(Value::String(vector_name));
                let compiled_reference = self.compile_expression(*reference)?;
                let compiled_k = CompiledExpression::Literal(Value::Int(k as i64));
                let compiled_metric = CompiledExpression::Literal(Value::String(format!("{:?}", metric)));
                let compiled_vector_type = CompiledExpression::Literal(Value::String(format!("{:?}", vector_type)));

                Ok(CompiledExpression::Function {
                    name: "KNN".to_string(),
                    args: vec![compiled_vector_name, compiled_reference, compiled_k, compiled_metric, compiled_vector_type],
                    result_type: ValueType::List(Box::new(ValueType::EntityId)),
                })
            }
        }
    }
}

impl Default for ExpressionCompiler {
    fn default() -> Self {
        Self::new()
    }
}
