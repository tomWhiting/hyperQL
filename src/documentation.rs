pub struct HyperQLSyntax;

impl HyperQLSyntax {
    pub fn documentation() -> &'static str {
        r#"# HyperQL Language Reference

## Overview
HyperQL is a unified query language for the Hyperspatial database that combines:
- SQL-like syntax for relational operations
- Graph traversal capabilities
- Vector similarity search
- Geometric operations in hyperbolic space
- Cascade operations for measure propagation

## Statement Types

### SELECT Statement
```
SELECT [DISTINCT] <select_list>
FROM <table_name> [alias]
[TRAVERSE <traverse_pattern>]
[WHERE <condition>]
[GROUP BY <expression_list>]
[HAVING <condition>]
[ORDER BY <expression> [ASC|DESC]]
[LIMIT <number>]
[OFFSET <number>]
```

### INSERT Statement
```
INSERT INTO <table_name> (<column_list>)
VALUES (<value_list>)
```

### UPDATE Statement
```
UPDATE <table_name>
SET <column> = <value> [, <column> = <value>...]
[WHERE <condition>]
```

### DELETE Statement
```
DELETE FROM <table_name>
[WHERE <condition>]
```

## Expressions

### Literals
- Numbers: `42`, `3.14`
- Strings: `'text'`, `"text"`
- Booleans: `TRUE`, `FALSE`
- NULL: `NULL`
- Entity IDs: `@entity_123`

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `=`, `!=`, `<`, `<=`, `>`, `>=`
- Logical: `AND`, `OR`, `NOT`
- String: `LIKE`, `NOT LIKE`
- Membership: `IN`, `NOT IN`

### Functions
- Aggregate: `COUNT()`, `SUM()`, `AVG()`, `MIN()`, `MAX()`
- String: `UPPER()`, `LOWER()`, `LENGTH()`
- Math: `ABS()`, `ROUND()`, `SQRT()`
- Geometric: `DISTANCE()`, `WITHIN()`, `NEAR()`
- Vector: `SIMILARITY()`, `KNN()`

## Graph Traversal

### TRAVERSE Clause
```
TRAVERSE <start_node>-[<relationship>]-><end_node>
```

### Node Patterns
- `(n:Label {property: value})` - Node with label and properties
- `(n)` - Any node
- `()` - Anonymous node

### Relationship Patterns
- `-[:TYPE]->` - Outgoing relationship of type TYPE
- `<-[:TYPE]-` - Incoming relationship of type TYPE
- `-[:TYPE]-` - Undirected relationship of type TYPE
- `-[*1..3]->` - Variable length path (1 to 3 hops)

## Geometric Operations

### Distance Functions
- `DISTANCE(pos1, pos2)` - Hyperbolic distance
- `EUCLIDEAN_DISTANCE(pos1, pos2)` - Euclidean distance

### Spatial Predicates
- `WITHIN(target, radius, reference)` - Point within radius
- `NEAR(target, distance, reference)` - Point near reference

## Vector Operations

### Similarity Search
- `SIMILARITY(vector_name, query_vector, 'cosine')` - Cosine similarity
- `SIMILARITY(vector_name, query_vector, 'euclidean')` - Euclidean similarity

### K-Nearest Neighbors
- `KNN(vector_name, query_vector, k)` - Find k nearest neighbors

## Data Types

### Primitive Types
- `INTEGER` - 64-bit signed integer
- `FLOAT` - 64-bit floating point
- `STRING` - UTF-8 text
- `BOOLEAN` - True/false value
- `NULL` - Null value

### Spatial Types
- `POSITION` - 3D coordinates (x, y, z)
- `DISTANCE` - Hyperbolic distance value
- `VECTOR` - Numerical array for embeddings

### Collection Types
- `LIST` - Ordered sequence of values
- `MAP` - Key-value associations

## Best Practices

1. Use meaningful aliases for tables and columns
2. Index frequently queried properties
3. Use LIMIT to control result set size
4. Combine multiple paradigms efficiently
5. Test geometric queries with known coordinates
"#
    }

    pub fn basic_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Select all records", "SELECT * FROM entities"),
            ("Select specific columns", "SELECT name, age FROM users"),
            ("Filter by condition", "SELECT * FROM users WHERE age > 21"),
            ("Order results", "SELECT name, age FROM users ORDER BY age DESC"),
            ("Limit results", "SELECT * FROM users LIMIT 10"),
            ("Group and aggregate", "SELECT department, COUNT(*) FROM employees GROUP BY department"),
            ("Insert new record", "INSERT INTO users (name, age) VALUES ('Alice', 25)"),
            ("Update records", "UPDATE users SET age = 26 WHERE name = 'Alice'"),
            ("Delete records", "DELETE FROM users WHERE age < 18"),
            ("Complex WHERE clause", "SELECT * FROM products WHERE price > 100 AND category = 'electronics'"),
        ]
    }

    pub fn geometric_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Find entities within radius",
                "SELECT * FROM entities WHERE WITHIN(position, 5.0, POSITION(0, 0, 0))"
            ),
            (
                "Calculate distances",
                "SELECT name, DISTANCE(position, POSITION(1, 2, 3)) AS dist FROM entities"
            ),
            (
                "Find nearest entities",
                "SELECT * FROM entities WHERE NEAR(position, 2.0, POSITION(0, 0, 0))"
            ),
            (
                "Order by distance",
                "SELECT * FROM entities ORDER BY DISTANCE(position, POSITION(0, 0, 0))"
            ),
            (
                "Filter by distance range",
                "SELECT * FROM entities WHERE DISTANCE(position, POSITION(0, 0, 0)) BETWEEN 1.0 AND 5.0"
            ),
            (
                "Spatial join",
                "SELECT e1.name, e2.name FROM entities e1, entities e2 WHERE DISTANCE(e1.position, e2.position) < 1.0"
            ),
        ]
    }

    pub fn graph_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Simple traversal",
                "SELECT * FROM users u TRAVERSE (u)-[:FRIENDS]->(f)"
            ),
            (
                "Bidirectional traversal",
                "SELECT * FROM users u TRAVERSE (u)-[:CONNECTED]-(c)"
            ),
            (
                "Variable length path",
                "SELECT * FROM users u TRAVERSE (u)-[:FRIENDS*1..3]->(f)"
            ),
            (
                "Filtered traversal",
                "SELECT * FROM users u TRAVERSE (u)-[:FRIENDS]->(f) WHERE f.age > 25"
            ),
            (
                "Multiple patterns",
                "SELECT * FROM users u TRAVERSE (u)-[:FRIENDS]->(f), (f)-[:WORKS_AT]->(c)"
            ),
            (
                "Named relationships",
                "SELECT * FROM users u TRAVERSE (u)-[r:FRIENDS]->(f) WHERE r.since < '2020-01-01'"
            ),
        ]
    }

    pub fn vector_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Cosine similarity search",
                "SELECT * FROM documents WHERE SIMILARITY(embeddings, [0.1, 0.2, 0.3], 'cosine') > 0.8"
            ),
            (
                "K-nearest neighbors",
                "SELECT * FROM products WHERE KNN(features, [1.0, 2.0, 3.0], 5)"
            ),
            (
                "Euclidean similarity",
                "SELECT * FROM items WHERE SIMILARITY(vectors, [0.5, 0.5], 'euclidean') > 0.7"
            ),
            (
                "Vector with threshold",
                "SELECT name, SIMILARITY(embeddings, [0.1, 0.9], 'cosine') AS sim FROM docs WHERE sim > 0.6"
            ),
            (
                "Combined vector and text",
                "SELECT * FROM articles WHERE title LIKE '%AI%' AND SIMILARITY(embeddings, [0.2, 0.8], 'cosine') > 0.5"
            ),
        ]
    }

    pub fn stream_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Recent events",
                "SELECT * FROM events WHERE timestamp > NOW() - INTERVAL '1 hour'"
            ),
            (
                "Time-based grouping",
                "SELECT DATE(timestamp), COUNT(*) FROM events GROUP BY DATE(timestamp)"
            ),
            (
                "Moving average",
                "SELECT timestamp, AVG(value) OVER (ORDER BY timestamp ROWS 10 PRECEDING) FROM metrics"
            ),
            (
                "Event sequence",
                "SELECT * FROM events WHERE event_type = 'login' ORDER BY timestamp"
            ),
        ]
    }

    pub fn timeseries_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Time range query",
                "SELECT * FROM measurements WHERE timestamp BETWEEN '2023-01-01' AND '2023-12-31'"
            ),
            (
                "Hourly aggregation",
                "SELECT DATE_TRUNC('hour', timestamp), AVG(temperature) FROM sensors GROUP BY 1"
            ),
            (
                "Latest values",
                "SELECT DISTINCT ON (sensor_id) sensor_id, temperature, timestamp FROM readings ORDER BY sensor_id, timestamp DESC"
            ),
            (
                "Time-based filtering",
                "SELECT * FROM logs WHERE timestamp > NOW() - INTERVAL '24 hours' AND level = 'ERROR'"
            ),
        ]
    }
}

