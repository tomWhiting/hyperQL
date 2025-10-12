//! Integration tests for geometric query syntax

use hyperQL::{parse_statement, Compiler};

#[test]
fn test_parse_near_query() {
    let query = "SELECT * FROM docs WHERE position NEAR origin WITHIN 2.0";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse NEAR query: {:?}", result.err());
    
    let statement = result.unwrap();
    // Verify it compiles
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement);
    assert!(compiled.is_ok(), "Failed to compile NEAR query: {:?}", compiled.err());
}

#[test]
fn test_parse_knn_with_limit() {
    let query = "SELECT * FROM docs ORDER BY position DISTANCE FROM origin LIMIT 10";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse k-NN query: {:?}", result.err());
    
    let statement = result.unwrap();
    // Verify it compiles
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement);
    assert!(compiled.is_ok(), "Failed to compile k-NN query: {:?}", compiled.err());
}

#[test]
fn test_near_with_entity_reference() {
    let query = "SELECT * FROM docs WHERE position NEAR entity_123 WITHIN 5.5";
    let result = parse_statement(query);
    assert!(result.is_ok());
}

#[test]
fn test_distance_with_entity_reference() {
    let query = "SELECT id, name FROM docs ORDER BY position DISTANCE FROM entity_456 LIMIT 20";
    let result = parse_statement(query);
    assert!(result.is_ok());
}

#[test]
fn test_near_case_insensitive() {
    let query = "SELECT * FROM docs WHERE POSITION near Origin within 2.0";
    let result = parse_statement(query);
    assert!(result.is_ok());
}

#[test]
fn test_distance_case_insensitive() {
    let query = "SELECT * FROM docs ORDER BY POSITION distance from Origin LIMIT 10";
    let result = parse_statement(query);
    assert!(result.is_ok());
}

#[test]
fn test_near_with_float_literal() {
    let query = "SELECT * FROM docs WHERE position NEAR origin WITHIN 3.14159";
    let result = parse_statement(query);
    assert!(result.is_ok());
}

#[test]
fn test_combined_query() {
    let query = "SELECT id, title FROM documents WHERE position NEAR origin WITHIN 5.0 ORDER BY position DISTANCE FROM origin LIMIT 100";
    let result = parse_statement(query);
    assert!(result.is_ok());
}

#[test]
fn test_near_missing_within_parses_but_not_as_geometric() {
    let query = "SELECT * FROM docs WHERE position NEAR origin";
    let result = parse_statement(query);
    // This parses but won't be recognized as a geometric NEAR expression
    // (it becomes a regular comparison instead)
    assert!(result.is_ok());
}

#[test]
fn test_distance_missing_from_parses_but_not_as_geometric() {
    let query = "SELECT * FROM docs ORDER BY position";
    let result = parse_statement(query);
    // This parses but won't be recognized as a geometric DISTANCE expression
    // (it becomes a regular column reference instead)
    assert!(result.is_ok());
}
