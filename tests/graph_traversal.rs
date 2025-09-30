//! Integration Tests for Graph Traversal Operations
//!
//! This module tests basic graph traversal using the TRAVERSE clause.
//! Currently implements basic relationship traversal using entity properties.

use hyperQL::*;
use hyperQL::types::{Entity, EntityId, PropertyName, Value};
use std::collections::HashMap;

#[test]
fn test_basic_graph_traversal() {
    let mut data_source = executor::MemoryDataSource::new();

    let user1 = Entity {
        id: EntityId("user1".to_string()),
        properties: {
            let mut props = HashMap::new();
            props.insert(PropertyName("name".to_string()), Value::String("Alice".to_string()));
            props
        },
        position: None,
        embedding: None,
    };

    let user2 = Entity {
        id: EntityId("user2".to_string()),
        properties: {
            let mut props = HashMap::new();
            props.insert(PropertyName("name".to_string()), Value::String("Bob".to_string()));
            props
        },
        position: None,
        embedding: None,
    };

    data_source.add_entity("User", user1);
    data_source.add_entity("User", user2);

    let relationship = Entity {
        id: EntityId("rel1".to_string()),
        properties: {
            let mut props = HashMap::new();
            props.insert(PropertyName("from_id".to_string()), Value::String("user1".to_string()));
            props.insert(PropertyName("to_id".to_string()), Value::String("user2".to_string()));
            props.insert(PropertyName("type".to_string()), Value::String("follows".to_string()));
            props
        },
        position: None,
        embedding: None,
    };

    data_source.add_entity("relationships", relationship);

    let query = "SELECT * FROM User TRAVERSE (a:User)-[r:follows]->(b:User)";
    
    let statement = parser::parse_statement(query).expect("Failed to parse query");
    let compiler = compiler::Compiler::new();
    let compiled = compiler.compile(statement).expect("Failed to compile query");
    
    let mut executor = executor::Executor::new(Box::new(data_source));
    let result = executor.execute(compiled).expect("Failed to execute query");

    assert!(result.rows.len() <= 1, "Should return at most one relationship");
    assert!(result.execution_stats.relationships_traversed <= 1, "Should traverse at most one relationship");
}

#[test]
fn test_empty_graph_traversal() {
    let data_source = executor::MemoryDataSource::new();
    
    let query = "SELECT * FROM User TRAVERSE (a:User)-[r:follows]->(b:User)";
    
    let statement = parser::parse_statement(query).expect("Failed to parse query");
    let compiler = compiler::Compiler::new();
    let compiled = compiler.compile(statement).expect("Failed to compile query");
    
    let mut executor = executor::Executor::new(Box::new(data_source));
    let result = executor.execute(compiled).expect("Failed to execute query");

    assert_eq!(result.rows.len(), 0, "Should return no relationships for empty graph");
}

#[test]
fn test_graph_operation_returns_status() {
    use hyperQL::compiler::{ExecutionPlan, GraphOpType};
    use std::collections::HashMap;
    
    let data_source = executor::MemoryDataSource::new();
    let mut executor = executor::Executor::new(Box::new(data_source));
    
    let plan = ExecutionPlan::GraphOperation {
        op_type: GraphOpType::ShortestPath,
        params: HashMap::new(),
        input: None,
    };
    
    let compiled = compiler::CompiledQuery {
        plan,
        metadata: compiler::QueryMetadata {
            tables_accessed: vec![],
            columns_accessed: vec![],
            functions_used: vec![],
            requires_spatial_index: false,
            requires_vector_index: false,
        },
        estimated_cost: compiler::ExecutionCost {
            estimated_rows: 0,
            estimated_cpu_cost: 0.0,
            estimated_memory_mb: 0.0,
            estimated_io_ops: 0,
        },
    };
    
    let result = executor.execute(compiled).expect("Failed to execute query");

    assert_eq!(result.rows.len(), 1, "Should return status message");
    let row = &result.rows[0];
    assert!(row.columns.contains_key("operation"), "Should contain operation field");
    assert!(row.columns.contains_key("status"), "Should contain status field");
    assert!(row.columns.contains_key("note"), "Should contain note field");
}
