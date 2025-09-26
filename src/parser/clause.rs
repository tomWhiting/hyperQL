use crate::ast::*;
use crate::error::*;
use super::expression;

pub fn parse_group_by_list(input: &str) -> Result<Vec<Expression>> {
    let mut expressions = Vec::new();
    
    for expr_str in input.split(',') {
        let expr_str = expr_str.trim();
        if !expr_str.is_empty() {
            expressions.push(expression::parse_simple_column_or_literal(expr_str)?);
        }
    }
    
    Ok(expressions)
}

pub fn parse_order_by_item(input: &str) -> Result<OrderByItem> {
    let input = input.trim();
    let parts: Vec<&str> = input.split_whitespace().collect();

    let direction = if parts.len() > 1 {
        match parts[1].to_uppercase().as_str() {
            "DESC" => OrderDirection::Desc,
            "ASC" => OrderDirection::Asc,
            _ => OrderDirection::Asc,
        }
    } else {
        OrderDirection::Asc
    };

    Ok(OrderByItem {
        expr: expression::parse_simple_column_or_literal(parts[0])?,
        direction,
    })
}
