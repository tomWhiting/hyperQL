use crate::ast::*;
use crate::error::Result;
use crate::type_checker::{TypeChecker, TypeInfo};
use crate::validator::{
    ValidationConfig, ValidationResult, ValidationError, ValidationWarning,
    ValidationErrorKind, ValidationWarningKind,
};

/// Expression validator that performs semantic validation beyond type checking
pub struct ExpressionValidator {
    config: ValidationConfig,
}

impl ExpressionValidator {
    /// Create a new expression validator
    pub fn new(config: &ValidationConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: &ValidationConfig) {
        self.config = config.clone();
    }

    /// Validate expressions in a statement using type checker
    pub fn validate(
        &self,
        statement: &Statement,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        result.metadata.phases_completed.push("expression".to_string());
        
        match statement {
            Statement::Select(select) => self.validate_select_expressions(select, type_checker, result),
            Statement::Insert(insert) => self.validate_insert_expressions(insert, type_checker, result),
            Statement::Update(update) => self.validate_update_expressions(update, type_checker, result),
            Statement::Delete(delete) => self.validate_delete_expressions(delete, type_checker, result),
            Statement::Schema(_schema_op) => {
                // TODO: Implement schema expression validation
                // Validate expressions in calculated fields and cascade configurations
                Ok(())
            },
            Statement::Stream(_stream_op) => {
                // TODO: Implement stream expression validation
                // Validate expressions in stream filters and transformations
                Ok(())
            },
        }
    }

    /// Validate SELECT statement expressions
    fn validate_select_expressions(
        &self,
        select: &SelectStatement,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate projection expressions
        for projection in &select.select_list {
            if let SelectItem::Expression { expr, .. } = projection {
                self.validate_expression_semantics(expr, type_checker, result)?;
            }
        }

        // Validate WHERE clause
        if let Some(where_expr) = &select.where_clause {
            self.validate_where_expression(where_expr, type_checker, result)?;
        }

        // Validate GROUP BY expressions
        for group_expr in &select.group_by {
            self.validate_expression_semantics(group_expr, type_checker, result)?;
        }

        // Validate HAVING clause
        if let Some(having_expr) = &select.having {
            self.validate_having_expression(having_expr, type_checker, result)?;
        }

        // Validate ORDER BY expressions
        for order_item in &select.order_by {
            self.validate_expression_semantics(&order_item.expr, type_checker, result)?;
        }

        // Note: LIMIT/OFFSET are now u64 values, not expressions
        // TODO: If they become expressions again, add validation here

        Ok(())
    }

    /// Validate INSERT expressions
    fn validate_insert_expressions(
        &self,
        insert: &InsertStatement,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate value expressions (INSERT values are Vec<Vec<Expression>>)
        for value_row in &insert.values {
            for value_expr in value_row {
                self.validate_expression_semantics(value_expr, type_checker, result)?;
            }
        }

        Ok(())
    }

    /// Validate UPDATE expressions
    fn validate_update_expressions(
        &self,
        update: &UpdateStatement,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate assignment expressions
        for assignment in &update.assignments {
            self.validate_expression_semantics(&assignment.value, type_checker, result)?;
        }

        // Validate WHERE clause
        if let Some(where_expr) = &update.where_clause {
            self.validate_where_expression(where_expr, type_checker, result)?;
        }

        Ok(())
    }

    /// Validate DELETE expressions
    fn validate_delete_expressions(
        &self,
        delete: &DeleteStatement,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate WHERE clause
        if let Some(where_expr) = &delete.where_clause {
            self.validate_where_expression(where_expr, type_checker, result)?;
        }

        Ok(())
    }

    /// Validate general expression semantics
    fn validate_expression_semantics(
        &self,
        expr: &Expression,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // First perform type checking
        if let Err(type_error) = type_checker.check_expression_type(expr) {
            result.add_error(
                ValidationError::new(
                    ValidationErrorKind::ExpressionError,
                    format!("Type checking failed: {}", type_error),
                )
            );
            return Ok(()); // Continue with other validations
        }

        // Then perform semantic validation
        match expr {
            Expression::Function { name, args } => {
                self.validate_function_call(name, args, type_checker, result)?;
                // Record function usage
                result.metadata.functions_used.push(name.clone());
            }
            Expression::Binary { left, op, right } => {
                self.validate_binary_expression(left, op, right, type_checker, result)?;
            }
            Expression::Unary { op, expr } => {
                self.validate_unary_expression(op, expr, type_checker, result)?;
            }
            Expression::Column(col_ref) => {
                self.validate_column_reference(col_ref, result);
                // Record column usage
                let column_name = match &col_ref.table {
                    Some(table) => format!("{}.{}", table, col_ref.name),
                    None => col_ref.name.clone(),
                };
                result.metadata.columns_referenced.push(column_name);
            }
            Expression::Literal(literal) => {
                self.validate_literal_value(literal, result)?;
            }
            Expression::Geometric(geo_expr) => {
                self.validate_geometric_expression(geo_expr, type_checker, result)?;
            }
            Expression::Vector(vec_expr) => {
                self.validate_vector_expression(vec_expr, type_checker, result)?;
            }
        }

        Ok(())
    }

