use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const SHA256_HEX_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageClass {
    Raw,
    Derived,
    Thumbnail,
    Transcript,
    OcrJson,
    Waveform,
    Keyframes,
}

impl StorageClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Derived => "derived",
            Self::Thumbnail => "thumbnail",
            Self::Transcript => "transcript",
            Self::OcrJson => "ocr_json",
            Self::Waveform => "waveform",
            Self::Keyframes => "keyframes",
        }
    }
}

impl fmt::Display for StorageClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub asset_id: String,
    pub sha256: String,
    pub media_type: String,
    pub storage_class: StorageClass,
    pub relative_path: String,
}

impl AssetRef {
    pub fn new(
        asset_id: impl Into<String>,
        sha256: impl Into<String>,
        media_type: impl Into<String>,
        storage_class: StorageClass,
        relative_path: impl Into<String>,
    ) -> Result<Self, AssetError> {
        let asset_id = asset_id.into();
        let sha256 = sha256.into();
        let media_type = media_type.into();
        let relative_path = relative_path.into();

        if asset_id.trim().is_empty() {
            return Err(AssetError::EmptyAssetId);
        }
        validate_sha256(&sha256)?;
        if media_type.trim().is_empty() {
            return Err(AssetError::EmptyMediaType);
        }
        if relative_path.trim().is_empty() {
            return Err(AssetError::EmptyRelativePath);
        }

        Ok(Self {
            asset_id,
            sha256,
            media_type,
            storage_class,
            relative_path,
        })
    }

