use crate::compiler::{CompiledExpression, CompiledAssignment};
use crate::types::Entity;
use crate::error::Result;
use super::{DataSource, TableSchema, ColumnType};
use std::collections::HashMap;

pub struct MemoryDataSource {
    entities: HashMap<String, Vec<Entity>>,
}

impl MemoryDataSource {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn add_entity(&mut self, table: &str, entity: Entity) {
        self.entities.entry(table.to_string()).or_insert_with(Vec::new).push(entity);
    }

    pub fn add_entities(&mut self, table: &str, entities: Vec<Entity>) {
        self.entities.entry(table.to_string()).or_insert_with(Vec::new).extend(entities);
    }
}

impl DataSource for MemoryDataSource {
    fn scan(&self, table: &str) -> Result<Vec<Entity>> {
        Ok(self.entities.get(table).cloned().unwrap_or_else(Vec::new))
    }

    fn insert(&mut self, table: &str, entities: Vec<Entity>) -> Result<u64> {
        let count = entities.len() as u64;
        self.entities.entry(table.to_string()).or_insert_with(Vec::new).extend(entities);
        Ok(count)
    }

    fn update(&mut self, table: &str, _filter: Option<&CompiledExpression>, _assignments: &[CompiledAssignment]) -> Result<u64> {
        if let Some(entities) = self.entities.get(table) {
            Ok(entities.len() as u64)
        } else {
            Ok(0)
        }
    }

    fn delete(&mut self, table: &str, _filter: Option<&CompiledExpression>) -> Result<u64> {
        if let Some(entities) = self.entities.get_mut(table) {
            let count = entities.len() as u64;
            entities.clear();
            Ok(count)
        } else {
            Ok(0)
        }
    }

    fn get_schema(&self, table: &str) -> Result<TableSchema> {
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ColumnType::EntityId);
        columns.insert("name".to_string(), ColumnType::String);
        columns.insert("age".to_string(), ColumnType::Integer);
        columns.insert("active".to_string(), ColumnType::Boolean);
        columns.insert("x".to_string(), ColumnType::Float);
        columns.insert("y".to_string(), ColumnType::Float);
        columns.insert("z".to_string(), ColumnType::Float);
        
        Ok(TableSchema {
            name: table.to_string(),
            columns,
        })
    }
}

impl Default for MemoryDataSource {
    fn default() -> Self {
        Self::new()
    }
}
