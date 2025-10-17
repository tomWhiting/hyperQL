//! Vector query syntax parser
//!
//! This module implements parsing for vector query expressions including:
//! - SIMILARITY() function for vector similarity calculations
//! - DISTANCE() function for vector distance calculations
//! - SIMILAR TO operator with threshold constraints
//! - ORDER BY SIMILARITY/DISTANCE for k-NN queries
//!
//! These expressions enable vector similarity queries over named embeddings.

use crate::ast::*;
use crate::ast::vector::similarity::{SimilarityMetric, VectorType};
use crate::error::*;
use super::expression::parse_simple_column_or_literal;

/// Parse SIMILARITY function call
///
/// Syntax: `SIMILARITY(vector_name, reference_expr, 'metric')`
///
/// Returns a Vector expression with Similarity variant
///
/// # Example
/// ```text
/// SIMILARITY(text_embedding, query_vector, 'cosine')
/// => Expression::Vector(VectorExpression::Similarity { ... })
/// ```
pub fn parse_similarity_function(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Verify this is a SIMILARITY function call
    let upper = input.to_uppercase();
    if !upper.starts_with("SIMILARITY(") {
        return Err(HyperQLError::simple_parse_error(
            "Expected SIMILARITY function call",
            input,
            1,
            1,
        ));
    }

    // Find matching closing parenthesis
    let start_paren = input.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing opening parenthesis in SIMILARITY function",
            input,
            1,
            1,
        ))?;

    let end_paren = find_matching_paren(input, start_paren)?;

    if end_paren != input.len() - 1 {
        return Err(HyperQLError::simple_parse_error(
            "Unexpected characters after SIMILARITY function",
            input,
            1,
            1,
        ));
    }

    // Parse arguments: vector_name, reference, metric
    let args_str = &input[start_paren + 1..end_paren];
    let args = split_function_args(args_str)?;

    if args.len() != 3 {
        return Err(HyperQLError::simple_parse_error(
            "SIMILARITY function requires exactly 3 arguments: (vector_name, reference, metric)",
            input,
            1,
            1,
        ));
    }

    // Parse vector name (first argument - should be identifier)
    let vector_name = args[0].trim().to_string();
    if vector_name.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Vector name cannot be empty",
            input,
            1,
            1,
        ));
    }

    // Parse reference expression (second argument)
    let reference = parse_simple_column_or_literal(args[1].trim())?;

    // Parse metric (third argument - should be string literal)
    let metric = parse_metric_string(args[2].trim())?;

    // Default vector type (dimensions will be validated at runtime)
    let vector_type = VectorType::Dense { dimensions: 0 };

    Ok(Expression::Vector(VectorExpression::Similarity {
        vector_name,
        reference: Box::new(reference),
        metric,
        threshold: None,
        vector_type,
    }))
}

/// Parse DISTANCE function call
///
/// Syntax: `DISTANCE(vector_name, reference_expr, 'metric')`
///
/// Returns a Vector expression with Similarity variant (distance is inverted similarity)
///
/// # Example
/// ```text
/// DISTANCE(embedding, target, 'euclidean')
/// => Expression::Vector(VectorExpression::Similarity { ... })
/// ```
pub fn parse_distance_function(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Verify this is a DISTANCE function call
    let upper = input.to_uppercase();
    if !upper.starts_with("DISTANCE(") {
        return Err(HyperQLError::simple_parse_error(
            "Expected DISTANCE function call",
            input,
            1,
            1,
        ));
    }

    // Find matching closing parenthesis
    let start_paren = input.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing opening parenthesis in DISTANCE function",
            input,
            1,
            1,
        ))?;

    let end_paren = find_matching_paren(input, start_paren)?;

    if end_paren != input.len() - 1 {
        return Err(HyperQLError::simple_parse_error(
            "Unexpected characters after DISTANCE function",
            input,
            1,
            1,
        ));
    }

    // Parse arguments: vector_name, reference, metric
    let args_str = &input[start_paren + 1..end_paren];
    let args = split_function_args(args_str)?;

    if args.len() != 3 {
        return Err(HyperQLError::simple_parse_error(
            "DISTANCE function requires exactly 3 arguments: (vector_name, reference, metric)",
            input,
            1,
            1,
        ));
    }

    // Parse vector name (first argument)
    let vector_name = args[0].trim().to_string();
    if vector_name.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Vector name cannot be empty",
            input,
            1,
            1,
        ));
    }

    // Parse reference expression (second argument)
    let reference = parse_simple_column_or_literal(args[1].trim())?;

    // Parse metric (third argument)
    let metric = parse_metric_string(args[2].trim())?;

    // Default vector type
    let vector_type = VectorType::Dense { dimensions: 0 };

    Ok(Expression::Vector(VectorExpression::Similarity {
        vector_name,
        reference: Box::new(reference),
        metric,
        threshold: None,
        vector_type,
    }))
}

