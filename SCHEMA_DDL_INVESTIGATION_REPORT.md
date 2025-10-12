# HyperQL DDL Implementation Pattern Report

**Investigation Date:** 2025-10-13  
**Purpose:** Understand existing DDL patterns to implement Schema DDL statements

---

## Executive Summary

HyperQL currently has **Stream DDL infrastructure** (`CREATE STREAM`, `DROP STREAM`, `DESCRIBE STREAM`) but **no schema DDL implementation**. The Stream DDL provides a clear pattern to follow. The Statement enum contains only DML (SELECT, INSERT, UPDATE, DELETE) - no DDL variants exist yet.

**Key Finding:** Stream DDL is **defined in AST but not parsed or compiled**. This is stub infrastructure.

---

## 1. Stream DDL Pattern (Reference Implementation)

### AST Structure (`src/ast/streams/operations.rs`)

Stream operations are defined in a **separate enum**:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreamOperation {
    CreateStream(CreateStreamStatement),
    Produce(ProduceStatement),
    Consume(ConsumeStatement),
    DropStream(DropStreamStatement),
    DescribeStream(DescribeStreamStatement),
}
```

### CREATE STREAM Statement Structure

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateStreamStatement {
    pub name: String,
    pub config: StreamConfig,
    pub source_query: Option<String>,
    pub schema: Option<StreamSchema>,
}

pub struct StreamConfig {
    pub partitions: Option<u32>,
    pub retention_hours: Option<u64>,
    pub hyperbolic_positioning: Option<bool>,
    pub compression: Option<String>,
    pub properties: HashMap<String, String>,
}

pub struct StreamSchema {
    pub fields: Vec<StreamField>,
    pub primary_key: Option<Vec<String>>,
}

pub struct StreamField {
    pub name: String,
    pub field_type: String,
    pub optional: bool,
    pub default: Option<String>,
}
```

**Pattern Observations:**
- **Nested structs** for complex configuration
- **HashMap** for extensible custom properties
- **Option types** for optional parameters
- **Vec** for lists of fields/keys
- **Separate schema struct** for field definitions

---

## 2. Current Statement Enum (`src/ast/mod.rs:130-139`)

```rust
pub enum Statement {
    /// SELECT statement for data retrieval
    Select(SelectStatement),
    /// INSERT statement for data insertion
    Insert(InsertStatement),
    /// UPDATE statement for data modification
    Update(UpdateStatement),
    /// DELETE statement for data removal
    Delete(DeleteStatement),
}
```

**Current State:**
- Only **4 DML variants** (all data manipulation)
- No DDL variants
- Clean separation of statement types

**How to Extend:**
Add new variants for DDL operations:
```rust
pub enum Statement {
    // Existing DML
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    
    // New DDL variants
    CreateSchema(CreateSchemaStatement),
    AlterSchema(AlterSchemaStatement),
    DropSchema(DropSchemaStatement),
    DescribeSchema(DescribeSchemaStatement),
}
```

---

## 3. Parser Entry Point (`src/parser/statement.rs:5-25`)

### Keyword-Based Dispatch Pattern

```rust
pub fn parse_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    if upper_input.starts_with("SELECT") {
        select::parse_select_statement(input)
    } else if upper_input.starts_with("INSERT") {
        parse_insert_statement(input)
    } else if upper_input.starts_with("UPDATE") {
        parse_update_statement(input)
    } else if upper_input.starts_with("DELETE") {
        parse_delete_statement(input)
    } else {
        Err(HyperQLError::simple_parse_error(
            "Unsupported statement type. Supported: SELECT, INSERT, UPDATE, DELETE",
            input,
            1,
            1,
        ))
    }
}
```

**Pattern Details:**
- **Simple string matching** on uppercase input
- **starts_with()** for keyword detection
- **Delegated parsing** to specialized functions
- **Error for unsupported** statements

**Integration Point for Schema DDL:**
```rust
} else if upper_input.starts_with("CREATE SCHEMA") {
    schema::parse_create_schema_statement(input)
} else if upper_input.starts_with("ALTER SCHEMA") {
    schema::parse_alter_schema_statement(input)
} else if upper_input.starts_with("DROP SCHEMA") {
    schema::parse_drop_schema_statement(input)
} else if upper_input.starts_with("DESCRIBE SCHEMA") {
    schema::parse_describe_schema_statement(input)
```

