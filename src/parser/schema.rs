//! # Schema DDL Parser
//!
//! Parser for schema DDL statements (CREATE SCHEMA, ALTER SCHEMA, DROP SCHEMA, DESCRIBE SCHEMA).
//! Supports field definitions with types, scopes, and cascade configurations.

use crate::ast::schema::*;
use crate::error::{HyperQLError, Result};

/// Parse schema DDL statement
///
/// Expects input starting with CREATE/ALTER/DROP/DESCRIBE SCHEMA
pub fn parse_schema_statement(input: &str) -> Result<SchemaOperation> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    if upper_input.starts_with("CREATE SCHEMA") {
        parse_create_schema(input).map(SchemaOperation::Create)
    } else if upper_input.starts_with("ALTER SCHEMA") {
        parse_alter_schema(input).map(SchemaOperation::Alter)
    } else if upper_input.starts_with("DROP SCHEMA") {
        parse_drop_schema(input).map(SchemaOperation::Drop)
    } else if upper_input.starts_with("DESCRIBE SCHEMA") {
        parse_describe_schema(input).map(SchemaOperation::Describe)
    } else {
        Err(HyperQLError::simple_parse_error(
            "Expected CREATE/ALTER/DROP/DESCRIBE SCHEMA",
            input,
            1,
            1,
        ))
    }
}

/// Parse CREATE SCHEMA statement
fn parse_create_schema(input: &str) -> Result<CreateSchemaStatement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    // Check for IF NOT EXISTS
    let if_not_exists = upper_input.contains(" IF NOT EXISTS");
    
    // Extract collection name
    let schema_start = if if_not_exists {
        upper_input.find(" IF NOT EXISTS").unwrap() + 15
    } else {
        "CREATE SCHEMA".len()
    };
    
    let after_schema = input[schema_start..].trim();
    
    // Find opening parenthesis for field definitions
    let paren_pos = after_schema.find('(')
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Expected '(' after collection name",
            input,
            1,
            1,
        ))?;
    
    let collection_name = after_schema[..paren_pos].trim().to_string();
    
    // Find matching closing parenthesis
    let close_paren = find_matching_paren(after_schema, paren_pos)?;
    
    // Parse field definitions
    let fields_str = &after_schema[paren_pos + 1..close_paren];
    let fields = parse_field_definitions(fields_str)?;
    
    // Parse WITH clause if present
    let after_fields = &after_schema[close_paren + 1..].trim();
    let (extensibility, description) = parse_with_clause(after_fields)?;
    
    Ok(CreateSchemaStatement {
        collection_name,
        fields,
        extensibility,
        description,
        if_not_exists,
    })
}

/// Parse ALTER SCHEMA statement
fn parse_alter_schema(input: &str) -> Result<AlterSchemaStatement> {
    let input = input.trim();
    
    // Extract collection name
    let after_alter = &input["ALTER SCHEMA".len()..].trim();
    
    // Find the operation keyword (ADD/DROP/MODIFY/SET)
    let parts: Vec<&str> = after_alter.split_whitespace().collect();
    if parts.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Missing collection name in ALTER SCHEMA",
            input,
            1,
            1,
        ));
    }
    
    let collection_name = parts[0].to_string();
    
    // Find operation type
    let rest = &after_alter[collection_name.len()..].trim();
    let upper_rest = rest.to_uppercase();
    
    let operation = if upper_rest.starts_with("ADD FIELD") {
        let field_def = &rest["ADD FIELD".len()..].trim();
        let field = parse_field_definition(field_def)?;
        AlterOperation::AddField(field)
    } else if upper_rest.starts_with("DROP FIELD") {
        let field_name = &rest["DROP FIELD".len()..].trim();
        // Remove semicolon if present
        let field_name = field_name.trim_end_matches(';').trim();
        AlterOperation::DropField(field_name.to_string())
    } else if upper_rest.starts_with("MODIFY FIELD") {
        let field_spec = &rest["MODIFY FIELD".len()..].trim();

        // Parse the entire field spec as a field definition, then extract the name
        let parsed_field = parse_field_definition(field_spec)?;
        let name = parsed_field.name.clone();

        AlterOperation::ModifyField {
            name,
            definition: parsed_field,
        }
    } else if upper_rest.starts_with("SET EXTENSIBILITY") {
        let mode_str = &rest["SET EXTENSIBILITY".len()..].trim();
        let mode = parse_extensibility_mode(mode_str)?;
        AlterOperation::SetExtensibility(mode)
    } else {
        return Err(HyperQLError::simple_parse_error(
            "Expected ADD FIELD, DROP FIELD, MODIFY FIELD, or SET EXTENSIBILITY",
            input,
            1,
            1,
        ));
    };
    
    Ok(AlterSchemaStatement {
        collection_name,
        operation,
    })
}

