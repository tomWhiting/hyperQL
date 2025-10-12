use crate::compiler::{CompiledExpression, GeometricOpType};
use crate::types::{ResultRow, Value, Position3D, HyperbolicDistance};
use crate::error::{Result, HyperQLError};
use super::expression_eval::ExpressionEvaluator;
use std::collections::HashMap;

pub struct GeometricEngine {
    default_curvature: f64,
    default_weights: [f32; 3],
}

impl GeometricEngine {
    pub fn new() -> Self {
        Self {
            default_curvature: 1.0,
            default_weights: [0.4, 0.4, 0.2],
        }
    }

    #[allow(dead_code)]
    pub fn new_with_curvature(curvature: f64) -> Self {
        Self {
            default_curvature: curvature,
            default_weights: [0.4, 0.4, 0.2],
        }
    }

    pub fn execute_operation(
        &self,
        op_type: &GeometricOpType,
        params: &HashMap<String, CompiledExpression>,
        input_rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
        _global_index: Option<&std::sync::Arc<dyn std::any::Any + Send + Sync>>,
        _hnsw: Option<&std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Result<Vec<ResultRow>> {
        match op_type {
            GeometricOpType::HyperbolicDistance => {
                self.execute_hyperbolic_distance(params, input_rows, evaluator)
            }
            GeometricOpType::WithinRadius => {
                self.execute_within_radius(params, input_rows, evaluator)
            }
            GeometricOpType::NearPositions => {
                self.execute_near_positions(params, input_rows, evaluator)
            }
            GeometricOpType::GeodesicDistance => {
                self.execute_geodesic_distance(params, input_rows, evaluator)
            }
            GeometricOpType::Contains => {
                self.execute_contains(params, input_rows, evaluator)
            }
            GeometricOpType::Intersects => {
                self.execute_intersects(params, input_rows, evaluator)
            }
        }
    }

    fn execute_hyperbolic_distance(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {

        let entity1_expr = params.get("entity1")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'entity1' parameter".to_string(),
                operation: "hyperbolic_distance".to_string(),
                entity_context: None,
            })?;
        let entity2_expr = params.get("entity2")
            .ok_or_else(|| HyperQLError::ExecutionError {
                message: "Missing 'entity2' parameter".to_string(),
                operation: "hyperbolic_distance".to_string(),
                entity_context: None,
            })?;

        if rows.is_empty() {
            let dummy_row = ResultRow { columns: HashMap::new() };
            let pos1 = self.extract_position(entity1_expr, &dummy_row, evaluator)?;
            let pos2 = self.extract_position(entity2_expr, &dummy_row, evaluator)?;

            let distance = self.calculate_hyperbolic_distance(&pos1, &pos2)?;

            let mut result_columns = HashMap::new();
            result_columns.insert("distance".to_string(), Value::Distance(HyperbolicDistance(distance)));

            return Ok(vec![ResultRow { columns: result_columns }]);
        }

        let mut result_rows = Vec::new();
        for row in rows {
            let pos1 = self.extract_position(entity1_expr, &row, evaluator)?;
            let pos2 = self.extract_position(entity2_expr, &row, evaluator)?;

            let distance = self.calculate_hyperbolic_distance(&pos1, &pos2)?;

            let mut result_columns = row.columns.clone();
            result_columns.insert("distance".to_string(), Value::Distance(HyperbolicDistance(distance)));

            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_within_radius(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        if rows.is_empty() {
            return Err(HyperQLError::ExecutionError { message: "within_radius requires input rows".to_string(), operation: "geometric_operation".to_string(), entity_context: None });
        }

        let center_expr = params.get("center")
            .ok_or_else(|| HyperQLError::ExecutionError { message: "Missing 'center' parameter for within_radius".to_string(), operation: "geometric_operation".to_string(), entity_context: None })?;
        let radius_expr = params.get("radius")
            .ok_or_else(|| HyperQLError::ExecutionError { message: "Missing 'radius' parameter for within_radius".to_string(), operation: "geometric_operation".to_string(), entity_context: None })?;

        let dummy_row = ResultRow { columns: HashMap::new() };
        let center_pos = self.extract_position(center_expr, &dummy_row, evaluator)?;

        let radius_value = evaluator.evaluate_expression(radius_expr, &dummy_row)?;
        let radius = match radius_value {
            Value::Float(f) => f,
            Value::Int(i) => i as f64,
            Value::Distance(HyperbolicDistance(d)) => d,
            _ => return Err(HyperQLError::ExecutionError { message: "Radius must be a numeric value".to_string(), operation: "geometric_operation".to_string(), entity_context: None }),
        };

        let mut filtered_rows = Vec::new();
        for row in rows {
            if let Ok(entity_pos) = self.extract_position_from_row(&row) {
                let distance = self.calculate_hyperbolic_distance(&center_pos, &entity_pos)?;
                if distance <= radius {
                    let mut result_columns = row.columns.clone();
                    result_columns.insert("distance_from_center".to_string(), Value::Distance(HyperbolicDistance(distance)));
                    filtered_rows.push(ResultRow { columns: result_columns });
                }
            }
        }

        Ok(filtered_rows)
    }

    fn execute_near_positions(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        if rows.is_empty() {
            return Err(HyperQLError::ExecutionError { message: "near_positions requires input rows".to_string(), operation: "geometric_operation".to_string(), entity_context: None });
        }

        let reference_expr = params.get("reference")
            .ok_or_else(|| HyperQLError::ExecutionError { message: "Missing 'reference' parameter for near_positions".to_string(), operation: "geometric_operation".to_string(), entity_context: None })?;
        let limit_expr = params.get("limit");

        let dummy_row = ResultRow { columns: HashMap::new() };
        let reference_pos = self.extract_position(reference_expr, &dummy_row, evaluator)?;

        let limit = if let Some(expr) = limit_expr {
            match evaluator.evaluate_expression(expr, &dummy_row)? {
                Value::Int(i) => Some(i as usize),
                _ => None,
            }
        } else {
            None
        };

        let mut distances: Vec<(usize, f64)> = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            if let Ok(entity_pos) = self.extract_position_from_row(row) {
                let distance = self.calculate_hyperbolic_distance(&reference_pos, &entity_pos)?;
                distances.push((idx, distance));
            }
        }

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let take_count = limit.unwrap_or(distances.len());
        let mut result_rows = Vec::new();

        for (idx, distance) in distances.into_iter().take(take_count) {
            let mut result_columns = rows[idx].columns.clone();
            result_columns.insert("distance".to_string(), Value::Distance(HyperbolicDistance(distance)));
            result_rows.push(ResultRow { columns: result_columns });
        }

        Ok(result_rows)
    }

    fn execute_geodesic_distance(
        &self,
        params: &HashMap<String, CompiledExpression>,
        rows: Vec<ResultRow>,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        self.execute_hyperbolic_distance(params, rows, evaluator)
    }

    fn execute_contains(
        &self,
        _params: &HashMap<String, CompiledExpression>,
        _rows: Vec<ResultRow>,
        _evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        Err(HyperQLError::ExecutionError { message: "Contains operation not yet implemented".to_string(), operation: "geometric_operation".to_string(), entity_context: None })
    }

    fn execute_intersects(
        &self,
        _params: &HashMap<String, CompiledExpression>,
        _rows: Vec<ResultRow>,
        _evaluator: &mut ExpressionEvaluator,
    ) -> Result<Vec<ResultRow>> {
        Err(HyperQLError::ExecutionError { message: "Intersects operation not yet implemented".to_string(), operation: "geometric_operation".to_string(), entity_context: None })
    }

    fn extract_position(
        &self,
        expr: &CompiledExpression,
        row: &ResultRow,
        evaluator: &mut ExpressionEvaluator,
    ) -> Result<Position3D> {
        let value = evaluator.evaluate_expression(expr, row)?;
        match value {
            Value::Position(pos) => Ok(pos),
            Value::EntityId(id) => {
                Err(HyperQLError::ExecutionError {
                    message: "Entity does not have a position. Position learning must be run first.".to_string(),
                    operation: "extract_position".to_string(),
                    entity_context: Some(id.0),
                })
            }
            _ => Err(HyperQLError::ExecutionError { message: "Expected position or entity with position".to_string(), operation: "geometric_operation".to_string(), entity_context: None }),
        }
    }

    fn extract_position_from_row(&self, row: &ResultRow) -> Result<Position3D> {
        let x = row.columns.get("x")
            .and_then(|v| if let Value::Float(f) = v { Some(*f) } else { None });
        let y = row.columns.get("y")
            .and_then(|v| if let Value::Float(f) = v { Some(*f) } else { None });
        let z = row.columns.get("z")
            .and_then(|v| if let Value::Float(f) = v { Some(*f) } else { None });

        match (x, y, z) {
            (Some(x), Some(y), Some(z)) => Ok(Position3D { x, y, z }),
            _ => {
                if let Some(Value::Position(pos)) = row.columns.get("position") {
                    Ok(pos.clone())
                } else {
                    Err(HyperQLError::ExecutionError {
                        message: "Row does not contain position data (x, y, z columns or position field)".to_string(),
                        operation: "extract_position_from_row".to_string(),
                        entity_context: None,
                    })
                }
            }
        }
    }

    fn calculate_hyperbolic_distance(&self, pos1: &Position3D, pos2: &Position3D) -> Result<f64> {
        let curvature = self.default_curvature;
        
        let inner = self.minkowski_inner_from_pos(pos1, pos2);
        
        let sqrt_c = curvature.sqrt();
        let arg = (-inner / curvature).max(1.0 + 1e-5);
        
        let distance = sqrt_c * arg.acosh();
        
        Ok(distance)
    }

    fn minkowski_inner_from_pos(&self, pos1: &Position3D, pos2: &Position3D) -> f64 {
        -pos1.x * pos2.x + pos1.y * pos2.y + pos1.z * pos2.z
    }
}

impl Default for GeometricEngine {
    fn default() -> Self {
        Self::new()
    }
}