---

## 4. Existing Schema Infrastructure

### Schema Validator (`src/validator/schema.rs`)

**Already exists** for runtime validation:
```rust
pub struct SchemaValidator {
    config: ValidationConfig,
    schema: Option<HashMap<String, Vec<String>>>,
}
```

Uses simple `HashMap<String, Vec<String>>` mapping table names to column names.

### Compiler EntitySchema (`src/compiler/expression.rs:15`)

```rust
pub struct EntitySchema {
    // Details not visible in grep output
}
```

### Executor TableSchema (`src/executor/mod.rs:73`)

```rust
pub struct TableSchema {
    // Used for runtime schema information
}
```

**Key Insight:** Multiple schema representations exist for different purposes (validation, compilation, execution). DDL schemas need to be **more comprehensive** than these.

---

## 5. Compiler Integration Pattern

### Statement Compilation (`src/compiler/mod.rs:327-393`)

```rust
pub fn compile(&self, statement: Statement) -> Result<CompiledQuery> {
    let plan = match &statement {
        Statement::Select(select) => self.select_compiler.compile_select(select.clone())?,
        Statement::Insert(insert) => self.statement_compiler.compile_insert(insert.clone())?,
        Statement::Update(update) => self.statement_compiler.compile_update(update.clone())?,
        Statement::Delete(delete) => self.statement_compiler.compile_delete(delete.clone())?,
    };
    // ... metadata and cost estimation
}
```

**Pattern:**
- **Match on statement variant**
- **Delegate to specialized compilers**
- **Generate metadata and cost estimates**
- **Return CompiledQuery with ExecutionPlan**

**DDL Integration:**
DDL statements (CREATE/ALTER/DROP SCHEMA) are **metadata operations**, not query plans:
```rust
Statement::CreateSchema(schema) => {
    // No execution plan needed - this is metadata
    // Instead: validate and store schema definition
    return Ok(CompiledQuery {
        plan: ExecutionPlan::SchemaDDL(SchemaDDLOperation::Create(schema)),
        metadata: QueryMetadata::ddl_metadata(),
        estimated_cost: ExecutionCost::metadata_operation(),
    });
}
```

---

## 6. Recommended Schema DDL Syntax

Based on stream DDL patterns and hyperspatial requirements:

### CREATE SCHEMA

```sql
CREATE SCHEMA <collection_name> (
    <field_name> <field_type> [REQUIRED] [METADATA|COLLECTION_SPECIFIC|DOMAIN_SHARED] [CASCADE_CONFIG],
    ...
) [OPTIONS (
    node_class_default = 'Active'|'Passive',
    collection_type = 'content'|'taxonomy'
)];
```

**Example: Topic Schema with Cascaded Field**

```sql
CREATE SCHEMA topics (
    id String REQUIRED METADATA,
    name String REQUIRED DOMAIN_SHARED,
    created_at Timestamp REQUIRED METADATA,
    avg_score Float DOMAIN_SHARED CASCADE (
        AGGREGATE Average,
        FROM articles.score,
        THROUGH tagged_with INCOMING,
        DECAY Exponential(0.1)
    ),
    article_count Int DOMAIN_SHARED CASCADE (
        AGGREGATE Count,
        FROM articles,
        THROUGH tagged_with INCOMING
    )
) OPTIONS (
    node_class_default = 'Passive'
);
```

**Example: Article Schema (Content)**

```sql
CREATE SCHEMA articles (
    id String REQUIRED METADATA,
    title String REQUIRED DOMAIN_SHARED,
    content String COLLECTION_SPECIFIC,
    score Float DOMAIN_SHARED,
    sentiment Float DOMAIN_SHARED,
    session_id String METADATA,
    created_at Timestamp METADATA
) OPTIONS (
    node_class_default = 'Active'
);
```

### ALTER SCHEMA

```sql
-- Add field
ALTER SCHEMA <collection_name> 
    ADD FIELD <field_name> <field_type> [REQUIRED] [SCOPE] [CASCADE_CONFIG];

-- Drop field
ALTER SCHEMA <collection_name> 
    DROP FIELD <field_name>;

-- Modify field
ALTER SCHEMA <collection_name> 
    MODIFY FIELD <field_name> <new_type> [new_attributes];

-- Change field scope
ALTER SCHEMA <collection_name>
    ALTER FIELD <field_name> SET SCOPE DOMAIN_SHARED;
```

