//! # HyperQL Query Parser
//!
//! This module implements a comprehensive parser for the HyperQL query language.
//! It supports SELECT, INSERT, UPDATE, DELETE statements with expressions,
//! aggregate functions, GROUP BY, HAVING, ORDER BY, and LIMIT clauses.

use crate::ast::*;
use crate::error::*;

/// Parse a complete HyperQL statement
pub fn parse_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    if upper_input.starts_with("SELECT") {
        parse_select_statement(input)
    } else if upper_input.starts_with("INSERT") {
        parse_insert_statement(input)
    } else if upper_input.starts_with("UPDATE") {
        parse_update_statement(input)
    } else if upper_input.starts_with("DELETE") {
        parse_delete_statement(input)
    } else {
        Err(HyperQLError::ParseError {
            message: "Unsupported statement type. Supported: SELECT, INSERT, UPDATE, DELETE".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })
    }
}

/// Parse a SELECT statement
fn parse_select_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    // Split the query into major parts
    let mut parts = split_query_parts(&upper_input, input);

    // Parse SELECT list
    let select_list = if let Some(select_part) = parts.remove("SELECT") {
        parse_select_list(&select_part)?
    } else {
        return Err(HyperQLError::ParseError {
            message: "Missing SELECT clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        });
    };

    // Parse FROM clause
    let from = if let Some(from_part) = parts.remove("FROM") {
        Some(FromClause::Table {
            name: from_part.split_whitespace().next().unwrap_or("").to_string(),
            alias: None,
        })
    } else {
        None
    };

    // Parse TRAVERSE clause
    let traverse_clause = if let Some(traverse_part) = parts.remove("TRAVERSE") {
        Some(parse_traverse_clause(&traverse_part)?)
    } else {
        None
    };

    // Parse WHERE clause
    let where_clause = if let Some(where_part) = parts.remove("WHERE") {
        Some(parse_simple_expression(&where_part)?)
    } else {
        None
    };

    // Parse GROUP BY clause
    let group_by = if let Some(group_part) = parts.remove("GROUP BY") {
        parse_group_by_list(&group_part)?
    } else {
        vec![]
    };

    // Parse HAVING clause
    let having = if let Some(having_part) = parts.remove("HAVING") {
        Some(parse_simple_expression(&having_part)?)
    } else {
        None
    };

    // Parse ORDER BY clause
    let order_by = if let Some(order_part) = parts.remove("ORDER BY") {
        vec![parse_order_by_item(&order_part)?]
    } else {
        vec![]
    };

    // Parse LIMIT clause
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

/// Split query into parts based on keywords
fn split_query_parts(upper_input: &str, original_input: &str) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;

    let mut parts = HashMap::new();
    let keywords = ["SELECT", "FROM", "TRAVERSE", "WHERE", "GROUP BY", "HAVING", "ORDER BY", "LIMIT", "OFFSET"];

    // Find keyword positions
    let mut keyword_positions = Vec::new();
    for keyword in &keywords {
        if let Some(pos) = upper_input.find(keyword) {
            keyword_positions.push((pos, keyword));
        }
    }

    // Sort by position
    keyword_positions.sort_by_key(|(pos, _)| *pos);

    // Extract parts
    for i in 0..keyword_positions.len() {
        let (start_pos, keyword) = keyword_positions[i];
        let content_start = start_pos + keyword.len();
        let content_end = if i + 1 < keyword_positions.len() {
            keyword_positions[i + 1].0
        } else {
            original_input.len()
        };

        let content = original_input[content_start..content_end].trim();
        parts.insert(keyword.to_string(), content.to_string());
    }

    parts
}

/// Parse SELECT list (comma-separated items)
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

        // Check for alias (AS keyword or just space-separated)
        if let Some(as_pos) = part.to_uppercase().find(" AS ") {
            let expr_part = part[..as_pos].trim();
            let alias_part = part[as_pos + 4..].trim();
            items.push(SelectItem::Expression {
                expr: parse_expression_or_function(expr_part)?,
                alias: Some(alias_part.to_string()),
            });
        } else if let Some(space_pos) = part.find(' ') {
            // Check if it's just column with alias (no AS)
            let expr_part = part[..space_pos].trim();
            let potential_alias = part[space_pos..].trim();
            if is_valid_identifier(potential_alias) {
                items.push(SelectItem::Expression {
                    expr: parse_expression_or_function(expr_part)?,
                    alias: Some(potential_alias.to_string()),
                });
            } else {
                items.push(SelectItem::Expression {
                    expr: parse_expression_or_function(part)?,
                    alias: None,
                });
            }
        } else {
            // Check if it's an aggregate function or regular expression
            items.push(SelectItem::Expression {
                expr: parse_expression_or_function(part)?,
                alias: None,
            });
        }
    }

    Ok(items)
}

