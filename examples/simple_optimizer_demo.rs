//! Simple demo showing the HyperQL query optimizer API

use hyperQL::*;

fn main() -> Result<()> {
    println!("HyperQL Query Optimizer Demo");
    println!("==============================\n");
    
    // Create compiler and optimizer
    let compiler = Compiler::new();
    let optimizer = QueryOptimizer::new();
    
    // Example query with optimization opportunities
    let query = "SELECT name FROM users WHERE age > 20 + 5";
    println!("Query: {}", query);
    
    // Parse and compile the query
    let statement = parse_statement(query)?;
    let unoptimized = compiler.compile(statement.clone())?;
    
    println!("\nUnoptimized plan structure: {:?}", 
             std::mem::discriminant(&unoptimized.plan));
    
    // Apply optimizations
    let (optimized, stats) = compiler.compile_with_optimizer_stats(statement, &optimizer)?;
    
    println!("Optimized plan structure: {:?}", 
             std::mem::discriminant(&optimized.plan));
    
    println!("\nOptimization statistics:");
    println!("- Constant folding applied: {}", stats.constant_folding_applied);
    println!("- Expression simplify applied: {}", stats.expression_simplify_applied);
    println!("- Predicate pushdown applied: {}", stats.predicate_pushdown_applied);
    println!("- Projection pushdown applied: {}", stats.projection_pushdown_applied);
    println!("- Plan changed: {}", stats.plan_changed);
    
    println!("\nCost comparison:");
    println!("- Original CPU cost: {:.2}", unoptimized.estimated_cost.estimated_cpu_cost);
    println!("- Optimized CPU cost: {:.2}", optimized.estimated_cost.estimated_cpu_cost);
    
    println!("\n✅ Query optimizer working successfully!");
    
    Ok(())
}
