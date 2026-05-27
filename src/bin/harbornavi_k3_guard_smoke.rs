use std::fs;
use std::path::Path;
use std::process;

use harborbeacon_local_agent::control_plane::models::{
    ModelEndpoint, ModelEndpointKind, ModelEndpointStatus, ModelKind, ModelRoutePolicy,
    PrivacyLevel,
};
use harborbeacon_local_agent::runtime::admin_console::AdminModelCenterState;
use harborbeacon_local_agent::runtime::model_center::{
    run_embedding_with_state, run_llm_text_with_state, run_llm_text_with_state_and_options,
    run_vlm_summary_with_state, LlmTextOptions,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct CheckResult {
    name: &'static str,
    ok: bool,
    status: String,
    detail: Value,
}

#[derive(Debug, Serialize)]
struct SmokeReport {
    ok: bool,
    checks: Vec<CheckResult>,
}

fn main() {
    let mut output = None;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--output" => {
                index += 1;
                output = args.get(index).cloned();
            }
            value if value.starts_with("--output=") => {
                output = Some(value["--output=".len()..].to_string());
            }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            value => fail(&format!("unknown argument {value}")),
        }
        index += 1;
    }

    let checks = vec![
        check_vlm_cloud_requires_vpf_manifest(),
        check_embedding_route_does_not_select_cloud(),
        check_semantic_router_does_not_select_cloud(),
        check_llm_cloud_fallback_redacted_audit(),
    ];
    let report = SmokeReport {
        ok: checks.iter().all(|check| check.ok),
        checks,
    };
    let body = serde_json::to_string_pretty(&report).unwrap_or_else(|error| {
        fail(&format!("failed to serialize smoke report: {error}"));
    });
    if let Some(path) = output {
        fs::write(&path, &body).unwrap_or_else(|error| {
            fail(&format!("failed to write smoke report {path}: {error}"));
        });
    } else {
        println!("{body}");
    }
    if !report.ok {
        process::exit(1);
    }
}

fn check_vlm_cloud_requires_vpf_manifest() -> CheckResult {
    let state = AdminModelCenterState {
        endpoints: vec![ModelEndpoint {
            model_endpoint_id: "vlm-cloud-smoke".to_string(),
            workspace_id: Some("home-1".to_string()),
            provider_account_id: None,
            model_kind: ModelKind::Vlm,
            endpoint_kind: ModelEndpointKind::Cloud,
            provider_key: "openai_compatible".to_string(),
            model_name: "vision-cloud-smoke".to_string(),
            capability_tags: vec!["vlm".to_string(), "cloud_fallback".to_string()],
            cost_policy: json!({}),
            status: ModelEndpointStatus::Active,
            metadata: json!({
                "mock_text": "should not be selected without VPF manifest",
            }),
        }],
        route_policies: vec![ModelRoutePolicy {
            route_policy_id: "retrieval.vision_summary".to_string(),
            workspace_id: "home-1".to_string(),
            domain_scope: "retrieval".to_string(),
            modality: "multimodal".to_string(),
            privacy_level: PrivacyLevel::AllowRedactedCloud,
            local_preferred: true,
            max_cost_per_run: None,
            fallback_order: vec!["cloud".to_string()],
            status: "active".to_string(),
            metadata: json!({}),
        }],
        ..AdminModelCenterState::default()
    };
    let result = run_vlm_summary_with_state(Path::new("source.jpg"), &state);
    let code = result.details["privacy_guard"]["code"]
        .as_str()
        .unwrap_or("");
    check(
        "vlm_cloud_requires_vpf_manifest",
        !result.available && result.status == "blocked" && code == "vpf_manifest_required",
        result.status,
        json!({
            "privacy_guard": result.details["privacy_guard"],
            "model_endpoint_id": result.model_endpoint_id,
        }),
    )
}

fn check_embedding_route_does_not_select_cloud() -> CheckResult {
    let state = AdminModelCenterState {
        endpoints: vec![ModelEndpoint {
            model_endpoint_id: "embed-cloud-smoke".to_string(),
            workspace_id: Some("home-1".to_string()),
            provider_account_id: None,
            model_kind: ModelKind::Embedder,
            endpoint_kind: ModelEndpointKind::Cloud,
            provider_key: "openai_compatible".to_string(),
            model_name: "cloud-embed-smoke".to_string(),
            capability_tags: vec!["embeddings".to_string(), "cloud_fallback".to_string()],
            cost_policy: json!({}),
            status: ModelEndpointStatus::Active,
            metadata: json!({
                "mock_embedding_dimensions": 4,
            }),
        }],
        route_policies: vec![ModelRoutePolicy {
            route_policy_id: "retrieval.embed".to_string(),
            workspace_id: "home-1".to_string(),
            domain_scope: "retrieval".to_string(),
            modality: "text".to_string(),
            privacy_level: PrivacyLevel::AllowRedactedCloud,
            local_preferred: true,
            max_cost_per_run: None,
            fallback_order: vec!["cloud".to_string()],
            status: "active".to_string(),
            metadata: json!({}),
        }],
        ..AdminModelCenterState::default()
    };
    let result = run_embedding_with_state("增量入库", &state);
    check(
        "embedding_route_does_not_select_cloud",
        !result.available && result.status == "disabled" && result.model_endpoint_id.is_none(),
        result.status,
        json!({
            "model_endpoint_id": result.model_endpoint_id,
            "vector_len": result.vector.len(),
        }),
    )
}

