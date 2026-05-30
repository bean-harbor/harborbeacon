use std::env;
use std::fs;

use harborbeacon_local_agent::runtime::vision_event::{
    build_ha_mqtt_payload, build_local_vision_family_summary, LocalVisionEvent,
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
    let family = build_local_vision_family_summary(&stored)?;
    let output = json!({
        "ok": true,
        "mode": "offline_contract",
        "classification": "family-summary-audit-ready",
        "family_summary": family.summary,
        "audit_record": family.audit_record,
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
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        let mut event_json = None;
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "-h" | "--help" => return Err(help_text()),
                "--event-json" => {
                    index += 1;
                    event_json = raw.get(index).cloned();
                }
                other => return Err(format!("unknown argument: {other}\n\n{}", help_text())),
            }
            index += 1;
        }
        Ok(Self {
            event_json: event_json.ok_or_else(help_text)?,
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
    "usage: harbornavi-family-summary-audit-smoke --event-json <stored-event.json>\n\
     Builds a metadata-only HarborNavi family summary and audit record from a LocalVisionEvent fixture."
        .to_string()
}