### DROP SCHEMA

```sql
DROP SCHEMA <collection_name> [IF EXISTS] [CASCADE];
```

### DESCRIBE SCHEMA

```sql
DESCRIBE SCHEMA <collection_name> [DETAILED];
```

---

## 7. Implementation Estimate

### Lines of Code by Component

| Component | Estimated LOC | Notes |
|-----------|--------------|-------|
| **AST Definitions** | 200-250 | CreateSchemaStatement, AlterSchemaStatement, field specs, cascade config |
| **Parser Implementation** | 300-400 | Keyword parsing, field parsing, cascade config parsing |
| **Validator Integration** | 150-200 | Schema conflict detection, field type validation |
| **Compiler/Executor** | 200-250 | Schema storage, SchemaEngine integration |
| **Tests** | 200-250 | Parser tests, validation tests, integration tests |
| **Total** | **1,050-1,350 LOC** | ~1,200 LOC average |

### Complexity Breakdown

**Low Complexity (2-3 hours):**
- Basic AST structures
- Simple parser dispatch
- DROP/DESCRIBE parsing

**Medium Complexity (4-6 hours):**
- Field specification parsing
- PropertyScope enum integration
- FieldKind parsing
- ALTER SCHEMA variations

**High Complexity (6-10 hours):**
- CASCADE configuration parsing
- Aggregation function parsing
- Decay function parsing
- Temporal window specifications
- Integration with existing SchemaEngine

**Total Implementation Time:** 12-19 hours

---

## 8. Proposed Schema DDL AST

### Core Structures

```rust
// In src/ast/schema/mod.rs

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchemaOperation {
    CreateSchema(CreateSchemaStatement),
    AlterSchema(AlterSchemaStatement),
    DropSchema(DropSchemaStatement),
    DescribeSchema(DescribeSchemaStatement),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSchemaStatement {
    pub collection_name: String,
    pub fields: Vec<FieldSpecification>,
    pub options: SchemaOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSpecification {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub scope: PropertyScope,
    pub kind: FieldKind,
    pub cascade_config: Option<CascadeConfig>,
    pub default_value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Timestamp,
    Vector(usize), // Vector(dimension)
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyScope {
    Metadata,
    CollectionSpecific,
    DomainShared,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldKind {
    Regular,
    Cascaded,
    Calculated,
    Computed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CascadeConfig {
    pub aggregation: AggregationFunction,
    pub source_collection: String,
    pub source_field: String,
    pub edge_type: String,
    pub direction: EdgeDirection,
    pub decay: Option<DecayFunction>,
    pub temporal_window: Option<TemporalWindow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationFunction {
    Sum,
    Average,
    WeightedAverage,
    Max,
    Min,
    Count,
    Latest,
    First,
    MovingMean(usize),
    MovingMedian(usize),
    TimeWindowedAverage,
    Percentile(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecayFunction {
    None,
    Exponential(f64),
    PowerLaw(f64),
    Linear(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeDirection {
    Incoming,
    Outgoing,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalWindow {
    pub duration: std::time::Duration,
    pub slide: Option<std::time::Duration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaOptions {
    pub node_class_default: Option<String>, // "Active" or "Passive"
    pub collection_type: Option<String>,    // "content" or "taxonomy"
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterSchemaStatement {
    AddField {
        collection_name: String,
        field: FieldSpecification,
    },
    DropField {
        collection_name: String,
        field_name: String,
    },
    ModifyField {
        collection_name: String,
        field_name: String,
        new_spec: FieldSpecification,
    },
    AlterFieldScope {
        collection_name: String,
        field_name: String,
        new_scope: PropertyScope,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropSchemaStatement {
    pub collection_name: String,
    pub if_exists: bool,
    pub cascade: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeSchemaStatement {
    pub collection_name: String,
    pub detailed: bool,
}
```

### Integration with Statement Enum

```rust
// In src/ast/mod.rs
pub enum Statement {
    // Existing DML
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    
    // New DDL (import from schema module)
    Schema(schema::SchemaOperation),
}
```

---

## 9. Parser Implementation Strategy

### Module Structure

```
src/parser/
├── mod.rs              # Re-export parse_statement
├── statement.rs        # Top-level dispatcher (modify)
├── schema.rs           # New: Schema DDL parser
├── select.rs           # Existing
├── expression.rs       # Existing
└── ...
```

