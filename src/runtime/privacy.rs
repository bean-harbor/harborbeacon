//! Privacy gates for model payloads and redacted visual artifacts.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RedactionDetection {
    pub class: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RedactionManifest {
    pub source_artifact_id: String,
    pub redacted_artifact_id: String,
    pub engine: String,
    pub profile: String,
    pub created_at: String,
    pub image_sha256: String,
    #[serde(default)]
    pub detections: Vec<RedactionDetection>,
    #[serde(default)]
    pub bbox_expansion: f32,
    #[serde(default)]
    pub metadata_stripped: bool,
    #[serde(default)]
    pub cloud_safe: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageRedactionContext {
    pub source_image_path: PathBuf,
    pub redacted_image_path: PathBuf,
    pub target_capability: String,
    pub manifest: RedactionManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyGuardError {
    pub code: String,
    pub message: String,
}

impl PrivacyGuardError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "code": self.code,
            "message": self.message,
        })
    }
}

impl fmt::Display for PrivacyGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PrivacyGuardError {}

pub fn validate_cloud_vlm_redaction(
    context: Option<&ImageRedactionContext>,
) -> Result<&ImageRedactionContext, PrivacyGuardError> {
    let context = context.ok_or_else(|| {
        PrivacyGuardError::new(
            "vpf_manifest_required",
            "cloud VLM requires a VPF redaction manifest",
        )
    })?;
    let manifest = &context.manifest;
    if !context.target_capability.eq_ignore_ascii_case("vlm.cloud") {
        return Err(PrivacyGuardError::new(
            "target_capability_mismatch",
            "redaction manifest target_capability must be vlm.cloud",
        ));
    }
    if !manifest.cloud_safe {
        return Err(PrivacyGuardError::new(
            "manifest_not_cloud_safe",
            "redaction manifest is not marked cloud_safe",
        ));
    }
    if !manifest.metadata_stripped {
        return Err(PrivacyGuardError::new(
            "metadata_not_stripped",
            "redacted artifact metadata must be stripped before cloud VLM",
        ));
    }
    if manifest.source_artifact_id.trim().is_empty()
        || manifest.redacted_artifact_id.trim().is_empty()
    {
        return Err(PrivacyGuardError::new(
            "manifest_artifact_id_missing",
            "redaction manifest must include source and redacted artifact ids",
        ));
    }
    if manifest.source_artifact_id == manifest.redacted_artifact_id {
        return Err(PrivacyGuardError::new(
            "redacted_artifact_matches_source",
            "cloud VLM requires a derivative redacted artifact",
        ));
    }
    if context.source_image_path == context.redacted_image_path {
        return Err(PrivacyGuardError::new(
            "redacted_path_matches_source",
            "cloud VLM must not read the source image path",
        ));
    }

    let manifest_value = serde_json::to_value(manifest).map_err(|error| {
        PrivacyGuardError::new(
            "manifest_serialize_failed",
            format!("failed to serialize redaction manifest: {error}"),
        )
    })?;
    scan_cloud_payload_value(&manifest_value)?;
    scan_cloud_payload_text(&context.target_capability)?;
    scan_cloud_payload_path(&context.redacted_image_path)?;
    Ok(context)
}

