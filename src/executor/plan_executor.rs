use crate::compiler::{CompiledQuery, ExecutionPlan, CompiledExpression, CompiledProjection, CompiledSortKey, CompiledAssignment, CompiledTraversePattern};
use crate::types::{QueryResult, ResultRow, ExecutionStats, Value};
use crate::error::Result;
use super::{DataSource, StatsCollector};
use super::expression_eval::ExpressionEvaluator;
use super::aggregation::AggregationEngine;
use super::geometric::GeometricEngine;
use super::vector::VectorEngine;
use super::join::JoinExecutor;
use std::collections::HashMap;
use std::time::Instant;

pub struct PlanExecutor {
    data_source: Box<dyn DataSource>,
    stats_collector: StatsCollector,
    expression_evaluator: ExpressionEvaluator,
    aggregation_engine: AggregationEngine,
    geometric_engine: GeometricEngine,
    vector_engine: VectorEngine,
    global_index: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    hnsw: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}

impl PlanExecutor {
    pub fn new(data_source: Box<dyn DataSource>) -> Self {
        Self {
            data_source,
            stats_collector: StatsCollector::new(),
            expression_evaluator: ExpressionEvaluator::new(),
            aggregation_engine: AggregationEngine::new(),
            geometric_engine: GeometricEngine::new(),
            vector_engine: VectorEngine::new(),
            global_index: None,
            hnsw: None,
        }
    }

