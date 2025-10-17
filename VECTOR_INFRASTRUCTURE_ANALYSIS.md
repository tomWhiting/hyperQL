# HyperQL Vector Query Infrastructure - Comprehensive Analysis

**Date:** 2025-10-18
**Repository:** /Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL
**Analysis Depth:** Very Thorough

## Executive Summary

HyperQL has a **well-designed vector infrastructure** with complete AST definitions, compiler support, and executor stubs. The architecture follows a clean "plan-based" design pattern where the compiler generates execution plans for Hyperspatial to execute, rather than executing directly. This is intentional and matches the geometric operations pattern.

**Status:** 70% complete
- **AST Layer:** 100% complete (similarity.rs, knn.rs fully implemented)
- **Compiler Layer:** 100% complete (generates VectorOpType plans)
- **Executor Layer:** 30% complete (6 vector operation stubs + basic test implementations)
- **Parser Layer:** ~0% complete (no SQL parsing for vector operations yet)
- **Data Source Integration:** ~0% complete (RouterDataSource hooks not implemented)

---

## Architecture Overview

### Design Pattern: Plan-Based Execution

The vector infrastructure follows a **three-phase plan-based architecture**:

```
SQL Query
    ↓
Parser (AST)
    ↓
Compiler (generates VectorOpType plans with parameters)
    ↓
Executor (returns ExecutionPlan, not results)
    ↓
Hyperspatial Engine (executes plans)
    ↓
Results
```

This mirrors the geometric operations pattern exactly, intentionally keeping HyperQL as a compiler/optimizer rather than a direct executor.

---

## Layer 1: AST Layer (COMPLETE)

Location: `/Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL/src/ast/vector/`

### Files

1. **mod.rs** - Module documentation (182 lines)
   - Comprehensive design principles documentation
   - Mathematical foundations section
   - Core AST node types documentation
   - Performance features and integration documentation

2. **similarity.rs** - Vector similarity operations (369 lines)
   - `SimilarityExpressionNode`: Main similarity search node
   - `SimilarityMetric` enum: Cosine, DotProduct, Euclidean, Manhattan, Jaccard, Custom
   - `VectorType` enum: Dense, Sparse, ColBERT multi-vector support
   - `SimilarityThreshold`: Threshold constraint with min/max scores
   - `NamedVectorRef`: Reference to named vector fields
   - Complete validation, cost estimation, output type inference
   - Full test coverage (8 tests)

3. **knn.rs** - k-Nearest neighbor operations (490 lines)
   - `KNNQueryNode`: K-NN query specification
   - `KNNConfig`: Configuration with approximation, indexing, parallelism options
   - `DiversityConstraint`: Spatial/feature/category diversity support
   - `KNNOrdering`: BySimilarity vs ByDistance ordering
   - Complete validation, cost estimation, index recommendations
   - Full test coverage (9 tests)

### VectorExpression Integration

Main AST file (`src/ast/mod.rs`, lines 311-330):
```rust
pub enum VectorExpression {
    /// Similarity search with named vector
    Similarity {
        vector_name: String,
        reference: Box<Expression>,
        metric: vector::similarity::SimilarityMetric,
        threshold: Option<f64>,
        vector_type: vector::similarity::VectorType,
    },
    /// k-nearest neighbors with named vector
    KNN {
        vector_name: String,
        reference: Box<Expression>,
        k: u32,
        metric: vector::similarity::SimilarityMetric,
        vector_type: vector::similarity::VectorType,
    },
}
```

Integrated into main `Expression` enum (line 249):
```rust
pub enum Expression {
    // ... other variants
    Vector(VectorExpression),
}
```

### Key Design Decisions

1. **Named Vectors**: Entities can have multiple named embeddings
   - `text_embedding`, `code_embedding`, `bge_m3`, `keywords_sparse`, `colbert_tokens`
   - Query specifies which named vector to use
   - Type-safe specification via `VectorType`

