use crate::ast::*;
use crate::error::{HyperQLError, Result};
use crate::validator::{
    ValidationConfig, ValidationResult, ValidationError, ValidationWarning,
    ValidationErrorKind, ValidationWarningKind,
};

/// Semantic validator for core query structure and logic validation
pub struct SemanticValidator {
    config: ValidationConfig,
}

impl SemanticValidator {
    /// Create a new semantic validator
    pub fn new(config: &ValidationConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: &ValidationConfig) {
        self.config = config.clone();
    }

    /// Validate a statement semantically
    pub fn validate(&self, statement: &Statement, result: &mut ValidationResult) -> Result<()> {
        result.metadata.phases_completed.push("semantic".to_string());
        
        match statement {
            Statement::Select(select) => self.validate_select(select, result),
            Statement::Insert(insert) => self.validate_insert(insert, result),
            Statement::Update(update) => self.validate_update(update, result),
            Statement::Delete(delete) => self.validate_delete(delete, result),
            Statement::Schema(_schema_op) => {
                // TODO: Implement schema semantic validation
                // Validate semantic consistency of schema operations
                Ok(())
            },
            Statement::Stream(_stream_op) => {
                // TODO: Implement stream semantic validation
                // Validate semantic consistency of stream operations
                Ok(())
            },
        }
    }

    /// Validate SELECT statement semantics
    fn validate_select(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        // Track tables and columns for metadata
        if let Some(from_clause) = &select.from {
            self.collect_table_references(from_clause, result);
        }
        
        // Validate structural relationships
        self.validate_select_structure(select, result)?;
        
        // Validate aggregation usage
        self.validate_aggregation_usage(select, result)?;
        
        // Validate GROUP BY/HAVING relationship
        self.validate_group_having_relationship(select, result)?;
        
        // Validate ORDER BY references
        self.validate_order_by_references(select, result)?;
        
        // Validate LIMIT/OFFSET usage
        self.validate_limit_offset_usage(select, result)?;

        // Validate multi-paradigm constructs
        if let Some(traverse) = &select.traverse_clause {
            self.validate_traverse_clause(traverse, result)?;
        }

        Ok(())
    }