/// Parse SIMILAR TO operator expression
///
/// Syntax: `vector_name SIMILAR TO reference_expr THRESHOLD threshold_value`
///
/// Returns a Vector expression with Similarity variant and threshold
///
/// # Example
/// ```text
/// text_embedding SIMILAR TO query_vec THRESHOLD 0.8
/// => Expression::Vector(VectorExpression::Similarity { threshold: Some(0.8), ... })
/// ```
pub fn parse_similar_to_expression(input: &str) -> Result<Expression> {
    let input = input.trim();

    // Look for "SIMILAR TO" keywords
    let upper = input.to_uppercase();
    let similar_to_pos = upper.find(" SIMILAR TO ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "SIMILAR TO keywords not found",
            input,
            1,
            1,
        ))?;

    // Parse vector name before SIMILAR TO
    let vector_name = input[..similar_to_pos].trim().to_string();
    if vector_name.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Vector name cannot be empty",
            input,
            1,
            1,
        ));
    }

    // Find THRESHOLD keyword
    let rest = &input[similar_to_pos + 12..]; // " SIMILAR TO " is 12 chars
    let upper_rest = rest.to_uppercase();
    let threshold_pos = upper_rest.find(" THRESHOLD ")
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "THRESHOLD keyword required after SIMILAR TO reference",
            input,
            1,
            1,
        ))?;

    // Parse reference expression between SIMILAR TO and THRESHOLD
    let reference_part = rest[..threshold_pos].trim();
    let reference = parse_simple_column_or_literal(reference_part)?;

    // Parse threshold value after THRESHOLD
    let threshold_part = rest[threshold_pos + 11..].trim(); // " THRESHOLD " is 11 chars
    let threshold_expr = parse_simple_column_or_literal(threshold_part)?;

    // Extract threshold as float
    let threshold = match threshold_expr {
        Expression::Literal(Literal::Float(f)) => f,
        Expression::Literal(Literal::Int(i)) => i as f64,
        _ => {
            return Err(HyperQLError::simple_parse_error(
                "THRESHOLD value must be a numeric literal",
                input,
                1,
                1,
            ));
        }
    };

    // Validate threshold range
    if threshold < -1.0 || threshold > 1.0 {
        return Err(HyperQLError::simple_parse_error(
            "THRESHOLD value must be between -1.0 and 1.0",
            input,
            1,
            1,
        ));
    }

    // Default to cosine similarity for SIMILAR TO operator
    let metric = SimilarityMetric::Cosine;
    let vector_type = VectorType::Dense { dimensions: 0 };

    Ok(Expression::Vector(VectorExpression::Similarity {
        vector_name,
        reference: Box::new(reference),
        metric,
        threshold: Some(threshold),
        vector_type,
    }))
}

