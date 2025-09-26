//! # Developer Experience Features Demo
//!
//! This example demonstrates the three new developer experience features
//! added to HyperQL:
//! 1. Query Builder API for fluent query construction
//! 2. Documentation System with built-in help and examples
//! 3. Utility Functions for common query patterns

use hyperQL::{
    HyperQLBuilder, HyperQLSyntax, HyperQLExamples, query_utils,
    ast::OrderDirection,
    types::Value
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 HyperQL Developer Experience Features Demo\n");

    // 1. Query Builder API Demo
    println!("📖 1. QUERY BUILDER API");
    println!("====================\n");
    
    // Build a simple SELECT query
    let simple_query = HyperQLBuilder::new()
        .select(["name", "age"])
        .from("users")
        .where_eq("age", Value::Int(21))
        .order_by("name", OrderDirection::Asc)
        .limit(10)
        .build()?;
    
    println!("✅ Built query: {:?}\n", simple_query);
    
    // Build a more complex query with multiple WHERE conditions
    let complex_query = HyperQLBuilder::new()
        .select(["*"])
        .from("products")
        .where_eq("category", Value::String("electronics".to_string()))
        .where_eq("in_stock", Value::Bool(true))
        .order_by("price", OrderDirection::Desc)
        .limit(5)
        .build()?;
    
    println!("✅ Built complex query: {:?}\n", complex_query);
    
    // 2. Documentation System Demo
    println!("📚 2. DOCUMENTATION SYSTEM");
    println!("=========================\n");
    
    // Show comprehensive documentation
    let doc = HyperQLSyntax::documentation();
    let doc_lines: Vec<&str> = doc.lines().take(15).collect();
    println!("📜 HyperQL Documentation (first 15 lines):");
    for line in doc_lines {
        println!("   {}", line);
    }
    println!("   ... (truncated for demo)\n");
    
    // Show categorized examples
    let examples = HyperQLExamples::new();
    
    println!("🔍 Basic Query Examples:");
    for (i, (desc, query)) in examples.basic.iter().take(3).enumerate() {
        println!("   {}. {}: {}", i + 1, desc, query);
    }
    println!();
    
    println!("📐 Geometric Query Examples:");
    for (i, (desc, query)) in examples.geometric.iter().take(2).enumerate() {
        println!("   {}. {}: {}", i + 1, desc, query);
    }
    println!();
    
    println!("🔗 Graph Traversal Examples:");
    for (i, (desc, query)) in examples.graph.iter().take(2).enumerate() {
        println!("   {}. {}: {}", i + 1, desc, query);
    }
    println!();
    
    // Demonstrate keyword search
    println!("🔎 Searching examples with keyword 'SELECT':");
    let select_examples = examples.find_examples_by_keyword("SELECT");
    for (i, (desc, _query)) in select_examples.iter().take(3).enumerate() {
        println!("   {}. {}", i + 1, desc);
    }
    println!();
    
    // 3. Utility Functions Demo
    println!("⚙️  3. UTILITY FUNCTIONS");
    println!("=====================\n");
    
    // Entity by label utility
    let label_query = query_utils::entity_by_label("Person")?;
    println!("👤 Find entities by label 'Person': {:?}\n", label_query);
    
    // Geometric utilities
    let near_query = query_utils::entities_near_point(1.0, 2.0, 3.0, 5.0)?;
    println!("📍 Find entities near point (1,2,3) within radius 5.0: {:?}\n", near_query);
    
    let within_query = query_utils::entities_within_radius("location", 0.0, 0.0, 0.0, 2.0)?;
    println!("🎯 Find entities within radius 2.0 of origin: {:?}\n", within_query);
    
    // Vector similarity search
    let similarity_query = query_utils::vector_similarity_search(
        "embeddings", 
        vec![0.1, 0.2, 0.3, 0.4], 
        10
    )?;
    println!("🔢 Vector similarity search (k=10): {:?}\n", similarity_query);
    
    // Graph traversal utility
    let traversal_query = query_utils::traverse_relationship(
        "user", 
        "FRIENDS", 
        Some("friend")
    )?;
    println!("🌐 Traverse friendship relationships: {:?}\n", traversal_query);
    
    // Range query utility
    let range_query = query_utils::entities_by_property_range(
        "employees",
        "salary",
        50000,
        100000
    )?;
    println!("💰 Find employees by salary range: {:?}\n", range_query);
    
    // Aggregation utility
    let agg_query = query_utils::aggregate_by_property(
        "sales",
        "region",
        "amount",
        "SUM"
    )?;
    println!("📊 Aggregate sales by region: {:?}\n", agg_query);
    
    println!("🎉 Demo completed successfully!");
    println!("\n💡 Key Benefits:");
    println!("   • Type-safe query construction with the builder API");
    println!("   • Comprehensive documentation and examples built-in");
    println!("   • Pre-built utilities for common query patterns");
    println!("   • Fluent, discoverable API design");
    println!("   • Full integration with HyperQL's multi-paradigm capabilities");
    
    Ok(())
}
