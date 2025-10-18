use crate::ast::*;
use crate::error::*;

use super::expression::ExpressionCompiler;
use super::{CompiledProjection, CompiledSortKey, CompiledExpression, ExecutionPlan, VectorOpType};
use std::collections::HashMap;

pub struct SelectCompiler {
    expression_compiler: ExpressionCompiler,
}

impl SelectCompiler {
    pub fn new() -> Self {
        Self {
            expression_compiler: ExpressionCompiler::new(),
        }
    }

    pub fn compile_select(&self, select: SelectStatement) -> Result<ExecutionPlan> {
        let mut plan = self.create_base_scan(&select)?;

        // Handle JOINs after base scan
        if !select.joins.is_empty() {
            plan = self.compile_joins(plan, &select.joins)?;
        }

        if let Some(traverse_clause) = select.traverse_clause {
            let compiled_patterns = self.compile_traverse_patterns(&traverse_clause.patterns)?;
            let traverse_plan = ExecutionPlan::Traverse {
                patterns: compiled_patterns,
            };
            if let ExecutionPlan::Scan { .. } = plan {
                plan = traverse_plan;
            } else {
                plan = traverse_plan;
            }
        }

        // CRITICAL: Check for vector operations in WHERE clause
        if let Some(where_expr) = select.where_clause {
            let compiled_predicate = self.expression_compiler.compile_expression(where_expr)?;

            // Detect if this is a vector operation that should use VectorOperation plan
            if self.is_vector_function(&compiled_predicate) {
                // Generate VectorOperation plan instead of Filter plan
                plan = self.create_vector_operation_plan(compiled_predicate, plan)?;
            } else {
                // Regular filter
                plan = ExecutionPlan::Filter {
                    input: Box::new(plan),
                    predicate: compiled_predicate,
                };
            }
        }

        // Compile projections first to check for aggregate functions
        let projections = self.compile_select_list(&select.select_list)?;

        // Check if any projection contains an aggregate function
        let has_aggregates = projections.iter().any(|proj| self.contains_aggregate(&proj.expression));

        if !select.group_by.is_empty() || has_aggregates {
            // GROUP BY with explicit grouping columns, or implicit grouping (aggregates without GROUP BY)
            let group_expressions = if !select.group_by.is_empty() {
                select.group_by.iter()
                    .map(|expr| self.expression_compiler.compile_expression(expr.clone()))
                    .collect::<Result<Vec<_>>>()?
            } else {
                // No GROUP BY clause but has aggregates - implicit single-group aggregation
                vec![]
            };

            plan = ExecutionPlan::GroupBy {
                input: Box::new(plan),
                group_expressions,
                aggregate_expressions: projections,
            };
        } else {
            // No aggregates - regular projection
            if !projections.is_empty() {
                plan = ExecutionPlan::Project {
                    input: Box::new(plan),
                    expressions: projections,
                    distinct: select.distinct,
                };
            }
        }

        if let Some(having_expr) = select.having {
            let compiled_having = self.expression_compiler.compile_expression(having_expr)?;
            plan = ExecutionPlan::Having {
                input: Box::new(plan),
                predicate: compiled_having,
            };
        }

        // CRITICAL: Check for vector operations in ORDER BY (k-NN pattern)
        if !select.order_by.is_empty() {
            // Check if ORDER BY uses a vector distance function (k-NN pattern)
            if let Some(vector_plan) = self.try_create_knn_plan(&select.order_by, select.limit, plan.clone())? {
                plan = vector_plan;
            } else {
                // Regular sort
                let sort_keys = self.compile_order_by(&select.order_by)?;
                plan = ExecutionPlan::Sort {
                    input: Box::new(plan),
                    sort_keys,
                };
            }
        }

        if let Some(limit) = select.limit {
            plan = ExecutionPlan::Limit {
                input: Box::new(plan),
                count: limit,
                offset: select.offset,
            };
        }

        Ok(plan)
    }

