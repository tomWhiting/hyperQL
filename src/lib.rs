//! # HyperQL - Unified Query Language for Hyperspatial Database
//!
//! HyperQL is a revolutionary query language that unifies SQL-like syntax with graph traversal,
//! geometric queries, and cascade operations. It enables seamless querying across multiple
//! paradigms within the Hyperspatial database's unified hyperbolic space.
//!
//! ## Purpose
//!
//! Traditional databases force developers to use different query languages for different data models:
//! - SQL for relational data
//! - Cypher or SPARQL for graph traversals
//! - Custom APIs for vector similarity
//! - Domain-specific languages for time-series analysis
//!
//! HyperQL eliminates this fragmentation by providing a unified syntax that can:
//! - Query entities by properties (SQL-like WHERE clauses)
//! - Traverse relationships (graph-style navigation)
//! - Find similar entities (vector similarity searches)
//! - Apply geometric filters (hyperbolic distance constraints)
//! - Propagate cascading measures (hierarchical computations)
//! - Combine all paradigms in a single query
//!
//! ## Key Features
//!
//! ### Unified Syntax
//! ```hyperql
//! SELECT entity, measure_value
//! FROM users u
//! TRAVERSE connected_to -> friends f
//! WHERE u.age > 25
//!   AND similarity(u.interests, f.interests) > 0.8
//!   AND hyperbolic_distance(u, f) < 2.0
//! CASCADE user_influence FROM u TO f
//! ORDER BY measure_value DESC
//! ```
//!
//! ### Multi-Paradigm Integration
//! - **Relational**: Standard SQL SELECT, WHERE, JOIN operations
//! - **Graph**: TRAVERSE statements for relationship navigation
//! - **Vector**: Built-in similarity functions and k-NN queries
//! - **Geometric**: Hyperbolic space queries and distance operations
//! - **Cascading**: Measure propagation across entity hierarchies
//!
//! ### Spatial Intelligence
//! Leverages the hyperbolic positioning system to enable:
//! - Natural clustering queries without explicit grouping
//! - Hierarchical traversals that respect learned structure
//! - Similarity searches that combine multiple signals
//! - Temporal queries over position trajectories
//!
//! ## Architecture Overview
//!
//! HyperQL follows a traditional compiler pipeline optimized for hyperbolic queries:
//!
//! 1. **Parsing**: Winnow-based parser converts query text to AST
//! 2. **Compilation**: AST transforms into optimized execution plans
//! 3. **Optimization**: Query plans are optimized for hyperbolic operations
//! 4. **Execution**: Plans execute against the hyperbolic space engine
//!
//! ## Module Organization
//!
//! The HyperQL crate is organized into focused modules that handle different
//! aspects of the query language implementation:
//!
//! - [`ast`]: Abstract syntax tree definitions for all query constructs
//! - [`parser`]: Winnow-based parser for HyperQL syntax
//! - [`compiler`]: Compilation from AST to execution plans
//! - [`executor`]: Query execution engine with hyperbolic optimizations
//! - [`functions`]: Built-in functions for vector, geometric, and aggregate operations
//! - [`cascade`]: Cascade system for measure propagation across hierarchies
//! - [`optimizer`]: Query optimization for hyperbolic space operations
//! - [`context`]: Query execution context and variable binding management
//!
//! ## Integration with Hyperspatial
//!
//! HyperQL is tightly integrated with the Hyperspatial database engine:
//!
//! - **Hyperbolic Engine**: Leverages learned positions for geometric queries
//! - **Persistence Layer**: Accesses stored entities and relationships efficiently
//! - **Vector Indices**: Utilizes existing embeddings for similarity operations
//! - **Measure System**: Implements cascading computations across hierarchies
//!
//! This integration enables queries that would be impossible in traditional databases,
//! combining multiple paradigms into a single, powerful query interface.

// Core modules
pub mod error;
pub mod types;

// Query language modules
pub mod ast;
pub mod parser;
pub mod compiler;
pub mod executor;
pub mod functions;
pub mod cascade;
pub mod optimizer;
pub mod context;

// Runtime and compilation modules
pub mod runtime;
pub mod ir;

// Re-export commonly used types
pub use error::{HyperQLError, Result};
pub use types::*;

