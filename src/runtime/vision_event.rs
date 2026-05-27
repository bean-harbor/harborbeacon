//! Local vision event records for HarborNavi K3 viability testing.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const VISION_EVENT_STORE_PATH_ENV: &str = "HARBOR_VISION_EVENT_STORE_PATH";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SnapshotArtifact {
    #[serde(default)]
    pub artifact_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub byte_size: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalVisionEvent {
    pub event_id: String,
    pub camera_id: String,
    pub event_type: String,
    pub confidence: f32,
    #[serde(default)]
    pub labels: Vec<String>,
    pub summary: String,
    pub snapshot_artifact: SnapshotArtifact,
    pub started_at: String,
    pub analyzer: String,
    pub latency_ms: u64,
    #[serde(default)]
    pub metrics: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredLocalVisionEvent {
    pub received_at: String,
    pub event: LocalVisionEvent,
    pub audit_record: Value,
    pub ha_mqtt_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalSnapshotAnalysisInput {
    pub camera_id: String,
    pub snapshot_path: PathBuf,
    #[serde(default)]
    pub analyzer: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub metrics: Value,
}

pub fn default_vision_event_store_path() -> PathBuf {
    std::env::var(VISION_EVENT_STORE_PATH_ENV)
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".harborbeacon/vision-events/events.jsonl"))
}

pub fn analyze_snapshot_file(
    input: LocalSnapshotAnalysisInput,
) -> Result<LocalVisionEvent, String> {
    let started_at = now_epoch_ms_string();
    let bytes = fs::read(&input.snapshot_path).map_err(|error| {
        format!(
            "failed to read snapshot {}: {error}",
            input.snapshot_path.display()
        )
    })?;
    let sha256 = hex_sha256(&bytes);
    let mime_type = infer_image_mime_type(&bytes);
    let valid_image = mime_type != "application/octet-stream";
    let analyzer = input
        .analyzer
        .unwrap_or_else(|| "cpu-snapshot-fallback".to_string());
    let event_type = if valid_image {
        "motion_like_scene"
    } else {
        "snapshot_unclassified"
    };
    let mut labels = vec!["snapshot".to_string(), "local_vision_event".to_string()];
    if valid_image {
        labels.push("image_frame".to_string());
    }
    if analyzer.contains("official-vision-runtime-missing") {
        labels.push("official_vision_runtime_missing".to_string());
    }

    let mut metrics = input.metrics;
    if !metrics.is_object() {
        metrics = json!({});
    }
    if let Some(map) = metrics.as_object_mut() {
        map.insert("byte_size".to_string(), json!(bytes.len()));
        map.insert("valid_image".to_string(), json!(valid_image));
        map.insert("image_mime_type".to_string(), json!(mime_type));
    }

    Ok(LocalVisionEvent {
        event_id: format!("lve_{}", Uuid::new_v4().simple()),
        camera_id: input.camera_id,
        event_type: event_type.to_string(),
        confidence: if valid_image { 0.55 } else { 0.10 },
        labels,
        summary: if analyzer.contains("official-vision-runtime-missing") {
            "K3 已完成本地抓拍并生成轻量视觉事件；官方视觉检测 runtime 尚未形成可用 recipe，当前为 CPU fallback 事件。".to_string()
        } else {
            "K3 已完成本地抓拍并生成轻量视觉事件。".to_string()
        },
        snapshot_artifact: SnapshotArtifact {
            artifact_id: Some(format!("artifact_{}", Uuid::new_v4().simple())),
            path: Some(input.snapshot_path.to_string_lossy().to_string()),
            mime_type: Some(mime_type.to_string()),
            byte_size: Some(bytes.len() as u64),
            sha256: Some(sha256),
            source: Some("k3-local-snapshot".to_string()),
        },
        started_at,
        analyzer,
        latency_ms: input.latency_ms.unwrap_or_default(),
        metrics,
    })
}

pub fn ingest_local_vision_event_default(
    event: LocalVisionEvent,
) -> Result<StoredLocalVisionEvent, String> {
    ingest_local_vision_event(&default_vision_event_store_path(), event)
}

