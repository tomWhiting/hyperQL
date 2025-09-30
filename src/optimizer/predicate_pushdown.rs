use crate::compiler::{ExecutionPlan, CompiledExpression};
use crate::error::Result;
use crate::ast::BinaryOperator;

pub fn optimize(plan: ExecutionPlan) -> Result<ExecutionPlan> {
    let mut optimizer = PredicatePushdownOptimizer::new();
    optimizer.optimize_plan(plan)
}

struct PredicatePushdownOptimizer {
}

impl PredicatePushdownOptimizer {
    fn new() -> Self {
        Self {}
    }

    fn optimize_plan(&mut self, plan: ExecutionPlan) -> Result<ExecutionPlan> {
        match plan {
            ExecutionPlan::Filter { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                self.push_predicate(optimized_input, predicate)
            }
            ExecutionPlan::Project { input, expressions } => {
                let optimized_input = self.optimize_plan(*input)?;
                Ok(ExecutionPlan::Project {
                    input: Box::new(optimized_input),
                    expressions,
                })
            }
            ExecutionPlan::GroupBy { input, group_expressions, aggregate_expressions } => {
                let optimized_input = self.optimize_plan(*input)?;
                Ok(ExecutionPlan::GroupBy {
                    input: Box::new(optimized_input),
                    group_expressions,
                    aggregate_expressions,
                })
            }
            ExecutionPlan::Having { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                Ok(ExecutionPlan::Having {
                    input: Box::new(optimized_input),
                    predicate,
                })
            }
            ExecutionPlan::Sort { input, sort_keys } => {
                let optimized_input = self.optimize_plan(*input)?;
                Ok(ExecutionPlan::Sort {
                    input: Box::new(optimized_input),
                    sort_keys,
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
            other => Ok(other),
        }
    }

    fn push_predicate(&mut self, plan: ExecutionPlan, predicate: CompiledExpression) -> Result<ExecutionPlan> {
        match plan {
            ExecutionPlan::Scan { table, filter, projection } => {
                let merged_filter = match filter {
                    Some(existing_filter) => {
                        Some(self.merge_filters(existing_filter, predicate))
                    }
                    None => Some(predicate),
                };
                Ok(ExecutionPlan::Scan {
                    table,
                    filter: merged_filter,
                    projection,
                })
            }
            ExecutionPlan::Filter { input, predicate: existing_predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                let merged_predicate = self.merge_filters(existing_predicate, predicate);
                self.push_predicate(optimized_input, merged_predicate)
            }
            ExecutionPlan::Project { input, expressions } => {
                if self.predicate_references_only_available_columns(&predicate, &expressions) {
                    let optimized_input = self.push_predicate(*input, predicate)?;
                    Ok(ExecutionPlan::Project {
                        input: Box::new(optimized_input),
                        expressions,
                    })
                } else {
                    Ok(ExecutionPlan::Filter {
                        input: Box::new(ExecutionPlan::Project {
                            input,
                            expressions,
                        }),
                        predicate,
                    })
                }
            }
            _ => {
                Ok(ExecutionPlan::Filter {
                    input: Box::new(plan),
                    predicate,
                })
            }
        }
    }

    fn merge_filters(&self, left: CompiledExpression, right: CompiledExpression) -> CompiledExpression {
        CompiledExpression::Binary {
            left: Box::new(left),
            op: BinaryOperator::And,
            right: Box::new(right),
            result_type: crate::compiler::ValueType::Bool,
        }
    }

    fn predicate_references_only_available_columns(
        &self, 
        predicate: &CompiledExpression, 
        available_projections: &[crate::compiler::CompiledProjection]
    ) -> bool {
        let referenced_columns = self.extract_column_references(predicate);
        let available_columns: std::collections::HashSet<String> = available_projections
            .iter()
            .map(|p| p.output_name.clone())
            .collect();

        referenced_columns.iter().all(|col| available_columns.contains(col))
    }

    fn extract_column_references(&self, expr: &CompiledExpression) -> std::collections::HashSet<String> {
        let mut columns = std::collections::HashSet::new();
        self.extract_column_references_recursive(expr, &mut columns);
        columns
    }

    fn extract_column_references_recursive(
        &self,
        expr: &CompiledExpression,
        columns: &mut std::collections::HashSet<String>
    ) {
        match expr {
            CompiledExpression::Column { name, .. } => {
                columns.insert(name.clone());
            }
            CompiledExpression::Binary { left, right, .. } => {
                self.extract_column_references_recursive(left, columns);
                self.extract_column_references_recursive(right, columns);
            }
            CompiledExpression::Unary { expr, .. } => {
                self.extract_column_references_recursive(expr, columns);
            }
            CompiledExpression::Function { args, .. } => {
                for arg in args {
                    self.extract_column_references_recursive(arg, columns);
                }
            }
            CompiledExpression::Literal(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BinaryOperator;
    use crate::compiler::{CompiledProjection, ValueType};
    use crate::types::Value;

    fn create_simple_scan() -> ExecutionPlan {
        ExecutionPlan::Scan {
            table: "users".to_string(),
            filter: None,
            projection: vec![],
        }
    }

    fn create_age_filter() -> CompiledExpression {
        CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Column {
                table: None,
                name: "age".to_string(),
                value_type: ValueType::Int,
            }),
            op: BinaryOperator::GreaterThan,
            right: Box::new(CompiledExpression::Literal(Value::Int(25))),
            result_type: ValueType::Bool,
        }
    }

    fn create_name_filter() -> CompiledExpression {
        CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Column {
                table: None,
                name: "name".to_string(),
                value_type: ValueType::String,
            }),
            op: BinaryOperator::Equal,
            right: Box::new(CompiledExpression::Literal(Value::String("Alice".to_string()))),
            result_type: ValueType::Bool,
        }
    }

    #[test]
    fn test_push_filter_to_scan() {
        let scan = create_simple_scan();
        let filter = ExecutionPlan::Filter {
            input: Box::new(scan),
            predicate: create_age_filter(),
        };

        let optimized = optimize(filter).unwrap();

        match optimized {
            ExecutionPlan::Scan { table, filter, .. } => {
                assert_eq!(table, "users");
                assert!(filter.is_some());
            }
            _ => panic!("Expected scan with filter"),
        }
    }

    #[test]
    fn test_merge_multiple_filters() {
        let scan = create_simple_scan();
        let inner_filter = ExecutionPlan::Filter {
            input: Box::new(scan),
            predicate: create_age_filter(),
        };
        let outer_filter = ExecutionPlan::Filter {
            input: Box::new(inner_filter),
            predicate: create_name_filter(),
        };

        let optimized = optimize(outer_filter).unwrap();

        match optimized {
            ExecutionPlan::Scan { table, filter, .. } => {
                assert_eq!(table, "users");
                assert!(filter.is_some());
                if let Some(CompiledExpression::Binary { op: BinaryOperator::And, .. }) = filter {
                } else {
                    panic!("Expected AND operation in merged filter");
                }
            }
            _ => panic!("Expected scan with merged filters"),
        }
    }

    #[test]
    fn test_push_through_project_when_possible() {
        let scan = create_simple_scan();
        let project = ExecutionPlan::Project {
            input: Box::new(scan),
            expressions: vec![
                CompiledProjection {
                    expression: CompiledExpression::Column {
                        table: None,
                        name: "age".to_string(),
                        value_type: ValueType::Int,
                    },
                    alias: None,
                    output_name: "age".to_string(),
                },
            ],
        };
        let filter = ExecutionPlan::Filter {
            input: Box::new(project),
            predicate: create_age_filter(),
        };

        let optimized = optimize(filter).unwrap();

        match optimized {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Scan { filter, .. } => {
                        assert!(filter.is_some(), "Filter should be pushed to scan");
                    }
                    _ => panic!("Expected scan as input to project"),
                }
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_cannot_push_through_project_when_columns_unavailable() {
        let scan = create_simple_scan();
        let project = ExecutionPlan::Project {
            input: Box::new(scan),
            expressions: vec![
                CompiledProjection {
                    expression: CompiledExpression::Column {
                        table: None,
                        name: "name".to_string(),
                        value_type: ValueType::String,
                    },
                    alias: None,
                    output_name: "name".to_string(),
                },
            ],
        };
        let filter = ExecutionPlan::Filter {
            input: Box::new(project),
            predicate: create_age_filter(), // References 'age' which is not projected
        };

        let optimized = optimize(filter).unwrap();

        match optimized {
            ExecutionPlan::Filter { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Project { .. } => {
                        // Filter should remain above project
                    }
                    _ => panic!("Expected project as input to filter"),
                }
            }
            _ => panic!("Expected filter plan"),
        }
    }

    #[test]
    fn test_no_change_when_no_optimization_possible() {
        let scan = create_simple_scan();
        let optimized = optimize(scan.clone()).unwrap();
        
        match (scan, optimized) {
            (ExecutionPlan::Scan { table: t1, .. }, ExecutionPlan::Scan { table: t2, .. }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Expected identical scan plans"),
        }
    }
}
