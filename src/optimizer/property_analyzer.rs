//! Property dependency analyzer for HyperQL query optimization.
//!
//! This module extracts all property names referenced in a SELECT statement,
//! enabling selective property fetching instead of loading all properties.

use crate::ast::{Expression, OrderByItem, SelectItem, SelectStatement};
use std::collections::HashSet;

/// Analyzes HyperQL queries to extract required property names.
///
/// This enables predicate pushdown optimization where only referenced
/// properties are fetched from storage, reducing data transfer and
/// improving query performance.
///
/// # Example
///
/// ```ignore
/// use hyperql::optimizer::PropertyAnalyzer;
///
/// // SELECT name, age WHERE status = 'active' ORDER BY created_at
/// let required = PropertyAnalyzer::analyze_required_properties(&stmt);
/// // Returns: {"name", "age", "status", "created_at"}
/// ```
pub struct PropertyAnalyzer;

impl PropertyAnalyzer {
    /// Extract all property names referenced in a SELECT statement.
    ///
    /// Analyzes all clauses that reference properties:
    /// - SELECT list (columns to return)
    /// - WHERE clause (filter predicates)
    /// - GROUP BY expressions
    /// - HAVING clause (grouped filters)
    /// - ORDER BY expressions
    ///
    /// # Arguments
    ///
    /// * `stmt` - The SELECT statement to analyze
    ///
    /// # Returns
    ///
    /// HashSet of property names. Empty if SELECT * is used (fetch all).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Simple select
    /// let props = PropertyAnalyzer::analyze_required_properties(&stmt);
    /// // SELECT name, age -> {"name", "age"}
    ///
    /// // With WHERE clause
    /// // SELECT name WHERE status = 'active' -> {"name", "status"}
    ///
    /// // Complex expressions
    /// // SELECT name WHERE (age > 18 AND status = 'active') OR priority > 5
    /// // -> {"name", "age", "status", "priority"}
    /// ```
    pub fn analyze_required_properties(stmt: &SelectStatement) -> HashSet<String> {
        let mut properties = HashSet::new();

        // Check for SELECT * (wildcard)
        // If present, return empty set to signal "fetch all properties"
        for item in &stmt.select_list {
            if matches!(item, SelectItem::Wildcard) {
                return HashSet::new();
            }
        }

        // Extract from SELECT list
        for item in &stmt.select_list {
            if let SelectItem::Expression { expr, .. } = item {
                Self::extract_from_expression(expr, &mut properties);
            }
        }

        // Extract from WHERE clause
        if let Some(ref where_expr) = stmt.where_clause {
            Self::extract_from_expression(where_expr, &mut properties);
        }

        // Extract from GROUP BY
        for expr in &stmt.group_by {
            Self::extract_from_expression(expr, &mut properties);
        }

        // Extract from HAVING clause
        if let Some(ref having_expr) = stmt.having {
            Self::extract_from_expression(having_expr, &mut properties);
        }

        // Extract from ORDER BY
        for order_item in &stmt.order_by {
            Self::extract_from_expression(&order_item.expr, &mut properties);
        }

        properties
    }

