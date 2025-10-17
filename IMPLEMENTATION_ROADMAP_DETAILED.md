# HyperQL SQL Implementation Roadmap

## Overview

This document provides a detailed implementation roadmap to "fully implement all SQL syntax" in HyperQL.

**Current State:** 45-50% complete (core SELECT/INSERT/UPDATE/DELETE working)  
**Target State:** 95%+ complete (full SQL support)  
**Estimated Timeline:** 2-3 weeks of focused development  
**Required Lines of Code:** 6,000-8,000 new lines

---

## Phase 1: Core SQL (Week 1) - 3 Days Critical Path

### 1.1 JOIN Operations (2-3 days)

**Why First:** Most queries need JOINs, infrastructure exists, high-impact

**Implementation Steps:**

1. **Extend AST (1-2 hours)**
   - Add `FromClause::Join` variant to `/src/ast/mod.rs`
   ```rust
   pub enum FromClause {
       Table { name: String, alias: Option<String> },
       Subquery { query: Box<SelectStatement>, alias: String },
       Join {
           left: Box<FromClause>,
           join_type: JoinType,
           right: Box<FromClause>,
           on_clause: Expression,
       }
   }
   
   pub enum JoinType {
       Inner, Left, Right, Full, Cross,
   }
   ```

2. **Parser Updates (4-6 hours)**
   - File: `/src/parser/select.rs`
   - Change FROM parsing from single table to recursive JOIN parsing
   - Add function: `parse_from_clause(input: &str) -> Result<FromClause>`
   - Parse: `table1 INNER JOIN table2 ON table1.id = table2.id`
   - Handle: `... JOIN ... JOIN ...` chaining
   
   ```rust
   fn parse_from_clause(input: &str) -> Result<FromClause> {
       // 1. Parse first table
       // 2. Loop: if sees JOIN keyword, parse join type, table, ON condition
       // 3. Build nested FromClause::Join tree
   }
   ```

3. **Compiler Updates (4-6 hours)**
   - File: `/src/compiler/mod.rs`
   - Add `ExecutionPlan::Join` variant
   - File: `/src/compiler/select.rs`
   - Add compilation logic for JOINs
   ```rust
   pub enum ExecutionPlan {
       // ... existing variants ...
       Join {
           left: Box<ExecutionPlan>,
           right: Box<ExecutionPlan>,
           join_type: JoinType,
           on_condition: CompiledExpression,
       }
   }
   ```

4. **Executor Implementation (8-12 hours)**
   - File: `/src/executor/plan_executor.rs`
   - Add match arm for `ExecutionPlan::Join`
   - Implement `execute_join()` function
   - Start with nested-loop join (simplest):
   ```rust
   fn execute_join(&mut self, left_plan: &ExecutionPlan, right_plan: &ExecutionPlan, 
                   join_type: JoinType, condition: &CompiledExpression) -> Result<Vec<ResultRow>> {
       let left_rows = self.execute_plan(left_plan)?;
       let right_rows = self.execute_plan(right_plan)?;
       let mut result = Vec::new();
       
       for left_row in &left_rows {
           for right_row in &right_rows {
               let combined = combine_rows(left_row, right_row);
               if self.expression_evaluator.eval(&condition, &combined)? == Value::Bool(true) {
                   result.push(combined);
               }
           }
       }
       Ok(result)
   }
   ```

5. **Testing**
   - Add test: `SELECT t1.name, t2.dept FROM table1 t1 INNER JOIN table2 t2 ON t1.id = t2.id`
   - Add test: Multiple JOINs
   - Add test: LEFT/RIGHT/FULL JOINs

**Files Modified:**
- `/src/ast/mod.rs`
- `/src/parser/select.rs`
- `/src/compiler/mod.rs`
- `/src/compiler/select.rs`
- `/src/executor/plan_executor.rs`
- `/tests/join_operations.rs` (new)

