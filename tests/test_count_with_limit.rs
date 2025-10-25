use hyperQL::parser::parse_statement;
use hyperQL::compiler::Compiler;
use hyperQL::executor::{Executor, MemoryDataSource, DataSource};
use hyperQL::types::{Entity, EntityId, PropertyName, Value};

#[test]
fn test_count_with_limit_returns_total_count() {
    let mut data_source = MemoryDataSource::new();

    // Create 100 entities
    let mut entities = Vec::new();
    for i in 0..100 {
        entities.push(Entity {
            id: EntityId(format!("entity_{}", i)),
            properties: vec![
                (PropertyName("name".to_string()), Value::String(format!("Entity {}", i))),
                (PropertyName("index".to_string()), Value::Int(i)),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        });
    }

    data_source.insert("test", entities).unwrap();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    // Test 1: COUNT(*) without LIMIT should return 100
    let query = "SELECT COUNT(*) FROM test.test";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();
    let result = executor.execute(compiled).unwrap();

    assert_eq!(result.rows.len(), 1, "Should return exactly 1 row");
    let count_value = result.rows[0].columns.get("COUNT(*)").expect("Should have COUNT(*) column");
    assert_eq!(count_value, &Value::Int(100), "COUNT(*) without LIMIT should return 100");

    // Test 2: COUNT(*) with LIMIT 1 should STILL return 100 (not 1)
    // LIMIT applies to result rows (always 1 for COUNT), not to the count itself
    let query_with_limit = "SELECT COUNT(*) FROM test.test LIMIT 1";
    let statement_with_limit = parse_statement(query_with_limit).unwrap();
    let compiled_with_limit = compiler.compile(statement_with_limit).unwrap();
    let result_with_limit = executor.execute(compiled_with_limit).unwrap();

    assert_eq!(result_with_limit.rows.len(), 1, "Should return exactly 1 row");
    let count_value_with_limit = result_with_limit.rows[0].columns.get("COUNT(*)").expect("Should have COUNT(*) column");
    assert_eq!(count_value_with_limit, &Value::Int(100), "COUNT(*) with LIMIT should STILL return 100 (total count)");

    // Test 3: COUNT(*) with LIMIT 10 should STILL return 100
    let query_with_limit_10 = "SELECT COUNT(*) FROM test.test LIMIT 10";
    let statement_with_limit_10 = parse_statement(query_with_limit_10).unwrap();
    let compiled_with_limit_10 = compiler.compile(statement_with_limit_10).unwrap();
    let result_with_limit_10 = executor.execute(compiled_with_limit_10).unwrap();

    assert_eq!(result_with_limit_10.rows.len(), 1, "Should return exactly 1 row");
    let count_value_with_limit_10 = result_with_limit_10.rows[0].columns.get("COUNT(*)").expect("Should have COUNT(*) column");
    assert_eq!(count_value_with_limit_10, &Value::Int(100), "COUNT(*) with LIMIT 10 should STILL return 100 (total count)");

    println!("All COUNT with LIMIT tests passed!");
}

#[test]
fn test_count_as_alias_with_limit() {
    let mut data_source = MemoryDataSource::new();

    // Create 50 entities
    let mut entities = Vec::new();
    for i in 0..50 {
        entities.push(Entity {
            id: EntityId(format!("doc_{}", i)),
            properties: vec![
                (PropertyName("title".to_string()), Value::String(format!("Document {}", i))),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        });
    }

    data_source.insert("docs", entities).unwrap();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    // Test with alias and LIMIT
    let query = "SELECT COUNT(*) AS total FROM docs.docs LIMIT 5";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();
    let result = executor.execute(compiled).unwrap();

    assert_eq!(result.rows.len(), 1, "Should return exactly 1 row");
    let total_value = result.rows[0].columns.get("total").expect("Should have 'total' column");
    assert_eq!(total_value, &Value::Int(50), "COUNT(*) AS total with LIMIT should return 50 (total count)");

    println!("COUNT with alias and LIMIT test passed!");
}
