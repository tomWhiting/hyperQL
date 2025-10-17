# HyperQL Vector Query Infrastructure - Architecture Diagrams

## 1. Overall Query Pipeline

```
                          HYPERQL QUERY
                                |
                                v
                    ╔═══════════════════╗
                    ║      PARSER       ║
                    ║  (Missing: 0%)   ║
                    ╚═══════════════════╝
                                |
                    [Produce AST with VectorExpression]
                                |
                                v
                    ╔═══════════════════╗
                    ║    COMPILER       ║
                    ║  (Complete: 100%)║
                    ╚═══════════════════╝
                                |
                    [Generate VectorOpType Execution Plans]
                                |
                                v
                    ╔═══════════════════╗
                    ║     EXECUTOR      ║
                    ║  (Partial: 30%)  ║
                    ╚═══════════════════╝
                                |
                    [Returns ExecutionPlan to Hyperspatial]
                                |
                                v
                    ╔═══════════════════╗
                    ║  HYPERSPATIAL     ║
                    ║  (External)       ║
                    ║  - Multi-HNSW     ║
                    ║  - Multi-Position ║
                    ║  - Router API     ║
                    ╚═══════════════════╝
                                |
                                v
                            RESULTS
```

## 2. AST Layer - Expression Hierarchy

```
                          Expression
                                |
                 ┌──────────────┼──────────────┐
                 |              |              |
           Geometric        Vector          Other
                 |              |              |
              GeometricExpr  VectorExpr    Column, etc
                              |
                     ┌─────────┴─────────┐
                     |                   |
                Similarity           KNN
                     |                   |
        ┌────────────┼────────────┐    │
        |            |            |    │
    Cosine    DotProduct    Euclidean │
    Metric   Metric      Metric       │
        |            |            |    │
        └────────────┴────────────┘    │
                                       │
                    ┌──────────────────┴──────────────────┐
                    |                                     |
            [k parameter]          [Diversity Constraints]
                                   - Spatial
                                   - Feature
                                   - Category
```

## 3. Compiler - Transformation to Plans

```
PARSE PHASE                COMPILE PHASE           PLAN PHASE
═════════════════════════════════════════════════════════════

VectorExpression          compile_vector_         VectorOpType
                          expression()
┌─────────────────┐            ↓                 ┌──────────────┐
│ Similarity      │       Extract params      │ SimilaritySearch
│ - vector_name   │       Infer types         │ - params MAP
│ - reference     │       Validate            │ - input rows
│ - metric        │                          │
│ - threshold     │                          │
│ - vector_type   │                          │
└─────────────────┘                          │
        ↓                                     │
                                             │
┌─────────────────┐        ↓                 │
│ KNN             │    Create                │
│ - vector_name   │    VectorOperation       │ VectorOpType
│ - reference     │    ExecutionPlan         │ - KNN
│ - k             │                          │ - params MAP
│ - metric        │                          │ - input rows
│ - vector_type   │                          │
│ - diversity     │                          │
└─────────────────┘                          └──────────────┘
```

## 4. Executor Layer - Operation Flow