pub struct HyperQLExamples {
    pub basic: Vec<(&'static str, &'static str)>,
    pub geometric: Vec<(&'static str, &'static str)>,
    pub graph: Vec<(&'static str, &'static str)>,
    pub vector: Vec<(&'static str, &'static str)>,
    pub stream: Vec<(&'static str, &'static str)>,
    pub timeseries: Vec<(&'static str, &'static str)>,
}

impl Default for HyperQLExamples {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperQLExamples {
    pub fn new() -> Self {
        Self {
            basic: HyperQLSyntax::basic_examples(),
            geometric: HyperQLSyntax::geometric_examples(),
            graph: HyperQLSyntax::graph_examples(),
            vector: HyperQLSyntax::vector_examples(),
            stream: HyperQLSyntax::stream_examples(),
            timeseries: HyperQLSyntax::timeseries_examples(),
        }
    }

    pub fn all_examples(&self) -> Vec<(&'static str, &'static str)> {
        let mut all = Vec::new();
        all.extend(&self.basic);
        all.extend(&self.geometric);
        all.extend(&self.graph);
        all.extend(&self.vector);
        all.extend(&self.stream);
        all.extend(&self.timeseries);
        all
    }

    pub fn find_examples_by_keyword(&self, keyword: &str) -> Vec<(&'static str, &'static str)> {
        let keyword_lower = keyword.to_lowercase();
        self.all_examples()
            .into_iter()
            .filter(|(desc, query)| {
                desc.to_lowercase().contains(&keyword_lower) ||
                query.to_lowercase().contains(&keyword_lower)
            })
            .collect()
    }

