use crate::compiler::{ExecutionPlan, CompiledExpression};
use crate::error::Result;
use crate::ast::{BinaryOperator, UnaryOperator};
use crate::types::Value;

pub fn optimize(plan: ExecutionPlan) -> Result<ExecutionPlan> {
    let mut optimizer = ConstantFoldingOptimizer::new();
    optimizer.optimize_plan(plan)
}

struct ConstantFoldingOptimizer {
}

impl ConstantFoldingOptimizer {
    fn new() -> Self {
        Self {}
    }

    fn optimize_plan(&mut self, plan: ExecutionPlan) -> Result<ExecutionPlan> {
        match plan {
            ExecutionPlan::Filter { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                let folded_predicate = self.fold_expression(predicate);
                Ok(ExecutionPlan::Filter {
                    input: Box::new(optimized_input),
                    predicate: folded_predicate,
                })
            }
            ExecutionPlan::Project { input, expressions } => {
                let optimized_input = self.optimize_plan(*input)?;
                let folded_expressions = expressions
                    .into_iter()
                    .map(|mut proj| {
                        proj.expression = self.fold_expression(proj.expression);
                        proj
                    })
                    .collect();
                Ok(ExecutionPlan::Project {
                    input: Box::new(optimized_input),
                    expressions: folded_expressions,
                })
            }
            ExecutionPlan::GroupBy { input, group_expressions, aggregate_expressions } => {
                let optimized_input = self.optimize_plan(*input)?;
                let folded_group_expressions = group_expressions
                    .into_iter()
                    .map(|expr| self.fold_expression(expr))
                    .collect();
                let folded_aggregate_expressions = aggregate_expressions
                    .into_iter()
                    .map(|mut proj| {
                        proj.expression = self.fold_expression(proj.expression);
                        proj
                    })
                    .collect();
                Ok(ExecutionPlan::GroupBy {
                    input: Box::new(optimized_input),
                    group_expressions: folded_group_expressions,
                    aggregate_expressions: folded_aggregate_expressions,
                })
            }
            ExecutionPlan::Having { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                let folded_predicate = self.fold_expression(predicate);
                Ok(ExecutionPlan::Having {
                    input: Box::new(optimized_input),
                    predicate: folded_predicate,
                })
            }
            ExecutionPlan::Sort { input, sort_keys } => {
                let optimized_input = self.optimize_plan(*input)?;
                let folded_sort_keys = sort_keys
                    .into_iter()
                    .map(|mut key| {
                        key.expression = self.fold_expression(key.expression);
                        key
                    })
                    .collect();
                Ok(ExecutionPlan::Sort {
                    input: Box::new(optimized_input),
                    sort_keys: folded_sort_keys,
                })
            }
            ExecutionPlan::Limit { input, count, offset } => {
                let optimized_input = self.optimize_plan(*input)?;
                Ok(ExecutionPlan::Limit {
                    input: Box::new(optimized_input),
                    count,
                    offset,
                })
            }
            ExecutionPlan::Scan { table, filter, projection } => {
                let folded_filter = filter.map(|f| self.fold_expression(f));
                let folded_projection = projection
                    .into_iter()
                    .map(|mut proj| {
                        proj.expression = self.fold_expression(proj.expression);
                        proj
                    })
                    .collect();
                Ok(ExecutionPlan::Scan {
                    table,
                    filter: folded_filter,
                    projection: folded_projection,
                })
            }
            other => Ok(other),
        }
    }

