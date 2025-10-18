use crate::ast::*;
use crate::error::*;
use serde::{Deserialize, Serialize};

// Re-export JoinType from AST for use in ExecutionPlan
pub use crate::ast::JoinType;

mod expression;
mod select;
mod statement;
mod traverse;
mod metadata;

use select::SelectCompiler;
use statement::StatementCompiler;
use metadata::{MetadataGenerator, CostEstimator};

pub use metadata::{QueryMetadata, ExecutionCost};

/// Geometric operation types - plans for Hyperspatial to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeometricOpType {
    /// Hyperbolic distance calculation
    HyperbolicDistance,
    /// Geodesic distance calculation
    GeodesicDistance,
    /// Within radius check
    WithinRadius,
    /// Near positions query
    NearPositions,
    /// Containment testing
    Contains,
    /// Intersection operations
    Intersects,
}

/// Vector operation types - plans for Hyperspatial to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Stream operation types - plans for Hyperspatial to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamOpType {
    /// Create stream
    CreateStream,
    /// Produce to stream
    ProduceStream,
    /// Consume from stream
    ConsumeStream,
    /// Tumbling window
    TumblingWindow,
    /// Sliding window
    SlidingWindow,
    /// Stream join
    StreamJoin,
}

/// Time series operation types - plans for Hyperspatial to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesOpType {
    /// Time bucket aggregation
    TimeBucket,
    /// Time difference calculation
    TimeDiff,
    /// Extract time component
    ExtractTime,
    /// Moving average
    MovingAverage,
    /// Exponential smoothing
    ExponentialSmoothing,
    /// Lag function
    Lag,
    /// Lead function
    Lead,
}

/// Graph operation types - plans for Hyperspatial to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphOpType {
    /// Shortest path calculation
    ShortestPath,
    /// Page rank calculation
    PageRank,
    /// Community detection
    CommunityDetection,
    /// Graph traversal
    GraphTraversal,
    /// Connected components
    ConnectedComponents,
    /// Centrality measures
    Centrality,
}

