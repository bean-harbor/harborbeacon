use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use harborbeacon_local_agent::runtime::vision_event::{
    analyze_snapshot_file, LocalSnapshotAnalysisInput, LocalVisionAnalyzerResult, LocalVisionEvent,
};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
struct Cli {
    camera_id: String,
    rtsp_url: Option<String>,
    snapshot_url: Option<String>,
    beacon_url: String,
    output_dir: PathBuf,
    duration_seconds: u64,
    interval_seconds: u64,
    no_post: bool,
    fixture: bool,
    ffmpeg: String,
    analyzer_command: Option<String>,
    model_path: String,
    label_path: String,
    provider: String,
    redact_paths: bool,
}

#[derive(Debug, Serialize)]
struct SmokeReport {
    ok: bool,
    camera_id: String,
    runtime_probe: Value,
    duration_seconds: u64,
    interval_seconds: u64,
    runs: Vec<SmokeRun>,
    summary: SmokeSummary,
}

#[derive(Debug, Serialize)]
struct SmokeRun {
    iteration: usize,
    ok: bool,
    snapshot_path: Option<String>,
    event: Option<LocalVisionEvent>,
    ingest_http_status: Option<u16>,
    capture_ms: u64,
    detector_ms: Option<u64>,
    analyze_ms: u64,
    event_ingest_ms: Option<u64>,
    total_ms: u64,
    provider: Option<String>,
    detection_count: Option<usize>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Default)]
struct SmokeSummary {
    total: usize,
    passed: usize,
    failed: usize,
    average_total_ms: u64,
    average_detector_ms: u64,
    average_event_ingest_ms: u64,
    max_total_ms: u64,
    under_2s: usize,
    under_5s: usize,
    detection_runs: usize,
    detection_count: usize,
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = fs::create_dir_all(&cli.output_dir) {
        fail(&format!(
            "failed to create output dir {}: {error}",
            cli.output_dir.display()
        ));
    }

    let runtime_probe = probe_official_vision_runtime();
    let start = Instant::now();
    let mut runs = Vec::new();
    let mut iteration = 0usize;
    loop {
        iteration += 1;
        runs.push(run_once(&cli, iteration, &runtime_probe));
        if cli.duration_seconds == 0 || start.elapsed().as_secs() >= cli.duration_seconds {
            break;
        }
        thread::sleep(Duration::from_secs(cli.interval_seconds.max(1)));
    }

    let summary = summarize_runs(&runs);
    let report = SmokeReport {
        ok: summary.failed == 0,
        camera_id: cli.camera_id,
        runtime_probe,
        duration_seconds: cli.duration_seconds,
        interval_seconds: cli.interval_seconds,
        runs,
        summary,
    };
    let report_path = cli.output_dir.join("local-vision-smoke-report.json");
    let text = serde_json::to_string_pretty(&report).unwrap_or_else(|error| {
        fail(&format!("failed to serialize smoke report: {error}"));
    });
    if let Err(error) = fs::write(&report_path, &text) {
        fail(&format!(
            "failed to write smoke report {}: {error}",
            report_path.display()
        ));
    }
    println!("{text}");
    if !report.ok {
        std::process::exit(1);
    }
}