/// Parse DROP SCHEMA statement
fn parse_drop_schema(input: &str) -> Result<DropSchemaStatement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    let if_exists = upper_input.contains(" IF EXISTS");
    
    let schema_start = if if_exists {
        upper_input.find(" IF EXISTS").unwrap() + 10
    } else {
        "DROP SCHEMA".len()
    };
    
    let collection_name = input[schema_start..].trim()
        .trim_end_matches(';')
        .trim()
        .to_string();
    
    Ok(DropSchemaStatement {
        collection_name,
        if_exists,
    })
}

/// Parse DESCRIBE SCHEMA statement
fn parse_describe_schema(input: &str) -> Result<DescribeSchemaStatement> {
    let input = input.trim();
    
    let after_describe = &input["DESCRIBE SCHEMA".len()..].trim();
    
    let detailed = after_describe.to_uppercase().contains(" DETAILED");
    
    let collection_name = if detailed {
        let detailed_pos = after_describe.to_uppercase().find(" DETAILED").unwrap();
        after_describe[..detailed_pos].trim()
    } else {
        after_describe.trim_end_matches(';').trim()
    }.to_string();
    
    Ok(DescribeSchemaStatement {
        collection_name,
        detailed,
    })
}

/// Parse field definitions (comma-separated)
fn parse_field_definitions(input: &str) -> Result<Vec<FieldDefinition>> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut paren_depth = 0;

    for ch in input.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current_field.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current_field.push(ch);
            }
            ',' if paren_depth == 0 => {
                if !current_field.trim().is_empty() {
                    fields.push(parse_field_definition(current_field.trim())?);
                }
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Parse last field
    if !current_field.trim().is_empty() {
        fields.push(parse_field_definition(current_field.trim())?);
    }

    Ok(fields)
}

/// Parse a single field definition
fn parse_field_definition(input: &str) -> Result<FieldDefinition> {
    let input = input.trim();

    // Parse: field_name TYPE [REQUIRED|OPTIONAL] [METADATA|COLLECTION_SPECIFIC|DOMAIN_SHARED] [CASCADED AS ...]

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(HyperQLError::simple_parse_error(
            "Field definition requires at least name and type",
            input,
            1,
            1,
        ));
    }

    let name = parts[0].to_string();
    let field_type = parse_field_type(parts[1])?;

    let mut required = false;
    let mut scope = None;
    let mut kind = FieldKind::Regular;
    let description = None;
    
    // Parse remaining modifiers
    let mut i = 2;
    while i < parts.len() {
        let part_upper = parts[i].to_uppercase();
        
        match part_upper.as_str() {
            "REQUIRED" => {
                required = true;
                i += 1;
            }
            "OPTIONAL" => {
                required = false;
                i += 1;
            }
            "METADATA" => {
                scope = Some(PropertyScope::Metadata);
                i += 1;
            }
            "COLLECTION_SPECIFIC" => {
                scope = Some(PropertyScope::CollectionSpecific);
                i += 1;
            }
            "DOMAIN_SHARED" => {
                scope = Some(PropertyScope::DomainShared);
                i += 1;
            }
            "CASCADED" => {
                // Expect: CASCADED AS AGGREGATE ... FROM ... [THROUGH ... direction]
                if i + 1 >= parts.len() || parts[i + 1].to_uppercase() != "AS" {
                    return Err(HyperQLError::simple_parse_error(
                        "CASCADED must be followed by AS",
                        input,
                        1,
                        1,
                    ));
                }
                
                // Find the rest of the cascade configuration
                let cascade_start = input.to_uppercase().find("CASCADED AS").unwrap() + 11;
                let cascade_config = parse_cascade_config(&input[cascade_start..])?;
                kind = FieldKind::Cascaded(cascade_config);
                break; // Cascade config consumes rest of definition
            }
            _ => {
                // Unknown modifier, skip or error
                i += 1;
            }
        }
    }
    
    Ok(FieldDefinition {
        name,
        field_type,
        required,
        scope,
        kind,
        description,
    })
}

