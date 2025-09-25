//! # HyperQL Query Compiler
//!
//! This module implements the comprehensive query compilation engine that transforms HyperQL
//! Abstract Syntax Trees (ASTs) into optimized execution plans. Supports SELECT, INSERT,
//! UPDATE, DELETE statements with aggregate functions, GROUP BY, HAVING clauses.

use crate::ast::*;
use crate::error::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main compiler interface
pub struct Compiler {
    /// Type checker for semantic validation
    type_checker: TypeChecker,
    /// Plan generator
    plan_generator: PlanGenerator,
}

/// Compiled query plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledQuery {
    /// Execution plan
    pub plan: ExecutionPlan,
    /// Query metadata
    pub metadata: QueryMetadata,
    /// Estimated execution cost
    pub estimated_cost: ExecutionCost,
}

/// Execution plan structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPlan {
    /// Sequential scan of entities
    Scan {
        table: String,
        filter: Option<CompiledExpression>,
        projection: Vec<CompiledProjection>,
    },
    /// Filtered scan with WHERE clause
    Filter {
        input: Box<ExecutionPlan>,
        predicate: CompiledExpression,
    },
    /// Projection of columns
    Project {
        input: Box<ExecutionPlan>,
        expressions: Vec<CompiledProjection>,
    },
    /// Group by aggregation
    GroupBy {
        input: Box<ExecutionPlan>,
        group_expressions: Vec<CompiledExpression>,
        aggregate_expressions: Vec<CompiledProjection>,
    },
    /// Having filter for grouped results
    Having {
        input: Box<ExecutionPlan>,
        predicate: CompiledExpression,
    },
    /// Sort operation
    Sort {
        input: Box<ExecutionPlan>,
        sort_keys: Vec<CompiledSortKey>,
    },
    /// Limit operation
    Limit {
        input: Box<ExecutionPlan>,
        count: u64,
        offset: Option<u64>,
    },
    /// INSERT operation
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<CompiledExpression>>,
    },
    /// UPDATE operation
    Update {
        table: String,
        assignments: Vec<CompiledAssignment>,
        filter: Option<CompiledExpression>,
    },
    /// DELETE operation
    Delete {
        table: String,
        filter: Option<CompiledExpression>,
    },
    /// TRAVERSE operation for graph pattern matching
    Traverse {
        patterns: Vec<CompiledTraversePattern>,
    },
}

/// Compiled expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompiledExpression {
    /// Literal value
    Literal(Value),
    /// Column reference
    Column {
        table: Option<String>,
        name: String,
        value_type: ValueType,
    },
    /// Binary operation
    Binary {
        left: Box<CompiledExpression>,
        op: BinaryOperator,
        right: Box<CompiledExpression>,
        result_type: ValueType,
    },
    /// Unary operation
    Unary {
        op: UnaryOperator,
        expr: Box<CompiledExpression>,
        result_type: ValueType,
    },
    /// Function call
    Function {
        name: String,
        args: Vec<CompiledExpression>,
        result_type: ValueType,
    },
}

/// Compiled projection item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledProjection {
    pub expression: CompiledExpression,
    pub alias: Option<String>,
    pub output_name: String,
}

/// Compiled sort key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSortKey {
    pub expression: CompiledExpression,
    pub direction: OrderDirection,
}

/// Compiled assignment for UPDATE statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledAssignment {
    pub column: String,
    pub value: CompiledExpression,
}

/// Compiled traverse pattern for graph operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledTraversePattern {
    pub start_node: CompiledNodePattern,
    pub relationship: CompiledRelationshipPattern,
    pub end_node: CompiledNodePattern,
}

/// Compiled node pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledNodePattern {
    pub variable: Option<String>,
    pub label: Option<String>,
    pub properties: Option<CompiledExpression>,
}

/// Compiled relationship pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledRelationshipPattern {
    pub variable: Option<String>,
    pub rel_type: Option<String>,
    pub direction: crate::ast::RelationshipDirection,
    pub variable_length: Option<crate::ast::VariableLength>,
    pub optional: bool,
    pub properties: Option<CompiledExpression>,
}

/// Value types in the type system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    String,
    EntityId,
    Position,
    Distance,
    Vector,
    List(Box<ValueType>),
    Map(Box<ValueType>),
    Timestamp,
    Duration,
}

/// Query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub tables_accessed: Vec<String>,
    pub columns_accessed: Vec<String>,
    pub functions_used: Vec<String>,
    pub requires_spatial_index: bool,
    pub requires_vector_index: bool,
}

/// Execution cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCost {
    pub estimated_rows: u64,
    pub estimated_cpu_cost: f64,
    pub estimated_memory_mb: f64,
    pub estimated_io_ops: u64,
}

