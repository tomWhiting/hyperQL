use std::collections::HashMap;

pub fn split_query_parts(upper_input: &str, original_input: &str) -> HashMap<String, String> {
    let mut parts = HashMap::new();
    let keywords = ["SELECT", "FROM", "TRAVERSE", "WHERE", "GROUP BY", "HAVING", "ORDER BY", "LIMIT", "OFFSET"];

    let mut keyword_positions = Vec::new();
    for keyword in &keywords {
        if let Some(pos) = upper_input.find(keyword) {
            keyword_positions.push((pos, keyword));
        }
    }

    keyword_positions.sort_by_key(|(pos, _)| *pos);

    for i in 0..keyword_positions.len() {
        let (start_pos, keyword) = keyword_positions[i];
        let content_start = start_pos + keyword.len();
        let content_end = if i + 1 < keyword_positions.len() {
            keyword_positions[i + 1].0
        } else {
            original_input.len()
        };

        let content = original_input[content_start..content_end].trim();
        parts.insert(keyword.to_string(), content.to_string());
    }

    parts
}

pub fn find_operator_position(input: &str, operator: &str) -> Option<usize> {
    let mut in_quote = false;
    let mut quote_char = '"';
    let chars: Vec<char> = input.chars().collect();

    for i in 0..=chars.len().saturating_sub(operator.len()) {
        if i < chars.len() {
            let ch = chars[i];
            if (ch == '"' || ch == '\'') && (i == 0 || chars[i - 1] != '\\') {
                if !in_quote {
                    in_quote = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quote = false;
                }
            }
        }

        if !in_quote && i + operator.len() <= chars.len() {
            let substr: String = chars[i..i + operator.len()].iter().collect();
            if substr.to_uppercase() == operator.to_uppercase() {
                return Some(i);
            }
        }
    }

    None
}

pub fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let chars: Vec<char> = s.chars().collect();
    if !chars[0].is_alphabetic() && chars[0] != '_' {
        return false;
    }

    chars.iter().all(|c| c.is_alphanumeric() || *c == '_')
}

pub fn is_aggregate_function(name: &str) -> bool {
    matches!(name.to_uppercase().as_str(), "COUNT" | "SUM" | "AVG" | "MIN" | "MAX")
}