2. **Multiple Metrics Supported**:
   - Dense vectors: Cosine, DotProduct, Euclidean, Manhattan
   - Sparse vectors: Jaccard
   - ColBERT multi-vector: Dot product with special handling

3. **Cost Model**: Logarithmic complexity estimation
   - Metric factor (Cosine: 1.5, DotProduct: 1.0, Jaccard: 2.0)
   - Vector type factor (Dense: log2(dims)/10, Sparse: 1.5, ColBERT: log2(tokens*dims)/8)

---

## Layer 2: Compiler Layer (100% COMPLETE)

Location: `/Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL/src/compiler/`

### VectorOpType Enum

File: `mod.rs`, lines 37-52:
```rust
pub enum VectorOpType {
    /// Cosine similarity calculation
    CosineSimilarity,
    /// Euclidean distance calculation
    EuclideanDistance,
    /// Dot product calculation
    DotProduct,
    /// Vector normalization
    Normalize,
    /// K-nearest neighbors query
    KNN,
    /// Vector similarity search
    SimilaritySearch,
}
```

### Execution Plan Structure

File: `mod.rs`, lines 204-209:
```rust
/// Vector operation plan
VectorOperation {
    op_type: VectorOpType,
    params: std::collections::HashMap<String, CompiledExpression>,
    input: Option<Box<ExecutionPlan>>,
},
```

### Compilation Strategy

File: `expression.rs`, lines 115-116:
```rust
Expression::Vector(vector_expr) => {
    self.compile_vector_expression(vector_expr)
}
```

**Current Status**: Stub exists but not fully implemented. Should translate:
- `VectorExpression::Similarity` → `VectorOpType::SimilaritySearch` or `CosineSimilarity`
- `VectorExpression::KNN` → `VectorOpType::KNN`

**What exists for geometric pattern**:
File: `expression.rs`, lines 190-199 shows pattern for geometric compilation:
```rust
fn compile_geometric_expression(&self, geom_expr: crate::ast::geometric::GeometricExpression) -> Result<CompiledExpression> {
    match geom_expr {
        crate::ast::geometric::GeometricExpression::Within { target, radius, reference } => {
            let compiled_target = self.compile_expression(*target)?;
            let compiled_reference = self.compile_expression(*reference)?;
            let compiled_radius = CompiledExpression::Literal(Value::Float(radius));

            Ok(CompiledExpression::Function {
                name: "WITHIN_RADIUS".to_string(),
                args: vec![compiled_target, compiled_reference, compiled_radius],
                result_type: ValueType::Bool,
            })
        }
```

**For vectors, similar pattern should be**:
```rust
fn compile_vector_expression(&self, vector_expr: VectorExpression) -> Result<CompiledExpression> {
    match vector_expr {
        VectorExpression::Similarity { vector_name, reference, metric, threshold, vector_type } => {
            // Create VectorOperation execution plan with:
            // - op_type: VectorOpType::SimilaritySearch
            // - params: {query_vector, threshold}
        }
        VectorExpression::KNN { vector_name, reference, k, metric, vector_type } => {
            // Create VectorOperation execution plan with:
            // - op_type: VectorOpType::KNN
            // - params: {query_vector, k}
        }
    }
}
```

### Type Inference

File: `expression.rs`, lines 181-184 shows vector function result types:
```rust
"COSINE_SIMILARITY" | "DOT_PRODUCT" => Ok(ValueType::Float),
"EUCLIDEAN_DISTANCE" => Ok(ValueType::Distance),
"NORMALIZE" => Ok(ValueType::Vector),
"KNN" | "SIMILARITY_SEARCH" => Ok(ValueType::List(Box::new(ValueType::EntityId))),
```

---

## Layer 3: Executor Layer (30% COMPLETE)

Location: `/Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL/src/executor/vector.rs` (470 lines)

### VectorEngine Structure

