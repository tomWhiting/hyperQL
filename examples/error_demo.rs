//! Demonstration of HyperQL's rich error reporting system
//!
//! This example showcases the advanced error context and suggestion system
//! that helps developers quickly identify and fix query issues.

use hyperQL::error_context::HyperQLErrorExt;
use hyperQL::{parse_statement, HyperQLError};

fn main() {
    println!("HyperQL Error Context Demo");
    println!("============================\n");
    
    // Example 1: Basic syntax error with suggestions
    demo_syntax_error();
    
    // Example 2: Column typo with edit distance suggestions
    demo_column_typo();
    
    // Example 3: Missing clause error
    demo_missing_clause();
    
    // Example 4: Complex traverse pattern error
    demo_traverse_error();
}

fn demo_syntax_error() {
    println!("1. Syntax Error Example:");
    println!("Query: 'SELCT * FROM users'\n");
    
    let query = "SELCT * FROM users";
    match parse_statement(query) {
        Ok(_) => println!("Unexpectedly parsed successfully"),
        Err(err) => {
            // Format with rich colors and suggestions
            println!("{}", err.format_rich());
        }
    }
    
    println!("\n{}\n", "=".repeat(60));
}

fn demo_column_typo() {
    println!("2. Column Typo with Suggestions:");
    println!("Query: 'SELECT emplyee_name FROM users'\n");
    
    // Simulate a semantic error for unknown column
    let error = HyperQLError::undefined_reference_with_suggestions(
        "emplyee_name",
        "column",
        vec![
            "employee_name".to_string(),
            "employee_id".to_string(),
            "employee_email".to_string(),
            "department".to_string(),
        ],
        Some("SELECT clause".to_string()),
    );
    
    println!("{}", error.format_rich());
    println!("\n{}\n", "=".repeat(60));
}

fn demo_missing_clause() {
    println!("3. Missing Clause Error:");
    println!("Query: 'SELECT * users'\n");
    
    let query = "SELECT * users";
    match parse_statement(query) {
        Ok(_) => println!("Unexpectedly parsed successfully"),
        Err(mut err) => {
            // Enhance the error with suggestions and examples
            err = err.with_suggestions(vec![
                "Add FROM clause to specify the data source".to_string(),
                "Use 'SELECT columns FROM table' syntax".to_string(),
            ]);
            
            err = err.with_examples(vec![
                "SELECT * FROM users".to_string(),
                "SELECT name, email FROM customers".to_string(),
            ]);
            
            println!("{}", err.format_rich());
        }
    }
    
    println!("\n{}\n", "=".repeat(60));
}

fn demo_traverse_error() {
    println!("4. Complex Traverse Pattern Error:");
    println!("Query: 'SELECT * FROM users TRAVERSE (a-[follows]->(b'\n");
    
    let query = "SELECT * FROM users TRAVERSE (a)-[follows]->(b";
    match parse_statement(query) {
        Ok(_) => println!("Unexpectedly parsed successfully"),
        Err(mut err) => {
            // Add specific suggestions for graph pattern errors
            err = err.with_suggestions(vec![
                "Check for balanced parentheses in graph patterns".to_string(),
                "End nodes must be enclosed in parentheses: (variable)".to_string(),
                "Graph patterns follow Cypher-style syntax".to_string(),
            ]);
            
            err = err.with_examples(vec![
                "(a)-[follows]->(b)".to_string(),
                "(user:Person)-[r:knows*1..3]->(friend:Person)".to_string(),
                "(start)-->(end)".to_string(),
            ]);
            
            println!("{}", err.format_rich());
        }
    }
    
    println!("\n{}\n", "=".repeat(60));
}

// Utility function to test plain text formatting
#[allow(dead_code)]
fn demo_plain_formatting() {
    println!("Plain Text Formatting (for non-color terminals):\n");
    
    let error = HyperQLError::undefined_reference_with_suggestions(
        "usr_name",
        "column",
        vec!["user_name".to_string(), "username".to_string()],
        Some("WHERE clause".to_string()),
    );
    
    println!("{}", error.format_plain());
}