/// Parse field type
fn parse_field_type(input: &str) -> Result<FieldType> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    match upper_input.as_str() {
        "STRING" => Ok(FieldType::String),
        "INTEGER" | "INT" => Ok(FieldType::Integer),
        "FLOAT" | "DOUBLE" => Ok(FieldType::Float),
        "BOOLEAN" | "BOOL" => Ok(FieldType::Boolean),
        "TIMESTAMP" => Ok(FieldType::Timestamp),
        "DURATION" => Ok(FieldType::Duration),
        "DATE" => Ok(FieldType::Date),
        "JSON" => Ok(FieldType::Json),
        "REFERENCE" | "REF" => Ok(FieldType::Reference),
        "EDGE" => Ok(FieldType::Edge),
        "POSITION3D" => Ok(FieldType::Position3D),
        "BYTES" => Ok(FieldType::Bytes),
        _ => {
            // Check for Array(T) or Map(T) or Embedding(N)
            if upper_input.starts_with("ARRAY(") && upper_input.ends_with(")") {
                let inner = &input[6..input.len() - 1];
                let element_type = parse_field_type(inner)?;
                Ok(FieldType::Array(Box::new(element_type)))
            } else if upper_input.starts_with("MAP(") && upper_input.ends_with(")") {
                let inner = &input[4..input.len() - 1];
                let value_type = parse_field_type(inner)?;
                Ok(FieldType::Map(Box::new(value_type)))
            } else if upper_input.starts_with("EMBEDDING(") && upper_input.ends_with(")") {
                let dim_str = &input[10..input.len() - 1];
                let dimension = dim_str.parse::<usize>()
                    .map_err(|_| HyperQLError::simple_parse_error(
                        "Invalid embedding dimension",
                        input,
                        1,
                        1,
                    ))?;
                Ok(FieldType::Embedding(dimension))
            } else {
                Err(HyperQLError::simple_parse_error(
                    &format!("Unknown field type: {}", input),
                    input,
                    1,
                    1,
                ))
            }
        }
    }
}

/// Parse cascade configuration
fn parse_cascade_config(input: &str) -> Result<CascadeConfiguration> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    // Expect: AGGREGATE function FROM property [THROUGH edges direction] [WITH DECAY decay]
    
    if !upper_input.starts_with("AGGREGATE") {
        return Err(HyperQLError::simple_parse_error(
            "Cascade config must start with AGGREGATE",
            input,
            1,
            1,
        ));
    }
    
    let after_aggregate = &input["AGGREGATE".len()..].trim();
    
    // Parse aggregation function
    let (aggregation, rest) = parse_aggregation_function(after_aggregate)?;

    // Expect FROM
    let rest = rest.trim();
    let upper_rest = rest.to_uppercase();
    if !upper_rest.starts_with("FROM") {
        return Err(HyperQLError::simple_parse_error(
            "Expected FROM after aggregation function",
            input,
            1,
            1,
        ));
    }

    let after_from = &rest["FROM".len()..].trim();
    
    // Parse source property
    let parts: Vec<&str> = after_from.split_whitespace().collect();
    if parts.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Expected property name after FROM",
            input,
            1,
            1,
        ));
    }
    
    let source_property = parts[0].to_string();
    
    // Check for optional THROUGH clause
    let mut direction = Some(EdgeDirection::Incoming); // Default
    let mut edge_types = None;
    let mut decay = None;
    let timestamp_field = None;
    
    let rest_after_property = &after_from[source_property.len()..].trim();
    let upper_rest = rest_after_property.to_uppercase();
    
    if upper_rest.starts_with("THROUGH") {
        let after_through = &rest_after_property["THROUGH".len()..].trim();
        
        // Parse edge types and direction
        let (edges, dir, remaining) = parse_through_clause(after_through)?;
        edge_types = Some(edges);
        direction = Some(dir);

        // Check for WITH DECAY
        let remaining = remaining.trim();
        let upper_remaining = remaining.to_uppercase();
        if upper_remaining.starts_with("WITH") {
            let after_with = &remaining[4..].trim();
            if after_with.to_uppercase().starts_with("DECAY") {
                let after_decay = &after_with[5..].trim();
                let (decay_func, _) = parse_decay_function(after_decay)?;
                decay = Some(decay_func);
            }
        }
    } else if upper_rest.starts_with("WITH") {
        let after_with = &rest_after_property[4..].trim();
        if after_with.to_uppercase().starts_with("DECAY") {
            let after_decay = &after_with[5..].trim();
            let (decay_func, _) = parse_decay_function(after_decay)?;
            decay = Some(decay_func);
        }
    }
    
    Ok(CascadeConfiguration {
        aggregation,
        source_properties: vec![source_property],
        direction,
        edge_types,
        decay,
        timestamp_field,
        update_frequency: None,
    })
}

