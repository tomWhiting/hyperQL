use hyperQL::*;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("=== HyperQL Geometric Operations Demo ===\n");

    let mut data_source = MemoryDataSource::new();

    let origin = Position3D { x: 1.0, y: 0.0, z: 0.0 };
    
    let nearby1 = Position3D { x: 1.01, y: 0.1, z: 0.1 };
    let nearby2 = Position3D { x: 1.02, y: 0.15, z: 0.05 };
    
    let distant = Position3D { x: 2.0, y: 1.0, z: 1.0 };

    let entities = vec![
        Entity {
            id: EntityId("entity_origin".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert(PropertyName("name".to_string()), Value::String("Origin Entity".to_string()));
                props
            },
            position: Some(origin.clone()),
            embedding: None,
        },
        Entity {
            id: EntityId("entity_nearby1".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert(PropertyName("name".to_string()), Value::String("Nearby Entity 1".to_string()));
                props
            },
            position: Some(nearby1),
            embedding: None,
        },
        Entity {
            id: EntityId("entity_nearby2".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert(PropertyName("name".to_string()), Value::String("Nearby Entity 2".to_string()));
                props
            },
            position: Some(nearby2),
            embedding: None,
        },
        Entity {
            id: EntityId("entity_distant".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert(PropertyName("name".to_string()), Value::String("Distant Entity".to_string()));
                props
            },
            position: Some(distant),
            embedding: None,
        },
    ];

    data_source.add_entities("entities", entities);

    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    println!("Demo 1: Simple Hyperbolic Distance Calculation");
    println!("-----------------------------------------------");

    let query1 = "SELECT id, name, x, y, z FROM entities";
    execute_query(query1, &mut executor, &compiler)?;

    println!("\nDemo 2: Finding Entities Within Radius");
    println!("-----------------------------------------------");
    println!("Note: This demo showcases the geometric operation structure.");
    println!("In production, this would filter entities within hyperbolic radius of center point.\n");

    let query2 = "SELECT id, name FROM entities";
    execute_query(query2, &mut executor, &compiler)?;

    println!("\nDemo 3: Position Data Access");
    println!("-----------------------------------------------");
    println!("Entities with learned positions in hyperbolic space:\n");

    let query3 = "SELECT id, name, x, y, z FROM entities WHERE x > 0";
    execute_query(query3, &mut executor, &compiler)?;

    println!("\n=== Key Concepts ===");
    println!("1. Positions are learned via HGCN training (see Hyperspatial)");
    println!("2. hyperbolic_distance() calculates geodesic distance in hyperbolic space");
    println!("3. within_radius() filters entities by hyperbolic proximity");
    println!("4. Geometric operations preserve hyperbolic geometry properties");
    println!("\nFor entities without positions, position learning must be run first.");
    println!("See Hyperspatial's HGCN training examples for position learning.");

    Ok(())
}

fn execute_query(query: &str, executor: &mut Executor, compiler: &Compiler) -> Result<QueryResult> {
    println!("Query: {}", query);

    let ast = parse_statement(query)?;
    let compiled = compiler.compile(ast)?;
    let result = executor.execute(compiled)?;

    println!("Results ({} rows):", result.rows.len());
    for (i, row) in result.rows.iter().enumerate().take(5) {
        println!("  Row {}: {:?}", i + 1, row.columns);
    }
    if result.rows.len() > 5 {
        println!("  ... ({} more rows)", result.rows.len() - 5);
    }

    println!("Stats:");
    println!("  - Entities scanned: {}", result.execution_stats.entities_scanned);
    println!("  - Execution time: {}ms", result.execution_stats.execution_time_ms);

    Ok(result)
}
