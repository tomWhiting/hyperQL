//! # Schema DDL Parser Tests
//!
//! Comprehensive tests for schema DDL statement parsing including CREATE SCHEMA,
//! ALTER SCHEMA, DROP SCHEMA, and DESCRIBE SCHEMA statements.

use hyperQL::parser::parse_statement;
use hyperQL::ast::{Statement, schema::*};

#[test]
fn test_parse_create_schema_simple() {
    let query = "CREATE SCHEMA topics (
        id String REQUIRED,
        name String REQUIRED
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse simple CREATE SCHEMA: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            assert_eq!(stmt.fields.len(), 2);
            assert_eq!(stmt.fields[0].name, "id");
            assert_eq!(stmt.fields[1].name, "name");
            assert!(stmt.fields[0].required);
            assert!(stmt.fields[1].required);
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_create_schema_with_scopes() {
    let query = "CREATE SCHEMA documents (
        id String REQUIRED METADATA,
        title String REQUIRED COLLECTION_SPECIFIC,
        sentiment Float DOMAIN_SHARED
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse CREATE SCHEMA with scopes: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.fields.len(), 3);
            assert_eq!(stmt.fields[0].scope, Some(PropertyScope::Metadata));
            assert_eq!(stmt.fields[1].scope, Some(PropertyScope::CollectionSpecific));
            assert_eq!(stmt.fields[2].scope, Some(PropertyScope::DomainShared));
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_create_schema_with_cascade() {
    let query = "CREATE SCHEMA topics (
        name String REQUIRED,
        avg_sentiment Float CASCADED AS AGGREGATE Average FROM sentiment THROUGH tagged_with INCOMING
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse CASCADE field: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.fields.len(), 2);
            
            // Check cascaded field
            match &stmt.fields[1].kind {
                FieldKind::Cascaded(config) => {
                    assert_eq!(config.aggregation, AggregationFunction::Average);
                    assert_eq!(config.source_properties, vec!["sentiment"]);
                    assert_eq!(config.direction, Some(EdgeDirection::Incoming));
                    assert_eq!(config.edge_types, Some(vec!["tagged_with".to_string()]));
                },
                _ => panic!("Expected Cascaded field kind"),
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_create_schema_with_complex_types() {
    let query = "CREATE SCHEMA complex (
        tags Array(String) OPTIONAL,
        metadata Map(String) OPTIONAL,
        embedding Embedding(768) REQUIRED
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse complex types: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.fields.len(), 3);
            
            // Check Array type
            match &stmt.fields[0].field_type {
                FieldType::Array(inner) => {
                    assert_eq!(**inner, FieldType::String);
                },
                _ => panic!("Expected Array type"),
            }
            
            // Check Map type
            match &stmt.fields[1].field_type {
                FieldType::Map(inner) => {
                    assert_eq!(**inner, FieldType::String);
                },
                _ => panic!("Expected Map type"),
            }
            
            // Check Embedding type
            match &stmt.fields[2].field_type {
                FieldType::Embedding(dim) => {
                    assert_eq!(*dim, 768);
                },
                _ => panic!("Expected Embedding type"),
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_create_schema_with_extensibility() {
    let query = "CREATE SCHEMA test (
        id String REQUIRED
    ) WITH EXTENSIBILITY Closed";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse WITH EXTENSIBILITY: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.extensibility, ExtensibilityMode::Closed);
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_cascade_with_decay() {
    let query = "CREATE SCHEMA test (
        score Float CASCADED AS AGGREGATE Sum FROM rating THROUGH edges INCOMING WITH DECAY Exponential(0.5)
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse CASCADE with DECAY: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            match &stmt.fields[0].kind {
                FieldKind::Cascaded(config) => {
                    assert_eq!(config.aggregation, AggregationFunction::Sum);
                    match config.decay {
                        Some(DecayFunction::Exponential { alpha }) => {
                            assert_eq!(alpha, 0.5);
                        },
                        _ => panic!("Expected Exponential decay"),
                    }
                },
                _ => panic!("Expected Cascaded field"),
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_cascade_with_temporal_functions() {
    let query = "CREATE SCHEMA test (
        moving_avg Float CASCADED AS AGGREGATE MovingMean(30) FROM value,
        percentile Float CASCADED AS AGGREGATE Percentile(95) FROM score
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse temporal aggregations: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.fields.len(), 2);
            
            // Check MovingMean
            match &stmt.fields[0].kind {
                FieldKind::Cascaded(config) => {
                    match config.aggregation {
                        AggregationFunction::MovingMean { window_days } => {
                            assert_eq!(window_days, 30);
                        },
                        _ => panic!("Expected MovingMean aggregation"),
                    }
                },
                _ => panic!("Expected Cascaded field"),
            }
            
            // Check Percentile
            match &stmt.fields[1].kind {
                FieldKind::Cascaded(config) => {
                    match config.aggregation {
                        AggregationFunction::Percentile { percentile } => {
                            assert_eq!(percentile, 95);
                        },
                        _ => panic!("Expected Percentile aggregation"),
                    }
                },
                _ => panic!("Expected Cascaded field"),
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_alter_schema_add_field() {
    let query = "ALTER SCHEMA topics ADD FIELD description String OPTIONAL";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse ALTER SCHEMA ADD FIELD: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Alter(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            match stmt.operation {
                AlterOperation::AddField(field) => {
                    assert_eq!(field.name, "description");
                    assert_eq!(field.field_type, FieldType::String);
                    assert!(!field.required);
                },
                _ => panic!("Expected AddField operation"),
            }
        },
        _ => panic!("Expected Schema Alter statement"),
    }
}

#[test]
fn test_parse_alter_schema_drop_field() {
    let query = "ALTER SCHEMA topics DROP FIELD old_field";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse ALTER SCHEMA DROP FIELD: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Alter(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            match stmt.operation {
                AlterOperation::DropField(name) => {
                    assert_eq!(name, "old_field");
                },
                _ => panic!("Expected DropField operation"),
            }
        },
        _ => panic!("Expected Schema Alter statement"),
    }
}

#[test]
fn test_parse_alter_schema_modify_field() {
    let query = "ALTER SCHEMA topics MODIFY FIELD name String REQUIRED DOMAIN_SHARED";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse ALTER SCHEMA MODIFY FIELD: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Alter(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            match stmt.operation {
                AlterOperation::ModifyField { name, definition } => {
                    assert_eq!(name, "name");
                    assert!(definition.required);
                    assert_eq!(definition.scope, Some(PropertyScope::DomainShared));
                },
                _ => panic!("Expected ModifyField operation"),
            }
        },
        _ => panic!("Expected Schema Alter statement"),
    }
}

#[test]
fn test_parse_drop_schema() {
    let query = "DROP SCHEMA topics";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse DROP SCHEMA: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Drop(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            assert!(!stmt.if_exists);
        },
        _ => panic!("Expected Schema Drop statement"),
    }
}

#[test]
fn test_parse_drop_schema_if_exists() {
    let query = "DROP SCHEMA IF EXISTS topics";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse DROP SCHEMA IF EXISTS: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Drop(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            assert!(stmt.if_exists);
        },
        _ => panic!("Expected Schema Drop statement"),
    }
}

#[test]
fn test_parse_describe_schema() {
    let query = "DESCRIBE SCHEMA topics";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse DESCRIBE SCHEMA: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Describe(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            assert!(!stmt.detailed);
        },
        _ => panic!("Expected Schema Describe statement"),
    }
}

