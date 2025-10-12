//! # Schema AST Tests

#[cfg(test)]
mod tests {
    use super::super::*;
    
    #[test]
    fn test_create_schema_statement() {
        let stmt = CreateSchemaStatement {
            collection_name: "test_collection".to_string(),
            fields: vec![],
            extensibility: ExtensibilityMode::Open,
            description: Some("Test schema".to_string()),
            if_not_exists: true,
        };
        
        assert_eq!(stmt.collection_name, "test_collection");
        assert_eq!(stmt.extensibility, ExtensibilityMode::Open);
        assert!(stmt.if_not_exists);
        assert_eq!(stmt.description, Some("Test schema".to_string()));
    }
    
    #[test]
    fn test_field_definition_regular() {
        let field = FieldDefinition {
            name: "title".to_string(),
            field_type: FieldType::String,
            required: true,
            scope: Some(PropertyScope::DomainShared),
            kind: FieldKind::Regular,
            description: Some("Document title".to_string()),
        };
        
        assert_eq!(field.name, "title");
        assert_eq!(field.field_type, FieldType::String);
        assert!(field.required);
        assert_eq!(field.scope, Some(PropertyScope::DomainShared));
    }
    
    #[test]
    fn test_field_definition_cascaded() {
        let cascade_config = CascadeConfiguration {
            aggregation: AggregationFunction::Average,
            source_properties: vec!["sentiment".to_string()],
            direction: Some(EdgeDirection::Incoming),
            edge_types: None,
            decay: None,
            timestamp_field: None,
            update_frequency: None,
        };
        
        let field = FieldDefinition {
            name: "avg_sentiment".to_string(),
            field_type: FieldType::Float,
            required: false,
            scope: Some(PropertyScope::CollectionSpecific),
            kind: FieldKind::Cascaded(cascade_config.clone()),
            description: None,
        };
        
        assert_eq!(field.name, "avg_sentiment");
        assert_eq!(field.field_type, FieldType::Float);
        
        match field.kind {
            FieldKind::Cascaded(config) => {
                assert_eq!(config.aggregation, AggregationFunction::Average);
                assert_eq!(config.source_properties.len(), 1);
            },
            _ => panic!("Expected Cascaded field kind"),
        }
    }
    
    #[test]
    fn test_cascade_configuration() {
        let config = CascadeConfiguration {
            aggregation: AggregationFunction::Sum,
            source_properties: vec!["count".to_string(), "total".to_string()],
            direction: Some(EdgeDirection::Outgoing),
            edge_types: Some(vec!["CONTAINS".to_string()]),
            decay: Some(DecayFunction::Exponential { alpha: 0.1 }),
            timestamp_field: Some("created_at".to_string()),
            update_frequency: Some(3600),
        };
        
        assert_eq!(config.source_properties.len(), 2);
        assert_eq!(config.direction, Some(EdgeDirection::Outgoing));
        assert!(config.edge_types.is_some());
        assert!(config.decay.is_some());
        assert_eq!(config.update_frequency, Some(3600));
    }
    
    #[test]
    fn test_aggregation_functions() {
        let funcs = vec![
            AggregationFunction::Sum,
            AggregationFunction::Average,
            AggregationFunction::WeightedAverage,
            AggregationFunction::Max,
            AggregationFunction::Min,
            AggregationFunction::Count,
            AggregationFunction::Latest,
            AggregationFunction::First,
            AggregationFunction::MovingMean { window_days: 30 },
            AggregationFunction::MovingMedian { window_days: 7 },
            AggregationFunction::TimeWindowedAverage { window_days: 14 },
            AggregationFunction::Percentile { percentile: 95 },
        ];
        
        assert_eq!(funcs.len(), 12);
    }
    
    #[test]
    fn test_decay_functions() {
        let decay_none = DecayFunction::None;
        let decay_exp = DecayFunction::Exponential { alpha: 0.5 };
        let decay_power = DecayFunction::PowerLaw { alpha: 1.5 };
        let decay_linear = DecayFunction::Linear { alpha: 0.01 };
        
        assert!(matches!(decay_none, DecayFunction::None));
        assert!(matches!(decay_exp, DecayFunction::Exponential { .. }));
        assert!(matches!(decay_power, DecayFunction::PowerLaw { .. }));
        assert!(matches!(decay_linear, DecayFunction::Linear { .. }));
    }
    
