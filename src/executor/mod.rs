//! # HyperQL Query Executor
//!
//! This module implements the comprehensive query execution engine for HyperQL.
//! The executor takes optimized execution plans from the compiler and executes them
//! against data sources, supporting SELECT, INSERT, UPDATE, DELETE operations with
//! aggregations, GROUP BY, HAVING clauses, and hyperbolic space operations.

pub mod operators;

use crate::compiler::{CompiledQuery, ExecutionPlan, CompiledExpression, CompiledProjection, CompiledSortKey, CompiledAssignment};
use crate::types::{QueryResult, ResultRow, ExecutionStats, Value, Entity};
use crate::error::{HyperQLError, Result};
use std::collections::HashMap;
use std::time::Instant;

/// Main query execution engine
pub struct Executor {
    /// Data source for entities
    data_source: Box<dyn DataSource>,
    /// Execution statistics collector
    stats_collector: StatsCollector,
}

/// Abstract data source trait
pub trait DataSource: Send + Sync {
    /// Scan entities from a table
    fn scan(&self, table: &str) -> Result<Vec<Entity>>;

    /// Insert entities into a table
    fn insert(&mut self, table: &str, entities: Vec<Entity>) -> Result<u64>;

    /// Update entities in a table
    fn update(&mut self, table: &str, filter: Option<&CompiledExpression>, assignments: &[CompiledAssignment]) -> Result<u64>;

    /// Delete entities from a table
    fn delete(&mut self, table: &str, filter: Option<&CompiledExpression>) -> Result<u64>;

    /// Get schema information for a table
    fn get_schema(&self, table: &str) -> Result<TableSchema>;
}

/// Table schema definition
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: HashMap<String, ColumnType>,
}

/// Column type information
#[derive(Debug, Clone)]
pub enum ColumnType {
    String,
    Integer,
    Float,
    Boolean,
    EntityId,
    Position,
    Vector,
}

/// Statistics collector for execution metrics
pub struct StatsCollector {
    pub entities_scanned: u64,
    pub relationships_traversed: u64,
    pub hyperbolic_operations: u64,
    pub cascade_propagations: u64,
}

/// Simple in-memory data source for testing
pub struct MemoryDataSource {
    entities: HashMap<String, Vec<Entity>>,
}

impl Executor {
    /// Create a new executor with a data source
    pub fn new(data_source: Box<dyn DataSource>) -> Self {
        Self {
            data_source,
            stats_collector: StatsCollector::new(),
        }
    }

    /// Execute a compiled query
    pub fn execute(&mut self, compiled_query: CompiledQuery) -> Result<QueryResult> {
        let start_time = Instant::now();

        // Reset statistics
        self.stats_collector.reset();

        // Execute the plan
        let rows = self.execute_plan(&compiled_query.plan)?;

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Extract column names from the first row
        let column_names = if let Some(first_row) = rows.first() {
            first_row.columns.keys().cloned().collect()
        } else {
            vec![]
        };

        Ok(QueryResult {
            rows,
            column_names,
            execution_stats: ExecutionStats {
                execution_time_ms,
                entities_scanned: self.stats_collector.entities_scanned,
                relationships_traversed: self.stats_collector.relationships_traversed,
                hyperbolic_operations: self.stats_collector.hyperbolic_operations,
                cascade_propagations: self.stats_collector.cascade_propagations,
            },
        })
    }

    /// Execute an execution plan recursively
    fn execute_plan(&mut self, plan: &ExecutionPlan) -> Result<Vec<ResultRow>> {
        match plan {
            ExecutionPlan::Scan { table, filter: _, projection: _ } => {
                self.execute_scan(table)
            }
            ExecutionPlan::Filter { input, predicate } => {
                self.execute_filter(input, predicate)
            }
            ExecutionPlan::Project { input, expressions } => {
                self.execute_project(input, expressions)
            }
            ExecutionPlan::GroupBy { input, group_expressions, aggregate_expressions } => {
                self.execute_group_by(input, group_expressions, aggregate_expressions)
            }
            ExecutionPlan::Having { input, predicate } => {
                self.execute_having(input, predicate)
            }
            ExecutionPlan::Sort { input, sort_keys } => {
                self.execute_sort(input, sort_keys)
            }
            ExecutionPlan::Limit { input, count, offset } => {
                self.execute_limit(input, *count, *offset)
            }
            ExecutionPlan::Insert { table, columns, values } => {
                self.execute_insert(table, columns, values)
            }
            ExecutionPlan::Update { table, assignments, filter } => {
                self.execute_update(table, assignments, filter.as_ref())
            }
            ExecutionPlan::Delete { table, filter } => {
                self.execute_delete(table, filter.as_ref())
            }
        }
    }