#[test]
fn test_parse_describe_schema_detailed() {
    let query = "DESCRIBE SCHEMA topics DETAILED";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse DESCRIBE SCHEMA DETAILED: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Describe(stmt)) => {
            assert_eq!(stmt.collection_name, "topics");
            assert!(stmt.detailed);
        },
        _ => panic!("Expected Schema Describe statement"),
    }
}

#[test]
fn test_parse_all_field_types() {
    let query = "CREATE SCHEMA test (
        str_field String,
        int_field Integer,
        float_field Float,
        bool_field Boolean,
        ts_field Timestamp,
        dur_field Duration,
        date_field Date,
        json_field Json,
        ref_field Reference,
        edge_field Edge,
        pos_field Position3D,
        bytes_field Bytes
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse all field types: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.fields.len(), 12);
            
            let types = vec![
                FieldType::String,
                FieldType::Integer,
                FieldType::Float,
                FieldType::Boolean,
                FieldType::Timestamp,
                FieldType::Duration,
                FieldType::Date,
                FieldType::Json,
                FieldType::Reference,
                FieldType::Edge,
                FieldType::Position3D,
                FieldType::Bytes,
            ];
            
            for (i, expected_type) in types.iter().enumerate() {
                assert_eq!(&stmt.fields[i].field_type, expected_type, 
                          "Field {} type mismatch", i);
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_all_aggregation_functions() {
    let query = "CREATE SCHEMA test (
        sum_field Float CASCADED AS AGGREGATE Sum FROM value,
        avg_field Float CASCADED AS AGGREGATE Average FROM value,
        max_field Float CASCADED AS AGGREGATE Max FROM value,
        min_field Float CASCADED AS AGGREGATE Min FROM value,
        count_field Integer CASCADED AS AGGREGATE Count FROM value,
        latest_field Float CASCADED AS AGGREGATE Latest FROM value,
        first_field Float CASCADED AS AGGREGATE First FROM value
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse aggregation functions: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            assert_eq!(stmt.fields.len(), 7);
            
            let expected_funcs = vec![
                AggregationFunction::Sum,
                AggregationFunction::Average,
                AggregationFunction::Max,
                AggregationFunction::Min,
                AggregationFunction::Count,
                AggregationFunction::Latest,
                AggregationFunction::First,
            ];
            
            for (i, expected_func) in expected_funcs.iter().enumerate() {
                match &stmt.fields[i].kind {
                    FieldKind::Cascaded(config) => {
                        assert_eq!(&config.aggregation, expected_func,
                                  "Field {} aggregation mismatch", i);
                    },
                    _ => panic!("Expected Cascaded field at index {}", i),
                }
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_edge_directions() {
    let query = "CREATE SCHEMA test (
        incoming Float CASCADED AS AGGREGATE Sum FROM value THROUGH edges INCOMING,
        outgoing Float CASCADED AS AGGREGATE Sum FROM value THROUGH edges OUTGOING,
        both Float CASCADED AS AGGREGATE Sum FROM value THROUGH edges BOTH
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse edge directions: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            let directions = vec![
                EdgeDirection::Incoming,
                EdgeDirection::Outgoing,
                EdgeDirection::Both,
            ];
            
            for (i, expected_dir) in directions.iter().enumerate() {
                match &stmt.fields[i].kind {
                    FieldKind::Cascaded(config) => {
                        assert_eq!(config.direction.as_ref().unwrap(), expected_dir,
                                  "Field {} direction mismatch", i);
                    },
                    _ => panic!("Expected Cascaded field"),
                }
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}

#[test]
fn test_parse_multiple_edge_types() {
    let query = "CREATE SCHEMA test (
        score Float CASCADED AS AGGREGATE Average FROM rating THROUGH tagged_with related_to INCOMING
    )";
    
    let result = parse_statement(query);
    assert!(result.is_ok(), "Failed to parse multiple edge types: {:?}", result.err());
    
    match result.unwrap() {
        Statement::Schema(SchemaOperation::Create(stmt)) => {
            match &stmt.fields[0].kind {
                FieldKind::Cascaded(config) => {
                    let edge_types = config.edge_types.as_ref().unwrap();
                    assert_eq!(edge_types.len(), 2);
                    assert!(edge_types.contains(&"tagged_with".to_string()));
                    assert!(edge_types.contains(&"related_to".to_string()));
                },
                _ => panic!("Expected Cascaded field"),
            }
        },
        _ => panic!("Expected Schema Create statement"),
    }
}
