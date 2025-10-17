# HyperQL JOIN Implementation: Gaps and Recommended Fixes

## Overview

This document details the specific gaps in the JOIN implementation and provides code snippets for recommended fixes.

---

## Gap 1: No Column Prefixing with Table Aliases

### The Problem

When joining two tables with overlapping column names, the current implementation combines columns without prefixing, causing data loss.

### Current Code (BROKEN)

**File:** `/src/executor/join.rs` (lines 277-292)

```rust
/// Combine two rows into a single row
fn combine_rows(left: &ResultRow, right: &ResultRow) -> ResultRow {
    let mut columns = HashMap::new();

    // Add all left columns (NO PREFIX)
    for (key, value) in &left.columns {
        columns.insert(key.clone(), value.clone());
    }

    // Add all right columns (NO PREFIX) - OVERWRITES LEFT COLUMNS WITH SAME NAME!
    for (key, value) in &right.columns {
        columns.insert(key.clone(), value.clone());  // BUG: HashMap::insert overwrites!
    }

    ResultRow { columns }
}
```

### Example of the Bug

```
Query:  SELECT p.name, a.name FROM patients p JOIN admissions a ON ...

Patient row:   {id: "p1", name: "Alice", age: 30}
Admission row: {id: "a1", hadm_id: 101, subject_id: 1}

Result:
Expected: {p.id: "p1", p.name: "Alice", p.age: 30, a.id: "a1", a.hadm_id: 101, a.subject_id: 1}
          (columns prefixed with table alias)

Actual:   {id: "a1", name: "Alice", age: 30, hadm_id: 101, subject_id: 1}
          (no prefixing, ambiguous, loses a.id)

If BOTH had 'name':
Patient row:   {name: "Alice"}
Admission row: {name: "Admission-001"}

Result:
Expected: {p.name: "Alice", a.name: "Admission-001"}
Actual:   {name: "Admission-001"}  <-- SILENT DATA LOSS!
```

### Recommended Fix

**Step 1: Add alias field to ExecutionPlan**

File: `/src/compiler/mod.rs` (lines 129-135)

Current:
```rust
Scan {
    table: String,
    entity_type: String,
    filter: Option<CompiledExpression>,
    projection: Vec<CompiledProjection>,
    limit: Option<u64>,
},
```

Fixed:
```rust
Scan {
    table: String,
    entity_type: String,
    alias: Option<String>,  // ADD THIS LINE
    filter: Option<CompiledExpression>,
    projection: Vec<CompiledProjection>,
    limit: Option<u64>,
},
```

**Step 2: Update compiler to propagate aliases**

File: `/src/compiler/select.rs` (lines 135-141)

Current:
```rust
Ok(ExecutionPlan::Scan {
    table: table_name,
    entity_type,
    filter: None,
    projection: vec![],
    limit: scan_limit,
})
```

Fixed:
```rust
Ok(ExecutionPlan::Scan {
    table: table_name,
    entity_type,
    alias: select.from.as_ref().and_then(|f| {
        match f {
            FromClause::Table { alias, .. } => alias.clone(),
            _ => None,
        }
    }),  // ADD THIS
    filter: None,
    projection: vec![],
    limit: scan_limit,
})
```

**Step 3: Track table alias in JoinExecutor**

File: `/src/executor/join.rs` - Modify to pass alias context

```rust
pub struct JoinContext {
    left_alias: Option<String>,
    right_alias: Option<String>,
}

fn combine_rows_with_alias(
    left: &ResultRow, 
    right: &ResultRow,
    context: &JoinContext,
) -> ResultRow {
    let mut columns = HashMap::new();

    // Add left columns with prefix if alias exists
    for (key, value) in &left.columns {
        let prefixed_key = if let Some(ref alias) = context.left_alias {
            format!("{}.{}", alias, key)
        } else {
            key.clone()
        };
        columns.insert(prefixed_key, value.clone());
    }

    // Add right columns with prefix if alias exists
    for (key, value) in &right.columns {
        let prefixed_key = if let Some(ref alias) = context.right_alias {
            format!("{}.{}", alias, key)
        } else {
            key.clone()
        };
        columns.insert(prefixed_key, value.clone());
    }

    ResultRow { columns }
}
```

---

## Gap 2: Alias Information Lost in ExecutionPlan

### The Problem

Table aliases are captured in the AST but not propagated through the ExecutionPlan, so the executor has no way to know the original table alias.

