use crate::ast::*;
use crate::error::*;
use super::{select, expression, schema};

pub fn parse_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    if upper_input.starts_with("SELECT") {
        select::parse_select_statement(input)
    } else if upper_input.starts_with("INSERT") {
        parse_insert_statement(input)
    } else if upper_input.starts_with("UPDATE") {
        parse_update_statement(input)
    } else if upper_input.starts_with("DELETE") {
        parse_delete_statement(input)
    } else if upper_input.starts_with("CREATE SCHEMA") || upper_input.starts_with("ALTER SCHEMA")
        || upper_input.starts_with("DROP SCHEMA") || upper_input.starts_with("DESCRIBE SCHEMA") {
        schema::parse_schema_statement(input).map(Statement::Schema)
    } else {
        Err(HyperQLError::simple_parse_error(
            "Unsupported statement type. Supported: SELECT, INSERT, UPDATE, DELETE, CREATE/ALTER/DROP/DESCRIBE SCHEMA",
            input,
            1,
            1,
        ))
    }
}

fn parse_insert_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    let into_pos = upper_input.find(" INTO ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "INSERT statement must contain INTO clause",
            input,
            1,
            1,
        ))?;

    let after_into = &input[into_pos + 6..].trim();
    let parts: Vec<&str> = after_into.split_whitespace().collect();
    let table = parts.first()
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing table name after INTO",
            input,
            1,
            1,
        ))?.to_string();

    let columns_start = after_into.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "INSERT statement must specify columns in parentheses",
            input,
            1,
            1,
        ))?;

    let columns_end = after_into.find(')')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "INSERT statement columns must be closed with )",
            input,
            1,
            1,
        ))?;

    let columns_str = &after_into[columns_start + 1..columns_end];
    let columns: Vec<String> = columns_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let values_pos = upper_input.find(" VALUES ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "INSERT statement must contain VALUES clause",
            input,
            1,
            1,
        ))?;

    let values_str = &input[values_pos + 8..];
    let values = parse_values_list(values_str)?;

    Ok(Statement::Insert(InsertStatement {
        table,
        columns,
        values,
    }))
}

fn parse_values_list(input: &str) -> Result<Vec<Vec<Expression>>> {
    let input = input.trim();
    
    if !input.starts_with('(') {
        return Err(HyperQLError::simple_parse_error(
            "VALUES must start with (",
            input,
            1,
            1,
        ));
    }

    let mut values = Vec::new();
    let mut current_pos = 0;
    
    while current_pos < input.len() {
        let start = input[current_pos..].find('(')
            .map(|pos| pos + current_pos);
        
        if let Some(start_pos) = start {
            let end = input[start_pos..].find(')')
                .map(|pos| pos + start_pos)
                .ok_or_else(|| HyperQLError::simple_parse_error(
                    "Unclosed parentheses in VALUES",
                    input,
                    1,
                    1,
                ))?;
            
            let value_str = &input[start_pos + 1..end];
            let mut row = Vec::new();
            
            for value_part in value_str.split(',') {
                let value_part = value_part.trim();
                row.push(expression::parse_literal_expression(value_part)?);
            }
            
            values.push(row);
            current_pos = end + 1;
        } else {
            break;
        }
    }

    Ok(values)
}

fn parse_update_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    let set_pos = upper_input.find(" SET ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "UPDATE statement must contain SET clause",
            input,
            1,
            1,
        ))?;

    let table_part = &input[6..set_pos].trim();
    let table = table_part.split_whitespace().next()
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing table name in UPDATE",
            input,
            1,
            1,
        ))?.to_string();

    let where_pos = upper_input.find(" WHERE ");
    let set_end = where_pos.unwrap_or(input.len());
    let set_str = &input[set_pos + 5..set_end];
    let assignments = parse_assignments(set_str)?;

    let where_clause = if let Some(where_pos) = where_pos {
        Some(expression::parse_simple_expression(&input[where_pos + 7..])?)  
    } else {
        None
    };

    Ok(Statement::Update(UpdateStatement {
        table,
        assignments,
        where_clause,
    }))
}

fn parse_assignments(input: &str) -> Result<Vec<Assignment>> {
    let mut assignments = Vec::new();
    
    for assignment_str in input.split(',') {
        let assignment_str = assignment_str.trim();
        let eq_pos = assignment_str.find('=')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Assignment must contain =",
                input,
                1,
                1,
            ))?;
        
        let column = assignment_str[..eq_pos].trim().to_string();
        let value = expression::parse_simple_expression(&assignment_str[eq_pos + 1..])?;
        
        assignments.push(Assignment { column, value });
    }
    
    Ok(assignments)
}

fn parse_delete_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    let from_pos = upper_input.find(" FROM ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "DELETE statement must contain FROM clause",
            input,
            1,
            1,
        ))?;

    let where_pos = upper_input.find(" WHERE ");
    let from_end = where_pos.unwrap_or(input.len());
    let table_str = &input[from_pos + 6..from_end];
    let table = table_str.trim().split_whitespace().next()
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing table name in DELETE FROM",
            input,
            1,
            1,
        ))?.to_string();

    let where_clause = if let Some(where_pos) = where_pos {
        Some(expression::parse_simple_expression(&input[where_pos + 7..])?)  
    } else {
        None
    };

    Ok(Statement::Delete(DeleteStatement {
        table,
        where_clause,
    }))
}