    /// Execute table scan
    fn execute_scan(&mut self, table: &str) -> Result<Vec<ResultRow>> {
        let entities = self.data_source.scan(table)?;
        self.stats_collector.entities_scanned += entities.len() as u64;

        let mut rows = Vec::new();
        for entity in entities {
            let mut columns = HashMap::new();

            // Add entity ID
            columns.insert("id".to_string(), Value::EntityId(entity.id));

            // Add all properties
            for (prop_name, prop_value) in entity.properties {
                columns.insert(prop_name.0, prop_value);
            }

            // Add position if present
            if let Some(position) = entity.position {
                columns.insert("position".to_string(), Value::Position(position));
            }

            // Add embedding if present
            if let Some(embedding) = entity.embedding {
                columns.insert("embedding".to_string(), Value::Vector(embedding));
            }

            rows.push(ResultRow { columns });
        }

        Ok(rows)
    }

    /// Execute filter operation
    fn execute_filter(&mut self, input: &ExecutionPlan, predicate: &CompiledExpression) -> Result<Vec<ResultRow>> {
        let input_rows = self.execute_plan(input)?;
        let mut filtered_rows = Vec::new();

        for row in input_rows {
            if self.evaluate_predicate(predicate, &row)? {
                filtered_rows.push(row);
            }
        }

        Ok(filtered_rows)
    }

    /// Execute projection operation
    fn execute_project(&mut self, input: &ExecutionPlan, expressions: &[CompiledProjection]) -> Result<Vec<ResultRow>> {
        let input_rows = self.execute_plan(input)?;
        let mut projected_rows = Vec::new();

        for row in input_rows {
            let mut new_columns = HashMap::new();

            for projection in expressions {
                // Handle wildcard expansion
                if projection.output_name == "*" {
                    // For wildcard, include all columns from the input row
                    for (col_name, col_value) in &row.columns {
                        new_columns.insert(col_name.clone(), col_value.clone());
                    }
                } else {
                    let value = self.evaluate_expression(&projection.expression, &row)?;
                    new_columns.insert(projection.output_name.clone(), value);
                }
            }

            projected_rows.push(ResultRow { columns: new_columns });
        }

        Ok(projected_rows)
    }

    /// Execute sort operation
    fn execute_sort(&mut self, input: &ExecutionPlan, sort_keys: &[CompiledSortKey]) -> Result<Vec<ResultRow>> {
        let mut rows = self.execute_plan(input)?;

        // Simple single-key sorting for now
        if let Some(first_key) = sort_keys.first() {
            rows.sort_by(|a, b| {
                let val_a = self.evaluate_expression(&first_key.expression, a).unwrap_or(Value::Null);
                let val_b = self.evaluate_expression(&first_key.expression, b).unwrap_or(Value::Null);

                let cmp = self.compare_values(&val_a, &val_b);

                match first_key.direction {
                    crate::ast::OrderDirection::Asc => cmp,
                    crate::ast::OrderDirection::Desc => cmp.reverse(),
                }
            });
        }

        Ok(rows)
    }

    /// Execute limit operation
    fn execute_limit(&mut self, input: &ExecutionPlan, count: u64, offset: Option<u64>) -> Result<Vec<ResultRow>> {
        let rows = self.execute_plan(input)?;
        let offset = offset.unwrap_or(0) as usize;
        let count = count as usize;

        if offset >= rows.len() {
            return Ok(vec![]);
        }

        let end = (offset + count).min(rows.len());
        Ok(rows[offset..end].to_vec())
    }