### Current Code (INCOMPLETE)

**File:** `/src/compiler/mod.rs` (lines 190-196)

```rust
Join {
    left: Box<ExecutionPlan>,
    right: Box<ExecutionPlan>,
    join_type: JoinType,
    on_condition: CompiledExpression,
    // MISSING: No alias fields to identify source tables!
},
```

### Where Aliases Are Lost

1. **AST captures alias:**
   ```rust
   // FROM patients p
   FromClause::Table {
       collection: "patients".to_string(),
       entity_type: "Patient".to_string(),
       alias: Some("p".to_string()),  // CAPTURED HERE
   }
   ```

2. **But compiler doesn't propagate:**
   ```rust
   ExecutionPlan::Scan {
       table: "patients".to_string(),
       entity_type: "Patient".to_string(),
       // alias LOST HERE - no field to store it
   }
   ```

### Recommended Fix

Add alias tracking to ExecutionPlan::Join:

```rust
Join {
    left: Box<ExecutionPlan>,
    right: Box<ExecutionPlan>,
    left_alias: Option<String>,   // ADD THIS
    right_alias: Option<String>,  // ADD THIS
    join_type: JoinType,
    on_condition: CompiledExpression,
},
```

**Update compiler/select.rs (lines 271-298):**

```rust
fn compile_joins(&self, left_plan: ExecutionPlan, joins: &[JoinClause]) -> Result<ExecutionPlan> {
    let mut current_plan = left_plan;
    let mut left_alias = None;  // Track left table alias

    for join_clause in joins {
        let right_plan = ExecutionPlan::Scan {
            table: join_clause.collection.clone(),
            entity_type: join_clause.entity_type.clone(),
            alias: join_clause.alias.clone(),  // Pass through alias
            filter: None,
            projection: vec![],
            limit: None,
        };

        let compiled_condition = self.expression_compiler.compile_expression(join_clause.on_condition.clone())?;

        current_plan = ExecutionPlan::Join {
            left: Box::new(current_plan),
            right: Box::new(right_plan),
            left_alias: left_alias.clone(),              // ADD THIS
            right_alias: join_clause.alias.clone(),      // ADD THIS
            join_type: join_clause.join_type.clone(),
            on_condition: compiled_condition,
        };
        
        left_alias = join_clause.alias.clone();  // Update for next iteration
    }

    Ok(current_plan)
}
```

---

## Gap 3: Executor Doesn't Track Table Context

### The Problem

When executing JOINs, the executor receives the alias information but doesn't pass it to the join executor, so the join executor can't prefix columns.

### Current Code (INCOMPLETE)

**File:** `/src/executor/plan_executor.rs` (lines 395-414)

```rust
fn execute_join(
    &mut self,
    left: &ExecutionPlan,
    right: &ExecutionPlan,
    join_type: &crate::compiler::JoinType,
    on_condition: &CompiledExpression,
) -> Result<Vec<ResultRow>> {
    let left_rows = self.execute_plan(left)?;
    let right_rows = self.execute_plan(right)?;

    // NO ALIAS INFORMATION PASSED TO JOINEXECUTOR!
    JoinExecutor::execute_join(
        left_rows,
        right_rows,
        join_type,
        on_condition,
        &self.expression_evaluator,
    )
}
```

### Recommended Fix

Update execute_join to extract and pass alias information:

```rust
fn execute_join(
    &mut self,
    left: &ExecutionPlan,
    right: &ExecutionPlan,
    left_alias: &Option<String>,      // ADD THESE
    right_alias: &Option<String>,     // ADD THESE
    join_type: &crate::compiler::JoinType,
    on_condition: &CompiledExpression,
) -> Result<Vec<ResultRow>> {
    let left_rows = self.execute_plan(left)?;
    let right_rows = self.execute_plan(right)?;

    // PASS ALIAS CONTEXT TO JOINEXECUTOR
    JoinExecutor::execute_join_with_alias(
        left_rows,
        right_rows,
        join_type,
        on_condition,
        &self.expression_evaluator,
        left_alias,
        right_alias,
    )
}
```

And update the plan executor's match statement:

```rust
ExecutionPlan::Join { left, right, left_alias, right_alias, join_type, on_condition } => {
    self.execute_join(left, right, left_alias, right_alias, join_type, on_condition)
},
```

---

## Gap 4: Qualified Column Resolution

