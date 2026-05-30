use std::env;
use std::fs;

use harborbeacon_local_agent::runtime::vision_event::{
    build_ha_mqtt_payload, build_local_vision_ha_mqtt_contract, LocalVisionEvent,
    StoredLocalVisionEvent,
};
use serde_json::json;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1).collect())?;
    let stored = read_stored_event(&args.event_json)?;
    let contract = build_local_vision_ha_mqtt_contract(&stored)?;
    let output = json!({
        "ok": true,
        "mode": "offline_contract",
        "classification": "ha-mqtt-contract-ready",
        "mqtt_topic_hint": args.topic_hint.unwrap_or_else(|| "harbornavi/local_vision/events".to_string()),
        "ha_event_type_hint": "harbornavi_local_vision_event",
        "ha_mqtt_payload": contract.payload,
        "audit_record": contract.audit_record,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|error| format!("failed to serialize smoke output: {error}"))?
    );
    Ok(())
}

struct Args {
    event_json: String,
    topic_hint: Option<String>,
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        let mut event_json = None;
        let mut topic_hint = None;
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "-h" | "--help" => return Err(help_text()),
                "--event-json" => {
                    index += 1;
                    event_json = raw.get(index).cloned();
                }
                "--topic-hint" => {
                    index += 1;
                    topic_hint = raw.get(index).cloned();
                }
                other => return Err(format!("unknown argument: {other}\n\n{}", help_text())),
            }
            index += 1;
        }
        Ok(Self {
            event_json: event_json.ok_or_else(help_text)?,
            topic_hint,
        })
    }
}

fn read_stored_event(path: &str) -> Result<StoredLocalVisionEvent, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    serde_json::from_str::<StoredLocalVisionEvent>(&text).or_else(|_| {
        let event = serde_json::from_str::<LocalVisionEvent>(&text)
            .map_err(|error| format!("failed to parse LocalVisionEvent fixture: {error}"))?;
        Ok(StoredLocalVisionEvent {
            received_at: "offline_fixture".to_string(),
            audit_record: json!({
                "audit_kind": "local_vision_event.offline_fixture",
                "event_id": event.event_id,
            }),
            ha_mqtt_payload: build_ha_mqtt_payload(&event),
            event,
        })
    })
}

fn help_text() -> String {
    "usage: harbornavi-ha-mqtt-event-contract-smoke --event-json <stored-event.json> [--topic-hint <topic>]\n\
     Builds the metadata-only HA/MQTT LocalVisionEvent payload without connecting to HA or an MQTT broker."
        .to_string()
}