**Success Metrics:**
- Parses JOIN syntax without errors
- Generates ExecutionPlan with Join variant
- Executor produces correct result rows

---

### 1.2 Subqueries (2-3 days)

**After:** JOIN (because subqueries can be in FROM for a simpler start)

**Implementation Steps:**

1. **Parser Updates (6-8 hours)**
   - File: `/src/parser/select.rs`
   - In `parse_from_clause()`, detect `(` as subquery indicator
   - Call `parse_select_statement()` recursively
   - Handle `AS alias` after `)`
   
   ```rust
   fn parse_from_term(input: &str) -> Result<FromClause> {
       if input.trim().starts_with('(') {
           // Find matching )
           let inner_select = parse_select_statement(&inner_part)?;
           let alias = parse_alias_after_paren()?;
           Ok(FromClause::Subquery { query: Box::new(inner_select), alias })
       } else {
           Ok(FromClause::Table { name, alias })
       }
   }
   ```

2. **Compiler Updates (4-6 hours)**
   - Handle `FromClause::Subquery` in select compiler
   - Generate plan that:
     1. Executes inner SELECT
     2. Treats result as temporary table
     3. Joins/filters with outer query

3. **Executor Updates (4-6 hours)**
   - Plan executor handles subquery execution
   - Cache subquery results (don't re-execute)

4. **Testing**
   - `FROM (SELECT * FROM t1) sub WHERE sub.val > 5`
   - `FROM t1 JOIN (SELECT * FROM t2) sub ON t1.id = sub.id`

---

### 1.3 Set Operations (1 day)

**After:** Basic SELECT working

**Implementation Steps:**

1. **Parser (2-3 hours)**
   - File: `/src/parser/select.rs`
   - Detect UNION/INTERSECT/EXCEPT keywords after first SELECT
   - Parse second SELECT recursively
   
   ```rust
   // After parsing first SELECT
   if input.contains("UNION") {
       let (left_select, right_select) = split_at_union(input)?;
       // Create SetOperation
   }
   ```

2. **AST (1-2 hours)**
   - Add `Statement::SetOperation { left, op, right }`
   ```rust
   pub enum Statement {
       Select(SelectStatement),
       Insert(InsertStatement),
       Update(UpdateStatement),
       Delete(DeleteStatement),
       SetOperation {
           left: Box<SelectStatement>,
           operator: SetOperator,
           right: Box<SelectStatement>,
       }
   }
   
   pub enum SetOperator {
       Union,
       UnionAll,
       Intersect,
       Except,
   }
   ```

3. **Compiler (3-4 hours)**
   - Add `ExecutionPlan::SetOperation { ... }`

4. **Executor (3-4 hours)**
   - Implement set algebra:
     - UNION: merge + dedup
     - INTERSECT: rows in both
     - EXCEPT: rows in left only

---

## Phase 2: Advanced SQL (Week 2) - 4-5 Days

### 2.1 CASE Expressions (1-2 days)

**Why Easy:** IR type exists, just needs plumbing

**Implementation:**

1. **Parser (3-4 hours)**
   - Detect `CASE` keyword in expression parsing
   - Parse `WHEN condition THEN expr` clauses
   - Parse optional `ELSE expr`
   
   ```rust
   // In parse_simple_expression or parse_expression_or_function
   if input.contains("CASE") {
       let case_expr = parse_case_expression(input)?;
   }
   ```

2. **Add to AST Expression (1 hour)**
   ```rust
   pub enum Expression {
       // ... existing ...
       Case {
           whens: Vec<(Expression, Expression)>, // (when, then)
           else_expr: Option<Box<Expression>>,
       }
   }
   ```

3. **Compiler (2-3 hours)**
   - Translate to `CompiledExpression::Case`
   - Add Case variant to CompiledExpression

4. **Executor (1-2 hours)**
   - In `expression_eval.rs`, handle Case:
   ```rust
   CompiledExpression::Case { whens, else_expr } => {
       for (when_cond, then_expr) in whens {
           if self.eval(when_cond)? == Value::Bool(true) {
               return self.eval(then_expr);
           }
       }
       if let Some(else_expr) = else_expr {
           self.eval(else_expr)
       } else {
           Ok(Value::Null)
       }
   }
   ```

**Files:**
- `/src/parser/expression.rs`
- `/src/ast/mod.rs`
- `/src/compiler/expression.rs`
- `/src/executor/expression_eval.rs`

---

### 2.2 Window Functions (4-5 days)

**Why Complex:** Requires frame computation

**Implementation:**

1. **Parser (8-10 hours)**
   - Recognize: `ROW_NUMBER()`, `RANK()`, `LAG()`, etc.
   - Parse: `OVER (PARTITION BY col ORDER BY col ROWS BETWEEN ...)`
   
   ```rust
   fn parse_window_function(func_name: &str, args: &[Expression], 
                           over_clause: &str) -> Result<Expression> {
       let partition_cols = parse_partition_by(&over_clause)?;
       let order_cols = parse_order_by(&over_clause)?;
       let frame = parse_frame_spec(&over_clause)?;
       
       Ok(Expression::Window {
           function: func_name,
           args,
           partition_by: partition_cols,
           order_by: order_cols,
           frame,
       })
   }
   ```

2. **AST (2-3 hours)**
   ```rust
   pub enum Expression {
       Window {
           function: String,
           args: Vec<Expression>,
           partition_by: Vec<Expression>,
           order_by: Vec<(Expression, OrderDirection)>,
           frame: FrameSpec,
       }
   }
   
   pub enum FrameSpec {
       UnboundedPreceding,
       RangeRows { start: FrameBound, end: FrameBound },
   }
   
   pub enum FrameBound {
       UnboundedPreceding,
       Preceding(u32),
       CurrentRow,
       Following(u32),
       UnboundedFollowing,
   }
   ```

3. **Compiler (4-6 hours)**
   - Generate window computation plans
   - Add `ExecutionPlan::Window { ... }`

4. **Executor (10-14 hours)**
   - Implement window computation:
     1. Partition rows by PARTITION BY columns
     2. Within each partition, sort by ORDER BY
     3. For each row, compute window functions over frame
     4. Attach results to original rows
   
   ```rust
   fn execute_window(&mut self, input: &ExecutionPlan, 
                    partition_cols: &[CompiledProjection],
                    window_functions: &[WindowFunc]) -> Result<Vec<ResultRow>> {
       let mut input_rows = self.execute_plan(input)?;
       
       // Group by partition columns
       let mut partitions: HashMap<PartitionKey, Vec<ResultRow>> = HashMap::new();
       for row in input_rows {
           let key = extract_partition_key(&row, partition_cols)?;
           partitions.entry(key).or_default().push(row);
       }
       
       let mut result = Vec::new();
       for (_, partition) in partitions {
           // Sort partition by ORDER BY
           let sorted_partition = self.sort_rows(partition, order_cols)?;
           
           // Compute window functions for each row
           for (i, row) in sorted_partition.iter().enumerate() {
               let frame_start = self.compute_frame_start(i, &window_frame);
               let frame_end = self.compute_frame_end(i, &window_frame);
               let frame_rows = &sorted_partition[frame_start..=frame_end];
               
               let row_with_window = self.add_window_values(row, frame_rows, 
                                                           &window_functions)?;
               result.push(row_with_window);
           }
       }
       Ok(result)
   }
   ```

**Files:**
- `/src/parser/expression.rs`
- `/src/ast/mod.rs`
- `/src/compiler/expression.rs`
- `/src/executor/plan_executor.rs`
- `/src/executor/window.rs` (new)

---

### 2.3 CTEs - Basic (1-2 days)

**Start Simple:** Non-recursive CTEs only

**Implementation:**

1. **Parser (6-8 hours)**
   - Detect `WITH cte_name AS (SELECT ...)`
   - Handle multiple CTEs: `WITH cte1 AS (...), cte2 AS (...)`
   - Remaining `SELECT` uses CTE

2. **AST (2-3 hours)**
   ```rust
   pub struct WithClause {
       pub ctes: Vec<CommonTableExpression>,
       pub query: Box<SelectStatement>,
   }
   
   pub struct CommonTableExpression {
       pub name: String,
       pub query: SelectStatement,
   }
   
   pub enum Statement {
       SelectWith(WithClause),
       // ...
   }
   ```

3. **Compiler (4-6 hours)**
   - Inline strategy: substitute CTE name with subquery
   - Or materialize: execute CTE once, cache

4. **Executor (4-6 hours)**
   - Execute once, cache results
   - Look up by name when referenced

**Files:**
- `/src/parser/select.rs`
- `/src/ast/mod.rs`
- `/src/compiler/select.rs`
- `/src/executor/plan_executor.rs`

---

## Phase 3: Hyperbolic-Specific Execution (1-2 days)

### 3.1 Geometric Operations Executor (1 day)

**Files:** `/src/executor/geometric.rs` (currently returns "not implemented")

**Implementation:**
1. Add hyperbolic distance formula
2. Integrate with Hyperspatial for position retrieval
3. Implement WITHIN radius filtering
4. Implement NEAR distance sorting

### 3.2 Vector Operations Executor (1 day)

**Files:** `/src/executor/vector.rs` (currently returns "not implemented")

**Implementation:**
1. Implement cosine similarity
2. Integrate with Hyperspatial for embedding retrieval
3. Implement KNN queries on HNSW index

---

## Phase 4: Polish & Testing (Optional, 2-3 days)

### 4.1 Recursive CTEs

**After:** Basic CTEs working

**Add:**
- WITH RECURSIVE support
- Cycle detection
- Termination condition validation
- Convergence detection in executor

### 4.2 Advanced Aggregation

**Add:**
- GROUP BY ROLLUP
- GROUP BY CUBE
- GROUPING SETS

### 4.3 Comprehensive Testing

- Fuzz testing for parser
- Integration tests for each feature
- Performance benchmarks
- Edge case handling

---

## Quick-Win Priority List

**If Only 1 Week Available:**
1. **JOIN** (3 days) - Most impactful
2. **Set Operations** (1 day) - Quick win
3. **Subqueries** (2 days) - Enables complex queries
4. **CASE** (1 day) - Quick win

**If Only 2 Weeks Available:**
1. Above (1 week)
2. **Window Functions** (4 days)
3. **CTEs** (2 days)

---

## Testing Strategy

### Unit Tests

Each feature needs:
- Parser tests (valid syntax, error cases)
- AST generation tests
- Compiler plan generation tests
- Executor result correctness tests

**Example Test Structure:**
```rust
#[test]
fn test_join_inner() {
    let query = "SELECT t1.id, t2.name FROM table1 t1 INNER JOIN table2 t2 ON t1.id = t2.id";
    let result = parse_and_execute(query);
    assert_eq!(result.rows.len(), expected_count);
}

#[test]
fn test_case_simple() {
    let query = "SELECT CASE WHEN val > 100 THEN 'high' ELSE 'low' END FROM t";
    let result = parse_and_execute(query);
    assert!(result.rows[0].get("CASE").is_ok());
}
```

### Integration Tests

- Multi-table queries with JOINs, subqueries, CTEs
- Complex window function queries
- Set operations combining multiple SELECT statements

### Performance Tests

- Query compilation time
- Execution time on various data sizes
- Memory usage

---

## Validation & Compatibility

**Good News:** Validator already recognizes keywords:
- JOIN, INNER, LEFT, RIGHT, OUTER (line 643 in validator/statement.rs)
- ON, USING (line 644)
- CASE, WHEN, THEN, ELSE (recognized but not validated)
- WITH (needs validation update)

No validator changes needed once parser is implemented.

---

## Estimated Effort Summary

| Feature | Parser | Compiler | Executor | Total |
|---------|--------|----------|----------|-------|
| JOIN | 4-6h | 4-6h | 8-12h | 16-24h (2-3 days) |
| Subqueries | 6-8h | 4-6h | 4-6h | 14-20h (2 days) |
| Set Operations | 2-3h | 3-4h | 3-4h | 8-11h (1 day) |
| CASE | 3-4h | 2-3h | 1-2h | 6-9h (1 day) |
| Window Functions | 8-10h | 10-12h | 10-14h | 28-36h (4-5 days) |
| CTEs (basic) | 6-8h | 4-6h | 4-6h | 14-20h (2 days) |
| Geometric Exec | - | - | 8-12h | 8-12h (1 day) |
| Vector Exec | - | - | 8-12h | 8-12h (1 day) |

**Total New Code:** 6,000-8,000 lines  
**Total Development Time:** 2-3 weeks  
**Per-Feature Complexity:** LOW to MEDIUM (mostly plumbing, not algorithm complexity)

---

## Success Criteria

After implementation, HyperQL should support:

1. **All SQL SELECT variants:**
   - Multiple table JOINs (INNER, LEFT, RIGHT, FULL, CROSS)
   - Subqueries in FROM and WHERE
   - Set operations (UNION, INTERSECT, EXCEPT)
   - CTEs and recursive CTEs
   - Window functions with frames

2. **Expression Features:**
   - CASE/WHEN/THEN/ELSE
   - Aggregate functions with FILTER
   - Window functions with partition/order/frame

3. **Hyperbolic-Specific:**
   - Geometric operations (distance, WITHIN, NEAR)
   - Vector operations (similarity, KNN)
   - ProximityJoin and SimilarityJoin

4. **Test Coverage:**
   - 100+ integration tests
   - All edge cases covered
   - Performance acceptable (<1s for typical queries)

---

## Files Summary

**New Files:**
- `/tests/join_operations.rs`
- `/tests/subquery_operations.rs`
- `/tests/set_operations.rs`
- `/tests/case_expressions.rs`
- `/tests/window_functions.rs`
- `/tests/cte_operations.rs`
- `/src/executor/window.rs`
- `/src/parser/cte.rs` (optional, can inline in select.rs)

**Modified Files:**
- `/src/ast/mod.rs` - Add expression/statement variants
- `/src/parser/select.rs` - Parser for all features
- `/src/parser/expression.rs` - Expression parsing
- `/src/compiler/mod.rs` - Add ExecutionPlan variants
- `/src/compiler/select.rs` - Compilation logic
- `/src/compiler/expression.rs` - Expression compilation
- `/src/executor/plan_executor.rs` - Add execution for new plans
- `/src/executor/expression_eval.rs` - Add expression evaluation
- `/src/executor/geometric.rs` - Complete geometry implementation
- `/src/executor/vector.rs` - Complete vector implementation

**Unchanged:**
- `/src/validator/` - Already recognizes keywords
- `/src/optimizer/` - Already handles most cases
- `/src/type_checker.rs` - Minor updates for new types

---

## Next Steps

1. **Start with JOIN** (highest impact, 2-3 days)
2. **Follow with Set Operations** (quick win, 1 day)
3. **Then Subqueries** (enables complex queries, 2 days)
4. **Then CASE** (quick win, 1 day)
5. **Then Window Functions** (analytical queries, 4-5 days)
6. **Then CTEs** (readability, 2+ days)
7. **Finally:** Hyperbolic execution (1-2 days) + recursive CTEs (1-2 days)

**Realistic Timeline:** 2-3 weeks for core features, 1 additional week for polish

