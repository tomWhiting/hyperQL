use crate::ast::*;
use crate::error::*;

use super::expression::ExpressionCompiler;
use super::{CompiledAssignment, ExecutionPlan};

pub struct StatementCompiler {
    expression_compiler: ExpressionCompiler,
}

impl StatementCompiler {
    pub fn new() -> Self {
        Self {
            expression_compiler: ExpressionCompiler::new(),
        }
    }

    pub fn compile_insert(&self, insert: InsertStatement) -> Result<ExecutionPlan> {
        let mut compiled_values = Vec::new();

        for value_row in insert.values {
            let mut compiled_row = Vec::new();
            for value_expr in value_row {
                let compiled_expr = self.expression_compiler.compile_expression(value_expr)?;
                compiled_row.push(compiled_expr);
            }
            compiled_values.push(compiled_row);
        }

        Ok(ExecutionPlan::Insert {
            table: insert.table.clone(),
            columns: insert.columns,
            values: compiled_values,
        })
    }

    pub fn compile_update(&self, update: UpdateStatement) -> Result<ExecutionPlan> {
        let mut compiled_assignments = Vec::new();

        for assignment in update.assignments {
            let compiled_value = self.expression_compiler.compile_expression(assignment.value)?;
            compiled_assignments.push(CompiledAssignment {
                column: assignment.column,
                value: compiled_value,
            });
        }

        let compiled_filter = if let Some(where_expr) = update.where_clause {
            Some(self.expression_compiler.compile_expression(where_expr)?)
        } else {
            None
        };

        Ok(ExecutionPlan::Update {
            table: update.table.clone(),
            assignments: compiled_assignments,
            filter: compiled_filter,
        })
    }

    pub fn compile_delete(&self, delete: DeleteStatement) -> Result<ExecutionPlan> {
        let compiled_filter = if let Some(where_expr) = delete.where_clause {
            Some(self.expression_compiler.compile_expression(where_expr)?)
        } else {
            None
        };

        Ok(ExecutionPlan::Delete {
            table: delete.table.clone(),
            filter: compiled_filter,
        })
    }
}

impl Default for StatementCompiler {
    fn default() -> Self {
        Self::new()
    }
}
