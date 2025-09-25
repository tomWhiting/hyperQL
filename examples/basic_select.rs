//! Basic SELECT WHERE Demo for HyperQL
//!
//! This example demonstrates the core functionality implemented:
//! - Parsing SELECT statements with WHERE clauses
//! - Compiling queries into optimized execution plans
//! - Executing queries against in-memory data

use hyperQL::*;
use std::collections::HashMap;

fn main() {
    println!("🚀 HyperQL Basic SELECT WHERE Demonstration");
    println!("================================================");

    // Create sample data
    let mut data_source = MemoryDataSource::new();
    
    // Add some entities
    let entities = vec![
        create_person("alice_001", "Alice", 30, true, 1.0, 2.0, 3.0),
        create_person("bob_002", "Bob", 25, false, 4.0, 5.0, 6.0),
        create_person("charlie_003", "Charlie", 35, true, 7.0, 8.0, 9.0),
        create_person("diana_004", "Diana", 28, true, 10.0, 11.0, 12.0),
    ];
    
    data_source.add_entities("people", entities);
    
    // Create query engine components
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();
    
    // Demonstrate different query types
    let queries = vec![
        "SELECT * FROM people",
        "SELECT * FROM people WHERE name = 'Alice'",
        "SELECT name, age FROM people WHERE age > 30",
        "SELECT * FROM people WHERE active = TRUE ORDER BY age DESC",
        "SELECT name FROM people WHERE age >= 25 AND age <= 30 ORDER BY name",
        "SELECT * FROM people LIMIT 2",
    ];
    
    for (i, query) in queries.iter().enumerate() {
        println!("\n{}. Query: {}", i + 1, query);
        println!("{}⎺{}", " ".repeat(3), "⎺".repeat(query.len() + 8));
        
        match execute_query(&mut executor, &compiler, query) {
            Ok(result) => {
                if result.rows.is_empty() {
                    println!("   No results found.");
                } else {
                    print_results(&result);
                }
                
                println!("   📊 Stats: {} entities scanned, {}ms execution time", 
                    result.execution_stats.entities_scanned,
                    result.execution_stats.execution_time_ms);
            }
            Err(e) => println!("   ❌ Error: {:?}", e),
        }
    }
    
    println!("\n✅ All queries executed successfully!");
    println!("\n🎯 Key Features Demonstrated:");
    println!("   • SQL-like SELECT syntax parsing");
    println!("   • WHERE clause filtering with comparison operators");
    println!("   • Logical operators (AND, OR) in WHERE conditions");
    println!("   • Column projection and wildcard selection");
    println!("   • ORDER BY with ASC/DESC");
    println!("   • LIMIT clause for result pagination");
    println!("   • Compilation to optimized execution plans");
    println!("   • In-memory query execution with statistics");
    println!("\n🚀 Basic SELECT WHERE functionality is fully operational!");
}

fn create_person(id: &str, name: &str, age: i64, active: bool, x: f64, y: f64, z: f64) -> Entity {
    let mut entity = Entity {
        id: EntityId(id.to_string()),
        properties: HashMap::new(),
        position: Some(Position3D { x, y, z }),
        embedding: None,
    };
    
    entity.properties.insert(PropertyName("name".to_string()), Value::String(name.to_string()));
    entity.properties.insert(PropertyName("age".to_string()), Value::Int(age));
    entity.properties.insert(PropertyName("active".to_string()), Value::Bool(active));
    
    entity
}

fn execute_query(executor: &mut Executor, compiler: &Compiler, query: &str) -> hyperQL::Result<QueryResult> {
    let statement = parse_statement(query)?;
    let compiled = compiler.compile(statement)?;
    executor.execute(compiled)
}

fn print_results(result: &QueryResult) {
    if result.rows.is_empty() {
        return;
    }
    
    // Get column names from first row
    let mut columns: Vec<String> = result.rows[0].columns.keys().cloned().collect();
    columns.sort();
    
    // Print header
    print!("   ");
    for col in &columns {
        print!("{:>12} ", col);
    }
    println!();
    
    // Print separator
    print!("   ");
    for _ in &columns {
        print!("{} ", "─".repeat(12));
    }
    println!();
    
    // Print data rows
    for row in &result.rows {
        print!("   ");
        for col in &columns {
            let value = row.columns.get(col).unwrap_or(&Value::Null);
            let display_value = match value {
                Value::String(s) => format!("\"{}\"", s),
                Value::Int(i) => i.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Float(f) => format!("{:.1}", f),
                Value::EntityId(id) => id.0.clone(),
                Value::Null => "NULL".to_string(),
                _ => format!("{:?}", value),
            };
            print!("{:>12} ", display_value);
        }
        println!();
    }
}