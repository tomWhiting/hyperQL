use hyperQL::parser::parse_statement;
use hyperQL::compiler::Compiler;
use hyperQL::executor::{Executor, MemoryDataSource};
use hyperQL::ast::Statement;
use hyperQL::types::{Entity, EntityId, PropertyName, Value};
use std::collections::HashMap;

fn create_test_data_source_with_duplicates() -> MemoryDataSource {
    let mut data_source = MemoryDataSource::new();
    
    let mut entities = Vec::new();
    
    for i in 0..10 {
        let mut properties = HashMap::new();
        properties.insert(
            PropertyName("name".to_string()),
            Value::String(format!("User{}", i % 3))
        );
        properties.insert(
            PropertyName("department".to_string()),
            Value::String(format!("Dept{}", i % 2))
        );
        properties.insert(
            PropertyName("age".to_string()),
            Value::Int(25 + (i % 3) as i64)
        );
        properties.insert(
            PropertyName("email".to_string()),
            if i % 4 == 0 {
                Value::Null
            } else {
                Value::String(format!("user{}@example.com", i % 3))
            }
        );
        
        entities.push(Entity {
            id: EntityId(format!("entity_{}", i)),
            properties,
            position: None,
            embedding: None,
        });
    }
    
    data_source.add_entities("users", entities);
    data_source
}

#[test]
fn test_parse_distinct_keyword() {
    let query = "SELECT DISTINCT name FROM users";
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse DISTINCT: {:?}", result.err());
    
    if let Ok(Statement::Select(select)) = result {
        assert!(select.distinct, "DISTINCT flag should be true");
        assert_eq!(select.select_list.len(), 1, "Should have one select item");
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_parse_select_without_distinct() {
    let query = "SELECT name FROM users";
    let result = parse_statement(query);
    assert!(result.is_ok());
    
    if let Ok(Statement::Select(select)) = result {
        assert!(!select.distinct, "DISTINCT flag should be false");
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_distinct_removes_duplicate_values() {
    let query = "SELECT DISTINCT name FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 3, "Should have 3 unique names (User0, User1, User2)");
    
    let mut names: Vec<String> = result.rows.iter()
        .map(|row| {
            if let Some(Value::String(name)) = row.columns.get("name") {
                name.clone()
            } else {
                panic!("Expected string name");
            }
        })
        .collect();
    names.sort();
    
    assert_eq!(names, vec!["User0", "User1", "User2"]);
}

#[test]
fn test_distinct_multiple_columns() {
    let query = "SELECT DISTINCT name, department FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 6, "Should have 6 unique (name, department) combinations");
}

#[test]
fn test_distinct_with_nulls() {
    let query = "SELECT DISTINCT email FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 4, "Should have 3 unique emails + 1 NULL");
    
    let null_count = result.rows.iter()
        .filter(|row| row.columns.get("email") == Some(&Value::Null))
        .count();
    
    assert_eq!(null_count, 1, "Should have exactly one NULL entry (deduplicated)");
}

#[test]
fn test_distinct_preserves_order() {
    let query = "SELECT DISTINCT age FROM users ORDER BY age";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 3, "Should have 3 unique ages");
    
    let ages: Vec<i64> = result.rows.iter()
        .map(|row| {
            if let Some(Value::Int(age)) = row.columns.get("age") {
                age.clone()
            } else {
                panic!("Expected integer age");
            }
        })
        .collect();
    
    assert_eq!(ages, vec![25, 26, 27], "Ages should be sorted");
}

#[test]
fn test_distinct_all_columns() {
    let query = "SELECT DISTINCT * FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert!(result.rows.len() <= 10, "Should have at most 10 unique rows");
}

#[test]
fn test_distinct_with_where_clause() {
    let query = "SELECT DISTINCT department FROM users WHERE age > 25";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 2, "Should have 2 unique departments");
}

#[test]
fn test_distinct_with_limit() {
    let query = "SELECT DISTINCT name FROM users LIMIT 2";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 2, "Should return exactly 2 rows due to LIMIT");
}

#[test]
fn test_non_distinct_has_duplicates() {
    let query = "SELECT name FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 10, "Should have all 10 rows without DISTINCT");
}

#[test]
fn test_distinct_with_integer_values() {
    let query = "SELECT DISTINCT age FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();
    
    let data_source = create_test_data_source_with_duplicates();
    let mut executor = Executor::new(Box::new(data_source));
    
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 3, "Should have 3 unique ages (25, 26, 27)");
}

#[test]
fn test_distinct_case_sensitivity() {
    let mut data_source = MemoryDataSource::new();
    
    let mut entities = Vec::new();
    for (i, name) in ["Alice", "alice", "ALICE", "Bob", "bob"].iter().enumerate() {
        let mut properties = HashMap::new();
        properties.insert(PropertyName("name".to_string()), Value::String(name.to_string()));
        
        entities.push(Entity {
            id: EntityId(format!("entity_{}", i)),
            properties,
            position: None,
            embedding: None,
        });
    }
    
    data_source.add_entities("users", entities);
    
    let query = "SELECT DISTINCT name FROM users";
    let statement = parse_statement(query).unwrap();
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(statement).unwrap();

    let mut executor = Executor::new(Box::new(data_source));
    let result = executor.execute(compiled).unwrap();
    
    assert_eq!(result.rows.len(), 5, "Should treat Alice, alice, ALICE as distinct (case-sensitive)");
}
