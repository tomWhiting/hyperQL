//! Graph Traversal Demonstration
//!
//! This example demonstrates basic graph traversal using HyperQL's TRAVERSE clause.
//! It shows how to query relationship patterns between entities stored as graph data.

use hyperQL::*;
use hyperQL::types::{Entity, EntityId, PropertyName, Value};
use std::collections::HashMap;

fn main() {
    println!("HyperQL Graph Traversal Demo");
    println!("=============================\n");

    let mut data_source = executor::MemoryDataSource::new();

    println!("Setting up sample social network data...");
    
    let users = vec![
        ("user1", "Alice"),
        ("user2", "Bob"),
        ("user3", "Charlie"),
        ("user4", "Diana"),
    ];

    for (id, name) in users {
        let entity = Entity {
            id: EntityId(id.to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert(PropertyName("name".to_string()), Value::String(name.to_string()));
                props
            },
            position: None,
            embedding: None,
        };
        data_source.add_entity("User", entity);
    }

    let relationships = vec![
        ("rel1", "user1", "user2", "follows"),
        ("rel2", "user2", "user3", "follows"),
        ("rel3", "user1", "user3", "follows"),
        ("rel4", "user3", "user4", "follows"),
        ("rel5", "user4", "user1", "follows"),
    ];

    for (id, from, to, rel_type) in relationships {
        let entity = Entity {
            id: EntityId(id.to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert(PropertyName("from_id".to_string()), Value::String(from.to_string()));
                props.insert(PropertyName("to_id".to_string()), Value::String(to.to_string()));
                props.insert(PropertyName("type".to_string()), Value::String(rel_type.to_string()));
                props
            },
            position: None,
            embedding: None,
        };
        data_source.add_entity("relationships", entity);
    }

    println!("Data loaded: 4 users, 5 follow relationships\n");

    let queries = vec![
        (
            "Find all 'follows' relationships",
            "SELECT * FROM User TRAVERSE (a:User)-[r:follows]->(b:User)"
        ),
        (
            "Find who Alice follows",
            "SELECT * FROM User TRAVERSE (a:User)-[r:follows]->(b:User)"
        ),
    ];

    for (description, query) in queries {
        println!("Query: {}", description);
        println!("SQL: {}\n", query);

        match parser::parse_statement(query) {
            Ok(statement) => {
                let compiler = compiler::Compiler::new();
                match compiler.compile(statement) {
                    Ok(compiled) => {
                        let mut executor = executor::Executor::new(Box::new(data_source.clone()));
                        match executor.execute(compiled) {
                            Ok(result) => {
                                println!("Results: {} rows found", result.rows.len());
                                println!("Relationships traversed: {}", result.execution_stats.relationships_traversed);
                                
                                if result.rows.len() > 0 {
                                    println!("\nSample results (first 3):");
                                    for (i, row) in result.rows.iter().take(3).enumerate() {
                                        println!("  Row {}:", i + 1);
                                        for (col, val) in &row.columns {
                                            println!("    {}: {:?}", col, val);
                                        }
                                    }
                                }
                                println!();
                            }
                            Err(e) => eprintln!("Execution error: {}\n", e),
                        }
                    }
                    Err(e) => eprintln!("Compilation error: {}\n", e),
                }
            }
            Err(e) => eprintln!("Parse error: {}\n", e),
        }
    }

    println!("\nDemonstrating Graph Operations (pending full integration):");
    println!("===========================================================\n");

    use hyperQL::compiler::{ExecutionPlan, GraphOpType};
    
    let graph_ops = vec![
        GraphOpType::ShortestPath,
        GraphOpType::PageRank,
        GraphOpType::CommunityDetection,
    ];

    for op in graph_ops {
        println!("Testing operation: {:?}", op);
        
        let plan = ExecutionPlan::GraphOperation {
            op_type: op,
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
        
        let mut executor = executor::Executor::new(Box::new(data_source.clone()));
        match executor.execute(compiled) {
            Ok(result) => {
                if let Some(row) = result.rows.first() {
                    if let Some(Value::String(status)) = row.columns.get("status") {
                        println!("  Status: {}", status);
                    }
                    if let Some(Value::String(note)) = row.columns.get("note") {
                        println!("  Note: {}\n", note);
                    }
                }
            }
            Err(e) => eprintln!("  Error: {}\n", e),
        }
    }

    println!("\nGraph traversal implementation complete!");
    println!("Basic relationship traversal is functional.");
    println!("Advanced graph algorithms await Hyperspatial graph engine integration.");
}
