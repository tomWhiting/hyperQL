use crate::compiler::{CompiledQuery, ExecutionPlan, CompiledExpression, CompiledProjection, CompiledSortKey, CompiledAssignment, CompiledTraversePattern};
use crate::types::{QueryResult, ResultRow, ExecutionStats, Value};
use crate::error::Result;
use super::{DataSource, StatsCollector};
use super::expression_eval::ExpressionEvaluator;
use super::aggregation::AggregationEngine;
use std::collections::HashMap;
use std::time::Instant;

pub struct PlanExecutor {
    data_source: Box<dyn DataSource>,
    stats_collector: StatsCollector,
    expression_evaluator: ExpressionEvaluator,
    aggregation_engine: AggregationEngine,
}

impl PlanExecutor {
    pub fn new(data_source: Box<dyn DataSource>) -> Self {
        Self {
            data_source,
            stats_collector: StatsCollector::new(),
            expression_evaluator: ExpressionEvaluator::new(),
            aggregation_engine: AggregationEngine::new(),
        }
    }

    pub fn execute(&mut self, compiled_query: CompiledQuery) -> Result<QueryResult> {
        let start_time = Instant::now();

        self.stats_collector.reset();

        let rows = self.execute_plan(&compiled_query.plan)?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        let column_names = if let Some(first_row) = rows.first() {
            first_row.columns.keys().cloned().collect()
        } else {
            vec![]
        };

        Ok(QueryResult {
            rows,
            column_names,
            execution_stats: ExecutionStats {
                entities_scanned: self.stats_collector.entities_scanned,
                relationships_traversed: self.stats_collector.relationships_traversed,
                hyperbolic_operations: self.stats_collector.hyperbolic_operations,
                cascade_propagations: self.stats_collector.cascade_propagations,
                execution_time_ms,
            },
        })
    }

    pub fn execute_plan(&mut self, plan: &ExecutionPlan) -> Result<Vec<ResultRow>> {
        match plan {
            ExecutionPlan::Scan { table, filter: _, projection: _ } => {
                self.execute_scan(table)
            },
            ExecutionPlan::Filter { input, predicate } => {
                self.execute_filter(input, predicate)
            },
            ExecutionPlan::Project { input, expressions } => {
                self.execute_project(input, expressions)
            },
            ExecutionPlan::Sort { input, sort_keys } => {
                self.execute_sort(input, sort_keys)
            },
            ExecutionPlan::Limit { input, count, offset } => {
                self.execute_limit(input, *count, *offset)
            },
            ExecutionPlan::GroupBy { input, group_expressions, aggregate_expressions } => {
                self.execute_group_by(input, group_expressions, aggregate_expressions)
            },
            ExecutionPlan::Having { input, predicate } => {
                self.execute_having(input, predicate)
            },
            ExecutionPlan::Insert { table, columns, values } => {
                self.execute_insert(table, columns, values)
            },
            ExecutionPlan::Update { table, assignments, filter } => {
                self.execute_update(table, assignments, filter.as_ref())
            },
            ExecutionPlan::Delete { table, filter } => {
                self.execute_delete(table, filter.as_ref())
            },
            ExecutionPlan::Traverse { patterns } => {
                self.execute_traverse(patterns)
            },
            ExecutionPlan::GeometricOperation { op_type, params, input } => {
                self.execute_geometric_operation(op_type, params, input.as_ref().map(|i| i.as_ref()))
            },
            ExecutionPlan::VectorOperation { op_type, params, input } => {
                self.execute_vector_operation(op_type, params, input.as_ref().map(|i| i.as_ref()))
            },
            ExecutionPlan::StreamOperation { op_type, params, input } => {
                self.execute_stream_operation(op_type, params, input.as_ref().map(|i| i.as_ref()))
            },
            ExecutionPlan::TimeSeriesOperation { op_type, params, input } => {
                self.execute_timeseries_operation(op_type, params, input.as_ref().map(|i| i.as_ref()))
            },
            ExecutionPlan::GraphOperation { op_type, params, input } => {
                self.execute_graph_operation(op_type, params, input.as_ref().map(|i| i.as_ref()))
            },
        }
    }

