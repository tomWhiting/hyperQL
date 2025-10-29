use hyperQL::parser::parse_statement;
use hyperQL::compiler::Compiler;
use hyperQL::executor::{Executor, MemoryDataSource, DataSource};
use hyperQL::types::{Entity, EntityId, PropertyName, Value};

#[test]
fn test_count_star_column_name() {
    let mut data_source = MemoryDataSource::new();
    
    let entities = vec![
        Entity {
            id: EntityId("e1".to_string()),
            properties: vec![
                (PropertyName("name".to_string()), Value::String("Alice".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("e2".to_string()),
            properties: vec![
                (PropertyName("name".to_string()), Value::String("Bob".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
    ];
    
    data_source.insert("test", entities).unwrap();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();
    
    let query = "SELECT COUNT(*) FROM test.test";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();
    
    let result = executor.execute(compiled).unwrap();
    
    println!("Column names: {:?}", result.column_names);
    
    // Check what the actual column name is
    if !result.rows.is_empty() {
        let row = &result.rows[0];
        println!("Column keys in result: {:?}", row.columns.keys().collect::<Vec<_>>());
        for (k, v) in &row.columns {
            println!("  '{}': {:?}", k, v);
        }
        
        // Fixed: column is now properly named "COUNT(*)" as expected
        assert!(row.columns.contains_key("COUNT(*)"), "Should have COUNT(*) column");
    }
}