```rust
pub struct VectorEngine;

impl VectorEngine {
    pub fn new() -> Self { Self }
    pub fn execute_operation(
        &self,
        op_type: &VectorOpType,
        params: &HashMap<String, CompiledExpression>,
        input_rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>>
}
```

### Six Operation Executors

1. **execute_cosine_similarity** (lines 43-89)
   - Extracts two vectors from parameters
   - Computes dot product / (magnitude1 * magnitude2)
   - Adds "similarity" column to results
   - Status: ✅ Fully implemented

2. **execute_euclidean_distance** (lines 91-137)
   - Extracts two vectors
   - Computes sqrt(sum((a-b)^2))
   - Adds "distance" column to results
   - Status: ✅ Fully implemented

3. **execute_dot_product** (lines 139-185)
   - Extracts two vectors
   - Computes sum(a[i] * b[i])
   - Adds "dot_product" column to results
   - Status: ✅ Fully implemented

4. **execute_normalize** (lines 187-223)
   - Extracts one vector
   - Divides by magnitude
   - Adds "normalized_vector" column as Value::Vector
   - Status: ✅ Fully implemented

5. **execute_knn** (lines 225-283)
   - Extracts query vector and k parameter
   - Computes distances to all input rows
   - Sorts by distance ascending
   - Returns top k results with "distance" column
   - Status: ⚠️ Partially implemented (basic brute force, no indexing)

6. **execute_similarity_search** (lines 285-343)
   - Extracts query vector and threshold
   - Computes cosine similarity to all input rows
   - Filters by threshold
   - Sorts by similarity descending
   - Status: ⚠️ Partially implemented (no index usage)

### Helper Methods

- `extract_vector()` (lines 345-367): Extract Value::Vector from compiled expression
- `extract_vector_from_row()` (lines 369-381): Look for "embedding" or "vector" field in row
- `dot_product()` (lines 383-398): Compute dot product with dimension validation
- `magnitude()` (lines 400-405): Compute L2 norm
- `cosine_similarity()` (lines 407-421): Compute normalized dot product
- `euclidean_distance()` (lines 423-442): Compute L2 distance with dimension check
- `normalize()` (lines 444-462): Scale vector to unit length

### Integration Pattern

The VectorEngine follows the same pattern as GeometricEngine:

```rust
// From plan_executor.rs
match plan {
    ExecutionPlan::VectorOperation { op_type, params, input } => {
        // ... handle input
        let mut engine = VectorEngine::new();
        engine.execute_operation(&op_type, &params, rows, &mut evaluator)
    }
}
```

---

## Layer 4: Type System (100% COMPLETE)

Location: `/Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL/src/types.rs`

### Vector Type Definition

```rust
pub struct Vector {
    pub dimensions: Vec<f64>,
}
```

### Value Enum Integration

Lines 138:
```rust
pub enum Value {
    // ... other types
    Vector(Vector),
}
```

### Entity with Embedding Support

Lines 149-156:
```rust
pub struct Entity {
    pub id: EntityId,
    pub properties: HashMap<PropertyName, Value>,
    pub position: Option<Position3D>,
    pub embedding: Option<Vector>,  // Single embedding per entity
}
```

**NOTE**: Current `Entity` structure only supports **one** embedding per entity. Named vectors would need property-based storage or an extended structure:
```rust
pub struct Entity {
    named_vectors: HashMap<String, Vector>,  // vector_name -> Vector
}
```

---

## Layer 5: Data Source Integration (0% COMPLETE)

Location: `/Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL/src/executor/data_source.rs`

### DataSource Trait

