use crate::ast::*;
use crate::error::*;
use super::{utils, geometric};

pub fn parse_simple_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Check for geometric expressions first (NEAR/WITHIN)
    let upper_input = input.to_uppercase();
    if upper_input.contains(" NEAR ") && upper_input.contains(" WITHIN ") {
        return geometric::parse_near_expression(input);
    }

    // Check for IS NULL / IS NOT NULL
    if let Some(is_null_expr) = try_parse_is_null(input)? {
        return Ok(is_null_expr);
    }

    // Check for BETWEEN
    if let Some(between_expr) = try_parse_between(input)? {
        return Ok(between_expr);
    }

    if let Some(and_pos) = utils::find_operator_position(input, " AND ") {
        let left = parse_simple_expression(&input[..and_pos])?;
        let right = parse_simple_expression(&input[and_pos + 5..])?;
        return Ok(Expression::Binary {
            left: Box::new(left),
            op: BinaryOperator::And,
            right: Box::new(right),
        });
    }

    if let Some(or_pos) = utils::find_operator_position(input, " OR ") {
        let left = parse_simple_expression(&input[..or_pos])?;
        let right = parse_simple_expression(&input[or_pos + 4..])?;
        return Ok(Expression::Binary {
            left: Box::new(left),
            op: BinaryOperator::Or,
            right: Box::new(right),
        });
    }

    // Check for LIKE / NOT LIKE
    if let Some(like_expr) = try_parse_like(input)? {
        return Ok(like_expr);
    }

    // Check for IN / NOT IN
    if let Some(in_expr) = try_parse_in(input)? {
        return Ok(in_expr);
    }

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
        if let Some(op_pos) = utils::find_operator_position(input, op_str) {
            let left_part = &input[..op_pos];
            let right_part = &input[op_pos + op_str.len()..];

            // Try parsing as arithmetic expression first, fallback to simple column/literal
            let left = if left_part.contains('+') || left_part.contains('-') || left_part.contains('*') || left_part.contains('/') {
                parse_arithmetic_expression(left_part)?
            } else {
                parse_simple_column_or_literal(left_part)?
            };

            let right = if right_part.contains('+') || right_part.contains('-') || right_part.contains('*') || right_part.contains('/') {
                parse_arithmetic_expression(right_part)?
            } else {
                parse_simple_column_or_literal(right_part)?
            };

            return Ok(Expression::Binary {
                left: Box::new(left),
                op: op_type.clone(),
                right: Box::new(right),
            });
        }
    }

    parse_simple_column_or_literal(input)
}

pub fn parse_simple_column_or_literal(input: &str) -> Result<Expression> {
    let input = input.trim();

    if (input.starts_with('"') && input.ends_with('"')) ||
       (input.starts_with('\'') && input.ends_with('\'')) {
        let content = &input[1..input.len()-1];
        return Ok(Expression::Literal(Literal::String(content.to_string())));
    }

    if let Ok(int_val) = input.parse::<i64>() {
        return Ok(Expression::Literal(Literal::Int(int_val)));
    }

    if let Ok(float_val) = input.parse::<f64>() {
        return Ok(Expression::Literal(Literal::Float(float_val)));
    }

    match input.to_uppercase().as_str() {
        "TRUE" => return Ok(Expression::Literal(Literal::Bool(true))),
        "FALSE" => return Ok(Expression::Literal(Literal::Bool(false))),
        "NULL" => return Ok(Expression::Literal(Literal::Null)),
        _ => {}
    }

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

pub fn parse_literal_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    if (input.starts_with('"') && input.ends_with('"')) ||
       (input.starts_with('\'') && input.ends_with('\'')) {
        let content = &input[1..input.len()-1];
        return Ok(Expression::Literal(Literal::String(content.to_string())));
    }

    if let Ok(int_val) = input.parse::<i64>() {
        return Ok(Expression::Literal(Literal::Int(int_val)));
    }

    if let Ok(float_val) = input.parse::<f64>() {
        return Ok(Expression::Literal(Literal::Float(float_val)));
    }

    match input.to_uppercase().as_str() {
        "TRUE" => Ok(Expression::Literal(Literal::Bool(true))),
        "FALSE" => Ok(Expression::Literal(Literal::Bool(false))),
        "NULL" => Ok(Expression::Literal(Literal::Null)),
        _ => Err(HyperQLError::simple_parse_error(
            &format!("Invalid literal value: {}", input),
            input,
            1,
            1,
        ))
    }
}

pub fn parse_expression_or_function(input: &str) -> Result<Expression> {
    let input = input.trim();
    
    if let Some(paren_pos) = input.find('(') {
        let func_name = input[..paren_pos].trim();
        
        if utils::is_aggregate_function(func_name) {
            return parse_function_call(input);
        }
    }
    
    if input.contains('+') || input.contains('-') || input.contains('*') || input.contains('/') {
        return parse_arithmetic_expression(input);
    }
    
    parse_simple_column_or_literal(input)
}

fn parse_function_call(input: &str) -> Result<Expression> {
    let input = input.trim();
    let paren_pos = input.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Invalid function call syntax",
            input,
            1,
            1,
        ))?;

    let func_name = input[..paren_pos].trim().to_string();

    let end_paren = input.rfind(')')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing closing parenthesis in function call",
            input,
            1,
            1,
        ))?;

    let args_str = &input[paren_pos + 1..end_paren];
    let mut args = Vec::new();

    if !args_str.trim().is_empty() {
        for arg in args_str.split(',') {
            let arg = arg.trim();
            if arg == "*" && utils::is_aggregate_function(&func_name) {
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

fn parse_arithmetic_expression(input: &str) -> Result<Expression> {
    parse_addition_expression(input)
}

fn parse_addition_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    let mut paren_depth = 0;
    let mut last_op_pos = None;
    let mut last_op_char = '+';

    for (i, ch) in input.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '+' | '-' if paren_depth == 0 && i > 0 => {
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

    parse_multiplication_expression(input)
}

fn parse_multiplication_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

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

    parse_primary_expression(input)
}

fn parse_primary_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    if input.starts_with('(') && input.ends_with(')') {
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
            let inner = &input[1..input.len()-1];
            return parse_arithmetic_expression(inner);
        }
    }

    parse_simple_column_or_literal(input)
}