/// Parse a simple expression for WHERE clauses
fn parse_simple_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Handle AND/OR
    if let Some(and_pos) = find_operator_position(input, " AND ") {
        let left = parse_simple_expression(&input[..and_pos])?;
        let right = parse_simple_expression(&input[and_pos + 5..])?;
        return Ok(Expression::Binary {
            left: Box::new(left),
            op: BinaryOperator::And,
            right: Box::new(right),
        });
    }

    if let Some(or_pos) = find_operator_position(input, " OR ") {
        let left = parse_simple_expression(&input[..or_pos])?;
        let right = parse_simple_expression(&input[or_pos + 4..])?;
        return Ok(Expression::Binary {
            left: Box::new(left),
            op: BinaryOperator::Or,
            right: Box::new(right),
        });
    }

    // Handle comparison operators
    let operators = [
        ("<=", BinaryOperator::LessThanOrEqual),
        (">=", BinaryOperator::GreaterThanOrEqual),
        ("<>", BinaryOperator::NotEqual),
        ("!=", BinaryOperator::NotEqual),
        ("=", BinaryOperator::Equal),
        ("<", BinaryOperator::LessThan),
        (">", BinaryOperator::GreaterThan),
    ];

    for (op_str, op_type) in &operators {
        if let Some(op_pos) = find_operator_position(input, op_str) {
            let left = parse_simple_column_or_literal(&input[..op_pos])?;
            let right = parse_simple_column_or_literal(&input[op_pos + op_str.len()..])?;
            return Ok(Expression::Binary {
                left: Box::new(left),
                op: op_type.clone(),
                right: Box::new(right),
            });
        }
    }

    // If no operators found, treat as column or literal
    parse_simple_column_or_literal(input)
}

/// Find operator position avoiding quoted strings
fn find_operator_position(input: &str, operator: &str) -> Option<usize> {
    let mut in_quote = false;
    let mut quote_char = '"';
    let chars: Vec<char> = input.chars().collect();

    for i in 0..=chars.len().saturating_sub(operator.len()) {
        // Check if we're entering/leaving a quote
        if i < chars.len() {
            let ch = chars[i];
            if (ch == '"' || ch == '\'') && (i == 0 || chars[i - 1] != '\\') {
                if !in_quote {
                    in_quote = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quote = false;
                }
            }
        }

        // If not in a quote, check for operator
        if !in_quote {
            let substr: String = chars[i..i + operator.len()].iter().collect();
            if substr.to_uppercase() == operator.to_uppercase() {
                return Some(i);
            }
        }
    }

    None
}

/// Parse simple column reference or literal
fn parse_simple_column_or_literal(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Check for string literals
    if (input.starts_with('"') && input.ends_with('"')) ||
       (input.starts_with('\'') && input.ends_with('\'')) {
        let content = &input[1..input.len()-1];
        return Ok(Expression::Literal(Literal::String(content.to_string())));
    }

    // Check for numeric literals
    if let Ok(int_val) = input.parse::<i64>() {
        return Ok(Expression::Literal(Literal::Int(int_val)));
    }

    if let Ok(float_val) = input.parse::<f64>() {
        return Ok(Expression::Literal(Literal::Float(float_val)));
    }

    // Check for boolean literals
    match input.to_uppercase().as_str() {
        "TRUE" => return Ok(Expression::Literal(Literal::Bool(true))),
        "FALSE" => return Ok(Expression::Literal(Literal::Bool(false))),
        "NULL" => return Ok(Expression::Literal(Literal::Null)),
        _ => {}
    }

    // Treat as column reference
    if let Some(dot_pos) = input.find('.') {
        Ok(Expression::Column(ColumnRef {
            table: Some(input[..dot_pos].trim().to_string()),
            name: input[dot_pos + 1..].trim().to_string(),
        }))
    } else {
        Ok(Expression::Column(ColumnRef {
            table: None,
            name: input.to_string(),
        }))
    }
}

