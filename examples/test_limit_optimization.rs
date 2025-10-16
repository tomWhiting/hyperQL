use hyperQL::parser::parse_statement;
use hyperQL::compiler::Compiler;
use hyperQL::compiler::ExecutionPlan;

fn main() {
    let compiler = Compiler::new();
    
    // Test 1: Simple SELECT with LIMIT (should push down)
    println!("Test 1: SELECT * FROM entities LIMIT 5");
    let query1 = "SELECT * FROM entities LIMIT 5";
    let statement1 = parse_statement(query1).unwrap();
    let compiled1 = compiler.compile(statement1).unwrap();
    
    match &compiled1.plan {
        ExecutionPlan::Limit { input, .. } => {
            match input.as_ref() {
                ExecutionPlan::Project { input, .. } => {
                    match input.as_ref() {
                        ExecutionPlan::Scan { limit, .. } => {
                            println!("  ✓ LIMIT pushed down to Scan: {:?}", limit);
                            assert_eq!(*limit, Some(5));
                        }
                        other => {
                            println!("  ✗ Expected Scan with limit, got: {:?}", other);
                            panic!("LIMIT not pushed down!");
                        }
                    }
                }
                other => {
                    println!("  ✗ Expected Project, got: {:?}", other);
                    panic!("Unexpected plan structure!");
                }
            }
        }
        other => {
            println!("  ✗ Expected Limit plan, got: {:?}", other);
            panic!("Unexpected top-level plan!");
        }
    }
    
    // Test 2: SELECT with WHERE and LIMIT (should NOT push down)
    println!("\nTest 2: SELECT * FROM entities WHERE age > 18 LIMIT 5");
    let query2 = "SELECT * FROM entities WHERE age > 18 LIMIT 5";
    let statement2 = parse_statement(query2).unwrap();
    let compiled2 = compiler.compile(statement2).unwrap();
    
    match &compiled2.plan {
        ExecutionPlan::Limit { input, .. } => {
            match input.as_ref() {
                ExecutionPlan::Project { input, .. } => {
                    match input.as_ref() {
                        ExecutionPlan::Filter { input, .. } => {
                            match input.as_ref() {
                                ExecutionPlan::Scan { limit, .. } => {
                                    println!("  ✓ LIMIT NOT pushed down to Scan (correct!): {:?}", limit);
                                    assert_eq!(*limit, None);
                                }
                                other => panic!("Expected Scan, got: {:?}", other)
                            }
                        }
                        other => panic!("Expected Filter, got: {:?}", other)
                    }
                }
                other => panic!("Expected Project, got: {:?}", other)
            }
        }
        other => panic!("Expected Limit plan, got: {:?}", other)
    }
    
    // Test 3: SELECT with ORDER BY and LIMIT (should NOT push down)
    println!("\nTest 3: SELECT * FROM entities ORDER BY name LIMIT 5");
    let query3 = "SELECT * FROM entities ORDER BY name LIMIT 5";
    let statement3 = parse_statement(query3).unwrap();
    let compiled3 = compiler.compile(statement3).unwrap();
    
    match &compiled3.plan {
        ExecutionPlan::Limit { input, .. } => {
            match input.as_ref() {
                ExecutionPlan::Sort { input, .. } => {
                    match input.as_ref() {
                        ExecutionPlan::Project { input, .. } => {
                            match input.as_ref() {
                                ExecutionPlan::Scan { limit, .. } => {
                                    println!("  ✓ LIMIT NOT pushed down to Scan (correct!): {:?}", limit);
                                    assert_eq!(*limit, None);
                                }
                                other => panic!("Expected Scan, got: {:?}", other)
                            }
                        }
                        other => panic!("Expected Project, got: {:?}", other)
                    }
                }
                other => panic!("Expected Sort, got: {:?}", other)
            }
        }
        other => panic!("Expected Limit plan, got: {:?}", other)
    }
    
    println!("\n✓ All tests passed! LIMIT optimization working correctly.");
}
