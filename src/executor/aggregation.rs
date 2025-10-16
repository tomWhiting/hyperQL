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
        for (group_key_str, group_rows) in groups {
            let mut aggregated_columns = HashMap::new();

            for (i, _group_expr) in group_expressions.iter().enumerate() {
                aggregated_columns.insert(
                    format!("group_{}", i),
                    Value::String(group_key_str.clone()),
                );
            }

            for aggregate_projection in aggregate_expressions {
                let agg_value = self.evaluate_aggregate(&aggregate_projection.expression, &group_rows, evaluator)?;
                let column_name = if let Some(ref alias) = aggregate_projection.alias {
                    alias.clone()
                } else {
                    format!("agg_{}", aggregated_columns.len())
                };
                aggregated_columns.insert(column_name.clone(), agg_value);
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
