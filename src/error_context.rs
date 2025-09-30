//! Error context and formatting utilities for HyperQL
//!
//! This module provides rich error messages with context, suggestions, and
//! helpful formatting for HyperQL parse, compile, and execution errors.
//! Adapted from GraphMeasure's error context system for HyperQL.

use crate::error::{HyperQLError, ErrorContext};
use colored::*;
use std::collections::HashMap;
use std::io::{self, IsTerminal};

/// Common error types for suggestion generation
#[derive(Debug, Clone)]
pub enum ErrorType {
    SyntaxError(String),
    UnknownColumn(String),
    UnknownFunction(String),
    TypeMismatch { expected: String, actual: String },
    MissingOperand,
    InvalidOperation(String),
    QueryTimeout,
    Other(String),
}

/// Suggestion generator for common HyperQL errors
pub struct SuggestionGenerator;

impl SuggestionGenerator {
    /// Generate suggestions based on error type and context
    pub fn generate_suggestions(error_type: &ErrorType, context: &str) -> Vec<String> {
        match error_type {
            ErrorType::UnknownColumn(col) => {
                let mut suggestions = vec![];
                
                // Check for common typos
                if let Some(similar) = Self::find_similar_column(col) {
                    suggestions.push(format!("Did you mean '{}'?", similar));
                }
                
                // Suggest checking available columns
                suggestions.push("Use 'DESCRIBE table_name' to see available columns".to_string());
                suggestions.push("Check column names are spelled correctly".to_string());
                
                suggestions
            }
            ErrorType::UnknownFunction(func) => {
                let mut suggestions = vec![];
                
                if let Some(similar) = Self::find_similar_function(func) {
                    suggestions.push(format!("Did you mean '{}'?", similar));
                }
                
                suggestions.push("Check function name spelling and availability".to_string());
                suggestions
            }
            ErrorType::TypeMismatch { expected, actual } => {
                vec![
                    format!("Cannot use {} where {} is expected", actual, expected),
                    Self::suggest_type_conversion(actual, expected),
                ]
            }
            ErrorType::SyntaxError(token) => {
                Self::suggest_syntax_fixes(token, context)
            }
            ErrorType::MissingOperand => {
                vec![
                    "Binary operators require two operands".to_string(),
                    "Check for missing values or expressions".to_string(),
                ]
            }
            _ => vec![],
        }
    }
    
    /// Find similar column names using edit distance
    fn find_similar_column(column: &str) -> Option<String> {
        // Common column names in HyperQL queries
        let common_columns = vec![
            "id", "name", "value", "type", "status", "description",
            "created_at", "updated_at", "position", "embedding",
            "employee_id", "department", "salary", "email", "age",
            "score", "category", "active", "enabled", "label",
        ];
        
        Self::find_similar_name(column, &common_columns)
    }
    
    /// Find similar function names using edit distance
    fn find_similar_function(function: &str) -> Option<String> {
        // Common function names in HyperQL
        let common_functions = vec![
            "COUNT", "SUM", "AVG", "MIN", "MAX",
            "UPPER", "LOWER", "LENGTH", "SUBSTRING", "TRIM",
            "CAST", "COALESCE", "NULLIF", "CASE",
            "ABS", "CEIL", "FLOOR", "ROUND", "SQRT",
            "NOW", "CURRENT_DATE", "CURRENT_TIME",
            "similarity", "knn", "distance", "within", "near",
        ];
        
        Self::find_similar_name(function, &common_functions)
    }
    
    /// Find similar names from a list using edit distance
    pub fn find_similar_names(name: &str, available: &[String]) -> Vec<String> {
        let mut suggestions = Vec::new();
        let name_lower = name.to_lowercase();
        
        for candidate in available {
            let candidate_lower = candidate.to_lowercase();
            let distance = Self::edit_distance(&name_lower, &candidate_lower);
            
            // Suggest if edit distance is reasonable (2 or less for most cases)
            let max_distance = if name.len() <= 3 { 1 } else { 2 };
            if distance <= max_distance {
                suggestions.push(candidate.clone());
            }
        }
        
        suggestions
    }
    
