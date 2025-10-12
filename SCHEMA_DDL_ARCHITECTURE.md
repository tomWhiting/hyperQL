# Schema DDL Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER INPUT (SQL)                         │
│  CREATE SCHEMA topics (id String, avg_score Float CASCADE(...)) │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PARSER (src/parser/)                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ statement.rs: parse_statement()                          │   │
│  │   if starts_with("CREATE SCHEMA") → schema parser        │   │
│  └──────────────────────┬───────────────────────────────────┘   │
│                         │                                        │
│  ┌──────────────────────▼───────────────────────────────────┐   │
│  │ schema.rs: parse_create_schema()                         │   │
│  │  • Extract collection name                               │   │
│  │  • Parse field specifications                            │   │
│  │  • Parse CASCADE configs                                 │   │
│  │  • Parse OPTIONS                                         │   │
│  └──────────────────────┬───────────────────────────────────┘   │
└─────────────────────────┼────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      AST (src/ast/schema/)                      │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Statement::Schema(SchemaOperation::CreateSchema(...))    │   │
│  └──────────────────────┬───────────────────────────────────┘   │
│                         │                                        │
│  ┌──────────────────────▼───────────────────────────────────┐   │
│  │ CreateSchemaStatement {                                  │   │
│  │   collection_name: "topics",                             │   │
│  │   fields: [                                              │   │
│  │     FieldSpecification {                                 │   │
│  │       name: "avg_score",                                 │   │
│  │       scope: PropertyScope::DomainShared,                │   │
│  │       kind: FieldKind::Cascaded,                         │   │
│  │       cascade_config: Some(CascadeConfig {               │   │
│  │         aggregation: Average,                            │   │
│  │         source_collection: "articles",                   │   │
│  │         ...                                              │   │
│  │       })                                                 │   │
│  │     }                                                    │   │
│  │   ],                                                     │   │
│  │   options: SchemaOptions { ... }                         │   │
│  │ }                                                        │   │
│  └──────────────────────┬───────────────────────────────────┘   │
└─────────────────────────┼────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                 VALIDATOR (src/validator/schema.rs)             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ validate_schema_ddl()                                    │   │
│  │  ✓ Check collection name unique                          │   │
│  │  ✓ Validate field types                                  │   │
│  │  ✓ Check CASCADE source exists (articles.score)          │   │
│  │  ✓ Validate edge type exists (tagged_with)               │   │
│  │  ✓ Detect circular dependencies                          │   │
│  └──────────────────────┬───────────────────────────────────┘   │
└─────────────────────────┼────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                  COMPILER (src/compiler/mod.rs)                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ compile(Statement::Schema(...))                          │   │
│  │  → ExecutionPlan::SchemaDDL(                             │   │
│  │      SchemaDDLOperation::CreateSchema(stmt)              │   │
│  │    )                                                     │   │
│  └──────────────────────┬───────────────────────────────────┘   │
└─────────────────────────┼────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│              EXECUTOR / SCHEMA ENGINE (hyperspatial)            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ execute_ddl(SchemaDDLOperation::CreateSchema(...))       │   │
│  │  1. Convert to Vec<FieldSpec>                            │   │
│  │  2. Register with SchemaEngine                           │   │
│  │  3. Create CascadeTriggerIndex entry                     │   │
│  │  4. Persist to schema database                           │   │
│  └──────────────────────┬───────────────────────────────────┘   │
└─────────────────────────┼────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       SCHEMA REGISTRY                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ topics: Schema {                                         │   │
│  │   fields: [                                              │   │
│  │     FieldSpec {                                          │   │
│  │       name: "avg_score",                                 │   │
│  │       scope: DomainShared,                               │   │
│  │       kind: Cascaded,                                    │   │
│  │       cascade_config: Some(...)                          │   │
│  │     }                                                    │   │
│  │   ]                                                      │   │
│  │ }                                                        │   │
│  │                                                          │   │
│  │ CascadeTriggerIndex:                                     │   │
│  │   articles.score → [topics.avg_score]                    │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### 1. Parse Phase
```
SQL String
  → parse_statement() detects "CREATE SCHEMA"
  → schema::parse_create_schema() extracts structure
  → Returns AST: Statement::Schema(SchemaOperation::CreateSchema(...))
```