```
ExecutionPlan::VectorOperation
├─ op_type: VectorOpType
├─ params: HashMap<String, CompiledExpression>
└─ input: Option<ExecutionPlan>
          ↓
    ╔════════════════════════════╗
    ║   VectorEngine::execute    ║
    ║      operation()           ║
    ╚════════════════════════════╝
          ↓
    Match op_type:
    ├─ CosineSimilarity ──→ ┌─────────────────────────┐
    │                       │ extract 2 vectors       │
    │                       │ compute dot product     │
    │                       │ compute magnitudes      │
    │                       │ return: dot/(m1*m2)     │
    │                       └─────────────────────────┘
    │
    ├─ EuclideanDistance ─→ ┌─────────────────────────┐
    │                       │ extract 2 vectors       │
    │                       │ compute: sqrt(sum(a-b)²)│
    │                       │ return: distance        │
    │                       └─────────────────────────┘
    │
    ├─ DotProduct ────────→ ┌─────────────────────────┐
    │                       │ extract 2 vectors       │
    │                       │ compute: sum(a[i]*b[i]) │
    │                       │ return: dot product     │
    │                       └─────────────────────────┘
    │
    ├─ Normalize ────────→ ┌─────────────────────────┐
    │                      │ extract 1 vector        │
    │                      │ compute magnitude       │
    │                      │ compute: v / ||v||      │
    │                      │ return: Value::Vector   │
    │                      └─────────────────────────┘
    │
    ├─ KNN ─────────────→ ┌─────────────────────────┐
    │                     │ extract query_vec, k    │
    │                     │ brute force O(n*d)      │
    │                     │ sort by distance        │
    │                     │ return: top k rows      │
    │                     │ [NO INDEX USED]         │
    │                     └─────────────────────────┘
    │
    └─ SimilaritySearch ─→ ┌─────────────────────────┐
                           │ extract query, threshold│
                           │ brute force O(n*d)      │
                           │ filter by threshold     │
                           │ return: matching rows   │
                           │ [NO INDEX USED]         │
                           └─────────────────────────┘
```

## 5. Data Flow - Vector Through Query

```
INPUT: Entity Row
    |
    ├─ Properties: {name, age, ...}
    ├─ Embedding: Some(Vector { dims: [...] })
    └─ Position: Some(x, y, z)
        |
        v
    Row Extracted into ResultRow
        |
        ├─ columns: HashMap
        ├─ "name" → Value::String
        ├─ "age" → Value::Int
        └─ "embedding" → Value::Vector([...])
        |
        v
    Vector Operation Executed
        |
        ├─ extract_vector(expr, row, evaluator)
        ├─ Compute similarity/distance
        └─ Result column added
        |
        v
    Output ResultRow
        |
        ├─ columns: HashMap
        ├─ ... original columns ...
        ├─ "similarity" → Value::Float (0.95)
        └─ "distance" → Value::Distance(0.45)
```

## 6. Named Vectors - Entity Structure (Current vs Desired)

```
CURRENT IMPLEMENTATION:
═════════════════════════════════════════

pub struct Entity {
    id: EntityId,
    properties: HashMap<PropertyName, Value>,
    position: Option<Position3D>,
    embedding: Option<Vector>,  ← ONLY 1 VECTOR!
}

Problem:
- Can't store multiple named vectors
- Can't query "text_embedding SIMILAR TO ..."
- Can't have both "text_vec" and "code_vec"


DESIRED IMPLEMENTATION:
═════════════════════════════════════════

pub struct Entity {
    id: EntityId,
    properties: HashMap<PropertyName, Value>,
    position: Option<Position3D>,
    vectors: HashMap<String, Vector>,  ← MULTIPLE VECTORS!
}

Usage:
- vectors.get("text_embedding") → Some(Vector { ... })
- vectors.get("code_embedding") → Some(Vector { ... })
- vectors.get("sparse_keywords") → Some(Vector { ... })

Enables:
✓ Multiple named embeddings
✓ Vector queries by name
✓ Schema-aware vector storage
```

## 7. Type System - Vector Integration

```
VALUE TYPE SYSTEM
═════════════════════════════════════════

pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    EntityId(EntityId),
    Position(Position3D),
    Distance(HyperbolicDistance),
    Vector(Vector),         ← Vector type
    List(Vec<Value>),       ← Can contain vectors
    Map(HashMap<String, Value>),
    Timestamp(i64),
    Duration(i64),
}

VECTOR TYPE:
═════════════════════════════════════════

pub struct Vector {
    pub dimensions: Vec<f64>,
}

VECTORTYPE ENUM (in AST):
═════════════════════════════════════════

pub enum VectorType {
    Dense { dimensions: u32 },
    Sparse { max_dimensions: Option<u32> },
    ColBERT {
        token_dimensions: u32,
        max_tokens: Option<u32>,
    },
}
```

## 8. Metrics Supported