fn run_once(cli: &Cli, iteration: usize, runtime_probe: &Value) -> SmokeRun {
    let total_started = Instant::now();
    let snapshot_path =
        cli.output_dir
            .join(format!("snapshot-{:04}-{}.jpg", iteration, epoch_millis()));
    let capture_started = Instant::now();
    let capture = capture_snapshot(cli, &snapshot_path);
    let capture_ms = capture_started.elapsed().as_millis() as u64;
    if let Err(error) = capture {
        return SmokeRun {
            iteration,
            ok: false,
            snapshot_path: None,
            event: None,
            ingest_http_status: None,
            capture_ms,
            detector_ms: None,
            analyze_ms: 0,
            event_ingest_ms: None,
            total_ms: total_started.elapsed().as_millis() as u64,
            provider: None,
            detection_count: None,
            error: Some(error),
        };
    }

    let analyzer_result = match run_analyzer(cli, &snapshot_path) {
        Ok(result) => result,
        Err(error) => {
            return SmokeRun {
                iteration,
                ok: false,
                snapshot_path: report_snapshot_path(cli, &snapshot_path),
                event: None,
                ingest_http_status: None,
                capture_ms,
                detector_ms: None,
                analyze_ms: 0,
                event_ingest_ms: None,
                total_ms: total_started.elapsed().as_millis() as u64,
                provider: None,
                detection_count: None,
                error: Some(sanitize_sensitive(&error)),
            };
        }
    };
    let detector_ms = analyzer_result
        .as_ref()
        .and_then(|result| result.command_latency_ms.or(result.latency_ms));
    let provider = analyzer_result
        .as_ref()
        .and_then(|result| result.provider.clone());
    let detection_count = analyzer_result
        .as_ref()
        .map(|result| result.detections.len());

    let analyze_started = Instant::now();
    let analyzer = if runtime_probe["official_status"] == json!("missing") {
        "official-vision-runtime-missing+cpu-snapshot-fallback"
    } else {
        "official-vision-runtime-unverified+cpu-snapshot-fallback"
    };
    let event = analyze_snapshot_file(LocalSnapshotAnalysisInput {
        camera_id: cli.camera_id.clone(),
        snapshot_path: snapshot_path.clone(),
        analyzer: Some(analyzer.to_string()),
        latency_ms: None,
        analyzer_result,
        metrics: json!({
            "runtime_probe": runtime_probe,
            "capture_ms": capture_ms,
            "detector_ms": detector_ms,
            "smoke_iteration": iteration,
        }),
    });
    let analyze_ms = analyze_started.elapsed().as_millis() as u64;
    let mut event = match event {
        Ok(mut event) => {
            event.latency_ms = total_started.elapsed().as_millis() as u64;
            event
        }
        Err(error) => {
            return SmokeRun {
                iteration,
                ok: false,
                snapshot_path: report_snapshot_path(cli, &snapshot_path),
                event: None,
                ingest_http_status: None,
                capture_ms,
                detector_ms,
                analyze_ms,
                event_ingest_ms: None,
                total_ms: total_started.elapsed().as_millis() as u64,
                provider,
                detection_count,
                error: Some(sanitize_sensitive(&error)),
            };
        }
    };
    event.latency_ms = total_started.elapsed().as_millis() as u64;

    let ingest_http_status = if cli.no_post {
        None
    } else {
        let ingest_started = Instant::now();
        match post_event(&cli.beacon_url, &event) {
            Ok(status) => {
                let event_ingest_ms = ingest_started.elapsed().as_millis() as u64;
                event.metrics.as_object_mut().map(|map| {
                    map.insert("event_ingest_ms".to_string(), json!(event_ingest_ms));
                });
                Some(status)
            }
            Err(error) => {
                return SmokeRun {
                    iteration,
                    ok: false,
                    snapshot_path: report_snapshot_path(cli, &snapshot_path),
                    event: Some(event),
                    ingest_http_status: None,
                    capture_ms,
                    detector_ms,
                    analyze_ms,
                    event_ingest_ms: Some(ingest_started.elapsed().as_millis() as u64),
                    total_ms: total_started.elapsed().as_millis() as u64,
                    provider,
                    detection_count,
                    error: Some(sanitize_sensitive(&error)),
                };
            }
        }
    };
    let event_ingest_ms = event.metrics.get("event_ingest_ms").and_then(Value::as_u64);
    let ok = ingest_http_status
        .map(|status| status == 200)
        .unwrap_or(true);
    SmokeRun {
        iteration,
        ok,
        snapshot_path: report_snapshot_path(cli, &snapshot_path),
        event: Some(event),
        ingest_http_status,
        capture_ms,
        detector_ms,
        analyze_ms,
        event_ingest_ms,
        total_ms: total_started.elapsed().as_millis() as u64,
        provider,
        detection_count,
        error: if ok {
            None
        } else {
            Some("event ingest returned non-200 status".to_string())
        },
    }
}

