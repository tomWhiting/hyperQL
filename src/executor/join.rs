//! JOIN execution engine
//!
//! This module implements various JOIN algorithms for combining data from multiple tables.
//! It supports all standard SQL JOIN types with optimized execution strategies.
//!
//! ## JOIN Types Supported
//!
//! - **INNER JOIN**: Returns only rows with matching values in both tables
//! - **LEFT JOIN**: Returns all rows from left table, with matching rows from right (NULLs for non-matches)
//! - **RIGHT JOIN**: Returns all rows from right table, with matching rows from left (NULLs for non-matches)
//! - **FULL OUTER JOIN**: Returns all rows from both tables, with NULLs for non-matches
//!
//! ## JOIN Algorithms
//!
//! - **Hash Join**: Used for equality predicates (e.g., a.id = b.id). Builds hash table on smaller
//!   relation and probes with larger relation. O(N + M) time complexity.
//! - **Nested Loop Join**: Used for complex predicates (e.g., a.value < b.value). Iterates through
//!   all combinations. O(N * M) time complexity but handles arbitrary join conditions.
//!
//! ## Memory Management
//!
//! - Hash tables are built in-memory for the right (joined) table
//! - Left table is streamed to minimize memory usage
//! - For large datasets, consider implementing disk-based hash join or sort-merge join

use crate::compiler::{CompiledExpression, JoinType};
use crate::types::{ResultRow, Value};
use crate::error::Result;
use super::expression_eval::ExpressionEvaluator;
use std::collections::HashMap;

/// JOIN executor implementing hash join and nested loop join algorithms
pub struct JoinExecutor;

impl JoinExecutor {
    /// Execute a JOIN operation
    pub fn execute_join(
        left_rows: Vec<ResultRow>,
        right_rows: Vec<ResultRow>,
        join_type: &JoinType,
        on_condition: &CompiledExpression,
        evaluator: &ExpressionEvaluator,
        left_alias: &Option<String>,
        right_alias: &Option<String>,
    ) -> Result<Vec<ResultRow>> {
        // Determine join algorithm based on condition type
        if Self::is_equality_join(on_condition) {
            Self::hash_join(left_rows, right_rows, join_type, on_condition, evaluator, left_alias, right_alias)
        } else {
            Self::nested_loop_join(left_rows, right_rows, join_type, on_condition, evaluator, left_alias, right_alias)
        }
    }

    /// Check if join condition is a simple equality (eligible for hash join)
    fn is_equality_join(condition: &CompiledExpression) -> bool {
        match condition {
            CompiledExpression::Binary { op, .. } => {
                matches!(op, crate::ast::BinaryOperator::Equal)
            }
            _ => false,
        }
    }