/// Parse metric string literal to SimilarityMetric enum
///
/// Supported metrics:
/// - 'cosine' -> Cosine
/// - 'dotproduct', 'dot' -> DotProduct
/// - 'euclidean' -> Euclidean
/// - 'manhattan' -> Manhattan
/// - 'jaccard' -> Jaccard
/// - Other strings -> Custom(String)
fn parse_metric_string(input: &str) -> Result<SimilarityMetric> {
    let input = input.trim();

    // Check for string literal (single or double quotes)
    if !(input.starts_with('\'') && input.ends_with('\'')) &&
       !(input.starts_with('"') && input.ends_with('"')) {
        return Err(HyperQLError::simple_parse_error(
            "Metric must be a string literal (e.g., 'cosine', \"euclidean\")",
            input,
            1,
            1,
        ));
    }

    // Extract string content
    let metric_str = &input[1..input.len() - 1];
    let metric_lower = metric_str.to_lowercase();

    Ok(match metric_lower.as_str() {
        "cosine" => SimilarityMetric::Cosine,
        "dotproduct" | "dot" => SimilarityMetric::DotProduct,
        "euclidean" => SimilarityMetric::Euclidean,
        "manhattan" => SimilarityMetric::Manhattan,
        "jaccard" => SimilarityMetric::Jaccard,
        _ => SimilarityMetric::Custom(metric_str.to_string()),
    })
}

/// Split function arguments handling nested parentheses and quotes
fn split_function_args(input: &str) -> Result<Vec<&str>> {
    let mut args = Vec::new();
    let mut current_start = 0;
    let mut paren_depth = 0;
    let mut in_quote = false;
    let mut quote_char = '"';
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        match ch {
            '\'' | '"' if i == 0 || chars[i - 1] != '\\' => {
                if !in_quote {
                    in_quote = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quote = false;
                }
            }
            '(' if !in_quote => paren_depth += 1,
            ')' if !in_quote => paren_depth -= 1,
            ',' if !in_quote && paren_depth == 0 => {
                args.push(&input[current_start..i]);
                current_start = i + 1;
            }
            _ => {}
        }
    }

    // Add final argument
    if current_start < input.len() {
        args.push(&input[current_start..]);
    }

    Ok(args)
}

