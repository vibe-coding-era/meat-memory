pub trait EmbeddingModel {
    fn provider_name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
    Qwen,
    Doubao,
    MiniMax,
    Glm,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Qwen => "qwen",
            Self::Doubao => "doubao",
            Self::MiniMax => "minimax",
            Self::Glm => "glm",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;

    #[test]
    fn renders_provider_name() {
        assert_eq!(Provider::Qwen.as_str(), "qwen");
    }
}
