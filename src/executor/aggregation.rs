use crate::compiler::{CompiledExpression, CompiledProjection};
use crate::types::{ResultRow, Value};
use crate::error::Result;
use super::expression_eval::ExpressionEvaluator;
use std::collections::HashMap;

pub struct AggregationEngine {
}

impl AggregationEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn group_and_aggregate(
        &self,
        rows: Vec<ResultRow>,
        group_expressions: &[CompiledExpression],
        aggregate_expressions: &[CompiledProjection],
        evaluator: &ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        if group_expressions.is_empty() {
            return self.aggregate_without_grouping(rows, aggregate_expressions, evaluator);
        }

        let mut groups: HashMap<String, Vec<ResultRow>> = HashMap::new();

        for row in rows {
            let mut group_key_parts = Vec::new();
            for group_expr in group_expressions {
                let group_value = evaluator.evaluate_expression(group_expr, &row)?;
                group_key_parts.push(format!("{:?}", group_value));
            }
            let group_key = group_key_parts.join("||");
            groups.entry(group_key).or_insert_with(Vec::new).push(row);
        }

        let mut result = Vec::new();
        for (_group_key_str, group_rows) in groups {
            let mut aggregated_columns = HashMap::new();

            // Extract actual group values from the first row in the group
            // This preserves the original column names and values
            if let Some(first_row) = group_rows.first() {
                for group_expr in group_expressions {
                    let group_value = evaluator.evaluate_expression(group_expr, first_row)?;
                    let column_name = self.extract_column_name(group_expr, aggregate_expressions);
                    aggregated_columns.insert(column_name, group_value);
                }
            }

            for aggregate_projection in aggregate_expressions {
                // Skip if this projection is actually a GROUP BY column
                // (we already added it above with the correct value)
                let is_group_column = group_expressions.iter().any(|group_expr| {
                    self.expressions_match(group_expr, &aggregate_projection.expression)
                });

                if !is_group_column {
                    let agg_value = self.evaluate_aggregate(&aggregate_projection.expression, &group_rows, evaluator)?;
                    let column_name = if let Some(ref alias) = aggregate_projection.alias {
                        alias.clone()
                    } else {
                        aggregate_projection.output_name.clone()
                    };
                    aggregated_columns.insert(column_name.clone(), agg_value);
                }
            }

            result.push(ResultRow { columns: aggregated_columns });
        }

        Ok(result)
    }

    fn aggregate_without_grouping(
        &self,
        rows: Vec<ResultRow>,
        aggregate_expressions: &[CompiledProjection],
        evaluator: &ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        let mut aggregated_columns = HashMap::new();

        for aggregate_projection in aggregate_expressions {
            let agg_value = self.evaluate_aggregate(&aggregate_projection.expression, &rows, evaluator)?;
            let column_name = if let Some(ref alias) = aggregate_projection.alias {
                alias.clone()
            } else {
                // Use the output_name from projection for better column naming
                aggregate_projection.output_name.clone()
            };
            aggregated_columns.insert(column_name.clone(), agg_value);
        }

        Ok(vec![ResultRow { columns: aggregated_columns }])
    }

    /// Extract a meaningful column name from a CompiledExpression
    ///
    /// For GROUP BY columns, we need to preserve the original column names
    /// that match the SELECT list. This function extracts the column name
    /// from the expression, handling simple columns, qualified columns,
    /// and complex expressions.
    fn extract_column_name(
        &self,
        expr: &CompiledExpression,
        aggregate_expressions: &[CompiledProjection],
    ) -> String {
        // First, try to find this expression in the aggregate_expressions
        // If found, use its output_name (which comes from the SELECT list)
        for agg_proj in aggregate_expressions {
            if self.expressions_match(expr, &agg_proj.expression) {
                return agg_proj.output_name.clone();
            }
        }

        // If not found in aggregate expressions, generate a name from the expression itself
        match expr {
            CompiledExpression::Column { table, name, .. } => {
                if let Some(table_name) = table {
                    format!("{}_{}", table_name, name)
                } else {
                    name.clone()
                }
            }
            CompiledExpression::Function { name, .. } => {
                format!("{}()", name)
            }
            _ => "expr".to_string(),
        }
    }

    /// Check if two expressions are semantically equivalent
    /// Used to match GROUP BY expressions with SELECT list expressions
    fn expressions_match(&self, expr1: &CompiledExpression, expr2: &CompiledExpression) -> bool {
        match (expr1, expr2) {
            (
                CompiledExpression::Column { table: t1, name: n1, .. },
                CompiledExpression::Column { table: t2, name: n2, .. }
            ) => t1 == t2 && n1 == n2,
            (
                CompiledExpression::Function { name: n1, args: a1, .. },
                CompiledExpression::Function { name: n2, args: a2, .. }
            ) => {
                n1 == n2 && a1.len() == a2.len() &&
                a1.iter().zip(a2.iter()).all(|(arg1, arg2)| self.expressions_match(arg1, arg2))
            }
            (CompiledExpression::Literal(v1), CompiledExpression::Literal(v2)) => v1 == v2,
            _ => false,
        }
    }

    fn evaluate_aggregate(
        &self,
        expr: &CompiledExpression,
        rows: &[ResultRow],
        evaluator: &ExpressionEvaluator,
    ) -> Result<Value> {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                match name.to_uppercase().as_str() {
                    "COUNT" => {
                        // COUNT(*) or COUNT() - count all rows
                        // Check for wildcard first to avoid evaluation errors
                        if args.is_empty() {
                            return Ok(Value::Int(rows.len() as i64));
                        }

                        // Check if first arg is wildcard column "*"
                        if let CompiledExpression::Column { name, .. } = &args[0] {
                            if name == "*" {
                                return Ok(Value::Int(rows.len() as i64));
                            }
                        }

                        // COUNT(column) - count non-null values
                        let mut count = 0;
                        for row in rows {
                            if evaluator.evaluate_expression(&args[0], row).is_ok() {
                                count += 1;
                            }
                        }
                        Ok(Value::Int(count))
                    },
                    "SUM" => {
                        let mut sum = 0.0;
                        let mut has_values = false;
                        for row in rows {
                            if let Ok(value) = evaluator.evaluate_expression(&args[0], row) {
                                match value {
                                    Value::Int(i) => {
                                        sum += i as f64;
                                        has_values = true;
                                    },
                                    Value::Float(f) => {
                                        sum += f;
                                        has_values = true;
                                    },
                                    _ => {}
                                }
                            }
                        }
                        if has_values {
                            if sum.fract() == 0.0 {
                                Ok(Value::Int(sum as i64))
                            } else {
                                Ok(Value::Float(sum))
                            }
                        } else {
                            Ok(Value::Null)
                        }
                    },
                    "AVG" => {
                        let mut sum = 0.0;
                        let mut count = 0;
                        for row in rows {
                            if let Ok(value) = evaluator.evaluate_expression(&args[0], row) {
                                match value {
                                    Value::Int(i) => {
                                        sum += i as f64;
                                        count += 1;
                                    },
                                    Value::Float(f) => {
                                        sum += f;
                                        count += 1;
                                    },
                                    _ => {}
                                }
                            }
                        }
                        if count > 0 {
                            Ok(Value::Float(sum / count as f64))
                        } else {
                            Ok(Value::Null)
                        }
                    },
                    "MIN" => {
                        let mut min_val: Option<Value> = None;
                        for row in rows {
                            if let Ok(value) = evaluator.evaluate_expression(&args[0], row) {
                                if let Some(ref current_min) = min_val {
                                    if evaluator.compare_values(&value, current_min) == std::cmp::Ordering::Less {
                                        min_val = Some(value);
                                    }
                                } else {
                                    min_val = Some(value);
                                }
                            }
                        }
                        Ok(min_val.unwrap_or(Value::Null))
                    },
                    "MAX" => {
                        let mut max_val: Option<Value> = None;
                        for row in rows {
                            if let Ok(value) = evaluator.evaluate_expression(&args[0], row) {
                                if let Some(ref current_max) = max_val {
                                    if evaluator.compare_values(&value, current_max) == std::cmp::Ordering::Greater {
                                        max_val = Some(value);
                                    }
                                } else {
                                    max_val = Some(value);
                                }
                            }
                        }
                        Ok(max_val.unwrap_or(Value::Null))
                    },
                    _ => evaluator.evaluate_expression(expr, &ResultRow { columns: HashMap::new() })
                }
            },
            _ => evaluator.evaluate_expression(expr, &ResultRow { columns: HashMap::new() })
        }
    }
}
