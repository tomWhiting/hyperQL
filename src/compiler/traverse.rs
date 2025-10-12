use crate::error::*;

use super::expression::ExpressionCompiler;
use super::{CompiledTraversePattern, CompiledNodePattern, CompiledRelationshipPattern};

#[allow(dead_code)]
pub struct TraverseCompiler {
    expression_compiler: ExpressionCompiler,
}

#[allow(dead_code)]
impl TraverseCompiler {
    pub fn new() -> Self {
        Self {
            expression_compiler: ExpressionCompiler::new(),
        }
    }

    pub fn compile_traverse_patterns(&self, patterns: &[crate::ast::TraversePattern]) -> Result<Vec<CompiledTraversePattern>> {
        let mut compiled_patterns = Vec::new();

        for pattern in patterns {
            let compiled_start_node = CompiledNodePattern {
                variable: pattern.start_node.variable.clone(),
                label: pattern.start_node.label.clone(),
                properties: if let Some(props) = &pattern.start_node.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
                } else {
                    None
                },
            };

            let compiled_end_node = CompiledNodePattern {
                variable: pattern.end_node.variable.clone(),
                label: pattern.end_node.label.clone(),
                properties: if let Some(props) = &pattern.end_node.properties {
                    Some(self.expression_compiler.compile_expression(props.clone())?)
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
                    Some(self.expression_compiler.compile_expression(props.clone())?)
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
}

impl Default for TraverseCompiler {
    fn default() -> Self {
        Self::new()
    }
}
