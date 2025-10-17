use crate::ast::*;
use crate::error::*;
use super::{expression, clause, traverse, utils};

pub fn parse_select_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    let mut parts = utils::split_query_parts(&upper_input, input);

    let (select_list, distinct) = if let Some(select_part) = parts.remove("SELECT") {
        let trimmed = select_part.trim();
        let upper_trimmed = trimmed.to_uppercase();

        if upper_trimmed.starts_with("DISTINCT ") {
            let list_part = &trimmed[9..];
            (parse_select_list(list_part)?, true)
        } else {
            (parse_select_list(trimmed)?, false)
        }
    } else {
        return Err(HyperQLError::simple_parse_error(
            "Missing SELECT clause",
            input,
            1,
            1,
        ));
    };

    let (from, joins) = if let Some(from_part) = parts.remove("FROM") {
        parse_from_and_joins(&from_part)?
    } else {
        (None, vec![])
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

    let offset = if let Some(offset_part) = parts.remove("OFFSET") {
        offset_part.trim().parse::<u64>().ok()
    } else {
        None
    };

    Ok(Statement::Select(SelectStatement {
        select_list,
        from,
        joins,
        traverse_clause,
        where_clause,
        group_by,
        having,
        order_by,
        limit,
        offset,
        distinct,
    }))
}

