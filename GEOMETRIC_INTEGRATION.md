# Geometric Query Integration with Hyperspatial

## Overview

This document describes Phase 5.1 implementation: Geometric Query Syntax for HyperQL.

## Status

**PARSER COMPLETE** - Geometric query syntax is fully parsed and ready for execution integration.

## Syntax Implemented

### 1. NEAR with WITHIN (Radius Queries)

```sql
SELECT * FROM documents 
WHERE position NEAR reference WITHIN 2.0
```

- `position`: Keyword for multi-modal position field
- `NEAR`: Proximity operator
- `reference`: Entity ID or column reference to compare against
- `WITHIN`: Radius constraint
- `2.0`: Maximum distance (uses multi-modal weighted distance)

**Parsed as:** `Function { name: "near", args: [reference_expr, distance_expr] }`

### 2. DISTANCE with FROM (k-NN Queries)

```sql
SELECT * FROM documents 
ORDER BY position DISTANCE FROM reference 
LIMIT 10
```

- `position`: Keyword for multi-modal position field
- `DISTANCE FROM`: Distance calculation operator
- `reference`: Entity ID or column reference to compare against
- `LIMIT`: k value for k-NN

**Parsed as:** `Function { name: "distance", args: [reference_expr] }`

### 3. Combined Queries

```sql
SELECT id, title FROM documents 
WHERE position NEAR origin WITHIN 5.0 
ORDER BY position DISTANCE FROM origin 
LIMIT 100
```

## Parser Implementation

### Files Created

1. **`src/parser/geometric.rs`** (205 lines)
   - `parse_near_expression()`: Parses NEAR...WITHIN syntax
   - `parse_distance_expression()`: Parses DISTANCE...FROM syntax
   - Comprehensive test suite (8 unit tests)

### Files Modified

2. **`src/parser/mod.rs`**
   - Added `mod geometric`
   - Exported geometric parser functions

3. **`src/parser/expression.rs`**
   - Added geometric expression detection in `parse_simple_expression()`
   - Checks for NEAR/WITHIN keywords before regular expression parsing

4. **`src/parser/clause.rs`**
   - Extended `parse_order_by_item()` to recognize DISTANCE/FROM syntax
   - Handles ASC/DESC modifiers correctly

### Tests Created

5. **`tests/geometric_queries.rs`** (90 lines)
   - 10 integration tests covering:
     - Basic NEAR and DISTANCE parsing
     - Case insensitivity
     - Entity references vs. literals
     - Combined WHERE + ORDER BY queries
     - Edge cases (missing keywords)

## Execution Integration (Not Yet Implemented)

### Strategy

Since hyperQL and hyperspatial have a cyclic dependency (hyperspatial imports hyperQL), 
the actual execution integration must happen at the **hyperspatial/engine.rs** level,
not in hyperQL.

### Proposed Execution Flow

1. **Parser** (DONE):
   - Query string → AST with `Function { name: "near", ... }` or `Function { name: "distance", ... }`

2. **Compiler** (existing):
   - AST → CompiledQuery with `GeometricOperation` plan nodes

3. **Executor** (needs implementation at hyperspatial level):
   ```rust
   // In hyperspatial/src/query/hyperql/execution.rs or similar
   fn execute_geometric_operation(
       op_type: GeometricOpType,
       params: &HashMap<String, CompiledExpression>,
       global_index: &GlobalIndex,
       hnsw: Option<&HyperbolicHNSW>,
   ) -> Result<Vec<ResultRow>> {
       match op_type {
           GeometricOpType::NearPositions => {
               // Extract reference and radius from params
               let reference_position = /* get MultiModalPosition from global_index */;
               let radius = /* extract from params */;
               
               if let Some(hnsw_index) = hnsw {
                   // Strategy 1: Use HNSW with large k, filter by radius
                   let k = 1000;
                   let results = hnsw_index.search_knn_multi_position(
                       &reference_position, 
                       k, 
                       50, // ef
                       NodeClassFilter::All
                   );
                   
                   // Filter by radius
                   results.into_iter()
                       .filter(|(_, distance)| *distance <= radius)
                       .collect()
               } else {
                   // Strategy 2: Linear scan with GlobalIndex
                   // Iterate all positions, compute weighted distance, filter
               }
           }
           GeometricOpType::HyperbolicDistance => {
               // Detect ORDER BY + LIMIT pattern for k-NN optimization
               let k = /* extract from LIMIT */;
               let reference_position = /* get MultiModalPosition */;
               
               if let Some(hnsw_index) = hnsw {
                   // Direct HNSW k-NN query (O(log N))
                   hnsw_index.search_knn_multi_position(
                       &reference_position,
                       k,
                       50,
                       NodeClassFilter::All
                   )
               } else {
                   // Fallback: linear scan + sort
               }
           }
       }
   }
   ```

### Key Integration Points

1. **RouterDataSource** (hyperspatial/src/query/hyperql/datasource.rs):
   - Add methods to access GlobalIndex and HNSW
   - Bridge between hyperQL executor and hyperspatial indices

