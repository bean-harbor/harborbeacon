//! Local vision event records for HarborNavi K3 viability testing.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::connectors::notifications::{
    NotificationContent, NotificationDelivery, NotificationDeliveryMode, NotificationDestination,
    NotificationDestinationKind, NotificationMetadata, NotificationPayloadFormat,
    NotificationRequest, NotificationSource,
};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm: Option<LocalVisionEventVlmSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LocalVisionEventVlmSummary {
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub derived_text: String,
    #[serde(default)]
    pub artifacts: Vec<LocalVisionEventArtifact>,
    #[serde(default)]
    pub ingest_metadata: Value,
    #[serde(default)]
    pub vlm_metrics: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LocalVisionEventArtifact {
    #[serde(default)]
    pub artifact_id: Option<String>,
    pub role: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub byte_size: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
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
pub struct LocalVisionNotificationIntent {
    pub notification_request: NotificationRequest,
    pub audit_record: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalVisionHaMqttContract {
    pub payload: Value,
    pub audit_record: Value,
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
        vlm: None,
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

pub fn list_recent_local_vision_events_default(
    limit: usize,
) -> Result<Vec<StoredLocalVisionEvent>, String> {
    list_recent_local_vision_events(&default_vision_event_store_path(), limit)
}

pub fn list_recent_local_vision_events(
    store_path: &Path,
    limit: usize,
) -> Result<Vec<StoredLocalVisionEvent>, String> {
    if !store_path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(store_path).map_err(|error| {
        format!(
            "failed to open vision event store {}: {error}",
            store_path.display()
        )
    })?;
    let mut events = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| format!("failed to read vision event store: {error}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event = serde_json::from_str::<StoredLocalVisionEvent>(trimmed)
            .map_err(|error| format!("failed to parse stored local vision event: {error}"))?;
        events.push(event);
    }
    events.reverse();
    events.truncate(limit.max(1));
    Ok(events)
}

pub fn build_ha_mqtt_payload(event: &LocalVisionEvent) -> Value {
    json!({
        "event_id": event.event_id,
        "camera_id": event.camera_id,
        "event_type": event.event_type,
        "confidence": event.confidence,
        "labels": event.labels,
        "summary": local_vision_automation_summary(event),
        "started_at": event.started_at,
        "latency_ms": event.latency_ms,
        "analyzer": event.analyzer,
        "vlm_status": local_vision_vlm_status(event),
    })
}

pub fn build_local_vision_ha_mqtt_contract(
    stored: &StoredLocalVisionEvent,
) -> Result<LocalVisionHaMqttContract, String> {
    validate_local_vision_event(&stored.event)?;
    let payload = build_ha_mqtt_payload(&stored.event);
    validate_ha_mqtt_payload(&payload)?;
    Ok(LocalVisionHaMqttContract {
        audit_record: build_local_vision_ha_mqtt_audit(stored, &payload),
        payload,
    })
}

pub fn validate_ha_mqtt_payload(payload: &Value) -> Result<(), String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "HA/MQTT local vision payload must be a JSON object".to_string())?;
    let required = [
        "event_id",
        "camera_id",
        "event_type",
        "confidence",
        "labels",
        "summary",
        "started_at",
        "analyzer",
        "latency_ms",
        "vlm_status",
    ];
    for key in required {
        if !object.contains_key(key) {
            return Err(format!("HA/MQTT local vision payload missing {key}"));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) {
            return Err(format!(
                "HA/MQTT local vision payload contains non-contract field {key}"
            ));
        }
    }
    if !matches!(
        payload.get("event_type").and_then(Value::as_str),
        Some("person_detected" | "pet_detected" | "vehicle_detected" | "motion_like_scene")
    ) {
        return Err("HA/MQTT local vision payload has unsupported event_type".to_string());
    }
    reject_sensitive_value(payload)?;
    reject_local_path_value(payload)?;
    Ok(())
}

pub fn build_local_vision_notification_intent(
    stored: &StoredLocalVisionEvent,
    route_key: &str,
) -> Result<LocalVisionNotificationIntent, String> {
    let notification_request = build_local_vision_notification_request(stored, route_key)?;
    let audit_record = build_local_vision_notification_audit(stored, &notification_request);
    Ok(LocalVisionNotificationIntent {
        notification_request,
        audit_record,
    })
}

pub fn build_local_vision_notification_request(
    stored: &StoredLocalVisionEvent,
    route_key: &str,
) -> Result<NotificationRequest, String> {
    validate_local_vision_event(&stored.event)?;
    let route_key = route_key.trim();
    if route_key.is_empty() {
        return Err("local vision notification requires destination.route_key".to_string());
    }
    let event = &stored.event;
    let digest = stable_event_notification_digest(event);
    let request = NotificationRequest {
        notification_id: format!("notif_{}", &digest[..24]),
        trace_id: format!("trace_{}", event.event_id.trim()),
        source: NotificationSource {
            service: "harborbeacon".to_string(),
            module: "local_vision_event".to_string(),
            event_type: "harbornavi.local_vision_event".to_string(),
        },
        destination: NotificationDestination {
            kind: NotificationDestinationKind::Conversation,
            route_key: route_key.to_string(),
            id: String::new(),
            platform: String::new(),
            recipient: None,
        },
        content: NotificationContent {
            title: local_vision_notification_title(event),
            body: local_vision_notification_body(event),
            payload_format: NotificationPayloadFormat::PlainText,
            structured_payload: local_vision_notification_metadata(stored),
            attachments: Vec::new(),
        },
        delivery: NotificationDelivery {
            mode: NotificationDeliveryMode::Send,
            reply_to_message_id: String::new(),
            update_message_id: String::new(),
            idempotency_key: format!("idem_{}", &digest[..24]),
        },
        metadata: NotificationMetadata {
            correlation_id: event.event_id.clone(),
        },
    };
    validate_local_vision_notification_request(&request)?;
    Ok(request)
}

pub fn validate_local_vision_notification_request(
    request: &NotificationRequest,
) -> Result<(), String> {
    if request.content.attachments.is_empty() {
        let payload = serde_json::to_value(request)
            .map_err(|error| format!("failed to inspect local vision notification: {error}"))?;
        reject_sensitive_value(&payload)?;
        reject_local_path_value(&payload)?;
        Ok(())
    } else {
        Err(
            "local vision notification must be text-only and cannot include attachments"
                .to_string(),
        )
    }
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

fn build_local_vision_notification_audit(
    stored: &StoredLocalVisionEvent,
    request: &NotificationRequest,
) -> Value {
    json!({
        "audit_kind": "local_vision_event.notification_intent_built",
        "event_id": stored.event.event_id,
        "camera_id": stored.event.camera_id,
        "event_type": stored.event.event_type,
        "notification_id": request.notification_id,
        "trace_id": request.trace_id,
        "delivery_mode": "send",
        "destination_route_bound": !request.destination.route_key.trim().is_empty(),
        "text_only": true,
        "attachments_included": false,
        "raw_image_included": false,
        "local_paths_included": false,
        "vlm_status": stored.event.vlm.as_ref().map(|vlm| vlm.status.clone()).unwrap_or_else(|| "not_sampled".to_string()),
        "classification": "p1_support_notification_smoke",
    })
}

fn build_local_vision_ha_mqtt_audit(stored: &StoredLocalVisionEvent, payload: &Value) -> Value {
    json!({
        "audit_kind": "local_vision_event.ha_mqtt_payload_built",
        "event_id": stored.event.event_id,
        "camera_id": stored.event.camera_id,
        "event_type": stored.event.event_type,
        "classification": "p1_ha_mqtt_event_contract",
        "metadata_only": true,
        "secret_scan": "clean",
        "raw_image_included": false,
        "local_paths_included": false,
        "payload_fields": payload.as_object().map(|object| object.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
    })
}

fn local_vision_notification_title(event: &LocalVisionEvent) -> String {
    match event.event_type.as_str() {
        "person_detected" => "HarborNavi 人员事件".to_string(),
        "pet_detected" => "HarborNavi 宠物事件".to_string(),
        "vehicle_detected" => "HarborNavi 车辆事件".to_string(),
        _ => "HarborNavi 家庭事件".to_string(),
    }
}

fn local_vision_notification_body(event: &LocalVisionEvent) -> String {
    let summary = local_vision_notification_summary(event);
    let confidence = (event.confidence.clamp(0.0, 1.0) * 100.0).round() as u32;
    format!(
        "{}：{}（摄像头 {}，置信度 {}%，延迟 {}ms）。",
        local_vision_event_type_label(&event.event_type),
        summary,
        event.camera_id,
        confidence,
        event.latency_ms
    )
}

fn local_vision_notification_summary(event: &LocalVisionEvent) -> String {
    local_vision_automation_summary(event)
}

fn local_vision_automation_summary(event: &LocalVisionEvent) -> String {
    if let Some(vlm) = event.vlm.as_ref() {
        if vlm.status == "active" && !vlm.summary.trim().is_empty() {
            return sanitize_model_text(vlm.summary.trim());
        }
        if !vlm.derived_text.trim().is_empty() {
            return sanitize_model_text(vlm.derived_text.trim());
        }
    }
    sanitize_model_text(event.summary.trim())
}

fn local_vision_vlm_status(event: &LocalVisionEvent) -> String {
    event
        .vlm
        .as_ref()
        .map(|vlm| {
            let status = vlm.status.trim();
            if status.is_empty() {
                "degraded".to_string()
            } else {
                status.to_string()
            }
        })
        .unwrap_or_else(|| "not_sampled".to_string())
}

fn local_vision_event_type_label(event_type: &str) -> &'static str {
    match event_type {
        "person_detected" => "检测到人员活动",
        "pet_detected" => "检测到宠物活动",
        "vehicle_detected" => "检测到车辆相关目标",
        "motion_like_scene" => "检测到画面变化",
        _ => "检测到本地视觉事件",
    }
}

fn local_vision_notification_metadata(stored: &StoredLocalVisionEvent) -> Value {
    let event = &stored.event;
    json!({
        "kind": "harbornavi.local_vision_event_notification",
        "event": {
            "event_id": event.event_id,
            "camera_id": event.camera_id,
            "event_type": event.event_type,
            "confidence": event.confidence,
            "labels": event.labels,
            "summary": local_vision_notification_summary(event),
            "started_at": event.started_at,
            "received_at": stored.received_at,
            "analyzer": event.analyzer,
            "latency_ms": event.latency_ms,
            "vlm_status": event.vlm.as_ref().map(|vlm| vlm.status.clone()).unwrap_or_else(|| "not_sampled".to_string()),
            "vlm_summary_present": event.vlm.as_ref().map(|vlm| !vlm.summary.trim().is_empty()).unwrap_or(false),
        },
        "privacy": {
            "text_only": true,
            "attachments_included": false,
            "raw_image_included": false,
            "local_paths_included": false,
        },
    })
}

fn stable_event_notification_digest(event: &LocalVisionEvent) -> String {
    hex_sha256(
        format!(
            "harbornavi.local_vision_event.notification:{}",
            event.event_id.trim()
        )
        .as_bytes(),
    )
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

pub fn attach_vlm_summary_to_event(
    mut event: LocalVisionEvent,
    vlm: crate::runtime::model_center::VlmSummaryExecution,
    elapsed_ms: u64,
    mut extra_metrics: Value,
) -> LocalVisionEvent {
    if !extra_metrics.is_object() {
        extra_metrics = json!({});
    }
    if let Some(map) = extra_metrics.as_object_mut() {
        map.insert("elapsed_ms".to_string(), json!(elapsed_ms));
        map.insert("provider_key".to_string(), json!(vlm.provider_key));
        map.insert(
            "model_endpoint_id".to_string(),
            json!(vlm.model_endpoint_id),
        );
        map.insert("available".to_string(), json!(vlm.available));
    }
    let vlm_text = vlm.text.trim();
    let available = vlm.available && !vlm_text.is_empty();
    let status = if available {
        "active"
    } else if vlm.status.trim().is_empty() {
        "degraded"
    } else {
        vlm.status.trim()
    };
    let summary = if available {
        sanitize_model_text(vlm_text)
    } else {
        event.summary.clone()
    };
    let mut tags = vec!["vlm".to_string(), "sampled_event_frame".to_string()];
    if !available {
        tags.push("vlm_degraded".to_string());
    }
    push_unique_label(&mut tags, &event.event_type);
    let mut labels = event.labels.clone();
    push_unique_label(&mut labels, "vlm_summary");
    if available {
        push_unique_label(&mut labels, "visual_semantics");
        event.summary = summary.clone();
    } else {
        push_unique_label(&mut labels, "vlm_degraded");
    }
    event.labels = labels.clone();
    let artifact = LocalVisionEventArtifact {
        artifact_id: event.snapshot_artifact.artifact_id.clone(),
        role: "sampled_event_frame".to_string(),
        mime_type: event.snapshot_artifact.mime_type.clone(),
        byte_size: event.snapshot_artifact.byte_size,
        sha256: event.snapshot_artifact.sha256.clone(),
        source: event.snapshot_artifact.source.clone(),
    };
    let error = if available {
        None
    } else {
        Some(sanitize_model_text(&vlm.summary))
    };
    let vlm_summary = LocalVisionEventVlmSummary {
        status: status.to_string(),
        summary: summary.clone(),
        tags,
        labels,
        derived_text: if available { summary } else { String::new() },
        artifacts: vec![artifact],
        ingest_metadata: json!({
            "source": "local_on_demand_vlm",
            "trigger": "sampled_event_frame",
            "event_id": event.event_id,
            "camera_id": event.camera_id,
            "frame_path_redacted": true,
            "raw_response_stored": false,
        }),
        vlm_metrics: extra_metrics,
        error,
    };
    if let Some(map) = event.metrics.as_object_mut() {
        map.insert("vlm_status".to_string(), json!(vlm_summary.status.clone()));
        map.insert("vlm_ms".to_string(), json!(elapsed_ms));
    }
    event.vlm = Some(vlm_summary);
    event
}

fn build_audit_record(event: &LocalVisionEvent) -> Value {
    let mut record = json!({
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
    });
    if let Some(vlm) = event.vlm.as_ref() {
        record["vlm"] = json!({
            "status": vlm.status,
            "summary": vlm.summary,
            "model_endpoint_id": vlm.vlm_metrics.get("model_endpoint_id"),
            "provider_key": vlm.vlm_metrics.get("provider_key"),
            "elapsed_ms": vlm.vlm_metrics.get("elapsed_ms"),
            "frame_path_redacted": true,
        });
    }
    record
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

fn reject_local_path_value(value: &Value) -> Result<(), String> {
    match value {
        Value::String(text) => reject_local_path_text(text),
        Value::Array(items) => {
            for item in items {
                reject_local_path_value(item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (key, item) in map {
                reject_local_path_text(key)?;
                reject_local_path_value(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn reject_local_path_text(text: &str) -> Result<(), String> {
    let lower = text.to_ascii_lowercase().replace('\\', "/");
    let blocked = [
        ".harborbeacon/",
        "/tmp/",
        "/var/tmp/",
        "/var/lib/",
        "/home/",
        "/run/",
        "c:/users/",
    ];
    if blocked.iter().any(|needle| lower.contains(needle)) {
        return Err("local vision notification contains local path material".to_string());
    }
    Ok(())
}

fn sanitize_model_text(text: &str) -> String {
    let mut sanitized = text.to_string();
    for needle in [
        "rtsp://",
        "ha_token",
        "home_assistant_token",
        "camera_credential",
        "rtsp_username",
        "rtsp_password",
        "api_key",
        "authorization: bearer",
        "bearer ",
        "sk-",
        "private key",
        "x-amz-signature=",
        "presigned",
    ] {
        sanitized = sanitized.replace(needle, "[redacted]");
        sanitized = sanitized.replace(&needle.to_ascii_uppercase(), "[redacted]");
    }
    sanitized.chars().take(500).collect()
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
        assert!(payload.get("snapshot_artifact").is_none());
        assert_eq!(payload["camera_id"], json!("cam-1"));
        assert_eq!(payload["vlm_status"], json!("not_sampled"));
        validate_ha_mqtt_payload(&payload).expect("valid HA/MQTT payload");
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
    fn list_recent_events_returns_newest_first() {
        let dir = std::env::temp_dir().join(format!("vision-event-list-{}", Uuid::new_v4()));
        let store = dir.join("events.jsonl");
        let mut first = sample_event();
        first.event_id = "event-1".to_string();
        let mut second = sample_event();
        second.event_id = "event-2".to_string();

        ingest_local_vision_event(&store, first).expect("ingest first");
        ingest_local_vision_event(&store, second).expect("ingest second");

        let events = list_recent_local_vision_events(&store, 1).expect("list events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.event_id, "event-2");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn attach_vlm_summary_adds_structured_fields_without_path() {
        let event = sample_event();
        let event = attach_vlm_summary_to_event(
            event,
            crate::runtime::model_center::VlmSummaryExecution {
                available: true,
                status: "active".to_string(),
                summary: "ok".to_string(),
                provider_key: "openai_compatible".to_string(),
                model_endpoint_id: Some("vlm-local-openai-compatible".to_string()),
                text: "画面中有一辆蓝色巴士。".to_string(),
                details: json!({"raw_response": "not persisted"}),
            },
            1961,
            json!({"model_id": "qwen3_5vl_0.8b-text-q41.gguf"}),
        );

        let vlm = event.vlm.expect("vlm summary");
        assert_eq!(vlm.status, "active");
        assert!(vlm.derived_text.contains("蓝色巴士"));
        assert_eq!(vlm.vlm_metrics["elapsed_ms"], json!(1961));
        assert_eq!(vlm.ingest_metadata["frame_path_redacted"], json!(true));
        let text = serde_json::to_string(&vlm).expect("serialize vlm");
        assert!(!text.contains("/tmp/source.jpg"));
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

    #[test]
    fn ha_mqtt_payload_supports_p1_event_types_as_metadata_only() {
        for event_type in [
            "person_detected",
            "pet_detected",
            "vehicle_detected",
            "motion_like_scene",
        ] {
            let mut event = sample_event();
            event.event_type = event_type.to_string();
            event.summary = format!("summary for {event_type}");
            let stored = sample_stored_event(event);

            let contract = build_local_vision_ha_mqtt_contract(&stored).expect("HA/MQTT contract");
            let payload = contract.payload;
            let keys = payload
                .as_object()
                .expect("payload object")
                .keys()
                .cloned()
                .collect::<Vec<_>>();

            assert_eq!(payload["event_type"], json!(event_type));
            assert_eq!(payload["camera_id"], json!("cam-1"));
            assert_eq!(payload["vlm_status"], json!("not_sampled"));
            assert_eq!(keys.len(), 10);
            assert!(payload.get("snapshot_artifact").is_none());
            assert!(payload.get("vlm").is_none());
            assert_eq!(
                contract.audit_record["audit_kind"],
                json!("local_vision_event.ha_mqtt_payload_built")
            );
            assert_eq!(contract.audit_record["metadata_only"], json!(true));
        }
    }

    #[test]
    fn ha_mqtt_payload_uses_active_vlm_summary_but_omits_vlm_artifacts() {
        let mut event = sample_event();
        event.vlm = Some(LocalVisionEventVlmSummary {
            status: "active".to_string(),
            summary: "画面中有人从门口经过。".to_string(),
            tags: vec!["person".to_string()],
            labels: vec!["person".to_string()],
            derived_text: "画面中有人从门口经过。".to_string(),
            artifacts: vec![LocalVisionEventArtifact {
                artifact_id: Some("artifact-sensitive-frame".to_string()),
                role: "sampled_event_frame".to_string(),
                mime_type: Some("image/jpeg".to_string()),
                byte_size: Some(123),
                sha256: Some("abc".to_string()),
                source: Some("k3-local-snapshot".to_string()),
            }],
            ingest_metadata: json!({"frame_path_redacted": true}),
            vlm_metrics: json!({"elapsed_ms": 1800}),
            error: None,
        });

        let payload = build_ha_mqtt_payload(&event);
        let text = serde_json::to_string(&payload).expect("serialize payload");

        assert_eq!(payload["summary"], json!("画面中有人从门口经过。"));
        assert_eq!(payload["vlm_status"], json!("active"));
        assert!(!text.contains("artifact-sensitive-frame"));
        assert!(!text.contains("elapsed_ms"));
        validate_ha_mqtt_payload(&payload).expect("valid HA/MQTT payload");
    }

    #[test]
    fn ha_mqtt_payload_degraded_vlm_does_not_block_base_event() {
        let mut event = sample_event();
        event.event_type = "pet_detected".to_string();
        event.summary = "K3 本地视觉检测到宠物活动。".to_string();
        event.vlm = Some(LocalVisionEventVlmSummary {
            status: "degraded".to_string(),
            summary: "VLM unavailable".to_string(),
            tags: Vec::new(),
            labels: Vec::new(),
            derived_text: String::new(),
            artifacts: Vec::new(),
            ingest_metadata: json!({}),
            vlm_metrics: json!({}),
            error: Some("timeout".to_string()),
        });

        let payload = build_ha_mqtt_payload(&event);

        assert_eq!(payload["event_type"], json!("pet_detected"));
        assert_eq!(payload["summary"], json!("K3 本地视觉检测到宠物活动。"));
        assert_eq!(payload["vlm_status"], json!("degraded"));
        validate_ha_mqtt_payload(&payload).expect("valid HA/MQTT payload");
    }

    #[test]
    fn ha_mqtt_payload_rejects_non_contract_fields_and_sensitive_text() {
        let mut payload = build_ha_mqtt_payload(&sample_event());
        payload["debug_path"] = json!("/tmp/source.jpg");

        let error = validate_ha_mqtt_payload(&payload).expect_err("extra field must fail");

        assert!(error.contains("non-contract field"));

        let mut payload = build_ha_mqtt_payload(&sample_event());
        payload["summary"] = json!("camera rtsp://user:pass@192.168.3.231/stream2");

        let error = validate_ha_mqtt_payload(&payload).expect_err("secret must fail");
        assert!(error.contains("sensitive") || error.contains("URL credentials"));
    }

    #[test]
    fn local_vision_notification_uses_yolo_summary_without_attachments_or_paths() {
        let stored = sample_stored_event(sample_event());

        let intent = build_local_vision_notification_intent(&stored, "gw_route_harbornavi_dev")
            .expect("notification intent");
        let request = intent.notification_request;
        let text = serde_json::to_string(&request).expect("serialize request");

        assert!(request.notification_id.starts_with("notif_"));
        assert!(request.delivery.idempotency_key.starts_with("idem_"));
        assert_eq!(request.source.module, "local_vision_event");
        assert_eq!(request.destination.route_key, "gw_route_harbornavi_dev");
        assert!(request.content.body.contains("本地视觉事件"));
        assert!(request.content.attachments.is_empty());
        assert!(!text.contains("/tmp/source.jpg"));
        assert!(!text.contains("rtsp://"));
        assert_eq!(
            intent.audit_record["audit_kind"],
            json!("local_vision_event.notification_intent_built")
        );
        assert_eq!(intent.audit_record["text_only"], json!(true));
    }

    #[test]
    fn local_vision_notification_prefers_active_vlm_summary() {
        let mut event = sample_event();
        event.event_type = "vehicle_detected".to_string();
        event.vlm = Some(LocalVisionEventVlmSummary {
            status: "active".to_string(),
            summary: "画面中有一辆蓝色车辆停在门口。".to_string(),
            tags: vec!["vehicle".to_string()],
            labels: vec!["car".to_string()],
            derived_text: "画面中有一辆蓝色车辆停在门口。".to_string(),
            artifacts: Vec::new(),
            ingest_metadata: json!({"frame_path_redacted": true}),
            vlm_metrics: json!({"elapsed_ms": 1880}),
            error: None,
        });
        let stored = sample_stored_event(event);

        let intent = build_local_vision_notification_intent(&stored, "gw_route_harbornavi_dev")
            .expect("notification intent");

        assert!(intent
            .notification_request
            .content
            .body
            .contains("蓝色车辆"));
        assert_eq!(
            intent.notification_request.content.structured_payload["event"]["vlm_status"],
            json!("active")
        );
        assert_eq!(intent.audit_record["vlm_status"], json!("active"));
    }

    #[test]
    fn local_vision_notification_idempotency_is_stable_for_same_event_id() {
        let first = sample_stored_event(sample_event());
        let mut changed_event = sample_event();
        changed_event.summary = "新的本地摘要不应改变同一事件的投递幂等键。".to_string();
        let second = sample_stored_event(changed_event);

        let first = build_local_vision_notification_intent(&first, "gw_route_harbornavi_dev")
            .expect("first intent");
        let second = build_local_vision_notification_intent(&second, "gw_route_harbornavi_dev")
            .expect("second intent");

        assert_eq!(
            first.notification_request.notification_id,
            second.notification_request.notification_id
        );
        assert_eq!(
            first.notification_request.delivery.idempotency_key,
            second.notification_request.delivery.idempotency_key
        );
    }

    #[test]
    fn local_vision_notification_rejects_sensitive_or_local_path_text() {
        let mut event = sample_event();
        event.summary = "调试帧在 /tmp/source.jpg".to_string();
        let stored = sample_stored_event(event);

        let error = build_local_vision_notification_intent(&stored, "gw_route_harbornavi_dev")
            .expect_err("local path must fail");

        assert!(error.contains("local path"));

        let mut request = build_local_vision_notification_intent(
            &sample_stored_event(sample_event()),
            "gw_route_harbornavi_dev",
        )
        .expect("intent")
        .notification_request;
        request.content.body = "camera credential rtsp://user:pass@192.168.3.231/stream2".into();

        let error = validate_local_vision_notification_request(&request)
            .expect_err("sensitive material must fail");
        assert!(error.contains("sensitive") || error.contains("URL credentials"));
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
            vlm: None,
        }
    }

    fn sample_stored_event(event: LocalVisionEvent) -> StoredLocalVisionEvent {
        StoredLocalVisionEvent {
            received_at: "epoch_ms:2".to_string(),
            audit_record: build_audit_record(&event),
            ha_mqtt_payload: build_ha_mqtt_payload(&event),
            event,
        }
    }
}
