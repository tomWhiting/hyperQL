use crate::ast::*;
use crate::error::*;
use super::{expression, geometric, vector};

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
    let upper_input = input.to_uppercase();

    // Extract direction (DESC/ASC) from the end first
    let mut expr_part = input;
    let mut direction = OrderDirection::Asc;

    if upper_input.ends_with(" DESC") {
        expr_part = &input[..input.len() - 5].trim();
        direction = OrderDirection::Desc;
    } else if upper_input.ends_with(" ASC") {
        expr_part = &input[..input.len() - 4].trim();
        direction = OrderDirection::Asc;
    }

    let upper_expr = expr_part.to_uppercase();

    // Check for vector SIMILARITY function
    if upper_expr.starts_with("SIMILARITY(") {
        return Ok(OrderByItem {
            expr: vector::parse_similarity_function(expr_part)?,
            direction,
        });
    }

    // Check for vector DISTANCE function
    if upper_expr.starts_with("DISTANCE(") {
        // Check if it's a vector DISTANCE function (has 3 args with metric)
        // vs geometric DISTANCE (has FROM keyword)
        if expr_part.contains(',') && !upper_expr.contains(" FROM ") {
            return Ok(OrderByItem {
                expr: vector::parse_distance_function(expr_part)?,
                direction,
            });
        }
    }

    // Check for geometric DISTANCE expression (position DISTANCE FROM reference)
    if upper_expr.contains(" DISTANCE ") && upper_expr.contains(" FROM ") {
        return Ok(OrderByItem {
            expr: geometric::parse_distance_expression(expr_part)?,
            direction,
        });
    }

    // Regular ORDER BY parsing
    let parts: Vec<&str> = expr_part.split_whitespace().collect();

    if parts.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Empty ORDER BY expression",
            input,
            1,
            1,
        ));
    }

    Ok(OrderByItem {
        expr: expression::parse_simple_column_or_literal(parts[0])?,
        direction,
    })
}
