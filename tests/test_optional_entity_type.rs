use hyperQL::parser::parse_statement;
use hyperQL::ast::{Statement, FromClause};

#[test]
fn test_from_collection_only() {
    let query = "SELECT * FROM mimic";
    let result = parse_statement(query);
    
    assert!(result.is_ok(), "Should parse collection-only FROM clause");
    
    if let Ok(Statement::Select(select)) = result {
        if let Some(FromClause::Table { collection, entity_type, .. }) = &select.from {
            assert_eq!(collection, "mimic");
            assert_eq!(*entity_type, None, "entity_type should be None for collection-only queries");
        } else {
            panic!("Expected FromClause::Table");
        }
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_from_collection_and_type() {
    let query = "SELECT * FROM mimic.Patient";
    let result = parse_statement(query);
    
    assert!(result.is_ok(), "Should parse collection.type FROM clause");
    
    if let Ok(Statement::Select(select)) = result {
        if let Some(FromClause::Table { collection, entity_type, .. }) = &select.from {
            assert_eq!(collection, "mimic");
            assert_eq!(*entity_type, Some("Patient".to_string()), "entity_type should be Some(Patient)");
        } else {
            panic!("Expected FromClause::Table");
        }
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_from_collection_with_where() {
    let query = "SELECT * FROM mimic WHERE age > 18";
    let result = parse_statement(query);
    
    assert!(result.is_ok(), "Should parse collection-only FROM with WHERE clause");
    
    if let Ok(Statement::Select(select)) = result {
        if let Some(FromClause::Table { collection, entity_type, .. }) = &select.from {
            assert_eq!(collection, "mimic");
            assert_eq!(*entity_type, None);
        } else {
            panic!("Expected FromClause::Table");
        }
        assert!(select.where_clause.is_some(), "Should have WHERE clause");
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_from_collection_with_limit() {
    let query = "SELECT * FROM documents LIMIT 10";
    let result = parse_statement(query);
    
    assert!(result.is_ok(), "Should parse collection-only FROM with LIMIT");
    
    if let Ok(Statement::Select(select)) = result {
        if let Some(FromClause::Table { collection, entity_type, .. }) = &select.from {
            assert_eq!(collection, "documents");
            assert_eq!(*entity_type, None);
        } else {
            panic!("Expected FromClause::Table");
        }
        assert_eq!(select.limit, Some(10), "Should have LIMIT 10");
    } else {
        panic!("Expected SELECT statement");
    }
}
