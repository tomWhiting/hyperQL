# Schema DDL Implementation Summary

## Quick Reference

### What Exists
- Stream DDL AST (stub, not parsed/compiled)
- SchemaValidator (runtime validation only)
- Statement enum (4 DML variants only)
- Parser with keyword dispatch pattern

### What's Needed
- Schema DDL AST structures
- Schema DDL parser
- Statement::Schema variant
- Compiler integration
- Validator extensions

---

## File Impact Analysis

### CREATE (5 new files)
```
src/ast/schema/
├── mod.rs              # SchemaOperation enum
├── operations.rs       # CreateSchema, AlterSchema, etc.
├── field_spec.rs       # FieldSpecification struct
└── cascade.rs          # CascadeConfig struct

src/parser/schema.rs    # Schema DDL parser
```

### MODIFY (5 existing files)
```
src/ast/mod.rs          # Add Statement::Schema variant
src/parser/mod.rs       # Re-export schema parser
src/parser/statement.rs # Add schema dispatch (4 new keywords)
src/compiler/mod.rs     # Add SchemaDDL execution plan
src/validator/schema.rs # Extend for DDL validation
```

---

## Syntax Examples

### CREATE SCHEMA
```sql
CREATE SCHEMA topics (
    id String REQUIRED METADATA,
    name String REQUIRED DOMAIN_SHARED,
    avg_score Float DOMAIN_SHARED CASCADE (
        AGGREGATE Average,
        FROM articles.score,
        THROUGH tagged_with INCOMING,
        DECAY Exponential(0.1)
    )
) OPTIONS (node_class_default = 'Passive');
```

### ALTER SCHEMA
```sql
ALTER SCHEMA topics ADD FIELD article_count Int DOMAIN_SHARED;
ALTER SCHEMA topics DROP FIELD old_field;
ALTER SCHEMA topics ALTER FIELD score SET SCOPE DOMAIN_SHARED;
```

### DROP/DESCRIBE
```sql
DROP SCHEMA topics IF EXISTS CASCADE;
DESCRIBE SCHEMA topics DETAILED;
```

---

## Implementation Phases

### Phase 1: Foundation (4-6 hours)
✓ AST structures  
✓ Statement::Schema variant  
✓ Basic parser skeleton  

### Phase 2: Core Parsing (6-8 hours)
✓ CREATE/DROP/DESCRIBE parsing  
✓ Field specification parsing  
✓ PropertyScope/FieldKind parsing  

### Phase 3: Advanced (8-12 hours)
✓ CASCADE config parsing  
✓ Aggregation/Decay functions  
✓ Validation integration  

### Phase 4: Execution (4-6 hours)
✓ Compiler integration  
✓ SchemaEngine execution  
✓ Error handling  

### Phase 5: Testing (6-8 hours)
✓ Parser tests  
✓ Validation tests  
✓ Integration tests  

**Total: 28-40 hours (3.5-5 days)**

---

## Complexity Heatmap

| Component | Complexity | Time | Critical Path? |
|-----------|-----------|------|----------------|
| Basic AST | ⭐ Low | 2h | ✓ |
| Parser dispatch | ⭐ Low | 1h | ✓ |
| Field spec parsing | ⭐⭐ Medium | 4h | ✓ |
| CASCADE parsing | ⭐⭐⭐ High | 8h | ✓ |
| Validation | ⭐⭐ Medium | 6h | - |
| Compiler | ⭐⭐ Medium | 4h | ✓ |
| Testing | ⭐⭐ Medium | 6h | - |

---

## Parser Pattern

### Existing (from statement.rs)
```rust
if upper_input.starts_with("SELECT") {
    select::parse_select_statement(input)
} else if upper_input.starts_with("INSERT") {
    parse_insert_statement(input)
}
```

### Addition Needed
```rust
} else if upper_input.starts_with("CREATE SCHEMA") {
    schema::parse_create_schema(input)
} else if upper_input.starts_with("ALTER SCHEMA") {
    schema::parse_alter_schema(input)
} else if upper_input.starts_with("DROP SCHEMA") {
    schema::parse_drop_schema(input)
} else if upper_input.starts_with("DESCRIBE SCHEMA") {
    schema::parse_describe_schema(input)
```

---

## AST Pattern

### Stream DDL (reference)
```rust
pub enum StreamOperation {
    CreateStream(CreateStreamStatement),
    Produce(ProduceStatement),
    Consume(ConsumeStatement),
    DropStream(DropStreamStatement),
    DescribeStream(DescribeStreamStatement),
}
```

### Schema DDL (proposed)
```rust
pub enum SchemaOperation {
    CreateSchema(CreateSchemaStatement),
    AlterSchema(AlterSchemaStatement),
    DropSchema(DropSchemaStatement),
    DescribeSchema(DescribeSchemaStatement),
}
```

### Statement Integration
```rust
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Schema(SchemaOperation),  // NEW
}
```

---

## Key Data Structures

### CreateSchemaStatement
```rust
pub struct CreateSchemaStatement {
    pub collection_name: String,
    pub fields: Vec<FieldSpecification>,
    pub options: SchemaOptions,
}
```

### FieldSpecification
```rust
pub struct FieldSpecification {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub scope: PropertyScope,      // METADATA | COLLECTION_SPECIFIC | DOMAIN_SHARED
    pub kind: FieldKind,            // Regular | Cascaded | Calculated | Computed
    pub cascade_config: Option<CascadeConfig>,
    pub default_value: Option<Expression>,
}
```

### CascadeConfig
```rust
pub struct CascadeConfig {
    pub aggregation: AggregationFunction,
    pub source_collection: String,
    pub source_field: String,
    pub edge_type: String,
    pub direction: EdgeDirection,
    pub decay: Option<DecayFunction>,
    pub temporal_window: Option<TemporalWindow>,
}
```

---

## Critical Success Factors

### Must Have ✓
- Robust CASCADE parsing
- Clear error messages
- SchemaEngine integration
- Backward compatibility
- Comprehensive tests

### Nice to Have
- Schema migration support
- Schema versioning
- Import/export tools
- Auto-completion
- Visualization

### Risks ⚠
- CASCADE syntax complexity
- Edge case handling
- Performance impact
- Breaking changes
- Circular dependencies

---

## Testing Strategy

### Parser Tests (50+ tests)
- Basic CREATE/DROP/ALTER/DESCRIBE
- Field specifications
- CASCADE configurations
- Options parsing
- Error cases

### Validation Tests (30+ tests)
- Duplicate names
- Invalid types
- CASCADE validation
- Circular dependencies
- Scope consistency

### Integration Tests (20+ tests)
- End-to-end workflows
- Schema persistence
- Multi-collection scenarios
- Error recovery

---

## Next Actions

1. **Read full report:** `SCHEMA_DDL_INVESTIGATION_REPORT.md`
2. **Create AST:** Start with `src/ast/schema/mod.rs`
3. **Add parser:** Implement `src/parser/schema.rs`
4. **Update dispatch:** Modify `src/parser/statement.rs`
5. **Test incrementally:** Write tests as you go

---

## References

- Full Report: `SCHEMA_DDL_INVESTIGATION_REPORT.md`
- Stream DDL: `src/ast/streams/operations.rs`
- Parser Entry: `src/parser/statement.rs`
- Schema Types: `src/persistence/schema/types.rs`
- Stage 4 Plan: `STAGE4_STAGE5_PLAN.md`