/// Parse ORDER BY item
fn parse_order_by_item(input: &str) -> Result<OrderByItem> {
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
        expr: parse_simple_column_or_literal(parts[0])?,
        direction,
    })
}

/// Check if a string is a valid identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let chars: Vec<char> = s.chars().collect();
    if !chars[0].is_alphabetic() && chars[0] != '_' {
        return false;
    }

    chars.iter().all(|c| c.is_alphanumeric() || *c == '_')
}

/// Parse an INSERT statement
fn parse_insert_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    // Find INTO keyword position
    let into_pos = upper_input.find(" INTO ")
        .ok_or_else(|| HyperQLError::ParseError {
            message: "INSERT statement must contain INTO clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    // Extract table name after INTO
    let after_into = &input[into_pos + 6..].trim();
    let parts: Vec<&str> = after_into.split_whitespace().collect();
    let table = parts.first()
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing table name after INTO".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?.to_string();

    // Find columns specification (table_name (col1, col2, ...))
    let columns_start = after_into.find('(')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "INSERT statement must specify columns in parentheses".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let columns_end = after_into.find(')')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing closing parenthesis for column list".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let column_list = &after_into[columns_start + 1..columns_end];
    let columns: Vec<String> = column_list
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Find VALUES keyword
    let values_pos = upper_input.find(" VALUES ")
        .ok_or_else(|| HyperQLError::ParseError {
            message: "INSERT statement must contain VALUES clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    // Parse VALUES clause
    let values_part = &input[values_pos + 8..].trim();
    let values = parse_values_list(values_part)?;

    Ok(Statement::Insert(InsertStatement {
        table,
        columns,
        values,
    }))
}

/// Parse VALUES list for INSERT statements
fn parse_values_list(input: &str) -> Result<Vec<Vec<Expression>>> {
    let mut values = Vec::new();
    let input = input.trim();

    if !input.starts_with('(') {
        return Err(HyperQLError::ParseError {
            message: "VALUES must start with opening parenthesis".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        });
    }

    // Simple parsing for now - assumes single VALUES row
    let end_paren = input.find(')')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing closing parenthesis in VALUES clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let values_content = &input[1..end_paren];
    let value_strings: Vec<&str> = values_content.split(',').collect();

    let mut row_values = Vec::new();
    for value_str in value_strings {
        row_values.push(parse_simple_column_or_literal(value_str.trim())?);
    }

    values.push(row_values);
    Ok(values)
}

/// Parse an UPDATE statement
fn parse_update_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    // Extract table name after UPDATE
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(HyperQLError::ParseError {
            message: "UPDATE statement must specify table name".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        });
    }
    let table = parts[1].to_string();

    // Find SET clause
    let set_pos = upper_input.find(" SET ")
        .ok_or_else(|| HyperQLError::ParseError {
            message: "UPDATE statement must contain SET clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    // Find WHERE clause (optional)
    let where_pos = upper_input.find(" WHERE ");
    let (set_part, where_clause) = if let Some(where_pos) = where_pos {
        let set_part = &input[set_pos + 5..where_pos];
        let where_part = &input[where_pos + 7..];
        (set_part, Some(parse_simple_expression(where_part)?))
    } else {
        let set_part = &input[set_pos + 5..];
        (set_part, None)
    };

    // Parse assignments in SET clause
    let assignments = parse_assignments(set_part)?;

    Ok(Statement::Update(UpdateStatement {
        table,
        assignments,
        where_clause,
    }))
}

/// Parse assignments for UPDATE statements (column = value, column = value)
fn parse_assignments(input: &str) -> Result<Vec<Assignment>> {
    let mut assignments = Vec::new();
    let input = input.trim();

    for assignment_str in input.split(',') {
        let assignment_str = assignment_str.trim();
        let parts: Vec<&str> = assignment_str.splitn(2, '=').collect();

        if parts.len() != 2 {
            return Err(HyperQLError::ParseError {
                message: "Invalid assignment syntax. Expected: column = value".to_string(),
                line: 1,
                column: 1,
                source_text: Some(assignment_str.to_string()),
            });
        }

        let column = parts[0].trim().to_string();
        let value = parse_simple_column_or_literal(parts[1].trim())?;

        assignments.push(Assignment { column, value });
    }

    Ok(assignments)
}

/// Parse a DELETE statement
fn parse_delete_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    // Find FROM clause
    let from_pos = upper_input.find(" FROM ")
        .ok_or_else(|| HyperQLError::ParseError {
            message: "DELETE statement must contain FROM clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    // Find WHERE clause (optional)
    let where_pos = upper_input.find(" WHERE ");
    let (table_part, where_clause) = if let Some(where_pos) = where_pos {
        let table_part = &input[from_pos + 6..where_pos];
        let where_part = &input[where_pos + 7..];
        (table_part, Some(parse_simple_expression(where_part)?))
    } else {
        let table_part = &input[from_pos + 6..];
        (table_part, None)
    };

    let table = table_part.trim().split_whitespace().next()
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing table name in DELETE FROM clause".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?.to_string();

    Ok(Statement::Delete(DeleteStatement {
        table,
        where_clause,
    }))
}

/// Parse GROUP BY clause
fn parse_group_by_list(input: &str) -> Result<Vec<Expression>> {
    let input = input.trim();
    let mut expressions = Vec::new();

    for part in input.split(',') {
        let expr = parse_simple_column_or_literal(part.trim())?;
        expressions.push(expr);
    }

    Ok(expressions)
}

/// Parse expressions including function calls and aggregates
fn parse_expression_or_function(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Check if it looks like a function call (contains parentheses)
    if let Some(paren_pos) = input.find('(') {
        let func_name = input[..paren_pos].trim();

        // Check if this is a known aggregate function
        if is_aggregate_function(func_name) {
            return parse_function_call(input);
        }
    }

    // Check for arithmetic expressions with parentheses or operators
    if input.contains('+') || input.contains('-') || input.contains('*') || input.contains('/') {
        return parse_arithmetic_expression(input);
    }

    // Default to column or literal parsing
    parse_simple_column_or_literal(input)
}

/// Parse function calls including aggregate functions
fn parse_function_call(input: &str) -> Result<Expression> {
    let input = input.trim();
    let paren_pos = input.find('(')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Invalid function call syntax".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let func_name = input[..paren_pos].trim().to_string();

    let end_paren = input.rfind(')')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing closing parenthesis in function call".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let args_str = &input[paren_pos + 1..end_paren];
    let mut args = Vec::new();

    if !args_str.trim().is_empty() {
        for arg in args_str.split(',') {
            let arg = arg.trim();
            if arg == "*" && is_aggregate_function(&func_name) {
                // Special case for COUNT(*), SUM(*), etc.
                args.push(Expression::Column(ColumnRef {
                    table: None,
                    name: "*".to_string(),
                }));
            } else {
                args.push(parse_simple_column_or_literal(arg)?);
            }
        }
    }

    Ok(Expression::Function {
        name: func_name,
        args,
    })
}

/// Parse arithmetic expressions with proper operator precedence (PEMDAS)
fn parse_arithmetic_expression(input: &str) -> Result<Expression> {
    parse_addition_expression(input)
}

/// Parse addition and subtraction (lowest precedence)
fn parse_addition_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Find rightmost + or - operator (left-associative)
    let mut paren_depth = 0;
    let mut last_op_pos = None;
    let mut last_op_char = '+';

    for (i, ch) in input.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '+' | '-' if paren_depth == 0 && i > 0 => {
                // Ensure it's not a unary operator at the start
                last_op_pos = Some(i);
                last_op_char = ch;
                break;
            },
            _ => {}
        }
    }

    if let Some(op_pos) = last_op_pos {
        let left_part = &input[..op_pos].trim();
        let right_part = &input[op_pos + 1..].trim();

        let left_expr = parse_addition_expression(left_part)?;
        let right_expr = parse_multiplication_expression(right_part)?;

        let op = match last_op_char {
            '+' => BinaryOperator::Add,
            '-' => BinaryOperator::Subtract,
            _ => unreachable!(),
        };

        return Ok(Expression::Binary {
            left: Box::new(left_expr),
            op,
            right: Box::new(right_expr),
        });
    }

    // No addition/subtraction found, parse multiplication/division
    parse_multiplication_expression(input)
}

/// Parse multiplication and division (higher precedence)
fn parse_multiplication_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Find rightmost * or / operator (left-associative)
    let mut paren_depth = 0;
    let mut last_op_pos = None;
    let mut last_op_char = '*';

    for (i, ch) in input.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '*' | '/' if paren_depth == 0 => {
                last_op_pos = Some(i);
                last_op_char = ch;
                break;
            },
            _ => {}
        }
    }

    if let Some(op_pos) = last_op_pos {
        let left_part = &input[..op_pos].trim();
        let right_part = &input[op_pos + 1..].trim();

        let left_expr = parse_multiplication_expression(left_part)?;
        let right_expr = parse_primary_expression(right_part)?;

        let op = match last_op_char {
            '*' => BinaryOperator::Multiply,
            '/' => BinaryOperator::Divide,
            _ => unreachable!(),
        };

        return Ok(Expression::Binary {
            left: Box::new(left_expr),
            op,
            right: Box::new(right_expr),
        });
    }

    // No multiplication/division found, parse primary expression
    parse_primary_expression(input)
}

