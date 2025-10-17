use serde::{Deserialize, Serialize};

use super::{ExecutionPlan, CompiledExpression};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub tables_accessed: Vec<String>,
    pub columns_accessed: Vec<String>,
    pub functions_used: Vec<String>,
    pub requires_spatial_index: bool,
    pub requires_vector_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCost {
    pub estimated_rows: u64,
    pub estimated_cpu_cost: f64,
    pub estimated_memory_mb: f64,
    pub estimated_io_ops: u64,
}

pub struct CostEstimator;

pub struct MetadataGenerator;

impl MetadataGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_metadata(&self, plan: &ExecutionPlan) -> QueryMetadata {
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
            ExecutionPlan::Project { input, expressions, .. } => {
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
            ExecutionPlan::Join { left, right, on_condition, .. } => {
                self.collect_metadata(left, metadata);
                self.collect_metadata(right, metadata);
                self.collect_expression_metadata(on_condition, metadata);
            }
            ExecutionPlan::GeometricOperation { params, input, .. } => {
                metadata.requires_spatial_index = true;
                for (_key, expr) in params {
                    self.collect_expression_metadata(expr, metadata);
                }
                if let Some(input_plan) = input {
                    self.collect_metadata(input_plan, metadata);
                }
            }
            ExecutionPlan::VectorOperation { params, input, .. } => {
                metadata.requires_vector_index = true;
                for (_key, expr) in params {
                    self.collect_expression_metadata(expr, metadata);
                }
                if let Some(input_plan) = input {
                    self.collect_metadata(input_plan, metadata);
                }
            }
            ExecutionPlan::StreamOperation { params, input, .. } => {
                for (_key, expr) in params {
                    self.collect_expression_metadata(expr, metadata);
                }
                if let Some(input_plan) = input {
                    self.collect_metadata(input_plan, metadata);
                }
            }
            ExecutionPlan::TimeSeriesOperation { params, input, .. } => {
                for (_key, expr) in params {
                    self.collect_expression_metadata(expr, metadata);
                }
                if let Some(input_plan) = input {
                    self.collect_metadata(input_plan, metadata);
                }
            }
            ExecutionPlan::GraphOperation { params, input, .. } => {
                for (_key, expr) in params {
                    self.collect_expression_metadata(expr, metadata);
                }
                if let Some(input_plan) = input {
                    self.collect_metadata(input_plan, metadata);
                }
            }
        }
    }

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
            }
        }
    }
}

impl CostEstimator {
    pub fn new() -> Self {
        Self
    }