    /// Validate WHERE clause expressions
    fn validate_where_expression(
        &self,
        expr: &Expression,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // WHERE must return boolean
        if let Ok(expr_type) = type_checker.check_expression_type(expr) {
            if !matches!(expr_type, TypeInfo::Bool | TypeInfo::Unknown) {
                result.add_error(
                    ValidationError::new(
                        ValidationErrorKind::ExpressionError,
                        "WHERE clause must evaluate to boolean".to_string(),
                    )
                    .with_suggestions(vec![
                        "Use comparison operators (=, <, >, etc.)".to_string(),
                        "Use logical operators (AND, OR, NOT)".to_string(),
                    ])
                );
            }
        }

        self.validate_expression_semantics(expr, type_checker, result)?;
        
        // Check for common WHERE clause anti-patterns
        self.check_where_antipatterns(expr, result);

        Ok(())
    }

    /// Validate HAVING clause expressions
    fn validate_having_expression(
        &self,
        expr: &Expression,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // HAVING must return boolean
        if let Ok(expr_type) = type_checker.check_expression_type(expr) {
            if !matches!(expr_type, TypeInfo::Bool | TypeInfo::Unknown) {
                result.add_error(
                    ValidationError::new(
                        ValidationErrorKind::ExpressionError,
                        "HAVING clause must evaluate to boolean".to_string(),
                    )
                );
            }
        }

        self.validate_expression_semantics(expr, type_checker, result)
    }