### 2. Validation Phase
```
AST
  → validate_schema_ddl() checks:
      - No duplicate collection names
      - CASCADE sources exist
      - No circular dependencies
      - Field types valid
  → Returns ValidationResult
```

### 3. Compilation Phase
```
Validated AST
  → compile() matches Statement::Schema
  → Creates ExecutionPlan::SchemaDDL(...)
  → Returns CompiledQuery with metadata
```

### 4. Execution Phase
```
ExecutionPlan
  → execute_ddl() converts AST to FieldSpec
  → Registers schema in SchemaEngine
  → Creates cascade trigger entries
  → Persists to disk
  → Returns SchemaResult
```

---

## Key Integration Points

### Parser → AST
```rust
// In src/parser/schema.rs
pub fn parse_create_schema(input: &str) -> Result<Statement> {
    // Parse fields, cascade configs, options
    Ok(Statement::Schema(SchemaOperation::CreateSchema(
        CreateSchemaStatement { ... }
    )))
}
```

### AST → Compiler
```rust
// In src/compiler/mod.rs
match statement {
    Statement::Schema(schema_op) => {
        Ok(CompiledQuery {
            plan: ExecutionPlan::SchemaDDL(schema_op),
            metadata: QueryMetadata::ddl_metadata(),
            estimated_cost: ExecutionCost::metadata_operation(),
        })
    }
}
```

### Compiler → Executor
```rust
// In executor or SchemaEngine
match execution_plan {
    ExecutionPlan::SchemaDDL(op) => {
        match op {
            SchemaDDLOperation::CreateSchema(stmt) => {
                let field_specs = convert_to_field_specs(stmt.fields);
                schema_engine.register_schema(stmt.collection_name, field_specs)?;
                // ...
            }
        }
    }
}
```

---

## File Dependency Graph

```
src/ast/mod.rs
  └─ imports src/ast/schema/mod.rs
       └─ defines SchemaOperation
            └─ uses CreateSchemaStatement (from operations.rs)
                 └─ uses FieldSpecification (from field_spec.rs)
                      └─ uses CascadeConfig (from cascade.rs)

src/parser/statement.rs
  └─ imports src/parser/schema.rs
       └─ defines parse_create_schema()
            └─ returns Statement::Schema(...)

src/compiler/mod.rs
  └─ matches Statement::Schema
       └─ creates ExecutionPlan::SchemaDDL
            └─ passes to executor/SchemaEngine

src/validator/schema.rs
  └─ extends validate()
       └─ adds validate_schema_ddl()
            └─ validates CASCADE references
```

---

## Existing Infrastructure (Reuse)

### Already Implemented in hyperspatial
```rust
// src/persistence/schema/types.rs
pub enum PropertyScope {
    Metadata,
    CollectionSpecific,
    DomainShared,
}

pub enum FieldKind {
    Regular,
    Cascaded,
    Calculated,
    Computed,
}

pub struct CascadeConfig {
    pub aggregation: AggregationFunction,
    pub source_collection: String,
    pub source_field: String,
    pub edge_type: String,
    pub direction: EdgeDirection,
    pub decay: Option<DecayFunction>,
    // ...
}
```

**Decision:** Reuse these types from hyperspatial, import in HyperQL AST

---

## Error Flow

### Parse Error Example
```
Input: CREATE SCHEMA topics (name Invalid_Type)
                                      ↑
Parser: "Unknown field type 'Invalid_Type'"
  → HyperQLError::simple_parse_error(...)
  → User sees: "Parse error at line 1, col 34: Unknown field type"
```

### Validation Error Example
```
Input: CREATE SCHEMA topics (
         avg_score Float CASCADE (
           AGGREGATE Average,
           FROM nonexistent.score,  ← error here
           ...
         )
       )

Validator: "Source collection 'nonexistent' does not exist"
  → ValidationError { kind: SchemaError, message: "..." }
  → User sees: "Validation failed: Source collection 'nonexistent' not found"
```

### Execution Error Example
```
Schema Engine: Attempting to create duplicate schema
  → SchemaResult::Err("Schema 'topics' already exists")
  → User sees: "Schema creation failed: Collection 'topics' already exists"
```