/// Parse primary expressions (highest precedence: parentheses, literals, columns)
fn parse_primary_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Handle parentheses
    if input.starts_with('(') && input.ends_with(')') {
        // Check if parentheses are balanced and outermost
        let mut paren_depth = 0;
        let mut all_enclosed = true;

        for (i, ch) in input.char_indices() {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth -= 1;
                    if paren_depth == 0 && i < input.len() - 1 {
                        all_enclosed = false;
                        break;
                    }
                },
                _ => {}
            }
        }

        if all_enclosed && paren_depth == 0 {
            // Remove outer parentheses and parse the inner expression
            let inner = &input[1..input.len()-1];
            return parse_arithmetic_expression(inner);
        }
    }

    // Parse as column, literal, or function call
    parse_simple_column_or_literal(input)
}

/// Check if a function name is a known aggregate function
fn is_aggregate_function(name: &str) -> bool {
    matches!(name.to_uppercase().as_str(), "COUNT" | "SUM" | "AVG" | "MIN" | "MAX")
}

/// Parse TRAVERSE clause with Cypher-style patterns
fn parse_traverse_clause(input: &str) -> Result<TraverseClause> {
    let input = input.trim();
    let mut patterns = Vec::new();

    // Split multiple patterns by comma
    for pattern_str in input.split(',') {
        let pattern_str = pattern_str.trim();
        if !pattern_str.is_empty() {
            patterns.push(parse_traverse_pattern(pattern_str)?);
        }
    }

    if patterns.is_empty() {
        return Err(HyperQLError::ParseError {
            message: "TRAVERSE clause must contain at least one pattern".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        });
    }

    Ok(TraverseClause { patterns })
}

