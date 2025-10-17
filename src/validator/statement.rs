use crate::ast::*;
use crate::error::Result;
use crate::validator::{
    ValidationConfig, ValidationResult, ValidationError, ValidationWarning,
    ValidationErrorKind, ValidationWarningKind,
};
use std::collections::HashSet;

/// Statement-level validator for different query types
pub struct StatementValidator {
    config: ValidationConfig,
}

impl StatementValidator {
    /// Create a new statement validator
    pub fn new(config: &ValidationConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: &ValidationConfig) {
        self.config = config.clone();
    }

    /// Validate a statement
    pub fn validate(&self, statement: &Statement, result: &mut ValidationResult) -> Result<()> {
        result.metadata.phases_completed.push("statement".to_string());
        
        match statement {
            Statement::Select(select) => self.validate_select_statement(select, result),
            Statement::Insert(insert) => self.validate_insert_statement(insert, result),
            Statement::Update(update) => self.validate_update_statement(update, result),
            Statement::Delete(delete) => self.validate_delete_statement(delete, result),
            Statement::Schema(_schema_op) => {
                // TODO: Implement schema DDL validation
                // Schema statements need validation for field definitions, cascade configs, etc.
                Ok(())
            },
            Statement::Stream(_stream_op) => {
                // TODO: Implement stream DDL validation
                // Stream statements need validation for stream configs and consumer actions
                Ok(())
            },
        }
    }

    /// Validate SELECT statement structure and clauses
    fn validate_select_statement(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        // Validate projection
        self.validate_select_projection(&select.select_list, result)?;
        
        // Validate FROM clause
        if let Some(from_clause) = &select.from {
            self.validate_from_clause(from_clause, result)?;
        }
        
        // Validate clause relationships and ordering
        self.validate_clause_relationships(select, result)?;

        // Validate DISTINCT usage
        if select.distinct {
            self.validate_distinct_usage(select, result)?;
        }

        // Validate multi-paradigm specific constructs
        if let Some(traverse) = &select.traverse_clause {
            self.validate_traverse_statement(traverse, result)?;
        }

        Ok(())
    }