    /// Hash join implementation for equality predicates
    ///
    /// Algorithm:
    /// 1. Build phase: Create hash table from right table using join key
    /// 2. Probe phase: For each left row, lookup matches in hash table
    /// 3. Combine matching rows into result set
    fn hash_join(
        left_rows: Vec<ResultRow>,
        right_rows: Vec<ResultRow>,
        join_type: &JoinType,
        on_condition: &CompiledExpression,
        _evaluator: &ExpressionEvaluator,
        left_alias: &Option<String>,
        right_alias: &Option<String>,
    ) -> Result<Vec<ResultRow>> {
        // Extract join keys from condition (e.g., a.id = b.id)
        let (left_key, right_key) = Self::extract_join_keys(on_condition)?;

        // Build phase: Hash right table
        let mut hash_table: HashMap<String, Vec<ResultRow>> = HashMap::new();
        for right_row in &right_rows {
            if let Some(key_value) = Self::lookup_column_flexible(right_row, &right_key) {
                let key_str = Self::value_to_key_string(key_value);
                hash_table.entry(key_str).or_insert_with(Vec::new).push(right_row.clone());
            }
        }

        // Track which right rows were matched (for FULL OUTER JOIN)
        let mut matched_right_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let right_rows_indexed: Vec<_> = right_rows.iter().enumerate().collect();

        let mut result = Vec::new();

        // Probe phase: For each left row, find matches
        for left_row in &left_rows {
            if let Some(key_value) = Self::lookup_column_flexible(left_row, &left_key) {
                let key_str = Self::value_to_key_string(key_value);

                if let Some(matching_right_rows) = hash_table.get(&key_str) {
                    // Found matches - emit joined rows
                    for right_row in matching_right_rows {
                        // Mark this right row as matched
                        if let Some((idx, _)) = right_rows_indexed.iter().find(|(_, r)| *r == right_row) {
                            matched_right_indices.insert(*idx);
                        }

                        let joined_row = Self::combine_rows(left_row, right_row, left_alias, right_alias);
                        result.push(joined_row);
                    }
                } else {
                    // No match found
                    match join_type {
                        JoinType::Left | JoinType::FullOuter => {
                            // Include left row with NULLs for right columns
                            let joined_row = Self::combine_rows_with_null_right(left_row, &right_rows, left_alias, right_alias);
                            result.push(joined_row);
                        }
                        _ => {
                            // INNER and RIGHT JOIN - skip unmatched left rows
                        }
                    }
                }
            } else {
                // Left row has NULL join key
                match join_type {
                    JoinType::Left | JoinType::FullOuter => {
                        // Include left row with NULLs for right columns
                        let joined_row = Self::combine_rows_with_null_right(left_row, &right_rows, left_alias, right_alias);
                        result.push(joined_row);
                    }
                    _ => {
                        // INNER and RIGHT JOIN - skip rows with NULL keys
                    }
                }
            }
        }

        // For RIGHT and FULL OUTER JOIN, include unmatched right rows
        match join_type {
            JoinType::Right | JoinType::FullOuter => {
                for (idx, right_row) in right_rows_indexed {
                    if !matched_right_indices.contains(&idx) {
                        // Unmatched right row - include with NULLs for left columns
                        let joined_row = Self::combine_rows_with_null_left(&left_rows, right_row, left_alias, right_alias);
                        result.push(joined_row);
                    }
                }
            }
            _ => {
                // LEFT and INNER JOIN don't include unmatched right rows
            }
        }

        Ok(result)
    }

    /// Nested loop join implementation for complex predicates
    ///
    /// Algorithm:
    /// 1. For each row in left table
    /// 2. For each row in right table
    /// 3. Evaluate join condition
    /// 4. If condition is true, emit combined row
    fn nested_loop_join(
        left_rows: Vec<ResultRow>,
        right_rows: Vec<ResultRow>,
        join_type: &JoinType,
        on_condition: &CompiledExpression,
        evaluator: &ExpressionEvaluator,
        left_alias: &Option<String>,
        right_alias: &Option<String>,
    ) -> Result<Vec<ResultRow>> {
        let mut result = Vec::new();
        let mut matched_right_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for left_row in &left_rows {
            let mut found_match = false;

            for (right_idx, right_row) in right_rows.iter().enumerate() {
                // Combine rows temporarily to evaluate condition
                let combined_row = Self::combine_rows(left_row, right_row, left_alias, right_alias);

                // Evaluate join condition
                if evaluator.evaluate_predicate(on_condition, &combined_row)? {
                    found_match = true;
                    matched_right_indices.insert(right_idx);
                    result.push(combined_row);
                }
            }

            // Handle unmatched left rows for LEFT and FULL OUTER JOIN
            if !found_match {
                match join_type {
                    JoinType::Left | JoinType::FullOuter => {
                        let joined_row = Self::combine_rows_with_null_right(left_row, &right_rows, left_alias, right_alias);
                        result.push(joined_row);
                    }
                    _ => {
                        // INNER and RIGHT JOIN - skip unmatched left rows
                    }
                }
            }
        }

        // Handle unmatched right rows for RIGHT and FULL OUTER JOIN
        match join_type {
            JoinType::Right | JoinType::FullOuter => {
                for (right_idx, right_row) in right_rows.iter().enumerate() {
                    if !matched_right_indices.contains(&right_idx) {
                        let joined_row = Self::combine_rows_with_null_left(&left_rows, right_row, left_alias, right_alias);
                        result.push(joined_row);
                    }
                }
            }
            _ => {
                // LEFT and INNER JOIN don't include unmatched right rows
            }
        }

        Ok(result)
    }