---

## Testing Architecture

### Unit Tests (Parser)
```rust
#[test]
fn test_parse_cascade_config() {
    let cascade = "CASCADE (AGGREGATE Average, FROM articles.score, ...)";
    let config = parse_cascade_config(cascade).unwrap();
    assert_eq!(config.aggregation, AggregationFunction::Average);
    assert_eq!(config.source_collection, "articles");
    assert_eq!(config.source_field, "score");
}
```

### Unit Tests (Validator)
```rust
#[test]
fn test_validate_circular_cascade() {
    let mut validator = SchemaValidator::new();
    // Setup: topics CASCADE from articles, articles CASCADE from topics
    let result = validator.validate_schema_ddl(...);
    assert!(result.has_error("Circular cascade dependency"));
}
```

### Integration Tests
```rust
#[test]
fn test_end_to_end_create_schema() {
    let sql = "CREATE SCHEMA topics (...)";
    let stmt = parse_statement(sql).unwrap();
    let compiled = compiler.compile(stmt).unwrap();
    let result = executor.execute(compiled.plan).unwrap();
    
    // Verify schema exists
    let schema = schema_engine.get_schema("topics").unwrap();
    assert_eq!(schema.fields.len(), 3);
}
```

---

## Performance Considerations

### Parse Performance
- **Estimated:** ~1-5ms for typical schema (10 fields)
- **Optimization:** Pre-compile regex patterns for field types
- **Bottleneck:** CASCADE config parsing (nested parentheses)

### Validation Performance
- **Estimated:** ~5-20ms (depends on cascade depth)
- **Optimization:** Cache schema lookups during validation
- **Bottleneck:** Circular dependency detection (graph traversal)

### Execution Performance
- **Estimated:** ~10-50ms for schema creation
- **Optimization:** Batch field registration
- **Bottleneck:** Disk I/O for schema persistence

### Runtime Impact
- **Property filtering:** O(1) lookup per field (pre-filtered metadata)
- **CASCADE triggers:** O(1) lookup in CascadeTriggerIndex
- **Schema validation:** O(n) where n = number of fields (query time)

---

## Migration Path

### Phase 1: Parser Only (No Breaking Changes)
- Implement AST and parser
- Return unimplemented!() from executor
- Users can parse DDL, get AST, but not execute
- **Zero runtime impact**

### Phase 2: Validation (No Breaking Changes)
- Add validation logic
- Still no execution
- Users get validation errors, helpful for development
- **Zero runtime impact**

### Phase 3: Execution (Opt-in)
- Implement SchemaEngine integration
- Feature flag: `enable_schema_ddl`
- Users opt-in to execution
- **Controlled rollout**

### Phase 4: Full Production
- Enable by default
- Deprecate old schema registration methods
- Full CASCADE support
- **Full functionality**

---

## Next Steps Checklist

### Prerequisites
- [x] Understand existing Stream DDL pattern
- [x] Identify Statement enum structure
- [x] Map parser entry points
- [x] Document cascade requirements

### Phase 1: AST (Start Here)
- [ ] Create `src/ast/schema/mod.rs`
- [ ] Define `SchemaOperation` enum
- [ ] Create `operations.rs` with statement structs
- [ ] Create `field_spec.rs` with FieldSpecification
- [ ] Create `cascade.rs` with CascadeConfig
- [ ] Add `Statement::Schema` variant to `src/ast/mod.rs`

### Phase 2: Parser
- [ ] Create `src/parser/schema.rs`
- [ ] Implement `parse_create_schema()`
- [ ] Implement `parse_alter_schema()`
- [ ] Implement `parse_drop_schema()`
- [ ] Implement `parse_describe_schema()`
- [ ] Add dispatch in `src/parser/statement.rs`

### Phase 3: Tests
- [ ] Write parser tests (50+ cases)
- [ ] Write validation tests (30+ cases)
- [ ] Write integration tests (20+ cases)

### Phase 4: Execution
- [ ] Extend compiler for SchemaDDL
- [ ] Implement SchemaEngine integration
- [ ] Add error handling
- [ ] Schema persistence

---

**Ready to Begin Implementation**

Start with Phase 1: Create the AST structures in `src/ast/schema/mod.rs`