### Parser Entry Point Modification

```rust
// In src/parser/statement.rs
pub fn parse_statement(input: &str) -> Result<Statement> {
    let input = input.trim();
    let upper_input = input.to_uppercase();

    if upper_input.starts_with("SELECT") {
        select::parse_select_statement(input)
    } else if upper_input.starts_with("INSERT") {
        parse_insert_statement(input)
    } else if upper_input.starts_with("UPDATE") {
        parse_update_statement(input)
    } else if upper_input.starts_with("DELETE") {
        parse_delete_statement(input)
    } else if upper_input.starts_with("CREATE SCHEMA") {
        schema::parse_create_schema(input)
    } else if upper_input.starts_with("ALTER SCHEMA") {
        schema::parse_alter_schema(input)
    } else if upper_input.starts_with("DROP SCHEMA") {
        schema::parse_drop_schema(input)
    } else if upper_input.starts_with("DESCRIBE SCHEMA") {
        schema::parse_describe_schema(input)
    } else {
        Err(HyperQLError::simple_parse_error(
            "Unsupported statement. Supported: SELECT, INSERT, UPDATE, DELETE, CREATE SCHEMA, ALTER SCHEMA, DROP SCHEMA, DESCRIBE SCHEMA",
            input,
            1,
            1,
        ))
    }
}
```

### Schema Parser Functions (src/parser/schema.rs)

```rust
use crate::ast::schema::*;
use crate::ast::Statement;
use crate::error::*;

pub fn parse_create_schema(input: &str) -> Result<Statement> {
    // Parse: CREATE SCHEMA <name> ( <fields> ) [OPTIONS (...)]
    // Extract collection name
    // Parse field specifications
    // Parse cascade configurations
    // Parse options
    // Return Statement::Schema(SchemaOperation::CreateSchema(...))
}

pub fn parse_alter_schema(input: &str) -> Result<Statement> {
    // Parse: ALTER SCHEMA <name> ADD/DROP/MODIFY/ALTER FIELD ...
    // Determine alter operation type
    // Parse field specifications as needed
    // Return Statement::Schema(SchemaOperation::AlterSchema(...))
}

pub fn parse_drop_schema(input: &str) -> Result<Statement> {
    // Parse: DROP SCHEMA <name> [IF EXISTS] [CASCADE]
    // Extract collection name and flags
    // Return Statement::Schema(SchemaOperation::DropSchema(...))
}

pub fn parse_describe_schema(input: &str) -> Result<Statement> {
    // Parse: DESCRIBE SCHEMA <name> [DETAILED]
    // Extract collection name and detailed flag
    // Return Statement::Schema(SchemaOperation::DescribeSchema(...))
}

fn parse_field_specification(field_str: &str) -> Result<FieldSpecification> {
    // Parse: <name> <type> [REQUIRED] [METADATA|COLLECTION_SPECIFIC|DOMAIN_SHARED] [CASCADE(...)]
    // Handle all field attributes
}

fn parse_cascade_config(cascade_str: &str) -> Result<CascadeConfig> {
    // Parse: CASCADE (AGGREGATE <func>, FROM <collection>.<field>, THROUGH <edge> <direction>, [DECAY ...])
    // Complex nested parsing
}

fn parse_aggregation_function(func_str: &str) -> Result<AggregationFunction> {
    // Parse: Sum|Average|WeightedAverage|Max|Min|Count|Latest|First|MovingMean(N)|...
}

fn parse_decay_function(decay_str: &str) -> Result<DecayFunction> {
    // Parse: None|Exponential(rate)|PowerLaw(exponent)|Linear(rate)
}
```

---

## 10. Validation Integration

### Schema Validation Requirements

1. **Name Conflicts:** Check for duplicate collection names
2. **Field Conflicts:** Check for duplicate field names within schema
3. **Type Validation:** Validate field types are supported
4. **Cascade Validation:**
   - Source collection exists
   - Source field exists
   - Edge type exists
   - Aggregation function compatible with field type
5. **Scope Consistency:** Validate PropertyScope usage
6. **Circular Dependencies:** Detect cascade cycles

### Validator Extension