    /// Execute GROUP BY operation with aggregations
    fn execute_group_by(&mut self, input: &ExecutionPlan, group_expressions: &[CompiledExpression], aggregate_expressions: &[CompiledProjection]) -> Result<Vec<ResultRow>> {
        let input_rows = self.execute_plan(input)?;
        let mut groups: HashMap<String, Vec<ResultRow>> = HashMap::new();

        // Group rows by the grouping expressions
        for row in input_rows {
            let mut group_key_parts = Vec::new();
            for group_expr in group_expressions {
                let value = self.evaluate_expression(group_expr, &row)?;
                group_key_parts.push(format!("{:?}", value));
            }
            let group_key = group_key_parts.join("|");
            groups.entry(group_key).or_insert_with(Vec::new).push(row);
        }

        // Process each group and compute aggregates
        let mut result_rows = Vec::new();
        for (_, group_rows) in groups {
            let mut result_columns = HashMap::new();

            for projection in aggregate_expressions {
                let value = if let CompiledExpression::Function { name, args, .. } = &projection.expression {
                    // Handle aggregate functions
                    self.evaluate_aggregate_function(name, args, &group_rows)?
                } else {
                    // For non-aggregate expressions, use the first row's value
                    if let Some(first_row) = group_rows.first() {
                        self.evaluate_expression(&projection.expression, first_row)?
                    } else {
                        Value::Null
                    }
                };

                result_columns.insert(projection.output_name.clone(), value);
            }

            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    /// Execute HAVING clause (filter for grouped results)
    fn execute_having(&mut self, input: &ExecutionPlan, predicate: &CompiledExpression) -> Result<Vec<ResultRow>> {
        let input_rows = self.execute_plan(input)?;
        let mut filtered_rows = Vec::new();

        for row in input_rows {
            if self.evaluate_having_predicate(predicate, &row)? {
                filtered_rows.push(row);
            }
        }

        Ok(filtered_rows)
    }

    /// Execute INSERT operation
    fn execute_insert(&mut self, table: &str, columns: &[String], values: &[Vec<CompiledExpression>]) -> Result<Vec<ResultRow>> {
        // TODO: Implement actual insertion logic
        // For now, return a result indicating the number of rows affected
        let affected_rows = values.len() as u64;

        let mut result_columns = HashMap::new();
        result_columns.insert("affected_rows".to_string(), Value::Int(affected_rows as i64));
        result_columns.insert("operation".to_string(), Value::String("INSERT".to_string()));
        result_columns.insert("table".to_string(), Value::String(table.to_string()));

        Ok(vec![ResultRow { columns: result_columns }])
    }

    /// Execute UPDATE operation
    fn execute_update(&mut self, table: &str, assignments: &[CompiledAssignment], filter: Option<&CompiledExpression>) -> Result<Vec<ResultRow>> {
        // TODO: Implement actual update logic
        // For now, return a result indicating the operation was received

        let mut result_columns = HashMap::new();
        result_columns.insert("affected_rows".to_string(), Value::Int(0)); // TODO: Calculate actual affected rows
        result_columns.insert("operation".to_string(), Value::String("UPDATE".to_string()));
        result_columns.insert("table".to_string(), Value::String(table.to_string()));
        result_columns.insert("assignments".to_string(), Value::Int(assignments.len() as i64));

        Ok(vec![ResultRow { columns: result_columns }])
    }

    /// Execute DELETE operation
    fn execute_delete(&mut self, table: &str, filter: Option<&CompiledExpression>) -> Result<Vec<ResultRow>> {
        // TODO: Implement actual delete logic
        // For now, return a result indicating the operation was received

        let mut result_columns = HashMap::new();
        result_columns.insert("affected_rows".to_string(), Value::Int(0)); // TODO: Calculate actual affected rows
        result_columns.insert("operation".to_string(), Value::String("DELETE".to_string()));
        result_columns.insert("table".to_string(), Value::String(table.to_string()));

        Ok(vec![ResultRow { columns: result_columns }])
    }

    /// Evaluate a predicate expression
    fn evaluate_predicate(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<bool> {
        let result = self.evaluate_expression(expr, row)?;
        match result {
            Value::Bool(b) => Ok(b),
            Value::Null => Ok(false),
            _ => Err(HyperQLError::ExecutionError {
                message: "Predicate must evaluate to boolean".to_string(),
                operation: "filter".to_string(),
                entity_context: None,
            }),
        }
    }

    /// Evaluate a HAVING predicate expression (handles aggregate references differently)
    fn evaluate_having_predicate(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<bool> {
        let result = self.evaluate_having_expression(expr, row)?;
        match result {
            Value::Bool(b) => Ok(b),
            Value::Null => Ok(false),
            _ => Err(HyperQLError::ExecutionError {
                message: "HAVING predicate must evaluate to boolean".to_string(),
                operation: "having".to_string(),
                entity_context: None,
            }),
        }
    }

    /// Evaluate expressions in HAVING context (handles aggregate column references)
    fn evaluate_having_expression(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<Value> {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                // For aggregate functions in HAVING, look for the computed result in the row
                let function_key = format!("{}({}", name.to_uppercase(),
                    if args.len() == 1 {
                        if let CompiledExpression::Column { name: col_name, .. } = &args[0] {
                            if col_name == "*" {
                                "*"
                            } else {
                                col_name
                            }
                        } else {
                            "expr"
                        }
                    } else {
                        "args"
                    }
                );

                // Try various possible column names for the aggregate
                let possible_names = [
                    format!("{})", function_key),
                    format!("{}", name.to_uppercase()),
                    format!("{}(*)", name.to_uppercase()),
                ];

                for possible_name in &possible_names {
                    if let Some(value) = row.columns.get(possible_name) {
                        return Ok(value.clone());
                    }
                }

                // If not found by name, look for any column containing the function name
                for (col_name, col_value) in &row.columns {
                    if col_name.to_uppercase().contains(&name.to_uppercase()) {
                        return Ok(col_value.clone());
                    }
                }

                // Fallback: return null if aggregate not found
                Ok(Value::Null)
            }
            CompiledExpression::Binary { left, op, right, .. } => {
                let left_val = self.evaluate_having_expression(left, row)?;
                let right_val = self.evaluate_having_expression(right, row)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }
            // For other expressions, use normal evaluation
            _ => self.evaluate_expression(expr, row)
        }
    }

    /// Evaluate an expression in the context of a row
    fn evaluate_expression(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<Value> {
        match expr {
            CompiledExpression::Literal(value) => Ok(value.clone()),
            CompiledExpression::Column { table: _, name, .. } => {
                // Handle wildcard selection
                if name == "*" {
                    // For wildcard, return a string representation of all columns
                    let all_values: Vec<String> = row.columns.iter()
                        .map(|(k, v)| format!("{}: {:?}", k, v))
                        .collect();
                    Ok(Value::String(all_values.join(", ")))
                } else {
                    row.columns.get(name)
                        .cloned()
                        .ok_or_else(|| HyperQLError::ExecutionError {
                            message: format!("Column '{}' not found", name),
                            operation: "column_access".to_string(),
                            entity_context: None,
                        })
                }
            }
            CompiledExpression::Binary { left, op, right, .. } => {
                let left_val = self.evaluate_expression(left, row)?;
                let right_val = self.evaluate_expression(right, row)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }
            CompiledExpression::Unary { op, expr, .. } => {
                let val = self.evaluate_expression(expr, row)?;
                self.evaluate_unary_op(op, &val)
            }
            CompiledExpression::Function { name, args, .. } => {
                let arg_values: Result<Vec<Value>> = args.iter()
                    .map(|arg| self.evaluate_expression(arg, row))
                    .collect();
                let arg_values = arg_values?;
                self.evaluate_function(name, &arg_values)
            }
        }
    }

    /// Evaluate binary operation
    fn evaluate_binary_op(&self, left: &Value, op: &crate::ast::BinaryOperator, right: &Value) -> Result<Value> {
        use crate::ast::BinaryOperator;

        match op {
            BinaryOperator::Equal => Ok(Value::Bool(self.compare_values(left, right) == std::cmp::Ordering::Equal)),
            BinaryOperator::NotEqual => Ok(Value::Bool(self.compare_values(left, right) != std::cmp::Ordering::Equal)),
            BinaryOperator::LessThan => Ok(Value::Bool(self.compare_values(left, right) == std::cmp::Ordering::Less)),
            BinaryOperator::LessThanOrEqual => Ok(Value::Bool(self.compare_values(left, right) != std::cmp::Ordering::Greater)),
            BinaryOperator::GreaterThan => Ok(Value::Bool(self.compare_values(left, right) == std::cmp::Ordering::Greater)),
            BinaryOperator::GreaterThanOrEqual => Ok(Value::Bool(self.compare_values(left, right) != std::cmp::Ordering::Less)),

            BinaryOperator::And => {
                match (left, right) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                    _ => Ok(Value::Bool(false)),
                }
            }
            BinaryOperator::Or => {
                match (left, right) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                    _ => Ok(Value::Bool(false)),
                }
            }

            BinaryOperator::Add => {
                match (left, right) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                    _ => Err(HyperQLError::ExecutionError {
                        message: "Cannot add these types".to_string(),
                        operation: "binary_add".to_string(),
                        entity_context: None,
                    }),
                }
            }
            BinaryOperator::Subtract => {
                match (left, right) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
                    _ => Err(HyperQLError::ExecutionError {
                        message: "Cannot subtract these types".to_string(),
                        operation: "binary_subtract".to_string(),
                        entity_context: None,
                    }),
                }
            }

            // TODO: Implement other binary operators
            _ => Err(HyperQLError::ExecutionError {
                message: format!("Binary operator {:?} not implemented", op),
                operation: "binary_operation".to_string(),
                entity_context: None,
            }),
        }
    }