    pub fn with_indices(
        data_source: Box<dyn DataSource>,
        global_index: std::sync::Arc<dyn std::any::Any + Send + Sync>,
        hnsw: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Self {
        Self {
            data_source,
            stats_collector: StatsCollector::new(),
            expression_evaluator: ExpressionEvaluator::new(),
            aggregation_engine: AggregationEngine::new(),
            geometric_engine: GeometricEngine::new(),
            vector_engine: VectorEngine::new(),
            global_index: Some(global_index),
            hnsw,
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
            ExecutionPlan::Scan { table, entity_type, alias: _, filter, projection, limit } => {
                self.execute_scan(table, entity_type, filter.as_ref(), projection, *limit)
            },
            ExecutionPlan::Filter { input, predicate } => {
                self.execute_filter(input, predicate)
            },
            ExecutionPlan::Project { input, expressions, distinct } => {
                self.execute_project(input, expressions, *distinct)
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
            ExecutionPlan::Join { left, right, join_type, on_condition } => {
                self.execute_join(left, right, join_type, on_condition)
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
            ExecutionPlan::Schema { operation } => {
                self.execute_schema(operation)
            },
        }
    }

    fn execute_scan(
        &mut self,
        table: &str,
        entity_type: &str,
        filter: Option<&CompiledExpression>,
        projection: &[CompiledProjection],
        limit: Option<u64>
    ) -> Result<Vec<ResultRow>> {
        // CRITICAL OPTIMIZATION: Use scan_with_limit when limit is present
        // This enables early termination at the DataSource level for 60-110x speedup
        let entities = if let Some(limit_count) = limit {
            self.data_source.scan_with_limit(table, entity_type, limit_count as usize)?
        } else {
            self.data_source.scan(table, entity_type)?
        };
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

            let row = ResultRow { columns };

            // Apply filter if present
            if let Some(filter_expr) = filter {
                if !self.expression_evaluator.evaluate_predicate(filter_expr, &row)? {
                    continue; // Skip rows that don't match the filter
                }
            }

            // Apply projection if present
            if !projection.is_empty() {
                let mut projected_columns = HashMap::new();
                for proj in projection {
                    let value = self.expression_evaluator.evaluate_expression(&proj.expression, &row)?;
                    let column_name = if let Some(ref alias) = proj.alias {
                        alias.clone()
                    } else {
                        proj.output_name.clone()
                    };
                    projected_columns.insert(column_name, value);
                }
                rows.push(ResultRow { columns: projected_columns });
            } else {
                rows.push(row);
            }
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

    fn execute_project(&mut self, input: &ExecutionPlan, expressions: &[CompiledProjection], distinct: bool) -> Result<Vec<ResultRow>> {
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

        if distinct {
            Ok(self.deduplicate_rows(projected_rows))
        } else {
            Ok(projected_rows)
        }
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
        // CRITICAL OPTIMIZATION: Fast path for COUNT(*) queries without GROUP BY
        // Detect pattern: SELECT COUNT(*) FROM table (no WHERE, no GROUP BY)
        let is_count_star = group_expressions.is_empty()
            && aggregate_expressions.len() == 1
            && self.is_count_star_only(aggregate_expressions)
            && self.is_simple_scan(input);

        if is_count_star {
            // Fast path: Use RouterDataSource.count_entities() instead of scan()
            if let ExecutionPlan::Scan { table, entity_type, alias: _, filter, projection, limit: _ } = input {
                // LIMIT is allowed in COUNT queries - it applies to result rows (always 1), not to the count
                if filter.is_none() && projection.is_empty() {
                    // Simple COUNT(*) FROM table - use fast count
                    let count = self.data_source.count_entities_fast(table, entity_type)?;

                    let mut columns = HashMap::new();
                    let column_name = if let Some(ref alias) = aggregate_expressions[0].alias {
                        alias.clone()
                    } else {
                        aggregate_expressions[0].output_name.clone()
                    };

                    columns.insert(column_name.clone(), Value::Int(count as i64));

                    self.stats_collector.entities_scanned = count as u64;

                    return Ok(vec![ResultRow { columns }]);
                }
            }
        }

        // Standard path: Execute input plan and aggregate
        let rows = self.execute_plan(input)?;
        self.aggregation_engine.group_and_aggregate(rows, group_expressions, aggregate_expressions, &self.expression_evaluator)
    }

    /// Check if aggregation is just COUNT(*)
    fn is_count_star_only(&self, aggregate_expressions: &[CompiledProjection]) -> bool {
        if aggregate_expressions.len() != 1 {
            return false;
        }

        match &aggregate_expressions[0].expression {
            CompiledExpression::Function { name, args, .. } => {
                name.to_uppercase() == "COUNT" &&
                (args.is_empty() || (args.len() == 1 && matches!(&args[0], CompiledExpression::Column { name, .. } if name == "*")))
            },
            _ => false
        }
    }

    /// Check if input plan is a simple scan without filters
    /// LIMIT is allowed - it applies to result rows, not to COUNT aggregation
    fn is_simple_scan(&self, plan: &ExecutionPlan) -> bool {
        matches!(plan, ExecutionPlan::Scan { filter, projection, limit: _, table: _, entity_type: _, alias: _ }
            if filter.is_none() && projection.is_empty())
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

    fn execute_join(
        &mut self,
        left: &ExecutionPlan,
        right: &ExecutionPlan,
        join_type: &crate::compiler::JoinType,
        on_condition: &CompiledExpression,
    ) -> Result<Vec<ResultRow>> {
        // Execute left and right plans
        let left_rows = self.execute_plan(left)?;
        let right_rows = self.execute_plan(right)?;

        // Extract aliases from execution plans
        let left_alias = Self::extract_alias_from_plan(left);
        let right_alias = Self::extract_alias_from_plan(right);

        // Perform JOIN using JoinExecutor with aliases
        JoinExecutor::execute_join(
            left_rows,
            right_rows,
            join_type,
            on_condition,
            &self.expression_evaluator,
            &left_alias,
            &right_alias,
        )
    }

    /// Extract alias from an execution plan (walks down to the Scan node)
    fn extract_alias_from_plan(plan: &ExecutionPlan) -> Option<String> {
        match plan {
            ExecutionPlan::Scan { alias, table, .. } => {
                // Use alias if present, otherwise use table name
                alias.clone().or_else(|| Some(table.clone()))
            }
            ExecutionPlan::Filter { input, .. } => Self::extract_alias_from_plan(input),
            ExecutionPlan::Project { input, .. } => Self::extract_alias_from_plan(input),
            ExecutionPlan::Sort { input, .. } => Self::extract_alias_from_plan(input),
            ExecutionPlan::Limit { input, .. } => Self::extract_alias_from_plan(input),
            ExecutionPlan::GroupBy { input, .. } => Self::extract_alias_from_plan(input),
            ExecutionPlan::Having { input, .. } => Self::extract_alias_from_plan(input),
            ExecutionPlan::Join { left, .. } => Self::extract_alias_from_plan(left),
            _ => None,
        }
    }

    fn execute_traverse(&mut self, patterns: &[CompiledTraversePattern]) -> Result<Vec<ResultRow>> {
        if patterns.is_empty() {
            return Ok(vec![]);
        }

        let mut result_rows = Vec::new();

        for pattern in patterns {
            let start_label = pattern.start_node.label.as_deref();
            let rel_type = pattern.relationship.rel_type.as_deref();

            // For TRAVERSE, we need to support old single-label style temporarily
            // TODO: Update TRAVERSE syntax to use collection.type format
            let start_entities = if let Some(label) = start_label {
                // Assume label is actually "collection.type" or just treat as type
                let parts: Vec<&str> = label.split('.').collect();
                if parts.len() == 2 {
                    self.data_source.scan(parts[0], parts[1])?
                } else {
                    // Legacy: treat label as both collection and type
                    self.data_source.scan(label, label)?
                }
            } else {
                vec![]
            };

            for start_entity in &start_entities {
                if let Some(ref start_constraint) = pattern.start_node.properties {
                    if !self.expression_evaluator.evaluate_predicate(start_constraint, &self.entity_to_row(start_entity))? {
                        continue;
                    }
                }

                let (min_hops, max_hops) = if let Some(ref var_length) = pattern.relationship.variable_length {
                    let min = var_length.min_hops.unwrap_or(1) as usize;
                    let max = var_length.max_hops.unwrap_or(min as u32) as usize;
                    (min, max)
                } else {
                    (1, 1)
                };

                let traversal_results = self.data_source.traverse_graph(&start_entity.id, max_hops, rel_type)?;

                let mut matched_end_entities = Vec::new();

                for (end_entity, depth) in traversal_results {
                    if depth < min_hops || depth > max_hops {
                        continue;
                    }

                    if depth == 0 {
                        continue;
                    }

                    if let Some(ref end_constraint) = pattern.end_node.properties {
                        if !self.expression_evaluator.evaluate_predicate(end_constraint, &self.entity_to_row(&end_entity))? {
                            continue;
                        }
                    }

                    matched_end_entities.push((end_entity, depth));
                }

                if matched_end_entities.is_empty() && pattern.relationship.optional {
                    let mut row_columns = HashMap::new();

                    if let Some(ref start_var) = pattern.start_node.variable {
                        row_columns.insert(start_var.clone(), Value::EntityId(start_entity.id.clone()));
                        row_columns.insert(format!("{}_id", start_var), Value::String(start_entity.id.0.clone()));
                    }

                    if let Some(ref end_var) = pattern.end_node.variable {
                        row_columns.insert(end_var.clone(), Value::Null);
                        row_columns.insert(format!("{}_id", end_var), Value::Null);
                    }

                    if let Some(ref rel_var) = pattern.relationship.variable {
                        row_columns.insert(rel_var.clone(), Value::Null);
                    }

                    result_rows.push(ResultRow { columns: row_columns });
                } else {
                    for (end_entity, depth) in matched_end_entities {
                        self.stats_collector.relationships_traversed += depth as u64;

                        let mut row_columns = HashMap::new();

                        if let Some(ref start_var) = pattern.start_node.variable {
                            row_columns.insert(start_var.clone(), Value::EntityId(start_entity.id.clone()));
                            row_columns.insert(format!("{}_id", start_var), Value::String(start_entity.id.0.clone()));
                        }

                        if let Some(ref end_var) = pattern.end_node.variable {
                            row_columns.insert(end_var.clone(), Value::EntityId(end_entity.id.clone()));
                            row_columns.insert(format!("{}_id", end_var), Value::String(end_entity.id.0.clone()));
                        }

                        if let Some(ref rel_var) = pattern.relationship.variable {
                            row_columns.insert(rel_var.clone(), Value::String(format!("path_depth_{}", depth)));
                        }

                        result_rows.push(ResultRow { columns: row_columns });
                    }
                }
            }
        }

        Ok(result_rows)
    }

    fn entity_to_row(&self, entity: &crate::types::Entity) -> ResultRow {
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), Value::String(entity.id.0.clone()));

        if let Some(ref position) = entity.position {
            columns.insert("x".to_string(), Value::Float(position.x));
            columns.insert("y".to_string(), Value::Float(position.y));
            columns.insert("z".to_string(), Value::Float(position.z));
        }

        for (prop_name, prop_value) in &entity.properties {
            columns.insert(prop_name.0.clone(), prop_value.clone());
        }

        ResultRow { columns }
    }

    fn execute_geometric_operation(&mut self, op_type: &crate::compiler::GeometricOpType, params: &HashMap<String, CompiledExpression>, input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        self.stats_collector.hyperbolic_operations += 1;

        let input_rows = if let Some(plan) = input {
            self.execute_plan(plan)?
        } else {
            vec![]
        };

        self.geometric_engine.execute_operation(
            op_type,
            params,
            input_rows,
            &mut self.expression_evaluator,
            self.global_index.as_ref(),
            self.hnsw.as_ref(),
        )
    }

    fn execute_vector_operation(&mut self, op_type: &crate::compiler::VectorOpType, params: &HashMap<String, CompiledExpression>, input: Option<&ExecutionPlan>) -> Result<Vec<ResultRow>> {
        let input_rows = if let Some(plan) = input {
            self.execute_plan(plan)?
        } else {
            vec![]
        };

        self.vector_engine.execute_operation(
            op_type,
            params,
            input_rows,
            &mut self.expression_evaluator,
        )
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
        row_columns.insert("status".to_string(), Value::String("Graph engine integration pending".to_string()));
        row_columns.insert("note".to_string(), Value::String("Full graph operations require Hyperspatial graph engine integration".to_string()));

        Ok(vec![ResultRow { columns: row_columns }])
    }

    fn deduplicate_rows(&self, rows: Vec<ResultRow>) -> Vec<ResultRow> {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        let mut unique_rows = Vec::new();

        for row in rows {
            let hash_key = self.compute_row_hash(&row);

            if seen.insert(hash_key) {
                unique_rows.push(row);
            }
        }

        unique_rows
    }

    fn compute_row_hash(&self, row: &ResultRow) -> Vec<HashableValue> {
        let mut column_names: Vec<_> = row.columns.keys().collect();
        column_names.sort();

        column_names.iter()
            .map(|col_name| {
                let value = row.columns.get(*col_name).unwrap();
                self.value_to_hashable(value)
            })
            .collect()
    }

    fn value_to_hashable(&self, value: &Value) -> HashableValue {
        match value {
            Value::Null => HashableValue::Null,
            Value::Bool(b) => HashableValue::Bool(*b),
            Value::Int(i) => HashableValue::Int(*i),
            Value::Float(f) => {
                if f.is_nan() {
                    HashableValue::Float(u64::MAX)
                } else {
                    HashableValue::Float(f.to_bits())
                }
            },
            Value::String(s) => HashableValue::String(s.clone()),
            Value::EntityId(id) => HashableValue::EntityId(id.0.clone()),
            Value::Position(pos) => HashableValue::Position(
                pos.x.to_bits(),
                pos.y.to_bits(),
                pos.z.to_bits()
            ),
            Value::Distance(dist) => HashableValue::Distance(dist.0.to_bits()),
            Value::Vector(vec) => HashableValue::Vector(
                vec.dimensions.iter().map(|d| d.to_bits()).collect()
            ),
            Value::List(list) => HashableValue::List(
                list.iter().map(|v| self.value_to_hashable(v)).collect()
            ),
            Value::Map(map) => {
                let mut sorted_entries: Vec<_> = map.iter().collect();
                sorted_entries.sort_by_key(|(k, _)| *k);
                HashableValue::Map(
                    sorted_entries.into_iter()
                        .map(|(k, v)| (k.clone(), self.value_to_hashable(v)))
                        .collect()
                )
            },
            Value::Timestamp(ts) => HashableValue::Timestamp(*ts),
            Value::Duration(dur) => HashableValue::Duration(*dur),
        }
    }

    fn execute_schema(&mut self, operation: &crate::ast::schema::SchemaOperation) -> Result<Vec<ResultRow>> {
        use crate::ast::schema::SchemaOperation;

        match operation {
            SchemaOperation::Create(create_op) => {
                self.data_source.create_schema(create_op)?;

                let mut columns = HashMap::new();
                columns.insert("status".to_string(), Value::String("Schema created successfully".to_string()));
                columns.insert("collection".to_string(), Value::String(create_op.collection_name.clone()));
                columns.insert("fields".to_string(), Value::Int(create_op.fields.len() as i64));

                Ok(vec![ResultRow { columns }])
            },
            SchemaOperation::Drop(drop_op) => {
                self.data_source.drop_schema(drop_op)?;

                let mut columns = HashMap::new();
                columns.insert("status".to_string(), Value::String("Schema dropped successfully".to_string()));
                columns.insert("collection".to_string(), Value::String(drop_op.collection_name.clone()));

                Ok(vec![ResultRow { columns }])
            },
            SchemaOperation::Alter(alter_op) => {
                self.data_source.alter_schema(alter_op)?;

                let mut columns = HashMap::new();
                columns.insert("status".to_string(), Value::String("Schema altered successfully".to_string()));
                columns.insert("collection".to_string(), Value::String(alter_op.collection_name.clone()));

                Ok(vec![ResultRow { columns }])
            },
            SchemaOperation::Describe(describe_op) => {
                let schema_info = self.data_source.describe_schema(describe_op)?;

                let mut columns = HashMap::new();
                columns.insert("collection".to_string(), Value::String(describe_op.collection_name.clone()));
                columns.insert("schema".to_string(), Value::String(schema_info));

                Ok(vec![ResultRow { columns }])
            },
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
enum HashableValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(u64),
    String(String),
    EntityId(String),
    Position(u64, u64, u64),
    Distance(u64),
    Vector(Vec<u64>),
    List(Vec<HashableValue>),
    Map(Vec<(String, HashableValue)>),
    Timestamp(i64),
    Duration(i64),
}
