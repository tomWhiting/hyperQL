use crate::compiler::{ExecutionPlan, CompiledExpression, ValueType};
use crate::error::Result;
use crate::ast::BinaryOperator;
use crate::types::Value;

pub fn optimize(plan: ExecutionPlan) -> Result<ExecutionPlan> {
    let mut optimizer = ExpressionSimplifyOptimizer::new();
    optimizer.optimize_plan(plan)
}

struct ExpressionSimplifyOptimizer {
}

impl ExpressionSimplifyOptimizer {
    fn new() -> Self {
        Self {}
    }

    fn optimize_plan(&mut self, plan: ExecutionPlan) -> Result<ExecutionPlan> {
        match plan {
            ExecutionPlan::Filter { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                let simplified_predicate = self.simplify_expression(predicate);
                
                // If predicate simplifies to always true, remove the filter
                if self.is_always_true(&simplified_predicate) {
                    return Ok(optimized_input);
                }
                
                // If predicate simplifies to always false, this could be optimized to empty result
                // For now, we keep the filter but mark it as such
                Ok(ExecutionPlan::Filter {
                    input: Box::new(optimized_input),
                    predicate: simplified_predicate,
                })
            }
            ExecutionPlan::Project { input, expressions, distinct } => {
                let optimized_input = self.optimize_plan(*input)?;
                let simplified_expressions = expressions
                    .into_iter()
                    .map(|mut proj| {
                        proj.expression = self.simplify_expression(proj.expression);
                        proj
                    })
                    .collect();
                Ok(ExecutionPlan::Project {
                    input: Box::new(optimized_input),
                    expressions: simplified_expressions,
                    distinct,
                })
            }
            ExecutionPlan::GroupBy { input, group_expressions, aggregate_expressions } => {
                let optimized_input = self.optimize_plan(*input)?;
                let simplified_group_expressions = group_expressions
                    .into_iter()
                    .map(|expr| self.simplify_expression(expr))
                    .collect();
                let simplified_aggregate_expressions = aggregate_expressions
                    .into_iter()
                    .map(|mut proj| {
                        proj.expression = self.simplify_expression(proj.expression);
                        proj
                    })
                    .collect();
                Ok(ExecutionPlan::GroupBy {
                    input: Box::new(optimized_input),
                    group_expressions: simplified_group_expressions,
                    aggregate_expressions: simplified_aggregate_expressions,
                })
            }
            ExecutionPlan::Having { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                let simplified_predicate = self.simplify_expression(predicate);
                
                if self.is_always_true(&simplified_predicate) {
                    return Ok(optimized_input);
                }
                
                Ok(ExecutionPlan::Having {
                    input: Box::new(optimized_input),
                    predicate: simplified_predicate,
                })
            }
            ExecutionPlan::Sort { input, sort_keys } => {
                let optimized_input = self.optimize_plan(*input)?;
                let simplified_sort_keys = sort_keys
                    .into_iter()
                    .map(|mut key| {
                        key.expression = self.simplify_expression(key.expression);
                        key
                    })
                    .collect();
                Ok(ExecutionPlan::Sort {
                    input: Box::new(optimized_input),
                    sort_keys: simplified_sort_keys,
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
            ExecutionPlan::Scan { table, entity_type, alias, filter, projection, limit } => {
                let simplified_filter = filter.map(|f| self.simplify_expression(f));
                let simplified_projection = projection
                    .into_iter()
                    .map(|mut proj| {
                        proj.expression = self.simplify_expression(proj.expression);
                        proj
                    })
                    .collect();
                Ok(ExecutionPlan::Scan {
                    entity_type,
                    table,
                    alias,
                    filter: simplified_filter,
                    projection: simplified_projection,
                    limit,
                })
            }
            other => Ok(other),
        }
    }

    fn simplify_expression(&self, expr: CompiledExpression) -> CompiledExpression {
        match expr {
            CompiledExpression::Binary { left, op, right, result_type } => {
                let simplified_left = self.simplify_expression(*left);
                let simplified_right = self.simplify_expression(*right);
                
                self.simplify_binary_expression(simplified_left, op, simplified_right, result_type)
            }
            CompiledExpression::Function { name, args, result_type } => {
                let simplified_args: Vec<_> = args.into_iter().map(|arg| self.simplify_expression(arg)).collect();
                CompiledExpression::Function {
                    name,
                    args: simplified_args,
                    result_type,
                }
            }
            other => other,
        }
    }

    fn simplify_binary_expression(
        &self,
        left: CompiledExpression,
        op: BinaryOperator,
        right: CompiledExpression,
        result_type: ValueType
    ) -> CompiledExpression {
        match (&left, &op, &right) {
            // Duplicate conditions: x = x, x > x, etc.
            (l, BinaryOperator::Equal, r) if self.expressions_equal(l, r) => {
                CompiledExpression::Literal(Value::Bool(true))
            }
            (l, BinaryOperator::NotEqual, r) if self.expressions_equal(l, r) => {
                CompiledExpression::Literal(Value::Bool(false))
            }
            (l, BinaryOperator::LessThan, r) if self.expressions_equal(l, r) => {
                CompiledExpression::Literal(Value::Bool(false))
            }
            (l, BinaryOperator::LessThanOrEqual, r) if self.expressions_equal(l, r) => {
                CompiledExpression::Literal(Value::Bool(true))
            }
            (l, BinaryOperator::GreaterThan, r) if self.expressions_equal(l, r) => {
                CompiledExpression::Literal(Value::Bool(false))
            }
            (l, BinaryOperator::GreaterThanOrEqual, r) if self.expressions_equal(l, r) => {
                CompiledExpression::Literal(Value::Bool(true))
            }
            
            // Simplify AND/OR with duplicate operands: x AND x -> x, x OR x -> x
            (l, BinaryOperator::And, r) if self.expressions_equal(l, r) => l.clone(),
            (l, BinaryOperator::Or, r) if self.expressions_equal(l, r) => l.clone(),
            
            // Contradiction detection: x = a AND x = b (where a != b) -> false
            (CompiledExpression::Binary { left: l1, op: BinaryOperator::Equal, right: r1, .. },
             BinaryOperator::And,
             CompiledExpression::Binary { left: l2, op: BinaryOperator::Equal, right: r2, .. }) => {
                if self.expressions_equal(l1, l2) {
                    match (r1.as_ref(), r2.as_ref()) {
                        (CompiledExpression::Literal(v1), CompiledExpression::Literal(v2)) if !self.values_equal(v1, v2) => {
                            return CompiledExpression::Literal(Value::Bool(false));
                        }
                        _ => {}
                    }
                }
                self.try_combine_and_conditions(left.clone(), right.clone(), result_type)
            }
            
            // Range simplification: x > a AND x > b -> x > max(a, b)
            (CompiledExpression::Binary { left: l1, op: BinaryOperator::GreaterThan, right: r1, .. },
             BinaryOperator::And,
             CompiledExpression::Binary { left: l2, op: BinaryOperator::GreaterThan, right: r2, .. }) => {
                if self.expressions_equal(l1, l2) {
                    if let (CompiledExpression::Literal(v1), CompiledExpression::Literal(v2)) = (r1.as_ref(), r2.as_ref()) {
                        let max_val = self.max_value(v1, v2);
                        if let Some(max) = max_val {
                            return CompiledExpression::Binary {
                                left: l1.clone(),
                                op: BinaryOperator::GreaterThan,
                                right: Box::new(CompiledExpression::Literal(max)),
                                result_type: ValueType::Bool,
                            };
                        }
                    }
                }
                CompiledExpression::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    result_type,
                }
            }
            
            // Range simplification: x < a AND x < b -> x < min(a, b)
            (CompiledExpression::Binary { left: l1, op: BinaryOperator::LessThan, right: r1, .. },
             BinaryOperator::And,
             CompiledExpression::Binary { left: l2, op: BinaryOperator::LessThan, right: r2, .. }) => {
                if self.expressions_equal(l1, l2) {
                    if let (CompiledExpression::Literal(v1), CompiledExpression::Literal(v2)) = (r1.as_ref(), r2.as_ref()) {
                        let min_val = self.min_value(v1, v2);
                        if let Some(min) = min_val {
                            return CompiledExpression::Binary {
                                left: l1.clone(),
                                op: BinaryOperator::LessThan,
                                right: Box::new(CompiledExpression::Literal(min)),
                                result_type: ValueType::Bool,
                            };
                        }
                    }
                }
                CompiledExpression::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    result_type,
                }
            }
            
            // OR simplification: x > a OR x > b -> x > min(a, b)
            (CompiledExpression::Binary { left: l1, op: BinaryOperator::GreaterThan, right: r1, .. },
             BinaryOperator::Or,
             CompiledExpression::Binary { left: l2, op: BinaryOperator::GreaterThan, right: r2, .. }) => {
                if self.expressions_equal(l1, l2) {
                    if let (CompiledExpression::Literal(v1), CompiledExpression::Literal(v2)) = (r1.as_ref(), r2.as_ref()) {
                        let min_val = self.min_value(v1, v2);
                        if let Some(min) = min_val {
                            return CompiledExpression::Binary {
                                left: l1.clone(),
                                op: BinaryOperator::GreaterThan,
                                right: Box::new(CompiledExpression::Literal(min)),
                                result_type: ValueType::Bool,
                            };
                        }
                    }
                }
                CompiledExpression::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    result_type,
                }
            }

            // General AND deduplication: flatten and deduplicate conditions
            (_, BinaryOperator::And, _) => {
                self.try_combine_and_conditions(left.clone(), right.clone(), result_type)
            }

            _ => {
                CompiledExpression::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    result_type,
                }
            }
        }
    }

    fn try_combine_and_conditions(
        &self,
        left: CompiledExpression,
        right: CompiledExpression,
        result_type: ValueType
    ) -> CompiledExpression {
        // Try to find redundant conditions in AND expressions
        let left_conditions = self.extract_and_conditions(&left);
        let right_conditions = self.extract_and_conditions(&right);

        let mut all_conditions = Vec::new();

        // Add left conditions
        for condition in left_conditions {
            if !all_conditions.iter().any(|existing| self.expressions_equal(existing, &condition)) {
                all_conditions.push(condition);
            }
        }

        // Add right conditions, avoiding duplicates
        for condition in right_conditions {
            if !all_conditions.iter().any(|existing| self.expressions_equal(existing, &condition)) {
                all_conditions.push(condition);
            }
        }

        if all_conditions.len() == 1 {
            all_conditions.into_iter().next().unwrap()
        } else {
            self.build_and_expression(all_conditions, result_type)
        }
    }

    fn extract_and_conditions(&self, expr: &CompiledExpression) -> Vec<CompiledExpression> {
        match expr {
            CompiledExpression::Binary { left, op: BinaryOperator::And, right, .. } => {
                let mut conditions = self.extract_and_conditions(left);
                conditions.extend(self.extract_and_conditions(right));
                conditions
            }
            _ => vec![expr.clone()],
        }
    }

    fn build_and_expression(
        &self,
        conditions: Vec<CompiledExpression>,
        _result_type: ValueType
    ) -> CompiledExpression {
        conditions.into_iter().reduce(|acc, condition| {
            CompiledExpression::Binary {
                left: Box::new(acc),
                op: BinaryOperator::And,
                right: Box::new(condition),
                result_type: ValueType::Bool,
            }
        }).unwrap_or(CompiledExpression::Literal(Value::Bool(true)))
    }

    fn expressions_equal(&self, left: &CompiledExpression, right: &CompiledExpression) -> bool {
        match (left, right) {
            (CompiledExpression::Column { name: n1, table: t1, .. },
             CompiledExpression::Column { name: n2, table: t2, .. }) => {
                n1 == n2 && t1 == t2
            }
            (CompiledExpression::Literal(v1), CompiledExpression::Literal(v2)) => {
                self.values_equal(v1, v2)
            }
            (CompiledExpression::Binary { left: l1, op: op1, right: r1, .. },
             CompiledExpression::Binary { left: l2, op: op2, right: r2, .. }) => {
                op1 == op2 && self.expressions_equal(l1, l2) && self.expressions_equal(r1, r2)
            }
            _ => false,
        }
    }

    fn values_equal(&self, v1: &Value, v2: &Value) -> bool {
        match (v1, v2) {
            (Value::Int(i1), Value::Int(i2)) => i1 == i2,
            (Value::Float(f1), Value::Float(f2)) => (f1 - f2).abs() < f64::EPSILON,
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Bool(b1), Value::Bool(b2)) => b1 == b2,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn min_value(&self, v1: &Value, v2: &Value) -> Option<Value> {
        match (v1, v2) {
            (Value::Int(i1), Value::Int(i2)) => Some(Value::Int(*i1.min(i2))),
            (Value::Float(f1), Value::Float(f2)) => Some(Value::Float(f1.min(*f2))),
            (Value::Int(i), Value::Float(f)) => Some(Value::Float((*i as f64).min(*f))),
            (Value::Float(f), Value::Int(i)) => Some(Value::Float(f.min(*i as f64))),
            _ => None,
        }
    }

    fn max_value(&self, v1: &Value, v2: &Value) -> Option<Value> {
        match (v1, v2) {
            (Value::Int(i1), Value::Int(i2)) => Some(Value::Int(*i1.max(i2))),
            (Value::Float(f1), Value::Float(f2)) => Some(Value::Float(f1.max(*f2))),
            (Value::Int(i), Value::Float(f)) => Some(Value::Float((*i as f64).max(*f))),
            (Value::Float(f), Value::Int(i)) => Some(Value::Float(f.max(*i as f64))),
            _ => None,
        }
    }

    fn is_always_true(&self, expr: &CompiledExpression) -> bool {
        matches!(expr, CompiledExpression::Literal(Value::Bool(true)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_column_expr(name: &str) -> CompiledExpression {
        CompiledExpression::Column {
            table: None,
            name: name.to_string(),
            value_type: ValueType::Int,
        }
    }

    fn create_int_literal(val: i64) -> CompiledExpression {
        CompiledExpression::Literal(Value::Int(val))
    }

    fn create_bool_literal(val: bool) -> CompiledExpression {
        CompiledExpression::Literal(Value::Bool(val))
    }

    #[test]
    fn test_duplicate_condition_simplification() {
        // x = x -> true
        let expr = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::Equal,
            right: Box::new(create_column_expr("x")),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr);

        match simplified {
            CompiledExpression::Literal(Value::Bool(true)) => {},
            _ => panic!("Expected x = x to simplify to true"),
        }
    }

    #[test]
    fn test_duplicate_and_simplification() {
        // x > 5 AND x > 5 -> x > 5
        let condition = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_int_literal(5)),
            result_type: ValueType::Bool,
        };
        let expr = CompiledExpression::Binary {
            left: Box::new(condition.clone()),
            op: BinaryOperator::And,
            right: Box::new(condition),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr);

        match simplified {
            CompiledExpression::Binary { left, op: BinaryOperator::GreaterThan, right, .. } => {
                match (left.as_ref(), right.as_ref()) {
                    (CompiledExpression::Column { name, .. }, CompiledExpression::Literal(Value::Int(5))) 
                        if name == "x" => {},
                    _ => panic!("Expected x > 5"),
                }
            }
            _ => panic!("Expected simplified condition"),
        }
    }

    #[test]
    fn test_range_and_simplification() {
        // x > 5 AND x > 10 -> x > 10
        let condition1 = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_int_literal(5)),
            result_type: ValueType::Bool,
        };
        let condition2 = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_int_literal(10)),
            result_type: ValueType::Bool,
        };
        let expr = CompiledExpression::Binary {
            left: Box::new(condition1),
            op: BinaryOperator::And,
            right: Box::new(condition2),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr);

        match &simplified {
            CompiledExpression::Binary { left, op: BinaryOperator::GreaterThan, right, .. } => {
                match (left.as_ref(), right.as_ref()) {
                    (CompiledExpression::Column { name, .. }, CompiledExpression::Literal(Value::Int(10)))
                        if name == "x" => {},
                    _ => panic!("Expected x > 10, got: {:?}", simplified),
                }
            }
            _ => panic!("Expected x > 10, got: {:?}", simplified),
        }
    }

    #[test]
    fn test_range_or_simplification() {
        // x > 5 OR x > 10 -> x > 5
        let condition1 = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_int_literal(5)),
            result_type: ValueType::Bool,
        };
        let condition2 = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_int_literal(10)),
            result_type: ValueType::Bool,
        };
        let expr = CompiledExpression::Binary {
            left: Box::new(condition1),
            op: BinaryOperator::Or,
            right: Box::new(condition2),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr);

        match simplified {
            CompiledExpression::Binary { left, op: BinaryOperator::GreaterThan, right, .. } => {
                match (left.as_ref(), right.as_ref()) {
                    (CompiledExpression::Column { name, .. }, CompiledExpression::Literal(Value::Int(5))) 
                        if name == "x" => {},
                    _ => panic!("Expected x > 5"),
                }
            }
            _ => panic!("Expected simplified condition"),
        }
    }

    #[test]
    fn test_contradiction_detection() {
        // x = 5 AND x = 10 -> false
        let condition1 = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::Equal,
            right: Box::new(create_int_literal(5)),
            result_type: ValueType::Bool,
        };
        let condition2 = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::Equal,
            right: Box::new(create_int_literal(10)),
            result_type: ValueType::Bool,
        };
        let expr = CompiledExpression::Binary {
            left: Box::new(condition1),
            op: BinaryOperator::And,
            right: Box::new(condition2),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr);

        match simplified {
            CompiledExpression::Literal(Value::Bool(false)) => {},
            _ => panic!("Expected contradiction to simplify to false"),
        }
    }

    #[test]
    fn test_filter_removal_when_always_true() {
        let scan = ExecutionPlan::Scan {
            table: "users".to_string(),
            entity_type: String::new(),
            alias: None,
            filter: None,
            projection: vec![],
            limit: None,
        };
        let filter = ExecutionPlan::Filter {
            input: Box::new(scan.clone()),
            predicate: create_bool_literal(true),
        };

        let optimized = optimize(filter).unwrap();

        match optimized {
            ExecutionPlan::Scan { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected filter with always-true predicate to be removed"),
        }
    }

    #[test]
    fn test_complex_condition_deduplication() {
        // (x > 5 AND y < 10) AND (x > 5 AND z = 'test') -> x > 5 AND y < 10 AND z = 'test'
        let condition_x = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_int_literal(5)),
            result_type: ValueType::Bool,
        };
        let condition_y = CompiledExpression::Binary {
            left: Box::new(create_column_expr("y")),
            op: BinaryOperator::LessThan,
            right: Box::new(create_int_literal(10)),
            result_type: ValueType::Bool,
        };
        let condition_z = CompiledExpression::Binary {
            left: Box::new(create_column_expr("z")),
            op: BinaryOperator::Equal,
            right: Box::new(CompiledExpression::Literal(Value::String("test".to_string()))),
            result_type: ValueType::Bool,
        };
        
        let left_group = CompiledExpression::Binary {
            left: Box::new(condition_x.clone()),
            op: BinaryOperator::And,
            right: Box::new(condition_y),
            result_type: ValueType::Bool,
        };
        let right_group = CompiledExpression::Binary {
            left: Box::new(condition_x),
            op: BinaryOperator::And,
            right: Box::new(condition_z),
            result_type: ValueType::Bool,
        };
        let expr = CompiledExpression::Binary {
            left: Box::new(left_group),
            op: BinaryOperator::And,
            right: Box::new(right_group),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr);

        // The result should have x > 5 appearing only once
        let conditions = optimizer.extract_and_conditions(&simplified);
        let x_conditions: Vec<_> = conditions.iter().filter(|c| {
            matches!(c, CompiledExpression::Binary { left, op: BinaryOperator::GreaterThan, .. }
                if matches!(left.as_ref(), CompiledExpression::Column { name, .. } if name == "x"))
        }).collect();
        
        assert_eq!(x_conditions.len(), 1, "x > 5 condition should appear only once");
    }

    #[test]
    fn test_no_change_when_no_simplification_possible() {
        let expr = CompiledExpression::Binary {
            left: Box::new(create_column_expr("x")),
            op: BinaryOperator::GreaterThan,
            right: Box::new(create_column_expr("y")),
            result_type: ValueType::Bool,
        };

        let optimizer = ExpressionSimplifyOptimizer::new();
        let simplified = optimizer.simplify_expression(expr.clone());

        match (expr, simplified) {
            (CompiledExpression::Binary { left: l1, op: op1, right: r1, .. },
             CompiledExpression::Binary { left: l2, op: op2, right: r2, .. }) => {
                assert_eq!(op1, op2);
                assert!(optimizer.expressions_equal(&l1, &l2));
                assert!(optimizer.expressions_equal(&r1, &r2));
            }
            _ => panic!("Expected no change in expression structure"),
        }
    }
}