/// Parse a single traverse pattern like (a)-[r:follows*1..3]->(b)
fn parse_traverse_pattern(input: &str) -> Result<TraversePattern> {
    let input = input.trim();

    // Find the start node pattern (a)
    let start_paren = input.find('(')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Pattern must start with node specification in parentheses".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let end_paren = input.find(')')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing closing parenthesis for start node".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let start_node = parse_node_pattern(&input[start_paren + 1..end_paren])?;

    // Find the relationship pattern -[...]-> or --> or --
    let after_start_node = &input[end_paren + 1..];

    // Parse relationship pattern
    let (relationship, relationship_end_pos) = parse_relationship_pattern(after_start_node)?;

    // Find the end node pattern (b)
    let end_node_part = &after_start_node[relationship_end_pos..];
    let start_paren_end = end_node_part.find('(')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Pattern must end with node specification in parentheses".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let end_paren_end = end_node_part.find(')')
        .ok_or_else(|| HyperQLError::ParseError {
            message: "Missing closing parenthesis for end node".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

    let end_node = parse_node_pattern(&end_node_part[start_paren_end + 1..end_paren_end])?;

    Ok(TraversePattern {
        start_node,
        relationship,
        end_node,
    })
}

/// Parse node pattern like 'a' or 'a:User' or 'a:User {name: "Alice"}'
fn parse_node_pattern(input: &str) -> Result<NodePattern> {
    let input = input.trim();

    if input.is_empty() {
        return Ok(NodePattern {
            variable: None,
            label: None,
            properties: None,
        });
    }

    // Simple parsing for now - just variable name and optional label
    let parts: Vec<&str> = input.split(':').collect();
    let variable = if !parts[0].is_empty() {
        Some(parts[0].trim().to_string())
    } else {
        None
    };

    let label = if parts.len() > 1 && !parts[1].is_empty() {
        Some(parts[1].trim().to_string())
    } else {
        None
    };

    Ok(NodePattern {
        variable,
        label,
        properties: None, // TODO: Parse property constraints
    })
}

/// Parse relationship pattern like -[r:follows*1..3]-> or --> or --
fn parse_relationship_pattern(input: &str) -> Result<(RelationshipPattern, usize)> {
    let input = input.trim();

    // Check for different arrow patterns
    if let Some(pos) = input.find("-->") {
        // Simple outgoing arrow
        Ok((RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelationshipDirection::Outgoing,
            variable_length: None,
            optional: false,
            properties: None,
        }, pos + 3))
    } else if let Some(pos) = input.find("<--") {
        // Simple incoming arrow
        Ok((RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelationshipDirection::Incoming,
            variable_length: None,
            optional: false,
            properties: None,
        }, pos + 3))
    } else if let Some(pos) = input.find("--") {
        // Simple undirected
        Ok((RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelationshipDirection::Undirected,
            variable_length: None,
            optional: false,
            properties: None,
        }, pos + 2))
    } else if input.starts_with('-') {
        // Complex relationship pattern -[...]-> or -[...]<- or -[...]-
        parse_complex_relationship_pattern(input)
    } else {
        Err(HyperQLError::ParseError {
            message: "Invalid relationship pattern. Expected arrow syntax like --> or -[type]->".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })
    }
}

