//! # HyperQL Query Optimizer Demo
//!
//! This example demonstrates the HyperQL query optimizer in action,
//! showing how it transforms execution plans to improve query performance.

use hyperQL::*;

fn main() {
    println!("🚀 HyperQL Query Optimizer Demo\n");

    // Create compiler and optimizer
    let compiler = Compiler::new();
    let optimizer = QueryOptimizer::new();
    
    // Custom optimizer configuration
    let custom_config = OptimizerConfig {
        enable_predicate_pushdown: true,
        enable_projection_pushdown: true,
        enable_constant_folding: true,
        enable_expression_simplify: true,
    };
    let full_optimizer = QueryOptimizer::with_config(custom_config);
    
    demo_constant_folding(&compiler, &full_optimizer);
    demo_predicate_pushdown(&compiler, &full_optimizer);
    demo_expression_simplification(&compiler, &full_optimizer);
    demo_multi_optimization(&compiler, &full_optimizer);
    
    println!("\n✅ Query optimizer demo completed!");
}

fn demo_constant_folding(compiler: &Compiler, optimizer: &QueryOptimizer) {
    println!("📊 Constant Folding Optimization:\n");
    
    let query = "SELECT name FROM users WHERE age > 20 + 5 AND score > 50.0 * 1.5";
    println!("Original Query: {}", query);
    
    let statement = parse_statement(query).expect("Should parse");
    
    // Compile without optimizer
    let unoptimized = compiler.compile(statement.clone()).expect("Should compile");
    println!("\nUnoptimized Plan: {:?}", unoptimized.plan);
    
    // Compile with optimizer
    let (optimized, stats) = compiler.compile_with_optimizer_stats(statement, optimizer)
        .expect("Should compile with optimizer");
    
    println!("\nOptimized Plan: {:?}", optimized.plan);
    println!("\nOptimizations Applied: {:?}", stats);
    
    println!("\n💡 The constants '20 + 5' and '50.0 * 1.5' should be folded to '25' and '75.0'");
    println!("{}", "-".repeat(60));
}

fn demo_predicate_pushdown(compiler: &Compiler, optimizer: &QueryOptimizer) {
    println!("\n🔄 Predicate Pushdown Optimization:\n");
    
    let query = "SELECT name, age FROM users WHERE age > 25 ORDER BY name";
    println!("Original Query: {}", query);
    
    let statement = parse_statement(query).expect("Should parse");
    
    // Compile with optimizer
    let (optimized, stats) = compiler.compile_with_optimizer_stats(statement, optimizer)
        .expect("Should compile with optimizer");
    
    println!("\nOptimized Plan: {:?}", optimized.plan);
    println!("\nOptimizations Applied: {:?}", stats);
    
    println!("\n💡 The WHERE filter should be pushed down closer to the data source");
    println!("{}", "-".repeat(60));
}

fn demo_expression_simplification(compiler: &Compiler, optimizer: &QueryOptimizer) {
    println!("\n🧹 Expression Simplification:\n");
    
    let query = "SELECT * FROM users WHERE age > 20 AND age > 20 AND active = true";
    println!("Original Query: {}", query);
    
    let statement = parse_statement(query).expect("Should parse");
    
    // Compile with optimizer
    let (optimized, stats) = compiler.compile_with_optimizer_stats(statement, optimizer)
        .expect("Should compile with optimizer");
    
    println!("\nOptimized Plan: {:?}", optimized.plan);
    println!("\nOptimizations Applied: {:?}", stats);
    
    println!("\n💡 The duplicate 'age > 20' conditions should be simplified to a single condition");
    println!("{}", "-".repeat(60));
}

fn demo_multi_optimization(compiler: &Compiler, optimizer: &QueryOptimizer) {
    println!("\n🎯 Multi-Pass Optimization:\n");
    
    let query = "SELECT name FROM users WHERE (age > 10 + 15) AND (score > 30 + 40) ORDER BY name LIMIT 5";
    println!("Original Query: {}", query);
    
    let statement = parse_statement(query).expect("Should parse");
    
    // Compile without optimizer
    let unoptimized = compiler.compile(statement.clone()).expect("Should compile");
    println!("\nUnoptimized Cost: {:.2} CPU, {} estimated rows", 
             unoptimized.estimated_cost.estimated_cpu_cost,
             unoptimized.estimated_cost.estimated_rows);
    
    // Compile with optimizer
    let (optimized, stats) = compiler.compile_with_optimizer_stats(statement, optimizer)
        .expect("Should compile with optimizer");
    
    println!("\nOptimized Plan: {:?}", optimized.plan);
    println!("\nOptimized Cost: {:.2} CPU, {} estimated rows", 
             optimized.estimated_cost.estimated_cpu_cost,
             optimized.estimated_cost.estimated_rows);
    
    println!("\nOptimizations Applied: {:?}", stats);
    
    let cpu_improvement = ((unoptimized.estimated_cost.estimated_cpu_cost 
                           - optimized.estimated_cost.estimated_cpu_cost) 
                          / unoptimized.estimated_cost.estimated_cpu_cost * 100.0).max(0.0);
    
    println!("\n📈 Performance Improvement: {:.1}% CPU cost reduction", cpu_improvement);
    
    println!("\n💡 Multiple optimizations working together:");
    println!("   - Constants folded: '10 + 15' → '25', '30 + 40' → '70'");
    println!("   - Predicates pushed down to scan level");
    println!("   - Unnecessary projections eliminated");
    println!("{}", "-".repeat(60));
}

// Utility function for drawing separator lines
trait Repeat {
    fn repeat(&self, n: usize) -> String;
}

impl Repeat for &str {
    fn repeat(&self, n: usize) -> String {
        self.chars().cycle().take(n).collect()
    }
}