    /// Validate basic SELECT structure
    fn validate_select_structure(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        // Check for SELECT without FROM (allow in some cases)
        if select.from.is_none() && !self.is_constant_query(select) {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    "SELECT without FROM clause - query will return single row".to_string(),
                )
                .with_suggestion("Consider adding FROM clause if you need to query data".to_string())
            );
        }

        // Check for wildcard usage with multiple tables (not applicable with current AST structure)
        // TODO: Add support for multiple table references when FROM clause supports it

        Ok(())
    }

    /// Validate aggregation usage rules
    fn validate_aggregation_usage(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        let _select_aggregates = self.find_aggregates_in_projections(&select.select_list);
        let where_aggregates = self.find_aggregates_in_expression(select.where_clause.as_ref());
        let having_aggregates = self.find_aggregates_in_expression(select.having.as_ref());

        // Rule: No aggregates in WHERE clause
        if !where_aggregates.is_empty() {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::AggregationError,
                    "Aggregate functions are not allowed in WHERE clause".to_string(),
                )
                .with_suggestions(vec![
                    "Use HAVING clause for aggregate conditions".to_string(),
                    "Move aggregate conditions from WHERE to HAVING".to_string(),
                ])
            );
        }

        // Rule: HAVING clause requires GROUP BY (unless all SELECT is aggregates)
        if !having_aggregates.is_empty() && select.group_by.is_empty() && !self.is_fully_aggregated_select(select) {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::AggregationError,
                    "HAVING clause requires GROUP BY clause or fully aggregated SELECT".to_string(),
                )
                .with_suggestions(vec![
                    "Add GROUP BY clause".to_string(),
                    "Remove HAVING clause".to_string(),
                    "Make all SELECT expressions aggregated".to_string(),
                ])
            );
        }

        // Rule: Check for nested aggregates
        if let Err(nested_error) = self.validate_no_nested_aggregates(&select.select_list) {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::AggregationError,
                    nested_error.to_string(),
                )
                .with_suggestions(vec![
                    "Remove nested aggregate functions".to_string(),
                    "Use subqueries for complex aggregations".to_string(),
                ])
            );
        }

        Ok(())
    }

    /// Validate GROUP BY and HAVING relationship
    fn validate_group_having_relationship(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        if select.group_by.is_empty() {
            return Ok(());
        }

        // With GROUP BY, all non-aggregate SELECT expressions must be in GROUP BY
        for projection in &select.select_list {
            match projection {
                SelectItem::Expression { expr, .. } => {
                    if !self.is_aggregate_or_constant(expr) && !self.is_in_group_by(expr, &select.group_by) {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::AggregationError,
                                "Non-aggregated column in SELECT must appear in GROUP BY clause".to_string(),
                            )
                            .with_suggestions(vec![
                                "Add column to GROUP BY clause".to_string(),
                                "Use an aggregate function on the column".to_string(),
                            ])
                        );
                    }
                }
                SelectItem::Wildcard => {
                    // Wildcard with GROUP BY is generally problematic
                    result.add_warning(
                        ValidationWarning::new(
                            ValidationWarningKind::UnusualPattern,
                            "Wildcard projection with GROUP BY may not work as expected".to_string(),
                        )
                        .with_suggestion("Use explicit column selection with GROUP BY".to_string())
                    );
                }
            }
        }

        Ok(())
    }

    /// Validate ORDER BY references
    fn validate_order_by_references(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        for order_item in &select.order_by {
            // Check if ORDER BY expression is valid
            if let Err(_) = self.validate_order_by_expression(&order_item.expr, select) {
                result.add_error(
                    ValidationError::new(
                        ValidationErrorKind::ExpressionError,
                        "Invalid expression in ORDER BY clause".to_string(),
                    )
                    .with_suggestions(vec![
                        "Use column names or aliases from SELECT clause".to_string(),
                        "Ensure ORDER BY expression is valid".to_string(),
                    ])
                );
            }
        }

        Ok(())
    }

    /// Validate LIMIT and OFFSET usage
    fn validate_limit_offset_usage(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        // Check for OFFSET without LIMIT
        if select.limit.is_none() && select.offset.is_some() {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    "OFFSET without LIMIT may have unexpected behavior".to_string(),
                )
                .with_suggestion("Consider adding LIMIT clause with OFFSET".to_string())
            );
        }

        // Check for invalid LIMIT/OFFSET values
        if let Some(limit_value) = select.limit {
            if limit_value == 0 {
                result.add_error(
                    ValidationError::new(
                        ValidationErrorKind::ExpressionError,
                        "LIMIT value must be positive".to_string(),
                    )
                );
            }
        }

        // OFFSET is already validated as u64 so it's always non-negative
        // No additional validation needed for offset

        Ok(())
    }

    /// Validate TRAVERSE clause (graph operations)
    fn validate_traverse_clause(&self, _traverse: &TraverseClause, result: &mut ValidationResult) -> Result<()> {
        // TODO: Add specific traverse validation based on AST structure
        // For now, mark as validated
        result.metadata.phases_completed.push("traverse_validation".to_string());

        Ok(())
    }

    /// Validate INSERT statement
    fn validate_insert(&self, insert: &InsertStatement, result: &mut ValidationResult) -> Result<()> {
        result.metadata.tables_referenced.push(insert.table.clone());

        // Validate column count matches values count for each row
        if !insert.columns.is_empty() {
            for (row_idx, value_row) in insert.values.iter().enumerate() {
                if insert.columns.len() != value_row.len() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::StructuralError,
                            format!(
                                "Row {}: Number of columns ({}) does not match number of values ({})",
                                row_idx + 1,
                                insert.columns.len(),
                                value_row.len()
                            ),
                        )
                        .with_suggestions(vec![
                            "Ensure each row has the same number of values as columns".to_string(),
                            "Check for missing or extra values in INSERT statement".to_string(),
                        ])
                    );
                }
            }
        }

        Ok(())
    }

    /// Validate UPDATE statement
    fn validate_update(&self, update: &UpdateStatement, result: &mut ValidationResult) -> Result<()> {
        result.metadata.tables_referenced.push(update.table.clone());

        // Validate assignments
        for assignment in &update.assignments {
            // Check for assignment to computed columns or system columns
            if self.is_system_column(&assignment.column) {
                result.add_warning(
                    ValidationWarning::new(
                        ValidationWarningKind::SafetyWarning,
                        format!("Updating system column '{}' may have unexpected effects", assignment.column),
                    )
                    .with_suggestion("Avoid updating system-managed columns".to_string())
                );
            }
        }

        // Warn about UPDATE without WHERE
        if update.where_clause.is_none() {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::SafetyWarning,
                    "UPDATE without WHERE clause will update all rows".to_string(),
                )
                .with_suggestion("Add WHERE clause to limit updates".to_string())
            );
        }

        Ok(())
    }

    /// Validate DELETE statement
    fn validate_delete(&self, delete: &DeleteStatement, result: &mut ValidationResult) -> Result<()> {
        result.metadata.tables_referenced.push(delete.table.clone());

        // Warn about DELETE without WHERE
        if delete.where_clause.is_none() {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::SafetyWarning,
                    "DELETE without WHERE clause will delete all rows".to_string(),
                )
                .with_suggestion("Add WHERE clause to limit deletions".to_string())
            );
        }

        Ok(())
    }

    // Helper methods

    /// Check if this is a constant query (no table access needed)
    fn is_constant_query(&self, select: &SelectStatement) -> bool {
        select.from.is_none() &&
        select.select_list.iter().all(|proj| {
            match proj {
                SelectItem::Expression { expr, .. } => self.is_constant_expression(expr),
                SelectItem::Wildcard => false,
            }
        })
    }

    /// Check if expression is a constant
    fn is_constant_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Literal(_) => true,
            Expression::Function { name, args } => {
                // Some functions are always constant (like NOW(), PI(), etc.)
                match name.to_uppercase().as_str() {
                    "NOW" | "CURRENT_TIMESTAMP" | "PI" => args.is_empty(),
                    _ => args.iter().all(|arg| self.is_constant_expression(arg)),
                }
            }
            Expression::Binary { left, right, .. } => {
                self.is_constant_expression(left) && self.is_constant_expression(right)
            }
            Expression::Unary { expr, .. } => self.is_constant_expression(expr),
            _ => false,
        }
    }

    /// Check if SELECT has wildcard projection
    #[allow(dead_code)]
    fn has_wildcard_projection(&self, select: &SelectStatement) -> bool {
        select.select_list.iter().any(|proj| matches!(proj, SelectItem::Wildcard))
    }

    /// Find aggregate functions in projections
    fn find_aggregates_in_projections(&self, projections: &[SelectItem]) -> Vec<String> {
        let mut aggregates = Vec::new();
        
        for projection in projections {
            match projection {
                SelectItem::Expression { expr, .. } => {
                    aggregates.extend(self.find_aggregates_in_expression(Some(expr)));
                }
                SelectItem::Wildcard => {}
            }
        }
        
        aggregates
    }

    /// Find aggregate functions in expression
    fn find_aggregates_in_expression(&self, expr: Option<&Expression>) -> Vec<String> {
        let mut aggregates = Vec::new();
        
        if let Some(expression) = expr {
            self.collect_aggregates(expression, &mut aggregates);
        }
        
        aggregates
    }

    /// Recursively collect aggregate functions
    fn collect_aggregates(&self, expr: &Expression, aggregates: &mut Vec<String>) {
        match expr {
            Expression::Function { name, args } => {
                if self.is_aggregate_function(name) {
                    aggregates.push(name.clone());
                }
                // Also check arguments for nested aggregates
                for arg in args {
                    self.collect_aggregates(arg, aggregates);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_aggregates(left, aggregates);
                self.collect_aggregates(right, aggregates);
            }
            Expression::Unary { expr, .. } => {
                self.collect_aggregates(expr, aggregates);
            }
            _ => {}
        }
    }

    /// Check if function is an aggregate
    fn is_aggregate_function(&self, name: &str) -> bool {
        matches!(name.to_uppercase().as_str(),
            "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | 
            "STDDEV" | "VARIANCE" | "GROUP_CONCAT" | "STRING_AGG"
        )
    }

    /// Check if SELECT is fully aggregated
    fn is_fully_aggregated_select(&self, select: &SelectStatement) -> bool {
        select.select_list.iter().all(|proj| {
            match proj {
                SelectItem::Expression { expr, .. } => self.is_aggregate_or_constant(expr),
                SelectItem::Wildcard => false,
            }
        })
    }

    /// Check if expression is aggregate or constant
    fn is_aggregate_or_constant(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Literal(_) => true,
            Expression::Function { name, args } => {
                if self.is_aggregate_function(name) {
                    true
                } else if self.is_constant_function(name) {
                    args.iter().all(|arg| self.is_constant_expression(arg))
                } else {
                    false
                }
            }
            Expression::Binary { left, right, .. } => {
                self.is_aggregate_or_constant(left) && self.is_aggregate_or_constant(right)
            }
            Expression::Unary { expr, .. } => self.is_aggregate_or_constant(expr),
            _ => false,
        }
    }

    /// Check if function is constant (doesn't vary per row)
    fn is_constant_function(&self, name: &str) -> bool {
        matches!(name.to_uppercase().as_str(),
            "NOW" | "CURRENT_TIMESTAMP" | "PI" | "RAND" | "UUID"
        )
    }

    /// Validate no nested aggregates
    fn validate_no_nested_aggregates(&self, projections: &[SelectItem]) -> Result<()> {
        for projection in projections {
            if let SelectItem::Expression { expr, .. } = projection {
                self.check_nested_aggregates(expr, false)?;
            }
        }
        Ok(())
    }

    /// Recursively check for nested aggregates
    fn check_nested_aggregates(&self, expr: &Expression, in_aggregate: bool) -> Result<()> {
        match expr {
            Expression::Function { name, args } => {
                let is_agg = self.is_aggregate_function(name);
                if is_agg && in_aggregate {
                    return Err(HyperQLError::ValidationError {
                        message: format!("Nested aggregate function '{}' is not allowed", name),
                        field: None,
                    });
                }
                
                // Check arguments
                for arg in args {
                    self.check_nested_aggregates(arg, is_agg || in_aggregate)?;
                }
            }
            Expression::Binary { left, right, .. } => {
                self.check_nested_aggregates(left, in_aggregate)?;
                self.check_nested_aggregates(right, in_aggregate)?;
            }
            Expression::Unary { expr, .. } => {
                self.check_nested_aggregates(expr, in_aggregate)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Check if expression is in GROUP BY
    fn is_in_group_by(&self, expr: &Expression, group_by: &[Expression]) -> bool {
        group_by.iter().any(|gb_expr| self.expressions_equivalent(expr, gb_expr))
    }

    /// Compare expressions for equivalence (simplified)
    fn expressions_equivalent(&self, expr1: &Expression, expr2: &Expression) -> bool {
        match (expr1, expr2) {
            (Expression::Column(col1), Expression::Column(col2)) => {
                col1.name == col2.name && col1.table == col2.table
            }
            (Expression::Literal(lit1), Expression::Literal(lit2)) => lit1 == lit2,
            // Add more cases as needed
            _ => false,
        }
    }

    /// Validate ORDER BY expression
    fn validate_order_by_expression(&self, _expr: &Expression, _select: &SelectStatement) -> Result<()> {
        // For now, just check that it's not completely invalid
        // TODO: Add more specific ORDER BY validation
        Ok(())
    }


    /// Check if column is a system column
    fn is_system_column(&self, column: &str) -> bool {
        matches!(column.to_lowercase().as_str(),
            "id" | "created_at" | "updated_at" | "version" | "rowid"
        )
    }

    /// Collect table references for metadata
    fn collect_table_references(&self, from_clause: &FromClause, result: &mut ValidationResult) {
        match from_clause {
            FromClause::Table { name, .. } => {
                result.metadata.tables_referenced.push(name.clone());
            }
            FromClause::Subquery { .. } => {
                // TODO: Handle subquery table references
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_aggregation_validation() {
        let config = ValidationConfig::default();
        let validator = SemanticValidator::new(&config);
        let mut result = ValidationResult::new();

        // Test HAVING without GROUP BY
        let select = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "name".to_string(),
                }),
                alias: None,
            }],
            from: Some(FromClause::Table {
                name: "users".to_string(),
                alias: None,
            }),
            where_clause: None,
            group_by: vec![], // No GROUP BY
            having: Some(Expression::Function {
                name: "COUNT".to_string(),
                args: vec![Expression::Literal(Literal::Int(1))],
            }),
            order_by: vec![],
            limit: Some(100),
            offset: None,
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select(&select, &mut result).unwrap();
        
        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| matches!(e.kind, ValidationErrorKind::AggregationError)));
    }

    #[test]
    fn test_constant_query_detection() {
        let config = ValidationConfig::default();
        let validator = SemanticValidator::new(&config);

        let select = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: Expression::Literal(Literal::String("Hello".to_string())),
                alias: None,
            }],
            from: None,
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
            traverse_clause: None,
        };

        assert!(validator.is_constant_query(&select));
    }

    #[test]
    fn test_update_without_where_warning() {
        let config = ValidationConfig::default();
        let validator = SemanticValidator::new(&config);
        let mut result = ValidationResult::new();

        let update = UpdateStatement {
            table: "users".to_string(),
            assignments: vec![Assignment {
                column: "name".to_string(),
                value: Expression::Literal(Literal::String("John".to_string())),
            }],
            where_clause: None,
        };

        validator.validate_update(&update, &mut result).unwrap();
        
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| matches!(w.kind, ValidationWarningKind::SafetyWarning)));
    }

    #[test]
    fn test_limit_offset_validation() {
        let config = ValidationConfig::default();
        let validator = SemanticValidator::new(&config);
        let mut result = ValidationResult::new();

        let select = SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: Some(FromClause::Table {
                name: "users".to_string(),
                alias: None,
            }),
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: Some(10), // OFFSET without LIMIT
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select(&select, &mut result).unwrap();
        
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| matches!(w.kind, ValidationWarningKind::UnusualPattern)));
    }
}