    fn create_base_scan(&self, select: &SelectStatement) -> Result<ExecutionPlan> {
        let (table_name, entity_type, alias) = match &select.from {
            Some(FromClause::Table { collection, entity_type, alias }) => {
                // Convert Option<String> to String (use empty string when None)
                let entity_type_str = entity_type.as_ref().map(|s| s.clone()).unwrap_or_default();
                (collection.clone(), entity_type_str, alias.clone())
            }
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

        // CRITICAL OPTIMIZATION: Pass LIMIT down to Scan when query is simple enough
        // Only push down LIMIT if there's no WHERE, ORDER BY, or GROUP BY
        // (those operations need full result set before limiting)
        let scan_limit = if select.where_clause.is_none()
            && select.order_by.is_empty()
            && select.group_by.is_empty()
            && select.having.is_none() {
            select.limit
        } else {
            None
        };

        Ok(ExecutionPlan::Scan {
            table: table_name,
            entity_type,
            alias,
            filter: None,
            projection: vec![],
            limit: scan_limit,
        })
    }

    fn compile_select_list(&self, select_list: &[SelectItem]) -> Result<Vec<CompiledProjection>> {
        let mut projections = Vec::new();

        for item in select_list {
            match item {
                SelectItem::Wildcard => {
                    projections.push(CompiledProjection {
                        expression: CompiledExpression::Literal(crate::types::Value::String("*".to_string())),
                        alias: None,
                        output_name: "*".to_string(),
                    });
                }
                SelectItem::Expression { expr, alias } => {
                    let compiled_expr = self.expression_compiler.compile_expression(expr.clone())?;
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

    fn compile_order_by(&self, order_by: &[OrderByItem]) -> Result<Vec<CompiledSortKey>> {
        let mut sort_keys = Vec::new();

        for item in order_by {
            let compiled_expr = self.expression_compiler.compile_expression(item.expr.clone())?;
            sort_keys.push(CompiledSortKey {
                expression: compiled_expr,
                direction: item.direction.clone(),
            });
        }

        Ok(sort_keys)
    }

    fn generate_expression_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Column(col_ref) => col_ref.name.clone(),
            Expression::Function { name, .. } => name.clone(),
            Expression::Literal(literal) => format!("{:?}", literal),
            _ => "expr".to_string(),
        }
    }

    fn compile_traverse_patterns(&self, patterns: &[crate::ast::TraversePattern]) -> Result<Vec<super::CompiledTraversePattern>> {
        let mut compiled_patterns = Vec::new();

        for pattern in patterns {
            let compiled_start_node = super::CompiledNodePattern {
                variable: pattern.start_node.variable.clone(),
                label: pattern.start_node.label.clone(),
                properties: if let Some(props) = &pattern.start_node.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_end_node = super::CompiledNodePattern {
                variable: pattern.end_node.variable.clone(),
                label: pattern.end_node.label.clone(),
                properties: if let Some(props) = &pattern.end_node.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_relationship = super::CompiledRelationshipPattern {
                variable: pattern.relationship.variable.clone(),
                rel_type: pattern.relationship.rel_type.clone(),
                direction: pattern.relationship.direction.clone(),
                variable_length: pattern.relationship.variable_length.clone(),
                optional: pattern.relationship.optional,
                properties: if let Some(props) = &pattern.relationship.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            compiled_patterns.push(super::CompiledTraversePattern {
                start_node: compiled_start_node,
                relationship: compiled_relationship,
                end_node: compiled_end_node,
            });
        }

        Ok(compiled_patterns)
    }

    /// Check if an expression contains an aggregate function (COUNT, SUM, AVG, MIN, MAX, etc.)
    fn contains_aggregate(&self, expr: &CompiledExpression) -> bool {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                // Check if this function is an aggregate
                let is_aggregate = matches!(
                    name.to_uppercase().as_str(),
                    "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "ARRAY_AGG" | "STRING_AGG"
                );

                if is_aggregate {
                    return true;
                }

                // Recursively check args for nested aggregates
                args.iter().any(|arg| self.contains_aggregate(arg))
            }
            CompiledExpression::Binary { left, right, .. } => {
                self.contains_aggregate(left) || self.contains_aggregate(right)
            }
            CompiledExpression::Unary { expr, .. } => {
                self.contains_aggregate(expr)
            }
            _ => false,
        }
    }

    /// Compile JOIN clauses into execution plan
    fn compile_joins(&self, left_plan: ExecutionPlan, joins: &[JoinClause]) -> Result<ExecutionPlan> {
        let mut current_plan = left_plan;

        for join_clause in joins {
            // Convert Option<String> to String (use empty string when None)
            let entity_type_str = join_clause.entity_type.as_ref().map(|s| s.clone()).unwrap_or_default();

            // Create scan plan for the right table with alias
            let right_plan = ExecutionPlan::Scan {
                table: join_clause.collection.clone(),
                entity_type: entity_type_str,
                alias: join_clause.alias.clone(),
                filter: None,
                projection: vec![],
                limit: None,
            };

            // Compile the ON condition
            let compiled_condition = self.expression_compiler.compile_expression(join_clause.on_condition.clone())?;

            // Create JOIN plan
            current_plan = ExecutionPlan::Join {
                left: Box::new(current_plan),
                right: Box::new(right_plan),
                join_type: join_clause.join_type.clone(),
                on_condition: compiled_condition,
            };
        }

        Ok(current_plan)
    }

    /// Check if an expression contains a vector function
    fn is_vector_function(&self, expr: &CompiledExpression) -> bool {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                // Check if this is a vector function
                let is_vector = matches!(
                    name.to_uppercase().as_str(),
                    "COSINE_SIMILARITY" | "EUCLIDEAN_DISTANCE" |
                    "DOT_PRODUCT" | "NORMALIZE" |
                    "KNN" | "SIMILARITY_SEARCH"
                );

                if is_vector {
                    return true;
                }

                // Recursively check args for nested vector functions
                args.iter().any(|arg| self.is_vector_function(arg))
            }
            CompiledExpression::Binary { left, right, .. } => {
                self.is_vector_function(left) || self.is_vector_function(right)
            }
            CompiledExpression::Unary { expr, .. } => {
                self.is_vector_function(expr)
            }
            _ => false,
        }
    }

    /// Extract the vector operation type from a function name
    fn function_name_to_vector_op_type(&self, name: &str) -> Option<VectorOpType> {
        match name.to_uppercase().as_str() {
            "COSINE_SIMILARITY" => Some(VectorOpType::CosineSimilarity),
            "EUCLIDEAN_DISTANCE" => Some(VectorOpType::EuclideanDistance),
            "DOT_PRODUCT" => Some(VectorOpType::DotProduct),
            "NORMALIZE" => Some(VectorOpType::Normalize),
            "KNN" => Some(VectorOpType::KNN),
            "SIMILARITY_SEARCH" => Some(VectorOpType::SimilaritySearch),
            _ => None,
        }
    }

    /// Extract the primary vector function from an expression tree
    fn extract_vector_function(&self, expr: &CompiledExpression) -> Option<(VectorOpType, Vec<CompiledExpression>)> {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                if let Some(op_type) = self.function_name_to_vector_op_type(name) {
                    return Some((op_type, args.clone()));
                }
                // Search in arguments
                for arg in args {
                    if let Some(result) = self.extract_vector_function(arg) {
                        return Some(result);
                    }
                }
                None
            }
            CompiledExpression::Binary { left, right, .. } => {
                // Check left first, then right
                self.extract_vector_function(left)
                    .or_else(|| self.extract_vector_function(right))
            }
            CompiledExpression::Unary { expr, .. } => {
                self.extract_vector_function(expr)
            }
            _ => None,
        }
    }

