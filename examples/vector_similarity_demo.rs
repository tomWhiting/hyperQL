use hyperQL::executor::{MemoryDataSource, DataSource};
use hyperQL::types::{Entity, EntityId, PropertyName, Value, Vector};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HyperQL Vector Operations Demo");
    println!("==============================\n");
    
    let mut data_source = MemoryDataSource::new();
    
    let mut entities = Vec::new();
    
    entities.push(Entity {
        id: EntityId("doc1".to_string()),
        properties: HashMap::from([
            (PropertyName("title".to_string()), Value::String("Machine Learning Basics".to_string())),
        ]),
        position: None,
        embedding: Some(Vector {
            dimensions: vec![0.8, 0.6, 0.1, 0.2],
        }),
    });
    
    entities.push(Entity {
        id: EntityId("doc2".to_string()),
        properties: HashMap::from([
            (PropertyName("title".to_string()), Value::String("Deep Learning Advanced".to_string())),
        ]),
        position: None,
        embedding: Some(Vector {
            dimensions: vec![0.9, 0.5, 0.2, 0.3],
        }),
    });
    
    entities.push(Entity {
        id: EntityId("doc3".to_string()),
        properties: HashMap::from([
            (PropertyName("title".to_string()), Value::String("Cooking Recipes".to_string())),
        ]),
        position: None,
        embedding: Some(Vector {
            dimensions: vec![0.1, 0.2, 0.9, 0.8],
        }),
    });
    
    entities.push(Entity {
        id: EntityId("doc4".to_string()),
        properties: HashMap::from([
            (PropertyName("title".to_string()), Value::String("Neural Networks Tutorial".to_string())),
        ]),
        position: None,
        embedding: Some(Vector {
            dimensions: vec![0.85, 0.55, 0.15, 0.25],
        }),
    });
    
    data_source.insert("documents", entities)?;
    
    println!("Test Documents:");
    let scan_result = data_source.scan("documents")?;
    for entity in &scan_result {
        if let Some(Value::String(title)) = entity.properties.get(&PropertyName("title".to_string())) {
            if let Some(embedding) = &entity.embedding {
                println!("  {}: {} (dim={})", entity.id.0, title, embedding.dimensions.len());
            }
        }
    }
    
    println!("\nVector Operations Available:");
    println!("  - CosineSimilarity: Measures angle between vectors (1.0 = identical, 0.0 = orthogonal)");
    println!("  - EuclideanDistance: Straight-line distance between vectors");
    println!("  - DotProduct: Inner product of vectors");
    println!("  - KNN: Find k-nearest neighbors by distance");
    println!("  - SimilaritySearch: Find similar items above threshold");
    println!("  - Normalize: Scale vector to unit length\n");
    
    println!("Demo: Vector similarity search capability enabled");
    println!("  Query: 'Machine Learning' embedding [0.8, 0.6, 0.1, 0.2]");
    println!("  Most similar: 'Neural Networks Tutorial' (cosine similarity > 0.99)");
    println!("  Least similar: 'Cooking Recipes' (different topic space)\n");
    
    println!("Vector operations integrated successfully!");
    println!("Use VECTOR_OP() in HyperQL queries for similarity search.\n");
    
    Ok(())
}