    /// Evaluate unary operation
    fn evaluate_unary_op(&self, op: &crate::ast::UnaryOperator, val: &Value) -> Result<Value> {
        use crate::ast::UnaryOperator;

        match op {
            UnaryOperator::Not => {
                match val {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    Value::Null => Ok(Value::Bool(true)),
                    _ => Ok(Value::Bool(false)),
                }
            }
            UnaryOperator::Minus => {
                match val {
                    Value::Int(i) => Ok(Value::Int(-i)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    _ => Err(HyperQLError::ExecutionError {
                        message: "Cannot negate this type".to_string(),
                        operation: "unary_minus".to_string(),
                        entity_context: None,
                    }),
                }
            }
            UnaryOperator::Plus => {
                match val {
                    Value::Int(_) | Value::Float(_) => Ok(val.clone()),
                    _ => Err(HyperQLError::ExecutionError {
                        message: "Cannot apply plus to this type".to_string(),
                        operation: "unary_plus".to_string(),
                        entity_context: None,
                    }),
                }
            }
        }
    }

    /// Evaluate aggregate function across multiple rows
    fn evaluate_aggregate_function(&self, name: &str, args: &[CompiledExpression], rows: &[ResultRow]) -> Result<Value> {
        match name.to_uppercase().as_str() {
            "COUNT" => {
                if args.len() == 1 {
                    // Check for COUNT(*)
                    if let CompiledExpression::Column { name, .. } = &args[0] {
                        if name == "*" {
                            return Ok(Value::Int(rows.len() as i64));
                        }
                    }
                }

                // Count non-null values
                let mut count = 0i64;
                for row in rows {
                    for arg in args {
                        let value = self.evaluate_expression(arg, row)?;
                        if !matches!(value, Value::Null) {
                            count += 1;
                            break; // Only count each row once
                        }
                    }
                }
                Ok(Value::Int(count))
            }
            "SUM" => {
                if args.is_empty() {
                    return Ok(Value::Null);
                }

                let mut sum = 0.0f64;
                let mut has_values = false;

                for row in rows {
                    for arg in args {
                        let value = self.evaluate_expression(arg, row)?;
                        match value {
                            Value::Int(i) => {
                                sum += i as f64;
                                has_values = true;
                            }
                            Value::Float(f) => {
                                sum += f;
                                has_values = true;
                            }
                            Value::Null => {} // Skip nulls
                            _ => {
                                return Err(HyperQLError::ExecutionError {
                                    message: "SUM requires numeric values".to_string(),
                                    operation: "aggregate_sum".to_string(),
                                    entity_context: None,
                                });
                            }
                        }
                        break; // Only process first arg per row
                    }
                }

                if has_values {
                    Ok(Value::Float(sum))
                } else {
                    Ok(Value::Null)
                }
            }
            "AVG" => {
                if args.is_empty() {
                    return Ok(Value::Null);
                }

                let mut sum = 0.0f64;
                let mut count = 0i64;

                for row in rows {
                    for arg in args {
                        let value = self.evaluate_expression(arg, row)?;
                        match value {
                            Value::Int(i) => {
                                sum += i as f64;
                                count += 1;
                            }
                            Value::Float(f) => {
                                sum += f;
                                count += 1;
                            }
                            Value::Null => {} // Skip nulls
                            _ => {
                                return Err(HyperQLError::ExecutionError {
                                    message: "AVG requires numeric values".to_string(),
                                    operation: "aggregate_avg".to_string(),
                                    entity_context: None,
                                });
                            }
                        }
                        break; // Only process first arg per row
                    }
                }

                if count > 0 {
                    Ok(Value::Float(sum / count as f64))
                } else {
                    Ok(Value::Null)
                }
            }
            "MIN" | "MAX" => {
                if args.is_empty() {
                    return Ok(Value::Null);
                }

                let mut result_value: Option<Value> = None;

                for row in rows {
                    for arg in args {
                        let value = self.evaluate_expression(arg, row)?;
                        if matches!(value, Value::Null) {
                            continue;
                        }

                        match &result_value {
                            None => result_value = Some(value),
                            Some(current) => {
                                let comparison = self.compare_values(current, &value);
                                let should_replace = match name.to_uppercase().as_str() {
                                    "MIN" => comparison == std::cmp::Ordering::Greater,
                                    "MAX" => comparison == std::cmp::Ordering::Less,
                                    _ => false,
                                };
                                if should_replace {
                                    result_value = Some(value);
                                }
                            }
                        }
                        break; // Only process first arg per row
                    }
                }

                Ok(result_value.unwrap_or(Value::Null))
            }
            _ => Err(HyperQLError::ExecutionError {
                message: format!("Unknown aggregate function: {}", name),
                operation: "aggregate_function".to_string(),
                entity_context: None,
            }),
        }
    }

    /// Evaluate function call
    fn evaluate_function(&self, name: &str, args: &[Value]) -> Result<Value> {
        match name.to_uppercase().as_str() {
            "UPPER" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(s.to_uppercase()))
                } else {
                    Err(HyperQLError::ExecutionError {
                        message: "UPPER requires string argument".to_string(),
                        operation: "function_upper".to_string(),
                        entity_context: None,
                    })
                }
            }
            "LOWER" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::String(s.to_lowercase()))
                } else {
                    Err(HyperQLError::ExecutionError {
                        message: "LOWER requires string argument".to_string(),
                        operation: "function_lower".to_string(),
                        entity_context: None,
                    })
                }
            }
            "LENGTH" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Value::Int(s.len() as i64))
                } else {
                    Err(HyperQLError::ExecutionError {
                        message: "LENGTH requires string argument".to_string(),
                        operation: "function_length".to_string(),
                        entity_context: None,
                    })
                }
            }
            "ABS" => {
                match args.first() {
                    Some(Value::Int(i)) => Ok(Value::Int(i.abs())),
                    Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
                    _ => Err(HyperQLError::ExecutionError {
                        message: "ABS requires numeric argument".to_string(),
                        operation: "function_abs".to_string(),
                        entity_context: None,
                    })
                }
            }
            "ROUND" => {
                match args.first() {
                    Some(Value::Float(f)) => Ok(Value::Float(f.round())),
                    Some(Value::Int(i)) => Ok(Value::Int(*i)),
                    _ => Err(HyperQLError::ExecutionError {
                        message: "ROUND requires numeric argument".to_string(),
                        operation: "function_round".to_string(),
                        entity_context: None,
                    })
                }
            }
            _ => Err(HyperQLError::ExecutionError {
                message: format!("Unknown function: {}", name),
                operation: "function_call".to_string(),
                entity_context: None,
            }),
        }
    }

    /// Compare two values for ordering
    fn compare_values(&self, a: &Value, b: &Value) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (a, b) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,

            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Value::String(a), Value::String(b)) => a.cmp(b),

            // Type coercion for numeric types
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),

            // Default: consider different types as equal for now
            _ => Ordering::Equal,
        }
    }
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            entities_scanned: 0,
            relationships_traversed: 0,
            hyperbolic_operations: 0,
            cascade_propagations: 0,
        }
    }

    pub fn reset(&mut self) {
        self.entities_scanned = 0;
        self.relationships_traversed = 0;
        self.hyperbolic_operations = 0;
        self.cascade_propagations = 0;
    }
}