    pub fn estimate_cost(&self, plan: &ExecutionPlan) -> ExecutionCost {
        match plan {
            ExecutionPlan::Scan { .. } => ExecutionCost {
                estimated_rows: 1000,
                estimated_cpu_cost: 1.0,
                estimated_memory_mb: 10.0,
                estimated_io_ops: 100,
            },
            ExecutionPlan::Filter { input, .. } => {
                let input_cost = self.estimate_cost(input);
                ExecutionCost {
                    estimated_rows: input_cost.estimated_rows / 2,
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
                    estimated_rows: input_cost.estimated_rows / 2,
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
                    estimated_memory_mb: input_cost.estimated_memory_mb * 2.0,
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
                    estimated_rows: 100,
                    estimated_cpu_cost: 2.0,
                    estimated_memory_mb: 5.0,
                    estimated_io_ops: 50,
                }
            }
            ExecutionPlan::Delete { .. } => {
                ExecutionCost {
                    estimated_rows: 100,
                    estimated_cpu_cost: 1.5,
                    estimated_memory_mb: 3.0,
                    estimated_io_ops: 30,
                }
            }
            ExecutionPlan::Traverse { patterns } => {
                let pattern_count = patterns.len() as f64;
                let estimated_traversal_cost = pattern_count * 10.0;

                ExecutionCost {
                    estimated_rows: 1000,
                    estimated_cpu_cost: estimated_traversal_cost,
                    estimated_memory_mb: pattern_count * 20.0,
                    estimated_io_ops: (pattern_count * 100.0) as u64,
                }
            }
            ExecutionPlan::Join { left, right, .. } => {
                let left_cost = self.estimate_cost(left);
                let right_cost = self.estimate_cost(right);

                // Hash join cost estimate: O(N + M)
                ExecutionCost {
                    estimated_rows: left_cost.estimated_rows.max(right_cost.estimated_rows),
                    estimated_cpu_cost: left_cost.estimated_cpu_cost + right_cost.estimated_cpu_cost +
                                       (left_cost.estimated_rows as f64 + right_cost.estimated_rows as f64) * 0.001,
                    estimated_memory_mb: left_cost.estimated_memory_mb + right_cost.estimated_memory_mb +
                                        (right_cost.estimated_rows as f64 * 0.01),
                    estimated_io_ops: left_cost.estimated_io_ops + right_cost.estimated_io_ops,
                }
            }
            ExecutionPlan::GeometricOperation { input, .. } => {
                let base_cost = ExecutionCost {
                    estimated_rows: 1000,
                    estimated_cpu_cost: 5.0,
                    estimated_memory_mb: 10.0,
                    estimated_io_ops: 50,
                };
                if let Some(input_plan) = input {
                    let input_cost = self.estimate_cost(input_plan);
                    ExecutionCost {
                        estimated_rows: input_cost.estimated_rows,
                        estimated_cpu_cost: input_cost.estimated_cpu_cost + base_cost.estimated_cpu_cost,
                        estimated_memory_mb: input_cost.estimated_memory_mb + base_cost.estimated_memory_mb,
                        estimated_io_ops: input_cost.estimated_io_ops + base_cost.estimated_io_ops,
                    }
                } else {
                    base_cost
                }
            }
            ExecutionPlan::VectorOperation { input, .. } => {
                let base_cost = ExecutionCost {
                    estimated_rows: 1000,
                    estimated_cpu_cost: 8.0,
                    estimated_memory_mb: 20.0,
                    estimated_io_ops: 100,
                };
                if let Some(input_plan) = input {
                    let input_cost = self.estimate_cost(input_plan);
                    ExecutionCost {
                        estimated_rows: input_cost.estimated_rows,
                        estimated_cpu_cost: input_cost.estimated_cpu_cost + base_cost.estimated_cpu_cost,
                        estimated_memory_mb: input_cost.estimated_memory_mb + base_cost.estimated_memory_mb,
                        estimated_io_ops: input_cost.estimated_io_ops + base_cost.estimated_io_ops,
                    }
                } else {
                    base_cost
                }
            }
            ExecutionPlan::StreamOperation { input, .. } => {
                let base_cost = ExecutionCost {
                    estimated_rows: 5000,
                    estimated_cpu_cost: 3.0,
                    estimated_memory_mb: 50.0,
                    estimated_io_ops: 200,
                };
                if let Some(input_plan) = input {
                    let input_cost = self.estimate_cost(input_plan);
                    ExecutionCost {
                        estimated_rows: input_cost.estimated_rows + base_cost.estimated_rows,
                        estimated_cpu_cost: input_cost.estimated_cpu_cost + base_cost.estimated_cpu_cost,
                        estimated_memory_mb: input_cost.estimated_memory_mb + base_cost.estimated_memory_mb,
                        estimated_io_ops: input_cost.estimated_io_ops + base_cost.estimated_io_ops,
                    }
                } else {
                    base_cost
                }
            }
            ExecutionPlan::TimeSeriesOperation { input, .. } => {
                let base_cost = ExecutionCost {
                    estimated_rows: 2000,
                    estimated_cpu_cost: 4.0,
                    estimated_memory_mb: 15.0,
                    estimated_io_ops: 75,
                };
                if let Some(input_plan) = input {
                    let input_cost = self.estimate_cost(input_plan);
                    ExecutionCost {
                        estimated_rows: input_cost.estimated_rows,
                        estimated_cpu_cost: input_cost.estimated_cpu_cost + base_cost.estimated_cpu_cost,
                        estimated_memory_mb: input_cost.estimated_memory_mb + base_cost.estimated_memory_mb,
                        estimated_io_ops: input_cost.estimated_io_ops + base_cost.estimated_io_ops,
                    }
                } else {
                    base_cost
                }
            }
            ExecutionPlan::GraphOperation { input, .. } => {
                let base_cost = ExecutionCost {
                    estimated_rows: 1000,
                    estimated_cpu_cost: 15.0,
                    estimated_memory_mb: 100.0,
                    estimated_io_ops: 500,
                };
                if let Some(input_plan) = input {
                    let input_cost = self.estimate_cost(input_plan);
                    ExecutionCost {
                        estimated_rows: input_cost.estimated_rows,
                        estimated_cpu_cost: input_cost.estimated_cpu_cost + base_cost.estimated_cpu_cost,
                        estimated_memory_mb: input_cost.estimated_memory_mb + base_cost.estimated_memory_mb,
                        estimated_io_ops: input_cost.estimated_io_ops + base_cost.estimated_io_ops,
                    }
                } else {
                    base_cost
                }
            }
        }
    }
}

impl Default for CostEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MetadataGenerator {
    fn default() -> Self {
        Self::new()
    }
}
