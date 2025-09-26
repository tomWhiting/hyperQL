use crate::ast::*;
use crate::error::*;

use super::expression::ExpressionCompiler;
use super::{CompiledProjection, CompiledSortKey, CompiledExpression, ExecutionPlan};

pub struct SelectCompiler {
    expression_compiler: ExpressionCompiler,
}

impl SelectCompiler {
    pub fn new() -> Self {
        Self {
            expression_compiler: ExpressionCompiler::new(),
        }
    }

    pub fn compile_select(&self, select: SelectStatement) -> Result<ExecutionPlan> {
        let mut plan = self.create_base_scan(&select)?;

        if let Some(traverse_clause) = select.traverse_clause {
            let compiled_patterns = self.compile_traverse_patterns(&traverse_clause.patterns)?;
            let traverse_plan = ExecutionPlan::Traverse {
                patterns: compiled_patterns,
            };
            if let ExecutionPlan::Scan { .. } = plan {
                plan = traverse_plan;
            } else {
                plan = traverse_plan;
            }
        }

        if let Some(where_expr) = select.where_clause {
            let compiled_predicate = self.expression_compiler.compile_expression(where_expr)?;
            plan = ExecutionPlan::Filter {
                input: Box::new(plan),
                predicate: compiled_predicate,
            };
        }

        if !select.group_by.is_empty() {
            let group_expressions = select.group_by.iter()
                .map(|expr| self.expression_compiler.compile_expression(expr.clone()))
                .collect::<Result<Vec<_>>>()?;

            let projections = self.compile_select_list(&select.select_list)?;

            plan = ExecutionPlan::GroupBy {
                input: Box::new(plan),
                group_expressions,
                aggregate_expressions: projections,
            };
        } else {
            let projections = self.compile_select_list(&select.select_list)?;
            if !projections.is_empty() {
                plan = ExecutionPlan::Project {
                    input: Box::new(plan),
                    expressions: projections,
                };
            }
        }

        if let Some(having_expr) = select.having {
            let compiled_having = self.expression_compiler.compile_expression(having_expr)?;
            plan = ExecutionPlan::Having {
                input: Box::new(plan),
                predicate: compiled_having,
            };
        }

        if !select.order_by.is_empty() {
            let sort_keys = self.compile_order_by(&select.order_by)?;
            plan = ExecutionPlan::Sort {
                input: Box::new(plan),
                sort_keys,
            };
        }

        if let Some(limit) = select.limit {
            plan = ExecutionPlan::Limit {
                input: Box::new(plan),
                count: limit,
                offset: select.offset,
            };
        }

        Ok(plan)
    }

    fn create_base_scan(&self, select: &SelectStatement) -> Result<ExecutionPlan> {
        let table_name = match &select.from {
            Some(FromClause::Table { name, .. }) => name.clone(),
            Some(FromClause::Subquery { .. }) => {
                return Err(HyperQLError::SemanticError {
                    message: "Subqueries are not yet supported".to_string(),
                    context: vec!["compiler".to_string()],
                });
            }
            None => {
                return Err(HyperQLError::SemanticError {
                    message: "FROM clause is required".to_string(),
                    context: vec!["compiler".to_string()],
                });
            }
        };

        Ok(ExecutionPlan::Scan {
            table: table_name,
            filter: None,
            projection: vec![],
        })
    }

    fn compile_select_list(&self, select_list: &[SelectItem]) -> Result<Vec<CompiledProjection>> {
        let mut projections = Vec::new();

        for item in select_list {
            match item {
                SelectItem::Wildcard => {
                    projections.push(CompiledProjection {
                        expression: CompiledExpression::Literal(crate::types::Value::String("*".to_string())),
                        alias: None,
                        output_name: "*".to_string(),
                    });
                }
                SelectItem::Expression { expr, alias } => {
                    let compiled_expr = self.expression_compiler.compile_expression(expr.clone())?;
                    let output_name = alias.clone().unwrap_or_else(|| {
                        self.generate_expression_name(expr)
                    });

                    projections.push(CompiledProjection {
                        expression: compiled_expr,
                        alias: alias.clone(),
                        output_name,
                    });
                }
            }
        }

        Ok(projections)
    }

    fn compile_order_by(&self, order_by: &[OrderByItem]) -> Result<Vec<CompiledSortKey>> {
        let mut sort_keys = Vec::new();

        for item in order_by {
            let compiled_expr = self.expression_compiler.compile_expression(item.expr.clone())?;
            sort_keys.push(CompiledSortKey {
                expression: compiled_expr,
                direction: item.direction.clone(),
            });
        }

        Ok(sort_keys)
    }

    fn generate_expression_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Column(col_ref) => col_ref.name.clone(),
            Expression::Function { name, .. } => name.clone(),
            Expression::Literal(literal) => format!("{:?}", literal),
            _ => "expr".to_string(),
        }
    }

    fn compile_traverse_patterns(&self, patterns: &[crate::ast::TraversePattern]) -> Result<Vec<super::CompiledTraversePattern>> {
        let mut compiled_patterns = Vec::new();

        for pattern in patterns {
            let compiled_start_node = super::CompiledNodePattern {
                variable: pattern.start_node.variable.clone(),
                label: pattern.start_node.label.clone(),
                properties: if let Some(props) = &pattern.start_node.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_end_node = super::CompiledNodePattern {
                variable: pattern.end_node.variable.clone(),
                label: pattern.end_node.label.clone(),
                properties: if let Some(props) = &pattern.end_node.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_relationship = super::CompiledRelationshipPattern {
                variable: pattern.relationship.variable.clone(),
                rel_type: pattern.relationship.rel_type.clone(),
                direction: pattern.relationship.direction.clone(),
                variable_length: pattern.relationship.variable_length.clone(),
                optional: pattern.relationship.optional,
                properties: if let Some(props) = &pattern.relationship.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            compiled_patterns.push(super::CompiledTraversePattern {
                start_node: compiled_start_node,
                relationship: compiled_relationship,
                end_node: compiled_end_node,
            });
        }

        Ok(compiled_patterns)
    }
}

impl Default for SelectCompiler {
    fn default() -> Self {
        Self::new()
    }
}
