#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub limit: usize,
}

impl SearchQuery {
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            limit,
        }
    }
}

pub fn normalize_query(query: &SearchQuery) -> String {
    query.text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{SearchQuery, normalize_query};

    #[test]
    fn normalizes_query_text() {
        let query = SearchQuery::new("  Hello Memory  ", 10);
        assert_eq!(normalize_query(&query), "hello memory");
    }
}