/// Find matching closing parenthesis for opening parenthesis at given position
fn find_matching_paren(input: &str, open_pos: usize) -> Result<usize> {
    let chars: Vec<char> = input.chars().collect();
    let mut depth = 0;
    let mut in_quote = false;
    let mut quote_char = '"';

    for i in open_pos..chars.len() {
        let ch = chars[i];

        match ch {
            '\'' | '"' if i == 0 || chars[i - 1] != '\\' => {
                if !in_quote {
                    in_quote = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quote = false;
                }
            }
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            _ => {}
        }
    }

    Err(HyperQLError::simple_parse_error(
        "Unmatched opening parenthesis",
        input,
        1,
        1,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_similarity_function_cosine() {
        let input = "SIMILARITY(text_embedding, query_vector, 'cosine')";
        let result = parse_similarity_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity {
                vector_name,
                metric,
                threshold,
                ..
            }) => {
                assert_eq!(vector_name, "text_embedding");
                assert!(matches!(metric, SimilarityMetric::Cosine));
                assert_eq!(threshold, None);
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similarity_function_dotproduct() {
        let input = "SIMILARITY(code_vec, target, \"dotproduct\")";
        let result = parse_similarity_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                assert!(matches!(metric, SimilarityMetric::DotProduct));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similarity_function_euclidean() {
        let input = "SIMILARITY(embedding, ref, 'euclidean')";
        let result = parse_similarity_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                assert!(matches!(metric, SimilarityMetric::Euclidean));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similarity_function_manhattan() {
        let input = "SIMILARITY(vec, reference, 'manhattan')";
        let result = parse_similarity_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                assert!(matches!(metric, SimilarityMetric::Manhattan));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similarity_function_jaccard() {
        let input = "SIMILARITY(sparse_vec, query, 'jaccard')";
        let result = parse_similarity_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                assert!(matches!(metric, SimilarityMetric::Jaccard));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similarity_function_custom() {
        let input = "SIMILARITY(vec, ref, 'custom_metric')";
        let result = parse_similarity_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                match metric {
                    SimilarityMetric::Custom(name) => assert_eq!(name, "custom_metric"),
                    _ => panic!("Expected Custom metric"),
                }
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_distance_function_euclidean() {
        let input = "DISTANCE(embedding, target, 'euclidean')";
        let result = parse_distance_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity {
                vector_name,
                metric,
                ..
            }) => {
                assert_eq!(vector_name, "embedding");
                assert!(matches!(metric, SimilarityMetric::Euclidean));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_distance_function_manhattan() {
        let input = "DISTANCE(vec, query, 'manhattan')";
        let result = parse_distance_function(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { metric, .. }) => {
                assert!(matches!(metric, SimilarityMetric::Manhattan));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similar_to_with_threshold() {
        let input = "text_embedding SIMILAR TO query_vec THRESHOLD 0.8";
        let result = parse_similar_to_expression(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity {
                vector_name,
                threshold,
                metric,
                ..
            }) => {
                assert_eq!(vector_name, "text_embedding");
                assert_eq!(threshold, Some(0.8));
                assert!(matches!(metric, SimilarityMetric::Cosine));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similar_to_with_integer_threshold() {
        let input = "code_embedding SIMILAR TO target THRESHOLD 1";
        let result = parse_similar_to_expression(input);
        assert!(result.is_ok());

        match result.unwrap() {
            Expression::Vector(VectorExpression::Similarity { threshold, .. }) => {
                assert_eq!(threshold, Some(1.0));
            }
            _ => panic!("Expected Vector::Similarity expression"),
        }
    }

    #[test]
    fn test_parse_similar_to_missing_threshold() {
        let input = "text_embedding SIMILAR TO query_vec";
        let result = parse_similar_to_expression(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_similar_to_invalid_threshold_range() {
        let input = "embedding SIMILAR TO query THRESHOLD 1.5";
        let result = parse_similar_to_expression(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_similarity_missing_arguments() {
        let input = "SIMILARITY(vec, ref)";
        let result = parse_similarity_function(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_similarity_invalid_metric() {
        let input = "SIMILARITY(vec, ref, cosine)";
        let result = parse_similarity_function(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_distance_missing_arguments() {
        let input = "DISTANCE(vec, ref)";
        let result = parse_distance_function(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_metric_string_parsing_dot_alias() {
        let result = parse_metric_string("'dot'");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SimilarityMetric::DotProduct));
    }

    #[test]
    fn test_metric_string_parsing_case_insensitive() {
        let result = parse_metric_string("'COSINE'");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SimilarityMetric::Cosine));
    }

    #[test]
    fn test_split_function_args_simple() {
        let input = "arg1, arg2, arg3";
        let result = split_function_args(input);
        assert!(result.is_ok());
        let args = result.unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].trim(), "arg1");
        assert_eq!(args[1].trim(), "arg2");
        assert_eq!(args[2].trim(), "arg3");
    }

    #[test]
    fn test_split_function_args_with_nested_parens() {
        let input = "func(a, b), arg2, arg3";
        let result = split_function_args(input);
        assert!(result.is_ok());
        let args = result.unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].trim(), "func(a, b)");
    }

    #[test]
    fn test_split_function_args_with_quotes() {
        let input = "arg1, 'string, with, commas', arg3";
        let result = split_function_args(input);
        assert!(result.is_ok());
        let args = result.unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[1].trim(), "'string, with, commas'");
    }

    #[test]
    fn test_find_matching_paren_simple() {
        let input = "(test)";
        let result = find_matching_paren(input, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_find_matching_paren_nested() {
        let input = "(a(b)c)";
        let result = find_matching_paren(input, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 6);
    }

    #[test]
    fn test_find_matching_paren_unmatched() {
        let input = "(test";
        let result = find_matching_paren(input, 0);
        assert!(result.is_err());
    }
}
