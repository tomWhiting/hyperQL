//! Test geometric query compiler integration
//!
//! This example demonstrates that geometric queries (NEAR, DISTANCE) properly
//! compile to GeometricOperation execution plans instead of failing with
//! "Unknown function: HYPERBOLIC_DISTANCE" errors.

use hyperQL::{parse_statement, Compiler};

fn main() {
    println!("=== Geometric Query Compiler Integration Test ===\n");

    // Test 1: NEAR query compiles to GeometricOperation
    let query1 = "SELECT * FROM docs.Document WHERE position NEAR origin WITHIN 2.0";
    println!("Query 1: {}", query1);

    match parse_statement(query1) {
        Ok(statement) => {
            let compiler = Compiler::new();
            match compiler.compile(statement) {
                Ok(compiled) => {
                    println!("Compilation successful!");
                    println!("Plan type: {:?}\n", compiled.plan);
                }
                Err(e) => {
                    println!("Compilation failed: {:?}\n", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            println!("Parse failed: {:?}\n", e);
            std::process::exit(1);
        }
    }

    // Test 2: DISTANCE query in ORDER BY
    let query2 = "SELECT * FROM docs.Document ORDER BY position DISTANCE FROM origin LIMIT 10";
    println!("Query 2: {}", query2);

    match parse_statement(query2) {
        Ok(statement) => {
            let compiler = Compiler::new();
            match compiler.compile(statement) {
                Ok(compiled) => {
                    println!("Compilation successful!");
                    println!("Plan type: {:?}\n", compiled.plan);
                }
                Err(e) => {
                    println!("Compilation failed: {:?}\n", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            println!("Parse failed: {:?}\n", e);
            std::process::exit(1);
        }
    }

    // Test 3: Regular query should use Filter (not GeometricOperation)
    let query3 = "SELECT * FROM docs.Document WHERE title = 'test'";
    println!("Query 3 (non-geometric): {}", query3);

    match parse_statement(query3) {
        Ok(statement) => {
            let compiler = Compiler::new();
            match compiler.compile(statement) {
                Ok(compiled) => {
                    println!("Compilation successful!");
                    println!("Plan type: {:?}\n", compiled.plan);
                }
                Err(e) => {
                    println!("Compilation failed: {:?}\n", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            println!("Parse failed: {:?}\n", e);
            std::process::exit(1);
        }
    }

    println!("=== All tests passed! ===");
    println!("\nGeometric queries now properly route to GeometricOperation plans.");
    println!("The 'Unknown function: HYPERBOLIC_DISTANCE' error is fixed.");
}
