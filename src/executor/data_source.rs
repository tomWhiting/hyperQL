use crate::compiler::{CompiledExpression, CompiledAssignment};
use crate::types::Entity;
use crate::error::Result;
use super::{DataSource, TableSchema, ColumnType};
use std::collections::HashMap;

#[derive(Clone)]
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
    fn scan(&self, table: &str, _entity_type: &str) -> Result<Vec<Entity>> {
        // For MemoryDataSource, we don't differentiate by entity_type (simple in-memory store)
        // Just return all entities from the "collection" (table)
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

    fn traverse_graph(
        &self,
        start_entity_id: &crate::types::EntityId,
        max_depth: usize,
        edge_type_filter: Option<&str>,
    ) -> Result<Vec<(Entity, usize)>> {
        use std::collections::{HashSet, VecDeque};
        use crate::types::{PropertyName, Value};

        let mut results = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((start_entity_id.clone(), 0));
        visited.insert(start_entity_id.clone());

        let relationships = self.entities.get("relationships").cloned().unwrap_or_default();

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            for (_table_name, table_entities) in &self.entities {
                if let Some(entity) = table_entities.iter().find(|e| e.id == current_id) {
                    results.push((entity.clone(), depth));
                    break;
                }
            }

            if depth < max_depth {
                for rel_entity in &relationships {
                    let from_id = rel_entity.properties.get(&PropertyName("from_id".to_string()));
                    let to_id = rel_entity.properties.get(&PropertyName("to_id".to_string()));
                    let rel_type = rel_entity.properties.get(&PropertyName("type".to_string()));

                    if let (Some(Value::String(from)), Some(Value::String(to)), Some(Value::String(rtype))) = (from_id, to_id, rel_type) {
                        if edge_type_filter.is_some() && edge_type_filter != Some(rtype.as_str()) {
                            continue;
                        }

                        let next_id = if from == &current_id.0 {
                            Some(crate::types::EntityId(to.clone()))
                        } else if to == &current_id.0 {
                            Some(crate::types::EntityId(from.clone()))
                        } else {
                            None
                        };

                        if let Some(next) = next_id {
                            if !visited.contains(&next) {
                                visited.insert(next.clone());
                                queue.push_back((next, depth + 1));
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

impl Default for MemoryDataSource {
    fn default() -> Self {
        Self::new()
    }
}