```rust
// In src/validator/schema.rs (extend existing)
impl SchemaValidator {
    pub fn validate_schema_ddl(&self, statement: &SchemaOperation, result: &mut ValidationResult) -> Result<()> {
        match statement {
            SchemaOperation::CreateSchema(create) => {
                self.validate_create_schema(create, result)?;
            }
            SchemaOperation::AlterSchema(alter) => {
                self.validate_alter_schema(alter, result)?;
            }
            SchemaOperation::DropSchema(drop) => {
                self.validate_drop_schema(drop, result)?;
            }
            SchemaOperation::DescribeSchema(_) => {
                // No validation needed - read-only
            }
        }
        Ok(())
    }
}
```

---

## 11. Compiler/Executor Integration

### Schema DDL Execution Plan

Schema DDL operations don't produce query plans - they are metadata operations:

```rust
// In src/compiler/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPlan {
    // Existing query plans...
    Scan { ... },
    Filter { ... },
    // ...
    
    // New: Schema DDL operations
    SchemaDDL(SchemaDDLOperation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaDDLOperation {
    CreateSchema(CreateSchemaStatement),
    AlterSchema(AlterSchemaStatement),
    DropSchema(DropSchemaStatement),
    DescribeSchema(DescribeSchemaStatement),
}
```

### Execution Strategy

Schema DDL is executed by **SchemaEngine** (already exists in hyperspatial):

```rust
// In executor or schema engine interface
impl SchemaEngine {
    pub fn execute_ddl(&mut self, operation: SchemaDDLOperation) -> Result<SchemaResult> {
        match operation {
            SchemaDDLOperation::CreateSchema(stmt) => {
                // Validate schema doesn't exist
                // Convert to FieldSpec vector
                // Register with SchemaEngine
                // Persist to disk
            }
            SchemaDDLOperation::AlterSchema(stmt) => {
                // Load existing schema
                // Apply modification
                // Validate compatibility
                // Persist updated schema
            }
            SchemaDDLOperation::DropSchema(stmt) => {
                // Check for dependencies (if not CASCADE)
                // Remove schema definition
                // Optionally drop data
            }
            SchemaDDLOperation::DescribeSchema(stmt) => {
                // Load schema
                // Format for display
                // Return schema metadata
            }
        }
    }
}
```

---

## 12. Testing Strategy

### Parser Tests

```rust
#[test]
fn test_parse_create_schema_basic() {
    let sql = "CREATE SCHEMA topics (id String REQUIRED METADATA, name String REQUIRED)";
    let stmt = parse_statement(sql).unwrap();
    // Assert structure
}

#[test]
fn test_parse_create_schema_with_cascade() {
    let sql = r#"
        CREATE SCHEMA topics (
            avg_score Float CASCADE (
                AGGREGATE Average,
                FROM articles.score,
                THROUGH tagged_with INCOMING
            )
        )
    "#;
    let stmt = parse_statement(sql).unwrap();
    // Assert cascade config
}

#[test]
fn test_parse_alter_schema_add_field() {
    let sql = "ALTER SCHEMA topics ADD FIELD article_count Int DOMAIN_SHARED";
    let stmt = parse_statement(sql).unwrap();
    // Assert structure
}

#[test]
fn test_parse_drop_schema_cascade() {
    let sql = "DROP SCHEMA topics IF EXISTS CASCADE";
    let stmt = parse_statement(sql).unwrap();
    // Assert flags
}
```

### Validation Tests

```rust
#[test]
fn test_validate_duplicate_schema_name() {
    // Test error when creating schema that exists
}

#[test]
fn test_validate_invalid_cascade_source() {
    // Test error when CASCADE references non-existent collection
}

#[test]
fn test_validate_circular_cascade_dependency() {
    // Test error when CASCADE creates cycle
}
```

### Integration Tests

```rust
#[test]
fn test_create_and_describe_schema() {
    // CREATE SCHEMA -> DESCRIBE SCHEMA
    // Assert schema details match
}

#[test]
fn test_alter_schema_workflow() {
    // CREATE -> ALTER (add field) -> DESCRIBE
    // Assert field added correctly
}
```

---

## 13. File Checklist

### Files to Create

1. **src/ast/schema/mod.rs** - Schema DDL AST module entry
2. **src/ast/schema/operations.rs** - Schema operation enums
3. **src/ast/schema/field_spec.rs** - Field specification structs
4. **src/ast/schema/cascade.rs** - Cascade configuration structs
5. **src/parser/schema.rs** - Schema DDL parser implementation

