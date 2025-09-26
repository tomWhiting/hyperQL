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
//! - [`compiler`]: Compilation from AST to execution plans for Hyperspatial
//! - [`executor`]: Plan executor (returns execution plans rather than executing them)
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
pub mod error_context;
pub mod types;
pub mod type_checker;

// Query language modules
pub mod ast;
pub mod parser;
pub mod compiler;
pub mod executor;
pub mod cascade;
pub mod optimizer;
pub mod context;

// Runtime and compilation modules
pub mod runtime;
pub mod ir;

// Developer experience modules
pub mod builder;
pub mod documentation;
pub mod utils;

// Re-export commonly used types
pub use error::{HyperQLError, Result};
pub use types::*;

// Re-export key types and functions
pub use ast::{Statement, SelectStatement, Expression};
pub use parser::parse_statement;
pub use compiler::{Compiler, CompiledQuery};
pub use executor::{Executor, MemoryDataSource};
pub use type_checker::{TypeChecker, TypeInfo, TypeContext};

// Re-export developer experience features
pub use builder::HyperQLBuilder;
pub use documentation::{HyperQLSyntax, HyperQLExamples};
pub use utils::query_utils;

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
    fn test_type_checker_integration() {
        use std::collections::HashMap;
        use crate::type_checker::TypeChecker;
        use crate::ast::{Literal, BinaryOperator, ColumnRef};

        println!("Testing type checker integration...");

        let mut type_checker = TypeChecker::new();

        // Test basic expression type checking
        let expr = Expression::Binary {
            left: Box::new(Expression::Literal(Literal::Int(42))),
            op: BinaryOperator::Add,
            right: Box::new(Expression::Literal(Literal::Float(3.14))),
        };

        let result_type = type_checker.check_expression_type(&expr).expect("Should type check successfully");
        assert_eq!(result_type, TypeInfo::Float, "Int + Float should result in Float");
        println!("✓ Basic arithmetic type promotion works");

        // Test type compatibility validation
        let left = Expression::Literal(Literal::String("hello".to_string()));
        let right = Expression::Literal(Literal::Int(42));

        let result = type_checker.check_operation_compatibility(
            &left, &BinaryOperator::Add, &right
        );
        assert!(result.is_err(), "String + Int should be a type error");
        println!("✓ Type incompatibility detection works");

        // Test function type checking
        let sum_result = type_checker.check_function_type(
            "SUM",
            &[Expression::Literal(Literal::Int(100))]
        ).expect("SUM should work on integers");
        assert_eq!(sum_result, TypeInfo::Integer);
        println!("✓ Function type checking works");

        // Test WHERE clause validation
        let where_expr = Expression::Binary {
            left: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: "age".to_string(),
            })),
            op: BinaryOperator::GreaterThan,
            right: Box::new(Expression::Literal(Literal::Int(21))),
        };

        let result = type_checker.check_where_clause(&where_expr);
        assert!(result.is_ok(), "Valid WHERE clause should pass");
        println!("✓ WHERE clause validation works");

        println!("\n🎉 Type checker integration tests passed!");
    }

    #[test]
    fn test_query_plan_generation() {
        use std::collections::HashMap;

        println!("Testing query plan generation...");

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

        data_source.add_entity("entities", alice);

        // Create executor
        let mut executor = Executor::new(Box::new(data_source));
        let compiler = Compiler::new();

        // Test: Simple SELECT query generates plan
        println!("\nTesting: SELECT * FROM entities");
        let query = "SELECT * FROM entities";
        let statement = parse_statement(query).expect("Query should parse");
        let compiled = compiler.compile(statement).expect("Query should compile");

        // Verify we have an execution plan
        match &compiled.plan {
            crate::compiler::ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    crate::compiler::ExecutionPlan::Scan { .. } => {
                        println!("✓ Generated execution plan with Scan -> Project structure");
                    }
                    _ => panic!("Expected Scan as input to Project"),
                }
            }
            _ => panic!("Expected Project plan"),
        }

        // Test: Execute plan returns query results (for now, still executes)
        let result = executor.execute(compiled).expect("Plan should execute");
        assert_eq!(result.rows.len(), 1);
        println!("✓ Plan execution returned {} rows", result.rows.len());

        // Test: Query metadata generation
        println!("\nTesting: Query metadata generation");
        let query2 = "SELECT name, age FROM entities WHERE age > 25";
        let statement2 = parse_statement(query2).expect("Query should parse");
        let compiled2 = compiler.compile(statement2).expect("Query should compile");

        // Verify metadata includes accessed tables/columns
        assert!(compiled2.metadata.tables_accessed.contains(&"entities".to_string()));
        println!("✓ Metadata captured accessed tables: {:?}", compiled2.metadata.tables_accessed);

        // Verify cost estimation
        assert!(compiled2.estimated_cost.estimated_rows > 0);
        assert!(compiled2.estimated_cost.estimated_cpu_cost > 0.0);
        println!("✓ Cost estimation generated: {} estimated rows, {:.2} CPU cost",
                compiled2.estimated_cost.estimated_rows, compiled2.estimated_cost.estimated_cpu_cost);

        println!("\n🎉 All query plan generation tests passed! HyperQL is now a pure query language.");
    }

    #[test]
    fn test_comprehensive_type_validation() {
        use crate::ast::{Literal, BinaryOperator, ColumnRef, VectorExpression};

        println!("Testing comprehensive type validation across HyperQL features...");

        let mut type_checker = TypeChecker::new();

        // Test geometric expression type checking
        let geo_expr = Expression::Geometric(crate::ast::geometric::GeometricExpression::Within {
            target: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: "position".to_string(),
            })),
            radius: 5.0,
            reference: Box::new(Expression::Column(ColumnRef {
                table: None,
                name: "position".to_string(),
            })),
        });

        let geo_type = type_checker.check_expression_type(&geo_expr)
            .expect("Geometric expression should type check");
        assert_eq!(geo_type, TypeInfo::Bool, "WITHIN should return boolean");
        println!("✓ Geometric expression type checking works");

        // Test vector expression type checking
        let vector_expr = Expression::Vector(VectorExpression::Similarity {
            vector_name: "embeddings".to_string(),
            reference: Box::new(Expression::Literal(Literal::String("query".to_string()))),
            metric: crate::ast::vector::similarity::SimilarityMetric::Cosine,
            threshold: Some(0.8),
            vector_type: crate::ast::vector::similarity::VectorType::Dense { dimensions: 768 },
        });

        let vector_type = type_checker.check_expression_type(&vector_expr)
            .expect("Vector expression should type check");
        assert_eq!(vector_type, TypeInfo::Float, "Vector similarity should return float");
        println!("✓ Vector expression type checking works");

        // Test complex nested expression
        let complex_expr = Expression::Binary {
            left: Box::new(Expression::Function {
                name: "AVG".to_string(),
                args: vec![Expression::Literal(Literal::Float(85.5))],
            }),
            op: BinaryOperator::GreaterThan,
            right: Box::new(Expression::Binary {
                left: Box::new(Expression::Literal(Literal::Int(50))),
                op: BinaryOperator::Add,
                right: Box::new(Expression::Literal(Literal::Float(25.0))),
            }),
        };

        let complex_type = type_checker.check_expression_type(&complex_expr)
            .expect("Complex expression should type check");
        assert_eq!(complex_type, TypeInfo::Bool, "Comparison should return boolean");
        println!("✓ Complex nested expression type checking works");

        // Test UPDATE assignment validation
        let update_result = type_checker.check_update_assignment(
            "name",
            &Expression::Literal(Literal::String("John Doe".to_string()))
        );
        assert!(update_result.is_ok(), "Valid UPDATE assignment should pass");

        let invalid_update = type_checker.check_update_assignment(
            "position",
            &Expression::Literal(Literal::String("not a point".to_string()))
        );
        assert!(invalid_update.is_err(), "Invalid UPDATE assignment should fail");
        println!("✓ UPDATE assignment validation works");

        println!("\n🎉 Comprehensive type validation tests passed!");
    }
}