    /// Create a VectorOperation execution plan from a compiled expression
    fn create_vector_operation_plan(
        &self,
        expr: CompiledExpression,
        input: ExecutionPlan,
    ) -> Result<ExecutionPlan> {
        // Extract the vector function and its parameters
        let (op_type, args) = self.extract_vector_function(&expr)
            .ok_or_else(|| HyperQLError::SemanticError {
                message: "No vector function found in expression".to_string(),
                context: vec!["create_vector_operation_plan".to_string()],
            })?;

        // Build parameter map from function arguments
        let mut params = HashMap::new();

        // Map arguments based on operation type
        match op_type {
            VectorOpType::CosineSimilarity | VectorOpType::EuclideanDistance | VectorOpType::DotProduct => {
                // These take: vector_name, reference, metric, threshold, vector_type
                if args.len() >= 2 {
                    params.insert("vector_name".to_string(), args[0].clone());
                    params.insert("reference".to_string(), args[1].clone());
                }
                if args.len() >= 3 {
                    params.insert("metric".to_string(), args[2].clone());
                }
                if args.len() >= 4 {
                    params.insert("threshold".to_string(), args[3].clone());
                }
                if args.len() >= 5 {
                    params.insert("vector_type".to_string(), args[4].clone());
                }
            }
            VectorOpType::KNN => {
                // KNN takes: vector_name, reference, k, metric, vector_type
                if args.len() >= 3 {
                    params.insert("vector_name".to_string(), args[0].clone());
                    params.insert("reference".to_string(), args[1].clone());
                    params.insert("k".to_string(), args[2].clone());
                }
                if args.len() >= 4 {
                    params.insert("metric".to_string(), args[3].clone());
                }
                if args.len() >= 5 {
                    params.insert("vector_type".to_string(), args[4].clone());
                }
            }
            VectorOpType::Normalize => {
                // Normalize takes: vector
                if !args.is_empty() {
                    params.insert("vector".to_string(), args[0].clone());
                }
            }
            VectorOpType::SimilaritySearch => {
                // SimilaritySearch takes similar args to KNN
                if args.len() >= 2 {
                    params.insert("vector_name".to_string(), args[0].clone());
                    params.insert("reference".to_string(), args[1].clone());
                }
                if args.len() >= 3 {
                    params.insert("metric".to_string(), args[2].clone());
                }
                if args.len() >= 4 {
                    params.insert("threshold".to_string(), args[3].clone());
                }
            }
        }

        // Store the full expression for evaluation
        params.insert("_full_expression".to_string(), expr);

        Ok(ExecutionPlan::VectorOperation {
            op_type,
            params,
            input: Some(Box::new(input)),
        })
    }