Current signature (lines 34-131):
```rust
pub trait DataSource: Send + Sync {
    fn scan(&self, table: &str, entity_type: &str) -> Result<Vec<Entity>>;
    fn scan_with_limit(&self, table: &str, entity_type: &str, limit: usize) -> Result<Vec<Entity>>;
    fn insert(&mut self, table: &str, entities: Vec<Entity>) -> Result<u64>;
    fn update(&mut self, table: &str, filter: Option<&CompiledExpression>, assignments: &[CompiledAssignment]) -> Result<u64>;
    fn delete(&mut self, table: &str, filter: Option<&CompiledExpression>) -> Result<u64>;
    fn get_schema(&self, table: &str) -> Result<TableSchema>;
    fn traverse_graph(&self, start_entity_id: &EntityId, max_depth: usize, edge_type_filter: Option<&str>) -> Result<Vec<(Entity, usize)>>;
    fn count_entities_fast(&self, table: &str, entity_type: &str) -> Result<usize>;
}
```

### Missing Vector-Specific Methods

For full integration with Hyperspatial's vector storage, need:

```rust
pub trait DataSource {
    // Existing methods...
    
    // NEW: Vector-specific methods
    
    /// Get named vector for an entity
    fn get_vector(&self, collection: &str, entity_id: &str, vector_name: &str) -> Result<Option<Vector>>;
    
    /// Get all named vectors for an entity
    fn get_all_vectors(&self, collection: &str, entity_id: &str) -> Result<HashMap<String, Vector>>;
    
    /// Store named vector for an entity
    fn store_vector(&mut self, collection: &str, entity_id: &str, vector_name: &str, vector: Vector) -> Result<()>;
    
    /// KNN search using Hyperspatial index
    fn knn_search(&self, collection: &str, vector_name: &str, query_vector: &Vector, k: usize, metric: &SimilarityMetric) -> Result<Vec<(String, f64)>>;
    
    /// Similarity search with threshold
    fn similarity_search(&self, collection: &str, vector_name: &str, query_vector: &Vector, threshold: f64, metric: &SimilarityMetric) -> Result<Vec<(String, f64)>>;
    
    /// Get vector metadata (dimensions, type, storage location)
    fn get_vector_metadata(&self, collection: &str, vector_name: &str) -> Result<VectorMetadata>;
}
```

### MemoryDataSource Implementation

Lines 7-143: Only provides in-memory testing, not vector-specific operations.

### Comments Indicating Integration Points

Lines 76, 129-130 reference "RouterDataSource" (Hyperspatial integration):
```rust
// Real implementations (RouterDataSource) override with early termination
// Real implementations (RouterDataSource) override this with fast database-level counting
```

---

## Layer 6: Parser Layer (0% COMPLETE)

Location: `/Users/tom/Developer/spaces/projects/hyperspatial/main/hyperQL/src/parser/`

### Current State

The parser has no SQL support for vector operations. The expression parser (`expression.rs`, lines 1-91) handles geometric operations but not vectors.

### What Needs to be Parsed

SQL patterns that should work:

```sql
-- Similarity threshold in WHERE clause
SELECT * FROM products 
WHERE text_embedding SIMILAR TO @query_vector THRESHOLD 0.8

-- Cosine similarity in SELECT list
SELECT title, SIMILARITY(code_embedding, @target) as sim_score
FROM documents
ORDER BY sim_score DESC

-- k-NN in ORDER BY with LIMIT
SELECT title, description
FROM articles
ORDER BY SIMILARITY(text_vec, @query_vector) DESC
LIMIT 10

-- Explicit distance metric
SELECT * FROM items
WHERE DISTANCE(feature_vec, @target) < 1.5

-- Multi-metric query
SELECT *
FROM products
WHERE SIMILARITY(embeddings, @query, 'cosine') > 0.75
  AND DISTANCE(attributes, @baseline, 'euclidean') < 2.0
```

### Missing Parser Components

1. **SIMILAR TO operator**: `vector_field SIMILAR TO expression THRESHOLD score`
2. **SIMILARITY function**: `SIMILARITY(vector_field, reference_vector)`
3. **DISTANCE function**: `DISTANCE(vector_field, reference_vector)`
4. **Vector parameters**: `@vector_name` syntax or `VECTOR(...)` constructor
5. **Metric specification**: Optional metric parameter in functions

