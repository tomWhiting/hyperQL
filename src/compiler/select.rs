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

        // CRITICAL: Check for vector/geometric operations in WHERE clause
        if let Some(where_expr) = select.where_clause {
            let compiled_predicate = self.expression_compiler.compile_expression(where_expr)?;

            // Detect if this is a geometric operation that should use GeometricOperation plan
            if self.is_geometric_function(&compiled_predicate) {
                // Generate GeometricOperation plan instead of Filter plan
                plan = self.create_geometric_operation_plan(compiled_predicate, plan)?;
            } else if self.is_vector_function(&compiled_predicate) {
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
        // Push LIMIT to scan when it's safe for early termination
        // Safe cases:
        // 1. No WHERE/ORDER BY/GROUP BY/HAVING: Can stop after LIMIT entities
        // 2. WHERE only (no ORDER BY/GROUP BY): Can stop after finding LIMIT matching entities
        //
        // NOT safe:
        // - ORDER BY: Need all entities to sort correctly
        // - GROUP BY: Need all entities to compute groups
        // - HAVING: Need all groups to filter
        let scan_limit = if select.where_clause.is_none()
            && select.order_by.is_empty()
            && select.group_by.is_empty()
            && select.having.is_none() {
            // No filtering/sorting/grouping: safe to push LIMIT
            select.limit
        } else if select.where_clause.is_some()
            && select.order_by.is_empty()
            && select.group_by.is_empty()
            && select.having.is_none() {
            // WHERE-only queries: push LIMIT for early termination
            // The scan will stop after finding LIMIT matching entities
            select.limit
        } else {
            // ORDER BY or GROUP BY present: need all data
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
            Expression::Column(col_ref) => {
                // Include table prefix if present
                if let Some(ref table) = col_ref.table {
                    format!("{}.{}", table, col_ref.name)
                } else {
                    col_ref.name.clone()
                }
            }
            Expression::Function { name, args } => {
                // Generate proper function call syntax for better column naming
                if args.is_empty() {
                    // COUNT() with no args
                    format!("{}(*)", name)
                } else if args.len() == 1 {
                    // Check if arg is a wildcard column reference
                    if let Expression::Column(col_ref) = &args[0] {
                        if col_ref.name == "*" {
                            return format!("{}(*)", name);
                        }
                        // For column references, include the column name
                        return format!("{}({})", name, self.generate_expression_name(&args[0]));
                    }
                    // For other single-arg functions
                    format!("{}({})", name, self.generate_expression_name(&args[0]))
                } else {
                    // Multi-arg functions
                    format!("{}(...)", name)
                }
            }
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

    /// Check if an expression contains a geometric function
    fn is_geometric_function(&self, expr: &CompiledExpression) -> bool {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                // Check if this is a geometric function
                // Note: "near" and "distance" are parser-generated names (not from ExpressionCompiler)
                let is_geometric = matches!(
                    name.to_uppercase().as_str(),
                    "HYPERBOLIC_DISTANCE" | "GEODESIC_DISTANCE" |
                    "WITHIN_RADIUS" | "NEAR_POSITIONS" |
                    "CONTAINS" | "INTERSECTS" |
                    "NEAR" | "DISTANCE"  // Parser-generated geometric functions
                );

                if is_geometric {
                    return true;
                }

                // Recursively check args for nested geometric functions
                args.iter().any(|arg| self.is_geometric_function(arg))
            }
            CompiledExpression::Binary { left, right, .. } => {
                self.is_geometric_function(left) || self.is_geometric_function(right)
            }
            CompiledExpression::Unary { expr, .. } => {
                self.is_geometric_function(expr)
            }
            _ => false,
        }
    }

    /// Extract the geometric operation type from a function name
    fn function_name_to_geometric_op_type(&self, name: &str) -> Option<super::GeometricOpType> {
        match name.to_uppercase().as_str() {
            "HYPERBOLIC_DISTANCE" => Some(super::GeometricOpType::HyperbolicDistance),
            "GEODESIC_DISTANCE" => Some(super::GeometricOpType::GeodesicDistance),
            "WITHIN_RADIUS" => Some(super::GeometricOpType::WithinRadius),
            "NEAR_POSITIONS" => Some(super::GeometricOpType::NearPositions),
            "CONTAINS" => Some(super::GeometricOpType::Contains),
            "INTERSECTS" => Some(super::GeometricOpType::Intersects),
            // Parser-generated function names
            "NEAR" => Some(super::GeometricOpType::NearPositions),
            "DISTANCE" => Some(super::GeometricOpType::HyperbolicDistance),
            _ => None,
        }
    }

    /// Extract the primary geometric function from an expression tree
    fn extract_geometric_function(&self, expr: &CompiledExpression) -> Option<(super::GeometricOpType, Vec<CompiledExpression>)> {
        match expr {
            CompiledExpression::Function { name, args, .. } => {
                if let Some(op_type) = self.function_name_to_geometric_op_type(name) {
                    return Some((op_type, args.clone()));
                }
                // Search in arguments
                for arg in args {
                    if let Some(result) = self.extract_geometric_function(arg) {
                        return Some(result);
                    }
                }
                None
            }
            CompiledExpression::Binary { left, right, .. } => {
                // Check left first, then right
                self.extract_geometric_function(left)
                    .or_else(|| self.extract_geometric_function(right))
            }
            CompiledExpression::Unary { expr, .. } => {
                self.extract_geometric_function(expr)
            }
            _ => None,
        }
    }

    /// Create a GeometricOperation execution plan from a compiled expression
    fn create_geometric_operation_plan(
        &self,
        expr: CompiledExpression,
        input: ExecutionPlan,
    ) -> Result<ExecutionPlan> {
        // Extract the geometric function and its parameters
        let (op_type, args) = self.extract_geometric_function(&expr)
            .ok_or_else(|| HyperQLError::SemanticError {
                message: "No geometric function found in expression".to_string(),
                context: vec!["create_geometric_operation_plan".to_string()],
            })?;

        // Build parameter map from function arguments
        let mut params = HashMap::new();

        // Map arguments based on operation type
        match op_type {
            super::GeometricOpType::HyperbolicDistance | super::GeometricOpType::GeodesicDistance => {
                // Parser-generated "distance" function: args = [reference]
                // Compiled HYPERBOLIC_DISTANCE: args = [entity1, entity2]
                if args.len() == 1 {
                    // Parser format: distance(reference)
                    // Create column reference for "position" and use reference as entity2
                    params.insert("entity1".to_string(), CompiledExpression::Column {
                        table: None,
                        name: "position".to_string(),
                        value_type: super::ValueType::Position,
                    });
                    params.insert("entity2".to_string(), args[0].clone());
                } else if args.len() >= 2 {
                    // Compiled format
                    params.insert("entity1".to_string(), args[0].clone());
                    params.insert("entity2".to_string(), args[1].clone());
                }
                // Optional third argument for max_distance in NEAR context
                if args.len() >= 3 {
                    params.insert("max_distance".to_string(), args[2].clone());
                }
            }
            super::GeometricOpType::WithinRadius => {
                // WITHIN_RADIUS takes: target, reference, radius
                if args.len() >= 3 {
                    params.insert("target".to_string(), args[0].clone());
                    params.insert("center".to_string(), args[1].clone());
                    params.insert("radius".to_string(), args[2].clone());
                }
            }
            super::GeometricOpType::NearPositions => {
                // Parser-generated "near" function: args = [reference, radius]
                // Compiled NEAR_POSITIONS: args = [reference, limit]
                if args.len() >= 2 {
                    // Parser format: near(reference, radius)
                    params.insert("reference".to_string(), args[0].clone());
                    params.insert("radius".to_string(), args[1].clone());
                } else if !args.is_empty() {
                    // Compiled format
                    params.insert("reference".to_string(), args[0].clone());
                    if args.len() >= 2 {
                        params.insert("limit".to_string(), args[1].clone());
                    }
                }
            }
            super::GeometricOpType::Contains | super::GeometricOpType::Intersects => {
                // These take: geometry1, geometry2
                if args.len() >= 2 {
                    params.insert("geometry1".to_string(), args[0].clone());
                    params.insert("geometry2".to_string(), args[1].clone());
                }
            }
        }

        // Store the full expression for evaluation
        params.insert("_full_expression".to_string(), expr);

        Ok(ExecutionPlan::GeometricOperation {
            op_type,
            params,
            input: Some(Box::new(input)),
        })
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
