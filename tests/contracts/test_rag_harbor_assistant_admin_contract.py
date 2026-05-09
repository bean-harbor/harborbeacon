import re

from conftest import read_doc


def test_rag_admin_endpoints_are_exposed_by_backend_and_harbor_assistant() -> None:
    backend = read_doc("src/bin/agent_hub_admin_api.rs")
    service = read_doc("frontend/harbor-assistant/src/app/core/admin-api.service.ts")

    required_routes = [
        ("GET", "/api/rag/readiness"),
        ("GET", "/api/knowledge/settings"),
        ("PUT", "/api/knowledge/settings"),
        ("POST", "/api/knowledge/index/run"),
        ("GET", "/api/knowledge/index/status"),
        ("GET", "/api/knowledge/index/jobs"),
        ("POST", "/api/knowledge/index/jobs/"),
    ]

    for method, route in required_routes:
        assert method in backend
        assert route in backend
        assert route.removeprefix("/api") in service

    assert "path.ends_with(\"/cancel\")" in backend
    assert "/cancel`" in service


def test_camera_dvr_admin_endpoints_are_exposed_by_backend_and_harbor_assistant() -> None:
    backend = read_doc("src/bin/agent_hub_admin_api.rs")
    service = read_doc("frontend/harbor-assistant/src/app/core/admin-api.service.ts")
    types = read_doc("frontend/harbor-assistant/src/app/core/admin-api.types.ts")
    panel = read_doc("frontend/harbor-assistant/src/app/shared/page-state-panel.component.html")

    required_backend_routes = [
        "/api/cameras/recording-settings",
        "/api/cameras/recordings/status",
        "/api/cameras/recordings/timeline",
        "recordings/start",
        "recordings/stop",
    ]
    for route in required_backend_routes:
        assert route in backend

    required_service_methods = [
        "getDvrRecordingSettings",
        "saveDvrRecordingSettings",
        "getDvrRecordingStatus",
        "getDvrTimeline",
        "startDvrRecording",
        "stopDvrRecording",
    ]
    for method in required_service_methods:
        assert method in service

    for field in [
        "recording_root",
        "retention_days",
        "segment_seconds",
        "enabled_device_ids",
        "low_bitrate_stream_preferred",
        "high_res_event_clips_enabled",
        "continuous_bitrate_mbps",
    ]:
        assert field in types

    assert "Local camera DVR" in panel
    assert "low_bitrate_stream_preferred" in panel
    assert "high_res_event_clips_enabled" in panel


def test_camera_dvr_sidecars_reuse_multimodal_rag_boundary() -> None:
    dvr = read_doc("src/runtime/dvr.rs")
    knowledge_index = read_doc("src/runtime/knowledge_index.rs")
    camera_skill = read_doc("skills/builtins/home.camera_hub/skill.yaml")

    assert "multimodal_rag_vlm" in dvr
    assert "reuse_model_center_vlm_and_existing_knowledge_index" in dvr
    assert "analysis_pending" in dvr
    assert "run_vlm_summary" in knowledge_index
    assert "DvrVlm" not in dvr
    assert "video_specific_embedding" not in dvr
    for action in [
        "camera.recording_start",
        "camera.recording_stop",
        "camera.recording_status",
        "camera.video_search",
        "camera.daily_report",
    ]:
        assert action in camera_skill


def test_harbor_assistant_admin_service_uses_same_origin_beacon_api_only() -> None:
    service = read_doc("frontend/harbor-assistant/src/app/core/admin-api.service.ts")

    direct_calls = re.findall(r"this\.http\.(?:get|post|put|delete)<", service)
    literal_calls = re.findall(
        r"this\.http\.(?:get|post|put|delete)<[^>]+>\(\s*([`'])([^`']+)",
        service,
    )
    api_url_calls = re.findall(r"this\.apiUrl\(\s*([`'])([^`']+)", service)

    assert direct_calls
    assert api_url_calls
    for _quote, url in literal_calls:
        assert url.startswith("/api/") or url.startswith("${this.apiUrl("), url
    for _quote, path in api_url_calls:
        assert path.startswith("/"), path
        assert not path.startswith("/api/"), path
        assert not path.startswith("http://"), path
        assert not path.startswith("https://"), path

    assert "this.http.get<" in service
    assert "private apiUrl(path: string): string" in service
    assert "private resolveApiBase(): string" in service
    assert "'/api/beacon' : '/api'" in service
    assert "'/api/harbor-assistant' : '/api'" not in service
    assert "http://" not in service
    assert "https://" not in service


def test_home_assistant_admin_page_is_beacon_owned_and_separate_from_devices_aiot() -> None:
    backend = read_doc("src/bin/agent_hub_admin_api.rs")
    service = read_doc("frontend/harbor-assistant/src/app/core/admin-api.service.ts")
    registry = read_doc("frontend/harbor-assistant/src/app/core/page-registry.ts")
    panel = read_doc("frontend/harbor-assistant/src/app/shared/page-state-panel.component.html")
    gate = read_doc("../HarborGate/rust/harborgate/src/server.rs")

    for route in [
        "/api/home-assistant/status",
        "/api/home-assistant/config",
        "/api/home-assistant/test",
        "/api/home-assistant/sync",
        "/api/home-assistant/entities",
        "/api/home-assistant/services",
        "/api/harboros/apps/home-assistant/status",
        "/api/harboros/apps/home-assistant/install-plan",
        "/api/harboros/apps/home-assistant/install",
    ]:
        assert route in backend

    for path in [
        "/home-assistant/status",
        "/home-assistant/config",
        "/home-assistant/test",
        "/home-assistant/sync",
        "/home-assistant/entities",
        "/home-assistant/services",
        "/harboros/apps/home-assistant/status",
    ]:
        assert path in service

    assert "id: 'home-assistant'" in registry
    assert registry.index("id: 'devices-aiot'") < registry.index("id: 'home-assistant'")
    assert "state.data.pageId === 'home-assistant'" in panel
    assert "state.data.pageId === 'devices-aiot'" in panel
    assert "beacon_proxy_target_path(\"home-assistant/status\"" in gate
    for forbidden in ["access_token", "entity_count", "HomeAssistantClient", "provider_kind=bridge"]:
        assert forbidden not in gate


def test_harbor_assistant_index_run_copy_preserves_async_job_boundary() -> None:
    component = read_doc("frontend/harbor-assistant/src/app/pages/desk-page.component.ts")
    panel = read_doc("frontend/harbor-assistant/src/app/shared/page-state-panel.component.html")

    message_body = component.split("private knowledgeIndexRunMessage", 1)[1].split(
        "private runDeviceAction", 1
    )[0]

    assert "queued" in message_body
    assert "accepted" in message_body
    assert "Track progress in Index jobs" in message_body
    assert "job_ids" in component
    assert "Knowledge index finished" not in message_body
    assert "Knowledge index completed" not in message_body
    assert "知识库索引已完成" not in message_body

    assert "Queueing knowledge index refresh" in panel
    assert "Queueing..." in panel
    assert "Running knowledge index" not in panel
    assert "Indexing..." not in panel