    /// Validate function call semantics
    fn validate_function_call(
        &self,
        name: &str,
        args: &[Expression],
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate argument count for known functions
        match name.to_uppercase().as_str() {
            // Aggregate functions
            "COUNT" => {
                if args.is_empty() || args.len() > 1 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            "COUNT() requires exactly one argument".to_string(),
                        )
                    );
                }
            }
            "SUM" | "AVG" | "MIN" | "MAX" => {
                if args.len() != 1 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            format!("{}() requires exactly one argument", name),
                        )
                    );
                }
            }
            // String functions
            "SUBSTRING" | "SUBSTR" => {
                if args.len() < 2 || args.len() > 3 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            "SUBSTRING() requires 2 or 3 arguments".to_string(),
                        )
                    );
                }
            }
            "LENGTH" | "UPPER" | "LOWER" | "TRIM" => {
                if args.len() != 1 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            format!("{}() requires exactly one argument", name),
                        )
                    );
                }
            }
            // Math functions
            "ABS" | "SQRT" | "FLOOR" | "CEIL" => {
                if args.len() != 1 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            format!("{}() requires exactly one argument", name),
                        )
                    );
                }
            }
            "POWER" | "MOD" => {
                if args.len() != 2 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            format!("{}() requires exactly two arguments", name),
                        )
                    );
                }
            }
            // Date functions
            "NOW" | "CURRENT_TIMESTAMP" => {
                if !args.is_empty() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::FunctionError,
                            format!("{}() takes no arguments", name),
                        )
                    );
                }
            }
            _ => {
                // Unknown function - warn but don't error
                result.add_warning(
                    ValidationWarning::new(
                        ValidationWarningKind::UnusualPattern,
                        format!("Unknown function '{}' - cannot validate arguments", name),
                    )
                );
            }
        }

        // Validate each argument
        for arg in args {
            self.validate_expression_semantics(arg, type_checker, result)?;
        }

        Ok(())
    }

    /// Validate binary expression semantics
    fn validate_binary_expression(
        &self,
        left: &Expression,
        op: &BinaryOperator,
        right: &Expression,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate sub-expressions
        self.validate_expression_semantics(left, type_checker, result)?;
        self.validate_expression_semantics(right, type_checker, result)?;

        // Check for division by zero
        if matches!(op, BinaryOperator::Divide) {
            match right {
                Expression::Literal(Literal::Int(0)) => {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::ExpressionError,
                            "Division by zero is not allowed".to_string(),
                        )
                    );
                }
                Expression::Literal(Literal::Float(f)) if *f == 0.0 => {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::ExpressionError,
                            "Division by zero is not allowed".to_string(),
                        )
                    );
                }
                _ => {}
            }
        }

        // Check for potentially problematic comparisons
        match op {
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.check_equality_comparison(left, right, result);
            }
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => {
                self.check_ordering_comparison(left, right, result);
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate unary expression semantics
    fn validate_unary_expression(
        &self,
        op: &UnaryOperator,
        expr: &Expression,
        type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        self.validate_expression_semantics(expr, type_checker, result)?;

        // Validate operator usage
        match op {
            UnaryOperator::Not => {
                // NOT should be applied to boolean expressions
                if let Ok(expr_type) = type_checker.check_expression_type(expr) {
                    if !matches!(expr_type, TypeInfo::Bool | TypeInfo::Unknown) {
                        result.add_warning(
                            ValidationWarning::new(
                                ValidationWarningKind::UnusualPattern,
                                "NOT operator applied to non-boolean expression".to_string(),
                            )
                        );
                    }
                }
            }
            UnaryOperator::Plus => {
                // Plus should be applied to numeric expressions
                if let Ok(expr_type) = type_checker.check_expression_type(expr) {
                    if !expr_type.is_numeric() && !matches!(expr_type, TypeInfo::Unknown) {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::ExpressionError,
                                "Plus operator can only be applied to numeric expressions".to_string(),
                            )
                        );
                    }
                }
            }
            UnaryOperator::Minus => {
                // Minus should be applied to numeric expressions
                if let Ok(expr_type) = type_checker.check_expression_type(expr) {
                    if !expr_type.is_numeric() && !matches!(expr_type, TypeInfo::Unknown) {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::ExpressionError,
                                "Minus operator can only be applied to numeric expressions".to_string(),
                            )
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate column reference
    fn validate_column_reference(&self, col_ref: &ColumnRef, result: &mut ValidationResult) {
        // Check for potentially problematic column names
        let col_name = &col_ref.name;
        
        if col_name.contains(' ') {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    format!("Column name '{}' contains spaces", col_name),
                )
                .with_suggestion("Consider using quoted identifiers for names with spaces".to_string())
            );
        }

        if col_name.to_uppercase() == *col_name && col_name.len() > 1 {
            result.add_warning(
                ValidationWarning::new(
                    ValidationWarningKind::UnusualPattern,
                    format!("Column name '{}' is all uppercase", col_name),
                )
                .with_suggestion("Consider using lowercase or mixed case for better readability".to_string())
            );
        }
    }

    /// Validate literal values
    fn validate_literal_value(&self, literal: &Literal, result: &mut ValidationResult) -> Result<()> {
        match literal {
            Literal::Float(f) => {
                if f.is_infinite() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::ExpressionError,
                            "Infinite floating point values are not allowed".to_string(),
                        )
                    );
                }
                if f.is_nan() {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::ExpressionError,
                            "NaN floating point values are not allowed".to_string(),
                        )
                    );
                }
            }
            Literal::String(s) => {
                if s.len() > 1000000 { // 1MB limit
                    result.add_warning(
                        ValidationWarning::new(
                            ValidationWarningKind::PerformanceWarning,
                            "Very large string literal may impact performance".to_string(),
                        )
                        .with_suggestion("Consider using parameters for large strings".to_string())
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate geometric expressions
    fn validate_geometric_expression(
        &self,
        _geo_expr: &geometric::GeometricExpression,
        _type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // TODO: Add specific geometric expression validation
        // For now, just validate as generic expressions
        
        result.metadata.phases_completed.push("geometric_expression".to_string());
        Ok(())
    }

    /// Validate vector expressions
    fn validate_vector_expression(
        &self,
        vec_expr: &VectorExpression,
        _type_checker: &mut TypeChecker,
        result: &mut ValidationResult,
    ) -> Result<()> {
        match vec_expr {
            VectorExpression::Similarity { threshold, .. } => {
                if let Some(thresh) = threshold {
                    if *thresh < 0.0 || *thresh > 1.0 {
                        result.add_error(
                            ValidationError::new(
                                ValidationErrorKind::VectorError,
                                "Similarity threshold must be between 0.0 and 1.0".to_string(),
                            )
                        );
                    }
                }
            }
            VectorExpression::KNN { k, .. } => {
                if *k <= 0 {
                    result.add_error(
                        ValidationError::new(
                            ValidationErrorKind::VectorError,
                            "K in K-NN must be positive".to_string(),
                        )
                    );
                }
                if *k > 1000 {
                    result.add_warning(
                        ValidationWarning::new(
                            ValidationWarningKind::PerformanceWarning,
                            "Large K values may impact performance".to_string(),
                        )
                        .with_suggestion("Consider using smaller K for better performance".to_string())
                    );
                }
            }
        }

        result.metadata.phases_completed.push("vector_expression".to_string());
        Ok(())
    }

    /// Check WHERE clause anti-patterns
    fn check_where_antipatterns(&self, expr: &Expression, result: &mut ValidationResult) {
        match expr {
            Expression::Binary { left, op, right } => {
                // Check for 1=1 or 1=0 patterns
                if matches!(op, BinaryOperator::Equal) {
                    if let (Expression::Literal(Literal::Int(1)), Expression::Literal(Literal::Int(1))) = (left.as_ref(), right.as_ref()) {
                        result.add_warning(
                            ValidationWarning::new(
                                ValidationWarningKind::UnusualPattern,
                                "WHERE 1=1 is always true - consider removing".to_string(),
                            )
                        );
                    }
                    if let (Expression::Literal(Literal::Int(1)), Expression::Literal(Literal::Int(0))) = (left.as_ref(), right.as_ref()) {
                        result.add_warning(
                            ValidationWarning::new(
                                ValidationWarningKind::UnusualPattern,
                                "WHERE 1=0 is always false - query will return no results".to_string(),
                            )
                        );
                    }
                }

                // Recursively check sub-expressions
                self.check_where_antipatterns(left, result);
                self.check_where_antipatterns(right, result);
            }
            Expression::Unary { expr, .. } => {
                self.check_where_antipatterns(expr, result);
            }
            _ => {}
        }
    }

    /// Check equality comparisons
    fn check_equality_comparison(&self, left: &Expression, right: &Expression, result: &mut ValidationResult) {
        // Warn about comparing different literal types
        match (left, right) {
            (Expression::Literal(Literal::String(_)), Expression::Literal(Literal::Int(_))) |
            (Expression::Literal(Literal::Int(_)), Expression::Literal(Literal::String(_))) => {
                result.add_warning(
                    ValidationWarning::new(
                        ValidationWarningKind::UnusualPattern,
                        "Comparing string and integer literals - may not work as expected".to_string(),
                    )
                );
            }
            _ => {}
        }
    }

    /// Check ordering comparisons
    fn check_ordering_comparison(&self, left: &Expression, right: &Expression, result: &mut ValidationResult) {
        // Similar checks for ordering comparisons
        match (left, right) {
            (Expression::Literal(Literal::String(_)), Expression::Literal(Literal::Int(_))) |
            (Expression::Literal(Literal::Int(_)), Expression::Literal(Literal::String(_))) => {
                result.add_warning(
                    ValidationWarning::new(
                        ValidationWarningKind::UnusualPattern,
                        "Ordering comparison between string and integer - may not work as expected".to_string(),
                    )
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use crate::type_checker::TypeChecker;

    #[test]
    fn test_division_by_zero_detection() {
        let config = ValidationConfig::default();
        let validator = ExpressionValidator::new(&config);
        let mut type_checker = TypeChecker::new();
        let mut result = ValidationResult::new();

        let expr = Expression::Binary {
            left: Box::new(Expression::Literal(Literal::Int(10))),
            op: BinaryOperator::Divide,
            right: Box::new(Expression::Literal(Literal::Int(0))),
        };

        validator.validate_expression_semantics(&expr, &mut type_checker, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Division by zero")));
    }

    #[test]
    fn test_function_argument_validation() {
        let config = ValidationConfig::default();
        let validator = ExpressionValidator::new(&config);
        let mut type_checker = TypeChecker::new();
        let mut result = ValidationResult::new();

        let expr = Expression::Function {
            name: "COUNT".to_string(),
            args: vec![], // COUNT requires an argument
        };

        validator.validate_expression_semantics(&expr, &mut type_checker, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("COUNT() requires")));
    }


    #[test]
    fn test_vector_validation() {
        let config = ValidationConfig::default();
        let validator = ExpressionValidator::new(&config);
        let mut type_checker = TypeChecker::new();
        let mut result = ValidationResult::new();

        let vec_expr = VectorExpression::Similarity {
            vector_name: "embeddings".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("test".to_string()))),
            metric: crate::ast::vector::similarity::SimilarityMetric::Cosine,
            threshold: Some(1.5), // Invalid threshold > 1.0
            vector_type: crate::ast::vector::similarity::VectorType::Dense { dimensions: 768 },
        };

        validator.validate_vector_expression(&vec_expr, &mut type_checker, &mut result).unwrap();

        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("Similarity threshold")));
    }

    #[test]
    fn test_where_antipattern_detection() {
        let config = ValidationConfig::default();
        let validator = ExpressionValidator::new(&config);
        let mut result = ValidationResult::new();

        let expr = Expression::Binary {
            left: Box::new(Expression::Literal(Literal::Int(1))),
            op: BinaryOperator::Equal,
            right: Box::new(Expression::Literal(Literal::Int(1))),
        };

        validator.check_where_antipatterns(&expr, &mut result);

        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.message.contains("WHERE 1=1")));
    }
}