fn parse_from_and_joins(from_part: &str) -> Result<(Option<FromClause>, Vec<JoinClause>)> {
    let upper_from = from_part.to_uppercase();

    // Find all JOIN keywords - order matters! Longest patterns first to avoid matching substrings
    let join_keywords = ["FULL OUTER JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN", "JOIN"];
    let mut join_positions: Vec<(usize, &str)> = Vec::new();

    for keyword in &join_keywords {
        let mut search_pos = 0;
        while let Some(pos) = upper_from[search_pos..].find(keyword) {
            let absolute_pos = search_pos + pos;
            // Only add if it's a word boundary (not part of another word)
            if (absolute_pos == 0 || !upper_from.chars().nth(absolute_pos - 1).unwrap_or(' ').is_alphanumeric())
                && (absolute_pos + keyword.len() >= upper_from.len()
                    || !upper_from.chars().nth(absolute_pos + keyword.len()).unwrap_or(' ').is_alphanumeric()) {
                join_positions.push((absolute_pos, *keyword));
            }
            search_pos = absolute_pos + keyword.len();
        }
    }

    // Sort by position
    join_positions.sort_by_key(|(pos, _)| *pos);

    // Deduplicate - remove overlapping matches (prefer the first/longest match at each position)
    let mut filtered_joins = Vec::new();
    let mut last_end = 0;
    for (pos, keyword) in join_positions {
        if pos >= last_end {
            filtered_joins.push((pos, keyword));
            last_end = pos + keyword.len();
        }
    }
    join_positions = filtered_joins;

    // Parse FROM table (everything before first JOIN)
    let from_end = if join_positions.is_empty() {
        from_part.len()
    } else {
        join_positions[0].0
    };

    let from_text = from_part[..from_end].trim();
    let from = if !from_text.is_empty() {
        let parts: Vec<&str> = from_text.split_whitespace().collect();
        let table_ref = parts[0];

        // Parse collection or collection.type format
        let table_parts: Vec<&str> = table_ref.split('.').collect();
        let (collection, entity_type) = match table_parts.len() {
            1 => {
                // Single part: collection only
                (table_parts[0].to_string(), None)
            }
            2 => {
                // Two parts: collection.type
                (table_parts[0].to_string(), Some(table_parts[1].to_string()))
            }
            _ => {
                return Err(HyperQLError::simple_parse_error(
                    "Table name must be in format 'collection' or 'collection.type' (e.g., 'mimic' or 'mimic.Patient')",
                    from_part,
                    1,
                    1,
                ));
            }
        };

        let alias = if parts.len() > 1 && parts[1].to_uppercase() != "JOIN" {
            Some(parts[1].to_string())
        } else {
            None
        };
        Some(FromClause::Table {
            collection,
            entity_type,
            alias,
        })
    } else {
        None
    };

    // Parse each JOIN clause
    let mut joins = Vec::new();
    for i in 0..join_positions.len() {
        let (start_pos, join_keyword) = join_positions[i];
        let join_start = start_pos + join_keyword.len();
        let join_end = if i + 1 < join_positions.len() {
            join_positions[i + 1].0
        } else {
            from_part.len()
        };

        let join_text = from_part[join_start..join_end].trim();
        let join_type = match join_keyword {
            "INNER JOIN" => JoinType::Inner,
            "LEFT JOIN" => JoinType::Left,
            "RIGHT JOIN" => JoinType::Right,
            "FULL OUTER JOIN" => JoinType::FullOuter,
            "JOIN" => JoinType::Inner, // Default JOIN is INNER JOIN
            _ => return Err(HyperQLError::simple_parse_error(
                &format!("Unsupported JOIN type: {}", join_keyword),
                from_part,
                1,
                1,
            )),
        };

        // Find ON keyword
        let upper_join_text = join_text.to_uppercase();
        let on_pos = upper_join_text.find(" ON ")
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "JOIN clause missing ON condition",
                join_text,
                1,
                1,
            ))?;

        // Parse table name and alias (before ON)
        let table_part = join_text[..on_pos].trim();
        let parts: Vec<&str> = table_part.split_whitespace().collect();
        if parts.is_empty() {
            return Err(HyperQLError::simple_parse_error(
                "JOIN clause missing table name",
                join_text,
                1,
                1,
            ));
        }

        let table_ref = parts[0];

        // Parse collection or collection.type format
        let table_parts: Vec<&str> = table_ref.split('.').collect();
        let (collection, entity_type) = match table_parts.len() {
            1 => {
                // Single part: collection only
                (table_parts[0].to_string(), None)
            }
            2 => {
                // Two parts: collection.type
                (table_parts[0].to_string(), Some(table_parts[1].to_string()))
            }
            _ => {
                return Err(HyperQLError::simple_parse_error(
                    "JOIN table name must be in format 'collection' or 'collection.type' (e.g., 'mimic' or 'mimic.Patient')",
                    join_text,
                    1,
                    1,
                ));
            }
        };

        let alias = if parts.len() > 1 {
            Some(parts[1].to_string())
        } else {
            None
        };

        // Parse ON condition (after ON)
        let on_text = join_text[on_pos + 4..].trim();
        let on_condition = expression::parse_simple_expression(on_text)?;

        joins.push(JoinClause {
            join_type,
            collection,
            entity_type,
            alias,
            on_condition,
        });
    }

    Ok((from, joins))
}

fn parse_select_list(input: &str) -> Result<Vec<SelectItem>> {
    let input = input.trim();

    if input == "*" {
        return Ok(vec![SelectItem::Wildcard]);
    }

    let mut items = Vec::new();
    // Split by comma, but respect parentheses and quotes
    for part in split_select_items(input) {
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

/// Split SELECT list items by comma, respecting parentheses and quotes
fn split_select_items(input: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut current_start = 0;
    let mut paren_depth = 0;
    let mut in_quote = false;
    let mut quote_char = '"';
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        match ch {
            '\'' | '"' if i == 0 || chars[i - 1] != '\\' => {
                if !in_quote {
                    in_quote = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quote = false;
                }
            }
            '(' if !in_quote => paren_depth += 1,
            ')' if !in_quote => paren_depth -= 1,
            ',' if !in_quote && paren_depth == 0 => {
                items.push(&input[current_start..i]);
                current_start = i + 1;
            }
            _ => {}
        }
    }

    // Add final item
    if current_start < input.len() {
        items.push(&input[current_start..]);
    }

    items
}