### Files to Modify

1. **src/ast/mod.rs** - Add Schema variant to Statement enum
2. **src/parser/mod.rs** - Re-export schema parser
3. **src/parser/statement.rs** - Add schema DDL dispatch
4. **src/compiler/mod.rs** - Add SchemaDDL to ExecutionPlan
5. **src/validator/schema.rs** - Extend for DDL validation

### Files to Reference

1. **src/ast/streams/operations.rs** - Pattern reference
2. **src/persistence/schema/types.rs** - PropertyScope, FieldKind, CascadeConfig (already exist!)
3. **src/persistence/schema/schema.rs** - SchemaEngine integration

---

## 14. Key Decisions

### Design Decisions

1. **Separate SchemaOperation enum vs Statement variants?**
   - **Decision:** Use Statement::Schema(SchemaOperation) pattern
   - **Rationale:** Mirrors StreamOperation pattern, cleaner separation

2. **Parser complexity: Hand-written vs parser combinator?**
   - **Decision:** Hand-written string parsing (matching existing code)
   - **Rationale:** Consistency with existing parsers, simpler dependencies

3. **Cascade config in AST vs just string?**
   - **Decision:** Full structured CascadeConfig in AST
   - **Rationale:** Type safety, validation at parse time, better errors

4. **PropertyScope as string vs enum?**
   - **Decision:** Use existing PropertyScope enum from persistence layer
   - **Rationale:** Reuse existing type, ensures consistency

5. **Execute DDL in compiler or separate engine?**
   - **Decision:** Compiler produces SchemaDDL plan, SchemaEngine executes
   - **Rationale:** Separation of concerns, SchemaEngine owns metadata

### Critical Integration Points

1. **AST → Persistence Schema:**
   - Convert CreateSchemaStatement to Vec<FieldSpec>
   - Mapping function in compiler or executor

2. **Parser → AST:**
   - CASCADE config string parsing is most complex
   - Need robust error messages for nested syntax

3. **Validation → SchemaEngine:**
   - Validator needs access to SchemaEngine for existence checks
   - Cycle detection needs graph traversal

---

## 15. Example End-to-End Flow

### User Input
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

### Parse Flow
1. **Parser** (`parse_statement`) → detects "CREATE SCHEMA"
2. **Schema Parser** (`parse_create_schema`) → extracts:
   - Collection name: "topics"
   - Fields: [id, name, avg_score]
   - avg_score cascade config
   - Options
3. **AST Construction** → `Statement::Schema(SchemaOperation::CreateSchema(...))`

### Validation Flow
4. **Validator** → checks:
   - "topics" doesn't exist
   - "articles" collection exists
   - "score" field exists in articles
   - "tagged_with" edge type exists
   - No circular cascades

### Compilation Flow
5. **Compiler** → produces:
   - ExecutionPlan::SchemaDDL(CreateSchema(...))
   - Metadata: { operation: "DDL", type: "CREATE_SCHEMA" }
   - Cost: { estimated_time: 50ms, io_ops: 1 }

### Execution Flow
6. **SchemaEngine** → executes:
   - Convert to FieldSpec vector
   - Register schema in schema registry
   - Create CascadeTriggerIndex entry for avg_score
   - Persist to schema database
7. **Return** → SchemaResult { created: true, collection: "topics" }

---

## 16. Critical Success Factors

### Must Have

1. **Robust CASCADE parsing** - Most complex syntax, needs careful testing
2. **Clear error messages** - Users need to understand DDL syntax errors
3. **Integration with existing SchemaEngine** - Reuse existing infrastructure
4. **Backward compatibility** - Don't break existing schema validation
5. **Comprehensive tests** - DDL is foundational, needs thorough testing

### Nice to Have

1. **Schema migration support** - ALTER SCHEMA compatibility checking
2. **Schema versioning** - Track schema changes over time
3. **Schema import/export** - Bulk schema definitions
4. **Auto-completion hints** - Parser could suggest field types
5. **Schema visualization** - Tool to display schema dependencies

### Risks

1. **CASCADE syntax complexity** - Nested parentheses, multiple keywords
2. **Edge case handling** - Optional fields, defaults, null handling
3. **Performance impact** - Schema validation on every query
4. **Breaking changes** - Schema modifications affecting existing data
5. **Circular dependencies** - Complex cascade graphs