    #[test]
    fn test_field_types() {
        let types = vec![
            FieldType::String,
            FieldType::Integer,
            FieldType::Float,
            FieldType::Boolean,
            FieldType::Timestamp,
            FieldType::Duration,
            FieldType::Date,
            FieldType::Json,
            FieldType::Array(Box::new(FieldType::String)),
            FieldType::Map(Box::new(FieldType::Integer)),
            FieldType::Reference,
            FieldType::Edge,
            FieldType::Embedding(512),
            FieldType::Position3D,
            FieldType::Bytes,
        ];
        
        assert_eq!(types.len(), 15);
        
        // Verify embedding dimension
        match types[12] {
            FieldType::Embedding(dim) => assert_eq!(dim, 512),
            _ => panic!("Expected Embedding type"),
        }
    }
    
    #[test]
    fn test_property_scopes() {
        let metadata = PropertyScope::Metadata;
        let collection = PropertyScope::CollectionSpecific;
        let domain = PropertyScope::DomainShared;
        
        assert!(matches!(metadata, PropertyScope::Metadata));
        assert!(matches!(collection, PropertyScope::CollectionSpecific));
        assert!(matches!(domain, PropertyScope::DomainShared));
    }
    
    #[test]
    fn test_alter_operations() {
        let add_field = AlterOperation::AddField(FieldDefinition {
            name: "new_field".to_string(),
            field_type: FieldType::Integer,
            required: false,
            scope: None,
            kind: FieldKind::Regular,
            description: None,
        });
        
        let drop_field = AlterOperation::DropField("old_field".to_string());
        
        let modify_field = AlterOperation::ModifyField {
            name: "existing_field".to_string(),
            definition: FieldDefinition {
                name: "existing_field".to_string(),
                field_type: FieldType::String,
                required: true,
                scope: None,
                kind: FieldKind::Regular,
                description: None,
            },
        };
        
        let set_ext = AlterOperation::SetExtensibility(ExtensibilityMode::Closed);
        
        assert!(matches!(add_field, AlterOperation::AddField(_)));
        assert!(matches!(drop_field, AlterOperation::DropField(_)));
        assert!(matches!(modify_field, AlterOperation::ModifyField { .. }));
        assert!(matches!(set_ext, AlterOperation::SetExtensibility(_)));
    }
    
    #[test]
    fn test_drop_schema_statement() {
        let stmt = DropSchemaStatement {
            collection_name: "old_collection".to_string(),
            if_exists: true,
        };
        
        assert_eq!(stmt.collection_name, "old_collection");
        assert!(stmt.if_exists);
    }
    
    #[test]
    fn test_describe_schema_statement() {
        let stmt = DescribeSchemaStatement {
            collection_name: "my_collection".to_string(),
            detailed: true,
        };
        
        assert_eq!(stmt.collection_name, "my_collection");
        assert!(stmt.detailed);
    }
    
    #[test]
    fn test_extensibility_modes() {
        let closed = ExtensibilityMode::Closed;
        let open = ExtensibilityMode::Open;
        let typed = ExtensibilityMode::Typed;
        
        assert!(matches!(closed, ExtensibilityMode::Closed));
        assert!(matches!(open, ExtensibilityMode::Open));
        assert!(matches!(typed, ExtensibilityMode::Typed));
    }
    
    #[test]
    fn test_schema_operation_enum() {
        let create = SchemaOperation::Create(CreateSchemaStatement {
            collection_name: "test".to_string(),
            fields: vec![],
            extensibility: ExtensibilityMode::Open,
            description: None,
            if_not_exists: false,
        });
        
        let drop = SchemaOperation::Drop(DropSchemaStatement {
            collection_name: "test".to_string(),
            if_exists: true,
        });
        
        assert!(matches!(create, SchemaOperation::Create(_)));
        assert!(matches!(drop, SchemaOperation::Drop(_)));
    }
    
    #[test]
    fn test_edge_directions() {
        let incoming = EdgeDirection::Incoming;
        let outgoing = EdgeDirection::Outgoing;
        let both = EdgeDirection::Both;
        
        assert!(matches!(incoming, EdgeDirection::Incoming));
        assert!(matches!(outgoing, EdgeDirection::Outgoing));
        assert!(matches!(both, EdgeDirection::Both));
    }
}