pub fn ingest_local_vision_event(
    store_path: &Path,
    event: LocalVisionEvent,
) -> Result<StoredLocalVisionEvent, String> {
    validate_local_vision_event(&event)?;
    let stored = StoredLocalVisionEvent {
        received_at: now_epoch_ms_string(),
        audit_record: build_audit_record(&event),
        ha_mqtt_payload: build_ha_mqtt_payload(&event),
        event,
    };
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create vision event store directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(store_path)
        .map_err(|error| {
            format!(
                "failed to open vision event store {}: {error}",
                store_path.display()
            )
        })?;
    let line = serde_json::to_string(&stored)
        .map_err(|error| format!("failed to serialize local vision event: {error}"))?;
    writeln!(file, "{line}")
        .map_err(|error| format!("failed to append local vision event: {error}"))?;
    Ok(stored)
}

pub fn build_ha_mqtt_payload(event: &LocalVisionEvent) -> Value {
    json!({
        "event_id": event.event_id,
        "camera_id": event.camera_id,
        "event_type": event.event_type,
        "confidence": event.confidence,
        "labels": event.labels,
        "summary": event.summary,
        "started_at": event.started_at,
        "latency_ms": event.latency_ms,
        "analyzer": event.analyzer,
        "snapshot_artifact": {
            "artifact_id": event.snapshot_artifact.artifact_id,
            "mime_type": event.snapshot_artifact.mime_type,
            "byte_size": event.snapshot_artifact.byte_size,
            "sha256": event.snapshot_artifact.sha256,
            "source": event.snapshot_artifact.source,
        },
    })
}

pub fn validate_local_vision_event(event: &LocalVisionEvent) -> Result<(), String> {
    if event.event_id.trim().is_empty() {
        return Err("local vision event requires event_id".to_string());
    }
    if event.camera_id.trim().is_empty() {
        return Err("local vision event requires camera_id".to_string());
    }
    if event.event_type.trim().is_empty() {
        return Err("local vision event requires event_type".to_string());
    }
    if !(0.0..=1.0).contains(&event.confidence) {
        return Err("local vision event confidence must be between 0 and 1".to_string());
    }
    let payload = serde_json::to_value(event)
        .map_err(|error| format!("failed to inspect local vision event: {error}"))?;
    reject_sensitive_value(&payload)
}

fn build_audit_record(event: &LocalVisionEvent) -> Value {
    json!({
        "audit_kind": "local_vision_event.ingested",
        "event_id": event.event_id,
        "camera_id": event.camera_id,
        "event_type": event.event_type,
        "confidence": event.confidence,
        "labels": event.labels,
        "analyzer": event.analyzer,
        "latency_ms": event.latency_ms,
        "snapshot_artifact_id": event.snapshot_artifact.artifact_id,
        "snapshot_sha256": event.snapshot_artifact.sha256,
    })
}