    fn fold_expression(&self, expr: CompiledExpression) -> CompiledExpression {
        match expr {
            CompiledExpression::Binary { left, op, right, result_type } => {
                let folded_left = self.fold_expression(*left);
                let folded_right = self.fold_expression(*right);
                
                if let (CompiledExpression::Literal(left_val), CompiledExpression::Literal(right_val)) = (&folded_left, &folded_right) {
                    if let Some(result) = self.evaluate_binary_operation(left_val, &op, right_val) {
                        return CompiledExpression::Literal(result);
                    }
                }
                
                // Special cases for optimization
                match (&folded_left, &op, &folded_right) {
                    // Identity operations: x + 0, x - 0, x * 1, x / 1
                    (expr, BinaryOperator::Add, CompiledExpression::Literal(Value::Int(0))) |
                    (expr, BinaryOperator::Subtract, CompiledExpression::Literal(Value::Int(0))) |
                    (expr, BinaryOperator::Multiply, CompiledExpression::Literal(Value::Int(1))) |
                    (expr, BinaryOperator::Divide, CompiledExpression::Literal(Value::Int(1))) => {
                        expr.clone()
                    }
                    (expr, BinaryOperator::Add, CompiledExpression::Literal(Value::Float(f))) if *f == 0.0 => expr.clone(),
                    (expr, BinaryOperator::Subtract, CompiledExpression::Literal(Value::Float(f))) if *f == 0.0 => expr.clone(),
                    (expr, BinaryOperator::Multiply, CompiledExpression::Literal(Value::Float(f))) if *f == 1.0 => expr.clone(),
                    (expr, BinaryOperator::Divide, CompiledExpression::Literal(Value::Float(f))) if *f == 1.0 => expr.clone(),
                    
                    // Commutative identity: 0 + x, 1 * x
                    (CompiledExpression::Literal(Value::Int(0)), BinaryOperator::Add, expr) |
                    (CompiledExpression::Literal(Value::Int(1)), BinaryOperator::Multiply, expr) => {
                        expr.clone()
                    }
                    (CompiledExpression::Literal(Value::Float(f)), BinaryOperator::Add, expr) if *f == 0.0 => expr.clone(),
                    (CompiledExpression::Literal(Value::Float(f)), BinaryOperator::Multiply, expr) if *f == 1.0 => expr.clone(),
                    
                    // Zero multiplication: x * 0, 0 * x
                    (_, BinaryOperator::Multiply, CompiledExpression::Literal(Value::Int(0))) |
                    (CompiledExpression::Literal(Value::Int(0)), BinaryOperator::Multiply, _) => {
                        CompiledExpression::Literal(Value::Int(0))
                    }
                    (_, BinaryOperator::Multiply, CompiledExpression::Literal(Value::Float(f))) if *f == 0.0 => {
                        CompiledExpression::Literal(Value::Float(0.0))
                    }
                    (CompiledExpression::Literal(Value::Float(f)), BinaryOperator::Multiply, _) if *f == 0.0 => {
                        CompiledExpression::Literal(Value::Float(0.0))
                    }
                    
                    // Boolean short-circuiting: true OR x, false AND x
                    (CompiledExpression::Literal(Value::Bool(true)), BinaryOperator::Or, _) => {
                        CompiledExpression::Literal(Value::Bool(true))
                    }
                    (_, BinaryOperator::Or, CompiledExpression::Literal(Value::Bool(true))) => {
                        CompiledExpression::Literal(Value::Bool(true))
                    }
                    (CompiledExpression::Literal(Value::Bool(false)), BinaryOperator::And, _) => {
                        CompiledExpression::Literal(Value::Bool(false))
                    }
                    (_, BinaryOperator::And, CompiledExpression::Literal(Value::Bool(false))) => {
                        CompiledExpression::Literal(Value::Bool(false))
                    }
                    
                    // Boolean identity: false OR x, true AND x
                    (CompiledExpression::Literal(Value::Bool(false)), BinaryOperator::Or, expr) |
                    (expr, BinaryOperator::Or, CompiledExpression::Literal(Value::Bool(false))) |
                    (CompiledExpression::Literal(Value::Bool(true)), BinaryOperator::And, expr) |
                    (expr, BinaryOperator::And, CompiledExpression::Literal(Value::Bool(true))) => {
                        expr.clone()
                    }
                    
                    _ => {
                        CompiledExpression::Binary {
                            left: Box::new(folded_left),
                            op,
                            right: Box::new(folded_right),
                            result_type,
                        }
                    }
                }
            }
            CompiledExpression::Unary { op, expr, result_type } => {
                let folded_expr = self.fold_expression(*expr);
                
                if let CompiledExpression::Literal(val) = &folded_expr {
                    if let Some(result) = self.evaluate_unary_operation(&op, val) {
                        return CompiledExpression::Literal(result);
                    }
                }
                
                // Double negation: NOT NOT x -> x
                if let (UnaryOperator::Not, CompiledExpression::Unary { op: UnaryOperator::Not, expr: inner, .. }) = (&op, &folded_expr) {
                    return (**inner).clone();
                }
                
                CompiledExpression::Unary {
                    op,
                    expr: Box::new(folded_expr),
                    result_type,
                }
            }
            CompiledExpression::Function { name, args, result_type } => {
                let folded_args: Vec<_> = args.into_iter().map(|arg| self.fold_expression(arg)).collect();
                
                // Try to evaluate constant functions
                if folded_args.iter().all(|arg| matches!(arg, CompiledExpression::Literal(_))) {
                    if let Some(result) = self.evaluate_function(&name, &folded_args) {
                        return CompiledExpression::Literal(result);
                    }
                }
                
                CompiledExpression::Function {
                    name,
                    args: folded_args,
                    result_type,
                }
            }
            other => other,
        }
    }