2. **GeometricEngine** (hyperQL/src/executor/geometric.rs):
   - Currently uses test Position3D type
   - Needs to be extended to work with any position provider trait
   - Should delegate to datasource for actual position lookups

3. **HyperQLEngine** (hyperspatial/src/query/hyperql/engine.rs):
   - Already has `global_index: Arc<GlobalIndex>`
   - Already has `hnsw: Option<Arc<HyperbolicHNSW>>`
   - Pass these to executor when executing geometric operations

### Type Mapping

**HyperQL types (hyperQL/src/types.rs):**
- `Position3D`: 3D visualization-only position (x, y, z)
- `EntityId`: String wrapper for entity identifiers
- `Value::Position`: Legacy 3D position value

**Hyperspatial types (hyperspatial/src/persistence/indices/):**
- `MultiModalPosition`: Three 17D positions (graph, embedding, property)
- `PositionND`: Single 17D hyperboloid position
- `GlobalKey`: Universal key format (collection:shard:entity-id)
- `HyperbolicHNSW`: Spatial index with O(log N) k-NN search

### Distance Computation

**Multi-modal weighted distance:**
```
distance = α × d_graph + β × d_embedding + γ × d_property
```

where each `d_X` is the hyperboloid distance between corresponding 17D positions.

**HNSW already computes this** using built-in `multi_modal_weights`:
- Query: `search_knn_multi_position(query, k, ef, filter)`
- Returns: `Vec<(GlobalKey, distance)>` sorted by distance
- No need to recompute distances - HNSW provides them directly

### Optimization Strategies

1. **k-NN with LIMIT (ORDER BY + LIMIT)**:
   - Pattern: `ORDER BY position DISTANCE FROM ref LIMIT k`
   - Strategy: Direct HNSW k-NN query
   - Complexity: O(log N)
   - Query time: 40-79μs average

2. **Radius query (NEAR + WITHIN)**:
   - Pattern: `WHERE position NEAR ref WITHIN r`
   - Strategy: HNSW with large k, filter by radius
   - Fallback: Linear scan with GlobalIndex
   - Complexity: O(log N) with HNSW, O(N) without

3. **Combined query**:
   - Pattern: `WHERE ... NEAR ... ORDER BY ... DISTANCE ... LIMIT k`
   - Strategy: HNSW k-NN first, then filter by radius
   - Complexity: O(log N)

## Testing

### Parser Tests (All Passing)

```bash
cd hyperQL
cargo test parser::geometric  # 8 unit tests
cargo test --test geometric_queries  # 10 integration tests
cargo test --lib  # 160 tests total (all pass)
```

### Integration Tests (Not Yet Implemented)

Recommended test file: `hyperspatial/tests/hyperql_geometric_integration.rs`

```rust
#[tokio::test]
async fn test_near_query_with_real_positions() {
    // Setup: Create entities with real MultiModalPosition
    let router = /* initialize */;
    let global_index = /* initialize */;
    let hnsw = /* build HNSW */;
    
    let engine = HyperQLEngine::from_components(
        router, 
        coordinator, 
        global_index, 
        Some(hnsw)
    ).unwrap();
    
    // Execute: NEAR query
    let results = engine.execute(
        "SELECT * FROM docs WHERE position NEAR origin WITHIN 2.0"
    ).unwrap();
    
    // Verify: Results within radius
    assert!(results.rows.len() > 0);
    // Check distances are <= 2.0
}

#[tokio::test]
async fn test_knn_query_with_hnsw() {
    // Test ORDER BY + LIMIT optimization path
    let results = engine.execute(
        "SELECT * FROM docs ORDER BY position DISTANCE FROM origin LIMIT 10"
    ).unwrap();
    
    assert_eq!(results.rows.len(), 10);
    // Verify results are sorted by distance
}
```

## Performance Expectations

Based on existing HNSW benchmarks (20K entities):

- **k-NN query latency**: 40-79μs average
- **Throughput**: 12,000-20,000 queries/second
- **Build time**: 13-15s (single mode), 60s (multi-modal)
- **Memory**: 4 MB for multi-positions (204 bytes × 20K)

## Next Steps

1. **Implement execution in hyperspatial**:
   - Create `geometric_execution.rs` module
   - Wire GeometricOpType handlers to GlobalIndex/HNSW
   - Add tests with real position data

2. **Update RouterDataSource**:
   - Add `get_multi_modal_position(key) -> Option<MultiModalPosition>`
   - Add `get_global_index() -> &GlobalIndex`
   - Add `get_hnsw() -> Option<&HyperbolicHNSW>`

3. **Extend GeometricEngine**:
   - Make position access trait-based
   - Support both 3D (legacy) and MultiModalPosition
   - Add proper error handling for missing positions

4. **Add comprehensive tests**:
   - Integration tests with real data
   - Performance benchmarks
   - Edge cases (missing HNSW, missing positions, etc.)

## References

- COMPLETION_PLAN.md: Phase 5.1 requirements
- CLAUDE.md: Multi-position architecture
- hyperspatial/src/persistence/indices/hnsw.rs: HNSW implementation
- hyperspatial/src/persistence/indices/multi_position.rs: MultiModalPosition
