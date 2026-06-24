use super::*;

pub(super) fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })
}

pub(super) fn first_string_deep(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(value) = first_string(value, keys).filter(|value| !value.trim().is_empty()) {
        return Some(value);
    }
    match value {
        Value::Array(items) => items
            .iter()
            .find_map(|item| first_string_deep(item, keys)),
        Value::Object(map) => map
            .values()
            .find_map(|value| first_string_deep(value, keys)),
        _ => None,
    }
}

pub(super) fn first_count(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(count) = map.get(*key).and_then(|value| parse_count_value(value, keys)) {
                    return Some(count);
                }
            }
            map.values().find_map(|value| first_count(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_count(value, keys)),
        _ => None,
    }
}

pub(super) fn parse_count_value(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64().or_else(|| {
            number
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value.round() as u64)
        }),
        Value::String(text) => parse_count_string(text),
        Value::Object(_) | Value::Array(_) => first_count(value, keys),
        _ => None,
    }
}

pub(super) fn parse_count_string(text: &str) -> Option<u64> {
    let compact: String = text
        .trim()
        .chars()
        .filter(|ch| !matches!(ch, ',' | '，' | ' ' | '\u{00a0}'))
        .collect();
    if compact.is_empty() {
        return None;
    }

    let lower = compact.to_ascii_lowercase();
    let multiplier = if compact.contains('亿') {
        100_000_000.0
    } else if compact.contains('万') || lower.contains('w') {
        10_000.0
    } else if lower.contains('k') {
        1_000.0
    } else {
        1.0
    };

    let mut numeric = String::new();
    let mut started = false;
    let mut saw_dot = false;
    for ch in lower.chars() {
        if ch.is_ascii_digit() {
            numeric.push(ch);
            started = true;
        } else if ch == '.' && !saw_dot {
            numeric.push(ch);
            saw_dot = true;
            started = true;
        } else if started {
            break;
        }
    }

    let value = numeric.parse::<f64>().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some((value * multiplier).round() as u64)
}

pub(super) fn first_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| value.get(*key).and_then(Value::as_i64))
}