/// Parse aggregation function
fn parse_aggregation_function(input: &str) -> Result<(AggregationFunction, &str)> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    // Check for functions with parameters first
    if upper_input.starts_with("MOVINGMEAN(") || upper_input.starts_with("MOVING_MEAN(") || upper_input.starts_with("MOVINGAVG(") || upper_input.starts_with("MOVING_AVG(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis in aggregation function",
                input,
                1,
                1,
            ))?;

        let param_str = &input[paren_start + 1..paren_end];
        let window_days = param_str.parse::<u32>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid window_days parameter",
                input,
                1,
                1,
            ))?;

        let rest = &input[paren_end + 1..];
        return Ok((AggregationFunction::MovingMean { window_days }, rest));
    }

    if upper_input.starts_with("MOVINGMEDIAN(") || upper_input.starts_with("MOVING_MEDIAN(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis",
                input,
                1,
                1,
            ))?;
        
        let param_str = &input[paren_start + 1..paren_end];
        let window_days = param_str.parse::<u32>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid window_days parameter",
                input,
                1,
                1,
            ))?;
        
        let rest = &input[paren_end + 1..];
        return Ok((AggregationFunction::MovingMedian { window_days }, rest));
    }
    
    if upper_input.starts_with("TIMEWINDOWEDAVERAGE(") || upper_input.starts_with("TIME_WINDOWED_AVERAGE(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis",
                input,
                1,
                1,
            ))?;
        
        let param_str = &input[paren_start + 1..paren_end];
        let window_days = param_str.parse::<u32>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid window_days parameter",
                input,
                1,
                1,
            ))?;
        
        let rest = &input[paren_end + 1..];
        return Ok((AggregationFunction::TimeWindowedAverage { window_days }, rest));
    }
    
    if upper_input.starts_with("PERCENTILE(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis",
                input,
                1,
                1,
            ))?;
        
        let param_str = &input[paren_start + 1..paren_end];
        let percentile = param_str.parse::<u8>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid percentile parameter (must be 0-100)",
                input,
                1,
                1,
            ))?;
        
        if percentile > 100 {
            return Err(HyperQLError::simple_parse_error(
                "Percentile must be between 0 and 100",
                input,
                1,
                1,
            ));
        }
        
        let rest = &input[paren_end + 1..];
        return Ok((AggregationFunction::Percentile { percentile }, rest));
    }
    
    // Simple aggregation functions (no parameters)
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err(HyperQLError::simple_parse_error(
            "Expected aggregation function",
            input,
            1,
            1,
        ));
    }

    let func_name = parts[0].to_uppercase();
    // Find where the function name ends in the original string
    let func_pos = input.find(parts[0]).unwrap();
    let rest = &input[func_pos + parts[0].len()..];
    
    let function = match func_name.as_str() {
        "SUM" => AggregationFunction::Sum,
        "AVG" | "AVERAGE" => AggregationFunction::Average,
        "WEIGHTED_AVG" | "WEIGHTED_AVERAGE" => AggregationFunction::WeightedAverage,
        "MAX" => AggregationFunction::Max,
        "MIN" => AggregationFunction::Min,
        "COUNT" => AggregationFunction::Count,
        "LATEST" => AggregationFunction::Latest,
        "FIRST" => AggregationFunction::First,
        _ => {
            return Err(HyperQLError::simple_parse_error(
                &format!("Unknown aggregation function: {}", func_name),
                input,
                1,
                1,
            ));
        }
    };
    
    Ok((function, rest))
}

/// Parse THROUGH clause
fn parse_through_clause(input: &str) -> Result<(Vec<String>, EdgeDirection, &str)> {
    let input = input.trim();

    // Split on whitespace and process tokens
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let mut edge_types = Vec::new();
    let mut direction = EdgeDirection::Incoming; // Default
    let mut direction_index = None;

    for (i, token) in tokens.iter().enumerate() {
        let upper_token = token.to_uppercase();

        // Check if this is a direction keyword
        if upper_token == "INCOMING" {
            direction = EdgeDirection::Incoming;
            direction_index = Some(i);
            break;
        } else if upper_token == "OUTGOING" {
            direction = EdgeDirection::Outgoing;
            direction_index = Some(i);
            break;
        } else if upper_token == "BOTH" {
            direction = EdgeDirection::Both;
            direction_index = Some(i);
            break;
        } else {
            // It's an edge type
            edge_types.push(token.to_string());
        }
    }

    // Find the remaining string after the direction keyword in the original input
    let remaining_str = if let Some(dir_idx) = direction_index {
        if dir_idx + 1 < tokens.len() {
            // Find where the direction token starts in the original input
            let direction_token = tokens[dir_idx];
            if let Some(dir_pos) = input.find(direction_token) {
                let after_dir = &input[dir_pos + direction_token.len()..];
                // Return everything after the direction, trimmed of leading whitespace
                after_dir.trim_start()
            } else {
                ""
            }
        } else {
            ""
        }
    } else {
        ""
    };

    Ok((edge_types, direction, remaining_str))
}