### The Problem

When a WHERE clause contains qualified columns like `p.subject_id`, the executor doesn't properly resolve which table the column came from.

### Current Code (WORKS BY ACCIDENT)

**File:** `/src/compiler/mod.rs` (lines 234-239)

```rust
Column {
    table: Option<String>,   // Can be None, but contains table.name when parsed
    name: String,
    value_type: ValueType,
}
```

The table information is stored but rarely used during evaluation.

### The Issue

When combining rows, all columns are in a flat HashMap with their original names. There's no tracking of which columns came from which table, so qualified lookups fail.

### Example

```
Query: SELECT * FROM patients p WHERE p.age > 40

Compiled WHERE condition:
  Column { table: Some("p"), name: "age", ... } > Literal(40)

After JOIN, combined row has:
  { "age": 30, ... }

But the evaluator needs to look for "p.age", not just "age"!
```

### Recommended Fix

**Option A: Prefix all columns during JOIN**

When combining rows with aliases, prefix columns:
```rust
{
    "p.id": "p1",
    "p.name": "Alice", 
    "p.age": 30,
    "a.id": "a1",
    "a.hadm_id": 101,
}
```

Then qualified column lookups work naturally: `p.age` finds 30.

**Option B: Add table context to ResultRow**

```rust
pub struct ResultRow {
    pub columns: HashMap<String, Value>,
    pub table_aliases: HashMap<String, Vec<String>>,  // Which columns came from which table
}
```

Then during evaluation:
```rust
fn evaluate_column(&self, table: Option<&str>, name: &str, row: &ResultRow) -> Result<Value> {
    if let Some(table_name) = table {
        // Lookup in specific table's columns
        let prefixed = format!("{}.{}", table_name, name);
        row.columns.get(&prefixed).cloned().ok_or(...)?
    } else {
        // Ambiguous, try unprefixed first
        row.columns.get(name).cloned().or_else(...)
    }
}
```

**Recommendation:** Use Option A (prefixing) as it's simpler and more compatible with standard SQL behavior.

---

## Implementation Timeline

### Phase 1: Critical (2-3 days)
- Add `alias: Option<String>` to ExecutionPlan::Scan
- Add `alias: Option<String>` to ExecutionPlan::Join
- Implement column prefixing in combine_rows()
- Update executor to pass alias context
- Add test for overlapping column names

### Phase 2: Important (3-4 days)  
- Update expression evaluator for qualified lookups
- Add test cases for qualified WHERE clauses
- Fix edge cases in multiple JOIN scenarios

### Phase 3: Enhancement (1 week)
- Add sort-merge join
- Implement join cardinality estimation
- Cost-based join ordering for multiple JOINs

---

## Test Case: Overlapping Column Names

Create this test to verify the fix:

```rust
#[test]
fn test_join_overlapping_column_names() {
    // Setup tables where both have 'name' column
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT p.name, a.name FROM patients p \
                 INNER JOIN admissions a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 3 rows
    assert_eq!(result.rows.len(), 3);

    // Verify BOTH columns are present with correct values
    for row in &result.rows {
        // Should have BOTH p.name AND a.name
        let p_name = row.columns.get("p.name");
        let a_name = row.columns.get("a.name");
        
        // Both should be present
        assert!(p_name.is_some(), "Missing p.name");
        assert!(a_name.is_some(), "Missing a.name");
        
        // p.name should be a patient name
        match p_name {
            Some(Value::String(s)) => {
                assert!(["Alice", "Bob", "Charlie"].contains(&s.as_str()));
            }
            _ => panic!("Expected string for p.name"),
        }
        
        // a.name should be admission type
        match a_name {
            Some(Value::String(s)) => {
                assert!(["EMERGENCY", "ELECTIVE", "URGENT"].contains(&s.as_str()));
            }
            _ => panic!("Expected string for a.name"),
        }
    }
}
```

---

## Summary

The HyperQL JOIN implementation is 85% complete but has a critical gap in column prefixing. The fixes are straightforward:

1. **Add alias fields** to ExecutionPlan (30 minutes)
2. **Propagate aliases** through compiler (30 minutes)
3. **Implement prefixing** in combine_rows() (1-2 hours)
4. **Update executor** to pass context (30 minutes)
5. **Add tests** (1 hour)
6. **Fix edge cases** (2-4 hours)

Total estimate: 6-9 hours for full production readiness.