fn try_parse_is_null(input: &str) -> Result<Option<Expression>> {
    let upper = input.to_uppercase();

    if let Some(pos) = utils::find_operator_position(&upper, " IS NOT NULL") {
        let expr_part = &input[..pos].trim();
        let expr = parse_simple_column_or_literal(expr_part)?;
        return Ok(Some(Expression::Unary {
            op: UnaryOperator::IsNotNull,
            expr: Box::new(expr),
        }));
    }

    if let Some(pos) = utils::find_operator_position(&upper, " IS NULL") {
        let expr_part = &input[..pos].trim();
        let expr = parse_simple_column_or_literal(expr_part)?;
        return Ok(Some(Expression::Unary {
            op: UnaryOperator::IsNull,
            expr: Box::new(expr),
        }));
    }

    Ok(None)
}

fn try_parse_between(input: &str) -> Result<Option<Expression>> {
    let upper = input.to_uppercase();

    let (negated, between_keyword) = if let Some(pos) = upper.find(" NOT BETWEEN ") {
        (true, pos)
    } else if let Some(pos) = upper.find(" BETWEEN ") {
        (false, pos)
    } else {
        return Ok(None);
    };

    let expr_part = &input[..between_keyword].trim();
    let expr = parse_simple_column_or_literal(expr_part)?;

    let between_keyword_len = if negated { 13 } else { 9 };
    let rest = &input[between_keyword + between_keyword_len..].trim();

    if let Some(and_pos) = utils::find_operator_position(rest, " AND ") {
        let lower_part = &rest[..and_pos].trim();
        let upper_part = &rest[and_pos + 5..].trim();

        let lower = parse_simple_column_or_literal(lower_part)?;
        let upper_expr = parse_simple_column_or_literal(upper_part)?;

        return Ok(Some(Expression::Between {
            expr: Box::new(expr),
            lower: Box::new(lower),
            upper: Box::new(upper_expr),
            negated,
        }));
    }

    Err(HyperQLError::simple_parse_error(
        "BETWEEN requires AND keyword",
        input,
        1,
        1,
    ))
}

fn try_parse_like(input: &str) -> Result<Option<Expression>> {
    let upper = input.to_uppercase();

    if let Some(pos) = utils::find_operator_position(&upper, " NOT LIKE ") {
        let left_part = &input[..pos].trim();
        let right_part = &input[pos + 11..].trim();

        let left = parse_simple_column_or_literal(left_part)?;
        let right = parse_simple_column_or_literal(right_part)?;

        return Ok(Some(Expression::Binary {
            left: Box::new(left),
            op: BinaryOperator::NotLike,
            right: Box::new(right),
        }));
    }

    if let Some(pos) = utils::find_operator_position(&upper, " LIKE ") {
        let left_part = &input[..pos].trim();
        let right_part = &input[pos + 6..].trim();

        let left = parse_simple_column_or_literal(left_part)?;
        let right = parse_simple_column_or_literal(right_part)?;

        return Ok(Some(Expression::Binary {
            left: Box::new(left),
            op: BinaryOperator::Like,
            right: Box::new(right),
        }));
    }

    Ok(None)
}

fn try_parse_in(input: &str) -> Result<Option<Expression>> {
    let upper = input.to_uppercase();

    let (negated, in_keyword_pos) = if let Some(pos) = utils::find_operator_position(&upper, " NOT IN ") {
        (true, pos)
    } else if let Some(pos) = utils::find_operator_position(&upper, " IN ") {
        (false, pos)
    } else {
        return Ok(None);
    };

    let expr_part = &input[..in_keyword_pos].trim();
    let expr = parse_simple_column_or_literal(expr_part)?;

    let in_keyword_len = if negated { 8 } else { 4 };
    let list_part = &input[in_keyword_pos + in_keyword_len..].trim();

    if !list_part.starts_with('(') || !list_part.ends_with(')') {
        return Err(HyperQLError::simple_parse_error(
            "IN operator requires parenthesized list",
            input,
            1,
            1,
        ));
    }

    let inner = &list_part[1..list_part.len()-1].trim();
    let mut values = Vec::new();

    if !inner.is_empty() {
        for item in split_list_items(inner) {
            values.push(parse_simple_column_or_literal(item.trim())?);
        }
    }

    let list_expr = Expression::Function {
        name: "__IN_LIST__".to_string(),
        args: values,
    };

    let op = if negated {
        BinaryOperator::NotIn
    } else {
        BinaryOperator::In
    };

    Ok(Some(Expression::Binary {
        left: Box::new(expr),
        op,
        right: Box::new(list_expr),
    }))
}

fn split_list_items(input: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut current_start = 0;
    let mut in_quote = false;
    let mut quote_char = '"';
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        if (ch == '"' || ch == '\'') && (i == 0 || chars[i - 1] != '\\') {
            if !in_quote {
                in_quote = true;
                quote_char = ch;
            } else if ch == quote_char {
                in_quote = false;
            }
        }

        if ch == ',' && !in_quote {
            items.push(&input[current_start..i]);
            current_start = i + 1;
        }
    }

    if current_start < input.len() {
        items.push(&input[current_start..]);
    }

    items
}