pub struct Compiler {
    select_compiler: SelectCompiler,
    statement_compiler: StatementCompiler,
    metadata_generator: MetadataGenerator,
    cost_estimator: CostEstimator,
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

/// Execution plan structure - represents operations to be executed by Hyperspatial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPlan {
    /// Sequential scan of entities
    Scan {
        table: String,
        entity_type: String,
        alias: Option<String>,
        filter: Option<CompiledExpression>,
        projection: Vec<CompiledProjection>,
        limit: Option<u64>,
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
        distinct: bool,
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
    /// JOIN operation for combining tables
    Join {
        left: Box<ExecutionPlan>,
        right: Box<ExecutionPlan>,
        join_type: JoinType,
        on_condition: CompiledExpression,
    },
    /// Geometric operation plan
    GeometricOperation {
        op_type: GeometricOpType,
        params: std::collections::HashMap<String, CompiledExpression>,
        input: Option<Box<ExecutionPlan>>,
    },
    /// Vector operation plan
    VectorOperation {
        op_type: VectorOpType,
        params: std::collections::HashMap<String, CompiledExpression>,
        input: Option<Box<ExecutionPlan>>,
    },
    /// Stream operation plan
    StreamOperation {
        op_type: StreamOpType,
        params: std::collections::HashMap<String, CompiledExpression>,
        input: Option<Box<ExecutionPlan>>,
    },
    /// Time series operation plan
    TimeSeriesOperation {
        op_type: TimeSeriesOpType,
        params: std::collections::HashMap<String, CompiledExpression>,
        input: Option<Box<ExecutionPlan>>,
    },
    /// Graph operation plan
    GraphOperation {
        op_type: GraphOpType,
        params: std::collections::HashMap<String, CompiledExpression>,
        input: Option<Box<ExecutionPlan>>,
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



impl Compiler {
    pub fn new() -> Self {
        Self {
            select_compiler: SelectCompiler::new(),
            statement_compiler: StatementCompiler::new(),
            metadata_generator: MetadataGenerator::new(),
            cost_estimator: CostEstimator::new(),
        }
    }

    pub fn compile(&self, statement: Statement) -> Result<CompiledQuery> {
        let plan = match &statement {
            Statement::Select(select) => self.select_compiler.compile_select(select.clone())?,
            Statement::Insert(insert) => {
                let plan = self.statement_compiler.compile_insert(insert.clone())?;
                let metadata = QueryMetadata {
                    tables_accessed: vec![insert.table.clone()],
                    columns_accessed: vec![],
                    functions_used: vec![],
                    requires_spatial_index: false,
                    requires_vector_index: false,
                };
                let estimated_cost = ExecutionCost {
                    estimated_rows: 1,
                    estimated_cpu_cost: 0.5,
                    estimated_memory_mb: 1.0,
                    estimated_io_ops: 1,
                };
                return Ok(CompiledQuery {
                    plan,
                    metadata,
                    estimated_cost,
                });
            },
            Statement::Update(update) => {
                let plan = self.statement_compiler.compile_update(update.clone())?;
                let metadata = QueryMetadata {
                    tables_accessed: vec![update.table.clone()],
                    columns_accessed: vec![],
                    functions_used: vec![],
                    requires_spatial_index: false,
                    requires_vector_index: false,
                };
                let estimated_cost = ExecutionCost {
                    estimated_rows: 100,
                    estimated_cpu_cost: 2.0,
                    estimated_memory_mb: 5.0,
                    estimated_io_ops: 50,
                };
                return Ok(CompiledQuery {
                    plan,
                    metadata,
                    estimated_cost,
                });
            },
            Statement::Delete(delete) => {
                let plan = self.statement_compiler.compile_delete(delete.clone())?;
                let metadata = QueryMetadata {
                    tables_accessed: vec![delete.table.clone()],
                    columns_accessed: vec![],
                    functions_used: vec![],
                    requires_spatial_index: false,
                    requires_vector_index: false,
                };
                let estimated_cost = ExecutionCost {
                    estimated_rows: 100,
                    estimated_cpu_cost: 1.5,
                    estimated_memory_mb: 3.0,
                    estimated_io_ops: 30,
                };
                return Ok(CompiledQuery {
                    plan,
                    metadata,
                    estimated_cost,
                });
            },
            Statement::Schema(_schema_op) => {
                // TODO: Implement schema DDL compilation
                // Schema statements need to be compiled into SchemaEngine operations
                // for creating, altering, and dropping schemas
                return Err(HyperQLError::SemanticError {
                    message: "Schema DDL compilation not yet implemented".to_string(),
                    context: vec!["Schema operations will be supported in a future release".to_string()],
                });
            },
            Statement::Stream(_stream_op) => {
                // TODO: Implement stream DDL compilation
                // Stream statements need to be compiled into stream engine operations
                // for creating streams, producing events, and consuming from streams
                return Err(HyperQLError::SemanticError {
                    message: "Stream DDL compilation not yet implemented".to_string(),
                    context: vec!["Stream operations will be supported in a future release".to_string()],
                });
            },
        };

        let metadata = self.metadata_generator.generate_metadata(&plan);
        let estimated_cost = self.cost_estimator.estimate_cost(&plan);

        Ok(CompiledQuery {
            plan,
            metadata,
            estimated_cost,
        })
    }








    /// Compile a statement with query optimization
    pub fn compile_with_optimizer(
        &self,
        statement: Statement,
        optimizer: &crate::optimizer::QueryOptimizer
    ) -> Result<CompiledQuery> {
        let compiled = self.compile(statement)?;

        // Apply optimizations to the execution plan
        let optimized_plan = optimizer.optimize(compiled.plan)?;

        // Recalculate metadata and cost estimates for the optimized plan
        let optimized_metadata = self.metadata_generator.generate_metadata(&optimized_plan);
        let optimized_cost = self.cost_estimator.estimate_cost(&optimized_plan);

        Ok(CompiledQuery {
            plan: optimized_plan,
            metadata: optimized_metadata,
            estimated_cost: optimized_cost,
        })
    }

    /// Compile with optimizer and return optimization statistics
    pub fn compile_with_optimizer_stats(
        &self,
        statement: Statement,
        optimizer: &crate::optimizer::QueryOptimizer
    ) -> Result<(CompiledQuery, crate::optimizer::OptimizationStats)> {
        let compiled = self.compile(statement)?;

        // Apply optimizations with statistics tracking
        let (optimized_plan, stats) = optimizer.optimize_with_stats(compiled.plan)?;

        // Recalculate metadata and cost estimates for the optimized plan
        let optimized_metadata = self.metadata_generator.generate_metadata(&optimized_plan);
        let optimized_cost = self.cost_estimator.estimate_cost(&optimized_plan);

        let optimized_query = CompiledQuery {
            plan: optimized_plan,
            metadata: optimized_metadata,
            estimated_cost: optimized_cost,
        };

        Ok((optimized_query, stats))
    }

    /// Compile a statement with validation
    pub fn compile_with_validation(
        &self,
        statement: Statement,
        validator: &mut crate::validator::QueryValidator,
    ) -> Result<CompiledQuery> {
        // Validate first
        let validation_result = validator.validate(&statement)?;

        if !validation_result.valid {
            return Err(HyperQLError::ValidationError {
                message: format!("Query validation failed with {} errors", validation_result.errors.len()),
                field: None,
            });
        }

        // If validation passes, compile normally
        self.compile(statement)
    }

    /// Compile with both validation and optimization
    pub fn compile_with_validation_and_optimization(
        &self,
        statement: Statement,
        validator: &mut crate::validator::QueryValidator,
        optimizer: &crate::optimizer::QueryOptimizer,
    ) -> Result<(CompiledQuery, crate::validator::ValidationResult, crate::optimizer::OptimizationStats)> {
        // Validate first
        let validation_result = validator.validate(&statement)?;

        if !validation_result.valid {
            return Err(HyperQLError::ValidationError {
                message: format!("Query validation failed with {} errors", validation_result.errors.len()),
                field: None,
            });
        }

        // Compile and optimize
        let (compiled_query, optimization_stats) = self.compile_with_optimizer_stats(statement, optimizer)?;

        Ok((compiled_query, validation_result, optimization_stats))
    }
}


impl Default for Compiler {
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
        let query = "SELECT * FROM test.Entity";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match compiled.plan {
            ExecutionPlan::Project { input, distinct, .. } => {
                assert!(!distinct, "DISTINCT should be false by default");
                match *input {
                    ExecutionPlan::Scan { table, entity_type, .. } => {
                        assert_eq!(table, "test");
                        assert_eq!(entity_type, "Entity");
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
        let query = "SELECT name FROM test.Entity WHERE name = 'Alice'";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should have Project -> Filter -> Scan structure
        match compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match *input {
                    ExecutionPlan::Filter { input, .. } => {
                        match *input {
                            ExecutionPlan::Scan { table, entity_type, .. } => {
                                assert_eq!(table, "test");
                                assert_eq!(entity_type, "Entity");
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
        let query = "SELECT * FROM test.Entity ORDER BY name LIMIT 10";
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
        let query = "SELECT category, COUNT(*) FROM shop.Product GROUP BY category";
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
        let query = "SELECT category, COUNT(*) FROM shop.Product GROUP BY category HAVING COUNT(*) > 5";
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
        let query = "SELECT * FROM social.User TRAVERSE (a:User)-[r:follows]->(b:User)";
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
        let query = "SELECT * FROM social.User TRAVERSE (a)-[follows*1..3]->(b)";
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
        let query = "SELECT * FROM social.User TRAVERSE (a)-[follows?]->(b)";
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
        let query = "SELECT * FROM social.User TRAVERSE (a)-[follows]->(b), (b)-[likes]->(c)";
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

    #[test]
    fn test_compile_vector_similarity_in_where() {
        let compiler = Compiler::new();
        // Use column reference instead of array literal (parser limitation)
        let query = r#"SELECT * FROM docs.Document
                       WHERE SIMILARITY(embedding, query_vec, 'cosine') > 0.8"#;
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should generate VectorOperation plan (not Filter plan)
        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::VectorOperation { op_type, params, input } => {
                        assert!(matches!(op_type, VectorOpType::CosineSimilarity));
                        assert!(params.contains_key("vector_name"));
                        assert!(params.contains_key("reference"));
                        assert!(input.is_some());
                    }
                    other => panic!("Expected VectorOperation as input to project, got: {:?}", other),
                }
            }
            _ => panic!("Expected PROJECT plan with VectorOperation input"),
        }
    }

    #[test]
    fn test_compile_knn_query_with_order_by() {
        let compiler = Compiler::new();
        // Use SIMILARITY with column reference for ORDER BY
        let query = r#"SELECT * FROM docs.Document
                       ORDER BY SIMILARITY(embedding, query_vec, 'euclidean')
                       LIMIT 10"#;
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should generate VectorOperation with KNN type
        match &compiled.plan {
            ExecutionPlan::Limit { input, count, .. } => {
                assert_eq!(*count, 10);
                match input.as_ref() {
                    ExecutionPlan::VectorOperation { op_type, params, .. } => {
                        assert!(matches!(op_type, VectorOpType::KNN));
                        assert!(params.contains_key("k"));
                        assert!(params.contains_key("vector_name"));
                        assert!(params.contains_key("reference"));
                    }
                    other => panic!("Expected VectorOperation for k-NN, got: {:?}", other),
                }
            }
            _ => panic!("Expected LIMIT plan with VectorOperation input"),
        }
    }

    #[test]
    fn test_compile_cosine_similarity_function() {
        let compiler = Compiler::new();
        // SIMILARITY function with cosine metric
        let query = r#"SELECT * FROM docs.Document
                       WHERE SIMILARITY(text_embedding, query_vec, 'cosine') > 0.7"#;
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Verify VectorOperation is generated
        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::VectorOperation { op_type, params, .. } => {
                        assert!(matches!(op_type, VectorOpType::CosineSimilarity));

                        // Check key parameters are captured
                        assert!(params.contains_key("vector_name"), "Missing vector_name parameter");
                        assert!(params.contains_key("reference"), "Missing reference parameter");
                        assert!(params.contains_key("metric"), "Missing metric parameter");
                    }
                    other => panic!("Expected VectorOperation, got: {:?}", other),
                }
            }
            other => panic!("Expected PROJECT plan, got: {:?}", other),
        }
    }

    #[test]
    fn test_compile_dot_product_function() {
        let compiler = Compiler::new();
        // SIMILARITY with dotproduct metric
        let query = r#"SELECT * FROM docs.Document
                       WHERE SIMILARITY(feature_vec, target_vec, 'dotproduct') > 0.5"#;
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::VectorOperation { op_type, .. } => {
                        assert!(matches!(op_type, VectorOpType::DotProduct));
                    }
                    other => panic!("Expected VectorOperation, got: {:?}", other),
                }
            }
            other => panic!("Expected PROJECT plan, got: {:?}", other),
        }
    }

    #[test]
    fn test_compile_euclidean_distance_function() {
        let compiler = Compiler::new();
        // DISTANCE/SIMILARITY with euclidean metric
        let query = r#"SELECT * FROM docs.Document
                       WHERE DISTANCE(embedding, query_vec, 'euclidean') < 2.0"#;
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::VectorOperation { op_type, .. } => {
                        assert!(matches!(op_type, VectorOpType::EuclideanDistance));
                    }
                    other => panic!("Expected VectorOperation, got: {:?}", other),
                }
            }
            other => panic!("Expected PROJECT plan, got: {:?}", other),
        }
    }

    #[test]
    fn test_regular_where_clause_not_vector() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM docs.Document WHERE title = 'test'";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should generate regular Filter plan (not VectorOperation)
        match &compiled.plan {
            ExecutionPlan::Project { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Filter { .. } => {
                        // Correct - regular filter for non-vector operations
                    }
                    ExecutionPlan::VectorOperation { .. } => {
                        panic!("Should not generate VectorOperation for non-vector WHERE clause");
                    }
                    other => panic!("Expected Filter, got: {:?}", other),
                }
            }
            other => panic!("Expected PROJECT plan, got: {:?}", other),
        }
    }

    #[test]
    fn test_regular_order_by_not_vector() {
        let compiler = Compiler::new();
        let query = "SELECT * FROM docs.Document ORDER BY created_at DESC LIMIT 10";
        let statement = parse_statement(query).unwrap();

        let compiled = compiler.compile(statement).unwrap();

        // Should generate regular Sort plan (not VectorOperation)
        match &compiled.plan {
            ExecutionPlan::Limit { input, .. } => {
                match input.as_ref() {
                    ExecutionPlan::Sort { .. } => {
                        // Correct - regular sort for non-vector ORDER BY
                    }
                    ExecutionPlan::VectorOperation { .. } => {
                        panic!("Should not generate VectorOperation for non-vector ORDER BY");
                    }
                    other => panic!("Expected Sort, got: {:?}", other),
                }
            }
            other => panic!("Expected LIMIT plan, got: {:?}", other),
        }
    }
}