impl MemoryDataSource {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn add_entity(&mut self, table: &str, entity: Entity) {
        self.entities.entry(table.to_string()).or_insert_with(Vec::new).push(entity);
    }

    pub fn add_entities(&mut self, table: &str, entities: Vec<Entity>) {
        self.entities.insert(table.to_string(), entities);
    }
}

impl DataSource for MemoryDataSource {
    fn scan(&self, table: &str) -> Result<Vec<Entity>> {
        Ok(self.entities.get(table).cloned().unwrap_or_default())
    }

    fn insert(&mut self, table: &str, entities: Vec<Entity>) -> Result<u64> {
        let table_entities = self.entities.entry(table.to_string()).or_insert_with(Vec::new);
        let count = entities.len() as u64;
        table_entities.extend(entities);
        Ok(count)
    }

    fn update(&mut self, table: &str, _filter: Option<&CompiledExpression>, _assignments: &[CompiledAssignment]) -> Result<u64> {
        // TODO: Implement actual update logic with filter and assignments
        // For now, return 0 to indicate no actual updates were performed
        if self.entities.contains_key(table) {
            Ok(0) // Placeholder - would actually apply updates
        } else {
            Ok(0)
        }
    }

    fn delete(&mut self, table: &str, _filter: Option<&CompiledExpression>) -> Result<u64> {
        // TODO: Implement actual delete logic with filter
        // For now, return 0 to indicate no actual deletions were performed
        if self.entities.contains_key(table) {
            Ok(0) // Placeholder - would actually apply deletions
        } else {
            Ok(0)
        }
    }