/// Parse complex relationship patterns like -[r:follows*1..3]->
fn parse_complex_relationship_pattern(input: &str) -> Result<(RelationshipPattern, usize)> {
    let input = input.trim();

    if !input.starts_with('-') {
        return Err(HyperQLError::ParseError {
            message: "Relationship pattern must start with dash".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        });
    }

    // Find the bracket part
    if let Some(bracket_start) = input.find('[') {
        if let Some(bracket_end) = input.find(']') {
            // Parse the content inside brackets
            let bracket_content = &input[bracket_start + 1..bracket_end];
            let (variable, rel_type, variable_length, optional) = parse_relationship_spec(bracket_content)?;

            // Determine direction from the arrow after the bracket
            let after_bracket = &input[bracket_end + 1..];
            let direction = if after_bracket.starts_with("->") {
                RelationshipDirection::Outgoing
            } else if after_bracket.starts_with("<-") {
                RelationshipDirection::Incoming
            } else if after_bracket.starts_with('-') {
                RelationshipDirection::Undirected
            } else {
                return Err(HyperQLError::ParseError {
                    message: "Invalid arrow direction after relationship specification".to_string(),
                    line: 1,
                    column: 1,
                    source_text: Some(input.to_string()),
                });
            };

            let arrow_len = match direction {
                RelationshipDirection::Outgoing | RelationshipDirection::Incoming => 2,
                RelationshipDirection::Undirected => 1,
            };

            Ok((RelationshipPattern {
                variable,
                rel_type,
                direction,
                variable_length,
                optional,
                properties: None,
            }, bracket_end + 1 + arrow_len))
        } else {
            Err(HyperQLError::ParseError {
                message: "Missing closing bracket in relationship pattern".to_string(),
                line: 1,
                column: 1,
                source_text: Some(input.to_string()),
            })
        }
    } else {
        Err(HyperQLError::ParseError {
            message: "Expected bracket in relationship pattern".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })
    }
}

/// Parse relationship specification inside brackets like "r:follows*1..3" or "follows?" or ""
fn parse_relationship_spec(input: &str) -> Result<(Option<String>, Option<String>, Option<VariableLength>, bool)> {
    let input = input.trim();

    if input.is_empty() {
        return Ok((None, None, None, false));
    }

    let mut variable = None;
    let mut rel_type = None;
    let mut variable_length = None;
    let mut optional = false;

    // Check for optional relationship (ends with ?)
    let input = if input.ends_with('?') {
        optional = true;
        &input[..input.len() - 1]
    } else {
        input
    };

    // Check for variable length specification (*1..3 or * or *2)
    let input = if let Some(star_pos) = input.find('*') {
        let var_len_str = &input[star_pos + 1..];
        let var_len = parse_variable_length(var_len_str)?;
        variable_length = Some(var_len);
        &input[..star_pos]
    } else {
        input
    };

    // Parse variable and relationship type
    if input.contains(':') {
        let parts: Vec<&str> = input.split(':').collect();
        if !parts[0].is_empty() {
            variable = Some(parts[0].trim().to_string());
        }
        if parts.len() > 1 && !parts[1].is_empty() {
            rel_type = Some(parts[1].trim().to_string());
        }
    } else if !input.is_empty() {
        // Just a relationship type, no variable
        rel_type = Some(input.trim().to_string());
    }

    Ok((variable, rel_type, variable_length, optional))
}