    /// Try to create a k-NN plan from ORDER BY clause
    /// This handles the pattern: ORDER BY distance_function(...) LIMIT k
    fn try_create_knn_plan(
        &self,
        order_by: &[OrderByItem],
        limit: Option<u64>,
        input: ExecutionPlan,
    ) -> Result<Option<ExecutionPlan>> {
        // k-NN pattern requires exactly one ORDER BY expression
        if order_by.len() != 1 {
            return Ok(None);
        }

        // Compile the ORDER BY expression
        let order_expr = self.expression_compiler.compile_expression(order_by[0].expr.clone())?;

        // Check if it's a vector distance function
        if !self.is_vector_function(&order_expr) {
            return Ok(None);
        }

        // Extract the vector function
        let (op_type, args) = match self.extract_vector_function(&order_expr) {
            Some(result) => result,
            None => return Ok(None),
        };

        // Only distance functions make sense for k-NN
        if !matches!(op_type, VectorOpType::CosineSimilarity | VectorOpType::EuclideanDistance | VectorOpType::DotProduct) {
            return Ok(None);
        }

        // Build KNN operation parameters
        let mut params = HashMap::new();

        if args.len() >= 2 {
            params.insert("vector_name".to_string(), args[0].clone());
            params.insert("reference".to_string(), args[1].clone());
        }
        if args.len() >= 3 {
            params.insert("metric".to_string(), args[2].clone());
        }
        if args.len() >= 5 {
            params.insert("vector_type".to_string(), args[4].clone());
        }

        // Add k from LIMIT clause
        if let Some(k) = limit {
            params.insert("k".to_string(), CompiledExpression::Literal(crate::types::Value::Int(k as i64)));
        }

        // Add sort direction (ASC for distance = nearest first)
        let direction = match order_by[0].direction {
            OrderDirection::Asc => "asc",
            OrderDirection::Desc => "desc",
        };
        params.insert("direction".to_string(), CompiledExpression::Literal(crate::types::Value::String(direction.to_string())));

        Ok(Some(ExecutionPlan::VectorOperation {
            op_type: VectorOpType::KNN,
            params,
            input: Some(Box::new(input)),
        }))
    }
}

impl Default for SelectCompiler {
    fn default() -> Self {
        Self::new()
    }
}