    /// Extract left and right join keys from equality condition
    fn extract_join_keys(condition: &CompiledExpression) -> Result<(String, String)> {
        match condition {
            CompiledExpression::Binary { left, op, right, .. } => {
                if !matches!(op, crate::ast::BinaryOperator::Equal) {
                    return Err(crate::error::HyperQLError::SemanticError {
                        message: "Hash join requires equality predicate".to_string(),
                        context: vec!["join".to_string()],
                    });
                }

                let left_key = Self::extract_column_name(left)?;
                let right_key = Self::extract_column_name(right)?;

                Ok((left_key, right_key))
            }
            _ => Err(crate::error::HyperQLError::SemanticError {
                message: "Invalid join condition format".to_string(),
                context: vec!["join".to_string()],
            }),
        }
    }

    /// Extract column name from expression
    ///
    /// For nested joins, columns may be prefixed with table/alias (e.g., "a_hadm_id").
    /// For simple scans, columns are bare (e.g., "hadm_id").
    /// We try the prefixed format first, then fall back to bare.
    fn extract_column_name(expr: &CompiledExpression) -> Result<String> {
        match expr {
            CompiledExpression::Column { name, table, .. } => {
                // Return the column name with table prefix if present
                // The actual lookup will try both prefixed and bare formats
                if let Some(table_name) = table {
                    Ok(format!("{}_{}", table_name, name))
                } else {
                    Ok(name.clone())
                }
            }
            _ => Err(crate::error::HyperQLError::SemanticError {
                message: "Join key must be a column reference".to_string(),
                context: vec!["join".to_string()],
            }),
        }
    }

