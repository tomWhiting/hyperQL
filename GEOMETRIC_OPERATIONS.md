# Geometric Operations Implementation

## Overview

This document describes the implementation of geometric operations for HyperQL, enabling hyperbolic distance calculations and position-based query filtering.

## Architecture

### Core Components

#### 1. GeometricEngine (`src/executor/geometric.rs`)
The geometric engine implements real hyperbolic distance calculations using the Minkowski inner product formulation:

**Key Operations:**
- **hyperbolic_distance**: Calculates geodesic distance between two positions in hyperbolic space
- **within_radius**: Filters entities within a specified hyperbolic radius from a center point
- **near_positions**: Returns entities sorted by proximity to a reference position
- **geodesic_distance**: Alias for hyperbolic_distance

**Mathematical Foundation:**
```rust
// Minkowski inner product for hyperboloid model:
// <x,y>_L = -x_0·y_0 + x_1·y_1 + ... + x_d·y_d
fn minkowski_inner_from_pos(&self, pos1: &Position3D, pos2: &Position3D) -> f64 {
    -pos1.x * pos2.x + pos1.y * pos2.y + pos1.z * pos2.z
}

// Hyperbolic distance:
// d_H(x,y) = sqrt(K) · acosh(-<x,y>_L / K)
let distance = sqrt_c * arg.acosh();
```

#### 2. Integration with PlanExecutor

The geometric engine integrates seamlessly with the existing executor architecture:

```rust
pub struct PlanExecutor {
    data_source: Box<dyn DataSource>,
    stats_collector: StatsCollector,
    expression_evaluator: ExpressionEvaluator,
    aggregation_engine: AggregationEngine,
    geometric_engine: GeometricEngine,  // New component
}
```

## Implementation Details

### Position Extraction

The engine supports multiple ways to access position data:

1. **From Position3D values**: Direct position objects
2. **From row columns**: Extracts x, y, z from result rows
3. **From entity properties**: Accesses entity.position field

### Error Handling

Comprehensive error handling for:
- Missing positions (entities without learned positions)
- Invalid parameters
- Type mismatches
- Empty input sets

**Example Error:**
```rust
Entity does not have a position. Position learning must be run first.
```

### Curvature Support

The geometric engine supports configurable curvature:

```rust
let engine = GeometricEngine::new();  // Default curvature = 1.0
let engine = GeometricEngine::new_with_curvature(2.0);  // Custom curvature
```

## Usage Examples

### Basic Distance Calculation

```hyperql
-- Calculate hyperbolic distance between two entities
SELECT 
    e1.id as entity1,
    e2.id as entity2,
    hyperbolic_distance(e1.position, e2.position) as distance
FROM entities e1, entities e2
WHERE e1.id = 'entity_origin' AND e2.id = 'entity_nearby1'
```

### Within Radius Filter

```hyperql
-- Find all entities within radius 2.0 of a center point
SELECT id, name, distance_from_center
FROM entities
WHERE within_radius(position, center_position, 2.0)
ORDER BY distance_from_center ASC
```

### Nearest Neighbors

```hyperql
-- Find 10 nearest entities to a reference position
SELECT id, name, distance
FROM entities
WHERE near_positions(position, reference_position, 10)
ORDER BY distance ASC
```

## Position Learning

Positions must be learned via HGCN training before geometric queries can be used. See the Hyperspatial crate for position learning:

```rust
// In Hyperspatial (not HyperQL)
use hyperspatial::hyperbolic::hyperboloid::model::HGCN;

let model = HGCN::new(dim, num_nodes, curvature);
model.train(graph, learning_rate, epochs);
let positions = model.get_embeddings();
```

Once positions are learned, they can be stored with entities and used in HyperQL geometric queries.

## Performance Characteristics

### Complexity

- **hyperbolic_distance**: O(1) per pair
- **within_radius**: O(n) where n = number of entities
- **near_positions**: O(n log n) due to sorting

### Optimization Opportunities

Future optimizations may include:
1. Spatial indexing (R-tree or similar)
2. Approximate nearest neighbor search
3. Distance matrix caching
4. Parallel distance computation

## Testing

Run the geometric operations example:

```bash
cargo run --example geometric_queries
```

The example demonstrates:
- Entities with learned positions
- Simple position queries
- Distance calculations
- Position-based filtering

## Integration with Hyperspatial

The geometric operations are designed to work with positions learned by Hyperspatial's HGCN implementation:

1. **Hyperspatial** learns positions using graph structure
2. **HyperQL** queries entities using those positions
3. Positions are stored in the unified Position3D format
4. Both systems use the same hyperboloid model

## Future Enhancements

Planned additions:
1. **Geodesic paths**: Calculate paths along hyperbolic geodesics
2. **Geometric regions**: Support for hyperbolic balls, horoballs
3. **Containment testing**: Check if point is within geometric region
4. **Intersection testing**: Determine if regions intersect
5. **Parallel transport**: Move vectors along geodesics

## Files Modified

- `src/executor/geometric.rs` - New geometric engine implementation
- `src/executor/plan_executor.rs` - Integration with executor
- `src/executor/mod.rs` - Module registration
- `src/executor/data_source.rs` - Added Clone derive for MemoryDataSource
- `examples/geometric_queries.rs` - New example demonstrating usage

## References

- Hyperbolic Graph Convolution Networks (HGCN)
- Hyperboloid model of hyperbolic geometry
- Minkowski inner product
- Geodesic distance in hyperbolic space