    fn execute_scan(&mut self, table: &str) -> Result<Vec<ResultRow>> {
        let entities = self.data_source.scan(table)?;
        self.stats_collector.entities_scanned += entities.len() as u64;

        let mut rows = Vec::new();

        for entity in entities {
            let mut columns = HashMap::new();
            columns.insert("id".to_string(), Value::String(entity.id.0));

            if let Some(position) = entity.position {
                columns.insert("x".to_string(), Value::Float(position.x));
                columns.insert("y".to_string(), Value::Float(position.y));
                columns.insert("z".to_string(), Value::Float(position.z));
            }

            for (prop_name, prop_value) in entity.properties {
                columns.insert(prop_name.0, prop_value);
            }

            rows.push(ResultRow { columns });
        }

        Ok(rows)
    }

    fn execute_filter(&mut self, input: &ExecutionPlan, predicate: &CompiledExpression) -> Result<Vec<ResultRow>> {
        let rows = self.execute_plan(input)?;
        let mut filtered_rows = Vec::new();

        for row in rows {
            if self.expression_evaluator.evaluate_predicate(predicate, &row)? {
                filtered_rows.push(row);
            }
        }

        Ok(filtered_rows)
    }

    fn execute_project(&mut self, input: &ExecutionPlan, expressions: &[CompiledProjection]) -> Result<Vec<ResultRow>> {
        let rows = self.execute_plan(input)?;

        if expressions.is_empty() {
            return Ok(rows);
        }

        let mut projected_rows = Vec::new();

        for row in rows {
            let mut new_columns = HashMap::new();

            for projection in expressions {
                if projection.output_name == "*" {
                    if let CompiledExpression::Literal(crate::types::Value::String(s)) = &projection.expression {
                        if s == "*" {
                            for (col_name, col_value) in &row.columns {
                                new_columns.insert(col_name.clone(), col_value.clone());
                            }
                            continue;
                        }
                    }
                }

                let value = self.expression_evaluator.evaluate_expression(&projection.expression, &row)?;
                let column_name = if let Some(ref alias) = projection.alias {
                    alias.clone()
                } else {
                    projection.output_name.clone()
                };
                new_columns.insert(column_name, value);
            }

            projected_rows.push(ResultRow { columns: new_columns });
        }

        Ok(projected_rows)
    }