    /// Extract property names from a single expression (recursive).
    ///
    /// Walks the expression tree and collects all Column references.
    ///
    /// # Arguments
    ///
    /// * `expr` - The expression to analyze
    /// * `props` - Mutable set to accumulate property names
    fn extract_from_expression(expr: &Expression, props: &mut HashSet<String>) {
        match expr {
            // Base case: Column reference
            Expression::Column(col_ref) => {
                props.insert(col_ref.name.clone());
            }

            // Recursive cases: Binary operations
            Expression::Binary { left, right, .. } => {
                Self::extract_from_expression(left, props);
                Self::extract_from_expression(right, props);
            }

            // Recursive case: Unary operations
            Expression::Unary { expr: inner, .. } => {
                Self::extract_from_expression(inner, props);
            }

            // Recursive case: BETWEEN
            Expression::Between {
                expr: inner,
                lower,
                upper,
                ..
            } => {
                Self::extract_from_expression(inner, props);
                Self::extract_from_expression(lower, props);
                Self::extract_from_expression(upper, props);
            }

            // Recursive case: Function calls
            Expression::Function { args, .. } => {
                for arg in args {
                    Self::extract_from_expression(arg, props);
                }
            }

            // Recursive case: Geometric expressions
            Expression::Geometric(geom_expr) => {
                use crate::ast::geometric::GeometricExpression;
                match geom_expr {
                    GeometricExpression::Within {
                        target, reference, ..
                    } => {
                        Self::extract_from_expression(target, props);
                        Self::extract_from_expression(reference, props);
                    }
                    GeometricExpression::Near {
                        target, reference, ..
                    } => {
                        Self::extract_from_expression(target, props);
                        Self::extract_from_expression(reference, props);
                    }
                    GeometricExpression::InRadius { target, center, .. } => {
                        Self::extract_from_expression(target, props);
                        Self::extract_from_expression(center, props);
                    }
                }
            }

            // Recursive case: Vector expressions
            Expression::Vector(vec_expr) => {
                use crate::ast::VectorExpression;
                match vec_expr {
                    VectorExpression::Similarity { reference, .. } => {
                        Self::extract_from_expression(reference, props);
                    }
                    VectorExpression::KNN { reference, .. } => {
                        Self::extract_from_expression(reference, props);
                    }
                }
            }

            // Base case: Literals don't reference properties
            Expression::Literal(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        BinaryOperator, ColumnRef, Expression, Literal, OrderByItem, OrderDirection, SelectItem,
        SelectStatement,
    };

    fn make_column(name: &str) -> Expression {
        Expression::Column(ColumnRef {
            name: name.to_string(),
            table: None,
        })
    }

    fn make_literal(value: i64) -> Expression {
        Expression::Literal(Literal::Int(value))
    }

    #[test]
    fn test_simple_select() {
        let stmt = SelectStatement {
            select_list: vec![
                SelectItem::Expression {
                    expr: make_column("name"),
                    alias: None,
                },
                SelectItem::Expression {
                    expr: make_column("age"),
                    alias: None,
                },
            ],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 2);
        assert!(props.contains("name"));
        assert!(props.contains("age"));
    }

    #[test]
    fn test_wildcard_returns_empty() {
        let stmt = SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 0); // Empty = fetch all
    }

    #[test]
    fn test_where_clause() {
        let stmt = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: make_column("name"),
                alias: None,
            }],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: Some(Expression::Binary {
                left: Box::new(make_column("status")),
                op: BinaryOperator::Equal,
                right: Box::new(Expression::Literal(Literal::String("active".to_string()))),
            }),
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 2);
        assert!(props.contains("name"));
        assert!(props.contains("status"));
    }

    #[test]
    fn test_complex_where_clause() {
        // WHERE (age > 18 AND status = 'active') OR priority > 5
        let where_expr = Expression::Binary {
            left: Box::new(Expression::Binary {
                left: Box::new(Expression::Binary {
                    left: Box::new(make_column("age")),
                    op: BinaryOperator::GreaterThan,
                    right: Box::new(make_literal(18)),
                }),
                op: BinaryOperator::And,
                right: Box::new(Expression::Binary {
                    left: Box::new(make_column("status")),
                    op: BinaryOperator::Equal,
                    right: Box::new(Expression::Literal(Literal::String("active".to_string()))),
                }),
            }),
            op: BinaryOperator::Or,
            right: Box::new(Expression::Binary {
                left: Box::new(make_column("priority")),
                op: BinaryOperator::GreaterThan,
                right: Box::new(make_literal(5)),
            }),
        };

        let stmt = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: make_column("name"),
                alias: None,
            }],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: Some(where_expr),
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 4);
        assert!(props.contains("name"));
        assert!(props.contains("age"));
        assert!(props.contains("status"));
        assert!(props.contains("priority"));
    }

    #[test]
    fn test_order_by() {
        let stmt = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: make_column("name"),
                alias: None,
            }],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![OrderByItem {
                expr: make_column("created_at"),
                direction: OrderDirection::Desc,
            }],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 2);
        assert!(props.contains("name"));
        assert!(props.contains("created_at"));
    }

    #[test]
    fn test_group_by_and_having() {
        let stmt = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: make_column("category"),
                alias: None,
            }],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: None,
            group_by: vec![make_column("category")],
            having: Some(Expression::Binary {
                left: Box::new(make_column("count")),
                op: BinaryOperator::GreaterThan,
                right: Box::new(make_literal(10)),
            }),
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 2);
        assert!(props.contains("category"));
        assert!(props.contains("count"));
    }

    #[test]
    fn test_function_calls() {
        let stmt = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: Expression::Function {
                    name: "UPPER".to_string(),
                    args: vec![make_column("name")],
                },
                alias: None,
            }],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: Some(Expression::Function {
                name: "LENGTH".to_string(),
                args: vec![make_column("description")],
            }),
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 2);
        assert!(props.contains("name"));
        assert!(props.contains("description"));
    }

    #[test]
    fn test_between_expression() {
        let stmt = SelectStatement {
            select_list: vec![SelectItem::Expression {
                expr: make_column("name"),
                alias: None,
            }],
            from: None,
            joins: vec![],
            traverse_clause: None,
            where_clause: Some(Expression::Between {
                expr: Box::new(make_column("age")),
                lower: Box::new(make_literal(18)),
                upper: Box::new(make_literal(65)),
                negated: false,
            }),
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let props = PropertyAnalyzer::analyze_required_properties(&stmt);
        assert_eq!(props.len(), 2);
        assert!(props.contains("name"));
        assert!(props.contains("age"));
    }
}