/// Parse decay function
fn parse_decay_function(input: &str) -> Result<(DecayFunction, &str)> {
    let input = input.trim();
    let upper_input = input.to_uppercase();
    
    if upper_input.starts_with("EXPONENTIAL(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis",
                input,
                1,
                1,
            ))?;
        
        let alpha_str = &input[paren_start + 1..paren_end];
        let alpha = alpha_str.parse::<f32>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid alpha parameter",
                input,
                1,
                1,
            ))?;
        
        let rest = &input[paren_end + 1..];
        return Ok((DecayFunction::Exponential { alpha }, rest));
    }
    
    if upper_input.starts_with("POWERLAW(") || upper_input.starts_with("POWER_LAW(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis",
                input,
                1,
                1,
            ))?;
        
        let alpha_str = &input[paren_start + 1..paren_end];
        let alpha = alpha_str.parse::<f32>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid alpha parameter",
                input,
                1,
                1,
            ))?;
        
        let rest = &input[paren_end + 1..];
        return Ok((DecayFunction::PowerLaw { alpha }, rest));
    }
    
    if upper_input.starts_with("LINEAR(") {
        let paren_start = input.find('(').unwrap();
        let paren_end = input.find(')')
            .ok_or_else(|| HyperQLError::simple_parse_error(
                "Missing closing parenthesis",
                input,
                1,
                1,
            ))?;
        
        let alpha_str = &input[paren_start + 1..paren_end];
        let alpha = alpha_str.parse::<f32>()
            .map_err(|_| HyperQLError::simple_parse_error(
                "Invalid alpha parameter",
                input,
                1,
                1,
            ))?;
        
        let rest = &input[paren_end + 1..];
        return Ok((DecayFunction::Linear { alpha }, rest));
    }
    
    // Check for NONE
    if upper_input.starts_with("NONE") {
        let rest = &input["NONE".len()..];
        return Ok((DecayFunction::None, rest));
    }
    
    Err(HyperQLError::simple_parse_error(
        "Unknown decay function (expected EXPONENTIAL, POWERLAW, LINEAR, or NONE)",
        input,
        1,
        1,
    ))
}

/// Parse WITH clause for CREATE SCHEMA
fn parse_with_clause(input: &str) -> Result<(ExtensibilityMode, Option<String>)> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    let mut extensibility = ExtensibilityMode::Open; // Default
    let description = None;
    
    if upper_input.starts_with("WITH") {
        let after_with = &input["WITH".len()..].trim();
        
        // Parse EXTENSIBILITY
        if after_with.to_uppercase().starts_with("EXTENSIBILITY") {
            let after_ext = &after_with["EXTENSIBILITY".len()..].trim();
            extensibility = parse_extensibility_mode(after_ext)?;
        }
        
        // TODO: Parse DESCRIPTION if needed
    }
    
    Ok((extensibility, description))
}

/// Parse extensibility mode
fn parse_extensibility_mode(input: &str) -> Result<ExtensibilityMode> {
    let input = input.trim().trim_end_matches(';').trim();
    let upper_input = input.to_uppercase();
    
    let mode_str = upper_input.split_whitespace().next()
        .ok_or_else(|| HyperQLError::simple_parse_error(
            "Expected extensibility mode",
            input,
            1,
            1,
        ))?;
    
    match mode_str {
        "OPEN" => Ok(ExtensibilityMode::Open),
        "CLOSED" => Ok(ExtensibilityMode::Closed),
        "TYPED" => Ok(ExtensibilityMode::Typed),
        _ => Err(HyperQLError::simple_parse_error(
            "Unknown extensibility mode (expected OPEN, CLOSED, or TYPED)",
            input,
            1,
            1,
        )),
    }
}

/// Find matching closing parenthesis
fn find_matching_paren(input: &str, start: usize) -> Result<usize> {
    let mut depth = 1;
    let chars: Vec<char> = input.chars().collect();
    
    for i in (start + 1)..chars.len() {
        match chars[i] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            _ => {}
        }
    }
    
    Err(HyperQLError::simple_parse_error(
        "Unmatched parenthesis",
        input,
        1,
        1,
    ))
}
