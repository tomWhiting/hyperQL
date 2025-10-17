use crate::compiler::CompiledExpression;
use crate::types::{ResultRow, Value};
use crate::error::{HyperQLError, Result};
use std::cmp::Ordering;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGEX_CACHE: Mutex<HashMap<String, Regex>> = Mutex::new(HashMap::new());
}

pub struct ExpressionEvaluator {
}

impl ExpressionEvaluator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn evaluate_predicate(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<bool> {
        let value = self.evaluate_expression(expr, row)?;
        
        match value {
            Value::Bool(b) => Ok(b),
            Value::Null => Ok(false),
            _ => Err(HyperQLError::simple_parse_error(
                "WHERE clause must evaluate to boolean",
                "",
                1,
                1,
            ))
        }
    }

    pub fn evaluate_having_predicate(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<bool> {
        self.evaluate_predicate(expr, row)
    }

    pub fn evaluate_expression(&self, expr: &CompiledExpression, row: &ResultRow) -> Result<Value> {
        match expr {
            CompiledExpression::Literal(value) => Ok(value.clone()),
            CompiledExpression::Column { name, table, .. } => {
                if name == "*" {
                    return Err(HyperQLError::simple_parse_error(
                        "Wildcard column '*' should be expanded at compile time, not evaluated at runtime",
                        "",
                        1,
                        1,
                    ));
                }

                // Try different column name formats to handle both qualified and unqualified names
                // 1. Try table-prefixed name (e.g., "p_age" for alias "p" and column "age")
                // 2. Try bare column name (e.g., "age")
                // 3. Try dot-qualified name (e.g., "p.age" - legacy format)
                let lookup_keys = if let Some(table_name) = table {
                    vec![
                        format!("{}_{}", table_name, name),  // Alias prefix format: "p_age"
                        name.clone(),                         // Bare name: "age"
                        format!("{}.{}", table_name, name),  // Dot format: "p.age"
                    ]
                } else {
                    vec![name.clone()]
                };

                for key in &lookup_keys {
                    if let Some(value) = row.columns.get(key) {
                        return Ok(value.clone());
                    }
                }

                Err(HyperQLError::simple_parse_error(
                    &format!("Column '{}' not found", name),
                    "",
                    1,
                    1,
                ))
            },
            CompiledExpression::Binary { left, op, right, .. } => {
                let left_val = self.evaluate_expression(left, row)?;
                let right_val = self.evaluate_expression(right, row)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            },
            CompiledExpression::Unary { op, expr, .. } => {
                let val = self.evaluate_expression(expr, row)?;
                self.evaluate_unary_op(op, &val)
            },
            CompiledExpression::Function { name, args, .. } => {
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.evaluate_expression(arg, row)?);
                }
                self.evaluate_function(name, &arg_values)
            },
        }
    }

    pub fn evaluate_literal(&self, expr: &CompiledExpression) -> Result<Value> {
        match expr {
            CompiledExpression::Literal(value) => Ok(value.clone()),
            _ => Err(HyperQLError::simple_parse_error(
                "Expected literal expression",
                "",
                1,
                1,
            ))
        }
    }

    fn evaluate_binary_op(&self, left: &Value, op: &crate::ast::BinaryOperator, right: &Value) -> Result<Value> {
        use crate::ast::BinaryOperator;
        
        match (left, op, right) {
            (Value::Int(a), BinaryOperator::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinaryOperator::Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), BinaryOperator::Add, Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Float(a), BinaryOperator::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            
            (Value::Int(a), BinaryOperator::Subtract, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinaryOperator::Subtract, Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), BinaryOperator::Subtract, Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            (Value::Float(a), BinaryOperator::Subtract, Value::Float(b)) => Ok(Value::Float(a - b)),
            
            (Value::Int(a), BinaryOperator::Multiply, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), BinaryOperator::Multiply, Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), BinaryOperator::Multiply, Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::Float(a), BinaryOperator::Multiply, Value::Float(b)) => Ok(Value::Float(a * b)),
            
            (Value::Int(a), BinaryOperator::Divide, Value::Int(b)) => {
                if *b == 0 {
                    Err(HyperQLError::simple_parse_error("Division by zero", "", 1, 1))
                } else {
                    Ok(Value::Float(*a as f64 / *b as f64))
                }
            },
            (Value::Float(a), BinaryOperator::Divide, Value::Float(b)) => {
                if *b == 0.0 {
                    Err(HyperQLError::simple_parse_error("Division by zero", "", 1, 1))
                } else {
                    Ok(Value::Float(a / b))
                }
            },
            
            (Value::Int(a), BinaryOperator::Equal, Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Float(a), BinaryOperator::Equal, Value::Float(b)) => Ok(Value::Bool((a - b).abs() < f64::EPSILON)),
            (Value::String(a), BinaryOperator::Equal, Value::String(b)) => Ok(Value::Bool(a == b)),
            (Value::Bool(a), BinaryOperator::Equal, Value::Bool(b)) => Ok(Value::Bool(a == b)),
            
            (Value::Int(a), BinaryOperator::NotEqual, Value::Int(b)) => Ok(Value::Bool(a != b)),
            (Value::Float(a), BinaryOperator::NotEqual, Value::Float(b)) => Ok(Value::Bool((a - b).abs() >= f64::EPSILON)),
            (Value::String(a), BinaryOperator::NotEqual, Value::String(b)) => Ok(Value::Bool(a != b)),
            (Value::Bool(a), BinaryOperator::NotEqual, Value::Bool(b)) => Ok(Value::Bool(a != b)),
            
            (Value::Int(a), BinaryOperator::LessThan, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), BinaryOperator::LessThan, Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::String(a), BinaryOperator::LessThan, Value::String(b)) => Ok(Value::Bool(a < b)),
            
            (Value::Int(a), BinaryOperator::LessThanOrEqual, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Float(a), BinaryOperator::LessThanOrEqual, Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Value::String(a), BinaryOperator::LessThanOrEqual, Value::String(b)) => Ok(Value::Bool(a <= b)),
            
            (Value::Int(a), BinaryOperator::GreaterThan, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Float(a), BinaryOperator::GreaterThan, Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Value::String(a), BinaryOperator::GreaterThan, Value::String(b)) => Ok(Value::Bool(a > b)),
            
            (Value::Int(a), BinaryOperator::GreaterThanOrEqual, Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (Value::Float(a), BinaryOperator::GreaterThanOrEqual, Value::Float(b)) => Ok(Value::Bool(a >= b)),
            (Value::String(a), BinaryOperator::GreaterThanOrEqual, Value::String(b)) => Ok(Value::Bool(a >= b)),
            
            (Value::Bool(a), BinaryOperator::And, Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), BinaryOperator::Or, Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

            (left_val, BinaryOperator::Like, Value::String(pattern)) => {
                self.evaluate_like(left_val, pattern, false)
            },
            (left_val, BinaryOperator::NotLike, Value::String(pattern)) => {
                self.evaluate_like(left_val, pattern, true)
            },

            (left_val, BinaryOperator::In, right_val) => {
                self.evaluate_in(left_val, right_val, false)
            },
            (left_val, BinaryOperator::NotIn, right_val) => {
                self.evaluate_in(left_val, right_val, true)
            },

            _ => Err(HyperQLError::simple_parse_error(
                &format!("Cannot apply operator {:?} to {:?} and {:?}", op, left, right),
                "",
                1,
                1,
            ))
        }
    }

    fn evaluate_unary_op(&self, op: &crate::ast::UnaryOperator, val: &Value) -> Result<Value> {
        use crate::ast::UnaryOperator;

        match (op, val) {
            (UnaryOperator::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            (UnaryOperator::Minus, Value::Int(i)) => Ok(Value::Int(-i)),
            (UnaryOperator::Minus, Value::Float(f)) => Ok(Value::Float(-f)),
            (UnaryOperator::Plus, Value::Int(i)) => Ok(Value::Int(*i)),
            (UnaryOperator::Plus, Value::Float(f)) => Ok(Value::Float(*f)),
            (UnaryOperator::IsNull, Value::Null) => Ok(Value::Bool(true)),
            (UnaryOperator::IsNull, _) => Ok(Value::Bool(false)),
            (UnaryOperator::IsNotNull, Value::Null) => Ok(Value::Bool(false)),
            (UnaryOperator::IsNotNull, _) => Ok(Value::Bool(true)),
            _ => Err(HyperQLError::simple_parse_error(
                &format!("Cannot apply unary operator {:?} to {:?}", op, val),
                "",
                1,
                1,
            ))
        }
    }

    fn evaluate_function(&self, name: &str, args: &[Value]) -> Result<Value> {
        match name {
            "__IN_LIST__" => {
                Ok(Value::List(args.to_vec()))
            },
            _ => match name.to_uppercase().as_str() {
                "COUNT" => {
                    if args.is_empty() {
                        Ok(Value::Int(1))
                    } else {
                        Ok(Value::Int(args.len() as i64))
                    }
                },
            "SUM" => {
                if args.is_empty() {
                    Ok(Value::Int(0))
                } else {
                    let mut sum = 0.0;
                    for arg in args {
                        match arg {
                            Value::Int(i) => sum += *i as f64,
                            Value::Float(f) => sum += f,
                            _ => return Err(HyperQLError::simple_parse_error(
                                "SUM function requires numeric arguments",
                                "",
                                1,
                                1,
                            ))
                        }
                    }
                    if sum.fract() == 0.0 {
                        Ok(Value::Int(sum as i64))
                    } else {
                        Ok(Value::Float(sum))
                    }
                }
            },
            "AVG" => {
                if args.is_empty() {
                    Ok(Value::Null)
                } else {
                    let mut sum = 0.0;
                    for arg in args {
                        match arg {
                            Value::Int(i) => sum += *i as f64,
                            Value::Float(f) => sum += f,
                            _ => return Err(HyperQLError::simple_parse_error(
                                "AVG function requires numeric arguments",
                                "",
                                1,
                                1,
                            ))
                        }
                    }
                    Ok(Value::Float(sum / args.len() as f64))
                }
            },
            "MIN" => {
                if args.is_empty() {
                    Ok(Value::Null)
                } else {
                    let mut min_val = &args[0];
                    for arg in &args[1..] {
                        if self.compare_values(arg, min_val) == Ordering::Less {
                            min_val = arg;
                        }
                    }
                    Ok(min_val.clone())
                }
            },
                "MAX" => {
                    if args.is_empty() {
                        Ok(Value::Null)
                    } else {
                        let mut max_val = &args[0];
                        for arg in &args[1..] {
                            if self.compare_values(arg, max_val) == Ordering::Greater {
                                max_val = arg;
                            }
                        }
                        Ok(max_val.clone())
                    }
                },
                _ => Err(HyperQLError::simple_parse_error(
                    &format!("Unknown function: {}", name),
                    "",
                    1,
                    1,
                ))
            }
        }
    }

    pub fn compare_values(&self, a: &Value, b: &Value) -> Ordering {
        match (a, b) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }

    fn evaluate_like(&self, value: &Value, pattern: &str, negated: bool) -> Result<Value> {
        let text = match value {
            Value::String(s) => s.as_str(),
            Value::Null => return Ok(Value::Bool(negated)),
            _ => return Err(HyperQLError::simple_parse_error(
                "LIKE operator requires string value",
                "",
                1,
                1,
            )),
        };

        let regex_pattern = sql_pattern_to_regex(pattern);
        let regex = get_cached_regex(&regex_pattern)?;
        let matches = regex.is_match(text);

        Ok(Value::Bool(if negated { !matches } else { matches }))
    }

    fn evaluate_in(&self, value: &Value, list: &Value, negated: bool) -> Result<Value> {
        if matches!(value, Value::Null) {
            return Ok(Value::Bool(false));
        }

        let contains = match list {
            Value::List(items) => {
                items.iter().any(|item| values_equal(value, item))
            },
            _ => return Err(HyperQLError::simple_parse_error(
                "IN operator requires list value",
                "",
                1,
                1,
            )),
        };

        Ok(Value::Bool(if negated { !contains } else { contains }))
    }
}

fn sql_pattern_to_regex(pattern: &str) -> String {
    let mut regex = String::from("^");

    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '%' => regex.push_str(".*"),
            '_' => regex.push('.'),
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    chars.next();
                    regex.push_str(&regex::escape(&next_ch.to_string()));
                } else {
                    regex.push_str(r"\\");
                }
            },
            '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' => {
                regex.push('\\');
                regex.push(ch);
            },
            _ => regex.push(ch),
        }
    }

    regex.push('$');
    regex
}

fn get_cached_regex(pattern: &str) -> Result<Regex> {
    let mut cache = REGEX_CACHE.lock().unwrap();

    if let Some(regex) = cache.get(pattern) {
        return Ok(regex.clone());
    }

    let regex = Regex::new(pattern).map_err(|e| {
        HyperQLError::simple_parse_error(
            &format!("Invalid LIKE pattern: {}", e),
            "",
            1,
            1,
        )
    })?;

    cache.insert(pattern.to_string(), regex.clone());
    Ok(regex)
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
        (Value::Int(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
        (Value::Float(a), Value::Int(b)) => (a - *b as f64).abs() < f64::EPSILON,
        (Value::String(a), Value::String(b)) => a == b,
        _ => false,
    }
}
