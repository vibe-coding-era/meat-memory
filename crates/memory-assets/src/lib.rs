#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRef {
    pub sha256: String,
    pub media_type: String,
}

impl AssetRef {
    pub fn new(
        sha256: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Result<Self, &'static str> {
        let sha256 = sha256.into();
        let media_type = media_type.into();

        if sha256.trim().is_empty() {
            return Err("sha256 must not be empty");
        }

        if media_type.trim().is_empty() {
            return Err("media_type must not be empty");
        }

        Ok(Self { sha256, media_type })
    }
}

#[cfg(test)]
mod tests {
    use super::AssetRef;

    #[test]
    fn rejects_empty_hash() {
        assert!(AssetRef::new("", "image/png").is_err());
    }
}