    fn evaluate_binary_operation(&self, left: &Value, op: &BinaryOperator, right: &Value) -> Option<Value> {
        match (left, op, right) {
            // Integer arithmetic
            (Value::Int(a), BinaryOperator::Add, Value::Int(b)) => Some(Value::Int(a + b)),
            (Value::Int(a), BinaryOperator::Subtract, Value::Int(b)) => Some(Value::Int(a - b)),
            (Value::Int(a), BinaryOperator::Multiply, Value::Int(b)) => Some(Value::Int(a * b)),
            (Value::Int(a), BinaryOperator::Divide, Value::Int(b)) if *b != 0 => Some(Value::Int(a / b)),
            (Value::Int(a), BinaryOperator::Modulo, Value::Int(b)) if *b != 0 => Some(Value::Int(a % b)),
            
            // Float arithmetic
            (Value::Float(a), BinaryOperator::Add, Value::Float(b)) => Some(Value::Float(a + b)),
            (Value::Float(a), BinaryOperator::Subtract, Value::Float(b)) => Some(Value::Float(a - b)),
            (Value::Float(a), BinaryOperator::Multiply, Value::Float(b)) => Some(Value::Float(a * b)),
            (Value::Float(a), BinaryOperator::Divide, Value::Float(b)) if *b != 0.0 => Some(Value::Float(a / b)),
            (Value::Float(a), BinaryOperator::Modulo, Value::Float(b)) if *b != 0.0 => Some(Value::Float(a % b)),
            
            // Mixed arithmetic (Int + Float)
            (Value::Int(a), BinaryOperator::Add, Value::Float(b)) => Some(Value::Float(*a as f64 + b)),
            (Value::Float(a), BinaryOperator::Add, Value::Int(b)) => Some(Value::Float(a + *b as f64)),
            (Value::Int(a), BinaryOperator::Subtract, Value::Float(b)) => Some(Value::Float(*a as f64 - b)),
            (Value::Float(a), BinaryOperator::Subtract, Value::Int(b)) => Some(Value::Float(a - *b as f64)),
            (Value::Int(a), BinaryOperator::Multiply, Value::Float(b)) => Some(Value::Float(*a as f64 * b)),
            (Value::Float(a), BinaryOperator::Multiply, Value::Int(b)) => Some(Value::Float(a * *b as f64)),
            (Value::Int(a), BinaryOperator::Divide, Value::Float(b)) if *b != 0.0 => Some(Value::Float(*a as f64 / b)),
            (Value::Float(a), BinaryOperator::Divide, Value::Int(b)) if *b != 0 => Some(Value::Float(a / *b as f64)),
            
            // Integer comparisons
            (Value::Int(a), BinaryOperator::Equal, Value::Int(b)) => Some(Value::Bool(a == b)),
            (Value::Int(a), BinaryOperator::NotEqual, Value::Int(b)) => Some(Value::Bool(a != b)),
            (Value::Int(a), BinaryOperator::LessThan, Value::Int(b)) => Some(Value::Bool(a < b)),
            (Value::Int(a), BinaryOperator::LessThanOrEqual, Value::Int(b)) => Some(Value::Bool(a <= b)),
            (Value::Int(a), BinaryOperator::GreaterThan, Value::Int(b)) => Some(Value::Bool(a > b)),
            (Value::Int(a), BinaryOperator::GreaterThanOrEqual, Value::Int(b)) => Some(Value::Bool(a >= b)),
            
            // Float comparisons
            (Value::Float(a), BinaryOperator::Equal, Value::Float(b)) => Some(Value::Bool((a - b).abs() < f64::EPSILON)),
            (Value::Float(a), BinaryOperator::NotEqual, Value::Float(b)) => Some(Value::Bool((a - b).abs() >= f64::EPSILON)),
            (Value::Float(a), BinaryOperator::LessThan, Value::Float(b)) => Some(Value::Bool(a < b)),
            (Value::Float(a), BinaryOperator::LessThanOrEqual, Value::Float(b)) => Some(Value::Bool(a <= b)),
            (Value::Float(a), BinaryOperator::GreaterThan, Value::Float(b)) => Some(Value::Bool(a > b)),
            (Value::Float(a), BinaryOperator::GreaterThanOrEqual, Value::Float(b)) => Some(Value::Bool(a >= b)),
            
            // String comparisons
            (Value::String(a), BinaryOperator::Equal, Value::String(b)) => Some(Value::Bool(a == b)),
            (Value::String(a), BinaryOperator::NotEqual, Value::String(b)) => Some(Value::Bool(a != b)),
            
            // Boolean operations
            (Value::Bool(a), BinaryOperator::And, Value::Bool(b)) => Some(Value::Bool(*a && *b)),
            (Value::Bool(a), BinaryOperator::Or, Value::Bool(b)) => Some(Value::Bool(*a || *b)),
            (Value::Bool(a), BinaryOperator::Equal, Value::Bool(b)) => Some(Value::Bool(a == b)),
            (Value::Bool(a), BinaryOperator::NotEqual, Value::Bool(b)) => Some(Value::Bool(a != b)),
            
            _ => None,
        }
    }