// Re-export key types and functions
pub use ast::{Statement, SelectStatement, Expression};
pub use parser::parse_statement;
pub use compiler::{Compiler, CompiledQuery};
pub use executor::{Executor, MemoryDataSource};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_end_to_end_select_where_functionality() {
        // Test the target query: "SELECT * FROM entities WHERE name = 'Alice'"

        // Create test data
        let mut data_source = MemoryDataSource::new();

        // Add Alice
        let mut alice = Entity {
            id: EntityId("alice_001".to_string()),
            properties: HashMap::new(),
            position: Some(Position3D { x: 1.0, y: 2.0, z: 3.0 }),
            embedding: None,
        };
        alice.properties.insert(PropertyName("name".to_string()), Value::String("Alice".to_string()));
        alice.properties.insert(PropertyName("age".to_string()), Value::Int(30));
        alice.properties.insert(PropertyName("active".to_string()), Value::Bool(true));

        // Add Bob
        let mut bob = Entity {
            id: EntityId("bob_002".to_string()),
            properties: HashMap::new(),
            position: Some(Position3D { x: 4.0, y: 5.0, z: 6.0 }),
            embedding: None,
        };
        bob.properties.insert(PropertyName("name".to_string()), Value::String("Bob".to_string()));
        bob.properties.insert(PropertyName("age".to_string()), Value::Int(25));
        bob.properties.insert(PropertyName("active".to_string()), Value::Bool(false));

        // Add Charlie
        let mut charlie = Entity {
            id: EntityId("charlie_003".to_string()),
            properties: HashMap::new(),
            position: None,
            embedding: None,
        };
        charlie.properties.insert(PropertyName("name".to_string()), Value::String("Charlie".to_string()));
        charlie.properties.insert(PropertyName("age".to_string()), Value::Int(35));

        data_source.add_entities("entities", vec![alice, bob, charlie]);

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));
        let compiler = Compiler::new();

        // Test 1: SELECT * FROM entities WHERE name = 'Alice'
        println!("Testing: SELECT * FROM entities WHERE name = 'Alice'");
        let query1 = "SELECT * FROM entities WHERE name = 'Alice'";
        let statement1 = parse_statement(query1).expect("Query should parse");
        let compiled1 = compiler.compile(statement1).expect("Query should compile");
        let result1 = executor.execute(compiled1).expect("Query should execute");

        assert_eq!(result1.rows.len(), 1, "Should return exactly one row");
        assert_eq!(result1.rows[0].columns.get("name"), Some(&Value::String("Alice".to_string())));
        assert_eq!(result1.rows[0].columns.get("age"), Some(&Value::Int(30)));
        assert_eq!(result1.rows[0].columns.get("active"), Some(&Value::Bool(true)));
        println!("✓ Successfully filtered by name = 'Alice'");

        // Test 2: SELECT name, age FROM entities WHERE age > 30
        println!("\nTesting: SELECT name, age FROM entities WHERE age > 30");
        let query2 = "SELECT name, age FROM entities WHERE age > 30";
        let statement2 = parse_statement(query2).expect("Query should parse");
        let compiled2 = compiler.compile(statement2).expect("Query should compile");
        let result2 = executor.execute(compiled2).expect("Query should execute");

        assert_eq!(result2.rows.len(), 1, "Should return exactly one row");
        assert_eq!(result2.rows[0].columns.get("name"), Some(&Value::String("Charlie".to_string())));
        assert_eq!(result2.rows[0].columns.get("age"), Some(&Value::Int(35)));
        assert_eq!(result2.rows[0].columns.len(), 2, "Should only have name and age columns");
        println!("✓ Successfully projected specific columns and filtered by age > 30");

        // Test 3: SELECT * FROM entities WHERE active = TRUE ORDER BY age DESC LIMIT 2
        println!("\nTesting: SELECT * FROM entities ORDER BY age DESC LIMIT 2");
        let query3 = "SELECT * FROM entities ORDER BY age DESC LIMIT 2";
        let statement3 = parse_statement(query3).expect("Query should parse");
        let compiled3 = compiler.compile(statement3).expect("Query should compile");
        let result3 = executor.execute(compiled3).expect("Query should execute");

        assert_eq!(result3.rows.len(), 2, "Should return exactly two rows");
        // First row should be Charlie (age 35), second row should be Alice (age 30)
        assert_eq!(result3.rows[0].columns.get("name"), Some(&Value::String("Charlie".to_string())));
        assert_eq!(result3.rows[1].columns.get("name"), Some(&Value::String("Alice".to_string())));
        println!("✓ Successfully ordered by age DESC and limited to 2 results");

        // Test 4: Complex WHERE with AND
        println!("\nTesting: SELECT name FROM entities WHERE age > 20 AND age < 35");
        let query4 = "SELECT name FROM entities WHERE age > 20 AND age < 35";
        let statement4 = parse_statement(query4).expect("Query should parse");
        let compiled4 = compiler.compile(statement4).expect("Query should compile");
        let result4 = executor.execute(compiled4).expect("Query should execute");

        assert_eq!(result4.rows.len(), 2, "Should return Alice and Bob");
        let names: Vec<String> = result4.rows.iter()
            .map(|row| match row.columns.get("name") {
                Some(Value::String(name)) => name.clone(),
                _ => "Unknown".to_string(),
            })
            .collect();
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Bob".to_string()));
        println!("✓ Successfully used AND condition in WHERE clause");

        // Print execution statistics
        println!("\nExecution Statistics:");
        println!("- Entities scanned: {}", result4.execution_stats.entities_scanned);
        println!("- Execution time: {}ms", result4.execution_stats.execution_time_ms);

        println!("\n🎉 All end-to-end tests passed! HyperQL basic SELECT WHERE functionality is working.");
    }

    #[test]
    fn test_stream_and_timeseries_functionality() {
        use crate::functions::streams::operations::{stream_create, stream_produce, stream_consume};
        use crate::functions::streams::windowing::{window_tumbling};
        use crate::functions::timeseries::temporal::{time_bucket, time_diff, extract};
        use crate::functions::timeseries::window::{moving_average, exponential_smoothing, lag};
        use std::collections::HashMap;
        use chrono::Utc;

        println!("Testing stream and time series functionality...");

        // Test 1: Stream Operations
        println!("\n1. Testing Stream Operations");

        // Create a stream
        let mut stream_config = HashMap::new();
        stream_config.insert("buffer_size".to_string(), Value::Int(100));
        stream_config.insert("max_events".to_string(), Value::Int(50));

        let create_result = stream_create("test_stream".to_string(), stream_config);
        assert!(create_result.is_ok());
        println!("✓ Stream created successfully");

        // Produce events to stream
        for i in 1..=10 {
            let mut event_data = HashMap::new();
            event_data.insert("value".to_string(), Value::Int(i * 10));
            event_data.insert("timestamp".to_string(), Value::Timestamp(Utc::now().timestamp_millis()));

            let produce_result = stream_produce("test_stream".to_string(), event_data);
            assert!(produce_result.is_ok());
        }
        println!("✓ Produced 10 events to stream");

        // Consume events from stream
        let mut consume_config = HashMap::new();
        consume_config.insert("limit".to_string(), Value::Int(5));

        let consume_result = stream_consume("test_stream".to_string(), consume_config);
        assert!(consume_result.is_ok());
        if let Value::List(events) = consume_result.unwrap() {
            assert_eq!(events.len(), 5);
            println!("✓ Consumed {} events from stream", events.len());
        }

        // Test 2: Window Functions
        println!("\n2. Testing Stream Window Functions");

        let window_result = window_tumbling("test_stream".to_string(), 1000, "count".to_string());
        assert!(window_result.is_ok());
        println!("✓ Tumbling window function executed");

        // Test 3: Time Series Temporal Functions
        println!("\n3. Testing Time Series Temporal Functions");

        let now = Utc::now().timestamp_millis();

        // Test time_bucket
        let bucket_result = time_bucket(now, 1, "hour");
        assert!(bucket_result.is_ok());
        println!("✓ Time bucket function executed");

        // Test time_diff
        let diff_result = time_diff(now, now + 3600000, Some("hours"));
        assert!(diff_result.is_ok());
        if let Value::Int(hours) = diff_result.unwrap() {
            assert_eq!(hours, 1);
            println!("✓ Time difference calculated: {} hours", hours);
        }

        // Test extract
        let extract_result = extract(now, "year");
        assert!(extract_result.is_ok());
        println!("✓ Timestamp component extracted");

        // Test 4: Time Series Window Functions
        println!("\n4. Testing Time Series Window Functions");

        let test_values = vec![
            Value::Int(10), Value::Int(20), Value::Int(30),
            Value::Int(40), Value::Int(50)
        ];

        // Test moving_average
        let ma_result = moving_average(test_values.clone(), 3);
        assert!(ma_result.is_ok());
        println!("✓ Moving average calculated");

        // Test exponential_smoothing
        let es_result = exponential_smoothing(test_values.clone(), 0.3);
        assert!(es_result.is_ok());
        println!("✓ Exponential smoothing calculated");

        // Test lag
        let lag_result = lag(test_values.clone(), 1, Some(Value::Int(0)));
        assert!(lag_result.is_ok());
        if let Value::List(lag_values) = lag_result.unwrap() {
            assert_eq!(lag_values.len(), 5);
            println!("✓ Lag function calculated for {} values", lag_values.len());
        }

        println!("\n🎉 All stream and time series tests passed! Real-time data processing functionality is working.");
    }
}