fn check_semantic_router_does_not_select_cloud() -> CheckResult {
    let state = AdminModelCenterState {
        endpoints: vec![ModelEndpoint {
            model_endpoint_id: "router-cloud-smoke".to_string(),
            workspace_id: Some("home-1".to_string()),
            provider_account_id: None,
            model_kind: ModelKind::Llm,
            endpoint_kind: ModelEndpointKind::Cloud,
            provider_key: "openai_compatible".to_string(),
            model_name: "router-cloud-smoke".to_string(),
            capability_tags: vec!["chat".to_string(), "cloud_fallback".to_string()],
            cost_policy: json!({}),
            status: ModelEndpointStatus::Active,
            metadata: json!({
                "mock_text": "cloud router should not be selected",
            }),
        }],
        route_policies: vec![ModelRoutePolicy {
            route_policy_id: "semantic.router".to_string(),
            workspace_id: "home-1".to_string(),
            domain_scope: "semantic".to_string(),
            modality: "text".to_string(),
            privacy_level: PrivacyLevel::AllowRedactedCloud,
            local_preferred: true,
            max_cost_per_run: None,
            fallback_order: vec!["cloud".to_string()],
            status: "active".to_string(),
            metadata: json!({}),
        }],
        ..AdminModelCenterState::default()
    };
    let result = run_llm_text_with_state_and_options(
        "帮我判断这个家庭指令的意图",
        &state,
        &LlmTextOptions {
            purpose: Some("semantic.router".to_string()),
            max_tokens: Some(16),
            ..Default::default()
        },
    );
    check(
        "semantic_router_does_not_select_cloud",
        !result.available && result.status == "disabled" && result.model_endpoint_id.is_none(),
        result.status,
        json!({
            "model_endpoint_id": result.model_endpoint_id,
            "details": result.details,
        }),
    )
}

fn check_llm_cloud_fallback_redacted_audit() -> CheckResult {
    let state = AdminModelCenterState {
        endpoints: vec![ModelEndpoint {
            model_endpoint_id: "llm-cloud-smoke".to_string(),
            workspace_id: Some("home-1".to_string()),
            provider_account_id: None,
            model_kind: ModelKind::Llm,
            endpoint_kind: ModelEndpointKind::Cloud,
            provider_key: "openai_compatible".to_string(),
            model_name: "cloud-answer-smoke".to_string(),
            capability_tags: vec!["chat".to_string(), "cloud_fallback".to_string()],
            cost_policy: json!({}),
            status: ModelEndpointStatus::Active,
            metadata: json!({
                "mock_text": "cloud answer",
                "api_key": "configured",
            }),
        }],
        route_policies: vec![ModelRoutePolicy {
            route_policy_id: "retrieval.answer".to_string(),
            workspace_id: "home-1".to_string(),
            domain_scope: "retrieval".to_string(),
            modality: "text".to_string(),
            privacy_level: PrivacyLevel::AllowRedactedCloud,
            local_preferred: true,
            max_cost_per_run: None,
            fallback_order: vec!["cloud".to_string()],
            status: "active".to_string(),
            metadata: json!({}),
        }],
        ..AdminModelCenterState::default()
    };
    let result = run_llm_text_with_state("summarize safely", &state);
    let redacted = result.details["privacy_transform"] == json!("redacted_text")
        && result.details["audit_prompt_storage"] == json!("redacted")
        && result.details.get("api_key").is_none();
    check(
        "llm_cloud_fallback_redacted_audit",
        result.available && result.text == "cloud answer" && redacted,
        result.status,
        json!({
            "selected_endpoint_kind": result.details["selected_endpoint_kind"],
            "privacy_transform": result.details["privacy_transform"],
            "audit_prompt_storage": result.details["audit_prompt_storage"],
            "has_api_key_field": result.details.get("api_key").is_some(),
        }),
    )
}

fn check(name: &'static str, ok: bool, status: String, detail: Value) -> CheckResult {
    CheckResult {
        name,
        ok,
        status,
        detail,
    }
}

fn print_usage() {
    println!("Usage: harbornavi-k3-guard-smoke [--output PATH]");
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(2);
}
