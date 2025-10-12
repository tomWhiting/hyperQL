use hyperQL::{parse_statement, Compiler, Executor, MemoryDataSource};
use hyperQL::types::{Entity, EntityId, PropertyName, Value};

#[test]
fn test_variable_length_traverse() {
    let mut datasource = MemoryDataSource::new();
    
    // Create entities: A -> B -> C
    let entity_a = Entity {
        id: EntityId("a".to_string()),
        properties: vec![(PropertyName("name".to_string()), Value::String("A".to_string()))].into_iter().collect(),
        position: None,
        embedding: None,
    };
    
    let entity_b = Entity {
        id: EntityId("b".to_string()),
        properties: vec![(PropertyName("name".to_string()), Value::String("B".to_string()))].into_iter().collect(),
        position: None,
        embedding: None,
    };
    
    let entity_c = Entity {
        id: EntityId("c".to_string()),
        properties: vec![(PropertyName("name".to_string()), Value::String("C".to_string()))].into_iter().collect(),
        position: None,
        embedding: None,
    };
    
    datasource.add_entities("nodes", vec![entity_a, entity_b, entity_c]);
    
    // Create relationships
    let rel_ab = Entity {
        id: EntityId("rel_ab".to_string()),
        properties: vec![
            (PropertyName("from_id".to_string()), Value::String("a".to_string())),
            (PropertyName("to_id".to_string()), Value::String("b".to_string())),
            (PropertyName("type".to_string()), Value::String("knows".to_string())),
        ].into_iter().collect(),
        position: None,
        embedding: None,
    };
    
    let rel_bc = Entity {
        id: EntityId("rel_bc".to_string()),
        properties: vec![
            (PropertyName("from_id".to_string()), Value::String("b".to_string())),
            (PropertyName("to_id".to_string()), Value::String("c".to_string())),
            (PropertyName("type".to_string()), Value::String("knows".to_string())),
        ].into_iter().collect(),
        position: None,
        embedding: None,
    };
    
    datasource.add_entities("relationships", vec![rel_ab, rel_bc]);
    
    // Test variable-length traversal
    let query = "SELECT * FROM nodes TRAVERSE (a:nodes)-[r:knows*1..2]->(b)";
    let statement = parse_statement(query).unwrap();

    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let mut executor = Executor::new(Box::new(datasource));
    let result = executor.execute(compiled).unwrap();
    
    // Should find B (1 hop) and C (2 hops) from A
    assert!(result.rows.len() >= 2, "Should find at least 2 paths, found {}", result.rows.len());
    println!("Variable-length traverse test passed! Found {} paths", result.rows.len());
}

#[test]
fn test_optional_traverse() {
    let mut datasource = MemoryDataSource::new();
    
    // Create entity A with no relationships
    let entity_a = Entity {
        id: EntityId("a".to_string()),
        properties: vec![(PropertyName("name".to_string()), Value::String("A".to_string()))].into_iter().collect(),
        position: None,
        embedding: None,
    };
    
    datasource.add_entities("nodes", vec![entity_a]);
    
    // Test optional traversal
    let query = "SELECT * FROM nodes TRAVERSE (a:nodes)-[r:knows?]->(b)";
    let statement = parse_statement(query).unwrap();

    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let mut executor = Executor::new(Box::new(datasource));
    let result = executor.execute(compiled).unwrap();
    
    // Should return 1 row with NULL end node
    assert_eq!(result.rows.len(), 1, "Should return 1 row with NULL end node");
    assert_eq!(result.rows[0].columns.get("b").unwrap(), &Value::Null, "End node should be NULL");
    println!("Optional traverse test passed!");
}