    pub fn uri(&self) -> String {
        format!("asset://{}", self.relative_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub asset_id: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub storage_class: StorageClass,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub page_count: Option<u32>,
    pub codec: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutAssetRequest {
    pub bytes: Vec<u8>,
    pub media_type: String,
    pub storage_class: StorageClass,
    pub extension: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub page_count: Option<u32>,
    pub codec: Option<String>,
}

impl PutAssetRequest {
    pub fn new(media_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            media_type: media_type.into(),
            storage_class: StorageClass::Raw,
            extension: None,
            width: None,
            height: None,
            duration_ms: None,
            page_count: None,
            codec: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAsset {
    pub reference: AssetRef,
    pub metadata: AssetMetadata,
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct FileSystemAssetStore {
    root: PathBuf,
}

impl FileSystemAssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, AssetError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn store_bytes(&self, request: PutAssetRequest) -> Result<StoredAsset, AssetError> {
        if request.bytes.is_empty() {
            return Err(AssetError::EmptyPayload);
        }
        if request.media_type.trim().is_empty() {
            return Err(AssetError::EmptyMediaType);
        }

        let sha256 = sha256_hex(&request.bytes);
        let extension = normalize_extension(request.extension.as_deref(), &request.media_type)?;
        let relative_path = build_relative_path(request.storage_class, &sha256, &extension);
        let absolute_path = self.root.join(&relative_path);

        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if !absolute_path.exists() {
            fs::write(&absolute_path, &request.bytes)?;
        }

        let asset_id = format!("asset_{sha256}");
        let reference = AssetRef::new(
            asset_id.clone(),
            sha256.clone(),
            request.media_type.clone(),
            request.storage_class,
            path_to_string(&relative_path),
        )?;
        let metadata = AssetMetadata {
            asset_id,
            sha256,
            size_bytes: request.bytes.len() as u64,
            mime_type: request.media_type,
            storage_class: request.storage_class,
            width: request.width,
            height: request.height,
            duration_ms: request.duration_ms,
            page_count: request.page_count,
            codec: request.codec,
        };

        Ok(StoredAsset {
            reference,
            metadata,
            absolute_path,
        })
    }

    pub fn absolute_path(&self, reference: &AssetRef) -> PathBuf {
        self.root.join(&reference.relative_path)
    }

    pub fn exists(&self, reference: &AssetRef) -> bool {
        self.absolute_path(reference).exists()
    }

    pub fn read_bytes(&self, reference: &AssetRef) -> Result<Vec<u8>, AssetError> {
        Ok(fs::read(self.absolute_path(reference))?)
    }
}

fn normalize_extension(explicit: Option<&str>, media_type: &str) -> Result<String, AssetError> {
    let extension = explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_start_matches('.').to_ascii_lowercase())
        .unwrap_or_else(|| infer_extension(media_type).to_string());

    if extension.is_empty() || !extension.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(AssetError::InvalidExtension(extension));
    }

    Ok(extension)
}

fn infer_extension(media_type: &str) -> &'static str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/json" => "json",
        "text/plain" => "txt",
        _ => "bin",
    }
}

fn build_relative_path(storage_class: StorageClass, sha256: &str, extension: &str) -> PathBuf {
    let prefix_a = &sha256[..2];
    let prefix_b = &sha256[2..4];
    PathBuf::from(storage_class.as_str())
        .join("sha256")
        .join(prefix_a)
        .join(prefix_b)
        .join(format!("{sha256}.{extension}"))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(
        String::with_capacity(digest.len() * 2),
        |mut output, byte| {
            let _ = write!(&mut output, "{byte:02x}");
            output
        },
    )
}

fn validate_sha256(sha256: &str) -> Result<(), AssetError> {
    if sha256.trim().is_empty() {
        return Err(AssetError::EmptySha256);
    }
    if sha256.len() != SHA256_HEX_LEN {
        return Err(AssetError::InvalidSha256Length(sha256.len()));
    }
    if !sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(AssetError::InvalidSha256Hex);
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("asset_id must not be empty")]
    EmptyAssetId,
    #[error("sha256 must not be empty")]
    EmptySha256,
    #[error("sha256 must contain 64 hex characters, got {0}")]
    InvalidSha256Length(usize),
    #[error("sha256 must be lowercase hex")]
    InvalidSha256Hex,
    #[error("media_type must not be empty")]
    EmptyMediaType,
    #[error("relative_path must not be empty")]
    EmptyRelativePath,
    #[error("asset payload must not be empty")]
    EmptyPayload,
    #[error("invalid asset extension '{0}'")]
    InvalidExtension(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::{
        AssetError, AssetRef, FileSystemAssetStore, PutAssetRequest, StorageClass, sha256_hex,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rejects_empty_hash() {
        assert!(matches!(
            AssetRef::new(
                "",
                "abc",
                "image/png",
                StorageClass::Raw,
                "raw/sha256/a/b.bin"
            ),
            Err(AssetError::EmptyAssetId)
        ));
    }

    #[test]
    fn rejects_invalid_sha256() {
        assert!(matches!(
            AssetRef::new(
                "asset_test",
                "not-a-sha",
                "image/png",
                StorageClass::Raw,
                "raw/sha256/a/b.bin"
            ),
            Err(AssetError::InvalidSha256Length(_))
        ));
    }

    #[test]
    fn stores_assets_in_content_addressed_paths() {
        let root = unique_test_dir("stores_assets");
        let store = FileSystemAssetStore::new(&root).expect("store should initialize");
        let bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];

        let stored = store
            .store_bytes(PutAssetRequest::new("image/png", bytes.clone()))
            .expect("asset should store");

        assert_eq!(stored.metadata.storage_class, StorageClass::Raw);
        assert_eq!(stored.metadata.mime_type, "image/png");
        assert_eq!(stored.metadata.size_bytes, bytes.len() as u64);
        assert!(stored.reference.uri().starts_with("asset://raw/sha256/"));
        assert!(store.exists(&stored.reference));
        assert_eq!(store.read_bytes(&stored.reference).unwrap(), bytes);

        cleanup_test_dir(root);
    }

    #[test]
    fn deduplicates_same_payload_by_sha256_path() {
        let root = unique_test_dir("deduplicates_payload");
        let store = FileSystemAssetStore::new(&root).expect("store should initialize");
        let bytes = b"same-image".to_vec();
        let expected_sha = sha256_hex(&bytes);

        let first = store
            .store_bytes(PutAssetRequest::new("image/jpeg", bytes.clone()))
            .expect("first asset should store");
        let second = store
            .store_bytes(PutAssetRequest::new("image/jpeg", bytes))
            .expect("second asset should store");

        assert_eq!(first.reference.sha256, expected_sha);
        assert_eq!(first.absolute_path, second.absolute_path);
        assert_eq!(first.reference.asset_id, second.reference.asset_id);

        cleanup_test_dir(root);
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("meat-memory-{label}-{nonce}"))
    }

    fn cleanup_test_dir(path: PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}