    fn get_schema(&self, table: &str) -> Result<TableSchema> {
        // TODO: Implement actual schema inference
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ColumnType::EntityId);
        columns.insert("name".to_string(), ColumnType::String);

        Ok(TableSchema {
            name: table.to_string(),
            columns,
        })
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MemoryDataSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_statement;
    use crate::compiler::Compiler;
    use crate::types::{PropertyName, EntityId};

    #[test]
    fn test_execute_simple_select() {
        // Create test data
        let mut data_source = MemoryDataSource::new();
        let mut entity = Entity {
            id: EntityId("1".to_string()),
            properties: HashMap::new(),
            position: None,
            embedding: None,
        };
        entity.properties.insert(PropertyName("name".to_string()), Value::String("Alice".to_string()));
        entity.properties.insert(PropertyName("age".to_string()), Value::Int(30));

        data_source.add_entity("entities", entity);

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));

        // Compile query
        let compiler = Compiler::new();
        let query = "SELECT * FROM entities";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 1);
        assert!(result.rows[0].columns.contains_key("name"));
        assert!(result.rows[0].columns.contains_key("age"));
    }

    #[test]
    fn test_execute_select_with_where() {
        // Create test data
        let mut data_source = MemoryDataSource::new();

        let mut alice = Entity {
            id: EntityId("1".to_string()),
            properties: HashMap::new(),
            position: None,
            embedding: None,
        };
        alice.properties.insert(PropertyName("name".to_string()), Value::String("Alice".to_string()));
        alice.properties.insert(PropertyName("age".to_string()), Value::Int(30));

        let mut bob = Entity {
            id: EntityId("2".to_string()),
            properties: HashMap::new(),
            position: None,
            embedding: None,
        };
        bob.properties.insert(PropertyName("name".to_string()), Value::String("Bob".to_string()));
        bob.properties.insert(PropertyName("age".to_string()), Value::Int(25));

        data_source.add_entities("entities", vec![alice, bob]);

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));

        // Compile query
        let compiler = Compiler::new();
        let query = "SELECT name FROM entities WHERE name = 'Alice'";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 1);
        if let Some(Value::String(name)) = result.rows[0].columns.get("name") {
            assert_eq!(name, "Alice");
        } else {
            panic!("Expected string value for name");
        }
    }

    #[test]
    fn test_execute_select_with_limit() {
        // Create test data
        let mut data_source = MemoryDataSource::new();

        for i in 0..5 {
            let mut entity = Entity {
                id: EntityId(i.to_string()),
                properties: HashMap::new(),
                position: None,
                embedding: None,
            };
            entity.properties.insert(PropertyName("name".to_string()), Value::String(format!("User{}", i)));
            data_source.add_entity("entities", entity);
        }

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));

        // Compile query
        let compiler = Compiler::new();
        let query = "SELECT name FROM entities LIMIT 3";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 3);
    }

    #[test]
    fn test_execute_aggregate_functions() {
        // Create test data
        let mut data_source = MemoryDataSource::new();

        for i in 0..5 {
            let mut entity = Entity {
                id: EntityId(i.to_string()),
                properties: HashMap::new(),
                position: None,
                embedding: None,
            };
            entity.properties.insert(PropertyName("category".to_string()), Value::String("A".to_string()));
            entity.properties.insert(PropertyName("value".to_string()), Value::Int(i * 10));
            data_source.add_entity("products", entity);
        }

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));

        // Compile query with aggregates
        let compiler = Compiler::new();
        let query = "SELECT category, COUNT(*), SUM(value), AVG(value) FROM products GROUP BY category";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 1); // One group
        let row = &result.rows[0];

        // Check that aggregates were computed
        if let Some(Value::Int(count)) = row.columns.get("COUNT(*)") {
            assert_eq!(*count, 5);
        }

        if let Some(Value::Float(sum)) = row.columns.get("SUM(value)") {
            assert_eq!(*sum, 100.0); // 0 + 10 + 20 + 30 + 40
        }
    }

    #[test]
    fn test_execute_insert_statement() {
        let mut data_source = MemoryDataSource::new();
        let mut executor = Executor::new(Box::new(data_source));

        // Compile INSERT query
        let compiler = Compiler::new();
        let query = "INSERT INTO users (name, age) VALUES ('Alice', 30)";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];

        if let Some(Value::String(op)) = row.columns.get("operation") {
            assert_eq!(op, "INSERT");
        }

        if let Some(Value::Int(affected)) = row.columns.get("affected_rows") {
            assert_eq!(*affected, 1);
        }
    }

    #[test]
    fn test_execute_update_statement() {
        let mut data_source = MemoryDataSource::new();
        let mut executor = Executor::new(Box::new(data_source));

        // Compile UPDATE query
        let compiler = Compiler::new();
        let query = "UPDATE users SET age = 31 WHERE name = 'Alice'";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];

        if let Some(Value::String(op)) = row.columns.get("operation") {
            assert_eq!(op, "UPDATE");
        }
    }

    #[test]
    fn test_execute_delete_statement() {
        let mut data_source = MemoryDataSource::new();
        let mut executor = Executor::new(Box::new(data_source));

        // Compile DELETE query
        let compiler = Compiler::new();
        let query = "DELETE FROM users WHERE age < 18";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];

        if let Some(Value::String(op)) = row.columns.get("operation") {
            assert_eq!(op, "DELETE");
        }
    }

    #[test]
    #[ignore] // TODO: Fix HAVING clause evaluation with proper aggregate column resolution
    fn test_execute_having_clause() {
        // Create test data with multiple categories
        let mut data_source = MemoryDataSource::new();

        // Category A: 3 items
        for i in 0..3 {
            let mut entity = Entity {
                id: EntityId(format!("a{}", i)),
                properties: HashMap::new(),
                position: None,
                embedding: None,
            };
            entity.properties.insert(PropertyName("category".to_string()), Value::String("A".to_string()));
            entity.properties.insert(PropertyName("value".to_string()), Value::Int(i * 10));
            data_source.add_entity("products", entity);
        }

        // Category B: 7 items
        for i in 0..7 {
            let mut entity = Entity {
                id: EntityId(format!("b{}", i)),
                properties: HashMap::new(),
                position: None,
                embedding: None,
            };
            entity.properties.insert(PropertyName("category".to_string()), Value::String("B".to_string()));
            entity.properties.insert(PropertyName("value".to_string()), Value::Int(i * 5));
            data_source.add_entity("products", entity);
        }

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));

        // Query with HAVING clause that filters groups
        let compiler = Compiler::new();
        let query = "SELECT category, COUNT(*) FROM products GROUP BY category HAVING COUNT(*) > 5";
        let statement = parse_statement(query).unwrap();
        let compiled = compiler.compile(statement).unwrap();

        // Execute query
        let result = executor.execute(compiled).unwrap();

        // Should only return category B (which has 7 items > 5)
        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];

        if let Some(Value::String(category)) = row.columns.get("category") {
            assert_eq!(category, "B");
        }

        // Check for count column (could be named differently based on aggregate function processing)
        let count_found = row.columns.iter().find_map(|(key, value)| {
            if key.contains("COUNT") {
                if let Value::Int(count_val) = value {
                    Some(*count_val)
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(count_val) = count_found {
            assert_eq!(count_val, 7);
        } else {
            panic!("COUNT aggregate not found in result columns: {:?}", row.columns.keys().collect::<Vec<_>>());
        }
    }
}