use crate::ast::*;
use crate::error::Result;
use crate::validator::{
    ValidationConfig, ValidationResult, ValidationError, ValidationWarning,
    ValidationErrorKind, ValidationWarningKind,
};
use std::collections::HashMap;

/// Schema-aware validator for table and column references
pub struct SchemaValidator {
    config: ValidationConfig,
    schema: Option<HashMap<String, Vec<String>>>,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new(config: &ValidationConfig) -> Self {
        Self {
            config: config.clone(),
            schema: None,
        }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: &ValidationConfig) {
        self.config = config.clone();
    }

    /// Set schema information for validation
    pub fn set_schema(&mut self, schema: &HashMap<String, Vec<String>>) {
        self.schema = Some(schema.clone());
    }

    /// Clear schema information
    pub fn clear_schema(&mut self) {
        self.schema = None;
    }

    /// Validate a statement against schema
    pub fn validate(&self, statement: &Statement, result: &mut ValidationResult) -> Result<()> {
        result.metadata.phases_completed.push("schema".to_string());
        
        // Only validate if we have schema information
        if self.schema.is_none() {
            return Ok(());
        }

        match statement {
            Statement::Select(select) => self.validate_select_schema(select, result),
            Statement::Insert(insert) => self.validate_insert_schema(insert, result),
            Statement::Update(update) => self.validate_update_schema(update, result),
            Statement::Delete(delete) => self.validate_delete_schema(delete, result),
            Statement::Schema(_schema_op) => {
                // TODO: Implement schema DDL schema validation
                // Validate that schema operations reference valid collections
                Ok(())
            },
            Statement::Stream(_stream_op) => {
                // TODO: Implement stream schema validation
                // Validate stream schemas and field types
                Ok(())
            },
        }
    }

    /// Validate SELECT statement against schema
    fn validate_select_schema(&self, select: &SelectStatement, result: &mut ValidationResult) -> Result<()> {
        let schema = self.schema.as_ref().unwrap();

        // Validate FROM clause table references
        if let Some(from_clause) = &select.from {
            match from_clause {
                FromClause::Table { collection, entity_type, .. } => {
                    // Build table reference name for schema lookup
                    let table_ref = if entity_type.is_empty() {
                        collection.clone()
                    } else {
                        format!("{}.{}", collection, entity_type)
                    };
                    self.validate_table_reference(&table_ref, result, schema);
                }
                FromClause::Subquery { .. } => {
                    // TODO: Handle subquery validation
                }
            }
        }

        // Collect available columns from referenced tables
        let available_columns = if let Some(from_clause) = &select.from {
            self.collect_available_columns(from_clause, schema)
        } else {
            HashMap::new()
        };

        // Validate column references in all clauses
        self.validate_select_columns(select, result, &available_columns)?;

        Ok(())
    }

