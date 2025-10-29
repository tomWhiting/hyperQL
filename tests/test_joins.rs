//! Comprehensive JOIN tests for HyperQL
//!
//! This test module validates all JOIN functionality including:
//! - All JOIN types (INNER, LEFT, RIGHT, FULL OUTER)
//! - Multiple JOINs in a single query
//! - JOINs with WHERE clauses
//! - JOINs with ORDER BY
//! - JOINs with aggregates
//! - Complex join conditions

use hyperQL::parser::parse_statement;
use hyperQL::compiler::Compiler;
use hyperQL::executor::{Executor, MemoryDataSource, DataSource};
use hyperQL::types::{Entity, EntityId, PropertyName, Value};

/// Create test entities for patients table
fn create_patients() -> Vec<Entity> {
    vec![
        Entity {
            id: EntityId("p1".to_string()),
            properties: vec![
                (PropertyName("subject_id".to_string()), Value::Int(1)),
                (PropertyName("name".to_string()), Value::String("Alice".to_string())),
                (PropertyName("age".to_string()), Value::Int(30)),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("p2".to_string()),
            properties: vec![
                (PropertyName("subject_id".to_string()), Value::Int(2)),
                (PropertyName("name".to_string()), Value::String("Bob".to_string())),
                (PropertyName("age".to_string()), Value::Int(45)),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("p3".to_string()),
            properties: vec![
                (PropertyName("subject_id".to_string()), Value::Int(3)),
                (PropertyName("name".to_string()), Value::String("Charlie".to_string())),
                (PropertyName("age".to_string()), Value::Int(55)),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
    ]
}

/// Create test entities for admissions table
fn create_admissions() -> Vec<Entity> {
    vec![
        Entity {
            id: EntityId("a1".to_string()),
            properties: vec![
                (PropertyName("hadm_id".to_string()), Value::Int(101)),
                (PropertyName("subject_id".to_string()), Value::Int(1)),
                (PropertyName("admission_type".to_string()), Value::String("EMERGENCY".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("a2".to_string()),
            properties: vec![
                (PropertyName("hadm_id".to_string()), Value::Int(102)),
                (PropertyName("subject_id".to_string()), Value::Int(1)),
                (PropertyName("admission_type".to_string()), Value::String("ELECTIVE".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("a3".to_string()),
            properties: vec![
                (PropertyName("hadm_id".to_string()), Value::Int(103)),
                (PropertyName("subject_id".to_string()), Value::Int(2)),
                (PropertyName("admission_type".to_string()), Value::String("URGENT".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
    ]
}

/// Create test entities for chartevents table
fn create_chartevents() -> Vec<Entity> {
    vec![
        Entity {
            id: EntityId("c1".to_string()),
            properties: vec![
                (PropertyName("hadm_id".to_string()), Value::Int(101)),
                (PropertyName("valuenum".to_string()), Value::Float(98.6)),
                (PropertyName("label".to_string()), Value::String("Temperature".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("c2".to_string()),
            properties: vec![
                (PropertyName("hadm_id".to_string()), Value::Int(102)),
                (PropertyName("valuenum".to_string()), Value::Float(120.0)),
                (PropertyName("label".to_string()), Value::String("Heart Rate".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
        Entity {
            id: EntityId("c3".to_string()),
            properties: vec![
                (PropertyName("hadm_id".to_string()), Value::Int(103)),
                (PropertyName("valuenum".to_string()), Value::Float(100.2)),
                (PropertyName("label".to_string()), Value::String("Temperature".to_string())),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
    ]
}

fn setup_test_data() -> MemoryDataSource {
    let mut data_source = MemoryDataSource::new();
    data_source.insert("patients", create_patients()).unwrap();
    data_source.insert("admissions", create_admissions()).unwrap();
    data_source.insert("chartevents", create_chartevents()).unwrap();
    data_source
}

#[test]
fn test_inner_join_basic() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 3 rows (patient 1 has 2 admissions, patient 2 has 1 admission)
    assert_eq!(result.rows.len(), 3);

    // Verify all rows have both patient and admission data (with alias prefixes)
    for row in &result.rows {
        assert!(row.columns.contains_key("p_subject_id") || row.columns.contains_key("subject_id"));
        assert!(row.columns.contains_key("p_name") || row.columns.contains_key("name"));
        assert!(row.columns.contains_key("a_hadm_id") || row.columns.contains_key("hadm_id"));
        assert!(row.columns.contains_key("a_admission_type") || row.columns.contains_key("admission_type"));
    }
}

#[test]
fn test_left_join() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p LEFT JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 4 rows (all patients, including Charlie with NULL admission)
    // Patient 1: 2 admissions
    // Patient 2: 1 admission
    // Patient 3: 0 admissions (NULL)
    assert_eq!(result.rows.len(), 4);

    // Find Charlie's row (should have NULL for admission fields)
    let charlie_rows: Vec<_> = result.rows.iter()
        .filter(|row| {
            if let Some(Value::String(name)) = row.columns.get("p_name").or_else(|| row.columns.get("name")) {
                name == "Charlie"
            } else {
                false
            }
        })
        .collect();

    assert_eq!(charlie_rows.len(), 1);
    // Charlie should have NULL hadm_id since no admissions
    assert_eq!(charlie_rows[0].columns.get("a_hadm_id").or_else(|| charlie_rows[0].columns.get("hadm_id")), Some(&Value::Null));
}

#[test]
fn test_right_join() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p RIGHT JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 3 rows (all admissions have matching patients)
    assert_eq!(result.rows.len(), 3);

    // All rows should have admission data (with alias prefix)
    for row in &result.rows {
        assert!(row.columns.contains_key("a_hadm_id") || row.columns.contains_key("hadm_id"),
                "Should have hadm_id, got keys: {:?}", row.columns.keys());
        assert!(row.columns.contains_key("a_admission_type") || row.columns.contains_key("admission_type"),
                "Should have admission_type, got keys: {:?}", row.columns.keys());
    }
}

#[test]
fn test_full_outer_join() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p FULL OUTER JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 4 rows:
    // - Patient 1 + 2 admissions = 2 rows
    // - Patient 2 + 1 admission = 1 row
    // - Patient 3 + NULL admission = 1 row
    assert_eq!(result.rows.len(), 4);
}

#[test]
fn test_multiple_joins() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 INNER JOIN chartevents.ChartEvent c ON a.hadm_id = c.hadm_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 3 rows (each admission has 1 chartevent)
    assert_eq!(result.rows.len(), 3);

    // Verify all rows have data from all three tables (with nested alias prefixes)
    // Note: Multiple joins create nested prefixes like "p_a_*" and "p_p_*"
    for row in &result.rows {
        // Check for any variation of subject_id
        let has_subject_id = row.columns.keys().any(|k| k.contains("subject_id"));
        assert!(has_subject_id, "Should have subject_id column, got keys: {:?}", row.columns.keys());

        // Check for name
        let has_name = row.columns.keys().any(|k| k.contains("name"));
        assert!(has_name, "Should have name column");

        // Check for hadm_id
        let has_hadm_id = row.columns.keys().any(|k| k.contains("hadm_id"));
        assert!(has_hadm_id, "Should have hadm_id column");

        // Check for valuenum
        let has_valuenum = row.columns.keys().any(|k| k.contains("valuenum"));
        assert!(has_valuenum, "Should have valuenum column");

        // Check for label
        let has_label = row.columns.keys().any(|k| k.contains("label"));
        assert!(has_label, "Should have label column");
    }
}

#[test]
fn test_join_with_where_clause() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 WHERE p.age > 40";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Only Bob (age 45) should match
    assert_eq!(result.rows.len(), 1);

    // Verify it's Bob (with alias prefix)
    let row = &result.rows[0];
    assert_eq!(row.columns.get("p_name").or_else(|| row.columns.get("name")), Some(&Value::String("Bob".to_string())));
}

#[test]
fn test_join_with_order_by() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 ORDER BY p.age DESC";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    assert_eq!(result.rows.len(), 3);

    // Results should be ordered by age descending
    // Bob (45) should be first, then Alice (30) twice
    if let Some(Value::String(first_name)) = result.rows[0].columns.get("p_name").or_else(|| result.rows[0].columns.get("name")) {
        assert_eq!(first_name, "Bob");
    } else {
        panic!("Expected name field");
    }
}

#[test]
fn test_join_with_limit() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT * FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 LIMIT 2";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should limit to 2 rows
    assert_eq!(result.rows.len(), 2);
}

#[test]
fn test_join_with_aggregates() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT p.subject_id, COUNT(*) FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 GROUP BY p.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 2 groups (patient 1 with 2 admissions, patient 2 with 1)
    assert_eq!(result.rows.len(), 2);

    // Check counts (with alias prefix)
    for row in &result.rows {
        // Try different possible column name formats
        let subject_id_value = row.columns.get("p_subject_id")
            .or_else(|| row.columns.get("subject_id"))
            .or_else(|| row.columns.get("p.subject_id"));

        if let Some(Value::Int(subject_id)) = subject_id_value {
            if *subject_id == 1 {
                // Patient 1 has 2 admissions
                let count = row.columns.get("COUNT(*)").or_else(|| row.columns.get("count")).unwrap();
                if let Value::Int(c) = count {
                    assert_eq!(*c, 2);
                }
            } else if *subject_id == 2 {
                // Patient 2 has 1 admission
                let count = row.columns.get("COUNT(*)").or_else(|| row.columns.get("count")).unwrap();
                if let Value::Int(c) = count {
                    assert_eq!(*c, 1);
                }
            }
        }
    }
}

#[test]
fn test_join_default_is_inner() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    // Test that JOIN without qualifier defaults to INNER JOIN
    let query = "SELECT * FROM patients.Patient p JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have same result as INNER JOIN (3 rows)
    assert_eq!(result.rows.len(), 3);
}

#[test]
fn test_join_with_table_aliases() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT p.name, a.admission_type FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    assert_eq!(result.rows.len(), 3);

    // Verify selected columns are present (with alias prefixes)
    for row in &result.rows {
        assert!(row.columns.contains_key("p_name") || row.columns.contains_key("name") || row.columns.contains_key("p.name"));
        assert!(row.columns.contains_key("a_admission_type") || row.columns.contains_key("admission_type") || row.columns.contains_key("a.admission_type"));
    }
}

#[test]
fn test_parser_recognizes_all_join_types() {
    // Test that parser correctly identifies all JOIN types
    let queries = vec![
        ("SELECT * FROM a.A INNER JOIN b.B ON a.id = b.id", "INNER"),
        ("SELECT * FROM a.A LEFT JOIN b.B ON a.id = b.id", "LEFT"),
        ("SELECT * FROM a.A RIGHT JOIN b.B ON a.id = b.id", "RIGHT"),
        ("SELECT * FROM a.A FULL OUTER JOIN b.B ON a.id = b.id", "FULL OUTER"),
        ("SELECT * FROM a.A JOIN b.B ON a.id = b.id", "DEFAULT (INNER)"),
    ];

    for (query, join_type) in queries {
        let statement = parse_statement(query);
        assert!(statement.is_ok(), "Failed to parse {} JOIN: {}", join_type, query);
    }
}

#[test]
fn test_join_missing_on_clause_error() {
    let query = "SELECT * FROM patients.Patient INNER JOIN admissions.Admission";
    let statement = parse_statement(query);

    // Should error because ON clause is missing
    assert!(statement.is_err());
}

#[test]
fn test_left_join_preserves_all_left_rows() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT p.name FROM patients.Patient p LEFT JOIN admissions.Admission a ON p.subject_id = a.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Count unique patient names (should be all 3 patients) - with alias prefix
    let mut names = std::collections::HashSet::new();
    for row in &result.rows {
        if let Some(Value::String(name)) = row.columns.get("p_name").or_else(|| row.columns.get("name")).or_else(|| row.columns.get("p.name")) {
            names.insert(name.clone());
        }
    }

    assert_eq!(names.len(), 3, "LEFT JOIN should preserve all patients");
    assert!(names.contains("Alice"));
    assert!(names.contains("Bob"));
    assert!(names.contains("Charlie"));
}

#[test]
fn test_mixed_join_types() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    // Mix INNER and LEFT JOIN
    let query = "SELECT * FROM patients.Patient p \
                 LEFT JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 INNER JOIN chartevents.ChartEvent c ON a.hadm_id = c.hadm_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // The INNER JOIN will filter out rows where hadm_id is NULL
    // So we should only get rows where both admissions and chartevents exist
    assert!(result.rows.len() > 0);
}

#[test]
fn test_join_with_aggregates_column_names() {
    let data_source = setup_test_data();
    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    let query = "SELECT p.subject_id, COUNT(*) FROM patients.Patient p \
                 INNER JOIN admissions.Admission a ON p.subject_id = a.subject_id \
                 GROUP BY p.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    // Should have 2 groups
    assert_eq!(result.rows.len(), 2);

    // CRITICAL: Verify column names are preserved from SELECT list, NOT generic group_0, group_1
    for row in &result.rows {
        let column_names: Vec<_> = row.columns.keys().cloned().collect();

        // Must NOT contain generic names like "group_0" or "group_1"
        assert!(!column_names.iter().any(|name| name.starts_with("group_")),
                "Found generic group_N column name: {:?}", column_names);

        // MUST contain either "p.subject_id" or "p_subject_id" (with table prefix)
        let has_subject_id = column_names.iter().any(|name|
            name == "p.subject_id" || name == "p_subject_id" || name == "subject_id"
        );
        assert!(has_subject_id, "Missing subject_id column. Found: {:?}", column_names);

        // MUST contain "COUNT(*)" (proper aggregate function naming)
        let has_count = column_names.iter().any(|name|
            name == "COUNT(*)" || name == "count"
        );
        assert!(has_count, "Missing COUNT(*) column. Found: {:?}", column_names);

        // Print column names for manual verification
        println!("Row column names: {:?}", column_names);
    }
}

#[test]
fn test_join_with_overlapping_column_names() {
    let mut data_source = MemoryDataSource::new();

    // Both tables have "name" and "subject_id" columns
    let users = vec![
        Entity {
            id: EntityId("u1".to_string()),
            properties: vec![
                (PropertyName("subject_id".to_string()), Value::Int(1)),
                (PropertyName("name".to_string()), Value::String("Alice".to_string())),
                (PropertyName("value".to_string()), Value::Int(100)),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
    ];

    let orders = vec![
        Entity {
            id: EntityId("o1".to_string()),
            properties: vec![
                (PropertyName("subject_id".to_string()), Value::Int(1)),
                (PropertyName("name".to_string()), Value::String("Order #1".to_string())),
                (PropertyName("value".to_string()), Value::Int(500)),
            ].into_iter().collect(),
            position: None,
            embedding: None,
        },
    ];

    data_source.insert("users", users).unwrap();
    data_source.insert("orders", orders).unwrap();

    let mut executor = Executor::new(Box::new(data_source));
    let compiler = Compiler::new();

    // Query with overlapping columns using collection.type format
    // Note: entity_type is set to same as collection name for testing
    let query = "SELECT * FROM users.users u JOIN orders.orders o ON u.subject_id = o.subject_id";
    let statement = parse_statement(query).unwrap();
    let compiled = compiler.compile(statement).unwrap();

    let result = executor.execute(compiled).unwrap();

    assert_eq!(result.rows.len(), 1);
    let row = &result.rows[0];

    // Both names should be present with prefixes
    assert!(row.columns.contains_key("u_name"), "Should have u_name, got keys: {:?}", row.columns.keys());
    assert!(row.columns.contains_key("o_name"), "Should have o_name, got keys: {:?}", row.columns.keys());
    assert_eq!(row.columns.get("u_name"), Some(&Value::String("Alice".to_string())));
    assert_eq!(row.columns.get("o_name"), Some(&Value::String("Order #1".to_string())));

    // Both values should be present
    assert!(row.columns.contains_key("u_value"), "Should have u_value");
    assert!(row.columns.contains_key("o_value"), "Should have o_value");
    assert_eq!(row.columns.get("u_value"), Some(&Value::Int(100)));
    assert_eq!(row.columns.get("o_value"), Some(&Value::Int(500)));

    // Both subject_ids should be present (critical - this was being lost before)
    assert!(row.columns.contains_key("u_subject_id"), "Should have u_subject_id");
    assert!(row.columns.contains_key("o_subject_id"), "Should have o_subject_id");
    assert_eq!(row.columns.get("u_subject_id"), Some(&Value::Int(1)));
    assert_eq!(row.columns.get("o_subject_id"), Some(&Value::Int(1)));
}
