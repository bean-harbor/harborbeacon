use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::scripts::integration::{
    default_midcli_service_query, IntegrationConfig, MidcliClient, MiddlewareClient,
};

const REQUIRED_MIDDLEWARE_METHODS: [&str; 5] = [
    "service.query",
    "service.control",
    "filesystem.listdir",
    "filesystem.copy",
    "filesystem.move",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateReport {
    pub mode: String,
    pub passed: bool,
    pub check_count: usize,
    pub checks: Vec<CheckResult>,
}

pub fn build_checks(_root: &Path) -> Vec<CheckResult> {
    Vec::new()
}

pub fn build_live_checks(config: &IntegrationConfig) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    let middleware = MiddlewareClient::new(config.clone());
    if middleware.is_available() {
        match middleware.get_methods("REST") {
            Ok((methods, _)) => {
                for method_name in REQUIRED_MIDDLEWARE_METHODS {
                    checks.push(CheckResult {
                        name: format!("middleware-method:{method_name}"),
                        passed: methods.contains_key(method_name),
                        skipped: Some(false),
                        details: "Checked with core.get_methods target=REST.".to_string(),
                    });
                }
            }
            Err(err) => checks.push(CheckResult {
                name: "middleware-live-probe".to_string(),
                passed: false,
                skipped: Some(false),
                details: err.to_string(),
            }),
        }
    } else {
        checks.push(CheckResult {
            name: "middleware-live-probe".to_string(),
            passed: false,
            skipped: Some(true),
            details: format!("middleware binary not found: {}", config.middleware_bin),
        });
    }

    let midcli = MidcliClient::new(config.clone());
    if midcli.is_available() {
        match midcli.run_csv_query(&default_midcli_service_query(config)) {
            Ok((rows, result)) => checks.push(CheckResult {
                name: "midcli-service-query".to_string(),
                passed: !rows.is_empty() || result.stdout.to_ascii_lowercase().contains("service"),
                skipped: Some(false),
                details: default_midcli_service_query(config),
            }),
            Err(err) => checks.push(CheckResult {
                name: "midcli-service-query".to_string(),
                passed: false,
                skipped: Some(false),
                details: err.to_string(),
            }),
        }
    } else {
        checks.push(CheckResult {
            name: "midcli-service-query".to_string(),
            passed: false,
            skipped: Some(true),
            details: format!("midcli binary not found: {}", config.midcli_bin),
        });
    }

    checks
}

pub fn run_validate(
    root: &Path,
    config: &IntegrationConfig,
    skip_live: bool,
    require_live: bool,
) -> ValidateReport {
    let mut checks = build_checks(root);
    if !skip_live {
        checks.extend(build_live_checks(config));
    }

    let mut passed = checks
        .iter()
        .all(|check| check.passed || check.skipped == Some(true));
    let live_executed = checks.iter().any(|check| {
        (check.name.starts_with("middleware-") || check.name.starts_with("midcli-"))
            && check.skipped != Some(true)
    });

    if require_live && !live_executed {
        passed = false;
        checks.push(CheckResult {
            name: "live-probe-required".to_string(),
            passed: false,
            skipped: Some(false),
            details: "--require-live was set but no live middleware/midcli probe executed."
                .to_string(),
        });
    }

    ValidateReport {
        mode: if live_executed {
            "live-integration".to_string()
        } else {
            "spec-scaffold".to_string()
        },
        passed,
        check_count: checks.len(),
        checks,
    }
}