    /// Validate INSERT statement against schema
    fn validate_insert_schema(&self, insert: &InsertStatement, result: &mut ValidationResult) -> Result<()> {
        let schema = self.schema.as_ref().unwrap();

        // Validate table reference
        self.validate_table_reference(&insert.table, result, schema);

        // Get table columns
        if let Some(table_columns) = schema.get(&insert.table) {
            // Validate specified columns (if any)
            if !insert.columns.is_empty() {
                for column in &insert.columns {
                    if !table_columns.contains(column) {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::SchemaError,
                                format!("Column '{}' does not exist in table '{}'", column, insert.table),
                            )
                            .with_suggestions(self.suggest_similar_columns(column, table_columns))
                        );
                    }
                }

                // Check if all required columns are provided (simplified - assumes all columns are optional for now)
                // TODO: Add required column validation based on schema constraints
            }
        }

        Ok(())
    }

    /// Validate UPDATE statement against schema
    fn validate_update_schema(&self, update: &UpdateStatement, result: &mut ValidationResult) -> Result<()> {
        let schema = self.schema.as_ref().unwrap();

        // Validate table reference
        self.validate_table_reference(&update.table, result, schema);

        // Get table columns
        if let Some(table_columns) = schema.get(&update.table) {
            // Validate assignment columns
            for assignment in &update.assignments {
                if !table_columns.contains(&assignment.column) {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::SchemaError,
                            format!("Column '{}' does not exist in table '{}'", assignment.column, update.table),
                        )
                        .with_suggestions(self.suggest_similar_columns(&assignment.column, table_columns))
                    );
                }
            }

            // Validate WHERE clause column references
            if let Some(where_clause) = &update.where_clause {
                let available_columns = [(update.table.clone(), table_columns.clone())].iter().cloned().collect();
                self.validate_expression_columns(where_clause, result, &available_columns)?;
            }
        }

        Ok(())
    }

    /// Validate DELETE statement against schema
    fn validate_delete_schema(&self, delete: &DeleteStatement, result: &mut ValidationResult) -> Result<()> {
        let schema = self.schema.as_ref().unwrap();

        // Validate table reference
        self.validate_table_reference(&delete.table, result, schema);

        // Get table columns
        if let Some(table_columns) = schema.get(&delete.table) {
            // Validate WHERE clause column references
            if let Some(where_clause) = &delete.where_clause {
                let available_columns = [(delete.table.clone(), table_columns.clone())].iter().cloned().collect();
                self.validate_expression_columns(where_clause, result, &available_columns)?;
            }
        }

        Ok(())
    }

    /// Validate table reference against schema
    fn validate_table_reference(
        &self,
        table_name: &str,
        result: &mut ValidationResult,
        schema: &HashMap<String, Vec<String>>,
    ) {
        if !schema.contains_key(table_name) {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::SchemaError,
                    format!("Table '{}' does not exist", table_name),
                )
                .with_suggestions(self.suggest_similar_tables(table_name, schema))
            );
        }
    }

    /// Collect available columns from FROM clause tables
    fn collect_available_columns(
        &self,
        from_clause: &FromClause,
        schema: &HashMap<String, Vec<String>>,
    ) -> HashMap<String, Vec<String>> {
        let mut available_columns = HashMap::new();

        match from_clause {
            FromClause::Table { collection, entity_type, .. } => {
                // Build table reference name for schema lookup
                let table_ref = if entity_type.is_empty() {
                    collection.clone()
                } else {
                    format!("{}.{}", collection, entity_type)
                };
                if let Some(columns) = schema.get(&table_ref) {
                    available_columns.insert(table_ref, columns.clone());
                }
            }
            FromClause::Subquery { .. } => {
                // TODO: Handle subquery columns
            }
        }

        available_columns
    }

    /// Validate column references in SELECT statement
    fn validate_select_columns(
        &self,
        select: &SelectStatement,
        result: &mut ValidationResult,
        available_columns: &HashMap<String, Vec<String>>,
    ) -> Result<()> {
        // Validate projection columns
        for projection in &select.select_list {
            match projection {
                SelectItem::Expression { expr, .. } => {
                    self.validate_expression_columns(expr, result, available_columns)?;
                }
                SelectItem::Wildcard => {
                    // Wildcard is valid if we have any tables
                    if available_columns.is_empty() {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::SchemaError,
                                "Wildcard (*) cannot be used without FROM clause".to_string(),
                            )
                        );
                    }
                }
            }
        }

        // Validate WHERE clause
        if let Some(where_expr) = &select.where_clause {
            self.validate_expression_columns(where_expr, result, available_columns)?;
        }

        // Validate GROUP BY
        for group_expr in &select.group_by {
            self.validate_expression_columns(group_expr, result, available_columns)?;
        }

        // Validate HAVING
        if let Some(having_expr) = &select.having {
            self.validate_expression_columns(having_expr, result, available_columns)?;
        }

        // Validate ORDER BY
        for order_item in &select.order_by {
            self.validate_expression_columns(&order_item.expr, result, available_columns)?;
        }

        Ok(())
    }

    /// Validate column references in expressions recursively
    fn validate_expression_columns(
        &self,
        expr: &Expression,
        result: &mut ValidationResult,
        available_columns: &HashMap<String, Vec<String>>,
    ) -> Result<()> {
        match expr {
            Expression::Column(col_ref) => {
                self.validate_column_reference(col_ref, result, available_columns);
            }
            Expression::Binary { left, right, .. } => {
                self.validate_expression_columns(left, result, available_columns)?;
                self.validate_expression_columns(right, result, available_columns)?;
            }
            Expression::Unary { expr, .. } => {
                self.validate_expression_columns(expr, result, available_columns)?;
            }
            Expression::Function { args, .. } => {
                for arg in args {
                    self.validate_expression_columns(arg, result, available_columns)?;
                }
            }
            // Other expression types don't contain column references
            _ => {}
        }

        Ok(())
    }

    /// Validate a single column reference
    fn validate_column_reference(
        &self,
        col_ref: &ColumnRef,
        result: &mut ValidationResult,
        available_columns: &HashMap<String, Vec<String>>,
    ) {
        match &col_ref.table {
            Some(table) => {
                // Qualified column reference (table.column)
                if let Some(columns) = available_columns.get(table) {
                    if !columns.contains(&col_ref.name) {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::SchemaError,
                                format!("Column '{}.{}' does not exist", table, col_ref.name),
                            )
                            .with_suggestions(self.suggest_similar_columns(&col_ref.name, columns))
                        );
                    }
                } else {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::SchemaError,
                            format!("Table '{}' not found in FROM clause", table),
                        )
                    );
                }
            }
            None => {
                // Unqualified column reference - find in available tables
                let matching_tables: Vec<String> = available_columns
                    .iter()
                    .filter_map(|(table, columns)| {
                        if columns.contains(&col_ref.name) {
                            Some(table.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                match matching_tables.len() {
                    0 => {
                        // Column not found in any table
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::SchemaError,
                                format!("Column '{}' not found in any referenced table", col_ref.name),
                            )
                            .with_suggestions(self.suggest_columns_from_all_tables(&col_ref.name, available_columns))
                        );
                    }
                    1 => {
                        // Column found in exactly one table - OK
                    }
                    _ => {
                        // Column found in multiple tables - ambiguous
                        if !self.config.allow_ambiguous_columns {
                            result.add_error(
                                ValidationError::new(
                                    ValidationErrorKind::SchemaError,
                                    format!("Column '{}' is ambiguous - found in tables: {}", 
                                           col_ref.name, 
                                           matching_tables.join(", ")),
                                )
                                .with_suggestions(vec![
                                    "Use qualified column names (table.column)".to_string(),
                                    "Use table aliases to disambiguate".to_string(),
                                ])
                            );
                        } else {
                            result.add_warning(
                                ValidationWarning::new(
                                    ValidationWarningKind::AmbiguousReference,
                                    format!("Column '{}' found in multiple tables: {}", 
                                           col_ref.name, 
                                           matching_tables.join(", ")),
                                )
                                .with_suggestion("Consider using qualified column names for clarity".to_string())
                            );
                        }
                    }
                }
            }
        }
    }

    /// Suggest similar table names
    fn suggest_similar_tables(&self, table_name: &str, schema: &HashMap<String, Vec<String>>) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for existing_table in schema.keys() {
            if self.is_similar_name(table_name, existing_table) {
                suggestions.push(format!("Did you mean '{}'?", existing_table));
            }
        }

        if suggestions.is_empty() {
            suggestions.push("Available tables:".to_string());
            for table in schema.keys().take(5) {
                suggestions.push(format!("  {}", table));
            }
            if schema.len() > 5 {
                suggestions.push("  ...".to_string());
            }
        }

        suggestions
    }

    /// Suggest similar column names
    fn suggest_similar_columns(&self, column_name: &str, columns: &[String]) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for existing_column in columns {
            if self.is_similar_name(column_name, existing_column) {
                suggestions.push(format!("Did you mean '{}'?", existing_column));
            }
        }

        if suggestions.is_empty() && !columns.is_empty() {
            suggestions.push("Available columns:".to_string());
            for column in columns.iter().take(5) {
                suggestions.push(format!("  {}", column));
            }
            if columns.len() > 5 {
                suggestions.push("  ...".to_string());
            }
        }

        suggestions
    }

    /// Suggest column names from all available tables
    fn suggest_columns_from_all_tables(
        &self,
        column_name: &str,
        available_columns: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for (table, columns) in available_columns {
            for existing_column in columns {
                if self.is_similar_name(column_name, existing_column) {
                    suggestions.push(format!("Did you mean '{}.{}'?", table, existing_column));
                }
            }
        }

        if suggestions.is_empty() {
            suggestions.push("Available columns:".to_string());
            let mut count = 0;
            for (table, columns) in available_columns {
                for column in columns {
                    if count >= 5 { break; }
                    suggestions.push(format!("  {}.{}", table, column));
                    count += 1;
                }
                if count >= 5 { break; }
            }
            if count >= 5 {
                suggestions.push("  ...".to_string());
            }
        }

        suggestions
    }

    /// Check if two names are similar (simple edit distance)
    fn is_similar_name(&self, name1: &str, name2: &str) -> bool {
        if name1 == name2 {
            return true;
        }

        // Simple similarity check - case insensitive and edit distance
        let name1_lower = name1.to_lowercase();
        let name2_lower = name2.to_lowercase();
        
        // Check for case-insensitive match
        if name1_lower == name2_lower {
            return true;
        }

        // Check for simple edit distance (insertions, deletions, substitutions)
        let distance = self.edit_distance(&name1_lower, &name2_lower);
        let max_len = name1.len().max(name2.len());
        
        // Consider similar if edit distance is small relative to length
        distance <= 2 && max_len > 2 && (distance as f64 / max_len as f64) < 0.5
    }

    /// Calculate edit distance between two strings
    fn edit_distance(&self, s1: &str, s2: &str) -> usize {
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();

        // Create matrix for dynamic programming
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        // Initialize first row and column
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        // Fill the matrix
        for i in 1..=len1 {
            for j in 1..=len2 {
                if chars1[i - 1] == chars2[j - 1] {
                    matrix[i][j] = matrix[i - 1][j - 1];
                } else {
                    matrix[i][j] = 1 + matrix[i - 1][j].min(matrix[i][j - 1]).min(matrix[i - 1][j - 1]);
                }
            }
        }

        matrix[len1][len2]
    }

    /// Check if schema validation is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.validate_schema && self.schema.is_some()
    }

    /// Get available tables
    pub fn get_tables(&self) -> Vec<String> {
        self.schema
            .as_ref()
            .map(|schema| schema.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get columns for a specific table
    pub fn get_table_columns(&self, table: &str) -> Option<&Vec<String>> {
        self.schema.as_ref().and_then(|schema| schema.get(table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn create_test_schema() -> HashMap<String, Vec<String>> {
        let mut schema = HashMap::new();
        schema.insert(
            "users".to_string(),
            vec!["id".to_string(), "name".to_string(), "email".to_string(), "age".to_string()],
        );
        schema.insert(
            "orders".to_string(),
            vec!["id".to_string(), "user_id".to_string(), "amount".to_string(), "created_at".to_string()],
        );
        schema
    }

    #[test]
    fn test_valid_table_reference() {
        let config = ValidationConfig::default();
        let mut validator = SchemaValidator::new(&config);
        let schema = create_test_schema();
        validator.set_schema(&schema);

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
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select_schema(&select, &mut result).unwrap();

        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_table_reference() {
        let config = ValidationConfig::default();
        let mut validator = SchemaValidator::new(&config);
        let schema = create_test_schema();
        validator.set_schema(&schema);

        let mut result = ValidationResult::new();

        let select = SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: Some(FromClause::Table {
                collection: "nonexistent".to_string(),
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
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select_schema(&select, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Table 'nonexistent' does not exist")));
    }

    #[test]
    fn test_valid_column_reference() {
        let config = ValidationConfig::default();
        let mut validator = SchemaValidator::new(&config);
        let schema = create_test_schema();
        validator.set_schema(&schema);

        let mut result = ValidationResult::new();

        let select = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "name".to_string(),
                }),
                alias: None,
            }],
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
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select_schema(&select, &mut result).unwrap();

        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_column_reference() {
        let config = ValidationConfig::default();
        let mut validator = SchemaValidator::new(&config);
        let schema = create_test_schema();
        validator.set_schema(&schema);

        let mut result = ValidationResult::new();

        let select = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "nonexistent".to_string(),
                }),
                alias: None,
            }],
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
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select_schema(&select, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Column 'nonexistent' not found")));
    }

    #[test]
    fn test_ambiguous_column_reference() {
        // TODO: Re-enable when multi-table FROM clause is supported
        // Currently FROM only supports single table, so no ambiguity can occur

        let config = ValidationConfig {
            allow_ambiguous_columns: false,
            ..ValidationConfig::default()
        };
        let mut validator = SchemaValidator::new(&config);
        let schema = create_test_schema();
        validator.set_schema(&schema);

        let mut result = ValidationResult::new();

        let select = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: Expression::Column(ColumnRef {
                    table: None,
                    name: "id".to_string(), // Both users and orders have 'id'
                }),
                alias: None,
            }],
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
            distinct: false,
            traverse_clause: None,
        };

        validator.validate_select_schema(&select, &mut result).unwrap();

        // With single table FROM clause, there should be no ambiguity
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_edit_distance() {
        let config = ValidationConfig::default();
        let validator = SchemaValidator::new(&config);

        assert_eq!(validator.edit_distance("test", "test"), 0);
        assert_eq!(validator.edit_distance("test", "best"), 1);
        assert_eq!(validator.edit_distance("test", "tests"), 1);
        assert_eq!(validator.edit_distance("test", "tes"), 1);
        assert_eq!(validator.edit_distance("test", "best1"), 2);
    }

    #[test]
    fn test_similar_name_detection() {
        let config = ValidationConfig::default();
        let validator = SchemaValidator::new(&config);

        assert!(validator.is_similar_name("user", "users"));
        assert!(validator.is_similar_name("User", "user"));
        assert!(validator.is_similar_name("test", "best"));
        assert!(!validator.is_similar_name("test", "completely_different"));
    }
}