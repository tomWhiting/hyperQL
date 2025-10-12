use crate::ast::*;
use crate::error::*;
use super::utils;

pub fn parse_simple_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

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