```
SIMILARITY METRICS - COMPREHENSIVE SUPPORT
═════════════════════════════════════════

pub enum SimilarityMetric {
    Cosine,          ← cos(θ) = ⟨a,b⟩/(||a|| ||b||) [0, 1]
    DotProduct,      ← ⟨a,b⟩ = Σᵢ aᵢbᵢ [-∞, ∞]
    Euclidean,       ← ||a-b||₂ inverted [0, ∞]
    Manhattan,       ← ||a-b||₁ inverted [0, ∞]
    Jaccard,         ← |A∩B|/|A∪B| [0, 1] (sparse only)
    Custom(String),  ← User-defined metric name
}

DISTANCE COMPUTATION:
═════════════════════════════════════════

Cosine:
    dot_product(v1, v2) / (magnitude(v1) * magnitude(v2))
    Result: [-1, 1] range

Euclidean:
    sqrt(sum((a[i] - b[i])²))
    Result: [0, ∞) range

DotProduct:
    sum(a[i] * b[i])
    Result: [-∞, ∞] range

Manhattan:
    sum(|a[i] - b[i]|)
    Result: [0, ∞) range

Jaccard (sparse):
    intersection_count / union_count
    Result: [0, 1] range
```

## 9. Cost Model

```
COST ESTIMATION LOGIC
═════════════════════════════════════════

Similarity Cost:
    base_cost = metric_factor
    metric_factor:
        Cosine: 1.5
        DotProduct: 1.0
        Euclidean: 1.2
        Manhattan: 1.1
        Jaccard: 2.0
        Custom: 3.0
    
    vector_type_factor:
        Dense(d): log₂(d) / 10
        Sparse: 1.5
        ColBERT: log₂(tokens*dims) / 8
    
    cost = base_cost * (1.0 + vector_type_factor)

KNN Cost:
    base_cost = k
    metric_factor: [same as above]
    vector_type_factor: [same as above]
    approximation_factor = 0.5 if approximate else 1.0
    diversity_factor = 1.0 + (constraints * 0.3)
    
    cost = base_cost * metric_factor * (1.0 + vector_type_factor)
           * approximation_factor * diversity_factor

EXAMPLE:
    Cosine similarity on 768D dense vector:
    base = 1.5
    type_factor = log₂(768)/10 ≈ 2.92
    cost = 1.5 * 1.292 ≈ 1.94

    KNN with k=10, cosine, 768D:
    base = 10
    cost = 10 * 1.5 * 1.292 * 1.0 * 1.0 ≈ 19.4
```

## 10. Missing Pieces - Implementation Roadmap

```
PARSER LAYER (0% → 100%)
═════════════════════════════════════════
Missing:
├─ SIMILARITY(field, ref) function parsing
├─ DISTANCE(field, ref) function parsing
├─ SIMILAR TO operator parsing
├─ Vector parameter syntax (@vec, VECTOR(...))
└─ Metric specification parsing

Example to implement:
    SELECT * FROM docs
    WHERE text_embedding SIMILAR TO @query THRESHOLD 0.8


COMPILER LAYER (100% ✓ but needs wiring)
═════════════════════════════════════════
Complete:
├─ VectorOpType enum (6 types)
├─ VectorOperation execution plan
└─ Type inference for vector functions

Needs:
└─ compile_vector_expression() implementation


DATA MODEL (0% → 100%)
═════════════════════════════════════════
Missing:
├─ Named vectors in Entity
├─ Vector retrieval methods in DataSource
├─ RouterDataSource implementation
└─ Vector metadata storage


EXECUTOR (30% → 100%)
═════════════════════════════════════════
Implemented:
├─ CosineSimilarity ✓
├─ EuclideanDistance ✓
├─ DotProduct ✓
└─ Normalize ✓

Needs optimization:
├─ KNN (add index support)
├─ SimilaritySearch (add index support)
└─ Batch operations


HYPERSPATIAL INTEGRATION (0% → 100%)
═════════════════════════════════════════
Missing:
├─ RouterDataSource → Router bridge
├─ HNSW index queries
├─ Multi-position vector handling
└─ Active/Passive node filtering
```

