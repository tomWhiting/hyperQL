//! Geometric query syntax parser
//!
//! This module implements parsing for geometric query expressions including:
//! - NEAR with WITHIN radius constraints
//! - DISTANCE for ORDER BY clauses
//!
//! These expressions enable spatial queries over multi-modal hyperboloid positions.

use crate::ast::*;
use crate::error::*;
use super::expression::parse_simple_column_or_literal;

/// Parse NEAR expression in WHERE clause
///
/// Syntax: `position NEAR reference WITHIN distance`
///
/// Returns a Function expression with name "near" and args [reference, distance]
///
/// # Example
/// ```text
/// position NEAR origin WITHIN 2.0
/// => Function { name: "near", args: [Column(origin), Literal(2.0)] }
/// ```
pub fn parse_near_expression(input: &str) -> Result<Expression> {
    let input = input.trim();
    
    // Look for "NEAR" keyword
    let near_upper = input.to_uppercase();
    let near_pos = near_upper.find(" NEAR ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "NEAR keyword not found in geometric expression",
            input,
            1,
            1,
        ))?;
    
    // Parse field name (should be "position")
    let field_part = input[..near_pos].trim();
    if field_part.to_uppercase() != "POSITION" {
        return Err(HyperQLError::simple_parse_error(
            "Expected 'position' before NEAR keyword",
            input,
            1,
            1,
        ));
    }
    
    // Find "WITHIN" keyword
    let within_pos = near_upper.find(" WITHIN ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "WITHIN keyword required after NEAR reference",
            input,
            1,
            1,
        ))?;
    
    // Parse reference between NEAR and WITHIN
    let reference_part = input[near_pos + 6..within_pos].trim();
    let reference_expr = parse_simple_column_or_literal(reference_part)?;
    
    // Parse distance after WITHIN
    let distance_part = input[within_pos + 8..].trim();
    let distance_expr = parse_simple_column_or_literal(distance_part)?;
    
    // Return as Function expression
    Ok(Expression::Function {
        name: "near".to_string(),
        args: vec![reference_expr, distance_expr],
    })
}

/// Parse DISTANCE expression in ORDER BY clause
///
/// Syntax: `position DISTANCE FROM reference`
///
/// Returns a Function expression with name "distance" and args [reference]
///
/// # Example
/// ```text
/// position DISTANCE FROM origin
/// => Function { name: "distance", args: [Column(origin)] }
/// ```
pub fn parse_distance_expression(input: &str) -> Result<Expression> {
    let input = input.trim();
    
    // Look for "DISTANCE" keyword
    let dist_upper = input.to_uppercase();
    let dist_pos = dist_upper.find(" DISTANCE ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "DISTANCE keyword not found in expression",
            input,
            1,
            1,
        ))?;
    
    // Parse field name (should be "position")
    let field_part = input[..dist_pos].trim();
    if field_part.to_uppercase() != "POSITION" {
        return Err(HyperQLError::simple_parse_error(
            "Expected 'position' before DISTANCE keyword",
            input,
            1,
            1,
        ));
    }
    
    // Find "FROM" keyword
    let from_pos = dist_upper.find(" FROM ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "FROM keyword required after DISTANCE",
            input,
            1,
            1,
        ))?;
    
    // Parse reference after FROM
    let reference_part = input[from_pos + 6..].trim();
    let reference_expr = parse_simple_column_or_literal(reference_part)?;
    
    // Return as Function expression
    Ok(Expression::Function {
        name: "distance".to_string(),
        args: vec![reference_expr],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_near_expression() {
        let input = "position NEAR origin WITHIN 2.0";
        let result = parse_near_expression(input);
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expression::Function { name, args } => {
                assert_eq!(name, "near");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_parse_near_with_column_reference() {
        let input = "position NEAR entity_123 WITHIN 5.5";
        let result = parse_near_expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_near_case_insensitive() {
        let input = "POSITION near Origin within 2.0";
        let result = parse_near_expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_near_missing_within() {
        let input = "position NEAR origin";
        let result = parse_near_expression(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_distance_expression() {
        let input = "position DISTANCE FROM origin";
        let result = parse_distance_expression(input);
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expression::Function { name, args } => {
                assert_eq!(name, "distance");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected Function expression"),
        }
    }

    #[test]
    fn test_parse_distance_with_entity_reference() {
        let input = "position DISTANCE FROM entity_456";
        let result = parse_distance_expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_distance_case_insensitive() {
        let input = "POSITION distance from Origin";
        let result = parse_distance_expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_distance_missing_from() {
        let input = "position DISTANCE origin";
        let result = parse_distance_expression(input);
        assert!(result.is_err());
    }
}
