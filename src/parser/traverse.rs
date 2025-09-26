use crate::ast::*;
use crate::error::*;

pub fn parse_traverse_clause(input: &str) -> Result<TraverseClause> {
    let input = input.trim();
    let mut patterns = Vec::new();

    for pattern_str in input.split(',') {
        let pattern_str = pattern_str.trim();
        if !pattern_str.is_empty() {
            patterns.push(parse_traverse_pattern(pattern_str)?);
        }
    }

    if patterns.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "TRAVERSE clause must contain at least one pattern",
            input,
            1,
            1,
        ));
    }

    Ok(TraverseClause { patterns })
}

fn parse_traverse_pattern(input: &str) -> Result<TraversePattern> {
    let input = input.trim();

    let start_paren = input.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Pattern must start with node specification in parentheses",
            input,
            1,
            1,
        ))?;

    let end_paren = input.find(')')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing closing parenthesis for start node",
            input,
            1,
            1,
        ))?;

    let start_node = parse_node_pattern(&input[start_paren + 1..end_paren])?;

    let after_start_node = &input[end_paren + 1..];

    let (relationship, relationship_end_pos) = parse_relationship_pattern(after_start_node)?;

    let end_node_part = &after_start_node[relationship_end_pos..];
    let start_paren_end = end_node_part.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Pattern must end with node specification in parentheses",
            input,
            1,
            1,
        ))?;

    let end_paren_end = end_node_part.find(')')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Missing closing parenthesis for end node",
            input,
            1,
            1,
        ))?;

    let end_node = parse_node_pattern(&end_node_part[start_paren_end + 1..end_paren_end])?;

    Ok(TraversePattern {
        start_node,
        relationship,
        end_node,
    })
}

fn parse_node_pattern(input: &str) -> Result<NodePattern> {
    let input = input.trim();

    if input.is_empty() {
        return Ok(NodePattern {
            variable: None,
            label: None,
            properties: None,
        });
    }

    let parts: Vec<&str> = input.split(':').collect();
    let variable = if !parts[0].is_empty() {
        Some(parts[0].trim().to_string())
    } else {
        None
    };

    let label = if parts.len() > 1 && !parts[1].is_empty() {
        Some(parts[1].trim().to_string())
    } else {
        None
    };

    Ok(NodePattern {
        variable,
        label,
        properties: None,
    })
}

fn parse_relationship_pattern(input: &str) -> Result<(RelationshipPattern, usize)> {
    let input = input.trim();

    if let Some(pos) = input.find("-->") {
        Ok((RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelationshipDirection::Outgoing,
            variable_length: None,
            optional: false,
            properties: None,
        }, pos + 3))
    } else if let Some(pos) = input.find("<--") {
        Ok((RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelationshipDirection::Incoming,
            variable_length: None,
            optional: false,
            properties: None,
        }, pos + 3))
    } else if let Some(pos) = input.find("--") {
        Ok((RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelationshipDirection::Undirected,
            variable_length: None,
            optional: false,
            properties: None,
        }, pos + 2))
    } else if input.starts_with('-') {
        parse_complex_relationship_pattern(input)
    } else {
        Err(HyperQLError::simple_parse_error(
            "Invalid relationship pattern. Expected arrow syntax like --> or -[type]->",
            input,
            1,
            1,
        ))
    }
}

fn parse_complex_relationship_pattern(input: &str) -> Result<(RelationshipPattern, usize)> {
    let input = input.trim();

    if !input.starts_with('-') {
        return Err(HyperQLError::simple_parse_error(
            "Relationship pattern must start with dash",
            input,
            1,
            1,
        ));
    }

    if let Some(bracket_start) = input.find('[') {
        if let Some(bracket_end) = input.find(']') {
            let bracket_content = &input[bracket_start + 1..bracket_end];
            let (variable, rel_type, variable_length, optional) = parse_relationship_spec(bracket_content)?;

            let after_bracket = &input[bracket_end + 1..];
            let direction = if after_bracket.starts_with("->") {
                RelationshipDirection::Outgoing
            } else if after_bracket.starts_with("<-") {
                RelationshipDirection::Incoming
            } else if after_bracket.starts_with('-') {
                RelationshipDirection::Undirected
            } else {
                return Err(HyperQLError::simple_parse_error(
                    "Invalid arrow direction after relationship specification",
                    input,
                    1,
                    1,
                ));
            };

            let arrow_len = match direction {
                RelationshipDirection::Outgoing | RelationshipDirection::Incoming => 2,
                RelationshipDirection::Undirected => 1,
            };

            Ok((RelationshipPattern {
                variable,
                rel_type,
                direction,
                variable_length,
                optional,
                properties: None,
            }, bracket_end + 1 + arrow_len))
        } else {
            Err(HyperQLError::simple_parse_error(
                "Missing closing bracket in relationship pattern",
                input,
                1,
                1,
            ))
        }
    } else {
        Err(HyperQLError::simple_parse_error(
            "Expected bracket in relationship pattern",
            input,
            1,
            1,
        ))
    }
}

fn parse_relationship_spec(input: &str) -> Result<(Option<String>, Option<String>, Option<VariableLength>, bool)> {
    let input = input.trim();

    if input.is_empty() {
        return Ok((None, None, None, false));
    }

    let mut variable = None;
    let mut rel_type = None;
    let mut variable_length = None;
    let mut optional = false;

    let input = if input.ends_with('?') {
        optional = true;
        &input[..input.len() - 1]
    } else {
        input
    };

    let input = if let Some(star_pos) = input.find('*') {
        let var_len_str = &input[star_pos + 1..];
        let var_len = parse_variable_length(var_len_str)?;
        variable_length = Some(var_len);
        &input[..star_pos]
    } else {
        input
    };

    if input.contains(':') {
        let parts: Vec<&str> = input.split(':').collect();
        if !parts[0].is_empty() {
            variable = Some(parts[0].trim().to_string());
        }
        if parts.len() > 1 && !parts[1].is_empty() {
            rel_type = Some(parts[1].trim().to_string());
        }
    } else if !input.is_empty() {
        rel_type = Some(input.trim().to_string());
    }

    Ok((variable, rel_type, variable_length, optional))
}

fn parse_variable_length(input: &str) -> Result<VariableLength> {
    let input = input.trim();

    if input.is_empty() {
        return Ok(VariableLength {
            min_hops: None,
            max_hops: None,
        });
    }

    if input.contains("..") {
        let parts: Vec<&str> = input.split("..").collect();
        let min_hops = if parts[0].is_empty() {
            None
        } else {
            Some(parts[0].parse::<u32>().map_err(|_| HyperQLError::simple_parse_error(
                "Invalid minimum hop count in variable length specification",
                input,
                1,
                1,
            ))?)
        };

        let max_hops = if parts.len() > 1 && !parts[1].is_empty() {
            Some(parts[1].parse::<u32>().map_err(|_| HyperQLError::simple_parse_error(
                "Invalid maximum hop count in variable length specification",
                input,
                1,
                1,
            ))?)
        } else {
            None
        };

        Ok(VariableLength { min_hops, max_hops })
    } else {
        let hops = input.parse::<u32>().map_err(|_| HyperQLError::simple_parse_error(
            "Invalid hop count in variable length specification",
            input,
            1,
            1,
        ))?;

        Ok(VariableLength {
            min_hops: Some(hops),
            max_hops: Some(hops),
        })
    }
}