## 11. Complete Query Example - Journey Through Layers

```
INPUT QUERY (SQL):
═══════════════════════════════════════════════════════════
SELECT d.title, d.summary, sim
FROM documents d
WHERE text_embedding SIMILAR TO @query_vector THRESHOLD 0.8
ORDER BY sim DESC
LIMIT 10

        ↓ PARSER (MISSING - Would parse to AST)

AST REPRESENTATION:
═══════════════════════════════════════════════════════════
SelectStatement {
    select_list: [
        SelectItem::Expression(Column("d.title")),
        SelectItem::Expression(Column("d.summary")),
        SelectItem::Expression(Column("sim"))
    ],
    from: FromClause::Table("documents", None, Some("d")),
    where_clause: Some(Expression::Vector(
        VectorExpression::Similarity {
            vector_name: "text_embedding",
            reference: Box::new(Expression::Literal(Parameter("@query_vector"))),
            metric: SimilarityMetric::Cosine,
            threshold: Some(0.8),
            vector_type: VectorType::Dense { dimensions: 768 }
        }
    )),
    order_by: [
        OrderByItem {
            expr: Column("sim"),
            direction: OrderDirection::Desc
        }
    ],
    limit: Some(10),
    ...
}

        ↓ COMPILER (100% complete structure, needs parser input)

EXECUTION PLAN:
═══════════════════════════════════════════════════════════
ExecutionPlan::VectorOperation {
    op_type: VectorOpType::SimilaritySearch,
    params: {
        "query_vector" → CompiledExpression::Literal(Value::Vector([...])),
        "threshold" → CompiledExpression::Literal(Value::Float(0.8))
    },
    input: Some(Box::new(
        ExecutionPlan::Scan {
            table: "documents",
            entity_type: "",
            filter: None,
            projection: [title, summary],
            limit: None
        }
    ))
}

        ↓ EXECUTOR (30% - needs index for optimization)

EXECUTION:
═══════════════════════════════════════════════════════════
1. Scan documents (from input)
2. For each document:
   a. Extract text_embedding vector
   b. Compute cosine_similarity(doc.embedding, query_vector)
   c. Filter by threshold >= 0.8
3. Sort by similarity descending
4. Take top 10 rows
5. Add "sim" column to each row

OUTPUT RESULTS:
═══════════════════════════════════════════════════════════
ResultRow {
    columns: {
        "title" → Value::String("Machine Learning 101"),
        "summary" → Value::String("Introduction to ML concepts..."),
        "sim" → Value::Float(0.95)
    }
}
... (10 rows total, sorted by similarity)
```

## 12. Integration Pattern - Following Geometric Example

```
GEOMETRIC OPERATIONS (COMPLETE PATTERN):
═════════════════════════════════════════════════════════════

Parser:
    ✓ parse_geometric_expression()
    ✓ "WITHIN radius OF position" syntax

Compiler:
    ✓ compile_geometric_expression()
    ✓ Maps to GeometricOpType enum

Executor:
    ✓ GeometricEngine with 6 operations
    ✓ Fully implemented


VECTOR OPERATIONS (SHOULD FOLLOW SAME PATTERN):
═════════════════════════════════════════════════════════════

Parser:  [TO DO]
    [ ] parse_vector_expression()
    [ ] "vector_field SIMILAR TO ..." syntax
    [ ] SIMILARITY(...) function

Compiler:  [STRUCTURE READY, NEEDS IMPLEMENTATION]
    [ ] compile_vector_expression()
    [ ] Maps to VectorOpType enum
    [ ] Parameter extraction

Executor:  [PARTIALLY DONE]
    [✓] VectorEngine with 6 operations
    [✓] 4 fully implemented (Cosine, Euclidean, DotProduct, Normalize)
    [~] 2 need optimization (KNN, SimilaritySearch)
    [ ] Integration with Hyperspatial HNSW
```

---

End of diagrams. For implementation details, see VECTOR_INFRASTRUCTURE_ANALYSIS.md