---

## 17. Next Steps

### Phase 1: Foundation (4-6 hours)
- [ ] Create AST structures in `src/ast/schema/`
- [ ] Add Statement::Schema variant
- [ ] Basic parser structure in `src/parser/schema.rs`
- [ ] Parser dispatch in statement.rs

### Phase 2: Core Parsing (6-8 hours)
- [ ] CREATE SCHEMA parser (without CASCADE)
- [ ] Field specification parsing
- [ ] PropertyScope and FieldKind parsing
- [ ] ALTER/DROP/DESCRIBE parsers

### Phase 3: Advanced Features (8-12 hours)
- [ ] CASCADE configuration parsing
- [ ] Aggregation function parsing
- [ ] Decay function parsing
- [ ] Temporal window parsing
- [ ] Validation integration

### Phase 4: Execution (4-6 hours)
- [ ] Compiler integration
- [ ] SchemaEngine execution
- [ ] Error handling and rollback
- [ ] Schema persistence

### Phase 5: Testing (6-8 hours)
- [ ] Parser unit tests
- [ ] Validation tests
- [ ] Integration tests
- [ ] Error case coverage
- [ ] Documentation and examples

**Total Estimated Time: 28-40 hours (3.5-5 days)**

---

## 18. References

### Existing Code Patterns
- **Stream DDL:** `/src/ast/streams/operations.rs`
- **Statement Parsing:** `/src/parser/statement.rs`
- **Expression Parsing:** `/src/parser/expression.rs`
- **Schema Validation:** `/src/validator/schema.rs`
- **Schema Types:** `/src/persistence/schema/types.rs`

### Related Documentation
- **Stage 4 Plan:** `STAGE4_STAGE5_PLAN.md`
- **Cascade Design:** `CASCADE_IMPLEMENTATION.md`
- **Schema Architecture:** `src/persistence/schema/mod.rs`

### External References
- SQL DDL standards (CREATE TABLE syntax)
- PostgreSQL schema DDL
- Apache Calcite DDL parser patterns
- StreamSQL CREATE STREAM examples

---

## Appendix: Complete Example Schema

```sql
-- Content collection schema
CREATE SCHEMA articles (
    -- Metadata fields (filtered from similarity)
    id String REQUIRED METADATA,
    session_id String METADATA,
    created_at Timestamp METADATA,
    updated_at Timestamp METADATA,
    
    -- Domain-shared fields (global comparison)
    title String REQUIRED DOMAIN_SHARED,
    score Float DOMAIN_SHARED,
    sentiment Float DOMAIN_SHARED,
    view_count Int DOMAIN_SHARED,
    
    -- Collection-specific fields (local comparison only)
    content String COLLECTION_SPECIFIC,
    raw_html String COLLECTION_SPECIFIC,
    internal_status String COLLECTION_SPECIFIC
) OPTIONS (
    node_class_default = 'Active',
    collection_type = 'content'
);

-- Taxonomy collection schema with cascades
CREATE SCHEMA topics (
    -- Metadata
    id String REQUIRED METADATA,
    created_at Timestamp METADATA,
    
    -- Regular fields
    name String REQUIRED DOMAIN_SHARED,
    description String COLLECTION_SPECIFIC,
    
    -- Cascaded aggregations
    avg_score Float DOMAIN_SHARED CASCADE (
        AGGREGATE Average,
        FROM articles.score,
        THROUGH tagged_with INCOMING,
        DECAY Exponential(0.1)
    ),
    article_count Int DOMAIN_SHARED CASCADE (
        AGGREGATE Count,
        FROM articles,
        THROUGH tagged_with INCOMING
    ),
    latest_article_date Timestamp DOMAIN_SHARED CASCADE (
        AGGREGATE Latest,
        FROM articles.created_at,
        THROUGH tagged_with INCOMING
    ),
    moving_avg_sentiment Float DOMAIN_SHARED CASCADE (
        AGGREGATE MovingMean(10),
        FROM articles.sentiment,
        THROUGH tagged_with INCOMING,
        DECAY PowerLaw(2.0)
    )
) OPTIONS (
    node_class_default = 'Passive',
    collection_type = 'taxonomy'
);
```

---

**Report Complete**  
**Next Action:** Begin Phase 1 implementation of AST structures