fn reject_sensitive_value(value: &Value) -> Result<(), String> {
    match value {
        Value::String(text) => reject_sensitive_text(text),
        Value::Array(items) => {
            for item in items {
                reject_sensitive_value(item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (key, item) in map {
                reject_sensitive_text(key)?;
                reject_sensitive_value(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn reject_sensitive_text(text: &str) -> Result<(), String> {
    let lower = text.to_ascii_lowercase();
    let blocked = [
        ("rtsp_url", "rtsp://"),
        ("ha_token", "ha_token"),
        ("ha_token", "home_assistant_token"),
        ("camera_credential", "camera_credential"),
        ("camera_credential", "rtsp_username"),
        ("camera_credential", "rtsp_password"),
        ("camera_credential", "password="),
        ("api_key", "api_key"),
        ("api_key", "authorization: bearer"),
        ("api_key", "bearer "),
        ("api_key", "sk-"),
        ("private_key", "private key"),
        ("upload_url", "x-amz-signature="),
        ("upload_url", "presigned"),
    ];
    for (code, needle) in blocked {
        if lower.contains(needle) {
            return Err(format!(
                "local vision event contains sensitive material: {code}"
            ));
        }
    }
    if looks_like_url_with_credentials(&lower) {
        return Err("local vision event contains URL credentials".to_string());
    }
    Ok(())
}

fn infer_image_mime_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_epoch_ms_string() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("epoch_ms:{millis}")
}

fn looks_like_url_with_credentials(text: &str) -> bool {
    ["http://", "https://", "rtsp://"]
        .iter()
        .filter_map(|scheme| text.find(scheme).map(|index| index + scheme.len()))
        .any(|start| {
            let tail = &text[start..];
            let at = tail.find('@');
            let slash = tail.find('/');
            matches!((at, slash), (Some(at), Some(slash)) if at < slash)
                || matches!((at, slash), (Some(_), None))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_vision_event_rejects_rtsp_url() {
        let mut event = sample_event();
        event.snapshot_artifact.path = Some("rtsp://user:pass@192.168.1.2/stream1".to_string());

        let error = validate_local_vision_event(&event).expect_err("sensitive url must fail");

        assert!(error.contains("rtsp_url") || error.contains("URL credentials"));
    }

    #[test]
    fn ha_mqtt_payload_omits_local_snapshot_path() {
        let event = sample_event();

        let payload = build_ha_mqtt_payload(&event);
        let text = serde_json::to_string(&payload).expect("serialize payload");

        assert!(!text.contains("/tmp/source.jpg"));
        assert_eq!(payload["camera_id"], json!("cam-1"));
        assert_eq!(payload["snapshot_artifact"]["sha256"], json!("abc123"));
    }

    #[test]
    fn ingest_appends_jsonl_audit_record() {
        let dir = std::env::temp_dir().join(format!("vision-event-test-{}", Uuid::new_v4()));
        let store = dir.join("events.jsonl");

        let stored = ingest_local_vision_event(&store, sample_event()).expect("ingest event");

        let text = fs::read_to_string(&store).expect("read store");
        assert!(stored.audit_record["audit_kind"] == json!("local_vision_event.ingested"));
        assert!(text.contains("local_vision_event.ingested"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn analyze_snapshot_file_builds_fallback_event() {
        let dir = std::env::temp_dir().join(format!("vision-event-image-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let image = dir.join("snapshot.jpg");
        fs::write(&image, [0xff, 0xd8, 0xff, 0xdb, 0x00]).expect("write jpg");

        let event = analyze_snapshot_file(LocalSnapshotAnalysisInput {
            camera_id: "cam-1".to_string(),
            snapshot_path: image,
            analyzer: Some("official-vision-runtime-missing+cpu-snapshot-fallback".to_string()),
            latency_ms: Some(42),
            metrics: json!({"runtime_probe": "missing"}),
        })
        .expect("analyze snapshot");

        assert_eq!(event.event_type, "motion_like_scene");
        assert_eq!(event.latency_ms, 42);
        assert_eq!(
            event.snapshot_artifact.mime_type.as_deref(),
            Some("image/jpeg")
        );
        assert!(event
            .labels
            .contains(&"official_vision_runtime_missing".to_string()));
        let _ = fs::remove_dir_all(dir);
    }

    fn sample_event() -> LocalVisionEvent {
        LocalVisionEvent {
            event_id: "event-1".to_string(),
            camera_id: "cam-1".to_string(),
            event_type: "motion_like_scene".to_string(),
            confidence: 0.55,
            labels: vec!["snapshot".to_string()],
            summary: "本地视觉事件".to_string(),
            snapshot_artifact: SnapshotArtifact {
                artifact_id: Some("artifact-1".to_string()),
                path: Some("/tmp/source.jpg".to_string()),
                mime_type: Some("image/jpeg".to_string()),
                byte_size: Some(5),
                sha256: Some("abc123".to_string()),
                source: Some("k3-local-snapshot".to_string()),
            },
            started_at: "epoch_ms:1".to_string(),
            analyzer: "cpu-snapshot-fallback".to_string(),
            latency_ms: 100,
            metrics: json!({}),
        }
    }
}