fn capture_snapshot(cli: &Cli, snapshot_path: &Path) -> Result<(), String> {
    if cli.fixture {
        return fs::write(
            snapshot_path,
            [0xff, 0xd8, 0xff, 0xdb, 0x00, 0x43, 0xff, 0xd9],
        )
        .map_err(|error| format!("failed to write fixture snapshot: {error}"));
    }
    if let Some(url) = cli.snapshot_url.as_deref() {
        let bytes = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?
            .get(url)
            .send()
            .map_err(|error| {
                format!(
                    "snapshot HTTP request failed: {}",
                    sanitize_sensitive(&error.to_string())
                )
            })?
            .error_for_status()
            .map_err(|error| {
                format!(
                    "snapshot HTTP status failed: {}",
                    sanitize_sensitive(&error.to_string())
                )
            })?
            .bytes()
            .map_err(|error| format!("failed to read snapshot bytes: {error}"))?;
        return fs::write(snapshot_path, bytes)
            .map_err(|error| format!("failed to write snapshot: {error}"));
    }
    let Some(rtsp_url) = cli.rtsp_url.as_deref() else {
        return Err(
            "missing snapshot source; pass --rtsp-url, --snapshot-url, or --fixture".to_string(),
        );
    };
    let output = Command::new(&cli.ffmpeg)
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-rtsp_transport")
        .arg("tcp")
        .arg("-y")
        .arg("-i")
        .arg(rtsp_url)
        .arg("-frames:v")
        .arg("1")
        .arg(snapshot_path)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to launch ffmpeg: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "ffmpeg snapshot failed: {}",
            sanitize_sensitive(stderr.trim())
        ));
    }
    let metadata = fs::metadata(snapshot_path)
        .map_err(|error| format!("ffmpeg did not create snapshot: {error}"))?;
    if metadata.len() == 0 {
        return Err("ffmpeg created an empty snapshot".to_string());
    }
    Ok(())
}

fn run_analyzer(
    cli: &Cli,
    snapshot_path: &Path,
) -> Result<Option<LocalVisionAnalyzerResult>, String> {
    let Some(command) = cli.analyzer_command.as_deref() else {
        return Ok(None);
    };
    if !Path::new(command).exists() {
        return Err(format!("analyzer command not found: {command}"));
    }
    if !Path::new(&cli.model_path).exists() {
        return Err(format!("analyzer model not found: {}", cli.model_path));
    }
    if !Path::new(&cli.label_path).exists() {
        return Err(format!("analyzer label file not found: {}", cli.label_path));
    }
    let started = Instant::now();
    let output = Command::new(command)
        .arg("--image")
        .arg(snapshot_path)
        .arg("--model")
        .arg(&cli.model_path)
        .arg("--label")
        .arg(&cli.label_path)
        .arg("--provider")
        .arg(&cli.provider)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to launch analyzer: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "analyzer failed status={:?}: {} {}",
            output.status.code(),
            sanitize_sensitive(stdout.trim()),
            sanitize_sensitive(stderr.trim())
        ));
    }
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("failed to parse analyzer JSON: {error}; stdout={stdout}"))?;
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "analyzer returned ok=false: {}",
            sanitize_sensitive(&stdout)
        ));
    }
    let mut result: LocalVisionAnalyzerResult = serde_json::from_value(value)
        .map_err(|error| format!("failed to decode analyzer result: {error}"))?;
    result.command_latency_ms = Some(started.elapsed().as_millis() as u64);
    if result.latency_ms.is_none() {
        result.latency_ms = result.command_latency_ms;
    }
    Ok(Some(result))
}