pub fn scan_cloud_payload_value(value: &Value) -> Result<(), PrivacyGuardError> {
    match value {
        Value::String(text) => scan_cloud_payload_text(text),
        Value::Array(items) => {
            for item in items {
                scan_cloud_payload_value(item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (key, item) in map {
                scan_cloud_payload_text(key)?;
                scan_cloud_payload_value(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn scan_cloud_payload_text(text: &str) -> Result<(), PrivacyGuardError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let lower = trimmed.to_ascii_lowercase();
    let blocked = [
        ("rtsp_url", "rtsp://"),
        ("ha_token", "ha_token"),
        ("ha_token", "home_assistant_token"),
        ("ha_token", "supervisor_token"),
        ("camera_credential", "camera_credential"),
        ("camera_credential", "rtsp_username"),
        ("camera_credential", "rtsp_password"),
        ("camera_credential", "password="),
        ("api_key", "api_key"),
        ("api_key", "authorization: bearer"),
        ("api_key", "bearer "),
        ("api_key", "sk-"),
        ("local_path", "c:\\"),
        ("local_path", "/home/"),
        ("local_path", "/mnt/"),
        ("local_path", "/var/lib/"),
        ("upload_url", "x-amz-signature="),
        ("upload_url", "presigned"),
    ];
    for (code, needle) in blocked {
        if lower.contains(needle) {
            return Err(PrivacyGuardError::new(
                format!("payload_contains_{code}"),
                "cloud payload contains sensitive material",
            ));
        }
    }
    if looks_like_email(trimmed) {
        return Err(PrivacyGuardError::new(
            "payload_contains_email",
            "cloud payload contains an email-like value",
        ));
    }
    if looks_like_phone(trimmed) {
        return Err(PrivacyGuardError::new(
            "payload_contains_phone",
            "cloud payload contains a phone-like value",
        ));
    }
    if looks_like_url_with_credentials(trimmed) {
        return Err(PrivacyGuardError::new(
            "payload_contains_url_credentials",
            "cloud payload contains URL credentials",
        ));
    }
    Ok(())
}

pub fn scan_cloud_payload_path(path: &Path) -> Result<(), PrivacyGuardError> {
    let text = path.to_string_lossy();
    let lower = text.to_ascii_lowercase();
    if lower.contains("rtsp://") || lower.contains("ha_token") || lower.contains("api_key") {
        return Err(PrivacyGuardError::new(
            "payload_path_sensitive",
            "redacted artifact path contains sensitive material",
        ));
    }
    Ok(())
}

fn looks_like_email(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let token = token.trim_matches(|ch: char| ",;:()[]{}<>\"'".contains(ch));
        let Some((left, right)) = token.split_once('@') else {
            return false;
        };
        !left.is_empty() && right.contains('.') && right.len() >= 4
    })
}

fn looks_like_phone(text: &str) -> bool {
    let mut run = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            run += 1;
            if run >= 10 {
                return true;
            }
        } else if matches!(ch, ' ' | '-' | '(' | ')' | '+') {
            continue;
        } else {
            run = 0;
        }
    }
    false
}

fn looks_like_url_with_credentials(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["http://", "https://", "rtsp://"]
        .iter()
        .filter_map(|scheme| lower.find(scheme).map(|index| index + scheme.len()))
        .any(|start| {
            let tail = &lower[start..];
            let at = tail.find('@');
            let slash = tail.find('/');
            matches!((at, slash), (Some(at), Some(slash)) if at < slash)
                || matches!((at, slash), (Some(_), None))
        })
}

#[cfg(test)]
mod tests {
    use super::{
        scan_cloud_payload_text, validate_cloud_vlm_redaction, ImageRedactionContext,
        RedactionManifest,
    };
    use std::path::PathBuf;

    fn valid_context() -> ImageRedactionContext {
        ImageRedactionContext {
            source_image_path: PathBuf::from("source.jpg"),
            redacted_image_path: PathBuf::from("redacted.jpg"),
            target_capability: "vlm.cloud".to_string(),
            manifest: RedactionManifest {
                source_artifact_id: "artifact-source".to_string(),
                redacted_artifact_id: "artifact-redacted".to_string(),
                engine: "deface-centerface".to_string(),
                profile: "cloud_vlm_default".to_string(),
                created_at: "2026-05-27T00:00:00Z".to_string(),
                image_sha256: "abc123".to_string(),
                bbox_expansion: 1.5,
                metadata_stripped: true,
                cloud_safe: true,
                detections: Vec::new(),
            },
        }
    }

    #[test]
    fn valid_cloud_vlm_redaction_context_passes() {
        let context = valid_context();
        assert!(validate_cloud_vlm_redaction(Some(&context)).is_ok());
    }

    #[test]
    fn missing_manifest_fails_closed() {
        let error = validate_cloud_vlm_redaction(None).expect_err("missing manifest");
        assert_eq!(error.code, "vpf_manifest_required");
    }

    #[test]
    fn unsafe_manifest_fails_closed() {
        let mut context = valid_context();
        context.manifest.cloud_safe = false;
        let error = validate_cloud_vlm_redaction(Some(&context)).expect_err("unsafe");
        assert_eq!(error.code, "manifest_not_cloud_safe");
    }

    #[test]
    fn payload_scan_rejects_sensitive_values() {
        for text in [
            "rtsp://admin:secret@example/stream",
            "api_key=sk-test",
            "call me at 155 5555 5555",
            "mail user@example.com",
            "/home/harbor/private/frame.jpg",
        ] {
            assert!(scan_cloud_payload_text(text).is_err(), "{text}");
        }
    }
}
