use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct Cli {
    camera_manifest: PathBuf,
    output_dir: Option<PathBuf>,
    duration_seconds: Option<u64>,
    interval_seconds: Option<u64>,
    beacon_url: Option<String>,
    analyzer_command: Option<String>,
    model_path: Option<String>,
    label_path: Option<String>,
    provider: Option<String>,
    local_smoke_bin: String,
    no_post: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraManifest {
    schema: Option<String>,
    duration_seconds: Option<u64>,
    interval_seconds: Option<u64>,
    beacon_url: Option<String>,
    output_dir: Option<PathBuf>,
    analyzer_command: Option<String>,
    model_path: Option<String>,
    label_path: Option<String>,
    provider: Option<String>,
    no_post: Option<bool>,
    cameras: Vec<CameraSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraSource {
    camera_id: String,
    kind: Option<String>,
    rtsp_url: Option<String>,
    snapshot_url: Option<String>,
    fixture: Option<bool>,
    duration_seconds: Option<u64>,
    interval_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct ResolvedConfig {
    schema: String,
    output_dir: PathBuf,
    duration_seconds: u64,
    interval_seconds: u64,
    beacon_url: String,
    analyzer_command: Option<String>,
    model_path: Option<String>,
    label_path: Option<String>,
    provider: String,
    no_post: bool,
    local_smoke_bin: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiVisionReport {
    ok: bool,
    schema: String,
    started_at_ms: u128,
    duration_seconds: u64,
    interval_seconds: u64,
    target_p95_ms: u64,
    output_scope: String,
    source_mix: SourceMix,
    aggregate: AggregateSummary,
    classification: String,
    cameras: Vec<CameraReport>,
    notes: Vec<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SourceMix {
    real: usize,
    replay: usize,
    snapshot: usize,
    fixture: usize,
    other: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CameraReport {
    camera_id: String,
    source_kind: String,
    ok: bool,
    child_exit_code: Option<i32>,
    total: usize,
    passed: usize,
    failed: usize,
    success_rate: f64,
    average_total_ms: u64,
    p95_total_ms: u64,
    max_total_ms: u64,
    average_capture_ms: u64,
    p95_capture_ms: u64,
    average_detector_ms: u64,
    p95_detector_ms: u64,
    average_event_ingest_ms: u64,
    p95_event_ingest_ms: u64,
    detection_runs: usize,
    detection_count: usize,
    error: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AggregateSummary {
    total: usize,
    passed: usize,
    failed: usize,
    success_rate: f64,
    p95_total_ms: u64,
    max_total_ms: u64,
    p95_capture_ms: u64,
    p95_detector_ms: u64,
    p95_event_ingest_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SingleSmokeReport {
    ok: bool,
    camera_id: String,
    runs: Vec<SingleSmokeRun>,
    summary: SingleSmokeSummary,
}

#[derive(Debug, Deserialize)]
struct SingleSmokeRun {
    ok: bool,
    capture_ms: u64,
    detector_ms: Option<u64>,
    event_ingest_ms: Option<u64>,
    total_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SingleSmokeSummary {
    total: usize,
    passed: usize,
    failed: usize,
    average_total_ms: u64,
    max_total_ms: u64,
    average_detector_ms: u64,
    average_event_ingest_ms: u64,
    detection_runs: usize,
    detection_count: usize,
}

fn main() {
    let cli = Cli::parse();
    let manifest = load_manifest(&cli.camera_manifest);
    let config = resolve_config(&cli, &manifest);
    if manifest.cameras.is_empty() {
        fail("camera manifest must contain at least one camera");
    }
    if let Err(error) = fs::create_dir_all(&config.output_dir) {
        fail(&format!(
            "failed to create output dir {}: {error}",
            config.output_dir.display()
        ));
    }

    let handles = manifest
        .cameras
        .iter()
        .cloned()
        .map(|camera| {
            let config = config.clone();
            thread::spawn(move || run_camera(camera, config))
        })
        .collect::<Vec<_>>();

    let mut camera_reports = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(report) => camera_reports.push(report),
            Err(_) => camera_reports.push(CameraReport::system_error(
                "unknown-camera",
                "thread-panic",
                "camera worker panicked",
            )),
        }
    }
    camera_reports.sort_by(|left, right| left.camera_id.cmp(&right.camera_id));

    let source_mix = summarize_sources(&manifest.cameras);
    let aggregate = summarize_aggregate(&camera_reports);
    let classification = classify_result(&aggregate, &camera_reports);
    let ok = classification == "pass";
    let notes = vec![
        "This is a mixed simulation load test: one real camera plus replay cameras does not prove four-vendor camera compatibility.".to_string(),
        "Raw RTSP URLs, credentials, local snapshot paths, and image bytes are intentionally excluded from this aggregate report.".to_string(),
    ];
    let report = MultiVisionReport {
        ok,
        schema: config.schema,
        started_at_ms: epoch_millis(),
        duration_seconds: config.duration_seconds,
        interval_seconds: config.interval_seconds,
        target_p95_ms: 5000,
        output_scope: "[redacted-local-path]".to_string(),
        source_mix,
        aggregate,
        classification,
        cameras: camera_reports,
        notes,
    };
    let text = serde_json::to_string_pretty(&report)
        .unwrap_or_else(|error| fail(&format!("failed to serialize report: {error}")));
    let report_path = config
        .output_dir
        .join("multi-vision-smoke-report.redacted.json");
    if let Err(error) = fs::write(&report_path, &text) {
        fail(&format!(
            "failed to write report {}: {error}",
            report_path.display()
        ));
    }
    println!("{text}");
    if !report.ok {
        std::process::exit(1);
    }
}

fn run_camera(camera: CameraSource, config: ResolvedConfig) -> CameraReport {
    let source_kind = camera.source_kind();
    let camera_output_dir = config.output_dir.join(&camera.camera_id);
    if let Err(error) = fs::create_dir_all(&camera_output_dir) {
        return CameraReport::system_error(
            &camera.camera_id,
            &source_kind,
            &format!("failed to create camera output dir: {error}"),
        );
    }
    let mut args = vec![
        "--camera-id".to_string(),
        camera.camera_id.clone(),
        "--duration-seconds".to_string(),
        camera
            .duration_seconds
            .unwrap_or(config.duration_seconds)
            .to_string(),
        "--interval-seconds".to_string(),
        camera
            .interval_seconds
            .unwrap_or(config.interval_seconds)
            .to_string(),
        "--beacon-url".to_string(),
        config.beacon_url.clone(),
        "--output-dir".to_string(),
        camera_output_dir.to_string_lossy().to_string(),
        "--provider".to_string(),
        config.provider.clone(),
        "--redact-paths".to_string(),
    ];
    if config.no_post {
        args.push("--no-post".to_string());
    }
    if let Some(command) = config.analyzer_command.as_deref() {
        args.push("--analyzer-command".to_string());
        args.push(command.to_string());
    }
    if let Some(path) = config.model_path.as_deref() {
        args.push("--model-path".to_string());
        args.push(path.to_string());
    }
    if let Some(path) = config.label_path.as_deref() {
        args.push("--label-path".to_string());
        args.push(path.to_string());
    }
    if camera.fixture.unwrap_or(false) {
        args.push("--fixture".to_string());
    } else if let Some(rtsp_url) = camera.rtsp_url.as_deref() {
        args.push("--rtsp-url".to_string());
        args.push(rtsp_url.to_string());
    } else if let Some(snapshot_url) = camera.snapshot_url.as_deref() {
        args.push("--snapshot-url".to_string());
        args.push(snapshot_url.to_string());
    } else {
        return CameraReport::system_error(
            &camera.camera_id,
            &source_kind,
            "camera source must define rtspUrl, snapshotUrl, or fixture=true",
        );
    }

    let output = Command::new(&config.local_smoke_bin)
        .args(&args)
        .stdin(Stdio::null())
        .output();
    let report_path = camera_output_dir.join("local-vision-smoke-report.json");
    match output {
        Ok(output) => {
            let exit_code = output.status.code();
            let stderr = String::from_utf8_lossy(&output.stderr);
            match read_single_report(&report_path) {
                Ok(report) => summarize_camera_report(
                    &camera.camera_id,
                    &source_kind,
                    exit_code,
                    &report,
                    if output.status.success() {
                        None
                    } else {
                        Some(sanitize_sensitive(&truncate(&stderr, 600)))
                    },
                ),
                Err(error) => CameraReport::system_error(
                    &camera.camera_id,
                    &source_kind,
                    &format!("child report missing or invalid: {error}"),
                )
                .with_exit_code(exit_code),
            }
        }
        Err(error) => CameraReport::system_error(
            &camera.camera_id,
            &source_kind,
            &format!("failed to launch local vision smoke binary: {error}"),
        ),
    }
}

fn read_single_report(path: &Path) -> Result<SingleSmokeReport, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read child report: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("failed to parse child report: {error}"))
}

fn summarize_camera_report(
    camera_id: &str,
    source_kind: &str,
    exit_code: Option<i32>,
    report: &SingleSmokeReport,
    error: Option<String>,
) -> CameraReport {
    let total_values = report
        .runs
        .iter()
        .map(|run| run.total_ms)
        .collect::<Vec<_>>();
    let capture_values = report
        .runs
        .iter()
        .map(|run| run.capture_ms)
        .collect::<Vec<_>>();
    let detector_values = report
        .runs
        .iter()
        .filter_map(|run| run.detector_ms)
        .collect::<Vec<_>>();
    let ingest_values = report
        .runs
        .iter()
        .filter_map(|run| run.event_ingest_ms)
        .collect::<Vec<_>>();
    let failed_runs = report.runs.iter().filter(|run| !run.ok).count();
    let total = report.summary.total.max(report.runs.len());
    let passed = report
        .summary
        .passed
        .min(total)
        .max(total.saturating_sub(failed_runs));
    let failed = report.summary.failed.max(failed_runs);
    CameraReport {
        camera_id: if report.camera_id.is_empty() {
            camera_id.to_string()
        } else {
            report.camera_id.clone()
        },
        source_kind: source_kind.to_string(),
        ok: report.ok && failed == 0 && error.is_none(),
        child_exit_code: exit_code,
        total,
        passed,
        failed,
        success_rate: rate(passed, total),
        average_total_ms: report.summary.average_total_ms,
        p95_total_ms: p95(&total_values),
        max_total_ms: report.summary.max_total_ms,
        average_capture_ms: average_u64(&capture_values),
        p95_capture_ms: p95(&capture_values),
        average_detector_ms: report.summary.average_detector_ms,
        p95_detector_ms: p95(&detector_values),
        average_event_ingest_ms: report.summary.average_event_ingest_ms,
        p95_event_ingest_ms: p95(&ingest_values),
        detection_runs: report.summary.detection_runs,
        detection_count: report.summary.detection_count,
        error,
    }
}

fn summarize_aggregate(cameras: &[CameraReport]) -> AggregateSummary {
    let total = cameras.iter().map(|camera| camera.total).sum::<usize>();
    let passed = cameras.iter().map(|camera| camera.passed).sum::<usize>();
    let failed = cameras.iter().map(|camera| camera.failed).sum::<usize>();
    AggregateSummary {
        total,
        passed,
        failed,
        success_rate: rate(passed, total),
        p95_total_ms: cameras
            .iter()
            .map(|camera| camera.p95_total_ms)
            .max()
            .unwrap_or(0),
        max_total_ms: cameras
            .iter()
            .map(|camera| camera.max_total_ms)
            .max()
            .unwrap_or(0),
        p95_capture_ms: cameras
            .iter()
            .map(|camera| camera.p95_capture_ms)
            .max()
            .unwrap_or(0),
        p95_detector_ms: cameras
            .iter()
            .map(|camera| camera.p95_detector_ms)
            .max()
            .unwrap_or(0),
        p95_event_ingest_ms: cameras
            .iter()
            .map(|camera| camera.p95_event_ingest_ms)
            .max()
            .unwrap_or(0),
    }
}

fn summarize_sources(cameras: &[CameraSource]) -> SourceMix {
    let mut mix = SourceMix::default();
    for camera in cameras {
        match camera.source_kind().as_str() {
            "real" => mix.real += 1,
            "replay" => mix.replay += 1,
            "snapshot" => mix.snapshot += 1,
            "fixture" => mix.fixture += 1,
            _ => mix.other += 1,
        }
    }
    mix
}

fn classify_result(aggregate: &AggregateSummary, cameras: &[CameraReport]) -> String {
    if cameras.iter().any(|camera| camera.error.is_some()) {
        return "system-risk".to_string();
    }
    if aggregate.success_rate < 0.99 || aggregate.p95_total_ms >= 5000 {
        if aggregate.p95_capture_ms >= 3000 && aggregate.p95_detector_ms < 1500 {
            return "capture-bottleneck".to_string();
        }
        if aggregate.p95_detector_ms >= 3000 && aggregate.p95_capture_ms < 1500 {
            return "analyzer-bottleneck".to_string();
        }
        return "capacity-risk".to_string();
    }
    "pass".to_string()
}

fn resolve_config(cli: &Cli, manifest: &CameraManifest) -> ResolvedConfig {
    let provider = cli
        .provider
        .clone()
        .or_else(|| manifest.provider.clone())
        .unwrap_or_else(|| "cpu".to_string());
    if provider != "cpu" && provider != "spacemit" {
        fail("--provider or manifest provider must be cpu or spacemit");
    }
    ResolvedConfig {
        schema: manifest
            .schema
            .clone()
            .unwrap_or_else(|| "harbornavi.k3.multiVisionSmoke.v1".to_string()),
        output_dir: cli
            .output_dir
            .clone()
            .or_else(|| manifest.output_dir.clone())
            .unwrap_or_else(|| PathBuf::from("/tmp/harbornavi-p1/4ch-mixed-simulation")),
        duration_seconds: cli
            .duration_seconds
            .or(manifest.duration_seconds)
            .unwrap_or(1800),
        interval_seconds: cli
            .interval_seconds
            .or(manifest.interval_seconds)
            .unwrap_or(10),
        beacon_url: cli
            .beacon_url
            .clone()
            .or_else(|| manifest.beacon_url.clone())
            .unwrap_or_else(|| "http://127.0.0.1:4174".to_string()),
        analyzer_command: cli
            .analyzer_command
            .clone()
            .or_else(|| manifest.analyzer_command.clone()),
        model_path: cli
            .model_path
            .clone()
            .or_else(|| manifest.model_path.clone()),
        label_path: cli
            .label_path
            .clone()
            .or_else(|| manifest.label_path.clone()),
        provider,
        no_post: cli.no_post || manifest.no_post.unwrap_or(false),
        local_smoke_bin: cli.local_smoke_bin.clone(),
    }
}

fn load_manifest(path: &Path) -> CameraManifest {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        fail(&format!(
            "failed to read camera manifest {}: {error}",
            path.display()
        ))
    });
    serde_json::from_str(&text).unwrap_or_else(|error| {
        fail(&format!(
            "failed to parse camera manifest {}: {error}",
            path.display()
        ))
    })
}

fn average_u64(values: &[u64]) -> u64 {
    if values.is_empty() {
        0
    } else {
        values.iter().sum::<u64>() / values.len() as u64
    }
}

fn p95(values: &[u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}

fn rate(passed: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        passed as f64 / total as f64
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn sanitize_sensitive(value: &str) -> String {
    let mut output = String::new();
    for token in value.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        if lower.contains("rtsp://") {
            output.push_str("rtsp://redacted ");
        } else if lower.contains("api_key")
            || lower.contains("apikey")
            || lower.contains("token")
            || lower.contains("private-key")
            || lower.contains("password")
            || lower.contains("credential")
        {
            output.push_str("[redacted-secret] ");
        } else if lower.starts_with("/tmp/")
            || lower.starts_with("/var/")
            || lower.starts_with("/home/")
            || lower.contains(":\\")
        {
            output.push_str("[redacted-local-path] ");
        } else {
            output.push_str(token);
            output.push(' ');
        }
    }
    output.trim().to_string()
}

fn default_local_smoke_bin() -> String {
    if let Ok(value) = std::env::var("HARBORNAVI_LOCAL_VISION_SMOKE_BIN") {
        return value;
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(parent) = current.parent() {
            let sibling = parent.join(format!(
                "harbornavi-k3-local-vision-smoke{}",
                std::env::consts::EXE_SUFFIX
            ));
            if sibling.exists() {
                return sibling.to_string_lossy().to_string();
            }
        }
    }
    "harbornavi-k3-local-vision-smoke".to_string()
}

impl CameraSource {
    fn source_kind(&self) -> String {
        if let Some(kind) = self.kind.as_deref() {
            return kind.to_string();
        }
        if self.fixture.unwrap_or(false) {
            "fixture".to_string()
        } else if self.rtsp_url.is_some() {
            "rtsp".to_string()
        } else if self.snapshot_url.is_some() {
            "snapshot".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

impl CameraReport {
    fn system_error(camera_id: &str, source_kind: &str, error: &str) -> Self {
        Self {
            camera_id: camera_id.to_string(),
            source_kind: source_kind.to_string(),
            ok: false,
            child_exit_code: None,
            total: 0,
            passed: 0,
            failed: 0,
            success_rate: 0.0,
            average_total_ms: 0,
            p95_total_ms: 0,
            max_total_ms: 0,
            average_capture_ms: 0,
            p95_capture_ms: 0,
            average_detector_ms: 0,
            p95_detector_ms: 0,
            average_event_ingest_ms: 0,
            p95_event_ingest_ms: 0,
            detection_runs: 0,
            detection_count: 0,
            error: Some(sanitize_sensitive(error)),
        }
    }

    fn with_exit_code(mut self, exit_code: Option<i32>) -> Self {
        self.child_exit_code = exit_code;
        self
    }
}

impl Cli {
    fn parse() -> Self {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut cli = Self {
            camera_manifest: PathBuf::new(),
            output_dir: None,
            duration_seconds: None,
            interval_seconds: None,
            beacon_url: None,
            analyzer_command: None,
            model_path: None,
            label_path: None,
            provider: None,
            local_smoke_bin: default_local_smoke_bin(),
            no_post: false,
        };
        let mut index = 0usize;
        while index < args.len() {
            match args[index].as_str() {
                "--camera-manifest" => {
                    cli.camera_manifest =
                        PathBuf::from(take_value(&args, &mut index, "--camera-manifest"))
                }
                "--output-dir" => {
                    cli.output_dir =
                        Some(PathBuf::from(take_value(&args, &mut index, "--output-dir")))
                }
                "--duration-seconds" => {
                    cli.duration_seconds = Some(parse_u64(&take_value(
                        &args,
                        &mut index,
                        "--duration-seconds",
                    )))
                }
                "--interval-seconds" => {
                    cli.interval_seconds = Some(parse_u64(&take_value(
                        &args,
                        &mut index,
                        "--interval-seconds",
                    )))
                }
                "--beacon-url" => {
                    cli.beacon_url = Some(take_value(&args, &mut index, "--beacon-url"))
                }
                "--analyzer-command" => {
                    cli.analyzer_command = Some(take_value(&args, &mut index, "--analyzer-command"))
                }
                "--model-path" => {
                    cli.model_path = Some(take_value(&args, &mut index, "--model-path"))
                }
                "--label-path" => {
                    cli.label_path = Some(take_value(&args, &mut index, "--label-path"))
                }
                "--provider" => cli.provider = Some(take_value(&args, &mut index, "--provider")),
                "--local-smoke-bin" => {
                    cli.local_smoke_bin = take_value(&args, &mut index, "--local-smoke-bin")
                }
                "--no-post" => cli.no_post = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => fail(&format!("unknown argument {other}")),
            }
            index += 1;
        }
        if cli.camera_manifest.as_os_str().is_empty() {
            fail("missing required --camera-manifest");
        }
        cli
    }
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> String {
    *index += 1;
    if *index >= args.len() {
        fail(&format!("missing value for {flag}"));
    }
    args[*index].clone()
}

fn parse_u64(value: &str) -> u64 {
    value
        .parse::<u64>()
        .unwrap_or_else(|_| fail(&format!("invalid integer value: {value}")))
}

fn print_usage() {
    println!(
        "Usage: harbornavi-k3-multi-vision-smoke --camera-manifest cameras.json [--output-dir PATH] [--duration-seconds N] [--interval-seconds N] [--beacon-url URL] [--analyzer-command PATH] [--model-path PATH] [--label-path PATH] [--provider cpu|spacemit] [--local-smoke-bin PATH] [--no-post]"
    );
}

fn epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_camel_case_manifest() {
        let manifest: CameraManifest = serde_json::from_str(
            r#"{
                "schema": "harbornavi.k3.multiVisionSmoke.v1",
                "durationSeconds": 30,
                "intervalSeconds": 10,
                "provider": "cpu",
                "cameras": [
                    {"cameraId": "cam-real", "kind": "real", "rtspUrl": "rtsp://example/redacted"},
                    {"cameraId": "cam-sim", "kind": "replay", "fixture": true}
                ]
            }"#,
        )
        .expect("manifest parses");
        assert_eq!(manifest.cameras.len(), 2);
        assert_eq!(manifest.cameras[0].camera_id, "cam-real");
        assert_eq!(manifest.cameras[1].source_kind(), "replay");
    }

    #[test]
    fn p95_uses_nearest_upper_rank() {
        assert_eq!(p95(&[1, 2, 3, 4, 5]), 5);
        assert_eq!(p95(&[50, 10, 30, 20]), 50);
        assert_eq!(p95(&[]), 0);
    }

    #[test]
    fn parses_snake_case_child_report() {
        let report: SingleSmokeReport = serde_json::from_str(
            r#"{
                "ok": true,
                "camera_id": "cam-a",
                "runtime_probe": {},
                "duration_seconds": 10,
                "interval_seconds": 10,
                "runs": [
                    {
                        "iteration": 1,
                        "ok": true,
                        "snapshot_path": "[redacted-local-path]",
                        "event": null,
                        "ingest_http_status": 200,
                        "capture_ms": 100,
                        "detector_ms": 200,
                        "analyze_ms": 10,
                        "event_ingest_ms": 30,
                        "total_ms": 350,
                        "provider": "cpu",
                        "detection_count": 1,
                        "error": null
                    }
                ],
                "summary": {
                    "total": 1,
                    "passed": 1,
                    "failed": 0,
                    "average_total_ms": 350,
                    "average_detector_ms": 200,
                    "average_event_ingest_ms": 30,
                    "max_total_ms": 350,
                    "under_2s": 1,
                    "under_5s": 1,
                    "detection_runs": 1,
                    "detection_count": 1
                }
            }"#,
        )
        .expect("child report parses");
        assert_eq!(report.camera_id, "cam-a");
        assert_eq!(report.runs[0].capture_ms, 100);
    }

    #[test]
    fn classifies_pass_and_bottlenecks() {
        let pass = AggregateSummary {
            total: 100,
            passed: 100,
            failed: 0,
            success_rate: 1.0,
            p95_total_ms: 1200,
            max_total_ms: 2000,
            p95_capture_ms: 700,
            p95_detector_ms: 300,
            p95_event_ingest_ms: 50,
        };
        assert_eq!(classify_result(&pass, &[]), "pass");
        let capture = AggregateSummary {
            success_rate: 1.0,
            p95_total_ms: 6000,
            p95_capture_ms: 3500,
            p95_detector_ms: 700,
            ..AggregateSummary::default()
        };
        assert_eq!(classify_result(&capture, &[]), "capture-bottleneck");
        let analyzer = AggregateSummary {
            success_rate: 1.0,
            p95_total_ms: 6000,
            p95_capture_ms: 700,
            p95_detector_ms: 3500,
            ..AggregateSummary::default()
        };
        assert_eq!(classify_result(&analyzer, &[]), "analyzer-bottleneck");
    }
}