    fn evaluate_unary_operation(&self, op: &UnaryOperator, val: &Value) -> Option<Value> {
        match (op, val) {
            (UnaryOperator::Not, Value::Bool(b)) => Some(Value::Bool(!b)),
            (UnaryOperator::Minus, Value::Int(i)) => Some(Value::Int(-i)),
            (UnaryOperator::Minus, Value::Float(f)) => Some(Value::Float(-f)),
            (UnaryOperator::Plus, Value::Int(i)) => Some(Value::Int(*i)),
            (UnaryOperator::Plus, Value::Float(f)) => Some(Value::Float(*f)),
            _ => None,
        }
    }

    fn evaluate_function(&self, name: &str, args: &[CompiledExpression]) -> Option<Value> {
        match name.to_uppercase().as_str() {
            "ABS" if args.len() == 1 => {
                if let CompiledExpression::Literal(Value::Int(i)) = &args[0] {
                    Some(Value::Int(i.abs()))
                } else if let CompiledExpression::Literal(Value::Float(f)) = &args[0] {
                    Some(Value::Float(f.abs()))
                } else {
                    None
                }
            }
            "UPPER" if args.len() == 1 => {
                if let CompiledExpression::Literal(Value::String(s)) = &args[0] {
                    Some(Value::String(s.to_uppercase()))
                } else {
                    None
                }
            }
            "LOWER" if args.len() == 1 => {
                if let CompiledExpression::Literal(Value::String(s)) = &args[0] {
                    Some(Value::String(s.to_lowercase()))
                } else {
                    None
                }
            }
            "LENGTH" | "LEN" if args.len() == 1 => {
                if let CompiledExpression::Literal(Value::String(s)) = &args[0] {
                    Some(Value::Int(s.len() as i64))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::ValueType;

    fn create_int_literal(val: i64) -> CompiledExpression {
        CompiledExpression::Literal(Value::Int(val))
    }

    fn create_float_literal(val: f64) -> CompiledExpression {
        CompiledExpression::Literal(Value::Float(val))
    }

    fn create_bool_literal(val: bool) -> CompiledExpression {
        CompiledExpression::Literal(Value::Bool(val))
    }

    fn create_string_literal(val: &str) -> CompiledExpression {
        CompiledExpression::Literal(Value::String(val.to_string()))
    }

    #[test]
    fn test_constant_arithmetic_folding() {
        let expr = CompiledExpression::Binary {
            left: Box::new(create_int_literal(2)),
            op: BinaryOperator::Add,
            right: Box::new(create_int_literal(3)),
            result_type: ValueType::Int,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Literal(Value::Int(5)) => {},
            _ => panic!("Expected folded result: 5"),
        }
    }

    #[test]
    fn test_mixed_arithmetic_folding() {
        let expr = CompiledExpression::Binary {
            left: Box::new(create_int_literal(2)),
            op: BinaryOperator::Multiply,
            right: Box::new(create_float_literal(3.5)),
            result_type: ValueType::Float,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Literal(Value::Float(f)) if (f - 7.0).abs() < f64::EPSILON => {},
            _ => panic!("Expected folded result: 7.0"),
        }
    }

    #[test]
    fn test_boolean_short_circuit_folding() {
        // true OR x -> true
        let expr = CompiledExpression::Binary {
            left: Box::new(create_bool_literal(true)),
            op: BinaryOperator::Or,
            right: Box::new(CompiledExpression::Column {
                table: None,
                name: "unknown".to_string(),
                value_type: ValueType::Bool,
            }),
            result_type: ValueType::Bool,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Literal(Value::Bool(true)) => {},
            _ => panic!("Expected short-circuited true"),
        }
    }

    #[test]
    fn test_identity_operation_folding() {
        // x + 0 -> x
        let column_expr = CompiledExpression::Column {
            table: None,
            name: "value".to_string(),
            value_type: ValueType::Int,
        };
        let expr = CompiledExpression::Binary {
            left: Box::new(column_expr.clone()),
            op: BinaryOperator::Add,
            right: Box::new(create_int_literal(0)),
            result_type: ValueType::Int,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Column { name, .. } if name == "value" => {},
            _ => panic!("Expected identity optimization to return original column"),
        }
    }

    #[test]
    fn test_zero_multiplication_folding() {
        // x * 0 -> 0
        let expr = CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Column {
                table: None,
                name: "value".to_string(),
                value_type: ValueType::Int,
            }),
            op: BinaryOperator::Multiply,
            right: Box::new(create_int_literal(0)),
            result_type: ValueType::Int,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Literal(Value::Int(0)) => {},
            _ => panic!("Expected zero multiplication result"),
        }
    }

    #[test]
    fn test_double_negation_folding() {
        // NOT NOT x -> x
        let column_expr = CompiledExpression::Column {
            table: None,
            name: "active".to_string(),
            value_type: ValueType::Bool,
        };
        let expr = CompiledExpression::Unary {
            op: UnaryOperator::Not,
            expr: Box::new(CompiledExpression::Unary {
                op: UnaryOperator::Not,
                expr: Box::new(column_expr.clone()),
                result_type: ValueType::Bool,
            }),
            result_type: ValueType::Bool,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Column { name, .. } if name == "active" => {},
            _ => panic!("Expected double negation to be eliminated"),
        }
    }

    #[test]
    fn test_string_function_folding() {
        let expr = CompiledExpression::Function {
            name: "UPPER".to_string(),
            args: vec![create_string_literal("hello")],
            result_type: ValueType::String,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Literal(Value::String(s)) if s == "HELLO" => {},
            _ => panic!("Expected UPPER function to be folded"),
        }
    }

    #[test]
    fn test_plan_optimization_with_constant_folding() {
        let scan = ExecutionPlan::Scan {
            table: "users".to_string(),
            filter: Some(CompiledExpression::Binary {
                left: Box::new(CompiledExpression::Column {
                    table: None,
                    name: "age".to_string(),
                    value_type: ValueType::Int,
                }),
                op: BinaryOperator::GreaterThan,
                right: Box::new(CompiledExpression::Binary {
                    left: Box::new(create_int_literal(20)),
                    op: BinaryOperator::Add,
                    right: Box::new(create_int_literal(5)),
                    result_type: ValueType::Int,
                }),
                result_type: ValueType::Bool,
            }),
            projection: vec![],
        };

        let optimized = optimize(scan).unwrap();

        match optimized {
            ExecutionPlan::Scan { filter: Some(filter), .. } => {
                match filter {
                    CompiledExpression::Binary { right, .. } => {
                        match right.as_ref() {
                            CompiledExpression::Literal(Value::Int(25)) => {},
                            _ => panic!("Expected constant to be folded to 25"),
                        }
                    }
                    _ => panic!("Expected binary expression"),
                }
            }
            _ => panic!("Expected scan with filter"),
        }
    }

    #[test]
    fn test_complex_expression_partial_folding() {
        // (2 + 3) > column -> 5 > column
        let expr = CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Binary {
                left: Box::new(create_int_literal(2)),
                op: BinaryOperator::Add,
                right: Box::new(create_int_literal(3)),
                result_type: ValueType::Int,
            }),
            op: BinaryOperator::GreaterThan,
            right: Box::new(CompiledExpression::Column {
                table: None,
                name: "value".to_string(),
                value_type: ValueType::Int,
            }),
            result_type: ValueType::Bool,
        };

        let optimizer = ConstantFoldingOptimizer::new();
        let folded = optimizer.fold_expression(expr);

        match folded {
            CompiledExpression::Binary { left, op: BinaryOperator::GreaterThan, right, .. } => {
                match (left.as_ref(), right.as_ref()) {
                    (CompiledExpression::Literal(Value::Int(5)), CompiledExpression::Column { name, .. })
                        if name == "value" => {},
                    _ => panic!("Expected partial folding: 5 > column"),
                }
            }
            _ => panic!("Expected binary comparison"),
        }
    }
}
