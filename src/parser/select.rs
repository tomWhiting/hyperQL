use crate::ast::*;
use crate::error::*;
use super::{expression, clause, traverse, utils};

pub fn parse_select_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    let mut parts = utils::split_query_parts(&upper_input, input);

    let select_list = if let Some(select_part) = parts.remove("SELECT") {
        parse_select_list(&select_part)?
    } else {
        return Err(HyperQLError::simple_parse_error(
            "Missing SELECT clause",
            input,
            1,
            1,
        ));
    };

    let from = if let Some(from_part) = parts.remove("FROM") {
        Some(FromClause::Table {
            name: from_part.split_whitespace().next().unwrap_or("").to_string(),
            alias: None,
        })
    } else {
        None
    };

    let traverse_clause = if let Some(traverse_part) = parts.remove("TRAVERSE") {
        Some(traverse::parse_traverse_clause(&traverse_part)?)
    } else {
        None
    };

    let where_clause = if let Some(where_part) = parts.remove("WHERE") {
        Some(expression::parse_simple_expression(&where_part)?)
    } else {
        None
    };

    let group_by = if let Some(group_part) = parts.remove("GROUP BY") {
        clause::parse_group_by_list(&group_part)?
    } else {
        vec![]
    };

    let having = if let Some(having_part) = parts.remove("HAVING") {
        Some(expression::parse_simple_expression(&having_part)?)
    } else {
        None
    };

    let order_by = if let Some(order_part) = parts.remove("ORDER BY") {
        vec![clause::parse_order_by_item(&order_part)?]
    } else {
        vec![]
    };

    let limit = if let Some(limit_part) = parts.remove("LIMIT") {
        limit_part.trim().parse::<u64>().ok()
    } else {
        None
    };

    Ok(Statement::Select(SelectStatement {
        select_list,
        from,
        traverse_clause,
        where_clause,
        group_by,
        having,
        order_by,
        limit,
        offset: None,
        distinct: false,
    }))
}

fn parse_select_list(input: &str) -> Result<Vec<SelectItem>> {
    let input = input.trim();

    if input == "*" {
        return Ok(vec![SelectItem::Wildcard]);
    }

    let mut items = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(as_pos) = part.to_uppercase().find(" AS ") {
            let expr_part = part[..as_pos].trim();
            let alias_part = part[as_pos + 4..].trim();
            items.push(SelectItem::Expression {
                expr: expression::parse_expression_or_function(expr_part)?,
                alias: Some(alias_part.to_string()),
            });
        } else if let Some(space_pos) = part.find(' ') {
            let expr_part = part[..space_pos].trim();
            let potential_alias = part[space_pos..].trim();
            if utils::is_valid_identifier(potential_alias) {
                items.push(SelectItem::Expression {
                    expr: expression::parse_expression_or_function(expr_part)?,
                    alias: Some(potential_alias.to_string()),
                });
            } else {
                items.push(SelectItem::Expression {
                    expr: expression::parse_expression_or_function(part)?,
                    alias: None,
                });
            }
        } else {
            items.push(SelectItem::Expression {
                expr: expression::parse_expression_or_function(part)?,
                alias: None,
            });
        }
    }

    Ok(items)
}