    /// Find a single similar name from a list
    fn find_similar_name(name: &str, candidates: &[&str]) -> Option<String> {
        let name_lower = name.to_lowercase();
        let mut best_candidate = None;
        let mut best_distance = usize::MAX;
        
        for &candidate in candidates {
            let distance = Self::edit_distance(&name_lower, candidate);
            
            // Only consider reasonable edit distances
            let max_distance = if name.len() <= 3 { 1 } else { 2 };
            if distance <= max_distance && distance < best_distance {
                best_distance = distance;
                best_candidate = Some(candidate.to_string());
            }
        }
        
        best_candidate
    }
    
    /// Calculate Levenshtein edit distance between two strings
    pub fn edit_distance(a: &str, b: &str) -> usize {
        let len_a = a.chars().count();
        let len_b = b.chars().count();
        
        if len_a == 0 {
            return len_b;
        }
        if len_b == 0 {
            return len_a;
        }
        
        let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];
        
        // Initialize first row and column
        for i in 0..=len_a {
            matrix[i][0] = i;
        }
        for j in 0..=len_b {
            matrix[0][j] = j;
        }
        
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        
        for (i, char_a) in a_chars.iter().enumerate() {
            for (j, char_b) in b_chars.iter().enumerate() {
                let cost = if char_a == char_b { 0 } else { 1 };
                matrix[i + 1][j + 1] = *[
                    matrix[i][j + 1] + 1,     // deletion
                    matrix[i + 1][j] + 1,     // insertion
                    matrix[i][j] + cost,      // substitution
                ].iter().min().unwrap();
            }
        }
        
        matrix[len_a][len_b]
    }
    
    /// Suggest type conversion based on types
    fn suggest_type_conversion(from: &str, to: &str) -> String {
        match (from.to_lowercase().as_str(), to.to_lowercase().as_str()) {
            ("string", "number") | ("text", "number") => {
                "Use CAST(value AS FLOAT) to convert string to number".to_string()
            }
            ("number", "string") | ("float", "string") | ("int", "string") => {
                "Use CAST(value AS STRING) to convert number to string".to_string()
            }
            ("string", "integer") | ("text", "int") => {
                "Use CAST(value AS INTEGER) to convert string to integer".to_string()
            }
            ("float", "integer") => {
                "Use CAST(value AS INTEGER) to truncate float to integer".to_string()
            }
            _ => format!("Consider using CAST to convert {} to {}", from, to),
        }
    }
    
    /// Suggest fixes for syntax errors based on token patterns
    fn suggest_syntax_fixes(token: &str, _context: &str) -> Vec<String> {
        let mut suggestions = vec![];
        
        // Common HyperQL keyword typos
        let keyword_fixes = HashMap::from([
            ("SELCT", "SELECT"),
            ("SLECT", "SELECT"),
            ("WERE", "WHERE"),
            ("WHRE", "WHERE"),
            ("FORM", "FROM"),
            ("FRON", "FROM"),
            ("GROPU", "GROUP"),
            ("GRPBY", "GROUP BY"),
            ("ORDERBY", "ORDER BY"),
            ("ORDEBY", "ORDER BY"),
            ("INNERJOIN", "INNER JOIN"),
            ("LEFTJOIN", "LEFT JOIN"),
            ("TRAVESE", "TRAVERSE"),
            ("TRAVRSE", "TRAVERSE"),
            ("CASCDE", "CASCADE"),
            ("CASCAD", "CASCADE"),
        ]);
        
        let token_upper = token.to_uppercase();
        if let Some(&correct) = keyword_fixes.get(token_upper.as_str()) {
            suggestions.push(format!("Did you mean '{}'?", correct));
        }
        
        // Check for missing spaces in compound keywords
        if token_upper.contains("ORDERBY") {
            suggestions.push("ORDER BY should be two separate words".to_string());
        }
        if token_upper.contains("GROUPBY") {
            suggestions.push("GROUP BY should be two separate words".to_string());
        }
        if token_upper.contains("INNERJOIN") {
            suggestions.push("INNER JOIN should be two separate words".to_string());
        }
        if token_upper.contains("LEFTJOIN") {
            suggestions.push("LEFT JOIN should be two separate words".to_string());
        }
        
        // Common punctuation mistakes
        if token.contains("'") && !token.starts_with("'") {
            suggestions.push("String literals should be enclosed in matching quotes".to_string());
        }
        
        if suggestions.is_empty() {
            suggestions.push("Check spelling and syntax of keywords".to_string());
        }
        
        suggestions
    }
}