/// Type checker for semantic analysis
pub struct TypeChecker {
    /// Known entity schemas
    schemas: HashMap<String, EntitySchema>,
}

/// Entity schema definition
#[derive(Debug, Clone)]
pub struct EntitySchema {
    pub name: String,
    pub properties: HashMap<String, ValueType>,
}

/// Plan generator
pub struct PlanGenerator {
    /// Cost estimator
    cost_estimator: CostEstimator,
}

/// Cost estimation component
pub struct CostEstimator;

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            type_checker: TypeChecker::new(),
            plan_generator: PlanGenerator::new(),
        }
    }

    /// Compile a statement into an execution plan
    pub fn compile(&self, statement: Statement) -> Result<CompiledQuery> {
        match statement {
            Statement::Select(select) => self.compile_select(select),
            Statement::Insert(insert) => self.compile_insert(insert),
            Statement::Update(update) => self.compile_update(update),
            Statement::Delete(delete) => self.compile_delete(delete),
        }
    }

    /// Compile a SELECT statement
    fn compile_select(&self, select: SelectStatement) -> Result<CompiledQuery> {
        // Start with base table scan
        let mut plan = self.create_base_scan(&select)?;

        // Apply TRAVERSE clause if present
        if let Some(traverse_clause) = select.traverse_clause {
            let compiled_patterns = self.compile_traverse_patterns(&traverse_clause.patterns)?;
            let traverse_plan = ExecutionPlan::Traverse {
                patterns: compiled_patterns,
            };
            // If we have a base scan, chain the traverse after it
            if let ExecutionPlan::Scan { .. } = plan {
                // For now, replace the scan with traverse
                // TODO: Properly chain scan -> traverse in execution flow
                plan = traverse_plan;
            } else {
                plan = traverse_plan;
            }
        }

        // Apply WHERE clause if present
        if let Some(where_expr) = select.where_clause {
            let compiled_predicate = self.type_checker.compile_expression(where_expr)?;
            plan = ExecutionPlan::Filter {
                input: Box::new(plan),
                predicate: compiled_predicate,
            };
        }

        // Apply GROUP BY if present
        if !select.group_by.is_empty() {
            let group_expressions = select.group_by.iter()
                .map(|expr| self.type_checker.compile_expression(expr.clone()))
                .collect::<Result<Vec<_>>>()?;

            let projections = self.compile_select_list(&select.select_list)?;

            plan = ExecutionPlan::GroupBy {
                input: Box::new(plan),
                group_expressions,
                aggregate_expressions: projections,
            };
        } else {
            // Apply projection for non-grouped queries
            let projections = self.compile_select_list(&select.select_list)?;
            if !projections.is_empty() {
                plan = ExecutionPlan::Project {
                    input: Box::new(plan),
                    expressions: projections,
                };
            }
        }

        // Apply HAVING clause if present
        if let Some(having_expr) = select.having {
            let compiled_having = self.type_checker.compile_expression(having_expr)?;
            plan = ExecutionPlan::Having {
                input: Box::new(plan),
                predicate: compiled_having,
            };
        }

        // Apply ORDER BY if present
        if !select.order_by.is_empty() {
            let sort_keys = self.compile_order_by(&select.order_by)?;
            plan = ExecutionPlan::Sort {
                input: Box::new(plan),
                sort_keys,
            };
        }

        // Apply LIMIT if present
        if let Some(limit) = select.limit {
            plan = ExecutionPlan::Limit {
                input: Box::new(plan),
                count: limit,
                offset: select.offset,
            };
        }

        // Generate metadata and cost estimate
        let metadata = self.generate_metadata(&plan);
        let estimated_cost = self.plan_generator.cost_estimator.estimate_cost(&plan);

        Ok(CompiledQuery {
            plan,
            metadata,
            estimated_cost,
        })
    }

    /// Create base table scan
    fn create_base_scan(&self, select: &SelectStatement) -> Result<ExecutionPlan> {
        let table_name = match &select.from {
            Some(FromClause::Table { name, .. }) => name.clone(),
            Some(FromClause::Subquery { .. }) => {
                return Err(HyperQLError::SemanticError {
                    message: "Subqueries are not yet supported".to_string(),
                    context: vec!["compiler".to_string()],
                });
            }
            None => {
                return Err(HyperQLError::SemanticError {
                    message: "FROM clause is required".to_string(),
                    context: vec!["compiler".to_string()],
                });
            }
        };

        Ok(ExecutionPlan::Scan {
            table: table_name,
            filter: None,
            projection: vec![],
        })
    }

    /// Compile SELECT list
    fn compile_select_list(&self, select_list: &[SelectItem]) -> Result<Vec<CompiledProjection>> {
        let mut projections = Vec::new();

        for item in select_list {
            match item {
                SelectItem::Wildcard => {
                    // TODO: Expand wildcard based on table schema
                    projections.push(CompiledProjection {
                        expression: CompiledExpression::Column {
                            table: None,
                            name: "*".to_string(),
                            value_type: ValueType::String, // Placeholder
                        },
                        alias: None,
                        output_name: "*".to_string(),
                    });
                }
                SelectItem::Expression { expr, alias } => {
                    let compiled_expr = self.type_checker.compile_expression(expr.clone())?;
                    let output_name = alias.clone().unwrap_or_else(|| {
                        self.generate_expression_name(expr)
                    });

                    projections.push(CompiledProjection {
                        expression: compiled_expr,
                        alias: alias.clone(),
                        output_name,
                    });
                }
            }
        }

        Ok(projections)
    }

    /// Compile ORDER BY clause
    fn compile_order_by(&self, order_by: &[OrderByItem]) -> Result<Vec<CompiledSortKey>> {
        let mut sort_keys = Vec::new();

        for item in order_by {
            let compiled_expr = self.type_checker.compile_expression(item.expr.clone())?;
            sort_keys.push(CompiledSortKey {
                expression: compiled_expr,
                direction: item.direction.clone(),
            });
        }

        Ok(sort_keys)
    }

    /// Generate expression name for output
    fn generate_expression_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Column(col_ref) => col_ref.name.clone(),
            Expression::Function { name, .. } => name.clone(),
            Expression::Literal(literal) => format!("{:?}", literal),
            _ => "expr".to_string(),
        }
    }

    /// Compile traverse patterns
    fn compile_traverse_patterns(&self, patterns: &[crate::ast::TraversePattern]) -> Result<Vec<CompiledTraversePattern>> {
        let mut compiled_patterns = Vec::new();

        for pattern in patterns {
            let compiled_start_node = CompiledNodePattern {
                variable: pattern.start_node.variable.clone(),
                label: pattern.start_node.label.clone(),
                properties: if let Some(props) = &pattern.start_node.properties {
                    Some(self.type_checker.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_end_node = CompiledNodePattern {
                variable: pattern.end_node.variable.clone(),
                label: pattern.end_node.label.clone(),
                properties: if let Some(props) = &pattern.end_node.properties {
                    Some(self.type_checker.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_relationship = CompiledRelationshipPattern {
                variable: pattern.relationship.variable.clone(),
                rel_type: pattern.relationship.rel_type.clone(),
                direction: pattern.relationship.direction.clone(),
                variable_length: pattern.relationship.variable_length.clone(),
                optional: pattern.relationship.optional,
                properties: if let Some(props) = &pattern.relationship.properties {
                    Some(self.type_checker.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            compiled_patterns.push(CompiledTraversePattern {
                start_node: compiled_start_node,
                relationship: compiled_relationship,
                end_node: compiled_end_node,
            });
        }

        Ok(compiled_patterns)
    }

    /// Generate query metadata
    fn generate_metadata(&self, plan: &ExecutionPlan) -> QueryMetadata {
        let mut metadata = QueryMetadata {
            tables_accessed: Vec::new(),
            columns_accessed: Vec::new(),
            functions_used: Vec::new(),
            requires_spatial_index: false,
            requires_vector_index: false,
        };

        self.collect_metadata(plan, &mut metadata);
        metadata
    }

    /// Compile an INSERT statement
    fn compile_insert(&self, insert: InsertStatement) -> Result<CompiledQuery> {
        let mut compiled_values = Vec::new();

        for value_row in insert.values {
            let mut compiled_row = Vec::new();
            for value_expr in value_row {
                let compiled_expr = self.type_checker.compile_expression(value_expr)?;
                compiled_row.push(compiled_expr);
            }
            compiled_values.push(compiled_row);
        }

        let plan = ExecutionPlan::Insert {
            table: insert.table.clone(),
            columns: insert.columns,
            values: compiled_values,
        };

        // Generate metadata and cost estimate
        let metadata = QueryMetadata {
            tables_accessed: vec![insert.table],
            columns_accessed: vec![],
            functions_used: vec![],
            requires_spatial_index: false,
            requires_vector_index: false,
        };

        let estimated_cost = ExecutionCost {
            estimated_rows: 1, // INSERT typically affects one or few rows
            estimated_cpu_cost: 0.5,
            estimated_memory_mb: 1.0,
            estimated_io_ops: 1,
        };

        Ok(CompiledQuery {
            plan,
            metadata,
            estimated_cost,
        })
    }

    /// Compile an UPDATE statement
    fn compile_update(&self, update: UpdateStatement) -> Result<CompiledQuery> {
        let mut compiled_assignments = Vec::new();

        for assignment in update.assignments {
            let compiled_value = self.type_checker.compile_expression(assignment.value)?;
            compiled_assignments.push(CompiledAssignment {
                column: assignment.column,
                value: compiled_value,
            });
        }

        let compiled_filter = if let Some(where_expr) = update.where_clause {
            Some(self.type_checker.compile_expression(where_expr)?)
        } else {
            None
        };

        let plan = ExecutionPlan::Update {
            table: update.table.clone(),
            assignments: compiled_assignments,
            filter: compiled_filter,
        };

        // Generate metadata and cost estimate
        let metadata = QueryMetadata {
            tables_accessed: vec![update.table],
            columns_accessed: vec![],
            functions_used: vec![],
            requires_spatial_index: false,
            requires_vector_index: false,
        };

        let estimated_cost = ExecutionCost {
            estimated_rows: 100, // UPDATE may affect multiple rows
            estimated_cpu_cost: 2.0,
            estimated_memory_mb: 5.0,
            estimated_io_ops: 50,
        };

        Ok(CompiledQuery {
            plan,
            metadata,
            estimated_cost,
        })
    }

    /// Compile a DELETE statement
    fn compile_delete(&self, delete: DeleteStatement) -> Result<CompiledQuery> {
        let compiled_filter = if let Some(where_expr) = delete.where_clause {
            Some(self.type_checker.compile_expression(where_expr)?)
        } else {
            None
        };

        let plan = ExecutionPlan::Delete {
            table: delete.table.clone(),
            filter: compiled_filter,
        };

        // Generate metadata and cost estimate
        let metadata = QueryMetadata {
            tables_accessed: vec![delete.table],
            columns_accessed: vec![],
            functions_used: vec![],
            requires_spatial_index: false,
            requires_vector_index: false,
        };

        let estimated_cost = ExecutionCost {
            estimated_rows: 100, // DELETE may affect multiple rows
            estimated_cpu_cost: 1.5,
            estimated_memory_mb: 3.0,
            estimated_io_ops: 30,
        };

        Ok(CompiledQuery {
            plan,
            metadata,
            estimated_cost,
        })
    }

    /// Recursively collect metadata from execution plan
    fn collect_metadata(&self, plan: &ExecutionPlan, metadata: &mut QueryMetadata) {
        match plan {
            ExecutionPlan::Scan { table, .. } => {
                if !metadata.tables_accessed.contains(table) {
                    metadata.tables_accessed.push(table.clone());
                }
            }
            ExecutionPlan::Filter { input, predicate } => {
                self.collect_metadata(input, metadata);
                self.collect_expression_metadata(predicate, metadata);
            }
            ExecutionPlan::Project { input, expressions } => {
                self.collect_metadata(input, metadata);
                for proj in expressions {
                    self.collect_expression_metadata(&proj.expression, metadata);
                }
            }
            ExecutionPlan::GroupBy { input, group_expressions, aggregate_expressions } => {
                self.collect_metadata(input, metadata);
                for expr in group_expressions {
                    self.collect_expression_metadata(expr, metadata);
                }
                for proj in aggregate_expressions {
                    self.collect_expression_metadata(&proj.expression, metadata);
                }
            }
            ExecutionPlan::Having { input, predicate } => {
                self.collect_metadata(input, metadata);
                self.collect_expression_metadata(predicate, metadata);
            }
            ExecutionPlan::Sort { input, sort_keys } => {
                self.collect_metadata(input, metadata);
                for key in sort_keys {
                    self.collect_expression_metadata(&key.expression, metadata);
                }
            }
            ExecutionPlan::Limit { input, .. } => {
                self.collect_metadata(input, metadata);
            }
            ExecutionPlan::Insert { table, values, .. } => {
                if !metadata.tables_accessed.contains(table) {
                    metadata.tables_accessed.push(table.clone());
                }
                for row in values {
                    for expr in row {
                        self.collect_expression_metadata(expr, metadata);
                    }
                }
            }
            ExecutionPlan::Update { table, assignments, filter } => {
                if !metadata.tables_accessed.contains(table) {
                    metadata.tables_accessed.push(table.clone());
                }
                for assignment in assignments {
                    self.collect_expression_metadata(&assignment.value, metadata);
                }
                if let Some(filter_expr) = filter {
                    self.collect_expression_metadata(filter_expr, metadata);
                }
            }
            ExecutionPlan::Delete { table, filter } => {
                if !metadata.tables_accessed.contains(table) {
                    metadata.tables_accessed.push(table.clone());
                }
                if let Some(filter_expr) = filter {
                    self.collect_expression_metadata(filter_expr, metadata);
                }
            }
            ExecutionPlan::Traverse { patterns } => {
                // Mark that this query uses graph traversal
                for pattern in patterns {
                    if let Some(label) = &pattern.start_node.label {
                        if !metadata.tables_accessed.contains(label) {
                            metadata.tables_accessed.push(label.clone());
                        }
                    }
                    if let Some(label) = &pattern.end_node.label {
                        if !metadata.tables_accessed.contains(label) {
                            metadata.tables_accessed.push(label.clone());
                        }
                    }
                    if let Some(props) = &pattern.start_node.properties {
                        self.collect_expression_metadata(props, metadata);
                    }
                    if let Some(props) = &pattern.end_node.properties {
                        self.collect_expression_metadata(props, metadata);
                    }
                    if let Some(props) = &pattern.relationship.properties {
                        self.collect_expression_metadata(props, metadata);
                    }
                }
            }
        }
    }

    /// Collect metadata from compiled expressions
    fn collect_expression_metadata(&self, expr: &CompiledExpression, metadata: &mut QueryMetadata) {
        match expr {
            CompiledExpression::Column { name, .. } => {
                if !metadata.columns_accessed.contains(name) {
                    metadata.columns_accessed.push(name.clone());
                }
            }
            CompiledExpression::Function { name, args, .. } => {
                if !metadata.functions_used.contains(name) {
                    metadata.functions_used.push(name.clone());
                }
                for arg in args {
                    self.collect_expression_metadata(arg, metadata);
                }
            }
            CompiledExpression::Binary { left, right, .. } => {
                self.collect_expression_metadata(left, metadata);
                self.collect_expression_metadata(right, metadata);
            }
            CompiledExpression::Unary { expr, .. } => {
                self.collect_expression_metadata(expr, metadata);
            }
            CompiledExpression::Literal(_) => {
                // No metadata to collect from literals
            }
        }
    }
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Compile an expression with type checking
    pub fn compile_expression(&self, expr: Expression) -> Result<CompiledExpression> {
        match expr {
            Expression::Literal(literal) => {
                let (value, value_type) = self.compile_literal(literal)?;
                Ok(CompiledExpression::Literal(value))
            }
            Expression::Column(col_ref) => {
                let value_type = self.infer_column_type(&col_ref)?;
                Ok(CompiledExpression::Column {
                    table: col_ref.table,
                    name: col_ref.name,
                    value_type,
                })
            }
            Expression::Binary { left, op, right } => {
                let compiled_left = self.compile_expression(*left)?;
                let compiled_right = self.compile_expression(*right)?;
                let result_type = self.infer_binary_result_type(&compiled_left, &op, &compiled_right)?;

                Ok(CompiledExpression::Binary {
                    left: Box::new(compiled_left),
                    op,
                    right: Box::new(compiled_right),
                    result_type,
                })
            }
            Expression::Unary { op, expr } => {
                let compiled_expr = self.compile_expression(*expr)?;
                let result_type = self.infer_unary_result_type(&op, &compiled_expr)?;

                Ok(CompiledExpression::Unary {
                    op,
                    expr: Box::new(compiled_expr),
                    result_type,
                })
            }
            Expression::Function { name, args } => {
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.compile_expression(arg)?);
                }
                let result_type = self.infer_function_result_type(&name, &compiled_args)?;

                Ok(CompiledExpression::Function {
                    name,
                    args: compiled_args,
                    result_type,
                })
            }
            Expression::Geometric(geom_expr) => {
                // TODO: Implement geometric expression compilation
                Err(HyperQLError::ExecutionError {
                    message: format!("Geometric expression compilation not yet implemented: {:?}", geom_expr),
                    operation: "compile_geometric_expression".to_string(),
                    entity_context: Some("geometric_compilation".to_string()),
                })
            }
            Expression::Vector(vector_expr) => {
                // TODO: Implement vector expression compilation
                Err(HyperQLError::ExecutionError {
                    message: format!("Vector expression compilation not yet implemented: {:?}", vector_expr),
                    operation: "compile_vector_expression".to_string(),
                    entity_context: Some("vector_compilation".to_string()),
                })
            }
        }
    }

    /// Compile a literal value
    fn compile_literal(&self, literal: Literal) -> Result<(Value, ValueType)> {
        match literal {
            Literal::Null => Ok((Value::Null, ValueType::Null)),
            Literal::Bool(b) => Ok((Value::Bool(b), ValueType::Bool)),
            Literal::Int(i) => Ok((Value::Int(i), ValueType::Int)),
            Literal::Float(f) => Ok((Value::Float(f), ValueType::Float)),
            Literal::String(s) => Ok((Value::String(s), ValueType::String)),
            Literal::EntityId(id) => Ok((Value::EntityId(id), ValueType::EntityId)),
        }
    }

    /// Infer column type (simplified for now)
    fn infer_column_type(&self, col_ref: &ColumnRef) -> Result<ValueType> {
        // TODO: Look up actual schema
        // For now, default to string type
        Ok(ValueType::String)
    }

    /// Infer binary operation result type
    fn infer_binary_result_type(&self, left: &CompiledExpression, op: &BinaryOperator, right: &CompiledExpression) -> Result<ValueType> {
        match op {
            BinaryOperator::Equal | BinaryOperator::NotEqual |
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual |
            BinaryOperator::And | BinaryOperator::Or => Ok(ValueType::Bool),

            BinaryOperator::Add | BinaryOperator::Subtract |
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                // TODO: Implement proper numeric type promotion
                Ok(ValueType::Float)
            }

            BinaryOperator::Like | BinaryOperator::NotLike => Ok(ValueType::Bool),
            BinaryOperator::In | BinaryOperator::NotIn => Ok(ValueType::Bool),
        }
    }

    /// Infer unary operation result type
    fn infer_unary_result_type(&self, op: &UnaryOperator, expr: &CompiledExpression) -> Result<ValueType> {
        match op {
            UnaryOperator::Not => Ok(ValueType::Bool),
            UnaryOperator::Minus | UnaryOperator::Plus => {
                // TODO: Preserve input numeric type
                Ok(ValueType::Float)
            }
        }
    }

    /// Infer function result type
    fn infer_function_result_type(&self, name: &str, _args: &[CompiledExpression]) -> Result<ValueType> {
        match name.to_uppercase().as_str() {
            // Aggregate functions
            "COUNT" => Ok(ValueType::Int),      // COUNT always returns integer
            "SUM" => Ok(ValueType::Float),      // SUM can return float for safety
            "AVG" => Ok(ValueType::Float),      // AVG always returns float
            "MIN" | "MAX" => Ok(ValueType::Float), // MIN/MAX can return various types, default to float

            // String functions
            "UPPER" | "LOWER" | "TRIM" => Ok(ValueType::String),
            "LENGTH" => Ok(ValueType::Int),

            // Mathematical functions
            "ABS" | "ROUND" | "FLOOR" | "CEIL" => Ok(ValueType::Float),
            "SQRT" | "POW" | "EXP" | "LOG" => Ok(ValueType::Float),

            // Date/Time functions
            "NOW" | "CURRENT_TIMESTAMP" => Ok(ValueType::Timestamp),

            _ => Ok(ValueType::String), // Default for unknown functions
        }
    }
}

impl PlanGenerator {
    /// Create a new plan generator
    pub fn new() -> Self {
        Self {
            cost_estimator: CostEstimator,
        }
    }
}

impl CostEstimator {
    /// Estimate execution cost for a plan
    pub fn estimate_cost(&self, plan: &ExecutionPlan) -> ExecutionCost {
        match plan {
            ExecutionPlan::Scan { .. } => ExecutionCost {
                estimated_rows: 1000, // TODO: Use actual statistics
                estimated_cpu_cost: 1.0,
                estimated_memory_mb: 10.0,
                estimated_io_ops: 100,
            },
            ExecutionPlan::Filter { input, .. } => {
                let input_cost = self.estimate_cost(input);
                ExecutionCost {
                    estimated_rows: input_cost.estimated_rows / 2, // Assume 50% selectivity
                    estimated_cpu_cost: input_cost.estimated_cpu_cost + 0.5,
                    estimated_memory_mb: input_cost.estimated_memory_mb,
                    estimated_io_ops: input_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::Project { input, .. } => {
                let input_cost = self.estimate_cost(input);
                ExecutionCost {
                    estimated_rows: input_cost.estimated_rows,
                    estimated_cpu_cost: input_cost.estimated_cpu_cost + 0.1,
                    estimated_memory_mb: input_cost.estimated_memory_mb,
                    estimated_io_ops: input_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::GroupBy { input, group_expressions, .. } => {
                let input_cost = self.estimate_cost(input);
                // Grouping typically reduces row count but increases CPU and memory usage
                let estimated_groups = (input_cost.estimated_rows / 10).max(1);
                ExecutionCost {
                    estimated_rows: estimated_groups,
                    estimated_cpu_cost: input_cost.estimated_cpu_cost + (input_cost.estimated_rows as f64 * 0.01),
                    estimated_memory_mb: input_cost.estimated_memory_mb + (group_expressions.len() as f64 * 5.0),
                    estimated_io_ops: input_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::Having { input, .. } => {
                let input_cost = self.estimate_cost(input);
                ExecutionCost {
                    estimated_rows: input_cost.estimated_rows / 2, // Assume 50% selectivity for HAVING
                    estimated_cpu_cost: input_cost.estimated_cpu_cost + 0.3,
                    estimated_memory_mb: input_cost.estimated_memory_mb,
                    estimated_io_ops: input_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::Sort { input, .. } => {
                let input_cost = self.estimate_cost(input);
                ExecutionCost {
                    estimated_rows: input_cost.estimated_rows,
                    estimated_cpu_cost: input_cost.estimated_cpu_cost + (input_cost.estimated_rows as f64 * 0.001),
                    estimated_memory_mb: input_cost.estimated_memory_mb * 2.0, // Sorting requires extra memory
                    estimated_io_ops: input_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::Limit { input, count, .. } => {
                let input_cost = self.estimate_cost(input);
                ExecutionCost {
                    estimated_rows: (*count).min(input_cost.estimated_rows),
                    estimated_cpu_cost: input_cost.estimated_cpu_cost,
                    estimated_memory_mb: input_cost.estimated_memory_mb,
                    estimated_io_ops: input_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::Insert { values, .. } => {
                ExecutionCost {
                    estimated_rows: values.len() as u64,
                    estimated_cpu_cost: values.len() as f64 * 0.1,
                    estimated_memory_mb: 2.0,
                    estimated_io_ops: values.len() as u64,
                }
            }
            ExecutionPlan::Update { .. } => {
                ExecutionCost {
                    estimated_rows: 100, // Estimate number of rows affected
                    estimated_cpu_cost: 2.0,
                    estimated_memory_mb: 5.0,
                    estimated_io_ops: 50,
                }
            }
            ExecutionPlan::Delete { .. } => {
                ExecutionCost {
                    estimated_rows: 100, // Estimate number of rows affected
                    estimated_cpu_cost: 1.5,
                    estimated_memory_mb: 3.0,
                    estimated_io_ops: 30,
                }
            }
            ExecutionPlan::Traverse { patterns } => {
                // Graph traversal costs depend on pattern complexity
                let pattern_count = patterns.len() as f64;
                let estimated_traversal_cost = pattern_count * 10.0; // Base cost per pattern

                ExecutionCost {
                    estimated_rows: 1000, // Estimate typical traversal result size
                    estimated_cpu_cost: estimated_traversal_cost,
                    estimated_memory_mb: pattern_count * 20.0, // Memory for graph traversal
                    estimated_io_ops: (pattern_count * 100.0) as u64, // Graph I/O operations
                }
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PlanGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_statement;

    #[test]
    fn test_compile_simple_select() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM entities";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match *input {
                    ExecutionPlan::Scan { table, .. } => {
                        assert_eq!(table, "entities");
                    }
                    _ => panic!("Expected scan as input to project"),
                }
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_compile_select_with_where() {
        let compiler = Compiler::new();
        let query = "SELECT name FROM entities WHERE name = 'Alice'";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should have Project -> Filter -> Scan structure
        match compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match *input {
                    ExecutionPlan::Filter { input, .. } => {
                        match *input {
                            ExecutionPlan::Scan { table, .. } => {
                                assert_eq!(table, "entities");
                            }
                            _ => panic!("Expected scan as input to filter"),
                        }
                    }
                    _ => panic!("Expected filter as input to project"),
                }
            }
            _ => panic!("Expected project plan"),
        }
    }

    #[test]
    fn test_compile_select_with_order_by_limit() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM entities ORDER BY name LIMIT 10";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should have Limit -> Sort -> Project -> Scan structure
        match compiled.plan {
            ExecutionPlan::Limit { input, count, .. } => {
                assert_eq!(count, 10);
                match *input {
                    ExecutionPlan::Sort { input, .. } => {
                        match *input {
                            ExecutionPlan::Project { input, .. } => {
                                match *input {
                                    ExecutionPlan::Scan { .. } => {
                                        // Success
                                    }
                                    _ => panic!("Expected scan"),
                                }
                            }
                            _ => panic!("Expected project"),
                        }
                    }
                    _ => panic!("Expected sort"),
                }
            }
            _ => panic!("Expected limit plan"),
        }
    }

    #[test]
    fn test_compile_insert_statement() {
        let compiler = Compiler::new();
        let query = "INSERT INTO users (name, age) VALUES ('Alice', 30)";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match compiled.plan {
            ExecutionPlan::Insert { table, columns, values } => {
                assert_eq!(table, "users");
                assert_eq!(columns, vec!["name", "age"]);
                assert_eq!(values.len(), 1);
                assert_eq!(values[0].len(), 2);
            }
            _ => panic!("Expected INSERT plan"),
        }
    }

    #[test]
    fn test_compile_update_statement() {
        let compiler = Compiler::new();
        let query = "UPDATE users SET age = 31 WHERE name = 'Alice'";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match compiled.plan {
            ExecutionPlan::Update { table, assignments, filter } => {
                assert_eq!(table, "users");
                assert_eq!(assignments.len(), 1);
                assert_eq!(assignments[0].column, "age");
                assert!(filter.is_some());
            }
            _ => panic!("Expected UPDATE plan"),
        }
    }

    #[test]
    fn test_compile_delete_statement() {
        let compiler = Compiler::new();
        let query = "DELETE FROM users WHERE age < 18";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match compiled.plan {
            ExecutionPlan::Delete { table, filter } => {
                assert_eq!(table, "users");
                assert!(filter.is_some());
            }
            _ => panic!("Expected DELETE plan"),
        }
    }

    #[test]
    fn test_compile_group_by_statement() {
        let compiler = Compiler::new();
        let query = "SELECT category, COUNT(*) FROM products GROUP BY category";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match compiled.plan {
            ExecutionPlan::GroupBy { group_expressions, aggregate_expressions, .. } => {
                assert_eq!(group_expressions.len(), 1);
                assert_eq!(aggregate_expressions.len(), 2); // category and COUNT(*)
            }
            _ => panic!("Expected GROUP BY plan"),
        }
    }

    #[test]
    fn test_compile_having_statement() {
        let compiler = Compiler::new();
        let query = "SELECT category, COUNT(*) FROM products GROUP BY category HAVING COUNT(*) > 5";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should have Having -> GroupBy -> Scan structure
        match compiled.plan {
            ExecutionPlan::Having { input, .. } => {
                match *input {
                    ExecutionPlan::GroupBy { .. } => {
                        // Success - correct structure
                    }
                    _ => panic!("Expected GROUP BY as input to HAVING"),
                }
            }
            _ => panic!("Expected HAVING plan"),
        }
    }

    #[test]
    fn test_compile_traverse_statement() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM users TRAVERSE (a:User)-[r:follows]->(b:User)";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Check for traverse plan (as input to project)
        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Traverse { patterns } => {
                        assert_eq!(patterns.len(), 1);
                        let pattern = &patterns[0];
                        assert_eq!(pattern.start_node.variable, Some("a".to_string()));
                        assert_eq!(pattern.start_node.label, Some("User".to_string()));
                        assert_eq!(pattern.end_node.variable, Some("b".to_string()));
                        assert_eq!(pattern.end_node.label, Some("User".to_string()));
                        assert_eq!(pattern.relationship.variable, Some("r".to_string()));
                        assert_eq!(pattern.relationship.rel_type, Some("follows".to_string()));
                    }
                    other => panic!("Expected TRAVERSE as input to project, got: {:?}", other),
                }
            }
            other => panic!("Expected PROJECT plan with TRAVERSE input, got: {:?}", other),
        }
    }

    #[test]
    fn test_compile_variable_length_traverse() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM users TRAVERSE (a)-[follows*1..3]->(b)";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Traverse { patterns } => {
                        let pattern = &patterns[0];
                        let var_len = pattern.relationship.variable_length.as_ref().unwrap();
                        assert_eq!(var_len.min_hops, Some(1));
                        assert_eq!(var_len.max_hops, Some(3));
                    }
                    _ => panic!("Expected TRAVERSE as input"),
                }
            }
            _ => panic!("Expected PROJECT plan with TRAVERSE input"),
        }
    }

    #[test]
    fn test_compile_optional_relationship() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM users TRAVERSE (a)-[follows?]->(b)";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Traverse { patterns } => {
                        let pattern = &patterns[0];
                        assert!(pattern.relationship.optional);
                    }
                    _ => panic!("Expected TRAVERSE as input"),
                }
            }
            _ => panic!("Expected PROJECT plan with TRAVERSE input"),
        }
    }

    #[test]
    fn test_compile_multiple_traverse_patterns() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM users TRAVERSE (a)-[follows]->(b), (b)-[likes]->(c)";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Traverse { patterns } => {
                        assert_eq!(patterns.len(), 2);
                        // Check first pattern
                        assert_eq!(patterns[0].relationship.rel_type, Some("follows".to_string()));
                        // Check second pattern
                        assert_eq!(patterns[1].relationship.rel_type, Some("likes".to_string()));
                    }
                    _ => panic!("Expected TRAVERSE as input"),
                }
            }
            _ => panic!("Expected PROJECT plan with TRAVERSE input"),
        }
    }
}