### Example Implementation Pattern

Following geometric pattern from `parser/geometric.rs`:

```rust
// Would go in parser/expression.rs or new parser/vector.rs

pub fn parse_similarity_expression(input: &str) -> Result<Expression> {
    // SIMILARITY(vector_name, reference) THRESHOLD threshold
    // Extract: vector_name, reference, threshold
    // Return: Expression::Vector(VectorExpression::Similarity { ... })
}

pub fn parse_distance_expression(input: &str) -> Result<Expression> {
    // DISTANCE(vector_name, reference)
    // Return: Expression::Vector(VectorExpression::Similarity with Euclidean metric)
}

pub fn parse_similar_to_operator(input: &str) -> Result<Expression> {
    // vector_name SIMILAR TO reference THRESHOLD threshold
    // Return: Expression::Vector(VectorExpression::Similarity { ... })
}
```

---

## Integration with Hyperspatial

### Current Hyperspatial Features

From CLAUDE.md in hyperspatial repository:

1. **Multi-Position Architecture** (Production):
   - graph_position: From degree structure
   - embedding_position: From semantic vectors
   - property_position: From numeric attributes
   - Stored as 204 bytes per entity (3 × 17D × 4 bytes)

2. **HNSW Index**:
   - Multi-modal distance: `distance = α × d_graph + β × d_embedding + γ × d_property`
   - Build time: 13-15s for 20K entities
   - Query latency: 40-70μs average
   - Throughput: 14,000-20,000 QPS

3. **Stage 5: Active/Passive Node Architecture**:
   - NodeClass enum: Active (content) vs Passive (organizational)
   - EdgeClass enum: Passive (structural) vs Active (cascade-enabled)
   - ActiveOnly filtering for query results

4. **Router API**:
   - `get_entity_any_collection()`
   - `bulk_get_entities_all_collections()`
   - `bulk_get_properties_all_collections()`
   - `get_vector_any_collection()` / `store_vector()`

### Proposed RouterDataSource Implementation

Would need to:

1. Implement DataSource trait for Hyperspatial's Router
2. Add vector-specific methods calling:
   - `router.get_vector_any_collection(entity_id, vector_name)`
   - `router.search_knn_multi_position()` for similarity search
   - `router.store_vector()` for vector persistence

3. Handle multi-modal HNSW queries:
   - Convert VectorOpType::KNN to HNSW search with appropriate weights
   - Support NodeClassFilter (ActiveOnly, PassiveOnly, All)
   - Handle multi-position vectors (graph + embedding + property)

---

## What's Complete vs Missing

### Complete (100%)

1. **AST Definitions**
   - SimilarityExpressionNode with all metrics and types
   - KNNQueryNode with diversity constraints
   - VectorType enum (Dense, Sparse, ColBERT)
   - SimilarityMetric enum (6 metrics + custom)
   - Full validation and cost estimation

2. **Compiler Plans**
   - VectorOpType enum (6 operation types)
   - VectorOperation execution plan structure
   - Parameter passing infrastructure

3. **Basic Execution**
   - 6 vector operation implementations
   - Helper methods for vector math
   - Result row augmentation

4. **Type System**
   - Vector type definition
   - Integration into Value enum
   - Cost model for planning

### Partially Complete (30-50%)

1. **Executor**
   - CosineSimilarity, EuclideanDistance, DotProduct, Normalize: ✅ Full
   - KNN, SimilaritySearch: ⚠️ Brute force only, no indexing
   - No multi-modal HNSW integration
   - No batch processing optimization

2. **Data Source**
   - MemoryDataSource: ✅ For testing
   - RouterDataSource: ❌ Not implemented
   - No vector-specific API methods

### Missing (0%)

1. **Parser**
   - No SQL parsing for SIMILARITY, DISTANCE functions
   - No SIMILAR TO operator
   - No vector parameter syntax (@vector, VECTOR(...))