/// Error formatter with cross-platform terminal color support
pub struct ErrorFormatter {
    use_color: bool,
}

impl ErrorFormatter {
    /// Create a new error formatter with automatic color detection
    pub fn new() -> Self {
        // Check if terminal supports color (cross-platform)
        let use_color = io::stderr().is_terminal();
        Self { use_color }
    }
    
    /// Create a new error formatter with explicit color setting
    pub fn with_color(use_color: bool) -> Self {
        Self { use_color }
    }
    
    /// Format an error with optional color support
    pub fn format(&self, context: &ErrorContext, error_message: &str) -> String {
        if self.use_color {
            self.format_with_color(context, error_message)
        } else {
            self.format_plain(context, error_message)
        }
    }
    
    /// Format error with ANSI colors for better visibility
    fn format_with_color(&self, context: &ErrorContext, error_message: &str) -> String {
        let mut output = String::new();
        
        // Red error header with error code
        if let Some(error_code) = &context.error_code {
            output.push_str(&format!(
                "{} {}: {}
",
                "Error".red().bold(),
                error_code.yellow(),
                error_message
            ));
        } else {
            output.push_str(&format!(
                "{}: {}
",
                "Error".red().bold(),
                error_message
            ));
        }
        
        // Blue location indicator
        if let (Some(line), Some(col)) = (context.line, context.column) {
            output.push_str(&format!(
                "  {} query:{}:{}
",
                "-->".blue().bold(),
                line,
                col
            ));
            
            // Show the problematic line with context
            if let Some(line_text) = context.get_line_at(line) {
                output.push_str(&format!("{}\n", "   |".blue()));
                output.push_str(&format!(
                    "{:3} {} {}
",
                    line.to_string().blue(),
                    "|".blue(),
                    line_text
                ));
                
                // Add pointer to the specific location
                if col > 0 {
                    let padding = " ".repeat(col.saturating_sub(1));
                    let pointer_len = context.get_token_length_at_position().unwrap_or(1);
                    let pointer = "^".repeat(pointer_len);
                    output.push_str(&format!(
                        "{} {} {}{} {}
",
                        "   |".blue(),
                        " ".repeat(3),
                        padding,
                        pointer.red().bold(),
                        error_message.dimmed()
                    ));
                }
            }
        }
        
        // Green help/suggestions
        if !context.suggestions.is_empty() {
            output.push_str(&format!("{}\n", "   |".blue()));
            for suggestion in &context.suggestions {
                output.push_str(&format!(
                    "   {} {}
",
                    "= help:".green().bold(),
                    suggestion
                ));
            }
        }
        
        // Cyan examples
        if !context.examples.is_empty() {
            output.push_str(&format!(
                "   {} Valid syntax examples:
",
                "= note:".cyan().bold()
            ));
            for example in &context.examples {
                output.push_str(&format!("           {}
", example.italic()));
            }
        }
        
        // Yellow documentation link
        if let Some(code) = &context.error_code {
            output.push_str(&format!(
                "   {} https://docs.hyperspatial.io/hyperql/errors/{}
",
                "= docs:".yellow().bold(),
                code.to_lowercase()
            ));
        }
        
        output
    }
    
    /// Format error without colors for compatibility
    fn format_plain(&self, context: &ErrorContext, error_message: &str) -> String {
        let mut output = String::new();
        
        // Plain error header
        if let Some(error_code) = &context.error_code {
            output.push_str(&format!("Error {}: {}\n", error_code, error_message));
        } else {
            output.push_str(&format!("Error: {}\n", error_message));
        }
        
        // Location information
        if let (Some(line), Some(col)) = (context.line, context.column) {
            output.push_str(&format!("  --> query:{}:{}\n", line, col));
            
            // Show the problematic line
            if let Some(line_text) = context.get_line_at(line) {
                output.push_str("   |\n");
                output.push_str(&format!("{:3} | {}\n", line, line_text));
                
                // Add pointer to the specific location
                if col > 0 {
                    let padding = " ".repeat(col.saturating_sub(1));
                    let pointer_len = context.get_token_length_at_position().unwrap_or(1);
                    let pointer = "^".repeat(pointer_len);
                    output.push_str(&format!("   |    {}{} {}\n", padding, pointer, error_message));
                }
            }
        }
        
        // Add suggestions
        if !context.suggestions.is_empty() {
            output.push_str("   |\n");
            for suggestion in &context.suggestions {
                output.push_str(&format!("   = help: {}\n", suggestion));
            }
        }
        
        // Add examples
        if !context.examples.is_empty() {
            output.push_str("   = note: Valid syntax examples:\n");
            for example in &context.examples {
                output.push_str(&format!("           {}\n", example));
            }
        }
        
        // Documentation link
        if let Some(code) = &context.error_code {
            output.push_str(&format!(
                "   = docs: https://docs.hyperspatial.io/hyperql/errors/{}\n",
                code.to_lowercase()
            ));
        }
        
        output
    }
}

impl Default for ErrorFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for adding rich context to HyperQL errors
pub trait HyperQLErrorExt {
    /// Format this error with rich context and colors
    fn format_rich(&self) -> String;
    /// Format this error with plain text (no colors)
    fn format_plain(&self) -> String;
    /// Add suggestions to an error
    fn with_suggestions(self, suggestions: Vec<String>) -> Self;
    /// Add examples to an error
    fn with_examples(self, examples: Vec<String>) -> Self;
}

impl HyperQLErrorExt for HyperQLError {
    fn format_rich(&self) -> String {
        let formatter = ErrorFormatter::new();
        
        match self {
            HyperQLError::ParseError { 
                message, 
                line, 
                column, 
                source_text, 
                error_code,
                suggestions,
                examples,
                position,
                ..
            } => {
                if let Some(source) = source_text {
                    let context = ErrorContext {
                        query_text: source.clone(),
                        position: *position,
                        line: Some(*line),
                        column: Some(*column),
                        problematic_text: None,
                        suggestions: suggestions.clone(),
                        examples: examples.clone(),
                        error_code: error_code.clone(),
                    };
                    formatter.format(&context, message)
                } else {
                    format!("{}", self)
                }
            }
            HyperQLError::UndefinedReference { 
                name,
                ref_type,
                available,
                suggestions,
                context,
                error_code,
                ..
            } => {
                let mut output = String::new();
                
                if let Some(code) = error_code {
                    output.push_str(&format!(
                        "{} {}: Undefined {}: '{}'
",
                        "Error".red().bold(),
                        code.yellow(),
                        ref_type,
                        name
                    ));
                } else {
                    output.push_str(&format!(
                        "{}: Undefined {}: '{}'
",
                        "Error".red().bold(),
                        ref_type,
                        name
                    ));
                }
                
                if let Some(ctx) = context {
                    output.push_str(&format!("   Context: {}\n", ctx));
                }
                
                if !suggestions.is_empty() {
                    for suggestion in suggestions {
                        output.push_str(&format!(
                            "   {} {}\n",
                            "= suggestion:".green().bold(),
                            suggestion
                        ));
                    }
                }
                
                if !available.is_empty() && available.len() <= 10 {
                    output.push_str(&format!(
                        "   {} {}\n",
                        "= available:".cyan().bold(),
                        available.join(", ")
                    ));
                }
                
                output
            }
            _ => format!("{}", self),
        }
    }
    
    fn format_plain(&self) -> String {
        let formatter = ErrorFormatter::with_color(false);
        
        match self {
            HyperQLError::ParseError { 
                message, 
                line, 
                column, 
                source_text, 
                error_code,
                suggestions,
                examples,
                position,
                ..
            } => {
                if let Some(source) = source_text {
                    let context = ErrorContext {
                        query_text: source.clone(),
                        position: *position,
                        line: Some(*line),
                        column: Some(*column),
                        problematic_text: None,
                        suggestions: suggestions.clone(),
                        examples: examples.clone(),
                        error_code: error_code.clone(),
                    };
                    formatter.format(&context, message)
                } else {
                    format!("{}", self)
                }
            }
            _ => format!("{}", self),
        }
    }
    
    fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        match &mut self {
            HyperQLError::ParseError { suggestions: s, .. } => {
                s.extend(suggestions);
            }
            HyperQLError::UndefinedReference { suggestions: s, .. } => {
                s.extend(suggestions);
            }
            _ => {}
        }
        self
    }
    
    fn with_examples(mut self, examples: Vec<String>) -> Self {
        match &mut self {
            HyperQLError::ParseError { examples: e, .. } => {
                e.extend(examples);
            }
            _ => {}
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_edit_distance() {
        assert_eq!(SuggestionGenerator::edit_distance("employee", "employee"), 0);
        assert_eq!(SuggestionGenerator::edit_distance("emplyee", "employee"), 1);
        assert_eq!(SuggestionGenerator::edit_distance("emp", "employee"), 5);
        assert_eq!(SuggestionGenerator::edit_distance("SELECT", "SELCT"), 1);
    }
    
    #[test]
    fn test_column_suggestions() {
        let similar = SuggestionGenerator::find_similar_column("emplyee_id");
        assert!(similar.is_some());
        
        let suggestions = SuggestionGenerator::generate_suggestions(
            &ErrorType::UnknownColumn("nam".to_string()),
            "SELECT nam FROM users"
        );
        assert!(!suggestions.is_empty());
    }
    
    #[test]
    fn test_syntax_suggestions() {
        let suggestions = SuggestionGenerator::generate_suggestions(
            &ErrorType::SyntaxError("SELCT".to_string()),
            "SELCT * FROM users"
        );
        assert!(suggestions.iter().any(|s| s.contains("SELECT")));
    }
    
    #[test]
    fn test_error_formatting() {
        let context = ErrorContext {
            query_text: "SELECT emplyee_id FROM entities".to_string(),
            position: Some(7),
            line: Some(1),
            column: Some(8),
            problematic_text: None,
            suggestions: vec!["Did you mean 'employee_id'?".to_string()],
            examples: vec!["SELECT employee_id FROM entities".to_string()],
            error_code: Some("E0102".to_string()),
        };
        
        let formatter = ErrorFormatter::with_color(false);
        let formatted = formatter.format(&context, "Unknown column 'emplyee_id'");
        
        assert!(formatted.contains("Error E0102"));
        assert!(formatted.contains("Unknown column 'emplyee_id'"));
        assert!(formatted.contains("Did you mean 'employee_id'?"));
        assert!(formatted.contains("SELECT employee_id FROM entities"));
        assert!(formatted.contains("query:1:8"));
    }
    
    #[test]
    fn test_color_detection() {
        let _formatter = ErrorFormatter::new();
        // Just test that it creates without panicking
        assert!(true);
    }
    
    #[test]
    fn test_find_similar_names() {
        let available = vec![
            "employee_name".to_string(),
            "employee_id".to_string(),
            "department".to_string(),
            "salary".to_string(),
        ];
        
        let suggestions = SuggestionGenerator::find_similar_names("emplyee_id", &available);
        assert!(suggestions.contains(&"employee_id".to_string()));
        
        let suggestions = SuggestionGenerator::find_similar_names("employee_nam", &available);
        assert!(suggestions.contains(&"employee_name".to_string()));
    }
}