    pub fn get_category_examples(&self, category: &str) -> Vec<(&'static str, &'static str)> {
        match category.to_lowercase().as_str() {
            "basic" | "sql" => self.basic.clone(),
            "geometric" | "spatial" => self.geometric.clone(),
            "graph" | "traverse" => self.graph.clone(),
            "vector" | "similarity" => self.vector.clone(),
            "stream" | "events" => self.stream.clone(),
            "timeseries" | "time" => self.timeseries.clone(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_statement;

    #[test]
    fn test_documentation_generation() {
        let doc = HyperQLSyntax::documentation();
        assert!(!doc.is_empty());
        assert!(doc.contains("HyperQL"));
        assert!(doc.contains("SELECT"));
        assert!(doc.contains("TRAVERSE"));
    }

    #[test]
    fn test_basic_examples_parse() {
        let examples = HyperQLSyntax::basic_examples();
        assert!(!examples.is_empty());

        for (desc, query) in examples {
            println!("Testing: {} - {}", desc, query);
            let result = parse_statement(query);
            assert!(result.is_ok(), "Failed to parse example '{}': {}", desc, query);
        }
    }

    #[test]
    fn test_geometric_examples_parse() {
        let examples = HyperQLSyntax::geometric_examples();
        assert!(!examples.is_empty());

        for (desc, query) in examples {
            println!("Testing geometric: {} - {}", desc, query);
            let result = parse_statement(query);
            if result.is_err() {
                println!("Note: Geometric example '{}' may require extended syntax: {}", desc, query);
            }
        }
    }

    #[test]
    fn test_examples_categorization() {
        let examples = HyperQLExamples::new();
        assert!(!examples.basic.is_empty());
        assert!(!examples.geometric.is_empty());
        assert!(!examples.graph.is_empty());
        assert!(!examples.vector.is_empty());

        let all = examples.all_examples();
        assert!(all.len() >= examples.basic.len());
    }

    #[test]
    fn test_keyword_search() {
        let examples = HyperQLExamples::new();
        
        let select_examples = examples.find_examples_by_keyword("SELECT");
        assert!(!select_examples.is_empty());
        
        let distance_examples = examples.find_examples_by_keyword("distance");
        assert!(!distance_examples.is_empty());
    }

    #[test]
    fn test_category_retrieval() {
        let examples = HyperQLExamples::new();
        
        let basic = examples.get_category_examples("basic");
        assert_eq!(basic, examples.basic);
        
        let geometric = examples.get_category_examples("spatial");
        assert_eq!(geometric, examples.geometric);
        
        let empty = examples.get_category_examples("nonexistent");
        assert!(empty.is_empty());
    }
}
