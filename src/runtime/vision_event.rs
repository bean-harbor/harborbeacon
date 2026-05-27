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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LocalVisionDetection {
    pub label: String,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LocalVisionAnalyzerResult {
    #[serde(default)]
    pub detections: Vec<LocalVisionDetection>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub analyzer: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub command_latency_ms: Option<u64>,
    #[serde(default)]
    pub preprocess_ms: Option<u64>,
    #[serde(default)]
    pub inference_ms: Option<u64>,
    #[serde(default)]
    pub postprocess_ms: Option<u64>,
    #[serde(default)]
    pub model_sha256: Option<String>,
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
    pub analyzer_result: Option<LocalVisionAnalyzerResult>,
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
    let analyzer_result = input.analyzer_result;
    let analyzer = analyzer_result
        .as_ref()
        .and_then(|result| result.analyzer.clone())
        .or(input.analyzer)
        .unwrap_or_else(|| "cpu-snapshot-fallback".to_string());
    let event_type = analyzer_result
        .as_ref()
        .and_then(|result| result.event_type.clone())
        .unwrap_or_else(|| {
            if valid_image {
                "motion_like_scene".to_string()
            } else {
                "snapshot_unclassified".to_string()
            }
        });
    let mut labels = vec!["snapshot".to_string(), "local_vision_event".to_string()];
    if valid_image {
        labels.push("image_frame".to_string());
    }
    if analyzer.contains("official-vision-runtime-missing") {
        labels.push("official_vision_runtime_missing".to_string());
    }
    if let Some(result) = analyzer_result.as_ref() {
        for label in &result.labels {
            push_unique_label(&mut labels, label);
        }
        for detection in &result.detections {
            push_unique_label(&mut labels, &detection.label);
        }
        if let Some(provider) = result.provider.as_deref() {
            push_unique_label(&mut labels, provider);
        }
    }

    let mut metrics = input.metrics;
    if !metrics.is_object() {
        metrics = json!({});
    }
    if let Some(map) = metrics.as_object_mut() {
        map.insert("byte_size".to_string(), json!(bytes.len()));
        map.insert("valid_image".to_string(), json!(valid_image));
        map.insert("image_mime_type".to_string(), json!(mime_type));
        if let Some(result) = analyzer_result.as_ref() {
            map.insert("detections".to_string(), json!(result.detections));
            map.insert("detected_labels".to_string(), json!(result.labels));
            map.insert("detector_provider".to_string(), json!(result.provider));
            map.insert("detector_latency_ms".to_string(), json!(result.latency_ms));
            map.insert(
                "detector_command_latency_ms".to_string(),
                json!(result.command_latency_ms),
            );
            map.insert(
                "detector_preprocess_ms".to_string(),
                json!(result.preprocess_ms),
            );
            map.insert(
                "detector_inference_ms".to_string(),
                json!(result.inference_ms),
            );
            map.insert(
                "detector_postprocess_ms".to_string(),
                json!(result.postprocess_ms),
            );
            map.insert("model_sha256".to_string(), json!(result.model_sha256));
        }
    }

    Ok(LocalVisionEvent {
        event_id: format!("lve_{}", Uuid::new_v4().simple()),
        camera_id: input.camera_id,
        event_type: event_type.clone(),
        confidence: analyzer_result
            .as_ref()
            .and_then(|result| result.confidence)
            .unwrap_or(if valid_image { 0.55 } else { 0.10 })
            .clamp(0.0, 1.0),
        labels,
        summary: summarize_local_vision_event(&event_type, analyzer_result.as_ref(), &analyzer),
        snapshot_artifact: SnapshotArtifact {
            artifact_id: Some(format!("artifact_{}", Uuid::new_v4().simple())),
            path: None,
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

pub fn classify_local_vision_event(
    detections: &[LocalVisionDetection],
) -> (String, f32, Vec<String>) {
    let mut labels = Vec::new();
    let mut max_confidence = 0.0f32;
    let mut has_person = false;
    let mut has_pet = false;
    let mut has_vehicle = false;

    for detection in detections {
        let label = detection.label.trim().to_ascii_lowercase();
        if label.is_empty() {
            continue;
        }
        push_unique_label(&mut labels, &label);
        max_confidence = max_confidence.max(detection.confidence);
        if label == "person" {
            has_person = true;
        }
        if matches!(label.as_str(), "cat" | "dog") {
            has_pet = true;
        }
        if matches!(
            label.as_str(),
            "car" | "bus" | "truck" | "motorcycle" | "bicycle"
        ) {
            has_vehicle = true;
        }
    }

    let event_type = if has_person {
        "person_detected"
    } else if has_pet {
        "pet_detected"
    } else if has_vehicle {
        "vehicle_detected"
    } else {
        "motion_like_scene"
    };
    (event_type.to_string(), max_confidence, labels)
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

fn summarize_local_vision_event(
    event_type: &str,
    analyzer_result: Option<&LocalVisionAnalyzerResult>,
    analyzer: &str,
) -> String {
    let detected = analyzer_result
        .map(|result| {
            result
                .labels
                .iter()
                .filter(|label| !label.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let label_text = if detected.is_empty() {
        String::new()
    } else {
        format!(" 检测标签：{}。", detected.join(", "))
    };
    match event_type {
        "person_detected" => format!("K3 本地视觉检测到人员活动。{label_text}"),
        "pet_detected" => format!("K3 本地视觉检测到宠物活动。{label_text}"),
        "vehicle_detected" => format!("K3 本地视觉检测到车辆相关目标。{label_text}"),
        "snapshot_unclassified" => "K3 已抓拍，但图片格式无法识别。".to_string(),
        _ if analyzer.contains("official-vision-runtime-missing") => {
            "K3 已完成本地抓拍并生成轻量视觉事件；官方视觉检测 runtime 尚未形成可用 recipe，当前为 CPU fallback 事件。".to_string()
        }
        _ => format!("K3 已完成本地抓拍并生成本地视觉事件。{label_text}"),
    }
}

fn push_unique_label(labels: &mut Vec<String>, label: &str) {
    let normalized = label.trim().to_ascii_lowercase();
    if !normalized.is_empty() && !labels.iter().any(|existing| existing == &normalized) {
        labels.push(normalized);
    }
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
            analyzer_result: None,
            metrics: json!({"runtime_probe": "missing"}),
        })
        .expect("analyze snapshot");

        assert_eq!(event.event_type, "motion_like_scene");
        assert_eq!(event.latency_ms, 42);
        assert_eq!(
            event.snapshot_artifact.mime_type.as_deref(),
            Some("image/jpeg")
        );
        assert_eq!(event.snapshot_artifact.path, None);
        assert!(event
            .labels
            .contains(&"official_vision_runtime_missing".to_string()));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn classify_local_vision_event_maps_product_labels() {
        let detections = vec![
            detection("car", 0.72),
            detection("dog", 0.81),
            detection("person", 0.66),
        ];

        let (event_type, confidence, labels) = classify_local_vision_event(&detections);

        assert_eq!(event_type, "person_detected");
        assert_eq!(confidence, 0.81);
        assert!(labels.contains(&"person".to_string()));
        assert!(labels.contains(&"dog".to_string()));
        assert!(labels.contains(&"car".to_string()));
    }

    #[test]
    fn analyze_snapshot_file_uses_analyzer_result() {
        let dir = std::env::temp_dir().join(format!("vision-event-yolo-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let image = dir.join("snapshot.jpg");
        fs::write(&image, [0xff, 0xd8, 0xff, 0xdb, 0x00]).expect("write jpg");

        let event = analyze_snapshot_file(LocalSnapshotAnalysisInput {
            camera_id: "cam-1".to_string(),
            snapshot_path: image,
            analyzer: None,
            latency_ms: Some(99),
            analyzer_result: Some(LocalVisionAnalyzerResult {
                detections: vec![detection("person", 0.73)],
                labels: vec!["person".to_string()],
                event_type: Some("person_detected".to_string()),
                confidence: Some(0.73),
                analyzer: Some("spacemit-yolov8n-192x320-short-command".to_string()),
                provider: Some("CPUExecutionProvider".to_string()),
                latency_ms: Some(41),
                command_latency_ms: Some(120),
                preprocess_ms: Some(1),
                inference_ms: Some(2),
                postprocess_ms: Some(3),
                model_sha256: Some("abc".to_string()),
            }),
            metrics: json!({}),
        })
        .expect("analyze snapshot");

        assert_eq!(event.event_type, "person_detected");
        assert_eq!(event.confidence, 0.73);
        assert_eq!(event.analyzer, "spacemit-yolov8n-192x320-short-command");
        assert!(event.summary.contains("人员"));
        assert_eq!(
            event.metrics["detector_provider"],
            json!("CPUExecutionProvider")
        );
        assert_eq!(event.metrics["detector_command_latency_ms"], json!(120));
        let _ = fs::remove_dir_all(dir);
    }

    fn detection(label: &str, confidence: f32) -> LocalVisionDetection {
        LocalVisionDetection {
            label: label.to_string(),
            confidence,
            x1: 1.0,
            y1: 2.0,
            x2: 3.0,
            y2: 4.0,
        }
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
