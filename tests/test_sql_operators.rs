use hyperQL::parser::parse_statement;
use hyperQL::ast::{Statement, BinaryOperator, UnaryOperator, Expression};

#[test]
fn test_parse_in_operator() {
    let query = "SELECT * FROM users WHERE age IN (25, 30, 35)";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse IN operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some(), "WHERE clause should exist");
        if let Some(Expression::Binary { op, .. }) = select.where_clause {
            assert_eq!(op, BinaryOperator::In, "Operator should be IN");
        } else {
            panic!("Expected Binary expression with IN operator");
        }
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_parse_not_in_operator() {
    let query = "SELECT * FROM users WHERE status NOT IN ('active', 'pending')";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse NOT IN operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Binary { op, .. }) = select.where_clause {
            assert_eq!(op, BinaryOperator::NotIn, "Operator should be NOT IN");
        } else {
            panic!("Expected Binary expression with NOT IN operator");
        }
    }
}

#[test]
fn test_parse_like_operator() {
    let query = "SELECT * FROM users WHERE name LIKE 'A%'";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse LIKE operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Binary { op, .. }) = select.where_clause {
            assert_eq!(op, BinaryOperator::Like, "Operator should be LIKE");
        } else {
            panic!("Expected Binary expression with LIKE operator");
        }
    }
}

#[test]
fn test_parse_not_like_operator() {
    let query = "SELECT * FROM users WHERE code NOT LIKE 'test%'";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse NOT LIKE operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Binary { op, .. }) = select.where_clause {
            assert_eq!(op, BinaryOperator::NotLike, "Operator should be NOT LIKE");
        } else {
            panic!("Expected Binary expression with NOT LIKE operator");
        }
    }
}

#[test]
fn test_parse_between_operator() {
    let query = "SELECT * FROM users WHERE age BETWEEN 18 AND 65";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse BETWEEN operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Between { negated, .. }) = select.where_clause {
            assert!(!negated, "BETWEEN should not be negated");
        } else {
            panic!("Expected BETWEEN expression");
        }
    }
}

#[test]
fn test_parse_not_between_operator() {
    let query = "SELECT * FROM users WHERE age NOT BETWEEN 18 AND 65";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse NOT BETWEEN operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Between { negated, .. }) = select.where_clause {
            assert!(negated, "NOT BETWEEN should be negated");
        } else {
            panic!("Expected BETWEEN expression");
        }
    }
}

#[test]
fn test_parse_is_null_operator() {
    let query = "SELECT * FROM users WHERE email IS NULL";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse IS NULL operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Unary { op, .. }) = select.where_clause {
            assert_eq!(op, UnaryOperator::IsNull, "Operator should be IS NULL");
        } else {
            panic!("Expected Unary expression with IS NULL operator");
        }
    }
}

#[test]
fn test_parse_is_not_null_operator() {
    let query = "SELECT * FROM users WHERE updated_at IS NOT NULL";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse IS NOT NULL operator: {:?}", result.err());

    if let Ok(Statement::Select(select)) = result {
        assert!(select.where_clause.is_some());
        if let Some(Expression::Unary { op, .. }) = select.where_clause {
            assert_eq!(op, UnaryOperator::IsNotNull, "Operator should be IS NOT NULL");
        } else {
            panic!("Expected Unary expression with IS NOT NULL operator");
        }
    }
}

#[test]
fn test_parse_like_with_wildcards() {
    let queries = vec![
        "SELECT * FROM users WHERE name LIKE 'A%'",        // prefix
        "SELECT * FROM users WHERE email LIKE '%@gmail.com'", // suffix
        "SELECT * FROM users WHERE desc LIKE '%test%'",    // contains
        "SELECT * FROM codes WHERE code LIKE 'A_C'",       // single char
    ];

    for query in queries {
        let result = parse_statement(query);
        assert!(result.is_ok(), "Failed to parse: {}\nError: {:?}", query, result.err());
    }
}

#[test]
fn test_parse_in_with_various_types() {
    let queries = vec![
        "SELECT * FROM users WHERE age IN (25, 30, 35)",                    // integers
        "SELECT * FROM users WHERE name IN ('Alice', 'Bob', 'Charlie')",    // strings
        "SELECT * FROM products WHERE price IN (9.99, 19.99, 29.99)",      // floats
    ];

    for query in queries {
        let result = parse_statement(query);
        assert!(result.is_ok(), "Failed to parse: {}\nError: {:?}", query, result.err());
    }
}

#[test]
fn test_parse_complex_conditions() {
    let queries = vec![
        "SELECT * FROM users WHERE age BETWEEN 18 AND 65 AND name LIKE 'A%'",
        "SELECT * FROM users WHERE status IN ('active', 'pending') AND updated_at IS NOT NULL",
        "SELECT * FROM products WHERE price NOT BETWEEN 10.0 AND 50.0 OR category LIKE '%sale%'",
        "SELECT * FROM users WHERE email IS NULL OR name NOT IN ('Admin', 'System')",
    ];

    for query in queries {
        let result = parse_statement(query);
        assert!(result.is_ok(), "Failed to parse: {}\nError: {:?}", query, result.err());
    }
}

#[test]
fn test_between_with_expressions() {
    let query = "SELECT * FROM events WHERE timestamp BETWEEN created_at AND updated_at";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse BETWEEN with column expressions: {:?}", result.err());
}

#[test]
fn test_in_empty_list() {
    let query = "SELECT * FROM users WHERE id IN ()";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse IN with empty list: {:?}", result.err());
}
