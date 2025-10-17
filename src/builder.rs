use crate::ast::{Statement, SelectStatement, SelectItem, FromClause, Expression, OrderByItem, OrderDirection, Literal, ColumnRef, BinaryOperator};
use crate::error::{HyperQLError, Result};
use crate::types::Value;

pub struct HyperQLBuilder {
    statement: Option<Statement>,
    errors: Vec<String>,
}

impl Default for HyperQLBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperQLBuilder {
    pub fn new() -> Self {
        Self {
            statement: None,
            errors: Vec::new(),
        }
    }

    pub fn select<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let select_list: Vec<SelectItem> = columns
            .into_iter()
            .map(|col| {
                let col_name = col.into();
                if col_name == "*" {
                    SelectItem::Wildcard
                } else {
                    SelectItem::Expression {
                        expr: Expression::Column(ColumnRef {
                            table: None,
                            name: col_name,
                        }),
                        alias: None,
                    }
                }
            })
            .collect();

        if select_list.is_empty() {
            self.errors.push("SELECT clause cannot be empty".to_string());
        }

        let select_stmt = SelectStatement {
            select_list,
            from: None,
            joins: Vec::new(),
            traverse_clause: None,
            where_clause: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        };

        self.statement = Some(Statement::Select(select_stmt));
        self
    }

    pub fn from(mut self, table: &str) -> Self {
        if table.is_empty() {
            self.errors.push("FROM table name cannot be empty".to_string());
            return self;
        }

        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                // Parse table string - support both "collection" and "collection.entity_type" formats
                let (collection, entity_type) = if let Some(dot_pos) = table.find('.') {
                    let (coll, et) = table.split_at(dot_pos);
                    (coll.to_string(), Some(et[1..].to_string()))
                } else {
                    (table.to_string(), None)
                };

                select_stmt.from = Some(FromClause::Table {
                    collection,
                    entity_type,
                    alias: None,
                });
            }
            _ => {
                self.errors.push("FROM clause can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn where_eq(mut self, column: &str, value: Value) -> Self {
        if column.is_empty() {
            self.errors.push("WHERE column name cannot be empty".to_string());
            return self;
        }

        let literal = match value {
            Value::Bool(b) => Literal::Bool(b),
            Value::Int(i) => Literal::Int(i),
            Value::Float(f) => Literal::Float(f),
            Value::String(s) => Literal::String(s),
            Value::EntityId(id) => Literal::EntityId(id),
            Value::Null => Literal::Null,
            _ => {
                self.errors.push(format!("Unsupported value type for WHERE clause: {:?}", value));
                return self;
            }
        };

        let where_expr = Expression::Binary {
            left: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: column.to_string(),
            })),
            op: BinaryOperator::Equal,
            right: Box::new(Expression::Literal(literal)),
        };

        self.add_where_condition(where_expr)
    }

    pub fn where_expr(self, expr: Expression) -> Self {
        self.add_where_condition(expr)
    }

    fn add_where_condition(mut self, new_expr: Expression) -> Self {
        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                match &select_stmt.where_clause {
                    Some(existing_expr) => {
                        select_stmt.where_clause = Some(Expression::Binary {
                            left: Box::new(existing_expr.clone()),
                            op: BinaryOperator::And,
                            right: Box::new(new_expr),
                        });
                    }
                    None => {
                        select_stmt.where_clause = Some(new_expr);
                    }
                }
            }
            _ => {
                self.errors.push("WHERE clause can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn group_by<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let group_exprs: Vec<Expression> = columns
            .into_iter()
            .map(|col| {
                Expression::Column(ColumnRef {
                    table: None,
                    name: col.into(),
                })
            })
            .collect();

        if group_exprs.is_empty() {
            self.errors.push("GROUP BY clause cannot be empty".to_string());
            return self;
        }

        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                select_stmt.group_by = group_exprs;
            }
            _ => {
                self.errors.push("GROUP BY clause can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        if column.is_empty() {
            self.errors.push("ORDER BY column name cannot be empty".to_string());
            return self;
        }

        let order_item = OrderByItem {
            expr: Expression::Column(ColumnRef {
                table: None,
                name: column.to_string(),
            }),
            direction,
        };

        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                select_stmt.order_by.push(order_item);
            }
            _ => {
                self.errors.push("ORDER BY clause can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        if limit == 0 {
            self.errors.push("LIMIT must be greater than 0".to_string());
            return self;
        }

        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                select_stmt.limit = Some(limit);
            }
            _ => {
                self.errors.push("LIMIT clause can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                select_stmt.offset = Some(offset);
            }
            _ => {
                self.errors.push("OFFSET clause can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn distinct(mut self) -> Self {
        match &mut self.statement {
            Some(Statement::Select(select_stmt)) => {
                select_stmt.distinct = true;
            }
            _ => {
                self.errors.push("DISTINCT can only be used with SELECT statements".to_string());
            }
        }
        self
    }

    pub fn build(self) -> Result<Statement> {
        if !self.errors.is_empty() {
            return Err(HyperQLError::BuilderError {
                errors: self.errors,
            });
        }

        match self.statement {
            Some(statement) => Ok(statement),
            None => Err(HyperQLError::BuilderError {
                errors: vec!["No statement was built".to_string()],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn test_basic_select_construction() {
        let statement = HyperQLBuilder::new()
            .select(["name", "age"])
            .from("users")
            .build()
            .expect("Should build successfully");

        if let Statement::Select(select_stmt) = statement {
            assert_eq!(select_stmt.select_list.len(), 2);
            assert!(matches!(select_stmt.from, Some(FromClause::Table { .. })));
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_where_clause_building() {
        let statement = HyperQLBuilder::new()
            .select(["*"])
            .from("entities")
            .where_eq("name", Value::String("Alice".to_string()))
            .build()
            .expect("Should build successfully");

        if let Statement::Select(select_stmt) = statement {
            assert!(select_stmt.where_clause.is_some());
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_method_chaining() {
        let statement = HyperQLBuilder::new()
            .select(["name", "age"])
            .from("users")
            .where_eq("age", Value::Int(21))
            .order_by("name", OrderDirection::Asc)
            .limit(10)
            .offset(5)
            .build()
            .expect("Should build successfully");

        if let Statement::Select(select_stmt) = statement {
            assert!(select_stmt.where_clause.is_some());
            assert_eq!(select_stmt.order_by.len(), 1);
            assert_eq!(select_stmt.limit, Some(10));
            assert_eq!(select_stmt.offset, Some(5));
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_error_conditions() {
        let result = HyperQLBuilder::new()
            .select(Vec::<String>::new())
            .build();

        assert!(result.is_err());
        if let Err(HyperQLError::BuilderError { errors }) = result {
            assert!(!errors.is_empty());
        }
    }

    #[test]
    fn test_multiple_where_conditions() {
        let statement = HyperQLBuilder::new()
            .select(["*"])
            .from("users")
            .where_eq("age", Value::Int(25))
            .where_eq("active", Value::Bool(true))
            .build()
            .expect("Should build successfully");

        if let Statement::Select(select_stmt) = statement {
            if let Some(Expression::Binary { op: BinaryOperator::And, .. }) = &select_stmt.where_clause {
                // Multiple conditions should be combined with AND
            } else {
                panic!("Expected AND operation for multiple WHERE conditions");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_group_by_clause() {
        let statement = HyperQLBuilder::new()
            .select(["department", "COUNT(*)"])
            .from("employees")
            .group_by(["department"])
            .build()
            .expect("Should build successfully");

        if let Statement::Select(select_stmt) = statement {
            assert_eq!(select_stmt.group_by.len(), 1);
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_distinct_flag() {
        let statement = HyperQLBuilder::new()
            .select(["category"])
            .from("products")
            .distinct()
            .build()
            .expect("Should build successfully");

        if let Statement::Select(select_stmt) = statement {
            assert!(select_stmt.distinct);
        } else {
            panic!("Expected SELECT statement");
        }
    }
}
