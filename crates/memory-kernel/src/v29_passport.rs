use crate::{Kernel, SecretDetector};
use anyhow::{Context, Result, anyhow, bail};
use memory_domain::{
    Artifact, ArtifactKind, EvidenceSpan, EvidenceSpanKind, Memory, MemoryId,
    MemoryPassportEncryption, MemoryPassportManifest, MemoryPassportObject,
    MemoryPassportObjectKind, MemoryPassportRedaction, RequestContext, ScopeId,
    SecretFindingAction, stable_hash,
};
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct MemoryProvenance {
    pub memory: Memory,
    pub evidence_spans: Vec<EvidenceSpan>,
}

#[derive(Debug, Clone)]
pub struct MemoryPassportBundle {
    pub manifest: MemoryPassportManifest,
    pub memories: Vec<Memory>,
    pub evidence_spans: Vec<EvidenceSpan>,
}

#[derive(Debug, Clone)]
pub struct MemoryPassportPaths {
    pub passport: PathBuf,
    pub manifest: PathBuf,
    pub memories: PathBuf,
    pub evidence: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MemoryPassportExportRequest {
    pub scope_id: ScopeId,
    pub limit: usize,
    pub redact_sensitive: bool,
    pub context: Option<RequestContext>,
}

impl MemoryPassportExportRequest {
    pub fn new(scope_id: ScopeId) -> Self {
        Self {
            scope_id,
            limit: 100,
            redact_sensitive: true,
            context: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPassportVerification {
    pub manifest: MemoryPassportManifest,
    pub valid: bool,
    pub checked_objects: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MemoryPassportImportRequest {
    pub input_dir: PathBuf,
    pub target_scope_id: Option<ScopeId>,
    pub dry_run: bool,
    pub context: Option<RequestContext>,
}

#[derive(Debug, Clone)]
pub struct MemoryPassportIdMapping {
    pub original_memory_id: MemoryId,
    pub imported_memory_id: MemoryId,
}

#[derive(Debug, Clone)]
pub struct MemoryPassportImportResult {
    pub manifest: MemoryPassportManifest,
    pub verified: bool,
    pub target_scope_id: ScopeId,
    pub imported_count: usize,
    pub skipped_count: usize,
    pub id_mappings: Vec<MemoryPassportIdMapping>,
}

impl Kernel {
    pub async fn memory_provenance(
        &self,
        scope_id: ScopeId,
        memory_id: MemoryId,
        context: Option<&RequestContext>,
    ) -> Result<MemoryProvenance> {
        let memory = self
            .get_memory(scope_id, memory_id)
            .await?
            .ok_or_else(|| anyhow!("memory not found"))?;
        self.ensure_key_can_access_scope(context, &memory.scope_id)?;
        Ok(MemoryProvenance {
            evidence_spans: derive_evidence_spans(&memory, None),
            memory,
        })
    }

    pub async fn export_memory_passport(
        &self,
        request: MemoryPassportExportRequest,
    ) -> Result<MemoryPassportBundle> {
        self.ensure_key_can_access_scope(request.context.as_ref(), &request.scope_id)?;
        let mut memories = self
            .browse_memories(Some(request.scope_id.clone()), request.limit)
            .await?;
        if request.redact_sensitive {
            memories = memories
                .into_iter()
                .map(redact_memory_for_passport)
                .collect::<Vec<_>>();
        }

        let evidence_spans = memories
            .iter()
            .flat_map(|memory| derive_evidence_spans(memory, None))
            .collect::<Vec<_>>();

        build_memory_passport_bundle(
            request.scope_id,
            memories,
            evidence_spans,
            if request.redact_sensitive {
                MemoryPassportRedaction::Sensitive
            } else {
                MemoryPassportRedaction::None
            },
        )
    }

    pub async fn import_memory_passport(
        &self,
        request: MemoryPassportImportRequest,
    ) -> Result<MemoryPassportImportResult> {
        let bundle = read_memory_passport_bundle(&request.input_dir)?;
        let verification = verify_memory_passport_bundle(&request.input_dir)?;
        if !verification.valid {
            bail!(
                "passport verification failed: {}",
                verification.errors.join("; ")
            );
        }

        let target_scope_id = request
            .target_scope_id
            .clone()
            .unwrap_or_else(|| bundle.manifest.source_scope_id.clone());
        self.ensure_key_can_access_scope(request.context.as_ref(), &target_scope_id)?;

        let mut imported_count = 0;
        let mut skipped_count = 0;
        let mut id_mappings = Vec::new();
        for mut memory in bundle.memories {
            let original_memory_id = memory.id.clone();
            let scope_changed = memory.scope_id != target_scope_id;
            if scope_changed {
                memory.id = MemoryId::new();
                memory.scope_id = target_scope_id.clone();
                memory.owner_scope_id = target_scope_id.clone();
                memory.published_from_scope_id = Some(bundle.manifest.source_scope_id.clone());
                memory.source_refs.push(format!(
                    "passport://{}/{}",
                    bundle.manifest.id.as_str(),
                    original_memory_id.as_str()
                ));
                memory.updated_at = OffsetDateTime::now_utc();
            }

            if self
                .get_memory(memory.scope_id.clone(), memory.id.clone())
                .await?
                .is_some()
            {
                skipped_count += 1;
                id_mappings.push(MemoryPassportIdMapping {
                    original_memory_id,
                    imported_memory_id: memory.id,
                });
                continue;
            }

            let imported_memory_id = memory.id.clone();
            if !request.dry_run {
                self.persist_memory(&memory).await?;
            }
            imported_count += 1;
            id_mappings.push(MemoryPassportIdMapping {
                original_memory_id,
                imported_memory_id,
            });
        }

        Ok(MemoryPassportImportResult {
            manifest: bundle.manifest,
            verified: true,
            target_scope_id,
            imported_count,
            skipped_count,
            id_mappings,
        })
    }
}

pub fn derive_evidence_spans(memory: &Memory, artifact: Option<&Artifact>) -> Vec<EvidenceSpan> {
    if let Some(artifact) = artifact {
        return vec![derive_artifact_evidence_span(memory, artifact)];
    }

    let quote = compact_quote(&memory.body);
    let hash = stable_hash(&memory.body);
    if memory.source_refs.is_empty() {
        return vec![EvidenceSpan::new_text(
            memory.scope_id.clone(),
            Some(memory.id.clone()),
            None,
            format!("memory:{}", memory.id.as_str()),
            quote,
            hash,
            0,
            memory.body.len(),
        )];
    }

    memory
        .source_refs
        .iter()
        .map(|source_ref| {
            EvidenceSpan::new_text(
                memory.scope_id.clone(),
                Some(memory.id.clone()),
                None,
                source_ref.clone(),
                quote.clone(),
                hash.clone(),
                0,
                memory.body.len(),
            )
        })
        .collect()
}

pub fn build_memory_passport_bundle(
    scope_id: ScopeId,
    memories: Vec<Memory>,
    evidence_spans: Vec<EvidenceSpan>,
    redaction: MemoryPassportRedaction,
) -> Result<MemoryPassportBundle> {
    let mut objects = Vec::with_capacity(memories.len() + evidence_spans.len());
    for memory in &memories {
        let payload = serde_json::to_string(memory)?;
        objects.push(MemoryPassportObject::new(
            MemoryPassportObjectKind::Memory,
            memory.id.as_str(),
            payload,
        ));
    }
    for span in &evidence_spans {
        let payload = serde_json::to_string(span)?;
        objects.push(MemoryPassportObject::new(
            MemoryPassportObjectKind::EvidenceSpan,
            span.id.as_str(),
            payload,
        ));
    }

    Ok(MemoryPassportBundle {
        manifest: MemoryPassportManifest::new(
            scope_id,
            objects,
            redaction,
            MemoryPassportEncryption::none(),
            OffsetDateTime::now_utc(),
        ),
        memories,
        evidence_spans,
    })
}

pub fn write_memory_passport_bundle(
    output_dir: &Path,
    bundle: &MemoryPassportBundle,
) -> Result<MemoryPassportPaths> {
    fs::create_dir_all(output_dir)?;
    let paths = MemoryPassportPaths {
        passport: output_dir.join("passport.json"),
        manifest: output_dir.join("manifest.json"),
        memories: output_dir.join("memories.json"),
        evidence: output_dir.join("evidence.json"),
        markdown: output_dir.join("manifest.md"),
    };
    fs::write(
        &paths.passport,
        serde_json::to_string_pretty(&bundle_json(bundle))?,
    )?;
    fs::write(
        &paths.manifest,
        serde_json::to_string_pretty(&bundle.manifest)?,
    )?;
    fs::write(
        &paths.memories,
        serde_json::to_string_pretty(&bundle.memories)?,
    )?;
    fs::write(
        &paths.evidence,
        serde_json::to_string_pretty(&bundle.evidence_spans)?,
    )?;
    fs::write(&paths.markdown, render_manifest_markdown(&bundle.manifest))?;
    Ok(paths)
}

pub fn read_memory_passport_bundle(input_dir: &Path) -> Result<MemoryPassportBundle> {
    let passport_path = input_dir.join("passport.json");
    if passport_path.exists() {
        let raw = fs::read_to_string(&passport_path)
            .with_context(|| format!("failed to read {}", passport_path.display()))?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse {}", passport_path.display()))?;
        return Ok(MemoryPassportBundle {
            manifest: serde_json::from_value(value["manifest"].clone())?,
            memories: serde_json::from_value(value["memories"].clone())?,
            evidence_spans: serde_json::from_value(value["evidence_spans"].clone())?,
        });
    }

    let manifest_path = input_dir.join("manifest.json");
    let memories_path = input_dir.join("memories.json");
    let evidence_path = input_dir.join("evidence.json");
    Ok(MemoryPassportBundle {
        manifest: read_json_file(&manifest_path)?,
        memories: read_json_file(&memories_path)?,
        evidence_spans: read_json_file(&evidence_path)?,
    })
}

pub fn verify_memory_passport_bundle(input_dir: &Path) -> Result<MemoryPassportVerification> {
    let bundle = read_memory_passport_bundle(input_dir)?;
    let actual_objects = passport_objects(&bundle.memories, &bundle.evidence_spans)?;
    let actual_by_key = actual_objects
        .iter()
        .map(|object| (object_key(object), object))
        .collect::<HashMap<_, _>>();
    let mut errors = Vec::new();

    for expected in &bundle.manifest.objects {
        match actual_by_key.get(&object_key(expected)) {
            Some(actual)
                if actual.hash == expected.hash && actual.byte_len == expected.byte_len => {}
            Some(actual) => errors.push(format!(
                "object hash mismatch: {}:{} expected {} got {}",
                expected.kind.as_str(),
                expected.id,
                expected.hash,
                actual.hash
            )),
            None => errors.push(format!(
                "object missing: {}:{}",
                expected.kind.as_str(),
                expected.id
            )),
        }
    }

    if bundle.manifest.object_count != actual_objects.len() {
        errors.push(format!(
            "object count mismatch: expected {} got {}",
            bundle.manifest.object_count,
            actual_objects.len()
        ));
    }

    let recomputed = MemoryPassportManifest::new(
        bundle.manifest.source_scope_id.clone(),
        actual_objects.clone(),
        bundle.manifest.redaction,
        bundle.manifest.encryption.clone(),
        bundle.manifest.exported_at,
    );
    if recomputed.bundle_hash != bundle.manifest.bundle_hash {
        errors.push(format!(
            "bundle hash mismatch: expected {} got {}",
            bundle.manifest.bundle_hash, recomputed.bundle_hash
        ));
    }

    Ok(MemoryPassportVerification {
        manifest: bundle.manifest,
        valid: errors.is_empty(),
        checked_objects: actual_by_key.len(),
        errors,
    })
}

pub fn bundle_json(bundle: &MemoryPassportBundle) -> serde_json::Value {
    json!({
        "manifest": bundle.manifest,
        "memories": bundle.memories,
        "evidence_spans": bundle.evidence_spans,
    })
}

pub fn verification_json(verification: &MemoryPassportVerification) -> serde_json::Value {
    json!({
        "manifest": verification.manifest,
        "valid": verification.valid,
        "checked_objects": verification.checked_objects,
        "errors": verification.errors,
    })
}

fn derive_artifact_evidence_span(memory: &Memory, artifact: &Artifact) -> EvidenceSpan {
    let source_ref = format!("artifact:{}", artifact.id.as_str());
    match artifact.kind {
        ArtifactKind::Image => EvidenceSpan::new_media(
            memory.scope_id.clone(),
            Some(memory.id.clone()),
            Some(artifact.id.clone()),
            source_ref,
            EvidenceSpanKind::ImageRegion,
            compact_quote(&artifact.content_text),
            artifact.content_hash.clone(),
        ),
        ArtifactKind::Audio => EvidenceSpan::new_media(
            memory.scope_id.clone(),
            Some(memory.id.clone()),
            Some(artifact.id.clone()),
            source_ref,
            EvidenceSpanKind::AudioSegment,
            compact_quote(&artifact.content_text),
            artifact.content_hash.clone(),
        ),
        ArtifactKind::Video => EvidenceSpan::new_media(
            memory.scope_id.clone(),
            Some(memory.id.clone()),
            Some(artifact.id.clone()),
            source_ref,
            EvidenceSpanKind::VideoSegment,
            compact_quote(&artifact.content_text),
            artifact.content_hash.clone(),
        ),
        _ => {
            let quote = best_text_quote(memory, &artifact.content_text);
            let start = artifact.content_text.find(&quote).unwrap_or_default();
            let end = start
                .saturating_add(quote.len())
                .min(artifact.content_text.len());
            EvidenceSpan::new_text(
                memory.scope_id.clone(),
                Some(memory.id.clone()),
                Some(artifact.id.clone()),
                source_ref,
                quote,
                artifact.content_hash.clone(),
                start,
                end,
            )
        }
    }
}

fn best_text_quote(memory: &Memory, artifact_text: &str) -> String {
    if artifact_text.contains(&memory.body) {
        return compact_quote(&memory.body);
    }
    compact_quote(artifact_text)
}

fn redact_memory_for_passport(mut memory: Memory) -> Memory {
    let source_ref = format!("passport:{}", memory.id.as_str());
    memory.title = redact_sensitive_text(&memory.scope_id, &source_ref, "title", &memory.title);
    memory.body = redact_sensitive_text(&memory.scope_id, &source_ref, "body", &memory.body);
    memory
}

fn redact_sensitive_text(scope_id: &ScopeId, source_ref: &str, field: &str, text: &str) -> String {
    let findings = SecretDetector::detect_text(scope_id, None, source_ref, field, text);
    let mut output = text.to_string();
    let mut ranges = findings
        .iter()
        .filter(|finding| finding.action != SecretFindingAction::Allow)
        .collect::<Vec<_>>();
    ranges.sort_by(|left, right| right.location.start.cmp(&left.location.start));
    for finding in ranges {
        if finding.location.end <= output.len() {
            output.replace_range(
                finding.location.start..finding.location.end,
                &finding.redacted_preview,
            );
        }
    }
    output
}

fn passport_objects(
    memories: &[Memory],
    evidence_spans: &[EvidenceSpan],
) -> Result<Vec<MemoryPassportObject>> {
    let mut objects = Vec::with_capacity(memories.len() + evidence_spans.len());
    for memory in memories {
        objects.push(MemoryPassportObject::new(
            MemoryPassportObjectKind::Memory,
            memory.id.as_str(),
            serde_json::to_string(memory)?,
        ));
    }
    for span in evidence_spans {
        objects.push(MemoryPassportObject::new(
            MemoryPassportObjectKind::EvidenceSpan,
            span.id.as_str(),
            serde_json::to_string(span)?,
        ));
    }
    Ok(objects)
}

fn object_key(object: &MemoryPassportObject) -> String {
    format!("{}:{}", object.kind.as_str(), object.id)
}

fn compact_quote(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.chars().take(240).collect()
}

fn render_manifest_markdown(manifest: &MemoryPassportManifest) -> String {
    let mut output = format!(
        "# Memory Passport Manifest\n\npassport_id: {}\nschema_version: {}\nsource_scope_id: {}\nobject_count: {}\nredaction: {}\nencryption: {}\nbundle_hash: {}\n\n## Objects\n",
        manifest.id.as_str(),
        manifest.schema_version,
        manifest.source_scope_id.as_str(),
        manifest.object_count,
        manifest.redaction.as_str(),
        manifest.encryption.enabled,
        manifest.bundle_hash,
    );
    for object in &manifest.objects {
        output.push_str(&format!(
            "- kind={} id={} hash={} bytes={}\n",
            object.kind.as_str(),
            object.id,
            object.hash,
            object.byte_len
        ));
    }
    output
}

fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{
        MemoryPassportExportRequest, MemoryPassportImportRequest, build_memory_passport_bundle,
        derive_evidence_spans, read_memory_passport_bundle, redact_sensitive_text,
        verify_memory_passport_bundle, write_memory_passport_bundle,
    };
    use crate::{Kernel, RememberTextRequest};
    use memory_domain::{
        Artifact, ArtifactKind, Memory, MemoryKind, MemoryPassportRedaction, ScopeId,
    };
    use tempfile::tempdir;

    #[test]
    fn derives_text_and_multimodal_evidence_spans() {
        let scope_id = ScopeId::from_string("scp_evidence_kernel");
        let memory = Memory::new(
            scope_id.clone(),
            MemoryKind::Fact,
            "release note",
            "release note body",
        )
        .unwrap();
        let text_artifact = Artifact::new(
            scope_id.clone(),
            ArtifactKind::Document,
            "prefix release note body suffix",
            vec![],
        )
        .unwrap();
        let image_artifact = Artifact::new(
            scope_id,
            ArtifactKind::Image,
            "image caption",
            vec!["asset://image".to_string()],
        )
        .unwrap();

        let text_span = derive_evidence_spans(&memory, Some(&text_artifact));
        let image_span = derive_evidence_spans(&memory, Some(&image_artifact));

        assert_eq!(text_span[0].quote, "release note body");
        assert!(text_span[0].source_ref.starts_with("artifact:art_"));
        assert!(text_span[0].artifact_id.is_some());
        assert_eq!(
            image_span[0].kind,
            memory_domain::EvidenceSpanKind::ImageRegion
        );
    }

    #[test]
    fn passport_writer_and_verifier_check_hash_contract() {
        let scope_id = ScopeId::from_string("scp_passport_kernel");
        let mut memory = Memory::new(scope_id.clone(), MemoryKind::Fact, "title", "body").unwrap();
        memory.source_refs = vec!["file://README.md".to_string()];
        let evidence = derive_evidence_spans(&memory, None);
        let bundle = build_memory_passport_bundle(
            scope_id,
            vec![memory],
            evidence,
            MemoryPassportRedaction::Sensitive,
        )
        .unwrap();
        let tempdir = tempdir().unwrap();

        write_memory_passport_bundle(tempdir.path(), &bundle).unwrap();
        let loaded = read_memory_passport_bundle(tempdir.path()).unwrap();
        let verification = verify_memory_passport_bundle(tempdir.path()).unwrap();

        assert_eq!(loaded.memories.len(), 1);
        assert!(verification.valid);
        assert_eq!(verification.checked_objects, bundle.manifest.object_count);
    }

    #[test]
    fn passport_redaction_removes_high_risk_secret_values() {
        let redacted = redact_sensitive_text(
            &ScopeId::from_string("scp_redact_passport"),
            "passport:test",
            "body",
            "api_key = sk_test_1234567890abcdef",
        );

        assert!(redacted.contains("[REDACTED:API_KEY]"));
        assert!(!redacted.contains("sk_test_1234567890abcdef"));
    }

    #[tokio::test]
    async fn passport_export_verify_import_roundtrips_markdown_scope() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();
        let passport_dir = tempdir().unwrap();
        let source_scope_id = ScopeId::from_string("scp_passport_source");
        let target_scope_id = ScopeId::from_string("scp_passport_target");
        let source_kernel = Kernel::builder()
            .with_markdown_root(source_dir.path())
            .unwrap()
            .build()
            .unwrap();
        let target_kernel = Kernel::builder()
            .with_markdown_root(target_dir.path())
            .unwrap()
            .build()
            .unwrap();
        let mut remember = RememberTextRequest::new(
            source_scope_id.clone(),
            "passport export body with durable project fact",
        );
        remember.title = Some("passport export fact".to_string());
        remember.source_refs = vec!["file://docs/passport.md".to_string()];
        source_kernel.remember_text(remember).await.unwrap();

        let mut export = MemoryPassportExportRequest::new(source_scope_id);
        export.redact_sensitive = true;
        let bundle = source_kernel.export_memory_passport(export).await.unwrap();
        write_memory_passport_bundle(passport_dir.path(), &bundle).unwrap();
        assert!(
            verify_memory_passport_bundle(passport_dir.path())
                .unwrap()
                .valid
        );

        let result = target_kernel
            .import_memory_passport(MemoryPassportImportRequest {
                input_dir: passport_dir.path().to_path_buf(),
                target_scope_id: Some(target_scope_id.clone()),
                dry_run: false,
                context: None,
            })
            .await
            .unwrap();
        let imported = target_kernel
            .browse_memories(Some(target_scope_id), 10)
            .await
            .unwrap();

        assert!(result.verified);
        assert_eq!(result.imported_count, 1);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].title, "passport export fact");
    }
}