    /// Validate INSERT statement structure
    fn validate_insert_statement(&self, insert: &InsertStatement, result: &mut ValidationResult) -> Result<()> {
        // Validate table name
        if insert.table.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::StructuralError,
                    "INSERT statement requires a table name".to_string(),
                )
            );
        }

        // Validate column/value relationship
        if !insert.columns.is_empty() {
            // Validate column count matches value count for each row
            for (row_idx, value_row) in insert.values.iter().enumerate() {
                if insert.columns.len() != value_row.len() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::StructuralError,
                            format!(
                                "Row {}: Column count ({}) does not match value count ({})",
                                row_idx + 1,
                                insert.columns.len(),
                                value_row.len()
                            ),
                        )
                    );
                }
            }

            // Check for duplicate columns
            let mut seen_columns = HashSet::new();
            for column in &insert.columns {
                if !seen_columns.insert(column) {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::StructuralError,
                            format!("Duplicate column '{}' in INSERT statement", column),
                        )
                    );
                }
            }
        }

        // Validate that we have values to insert
        if insert.values.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::StructuralError,
                    "INSERT statement must specify values to insert".to_string(),
                )
            );
        }

        Ok(())
    }

    /// Validate UPDATE statement structure
    fn validate_update_statement(&self, update: &UpdateStatement, result: &mut ValidationResult) -> Result<()> {
        // Validate table name
        if update.table.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::StructuralError,
                    "UPDATE statement requires a table name".to_string(),
                )
            );
        }

        // Validate assignments
        if update.assignments.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::StructuralError,
                    "UPDATE statement requires at least one assignment".to_string(),
                )
            );
        }

        // Check for duplicate column assignments
        let mut assigned_columns = HashSet::new();
        for assignment in &update.assignments {
            if !assigned_columns.insert(&assignment.column) {
                result.add_error(
                    ValidationError::new(
                        ValidationErrorKind::StructuralError,
                        format!("Column '{}' assigned multiple times in UPDATE", assignment.column),
                    )
                );
            }
        }

        // Validate self-assignment patterns
        for assignment in &update.assignments {
            if let Expression::Column(col_ref) = &assignment.value {
                if col_ref.table.is_none() && col_ref.name == assignment.column {
                    result.add_warning(
                        ValidationWarning::new(
                            ValidationWarningKind::UnusualPattern,
                            format!("Self-assignment detected: {} = {}", assignment.column, assignment.column),
                        )
                        .with_suggestion("Self-assignments have no effect".to_string())
                    );
                }
            }
        }

        Ok(())
    }

    /// Validate DELETE statement structure
    fn validate_delete_statement(&self, delete: &DeleteStatement, result: &mut ValidationResult) -> Result<()> {
        // Validate table name
        if delete.table.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::StructuralError,
                    "DELETE statement requires a table name".to_string(),
                )
            );
        }

        Ok(())
    }

    /// Validate SELECT projection
    fn validate_select_projection(&self, projection: &[SelectItem], result: &mut ValidationResult) -> Result<()> {
        if projection.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::StructuralError,
                    "SELECT statement requires at least one projection item".to_string(),
                )
            );
        }

        // Check for mixed wildcard and explicit columns
        let has_wildcard = projection.iter().any(|item| matches!(item, SelectItem::Wildcard));
        let has_explicit = projection.iter().any(|item| matches!(item, SelectItem::Expression { .. }));

        if has_wildcard && has_explicit {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    "Mixing wildcard (*) with explicit columns - wildcard will include all columns".to_string(),
                )
                .with_suggestion("Use either wildcard or explicit columns, not both".to_string())
            );
        }

        // Check for duplicate aliases
        let mut aliases = HashSet::new();
        for item in projection {
            if let SelectItem::Expression { alias: Some(alias), .. } = item {
                if !aliases.insert(alias) {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::StructuralError,
                            format!("Duplicate alias '{}' in SELECT projection", alias),
                        )
                    );
                }
            }
        }

        Ok(())
    }

    /// Validate FROM clause
    fn validate_from_clause(&self, from: &FromClause, result: &mut ValidationResult) -> Result<()> {
        // Basic FROM clause validation
        match from {
            FromClause::Table { collection, entity_type, alias } => {
                // Check for empty collection name
                if collection.is_empty() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::StructuralError,
                            "Collection name cannot be empty".to_string(),
                        )
                    );
                }

                // Build table reference for alias comparison
                let table_ref = if entity_type.is_empty() {
                    collection.clone()
                } else {
                    format!("{}.{}", collection, entity_type)
                };

                // Check if alias conflicts with table name
                if let Some(alias_name) = alias {
                    if table_ref == *alias_name {
                        result.add_warning(
                            ValidationWarning::new(
                                ValidationWarningKind::AmbiguousReference,
                                format!("Table alias '{}' is the same as table name", alias_name),
                            )
                            .with_suggestion("Use different alias to avoid confusion".to_string())
                        );
                    }
                }
            }
            FromClause::Subquery { alias, .. } => {
                // Subquery must have an alias
                if alias.is_empty() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::StructuralError,
                            "Subquery must have an alias".to_string(),
                        )
                    );
                }
            }
        }

        Ok(())
    }

    /// Validate clause relationships and ordering
    fn validate_clause_relationships(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        // HAVING without GROUP BY (handled in semantic validator, but double-check here)
        if select.having.is_some() && select.group_by.is_empty() {
            // Check if all SELECT items are aggregates
            let all_aggregates = select.select_list.iter().all(|item| {
                match item {
                    SelectItem::Expression { expr, .. } => self.is_aggregate_expression(expr),
                    SelectItem::Wildcard => false,
                }
            });

            if !all_aggregates {
                result.add_error(
                    ValidationError::new(
                        ValidationErrorKind::StructuralError,
                        "HAVING clause requires GROUP BY clause unless all SELECT items are aggregates".to_string(),
                    )
                );
            }
        }

        // ORDER BY with LIMIT recommendations
        if !select.order_by.is_empty() && select.limit.is_none() {
            if self.config.warn_performance_issues {
                result.add_warning(
                    ValidationWarning::new(
                        ValidationWarningKind::PerformanceWarning,
                        "ORDER BY without LIMIT may be inefficient for large result sets".to_string(),
                    )
                    .with_suggestion("Consider adding LIMIT if you don't need all results".to_string())
                );
            }
        }

        // OFFSET without LIMIT
        if select.offset.is_some() && select.limit.is_none() {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    "OFFSET without LIMIT may have database-specific behavior".to_string(),
                )
                .with_suggestion("Add LIMIT clause for predictable results".to_string())
            );
        }

        Ok(())
    }

    /// Validate DISTINCT usage
    fn validate_distinct_usage(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        // DISTINCT with ORDER BY - check if ORDER BY columns are in SELECT
        if !select.order_by.is_empty() {
            for order_item in &select.order_by {
                if !self.is_expression_in_projection(&order_item.expr, &select.select_list) {
                    result.add_warning(
                        ValidationWarning::new(
                            ValidationWarningKind::UnusualPattern,
                            "ORDER BY expression not in SELECT list with DISTINCT".to_string(),
                        )
                        .with_suggestion("Add ORDER BY expressions to SELECT list when using DISTINCT".to_string())
                    );
                }
            }
        }

        // DISTINCT with wildcard might be inefficient
        if select.select_list.iter().any(|item| matches!(item, SelectItem::Wildcard)) {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::PerformanceWarning,
                    "DISTINCT with wildcard (*) may be inefficient".to_string(),
                )
                .with_suggestion("Consider selecting only necessary columns with DISTINCT".to_string())
            );
        }

        Ok(())
    }

    /// Validate TRAVERSE clause (graph operations)
    fn validate_traverse_statement(&self, _traverse: &TraverseClause, result: &mut ValidationResult) -> Result<()> {
        // TODO: Implement specific traverse validation based on AST structure
        // For now, just add placeholder validation
        
        result.metadata.phases_completed.push("traverse_statement".to_string());
        
        // Basic validation - ensure we have required components
        // This would be expanded based on the actual TraverseClause structure
        
        Ok(())
    }

    // Helper methods

    /// Check if expression is an aggregate
    fn is_aggregate_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Function { name, .. } => {
                matches!(name.to_uppercase().as_str(),
                    "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | 
                    "STDDEV" | "VARIANCE" | "GROUP_CONCAT" | "STRING_AGG"
                )
            }
            _ => false,
        }
    }

    /// Check if expression is in projection list
    fn is_expression_in_projection(&self, expr: &Expression, projection: &[SelectItem]) -> bool {
        for item in projection {
            match item {
                SelectItem::Expression { expr: proj_expr, .. } => {
                    if self.expressions_equivalent(expr, proj_expr) {
                        return true;
                    }
                }
                SelectItem::Wildcard => {
                    // Wildcard includes all columns, so ORDER BY columns might be included
                    return true;
                }
            }
        }
        false
    }

    /// Compare expressions for equivalence (simplified)
    fn expressions_equivalent(&self, expr1: &Expression, expr2: &Expression) -> bool {
        match (expr1, expr2) {
            (Expression::Column(col1), Expression::Column(col2)) => {
                col1.name == col2.name && col1.table == col2.table
            }
            (Expression::Literal(lit1), Expression::Literal(lit2)) => lit1 == lit2,
            (Expression::Function { name: n1, args: a1 }, Expression::Function { name: n2, args: a2 }) => {
                n1 == n2 && a1.len() == a2.len() && 
                a1.iter().zip(a2.iter()).all(|(arg1, arg2)| self.expressions_equivalent(arg1, arg2))
            }
            // Add more cases as needed
            _ => false,
        }
    }

    /// Validate reserved words and identifiers
    #[allow(dead_code)]
    fn validate_identifier(&self, name: &str, context: &str, result: &mut ValidationResult) {
        // Check for SQL reserved words
        let reserved_words = [
            "SELECT", "FROM", "WHERE", "GROUP", "ORDER", "BY", "HAVING", "DISTINCT",
            "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "TABLE",
            "INDEX", "VIEW", "UNION", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER",
            "ON", "AS", "AND", "OR", "NOT", "NULL", "TRUE", "FALSE", "LIKE",
            "IN", "EXISTS", "BETWEEN", "CASE", "WHEN", "THEN", "ELSE", "END"
        ];

        if reserved_words.contains(&name.to_uppercase().as_str()) {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::SafetyWarning,
                    format!("'{}' is a reserved word used as {}", name, context),
                )
                .with_suggestion("Consider using quoted identifiers or different names".to_string())
            );
        }

        // Check identifier naming conventions
        if name.chars().any(|c| c.is_whitespace()) {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    format!("{} '{}' contains whitespace", context, name),
                )
                .with_suggestion("Consider using underscores instead of spaces".to_string())
            );
        }

        if name.len() > 64 {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    format!("{} '{}' is very long ({}+ characters)", context, name, name.len()),
                )
                .with_suggestion("Consider shorter, more descriptive names".to_string())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_select_projection_validation() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        // Empty projection should error
        let empty_projection: Vec<SelectItem> = vec![];
        validator.validate_select_projection(&empty_projection, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("at least one projection")));
    }

    #[test]
    fn test_mixed_wildcard_projection_warning() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        let mixed_projection = vec![
            SelectItem::Wildcard,
            SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "name".to_string(),
                }),
                alias: None,
            },
        ];

        validator.validate_select_projection(&mixed_projection, &mut result).unwrap();

        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.message.contains("Mixing wildcard")));
    }

    #[test]
    fn test_duplicate_alias_error() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        let duplicate_alias_projection = vec![
            SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "name".to_string(),
                }),
                alias: Some("alias1".to_string()),
            },
            SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "age".to_string(),
                }),
                alias: Some("alias1".to_string()), // Duplicate alias
            },
        ];

        validator.validate_select_projection(&duplicate_alias_projection, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Duplicate alias 'alias1'")));
    }

    #[test]
    fn test_insert_column_value_mismatch() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        let insert = InsertStatement {
            table: "users".to_string(),
            columns: vec!["name".to_string(), "age".to_string()], // 2 columns
            values: vec![vec![Expression::Literal(Literal::String("John".to_string()))]], // 1 value in 1 row
        };

        validator.validate_insert_statement(&insert, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Column count (2) does not match value count (1)")));
    }

    #[test]
    fn test_update_duplicate_assignment() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        let update = UpdateStatement {
            table: "users".to_string(),
            assignments: vec![
                Assignment {
                    column: "name".to_string(),
                    value: Expression::Literal(Literal::String("John".to_string())),
                },
                Assignment {
                    column: "name".to_string(), // Duplicate assignment
                    value: Expression::Literal(Literal::String("Jane".to_string())),
                },
            ],
            where_clause: None,
        };

        validator.validate_update_statement(&update, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Column 'name' assigned multiple times")));
    }

    #[test]
    fn test_self_assignment_warning() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        let update = UpdateStatement {
            table: "users".to_string(),
            assignments: vec![
                Assignment {
                    column: "name".to_string(),
                    value: Expression::Column(ColumnRef {
                        table: None,
                        name: "name".to_string(), // Self-assignment
                    }),
                },
            ],
            where_clause: None,
        };

        validator.validate_update_statement(&update, &mut result).unwrap();

        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.message.contains("Self-assignment detected")));
    }

    #[test]
    fn test_distinct_with_wildcard_warning() {
        let config = ValidationConfig::default();
        let validator = StatementValidator::new(&config);
        let mut result = ValidationResult::new();

        let select = SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: Some(FromClause::Table {
                collection: "users".to_string(),
                entity_type: String::new(),
                alias: None,
            }),
            joins: Vec::new(),
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: true, // DISTINCT with wildcard
            traverse_clause: None,
        };

        validator.validate_distinct_usage(&select, &mut result).unwrap();

        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.message.contains("DISTINCT with wildcard")));
    }
}