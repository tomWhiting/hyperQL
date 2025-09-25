use crate::ast::*;
use crate::error::{HyperQLError, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Integer,
    Float,
    String,
    Bool,
    Point,
    EntityId,
    Timestamp,
    List(Box<TypeInfo>),
    Map,
    Null,
    Numeric,
    Unknown,
}

impl TypeInfo {
    pub fn name(&self) -> &'static str {
        match self {
            TypeInfo::Integer => "integer",
            TypeInfo::Float => "float",
            TypeInfo::String => "string",
            TypeInfo::Bool => "boolean",
            TypeInfo::Point => "point",
            TypeInfo::EntityId => "entity_id",
            TypeInfo::Timestamp => "timestamp",
            TypeInfo::List(_) => "list",
            TypeInfo::Map => "map",
            TypeInfo::Null => "null",
            TypeInfo::Numeric => "numeric",
            TypeInfo::Unknown => "unknown",
        }
    }
    
    pub fn is_numeric(&self) -> bool {
        matches!(self, TypeInfo::Integer | TypeInfo::Float | TypeInfo::Numeric)
    }

    pub fn can_be_null(&self) -> bool {
        true
    }
    
    pub fn is_comparable(&self) -> bool {
        matches!(self,
            TypeInfo::Integer | TypeInfo::Float | TypeInfo::String |
            TypeInfo::Bool | TypeInfo::Timestamp | TypeInfo::Numeric |
            TypeInfo::Unknown
        )
    }
    
    pub fn supports_equality(&self) -> bool {
        !matches!(self, TypeInfo::List(_) | TypeInfo::Map)
    }
    
    pub fn is_compatible_with(&self, other: &TypeInfo) -> bool {
        match (self, other) {
            (a, b) if a == b => true,
            (TypeInfo::Integer, TypeInfo::Float) | 
            (TypeInfo::Float, TypeInfo::Integer) => true,
            (TypeInfo::Numeric, other) | (other, TypeInfo::Numeric) => other.is_numeric(),
            (TypeInfo::Unknown, _) | (_, TypeInfo::Unknown) => true,
            (TypeInfo::Null, _) | (_, TypeInfo::Null) => true,
            _ => false,
        }
    }
    
    pub fn arithmetic_result_type(&self, other: &TypeInfo) -> Option<TypeInfo> {
        match (self, other) {
            (TypeInfo::Integer, TypeInfo::Integer) => Some(TypeInfo::Integer),
            (TypeInfo::Float, TypeInfo::Float) => Some(TypeInfo::Float),
            (TypeInfo::Integer, TypeInfo::Float) | 
            (TypeInfo::Float, TypeInfo::Integer) => Some(TypeInfo::Float),
            (TypeInfo::Numeric, other) | (other, TypeInfo::Numeric) if other.is_numeric() => {
                Some(TypeInfo::Numeric)
            },
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeContext {
    variables: HashMap<String, TypeInfo>,
    column_types: HashMap<String, TypeInfo>,
}

impl TypeContext {
    pub fn new() -> Self {
        let mut context = Self::default();
        
        context.column_types.insert("position".to_string(), TypeInfo::Point);
        context.column_types.insert("label".to_string(), TypeInfo::String);
        context.column_types.insert("name".to_string(), TypeInfo::String);
        context.column_types.insert("id".to_string(), TypeInfo::EntityId);
        context.column_types.insert("created_at".to_string(), TypeInfo::Timestamp);
        context.column_types.insert("updated_at".to_string(), TypeInfo::Timestamp);
        
        context
    }
    
    pub fn set_variable_type(&mut self, name: String, type_info: TypeInfo) {
        self.variables.insert(name, type_info);
    }
    
    pub fn get_variable_type(&self, name: &str) -> Option<&TypeInfo> {
        self.variables.get(name)
    }
    
    pub fn set_column_type(&mut self, column: String, type_info: TypeInfo) {
        self.column_types.insert(column, type_info);
    }
    
    pub fn get_column_type(&self, column: &str) -> Option<&TypeInfo> {
        self.column_types.get(column)
    }
}

pub struct TypeChecker {
    context: TypeContext,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
        }
    }
    
    pub fn with_context(context: TypeContext) -> Self {
        Self { context }
    }
    
    pub fn check_expression_type(
        &mut self,
        expr: &Expression,
    ) -> Result<TypeInfo> {
        match expr {
            Expression::Literal(literal) => self.check_literal_type(literal),
            Expression::Column(column_ref) => self.check_column_type(column_ref),
            Expression::Binary { left, op, right } => self.check_binary_expression_type(left, op, right),
            Expression::Unary { op, expr } => self.check_unary_expression_type(op, expr),
            Expression::Function { name, args } => self.check_function_type(name, args),
            Expression::Geometric(geo) => self.check_geometric_expression_type(geo),
            Expression::Vector(vec_expr) => self.check_vector_expression_type(vec_expr),
        }
    }
    
    pub fn check_operation_compatibility(
        &mut self,
        left_expr: &Expression,
        operator: &BinaryOperator,
        right_expr: &Expression,
    ) -> Result<()> {
        let left_type = self.check_expression_type(left_expr)?;
        let right_type = self.check_expression_type(right_expr)?;
        
        match operator {
            BinaryOperator::Add | BinaryOperator::Subtract | 
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                if !left_type.is_numeric() {
                    return Err(HyperQLError::TypeError {
                        expected: "numeric".to_string(),
                        found: left_type.name().to_string(),
                        context: format!("left operand of {:?}", operator),
                    });
                }
                if !right_type.is_numeric() {
                    return Err(HyperQLError::TypeError {
                        expected: "numeric".to_string(),
                        found: right_type.name().to_string(),
                        context: format!("right operand of {:?}", operator),
                    });
                }
            },
            
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => {
                if !left_type.is_comparable() {
                    return Err(HyperQLError::TypeError {
                        expected: "comparable type".to_string(),
                        found: left_type.name().to_string(),
                        context: format!("left operand of {:?}", operator),
                    });
                }
                if !right_type.is_comparable() {
                    return Err(HyperQLError::TypeError {
                        expected: "comparable type".to_string(),
                        found: right_type.name().to_string(),
                        context: format!("right operand of {:?}", operator),
                    });
                }
                if !left_type.is_compatible_with(&right_type) {
                    return Err(HyperQLError::TypeError {
                        expected: left_type.name().to_string(),
                        found: right_type.name().to_string(),
                        context: "comparison operands".to_string(),
                    });
                }
            },
            
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if !left_type.supports_equality() {
                    return Err(HyperQLError::TypeError {
                        expected: "equality-comparable type".to_string(),
                        found: left_type.name().to_string(),
                        context: format!("left operand of {:?}", operator),
                    });
                }
                if !right_type.supports_equality() {
                    return Err(HyperQLError::TypeError {
                        expected: "equality-comparable type".to_string(),
                        found: right_type.name().to_string(),
                        context: format!("right operand of {:?}", operator),
                    });
                }
            },
            
            BinaryOperator::And | BinaryOperator::Or => {
            },
            
            BinaryOperator::Like | BinaryOperator::NotLike => {
                if !matches!(left_type, TypeInfo::String | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "string".to_string(),
                        found: left_type.name().to_string(),
                        context: format!("left operand of {:?}", operator),
                    });
                }
                if !matches!(right_type, TypeInfo::String | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "string".to_string(),
                        found: right_type.name().to_string(),
                        context: format!("right operand of {:?}", operator),
                    });
                }
            },
            
            BinaryOperator::In | BinaryOperator::NotIn => {
            },
        }
        
        Ok(())
    }
    
    fn check_literal_type(&self, literal: &Literal) -> Result<TypeInfo> {
        Ok(match literal {
            Literal::String(_) => TypeInfo::String,
            Literal::Int(_) => TypeInfo::Integer,
            Literal::Float(_) => TypeInfo::Float,
            Literal::Bool(_) => TypeInfo::Bool,
            Literal::Null => TypeInfo::Null,
            Literal::EntityId(_) => TypeInfo::EntityId,
        })
    }
    
    fn check_column_type(&self, column_ref: &ColumnRef) -> Result<TypeInfo> {
        if let Some(type_info) = self.context.get_column_type(&column_ref.name) {
            Ok(type_info.clone())
        } else {
            Ok(TypeInfo::Unknown)
        }
    }
    
    fn check_binary_expression_type(
        &mut self, 
        left: &Expression,
        operator: &BinaryOperator,
        right: &Expression,
    ) -> Result<TypeInfo> {
        let left_type = self.check_expression_type(left)?;
        let right_type = self.check_expression_type(right)?;
        
        self.check_operation_compatibility(left, operator, right)?;
        
        Ok(match operator {
            BinaryOperator::Add | BinaryOperator::Subtract |
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                left_type.arithmetic_result_type(&right_type)
                    .unwrap_or(TypeInfo::Numeric)
            },
            
            BinaryOperator::Equal | BinaryOperator::NotEqual |
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual |
            BinaryOperator::Like | BinaryOperator::NotLike |
            BinaryOperator::In | BinaryOperator::NotIn => TypeInfo::Bool,
            
            BinaryOperator::And | BinaryOperator::Or => TypeInfo::Bool,
        })
    }
    
    fn check_unary_expression_type(
        &mut self, 
        operator: &UnaryOperator,
        operand: &Expression,
    ) -> Result<TypeInfo> {
        let operand_type = self.check_expression_type(operand)?;
        
        Ok(match operator {
            UnaryOperator::Not => {
                TypeInfo::Bool
            },
            UnaryOperator::Minus | UnaryOperator::Plus => {
                if !operand_type.is_numeric() {
                    return Err(HyperQLError::TypeError {
                        expected: "numeric".to_string(),
                        found: operand_type.name().to_string(),
                        context: format!("operand of unary {:?}", operator),
                    });
                }
                operand_type
            },
        })
    }
    
    pub fn check_function_type(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<TypeInfo> {
        match name.to_uppercase().as_str() {
            "COUNT" => Ok(TypeInfo::Integer),
            "SUM" => {
                if args.is_empty() {
                    return Err(HyperQLError::ValidationError {
                        message: "SUM function requires at least one argument".to_string(),
                        field: None,
                    });
                }
                let arg_type = self.check_expression_type(&args[0])?;
                if !arg_type.is_numeric() {
                    return Err(HyperQLError::TypeError {
                        expected: "numeric".to_string(),
                        found: arg_type.name().to_string(),
                        context: "SUM argument".to_string(),
                    });
                }
                Ok(arg_type)
            },
            "AVG" => {
                if args.is_empty() {
                    return Err(HyperQLError::ValidationError {
                        message: "AVG function requires at least one argument".to_string(),
                        field: None,
                    });
                }
                let arg_type = self.check_expression_type(&args[0])?;
                if !arg_type.is_numeric() {
                    return Err(HyperQLError::TypeError {
                        expected: "numeric".to_string(),
                        found: arg_type.name().to_string(),
                        context: "AVG argument".to_string(),
                    });
                }
                Ok(TypeInfo::Float)
            },
            "MIN" | "MAX" => {
                if args.is_empty() {
                    return Err(HyperQLError::ValidationError {
                        message: format!("{} function requires at least one argument", name),
                        field: None,
                    });
                }
                let arg_type = self.check_expression_type(&args[0])?;
                if !arg_type.is_comparable() {
                    return Err(HyperQLError::TypeError {
                        expected: "comparable".to_string(),
                        found: arg_type.name().to_string(),
                        context: format!("{} argument", name),
                    });
                }
                Ok(arg_type)
            },
            
            "UPPER" | "LOWER" | "TRIM" => {
                if args.len() != 1 {
                    return Err(HyperQLError::ValidationError {
                        message: format!("{} function requires exactly one argument", name),
                        field: None,
                    });
                }
                let arg_type = self.check_expression_type(&args[0])?;
                if !matches!(arg_type, TypeInfo::String | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "string".to_string(),
                        found: arg_type.name().to_string(),
                        context: format!("{} argument", name),
                    });
                }
                Ok(TypeInfo::String)
            },
            "LENGTH" => {
                if args.len() != 1 {
                    return Err(HyperQLError::ValidationError {
                        message: "LENGTH function requires exactly one argument".to_string(),
                        field: None,
                    });
                }
                let arg_type = self.check_expression_type(&args[0])?;
                if !matches!(arg_type, TypeInfo::String | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "string".to_string(),
                        found: arg_type.name().to_string(),
                        context: "LENGTH argument".to_string(),
                    });
                }
                Ok(TypeInfo::Integer)
            },
            
            "ABS" | "SQRT" | "CEIL" | "FLOOR" => {
                if args.len() != 1 {
                    return Err(HyperQLError::ValidationError {
                        message: format!("{} function requires exactly one argument", name),
                        field: None,
                    });
                }
                let arg_type = self.check_expression_type(&args[0])?;
                if !arg_type.is_numeric() {
                    return Err(HyperQLError::TypeError {
                        expected: "numeric".to_string(),
                        found: arg_type.name().to_string(),
                        context: format!("{} argument", name),
                    });
                }
                Ok(match name {
                    "CEIL" | "FLOOR" => TypeInfo::Integer,
                    _ => arg_type,
                })
            },
            
            "DISTANCE" => {
                if args.len() != 2 {
                    return Err(HyperQLError::ValidationError {
                        message: "DISTANCE function requires exactly two arguments".to_string(),
                        field: None,
                    });
                }
                for (i, arg) in args.iter().enumerate() {
                    let arg_type = self.check_expression_type(arg)?;
                    if !matches!(arg_type, TypeInfo::Point | TypeInfo::Unknown) {
                        return Err(HyperQLError::TypeError {
                            expected: "point".to_string(),
                            found: arg_type.name().to_string(),
                            context: format!("DISTANCE argument {}", i + 1),
                        });
                    }
                }
                Ok(TypeInfo::Float)
            },
            
            _ => Ok(TypeInfo::Unknown),
        }
    }
    
    fn check_geometric_expression_type(
        &mut self, 
        geo: &geometric::GeometricExpression,
    ) -> Result<TypeInfo> {
        match geo {
            geometric::GeometricExpression::Within { target, radius: _, reference } => {
                let target_type = self.check_expression_type(target)?;
                if !matches!(target_type, TypeInfo::Point | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "point".to_string(),
                        found: target_type.name().to_string(),
                        context: "WITHIN target".to_string(),
                    });
                }
                
                let reference_type = self.check_expression_type(reference)?;
                if !matches!(reference_type, TypeInfo::Point | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "point".to_string(),
                        found: reference_type.name().to_string(),
                        context: "WITHIN reference".to_string(),
                    });
                }
                
                Ok(TypeInfo::Bool)
            },
            
            geometric::GeometricExpression::Near { target, reference, max_distance: _ } => {
                let target_type = self.check_expression_type(target)?;
                if !matches!(target_type, TypeInfo::Point | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "point".to_string(),
                        found: target_type.name().to_string(),
                        context: "NEAR target".to_string(),
                    });
                }
                
                let reference_type = self.check_expression_type(reference)?;
                if !matches!(reference_type, TypeInfo::Point | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "point".to_string(),
                        found: reference_type.name().to_string(),
                        context: "NEAR reference".to_string(),
                    });
                }
                
                Ok(TypeInfo::Bool)
            },
            
            geometric::GeometricExpression::InRadius { target, center, radius: _ } => {
                let target_type = self.check_expression_type(target)?;
                if !matches!(target_type, TypeInfo::Point | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "point".to_string(),
                        found: target_type.name().to_string(),
                        context: "IN_RADIUS target".to_string(),
                    });
                }
                
                let center_type = self.check_expression_type(center)?;
                if !matches!(center_type, TypeInfo::Point | TypeInfo::Unknown) {
                    return Err(HyperQLError::TypeError {
                        expected: "point".to_string(),
                        found: center_type.name().to_string(),
                        context: "IN_RADIUS center".to_string(),
                    });
                }
                
                Ok(TypeInfo::Bool)
            },
        }
    }

    fn check_vector_expression_type(
        &mut self,
        vec_expr: &VectorExpression,
    ) -> Result<TypeInfo> {
        match vec_expr {
            VectorExpression::Similarity { reference, .. } => {
                let _reference_type = self.check_expression_type(reference)?;
                Ok(TypeInfo::Float)
            },
            
            VectorExpression::KNN { reference, .. } => {
                let _reference_type = self.check_expression_type(reference)?;
                Ok(TypeInfo::Float)
            },
        }
    }
    
    pub fn check_update_assignment(
        &mut self,
        column: &str,
        value_expr: &Expression,
    ) -> Result<()> {
        let value_type = self.check_expression_type(value_expr)?;
        
        if let Some(expected_type) = self.context.get_column_type(column) {
            if !value_type.is_compatible_with(expected_type) {
                return Err(HyperQLError::TypeError {
                    expected: expected_type.name().to_string(),
                    found: value_type.name().to_string(),
                    context: format!("assignment to column {}", column),
                });
            }
        }
        
        Ok(())
    }
    
    pub fn check_where_clause(
        &mut self,
        expr: &Expression,
    ) -> Result<()> {
        let expr_type = self.check_expression_type(expr)?;
        
        match expr_type {
            TypeInfo::Bool | TypeInfo::Unknown => Ok(()),
            _ => {
                Ok(())
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_compatibility() {
        assert!(TypeInfo::Integer.is_compatible_with(&TypeInfo::Float));
        assert!(TypeInfo::Float.is_compatible_with(&TypeInfo::Integer));
        assert!(TypeInfo::Integer.is_compatible_with(&TypeInfo::Numeric));
        assert!(TypeInfo::Unknown.is_compatible_with(&TypeInfo::String));
        assert!(!TypeInfo::String.is_compatible_with(&TypeInfo::Integer));
    }

    #[test]
    fn test_arithmetic_result_type() {
        assert_eq!(
            TypeInfo::Integer.arithmetic_result_type(&TypeInfo::Integer),
            Some(TypeInfo::Integer)
        );
        assert_eq!(
            TypeInfo::Integer.arithmetic_result_type(&TypeInfo::Float),
            Some(TypeInfo::Float)
        );
        assert_eq!(
            TypeInfo::String.arithmetic_result_type(&TypeInfo::Integer),
            None
        );
    }

    #[test]
    fn test_literal_type_checking() {
        let checker = TypeChecker::new();

        assert_eq!(
            checker.check_literal_type(&Literal::Int(42)).unwrap(),
            TypeInfo::Integer
        );
        assert_eq!(
            checker.check_literal_type(&Literal::String("test".to_string())).unwrap(),
            TypeInfo::String
        );
        assert_eq!(
            checker.check_literal_type(&Literal::Bool(true)).unwrap(),
            TypeInfo::Bool
        );
    }

    #[test]
    fn test_binary_operation_type_checking() {
        let mut checker = TypeChecker::new();

        let left = Expression::Literal(Literal::Int(5));
        let right = Expression::Literal(Literal::Float(2.5));
        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::Add, &right
        );
        assert!(result.is_ok());

        let left = Expression::Literal(Literal::String("hello".to_string()));
        let right = Expression::Literal(Literal::Int(5));
        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::Add, &right
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_comparison_type_checking() {
        let mut checker = TypeChecker::new();

        let left = Expression::Literal(Literal::Int(5));
        let right = Expression::Literal(Literal::Int(10));
        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::GreaterThan, &right
        );
        assert!(result.is_ok());

        let left = Expression::Literal(Literal::String("hello".to_string()));
        let right = Expression::Literal(Literal::Bool(true));
        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::LessThan, &right
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_function_type_checking() {
        let mut checker = TypeChecker::new();

        let result = checker.check_function_type(
            "SUM", 
            &[Expression::Literal(Literal::Int(42))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Integer);

        let result = checker.check_function_type(
            "SUM", 
            &[Expression::Literal(Literal::String("test".to_string()))]
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_string_operations() {
        let mut checker = TypeChecker::new();

        let left = Expression::Literal(Literal::String("test".to_string()));
        let right = Expression::Literal(Literal::String("%es%".to_string()));
        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::Like, &right
        );
        assert!(result.is_ok());

        let left = Expression::Literal(Literal::Int(42));
        let right = Expression::Literal(Literal::String("%test%".to_string()));
        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::Like, &right
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unary_operations() {
        let mut checker = TypeChecker::new();

        let result = checker.check_unary_expression_type(
            &UnaryOperator::Minus, 
            &Expression::Literal(Literal::Int(5))
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Integer);

        let result = checker.check_unary_expression_type(
            &UnaryOperator::Minus, 
            &Expression::Literal(Literal::String("test".to_string()))
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_assignment_checking() {
        let mut checker = TypeChecker::new();

        let result = checker.check_update_assignment(
            "name",
            &Expression::Literal(Literal::String("Alice".to_string())),
        );
        assert!(result.is_ok());

        let result = checker.check_update_assignment(
            "unknown_column",
            &Expression::Literal(Literal::String("value".to_string())),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_numeric_types() {
        let mut checker = TypeChecker::new();

        let left = Expression::Literal(Literal::Int(10));
        let right = Expression::Literal(Literal::Float(3.14));

        let result = checker.check_operation_compatibility(
            &left, &BinaryOperator::Multiply, &right
        );
        assert!(result.is_ok());

        let result_type = checker.check_binary_expression_type(
            &left, &BinaryOperator::Multiply, &right
        );
        assert!(result_type.is_ok());
        assert_eq!(result_type.unwrap(), TypeInfo::Float);
    }

    #[test]
    fn test_aggregate_functions() {
        let mut checker = TypeChecker::new();

        let result = checker.check_function_type(
            "COUNT", 
            &[Expression::Literal(Literal::String("test".to_string()))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Integer);

        let result = checker.check_function_type(
            "AVG", 
            &[Expression::Literal(Literal::Int(42))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Float);

        let result = checker.check_function_type(
            "AVG", 
            &[Expression::Literal(Literal::String("test".to_string()))]
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_math_functions() {
        let mut checker = TypeChecker::new();

        let result = checker.check_function_type(
            "ABS", 
            &[Expression::Literal(Literal::Int(-5))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Integer);

        let result = checker.check_function_type(
            "SQRT", 
            &[Expression::Literal(Literal::Float(16.0))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Float);

        let result = checker.check_function_type(
            "ABS", 
            &[Expression::Literal(Literal::String("not a number".to_string()))]
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_string_functions() {
        let mut checker = TypeChecker::new();

        let result = checker.check_function_type(
            "UPPER", 
            &[Expression::Literal(Literal::String("hello".to_string()))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::String);

        let result = checker.check_function_type(
            "LENGTH", 
            &[Expression::Literal(Literal::String("test".to_string()))]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Integer);

        let result = checker.check_function_type(
            "UPPER", 
            &[Expression::Literal(Literal::Int(42))]
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_distance_function_validation() {
        let mut checker = TypeChecker::new();

        let result = checker.check_function_type(
            "DISTANCE", 
            &[
                Expression::Column(ColumnRef {
                    table: None,
                    name: "position".to_string(),
                }),
                Expression::Column(ColumnRef {
                    table: None,
                    name: "position".to_string(),
                }),
            ]
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TypeInfo::Float);

        let result = checker.check_function_type(
            "DISTANCE", 
            &[Expression::Column(ColumnRef {
                table: None,
                name: "position".to_string(),
            })]
        );
        assert!(result.is_err());
    }
}