/// Parse variable length specification like "1..3" or "2" or ""
fn parse_variable_length(input: &str) -> Result<VariableLength> {
    let input = input.trim();

    if input.is_empty() {
        // Just * means any number of hops
        return Ok(VariableLength {
            min_hops: None,
            max_hops: None,
        });
    }

    if input.contains("..") {
        // Range specification like "1..3"
        let parts: Vec<&str> = input.split("..").collect();
        let min_hops = if parts[0].is_empty() {
            None
        } else {
            Some(parts[0].parse::<u32>().map_err(|_| HyperQLError::ParseError {
                message: "Invalid minimum hop count in variable length specification".to_string(),
                line: 1,
                column: 1,
                source_text: Some(input.to_string()),
            })?)
        };

        let max_hops = if parts.len() > 1 && !parts[1].is_empty() {
            Some(parts[1].parse::<u32>().map_err(|_| HyperQLError::ParseError {
                message: "Invalid maximum hop count in variable length specification".to_string(),
                line: 1,
                column: 1,
                source_text: Some(input.to_string()),
            })?)
        } else {
            None
        };

        Ok(VariableLength { min_hops, max_hops })
    } else {
        // Single number means exactly that many hops
        let hops = input.parse::<u32>().map_err(|_| HyperQLError::ParseError {
            message: "Invalid hop count in variable length specification".to_string(),
            line: 1,
            column: 1,
            source_text: Some(input.to_string()),
        })?;

        Ok(VariableLength {
            min_hops: Some(hops),
            max_hops: Some(hops),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select_wildcard() {
        let query = "SELECT * FROM entities";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.select_list.len(), 1);
                assert!(matches!(stmt.select_list[0], SelectItem::Wildcard));
                assert!(stmt.from.is_some());
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_where() {
        let query = "SELECT name FROM entities WHERE name = 'Alice'";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.select_list.len(), 1);
                assert!(stmt.where_clause.is_some());
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_multiple_columns() {
        let query = "SELECT name, age, active FROM users";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.select_list.len(), 3);
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_complex_where() {
        let query = "SELECT * FROM entities WHERE name = 'Alice' AND age > 25";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert!(stmt.where_clause.is_some());
                if let Some(Expression::Binary { op, .. }) = stmt.where_clause {
                    assert_eq!(op, BinaryOperator::And);
                }
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_order_by() {
        let query = "SELECT name FROM entities ORDER BY name DESC";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.order_by.len(), 1);
                assert_eq!(stmt.order_by[0].direction, OrderDirection::Desc);
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_limit() {
        let query = "SELECT * FROM entities LIMIT 10";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.limit, Some(10));
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_aggregate_functions() {
        let query = "SELECT COUNT(*), AVG(age), MAX(score) FROM users";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.select_list.len(), 3);
                // Check that all are function expressions
                for item in &stmt.select_list {
                    if let SelectItem::Expression { expr, .. } = item {
                        assert!(matches!(expr, Expression::Function { .. }));
                    }
                }
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_group_by() {
        let query = "SELECT category, COUNT(*) FROM products GROUP BY category";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.group_by.len(), 1);
                assert!(matches!(stmt.group_by[0], Expression::Column(_)));
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_select_with_having() {
        let query = "SELECT category, COUNT(*) FROM products GROUP BY category HAVING COUNT(*) > 5";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert!(stmt.having.is_some());
                assert_eq!(stmt.group_by.len(), 1);
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_insert_statement() {
        let query = "INSERT INTO users (name, age, email) VALUES ('Alice', 30, 'alice@example.com')";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Insert(stmt) => {
                assert_eq!(stmt.table, "users");
                assert_eq!(stmt.columns, vec!["name", "age", "email"]);
                assert_eq!(stmt.values.len(), 1);
                assert_eq!(stmt.values[0].len(), 3);
            },
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_update_statement() {
        let query = "UPDATE users SET name = 'Bob', age = 25 WHERE id = 1";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Update(stmt) => {
                assert_eq!(stmt.table, "users");
                assert_eq!(stmt.assignments.len(), 2);
                assert_eq!(stmt.assignments[0].column, "name");
                assert_eq!(stmt.assignments[1].column, "age");
                assert!(stmt.where_clause.is_some());
            },
            _ => panic!("Expected UPDATE statement"),
        }
    }

    #[test]
    fn test_delete_statement() {
        let query = "DELETE FROM users WHERE age < 18";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Delete(stmt) => {
                assert_eq!(stmt.table, "users");
                assert!(stmt.where_clause.is_some());
            },
            _ => panic!("Expected DELETE statement"),
        }
    }

    #[test]
    fn test_delete_without_where() {
        let query = "DELETE FROM users";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Delete(stmt) => {
                assert_eq!(stmt.table, "users");
                assert!(stmt.where_clause.is_none());
            },
            _ => panic!("Expected DELETE statement"),
        }
    }

    #[test]
    fn test_arithmetic_expressions() {
        let query = "SELECT price * quantity, price + tax FROM orders";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.select_list.len(), 2);
                for item in &stmt.select_list {
                    if let SelectItem::Expression { expr, .. } = item {
                        assert!(matches!(expr, Expression::Binary { .. }));
                    }
                }
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_function_calls() {
        let query = "SELECT UPPER(name), LENGTH(description) FROM products";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert_eq!(stmt.select_list.len(), 2);
                for item in &stmt.select_list {
                    if let SelectItem::Expression { expr, .. } = item {
                        if let Expression::Function { name, .. } = expr {
                            assert!(name == "UPPER" || name == "LENGTH");
                        }
                    }
                }
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_simple_traverse() {
        let query = "SELECT name FROM users TRAVERSE (a)-->(b)";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert!(stmt.traverse_clause.is_some());
                let traverse = stmt.traverse_clause.unwrap();
                assert_eq!(traverse.patterns.len(), 1);

                let pattern = &traverse.patterns[0];
                assert_eq!(pattern.start_node.variable, Some("a".to_string()));
                assert_eq!(pattern.end_node.variable, Some("b".to_string()));
                assert_eq!(pattern.relationship.direction, RelationshipDirection::Outgoing);
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_complex_traverse_pattern() {
        let query = "SELECT * FROM users TRAVERSE (a:User)-[r:follows*1..3]->(b:User)";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert!(stmt.traverse_clause.is_some());
                let traverse = stmt.traverse_clause.unwrap();
                assert_eq!(traverse.patterns.len(), 1);

                let pattern = &traverse.patterns[0];
                assert_eq!(pattern.start_node.variable, Some("a".to_string()));
                assert_eq!(pattern.start_node.label, Some("User".to_string()));
                assert_eq!(pattern.end_node.variable, Some("b".to_string()));
                assert_eq!(pattern.end_node.label, Some("User".to_string()));

                assert_eq!(pattern.relationship.variable, Some("r".to_string()));
                assert_eq!(pattern.relationship.rel_type, Some("follows".to_string()));
                assert_eq!(pattern.relationship.direction, RelationshipDirection::Outgoing);

                let var_len = pattern.relationship.variable_length.as_ref().unwrap();
                assert_eq!(var_len.min_hops, Some(1));
                assert_eq!(var_len.max_hops, Some(3));
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_undirected_traverse() {
        let query = "SELECT * FROM users TRAVERSE (a)-[knows]-(b)";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert!(stmt.traverse_clause.is_some());
                let traverse = stmt.traverse_clause.unwrap();
                let pattern = &traverse.patterns[0];
                assert_eq!(pattern.relationship.direction, RelationshipDirection::Undirected);
                assert_eq!(pattern.relationship.rel_type, Some("knows".to_string()));
            },
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_optional_relationship() {
        let query = "SELECT * FROM users TRAVERSE (a)-[follows?]->(b)";
        let result = parse_statement(query).unwrap();

        match result {
            Statement::Select(stmt) => {
                assert!(stmt.traverse_clause.is_some());
                let traverse = stmt.traverse_clause.unwrap();
                let pattern = &traverse.patterns[0];
                assert!(pattern.relationship.optional);
                assert_eq!(pattern.relationship.rel_type, Some("follows".to_string()));
            },
            _ => panic!("Expected SELECT statement"),
        }
    }
}