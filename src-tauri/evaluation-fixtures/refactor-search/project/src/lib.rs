pub fn search(values: &[&str], query: &str) -> Vec<String> {
    values
        .iter()
        .filter(|value| value.contains(query))
        .map(|value| (*value).to_string())
        .collect()
}
