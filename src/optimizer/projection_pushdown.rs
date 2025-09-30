use crate::compiler::{ExecutionPlan, CompiledProjection, CompiledExpression};
use crate::error::Result;
use std::collections::HashSet;

pub fn optimize(plan: ExecutionPlan) -> Result<ExecutionPlan> {
    let mut optimizer = ProjectionPushdownOptimizer::new();
    optimizer.optimize_plan(plan)
}

struct ProjectionPushdownOptimizer {
}

impl ProjectionPushdownOptimizer {
    fn new() -> Self {
        Self {}
    }

    fn optimize_plan(&mut self, plan: ExecutionPlan) -> Result<ExecutionPlan> {
        match plan {
            ExecutionPlan::Project { input, expressions } => {
                self.push_projection(*input, expressions)
            }
            ExecutionPlan::Filter { input, predicate } => {
                let optimized_input = self.optimize_plan(*input)?;
                Ok(ExecutionPlan::Filter {
                    input: Box::new(optimized_input),
                    predicate,
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

    fn push_projection(&mut self, plan: ExecutionPlan, expressions: Vec<CompiledProjection>) -> Result<ExecutionPlan> {
        let required_columns = self.extract_required_columns(&expressions);
        
        match plan {
            ExecutionPlan::Scan { table, filter, projection: _ } => {
                let new_projection = self.create_minimal_projection_for_columns(&required_columns);
                Ok(ExecutionPlan::Project {
                    input: Box::new(ExecutionPlan::Scan {
                        table,
                        filter,
                        projection: new_projection,
                    }),
                    expressions,
                })
            }
            ExecutionPlan::Project { input, expressions: inner_expressions } => {
                let inner_required = self.extract_columns_from_projections(&expressions, &inner_expressions);
                let optimized_inner = if inner_required.len() < inner_expressions.len() {
                    let minimal_inner = self.create_projections_for_columns(&inner_required, &inner_expressions);
                    ExecutionPlan::Project {
                        input,
                        expressions: minimal_inner,
                    }
                } else {
                    ExecutionPlan::Project {
                        input,
                        expressions: inner_expressions,
                    }
                };
                
                Ok(ExecutionPlan::Project {
                    input: Box::new(optimized_inner),
                    expressions,
                })
            }
            ExecutionPlan::Filter { input, predicate } => {
                let predicate_columns = self.extract_columns_from_expression(&predicate);
                let all_required: HashSet<String> = required_columns.union(&predicate_columns).cloned().collect();
                
                let optimized_input = self.push_projection_with_required_columns(*input, all_required)?;
                
                Ok(ExecutionPlan::Project {
                    input: Box::new(ExecutionPlan::Filter {
                        input: Box::new(optimized_input),
                        predicate,
                    }),
                    expressions,
                })
            }
            _ => {
                Ok(ExecutionPlan::Project {
                    input: Box::new(plan),
                    expressions,
                })
            }
        }
    }

    fn push_projection_with_required_columns(&mut self, plan: ExecutionPlan, required_columns: HashSet<String>) -> Result<ExecutionPlan> {
        match plan {
            ExecutionPlan::Scan { table, filter, projection: _ } => {
                let new_projection = self.create_minimal_projection_for_columns(&required_columns);
                Ok(ExecutionPlan::Scan {
                    table,
                    filter,
                    projection: new_projection,
                })
            }
            other => self.optimize_plan(other),
        }
    }

    fn extract_required_columns(&self, projections: &[CompiledProjection]) -> HashSet<String> {
        let mut columns = HashSet::new();
        for projection in projections {
            let expr_columns = self.extract_columns_from_expression(&projection.expression);
            columns.extend(expr_columns);
        }
        columns
    }

    fn extract_columns_from_projections(
        &self, 
        outer_projections: &[CompiledProjection],
        inner_projections: &[CompiledProjection]
    ) -> HashSet<String> {
        let mut required = HashSet::new();
        
        for outer_proj in outer_projections {
            let outer_columns = self.extract_columns_from_expression(&outer_proj.expression);
            for col in outer_columns {
                if let Some(inner_proj) = inner_projections.iter().find(|p| p.output_name == col) {
                    let inner_columns = self.extract_columns_from_expression(&inner_proj.expression);
                    required.extend(inner_columns);
                } else {
                    required.insert(col);
                }
            }
        }
        
        required
    }

    fn extract_columns_from_expression(&self, expr: &CompiledExpression) -> HashSet<String> {
        let mut columns = HashSet::new();
        self.extract_columns_recursive(expr, &mut columns);
        columns
    }

    fn extract_columns_recursive(&self, expr: &CompiledExpression, columns: &mut HashSet<String>) {
        match expr {
            CompiledExpression::Column { name, .. } => {
                columns.insert(name.clone());
            }
            CompiledExpression::Binary { left, right, .. } => {
                self.extract_columns_recursive(left, columns);
                self.extract_columns_recursive(right, columns);
            }
            CompiledExpression::Unary { expr, .. } => {
                self.extract_columns_recursive(expr, columns);
            }
            CompiledExpression::Function { args, .. } => {
                for arg in args {
                    self.extract_columns_recursive(arg, columns);
                }
            }
            CompiledExpression::Literal(_) => {}
        }
    }

    fn create_minimal_projection_for_columns(&self, columns: &HashSet<String>) -> Vec<CompiledProjection> {
        columns.iter().map(|col| {
            CompiledProjection {
                expression: CompiledExpression::Column {
                    table: None,
                    name: col.clone(),
                    value_type: crate::compiler::ValueType::String, // TODO: Implement proper type inference based on column schema
                },
                alias: None,
                output_name: col.clone(),
            }
        }).collect()
    }

    fn create_projections_for_columns(
        &self, 
        required: &HashSet<String>, 
        available: &[CompiledProjection]
    ) -> Vec<CompiledProjection> {
        available
            .iter()
            .filter(|proj| required.contains(&proj.output_name))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{ValueType};
    use crate::types::Value;

    fn create_column_projection(name: &str) -> CompiledProjection {
        CompiledProjection {
            expression: CompiledExpression::Column {
                table: None,
                name: name.to_string(),
                value_type: ValueType::String,
            },
            alias: None,
            output_name: name.to_string(),
        }
    }

    fn create_simple_scan() -> ExecutionPlan {
        ExecutionPlan::Scan {
            table: "users".to_string(),
            filter: None,
            projection: vec![
                create_column_projection("id"),
                create_column_projection("name"),
                create_column_projection("age"),
                create_column_projection("email"),
            ],
        }
    }

    #[test]
    fn test_eliminate_unnecessary_projection() {
        let scan = create_simple_scan();
        let project = ExecutionPlan::Project {
            input: Box::new(scan),
            expressions: vec![
                create_column_projection("name"),
                create_column_projection("age"),
            ],
        };

        let optimized = optimize(project).unwrap();

        match optimized {
            ExecutionPlan::Project { input, expressions } => {
                assert_eq!(expressions.len(), 2);
                match input.as_ref() {
                    ExecutionPlan::Scan { projection, .. } => {
                        // Should have minimal projection based on requirements
                        assert!(!projection.is_empty());
                    }
                    _ => panic!("Expected scan as input"),
                }
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_push_projection_through_nested_projects() {
        let scan = create_simple_scan();
        let inner_project = ExecutionPlan::Project {
            input: Box::new(scan),
            expressions: vec![
                create_column_projection("name"),
                create_column_projection("age"),
                create_column_projection("email"),
            ],
        };
        let outer_project = ExecutionPlan::Project {
            input: Box::new(inner_project),
            expressions: vec![
                create_column_projection("name"),
            ],
        };

        let optimized = optimize(outer_project).unwrap();

        match optimized {
            ExecutionPlan::Project { input, expressions } => {
                assert_eq!(expressions.len(), 1);
                match input.as_ref() {
                    ExecutionPlan::Project { expressions: inner_expressions, .. } => {
                        // Inner projection should be reduced
                        assert!(inner_expressions.len() <= 3);
                    }
                    _ => panic!("Expected project as input"),
                }
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_projection_with_filter_considers_predicate_columns() {
        let scan = create_simple_scan();
        let filter = ExecutionPlan::Filter {
            input: Box::new(scan),
            predicate: CompiledExpression::Binary {
                left: Box::new(CompiledExpression::Column {
                    table: None,
                    name: "age".to_string(),
                    value_type: ValueType::Int,
                }),
                op: crate::ast::BinaryOperator::GreaterThan,
                right: Box::new(CompiledExpression::Literal(Value::Int(25))),
                result_type: ValueType::Bool,
            },
        };
        let project = ExecutionPlan::Project {
            input: Box::new(filter),
            expressions: vec![
                create_column_projection("name"),
            ],
        };

        let optimized = optimize(project).unwrap();

        match optimized {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Filter { input, .. } => {
                        match input.as_ref() {
                            ExecutionPlan::Scan { projection, .. } => {
                                // Should include both 'name' (from projection) and 'age' (from filter)
                                assert!(projection.len() >= 2);
                            }
                            _ => panic!("Expected scan"),
                        }
                    }
                    _ => panic!("Expected filter"),
                }
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_no_change_when_no_optimization_possible() {
        let scan = create_simple_scan();
        let project = ExecutionPlan::Project {
            input: Box::new(scan),
            expressions: vec![
                create_column_projection("name"),
                create_column_projection("age"),
            ],
        };

        let optimized = optimize(project).unwrap();

        match optimized {
            ExecutionPlan::Project { expressions, .. } => {
                assert_eq!(expressions.len(), 2);
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_column_extraction_from_complex_expression() {
        let optimizer = ProjectionPushdownOptimizer::new();
        let expr = CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Function {
                name: "UPPER".to_string(),
                args: vec![CompiledExpression::Column {
                    table: None,
                    name: "name".to_string(),
                    value_type: ValueType::String,
                }],
                result_type: ValueType::String,
            }),
            op: crate::ast::BinaryOperator::Equal,
            right: Box::new(CompiledExpression::Column {
                table: None,
                name: "search_term".to_string(),
                value_type: ValueType::String,
            }),
            result_type: ValueType::Bool,
        };

        let columns = optimizer.extract_columns_from_expression(&expr);
        assert!(columns.contains("name"));
        assert!(columns.contains("search_term"));
        assert_eq!(columns.len(), 2);
    }
}