fn post_event(beacon_url: &str, event: &LocalVisionEvent) -> Result<u16, String> {
    let url = format!("{}/api/vision/events", beacon_url.trim_end_matches('/'));
    let response = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?
        .post(url)
        .json(event)
        .send()
        .map_err(|error| format!("failed to POST local vision event: {error}"))?;
    Ok(response.status().as_u16())
}

fn probe_official_vision_runtime() -> Value {
    let package_candidates = [
        "spacemit-onnxruntime",
        "spacemit-ort",
        "onnxruntime-spacemit",
        "libonnxruntime-spacemit",
        "spacemit-ai-runtime",
        "bianbu-ai-runtime",
    ];
    let command_candidates = [
        "onnxruntime_perf_test",
        "spacemit-ort-runner",
        "spacemit-vision-demo",
        "hailo",
    ];
    let mut packages = Vec::new();
    if command_available("dpkg-query") {
        for package in package_candidates {
            let output = Command::new("dpkg-query").arg("-W").arg(package).output();
            if let Ok(output) = output {
                if output.status.success() {
                    packages.push(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }
    }
    let commands = command_candidates
        .into_iter()
        .filter(|command| command_available(command))
        .collect::<Vec<_>>();
    let official_status = if packages.is_empty() && commands.is_empty() {
        "missing"
    } else {
        "present_unverified"
    };
    json!({
        "official_status": official_status,
        "packages": packages,
        "commands": commands,
        "baseline_note": if official_status == "missing" {
            "official SpacemiT/Bianbu vision runtime recipe was not found by package/command probe"
        } else {
            "official runtime candidate exists but this smoke still uses CPU fallback until model recipe is verified"
        },
    })
}

fn command_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success() || status.code().is_some())
        .unwrap_or(false)
}

fn summarize_runs(runs: &[SmokeRun]) -> SmokeSummary {
    let total = runs.len();
    let passed = runs.iter().filter(|run| run.ok).count();
    let failed = total.saturating_sub(passed);
    let sum_total = runs.iter().map(|run| run.total_ms).sum::<u64>();
    let detector_values = runs
        .iter()
        .filter_map(|run| run.detector_ms)
        .collect::<Vec<_>>();
    let event_ingest_values = runs
        .iter()
        .filter_map(|run| run.event_ingest_ms)
        .collect::<Vec<_>>();
    let detection_count = runs
        .iter()
        .filter_map(|run| run.detection_count)
        .sum::<usize>();
    SmokeSummary {
        total,
        passed,
        failed,
        average_total_ms: if total == 0 {
            0
        } else {
            sum_total / total as u64
        },
        average_detector_ms: average_u64(&detector_values),
        average_event_ingest_ms: average_u64(&event_ingest_values),
        max_total_ms: runs.iter().map(|run| run.total_ms).max().unwrap_or(0),
        under_2s: runs.iter().filter(|run| run.total_ms < 2000).count(),
        under_5s: runs.iter().filter(|run| run.total_ms < 5000).count(),
        detection_runs: detector_values.len(),
        detection_count,
    }
}

fn average_u64(values: &[u64]) -> u64 {
    if values.is_empty() {
        0
    } else {
        values.iter().sum::<u64>() / values.len() as u64
    }
}

fn report_snapshot_path(cli: &Cli, snapshot_path: &Path) -> Option<String> {
    if cli.redact_paths {
        Some("[redacted-local-path]".to_string())
    } else {
        Some(snapshot_path.to_string_lossy().to_string())
    }
}

fn sanitize_sensitive(value: &str) -> String {
    let mut output = String::new();
    for token in value.split_whitespace() {
        if token.to_ascii_lowercase().contains("rtsp://") {
            output.push_str("rtsp://redacted ");
        } else if token.to_ascii_lowercase().contains("password=") {
            output.push_str("password=redacted ");
        } else {
            output.push_str(token);
            output.push(' ');
        }
    }
    output.trim().to_string()
}

fn epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn default_analyzer_command() -> Option<String> {
    let installed = "/usr/lib/harboros-beacon/harbornavi_k3_yolov8_analyzer.py";
    if Path::new(installed).exists() {
        Some(installed.to_string())
    } else {
        None
    }
}

impl Cli {
    fn parse() -> Self {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut cli = Self {
            camera_id: "k3-local-camera".to_string(),
            rtsp_url: None,
            snapshot_url: None,
            beacon_url: "http://127.0.0.1:4174".to_string(),
            output_dir: PathBuf::from("/tmp/harbornavi-p0/local-vision-event"),
            duration_seconds: 0,
            interval_seconds: 10,
            no_post: false,
            fixture: false,
            ffmpeg: std::env::var("HARBOR_FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".to_string()),
            analyzer_command: std::env::var("HARBOR_K3_YOLO_ANALYZER")
                .ok()
                .or_else(default_analyzer_command),
            model_path: std::env::var("HARBOR_K3_YOLO_MODEL").unwrap_or_else(|_| {
                "/var/lib/harboros-beacon/models/yolov8n_192x320.q.onnx".to_string()
            }),
            label_path: std::env::var("HARBOR_K3_YOLO_LABELS")
                .unwrap_or_else(|_| "/var/lib/harboros-beacon/models/label.txt".to_string()),
            provider: std::env::var("HARBOR_K3_YOLO_PROVIDER")
                .unwrap_or_else(|_| "cpu".to_string()),
            redact_paths: false,
        };
        let mut index = 0usize;
        while index < args.len() {
            match args[index].as_str() {
                "--camera-id" => cli.camera_id = take_value(&args, &mut index, "--camera-id"),
                "--rtsp-url" => cli.rtsp_url = Some(take_value(&args, &mut index, "--rtsp-url")),
                "--snapshot-url" => {
                    cli.snapshot_url = Some(take_value(&args, &mut index, "--snapshot-url"))
                }
                "--beacon-url" => cli.beacon_url = take_value(&args, &mut index, "--beacon-url"),
                "--output-dir" => {
                    cli.output_dir = PathBuf::from(take_value(&args, &mut index, "--output-dir"))
                }
                "--duration-seconds" => {
                    cli.duration_seconds =
                        parse_u64(&take_value(&args, &mut index, "--duration-seconds"))
                }
                "--interval-seconds" => {
                    cli.interval_seconds =
                        parse_u64(&take_value(&args, &mut index, "--interval-seconds"))
                }
                "--ffmpeg" => cli.ffmpeg = take_value(&args, &mut index, "--ffmpeg"),
                "--analyzer-command" => {
                    cli.analyzer_command = Some(take_value(&args, &mut index, "--analyzer-command"))
                }
                "--model-path" => cli.model_path = take_value(&args, &mut index, "--model-path"),
                "--label-path" => cli.label_path = take_value(&args, &mut index, "--label-path"),
                "--provider" => {
                    cli.provider = take_value(&args, &mut index, "--provider");
                    if cli.provider != "cpu" && cli.provider != "spacemit" {
                        fail("--provider must be cpu or spacemit");
                    }
                }
                "--disable-analyzer" => cli.analyzer_command = None,
                "--no-post" => cli.no_post = true,
                "--fixture" => cli.fixture = true,
                "--redact-paths" => cli.redact_paths = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => fail(&format!("unknown argument {other}")),
            }
            index += 1;
        }
        if cli.provider != "cpu" && cli.provider != "spacemit" {
            fail("--provider must be cpu or spacemit");
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
        "Usage: harbornavi-k3-local-vision-smoke [--camera-id ID] [--rtsp-url URL | --snapshot-url URL | --fixture] [--duration-seconds N] [--interval-seconds N] [--beacon-url URL] [--output-dir PATH] [--analyzer-command PATH] [--model-path PATH] [--label-path PATH] [--provider cpu|spacemit] [--disable-analyzer] [--no-post] [--redact-paths]"
    );
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}