    /// Flexible column lookup that tries multiple naming formats
    ///
    /// For nested joins, columns may be prefixed (e.g., "a_hadm_id").
    /// For fresh scans, columns are bare (e.g., "hadm_id").
    /// This function tries:
    /// 1. Exact key (e.g., "a_hadm_id" for nested joins)
    /// 2. Bare column name (e.g., "hadm_id" if key was "a_hadm_id")
    fn lookup_column_flexible<'a>(row: &'a ResultRow, key: &str) -> Option<&'a Value> {
        // Try exact match first
        if let Some(value) = row.columns.get(key) {
            return Some(value);
        }

        // If key contains underscore (e.g., "a_hadm_id"), try without prefix
        if key.contains('_') {
            if let Some(pos) = key.find('_') {
                let bare_name = &key[pos + 1..];
                if let Some(value) = row.columns.get(bare_name) {
                    return Some(value);
                }
            }
        }

        None
    }

    /// Convert value to string for hash key
    fn value_to_key_string(value: &Value) -> String {
        match value {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::EntityId(id) => id.0.clone(),
            Value::Position(pos) => format!("({},{},{})", pos.x, pos.y, pos.z),
            Value::Distance(d) => d.0.to_string(),
            Value::Timestamp(ts) => ts.to_string(),
            Value::Duration(dur) => dur.to_string(),
            _ => format!("{:?}", value),
        }
    }

    /// Combine two rows into a single row with column prefixing
    fn combine_rows(
        left: &ResultRow,
        right: &ResultRow,
        left_alias: &Option<String>,
        right_alias: &Option<String>,
    ) -> ResultRow {
        let mut columns = HashMap::new();

        // Add all left columns with prefix
        for (key, value) in &left.columns {
            let prefixed_key = if let Some(alias) = left_alias {
                format!("{}_{}", alias, key)
            } else {
                key.clone()
            };
            columns.insert(prefixed_key, value.clone());
        }

        // Add all right columns with prefix
        for (key, value) in &right.columns {
            let prefixed_key = if let Some(alias) = right_alias {
                format!("{}_{}", alias, key)
            } else {
                key.clone()
            };
            columns.insert(prefixed_key, value.clone());
        }

        ResultRow { columns }
    }

    /// Combine left row with NULL values for all right columns
    fn combine_rows_with_null_right(
        left: &ResultRow,
        right_sample: &[ResultRow],
        left_alias: &Option<String>,
        right_alias: &Option<String>,
    ) -> ResultRow {
        let mut columns = HashMap::new();

        // Add all left columns with prefix
        for (key, value) in &left.columns {
            let prefixed_key = if let Some(alias) = left_alias {
                format!("{}_{}", alias, key)
            } else {
                key.clone()
            };
            columns.insert(prefixed_key, value.clone());
        }

        // Add NULL for all right columns with prefix (sample from first right row if available)
        if let Some(sample_row) = right_sample.first() {
            for key in sample_row.columns.keys() {
                let prefixed_key = if let Some(alias) = right_alias {
                    format!("{}_{}", alias, key)
                } else {
                    key.clone()
                };
                if !columns.contains_key(&prefixed_key) {
                    columns.insert(prefixed_key, Value::Null);
                }
            }
        }

        ResultRow { columns }
    }

    /// Combine NULL values for all left columns with right row
    fn combine_rows_with_null_left(
        left_sample: &[ResultRow],
        right: &ResultRow,
        left_alias: &Option<String>,
        right_alias: &Option<String>,
    ) -> ResultRow {
        let mut columns = HashMap::new();

        // Add NULL for all left columns with prefix (sample from first left row if available)
        if let Some(sample_row) = left_sample.first() {
            for key in sample_row.columns.keys() {
                let prefixed_key = if let Some(alias) = left_alias {
                    format!("{}_{}", alias, key)
                } else {
                    key.clone()
                };
                columns.insert(prefixed_key, Value::Null);
            }
        }

        // Add all right columns with prefix
        for (key, value) in &right.columns {
            let prefixed_key = if let Some(alias) = right_alias {
                format!("{}_{}", alias, key)
            } else {
                key.clone()
            };
            columns.insert(prefixed_key, value.clone());
        }

        ResultRow { columns }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CompiledExpression;
    use crate::types::{Value, ResultRow};
    use std::collections::HashMap;

    fn create_test_row(id: i64, name: &str) -> ResultRow {
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), Value::Int(id));
        columns.insert("name".to_string(), Value::String(name.to_string()));
        ResultRow { columns }
    }

    fn create_test_row_with_table(table: &str, id: i64, value: &str) -> ResultRow {
        let mut columns = HashMap::new();
        columns.insert(format!("{}.id", table), Value::Int(id));
        columns.insert(format!("{}.value", table), Value::String(value.to_string()));
        ResultRow { columns }
    }

    #[test]
    fn test_is_equality_join() {
        let eq_condition = CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Column {
                table: Some("a".to_string()),
                name: "id".to_string(),
                value_type: crate::compiler::ValueType::Int,
            }),
            op: crate::ast::BinaryOperator::Equal,
            right: Box::new(CompiledExpression::Column {
                table: Some("b".to_string()),
                name: "id".to_string(),
                value_type: crate::compiler::ValueType::Int,
            }),
            result_type: crate::compiler::ValueType::Bool,
        };

        assert!(JoinExecutor::is_equality_join(&eq_condition));
    }

    #[test]
    fn test_extract_join_keys() {
        let condition = CompiledExpression::Binary {
            left: Box::new(CompiledExpression::Column {
                table: Some("a".to_string()),
                name: "id".to_string(),
                value_type: crate::compiler::ValueType::Int,
            }),
            op: crate::ast::BinaryOperator::Equal,
            right: Box::new(CompiledExpression::Column {
                table: Some("b".to_string()),
                name: "id".to_string(),
                value_type: crate::compiler::ValueType::Int,
            }),
            result_type: crate::compiler::ValueType::Bool,
        };

        let (left_key, right_key) = JoinExecutor::extract_join_keys(&condition).unwrap();
        assert_eq!(left_key, "a_id");
        assert_eq!(right_key, "b_id");
    }

    #[test]
    fn test_combine_rows() {
        let left = create_test_row(1, "Alice");
        let mut right_columns = HashMap::new();
        right_columns.insert("age".to_string(), Value::Int(30));
        let right = ResultRow { columns: right_columns };

        let left_alias = Some("l".to_string());
        let right_alias = Some("r".to_string());
        let combined = JoinExecutor::combine_rows(&left, &right, &left_alias, &right_alias);

        assert_eq!(combined.columns.get("l_id"), Some(&Value::Int(1)));
        assert_eq!(combined.columns.get("l_name"), Some(&Value::String("Alice".to_string())));
        assert_eq!(combined.columns.get("r_age"), Some(&Value::Int(30)));
    }
}