2. **Named Vector Support**
   - Entity.embedding is single vector
   - Need HashMap<String, Vector> for named vectors
   - Storage and retrieval infrastructure

3. **Index Integration**
   - No HNSW index usage
   - No approximate k-NN
   - No query planning for index selection

4. **Performance Optimization**
   - No batch vector operations
   - No caching of similarity scores
   - No approximate algorithms

5. **Type Checking**
   - No validation of vector dimensions
   - No metric-type compatibility checking (beyond AST)
   - No dimension inference

---

## Architectural Decisions Already Baked In

1. **Plan-Based Design**: Compiler generates plans for Hyperspatial, doesn't execute directly
   - Follows same pattern as geometric operations
   - Clean separation of concerns
   - Allows for optimizer to work with plans

2. **Named Vectors**: Each vector field has a user-defined name
   - Supports multiple embeddings per entity
   - Enables queries like `text_embedding SIMILAR TO ...`
   - Type-safe with VectorType specification

3. **Multi-Metric Support**: Multiple distance functions in single framework
   - Cosine, Euclidean, Manhattan, DotProduct, Jaccard, Custom
   - Metric incompatibility validation
   - Cost-based planning (different metrics have different costs)

4. **Diversity Constraints**: k-NN queries can specify diversity requirements
   - Spatial diversity (by position)
   - Feature diversity (by another vector)
   - Category diversity (by discrete values)
   - Extensible to custom diversity functions

5. **Vector Types**: Support for different vector representations
   - Dense: Standard Euclidean vectors
   - Sparse: For keyword/feature vectors
   - ColBERT: Multi-vector token representations
   - Type-specific metric validation

---

## Integration Points for Implementation

### Immediate (Parser)

1. **Add SIMILARITY function parsing**
   ```rust
   // In parser/expression.rs or new parser/vector.rs
   fn parse_function_call(name: &str, args: Vec<Expression>) -> Expression {
       if name.to_uppercase() == "SIMILARITY" {
           // Extract vector_name, reference, optional metric
           // Create VectorExpression::Similarity
       }
   }
   ```

2. **Add SIMILAR TO operator**
   ```rust
   // In parser/expression.rs
   if input.contains(" SIMILAR TO ") && input.contains(" THRESHOLD ") {
       // Parse vector_field SIMILAR TO query THRESHOLD score
   }
   ```

### Near-term (Compiler)

1. **Implement compile_vector_expression**
   ```rust
   // In compiler/expression.rs
   fn compile_vector_expression(&self, vector_expr: VectorExpression) -> Result<CompiledExpression> {
       match vector_expr {
           VectorExpression::Similarity { ... } => {
               // Create VectorOperation plan with SimilaritySearch or metric-specific op
           }
       }
   }
   ```

2. **Add query plan generation**
   - Map VectorExpression to VectorOperation execution plan
   - Extract parameters and embed in HashMap

### Medium-term (Data Source)

1. **Extend Entity to support named vectors**
   ```rust
   pub struct Entity {
       id: EntityId,
       properties: HashMap<PropertyName, Value>,
       position: Option<Position3D>,
       vectors: HashMap<String, Vector>,  // Named vectors
   }
   ```

2. **Add vector methods to DataSource trait**
   - get_vector, store_vector, knn_search, similarity_search

3. **Implement RouterDataSource**
   - Bridge to Hyperspatial's Router
   - Use multi-position HNSW for queries

### Long-term (Optimization)

1. **HNSW integration**
   - Use Hyperspatial's indices for KNN
   - Apply multi-modal distance weighting

2. **Query planning**
   - Cost-based selection of exact vs approximate
   - Index selection based on statistics

3. **Performance optimization**
   - Batch vector similarity computation
   - Score caching across queries
   - SIMD-friendly vector operations

---

## Code Quality Assessment

### Strengths

