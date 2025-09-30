use crate::compiler::{CompiledExpression, VectorOpType};
use crate::types::{ResultRow, Value, Vector};
use crate::error::{Result, HyperQLError};
use super::expression_eval::ExpressionEvaluator;
use std::collections::HashMap;

pub struct VectorEngine;

impl VectorEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_operation(
        &self,
        op_type: &VectorOpType,
        params: &HashMap<String, CompiledExpression>,
        input_rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        match op_type {
            VectorOpType::CosineSimilarity => {
                self.execute_cosine_similarity(params, input_rows, evaluator)
            }
            VectorOpType::EuclideanDistance => {
                self.execute_euclidean_distance(params, input_rows, evaluator)
            }
            VectorOpType::DotProduct => {
                self.execute_dot_product(params, input_rows, evaluator)
            }
            VectorOpType::Normalize => {
                self.execute_normalize(params, input_rows, evaluator)
            }
            VectorOpType::KNN => {
                self.execute_knn(params, input_rows, evaluator)
            }
            VectorOpType::SimilaritySearch => {
                self.execute_similarity_search(params, input_rows, evaluator)
            }
        }
    }

    fn execute_cosine_similarity(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        let vector1_expr = params.get("vector1")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector1' parameter".to_string(),
                operation: "cosine_similarity".to_string(),
                entity_context: None,
            })?;
        let vector2_expr = params.get("vector2")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector2' parameter".to_string(),
                operation: "cosine_similarity".to_string(),
                entity_context: None,
            })?;

        if rows.is_empty() {
            let dummy_row = ResultRow { columns: HashMap::new() };
            let vec1 = self.extract_vector(vector1_expr, &dummy_row, evaluator)?;
            let vec2 = self.extract_vector(vector2_expr, &dummy_row, evaluator)?;

            let similarity = self.cosine_similarity(&vec1, &vec2)?;

            let mut result_columns = HashMap::new();
            result_columns.insert("similarity".to_string(), Value::Float(similarity));

            return Ok(vec![ResultRow { columns: result_columns }]);
        }

        let mut result_rows = Vec::new();
        for row in rows {
            let vec1 = self.extract_vector(vector1_expr, &row, evaluator)?;
            let vec2 = self.extract_vector(vector2_expr, &row, evaluator)?;

            let similarity = self.cosine_similarity(&vec1, &vec2)?;

            let mut result_columns = row.columns.clone();
            result_columns.insert("similarity".to_string(), Value::Float(similarity));

            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_euclidean_distance(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        let vector1_expr = params.get("vector1")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector1' parameter".to_string(),
                operation: "euclidean_distance".to_string(),
                entity_context: None,
            })?;
        let vector2_expr = params.get("vector2")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector2' parameter".to_string(),
                operation: "euclidean_distance".to_string(),
                entity_context: None,
            })?;

        if rows.is_empty() {
            let dummy_row = ResultRow { columns: HashMap::new() };
            let vec1 = self.extract_vector(vector1_expr, &dummy_row, evaluator)?;
            let vec2 = self.extract_vector(vector2_expr, &dummy_row, evaluator)?;

            let distance = self.euclidean_distance(&vec1, &vec2)?;

            let mut result_columns = HashMap::new();
            result_columns.insert("distance".to_string(), Value::Float(distance));

            return Ok(vec![ResultRow { columns: result_columns }]);
        }

        let mut result_rows = Vec::new();
        for row in rows {
            let vec1 = self.extract_vector(vector1_expr, &row, evaluator)?;
            let vec2 = self.extract_vector(vector2_expr, &row, evaluator)?;

            let distance = self.euclidean_distance(&vec1, &vec2)?;

            let mut result_columns = row.columns.clone();
            result_columns.insert("distance".to_string(), Value::Float(distance));

            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_dot_product(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        let vector1_expr = params.get("vector1")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector1' parameter".to_string(),
                operation: "dot_product".to_string(),
                entity_context: None,
            })?;
        let vector2_expr = params.get("vector2")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector2' parameter".to_string(),
                operation: "dot_product".to_string(),
                entity_context: None,
            })?;

        if rows.is_empty() {
            let dummy_row = ResultRow { columns: HashMap::new() };
            let vec1 = self.extract_vector(vector1_expr, &dummy_row, evaluator)?;
            let vec2 = self.extract_vector(vector2_expr, &dummy_row, evaluator)?;

            let dot_prod = self.dot_product(&vec1, &vec2)?;

            let mut result_columns = HashMap::new();
            result_columns.insert("dot_product".to_string(), Value::Float(dot_prod));

            return Ok(vec![ResultRow { columns: result_columns }]);
        }

        let mut result_rows = Vec::new();
        for row in rows {
            let vec1 = self.extract_vector(vector1_expr, &row, evaluator)?;
            let vec2 = self.extract_vector(vector2_expr, &row, evaluator)?;

            let dot_prod = self.dot_product(&vec1, &vec2)?;

            let mut result_columns = row.columns.clone();
            result_columns.insert("dot_product".to_string(), Value::Float(dot_prod));

            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_normalize(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        let vector_expr = params.get("vector")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'vector' parameter".to_string(),
                operation: "normalize".to_string(),
                entity_context: None,
            })?;

        if rows.is_empty() {
            let dummy_row = ResultRow { columns: HashMap::new() };
            let vec = self.extract_vector(vector_expr, &dummy_row, evaluator)?;
            let normalized = self.normalize(&vec)?;

            let mut result_columns = HashMap::new();
            result_columns.insert("normalized_vector".to_string(), Value::Vector(normalized));

            return Ok(vec![ResultRow { columns: result_columns }]);
        }

        let mut result_rows = Vec::new();
        for row in rows {
            let vec = self.extract_vector(vector_expr, &row, evaluator)?;
            let normalized = self.normalize(&vec)?;

            let mut result_columns = row.columns.clone();
            result_columns.insert("normalized_vector".to_string(), Value::Vector(normalized));

            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_knn(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        if rows.is_empty() {
            return Err(HyperQLError::ExecutionError {
                message: "KNN requires input rows".to_string(),
                operation: "knn".to_string(),
                entity_context: None,
            });
        }

        let query_vector_expr = params.get("query_vector")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'query_vector' parameter".to_string(),
                operation: "knn".to_string(),
                entity_context: None,
            })?;
        let k_expr = params.get("k")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'k' parameter".to_string(),
                operation: "knn".to_string(),
                entity_context: None,
            })?;

        let dummy_row = ResultRow { columns: HashMap::new() };
        let query_vector = self.extract_vector(query_vector_expr, &dummy_row, evaluator)?;

        let k_value = evaluator.evaluate_expression(k_expr, &dummy_row)?;
        let k = match k_value {
            Value::Int(i) => i as usize,
            _ => return Err(HyperQLError::ExecutionError {
                message: "k must be an integer".to_string(),
                operation: "knn".to_string(),
                entity_context: None,
            }),
        };

        let mut distances: Vec<(usize, f64)> = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if let Ok(entity_vector) = self.extract_vector_from_row(row) {
                let distance = self.euclidean_distance(&query_vector, &entity_vector)?;
                distances.push((idx, distance));
            }
        }

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut result_rows = Vec::new();
        for (idx, distance) in distances.into_iter().take(k) {
            let mut result_columns = rows[idx].columns.clone();
            result_columns.insert("distance".to_string(), Value::Float(distance));
            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_similarity_search(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        if rows.is_empty() {
            return Err(HyperQLError::ExecutionError {
                message: "Similarity search requires input rows".to_string(),
                operation: "similarity_search".to_string(),
                entity_context: None,
            });
        }

        let query_vector_expr = params.get("query_vector")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'query_vector' parameter".to_string(),
                operation: "similarity_search".to_string(),
                entity_context: None,
            })?;
        let threshold_expr = params.get("threshold");

        let dummy_row = ResultRow { columns: HashMap::new() };
        let query_vector = self.extract_vector(query_vector_expr, &dummy_row, evaluator)?;

        let threshold = if let Some(expr) = threshold_expr {
            match evaluator.evaluate_expression(expr, &dummy_row)? {
                Value::Float(f) => Some(f),
                _ => None,
            }
        } else {
            None
        };

        let mut similarities: Vec<(usize, f64)> = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if let Ok(entity_vector) = self.extract_vector_from_row(row) {
                let similarity = self.cosine_similarity(&query_vector, &entity_vector)?;
                similarities.push((idx, similarity));
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut result_rows = Vec::new();
        for (idx, similarity) in similarities {
            if let Some(threshold_val) = threshold {
                if similarity < threshold_val {
                    continue;
                }
            }

            let mut result_columns = rows[idx].columns.clone();
            result_columns.insert("similarity".to_string(), Value::Float(similarity));
            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn extract_vector(
        &self,
        expr: &CompiledExpression,
        row: &ResultRow,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vector> {
        let value = evaluator.evaluate_expression(expr, row)?;
        match value {
            Value::Vector(vec) => Ok(vec),
            Value::EntityId(id) => {
                Err(HyperQLError::ExecutionError {
                    message: "Entity does not have an embedding vector".to_string(),
                    operation: "extract_vector".to_string(),
                    entity_context: Some(id.0),
                })
            }
            _ => Err(HyperQLError::ExecutionError {
                message: "Expected vector or entity with embedding".to_string(),
                operation: "extract_vector".to_string(),
                entity_context: None,
            }),
        }
    }

    fn extract_vector_from_row(&self, row: &ResultRow) -> Result<Vector> {
        if let Some(Value::Vector(vec)) = row.columns.get("embedding") {
            Ok(vec.clone())
        } else if let Some(Value::Vector(vec)) = row.columns.get("vector") {
            Ok(vec.clone())
        } else {
            Err(HyperQLError::ExecutionError {
                message: "Row does not contain vector data (embedding or vector field)".to_string(),
                operation: "extract_vector_from_row".to_string(),
                entity_context: None,
            })
        }
    }

    fn dot_product(&self, vec1: &Vector, vec2: &Vector) -> Result<f64> {
        if vec1.dimensions.len() != vec2.dimensions.len() {
            return Err(HyperQLError::ExecutionError {
                message: format!("Vector dimension mismatch: {} vs {}", vec1.dimensions.len(), vec2.dimensions.len()),
                operation: "dot_product".to_string(),
                entity_context: None,
            });
        }

        let dot_prod = vec1.dimensions.iter()
            .zip(vec2.dimensions.iter())
            .map(|(a, b)| a * b)
            .sum();

        Ok(dot_prod)
    }

    fn magnitude(&self, vec: &Vector) -> f64 {
        vec.dimensions.iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt()
    }

    fn cosine_similarity(&self, vec1: &Vector, vec2: &Vector) -> Result<f64> {
        let dot_prod = self.dot_product(vec1, vec2)?;
        let mag1 = self.magnitude(vec1);
        let mag2 = self.magnitude(vec2);

        if mag1 == 0.0 || mag2 == 0.0 {
            return Err(HyperQLError::ExecutionError {
                message: "Cannot compute cosine similarity for zero vectors".to_string(),
                operation: "cosine_similarity".to_string(),
                entity_context: None,
            });
        }

        Ok(dot_prod / (mag1 * mag2))
    }

    fn euclidean_distance(&self, vec1: &Vector, vec2: &Vector) -> Result<f64> {
        if vec1.dimensions.len() != vec2.dimensions.len() {
            return Err(HyperQLError::ExecutionError {
                message: format!("Vector dimension mismatch: {} vs {}", vec1.dimensions.len(), vec2.dimensions.len()),
                operation: "euclidean_distance".to_string(),
                entity_context: None,
            });
        }

        let distance = vec1.dimensions.iter()
            .zip(vec2.dimensions.iter())
            .map(|(a, b)| {
                let diff = a - b;
                diff * diff
            })
            .sum::<f64>()
            .sqrt();

        Ok(distance)
    }

    fn normalize(&self, vec: &Vector) -> Result<Vector> {
        let mag = self.magnitude(vec);

        if mag == 0.0 {
            return Err(HyperQLError::ExecutionError {
                message: "Cannot normalize zero vector".to_string(),
                operation: "normalize".to_string(),
                entity_context: None,
            });
        }

        let normalized_dimensions = vec.dimensions.iter()
            .map(|x| x / mag)
            .collect();

        Ok(Vector {
            dimensions: normalized_dimensions,
        })
    }
}

impl Default for VectorEngine {
    fn default() -> Self {
        Self::new()
    }
}