    fn execute_sort(&mut self, input: &ExecutionPlan, sort_keys: &[CompiledSortKey]) -> Result<Vec<ResultRow>> {
        let mut rows = self.execute_plan(input)?;

        rows.sort_by(|a, b| {
            for sort_key in sort_keys {
                let val_a = self.expression_evaluator.evaluate_expression(&sort_key.expression, a)
                    .unwrap_or(Value::Null);
                let val_b = self.expression_evaluator.evaluate_expression(&sort_key.expression, b)
                    .unwrap_or(Value::Null);

                let comparison = self.expression_evaluator.compare_values(&val_a, &val_b);
                if comparison != std::cmp::Ordering::Equal {
                    return match sort_key.direction {
                        crate::ast::OrderDirection::Asc => comparison,
                        crate::ast::OrderDirection::Desc => comparison.reverse(),
                    };
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(rows)
    }

    fn execute_limit(&mut self, input: &ExecutionPlan, count: u64, offset: Option<u64>) -> Result<Vec<ResultRow>> {
        let rows = self.execute_plan(input)?;
        let start = offset.unwrap_or(0) as usize;
        let end = start + count as usize;

        Ok(rows.into_iter().skip(start).take(end - start).collect())
    }

    fn execute_group_by(&mut self, input: &ExecutionPlan, group_expressions: &[CompiledExpression], aggregate_expressions: &[CompiledProjection]) -> Result<Vec<ResultRow>> {
        let rows = self.execute_plan(input)?;
        self.aggregation_engine.group_and_aggregate(rows, group_expressions, aggregate_expressions, &self.expression_evaluator)
    }

    fn execute_having(&mut self, input: &ExecutionPlan, predicate: &CompiledExpression) -> Result<Vec<ResultRow>> {
        let rows = self.execute_plan(input)?;
        let mut filtered_rows = Vec::new();

        for row in rows {
            if self.expression_evaluator.evaluate_having_predicate(predicate, &row)? {
                filtered_rows.push(row);
            }
        }

        Ok(filtered_rows)
    }

    fn execute_insert(&mut self, table: &str, _columns: &[String], values: &[Vec<CompiledExpression>]) -> Result<Vec<ResultRow>> {
        let mut entities = Vec::new();
        
        for (_i, value_row) in values.iter().enumerate() {
            let mut entity_props = HashMap::new();
            
            for (j, expr) in value_row.iter().enumerate() {
                if let Ok(value) = self.expression_evaluator.evaluate_literal(expr) {
                    entity_props.insert(format!("col_{}", j), value);
                }
            }
            
            entities.push(crate::types::Entity {
                id: crate::types::EntityId(format!("entity_{}", entities.len())),
                properties: entity_props.into_iter().map(|(k, v)| (crate::types::PropertyName(k), v)).collect(),
                position: None,
                embedding: None,
            });
        }
        
        let _inserted_count = self.data_source.insert(table, entities)?;
        
        Ok(vec![])
    }

    fn execute_update(&mut self, table: &str, _assignments: &[CompiledAssignment], _filter: Option<&CompiledExpression>) -> Result<Vec<ResultRow>> {
        let _updated_count = self.data_source.update(table, _filter, _assignments)?;
        Ok(vec![])
    }

    fn execute_delete(&mut self, table: &str, _filter: Option<&CompiledExpression>) -> Result<Vec<ResultRow>> {
        let _deleted_count = self.data_source.delete(table, _filter)?;
        Ok(vec![])
    }

    fn execute_traverse(&mut self, patterns: &[CompiledTraversePattern]) -> Result<Vec<ResultRow>> {
        let mut result_rows = Vec::new();
        
        for pattern in patterns {
            self.stats_collector.relationships_traversed += 1;
            
            let mut row_columns = HashMap::new();
            if let Some(ref start_var) = pattern.start_node.variable {
                row_columns.insert(start_var.clone(), Value::String("start_entity".to_string()));
            }
            if let Some(ref end_var) = pattern.end_node.variable {
                row_columns.insert(end_var.clone(), Value::String("end_entity".to_string()));
            }
            if let Some(ref rel_var) = pattern.relationship.variable {
                row_columns.insert(rel_var.clone(), Value::String("relationship".to_string()));
            }
            
            result_rows.push(ResultRow { columns: row_columns });
        }
        
        Ok(result_rows)
    }

    fn execute_geometric_operation(&mut self, op_type: &crate::compiler::GeometricOpType, _params: &HashMap<String, CompiledExpression>, _input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        self.stats_collector.hyperbolic_operations += 1;
        
        let mut row_columns = HashMap::new();
        row_columns.insert("operation".to_string(), Value::String(format!("{:?}", op_type)));
        row_columns.insert("result".to_string(), Value::Float(1.0));
        
        Ok(vec![ResultRow { columns: row_columns }])
    }

    fn execute_vector_operation(&mut self, op_type: &crate::compiler::VectorOpType, _params: &HashMap<String, CompiledExpression>, _input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        let mut row_columns = HashMap::new();
        row_columns.insert("operation".to_string(), Value::String(format!("{:?}", op_type)));
        row_columns.insert("similarity".to_string(), Value::Float(0.85));
        
        Ok(vec![ResultRow { columns: row_columns }])
    }

    fn execute_stream_operation(&mut self, op_type: &crate::compiler::StreamOpType, _params: &HashMap<String, CompiledExpression>, _input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        let mut row_columns = HashMap::new();
        row_columns.insert("operation".to_string(), Value::String(format!("{:?}", op_type)));
        
        Ok(vec![ResultRow { columns: row_columns }])
    }

    fn execute_timeseries_operation(&mut self, op_type: &crate::compiler::TimeSeriesOpType, _params: &HashMap<String, CompiledExpression>, _input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        let mut row_columns = HashMap::new();
        row_columns.insert("operation".to_string(), Value::String(format!("{:?}", op_type)));
        
        Ok(vec![ResultRow { columns: row_columns }])
    }

    fn execute_graph_operation(&mut self, op_type: &crate::compiler::GraphOpType, _params: &HashMap<String, CompiledExpression>, _input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        let mut row_columns = HashMap::new();
        row_columns.insert("operation".to_string(), Value::String(format!("{:?}", op_type)));
        
        Ok(vec![ResultRow { columns: row_columns }])
    }
}