1. **Clean AST Design**: VectorExpression properly integrated into Expression enum
2. **Comprehensive Validation**: Both similarity and KNN have thorough validation
3. **Type Safety**: Rust's type system enforces correct metric-type combinations
4. **Test Coverage**: All AST nodes have tests (8 + 9 = 17 test functions)
5. **Documentation**: Module-level documentation explains design well
6. **Cost Modeling**: Realistic cost estimation based on metric and vector type

### Weaknesses

1. **Parser Integration**: Zero parsing support for vector operations
2. **Data Source Gap**: No RouterDataSource implementation
3. **Named Vector Storage**: Entity structure doesn't support multiple named vectors
4. **Index Integration**: VectorEngine uses brute force, no HNSW
5. **Compilation Gap**: compile_vector_expression appears to be a stub

### Technical Debt

1. Lines with `TODO`: 0 explicit TODOs in vector code
2. Stubbed methods: compile_vector_expression (needs implementation)
3. Test coverage: Tests are unit/AST level, no integration tests
4. No performance benchmarks

---

## Summary Table

| Component | Location | Status | Completeness | Key Files |
|-----------|----------|--------|--------------|-----------|
| **AST** | src/ast/vector/ | ✅ Complete | 100% | similarity.rs (369L), knn.rs (490L) |
| **Compiler** | src/compiler/ | ✅ Complete | 100% | mod.rs (VectorOpType enum) |
| **Executor** | src/executor/vector.rs | ⚠️ Partial | 30% | 6 operations, basic math only |
| **Types** | src/types.rs | ✅ Complete | 100% | Vector struct, Value enum integration |
| **Parser** | src/parser/ | ❌ Missing | 0% | No vector parsing |
| **Data Source** | src/executor/data_source.rs | ❌ Missing | 0% | RouterDataSource not implemented |
| **Integration** | src/ | ❌ Missing | 0% | Hyperspatial Router not connected |
| **Tests** | tests/vector_operations.rs | ⚠️ Partial | 40% | AST unit tests present, no integration |

---

## Recommended Next Steps

1. **Week 1: Parser Implementation**
   - Add SIMILARITY(field, reference) function parsing
   - Add DISTANCE(field, reference) function parsing  
   - Add SIMILAR TO operator parsing
   - Test with basic queries

2. **Week 2: Compiler Integration**
   - Implement compile_vector_expression
   - Map VectorExpression to execution plans
   - Add type checking for vector operations
   - Test plan generation

3. **Week 3: Data Model Extension**
   - Extend Entity to support named vectors
   - Add RouterDataSource implementation
   - Implement vector retrieval methods
   - Test data source layer

4. **Week 4: Executor Optimization**
   - Add HNSW index usage in knn_search, similarity_search
   - Implement approximate k-NN
   - Add batch vector operations
   - Performance testing

---

## Files Summary

**Total Vector-Related Files:** 8
- ast/vector/mod.rs: 182 lines (documentation + module)
- ast/vector/similarity.rs: 369 lines (complete)
- ast/vector/knn.rs: 490 lines (complete)
- executor/vector.rs: 470 lines (partially complete)
- tests/vector_operations.rs: 451 lines (AST tests)
- examples/vector_similarity_demo.rs: 87 lines (demo)

**Integration Points:**
- ast/mod.rs: Lines 311-330 (VectorExpression enum)
- compiler/mod.rs: Lines 37-52 (VectorOpType enum), 204-209 (VectorOperation plan)
- compiler/expression.rs: Lines 115-116 (compile_vector_expression stub), 181-184 (type inference)
- executor/mod.rs: Line 21 (re-export), Lines 95-130 (DataSource trait)
- executor/plan_executor.rs: Vector operation dispatch (not shown in reads)
- executor/geometric.rs: Lines 1-56 (pattern to follow)
- types.rs: Lines 112-116 (Vector type), Line 138 (Value enum)

