//! Home Assistant REST connector.

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_TIMEOUT_SECONDS: u64 = 8;
pub const HOME_ASSISTANT_TOKEN_REDACTION: &str = "__harbor_redacted__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeAssistantClientConfig {
    pub base_url: String,
    pub access_token: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct HomeAssistantClient {
    base_url: Url,
    access_token: String,
    http: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HomeAssistantConfigSummary {
    pub base_url: String,
    pub configured: bool,
    pub token_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HomeAssistantCoreConfig {
    #[serde(default)]
    pub location_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub unit_system: Value,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HomeAssistantConnectionTest {
    pub ok: bool,
    pub status: String,
    #[serde(default)]
    pub location_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HomeAssistantEntity {
    pub entity_id: String,
    pub domain: String,
    pub state: String,
    pub display_name: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub device_class: Option<String>,
    #[serde(default)]
    pub last_changed: Option<String>,
    #[serde(default)]
    pub last_updated: Option<String>,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HomeAssistantServiceDomain {
    pub domain: String,
    #[serde(default)]
    pub services: Vec<HomeAssistantService>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HomeAssistantService {
    pub service: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub fields: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RawHomeAssistantEntity {
    entity_id: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    attributes: Value,
    #[serde(default)]
    last_changed: Option<String>,
    #[serde(default)]
    last_updated: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawHomeAssistantServiceDomain {
    domain: String,
    #[serde(default)]
    services: Value,
}

impl HomeAssistantClientConfig {
    pub fn new(base_url: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            access_token: access_token.into(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }

    pub fn configured(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.access_token.trim().is_empty()
    }

    pub fn redacted_summary(&self) -> HomeAssistantConfigSummary {
        HomeAssistantConfigSummary {
            base_url: self.base_url.trim().trim_end_matches('/').to_string(),
            configured: self.configured(),
            token_configured: !self.access_token.trim().is_empty(),
        }
    }
}

impl HomeAssistantClient {
    pub fn new(config: HomeAssistantClientConfig) -> Result<Self, String> {
        let base_url = normalize_base_url(&config.base_url)?;
        let access_token = config.access_token.trim().to_string();
        if access_token.is_empty() {
            return Err("Home Assistant access token is required".to_string());
        }
        let timeout = Duration::from_secs(config.timeout_seconds.max(1));
        let http = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| format!("failed to build Home Assistant client: {error}"))?;

        Ok(Self {
            base_url,
            access_token,
            http,
        })
    }

    pub fn test_connection(&self) -> HomeAssistantConnectionTest {
        match self.fetch_core_config() {
            Ok(config) => HomeAssistantConnectionTest {
                ok: true,
                status: "connected".to_string(),
                location_name: config.location_name,
                version: config.version,
                error: None,
            },
            Err(error) => HomeAssistantConnectionTest {
                ok: false,
                status: "error".to_string(),
                location_name: None,
                version: None,
                error: Some(error),
            },
        }
    }

    pub fn fetch_core_config(&self) -> Result<HomeAssistantCoreConfig, String> {
        self.get_json("/api/config")
    }

    pub fn fetch_entities(&self) -> Result<Vec<HomeAssistantEntity>, String> {
        let raw: Vec<RawHomeAssistantEntity> = self.get_json("/api/states")?;
        Ok(raw.into_iter().map(normalize_entity).collect())
    }

    pub fn fetch_services(&self) -> Result<Vec<HomeAssistantServiceDomain>, String> {
        let raw: Vec<RawHomeAssistantServiceDomain> = self.get_json("/api/services")?;
        Ok(raw.into_iter().map(normalize_service_domain).collect())
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, String> {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| format!("invalid Home Assistant endpoint {path}: {error}"))?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .map_err(|error| format!("Home Assistant request failed: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format_home_assistant_status_error(status));
        }
        response
            .json::<T>()
            .map_err(|error| format!("failed to parse Home Assistant response: {error}"))
    }
}

pub fn token_is_redacted(value: &str) -> bool {
    value.trim().is_empty() || value.trim() == HOME_ASSISTANT_TOKEN_REDACTION
}

pub fn redact_home_assistant_token(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        HOME_ASSISTANT_TOKEN_REDACTION.to_string()
    }
}

pub fn normalize_base_url(value: &str) -> Result<Url, String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Home Assistant base URL is required".to_string());
    }
    let url =
        Url::parse(trimmed).map_err(|error| format!("invalid Home Assistant base URL: {error}"))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        scheme => Err(format!(
            "unsupported Home Assistant URL scheme {scheme}; expected http or https"
        )),
    }
}

fn normalize_entity(raw: RawHomeAssistantEntity) -> HomeAssistantEntity {
    let domain = raw
        .entity_id
        .split_once('.')
        .map(|(domain, _)| domain.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let display_name = raw
        .attributes
        .get("friendly_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| raw.entity_id.clone());
    let area_id = raw
        .attributes
        .get("area_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let device_class = raw
        .attributes
        .get("device_class")
        .and_then(Value::as_str)
        .map(str::to_string);

    HomeAssistantEntity {
        entity_id: raw.entity_id,
        domain,
        state: raw.state,
        display_name,
        area_id,
        device_class,
        last_changed: raw.last_changed,
        last_updated: raw.last_updated,
        attributes: raw.attributes,
    }
}

fn normalize_service_domain(raw: RawHomeAssistantServiceDomain) -> HomeAssistantServiceDomain {
    let mut services = Vec::new();
    if let Some(map) = raw.services.as_object() {
        for (service, value) in map {
            services.push(HomeAssistantService {
                service: service.clone(),
                name: value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                description: value
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                fields: value.get("fields").cloned().unwrap_or_else(|| json!({})),
            });
        }
    }
    services.sort_by(|left, right| left.service.cmp(&right.service));
    HomeAssistantServiceDomain {
        domain: raw.domain,
        services,
    }
}

fn format_home_assistant_status_error(status: StatusCode) -> String {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "Home Assistant token was rejected".to_string()
        }
        StatusCode::NOT_FOUND => "Home Assistant API endpoint was not found".to_string(),
        _ => format!("Home Assistant returned HTTP {status}"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        normalize_base_url, redact_home_assistant_token, token_is_redacted,
        HomeAssistantClientConfig, HOME_ASSISTANT_TOKEN_REDACTION,
    };

    #[test]
    fn config_summary_redacts_token_material() {
        let config = HomeAssistantClientConfig::new("http://ha.local:8123/", "secret-token");

        let summary = config.redacted_summary();

        assert_eq!(summary.base_url, "http://ha.local:8123");
        assert!(summary.configured);
        assert!(summary.token_configured);
        assert_eq!(
            redact_home_assistant_token("secret-token"),
            HOME_ASSISTANT_TOKEN_REDACTION
        );
    }

    #[test]
    fn token_redaction_marker_is_secret_preserving() {
        assert!(token_is_redacted(""));
        assert!(token_is_redacted(HOME_ASSISTANT_TOKEN_REDACTION));
        assert!(!token_is_redacted("new-token"));
    }

    #[test]
    fn base_url_requires_http_scheme() {
        assert!(normalize_base_url("http://127.0.0.1:8123").is_ok());
        assert!(normalize_base_url("https://ha.example.test").is_ok());
        assert!(normalize_base_url("ws://ha.example.test").is_err());
    }

    #[test]
    fn raw_entity_shape_normalizes_friendly_name() {
        let raw = super::RawHomeAssistantEntity {
            entity_id: "light.kitchen".to_string(),
            state: "on".to_string(),
            attributes: json!({"friendly_name": "Kitchen", "device_class": "light"}),
            last_changed: Some("2026-05-09T01:02:03Z".to_string()),
            last_updated: None,
        };

        let entity = super::normalize_entity(raw);

        assert_eq!(entity.domain, "light");
        assert_eq!(entity.display_name, "Kitchen");
        assert_eq!(entity.device_class.as_deref(), Some("light"));
    }
}
