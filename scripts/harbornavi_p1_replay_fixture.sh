#!/usr/bin/env bash
set -euo pipefail

root="${HARBORNAVI_FIXTURE_ROOT:-/var/tmp/harbornavi-p1}"
run_dir="${root}/run"
log_dir="${root}/logs"
sample_dir="${root}/samples"
config_path="${root}/mediamtx-p1.yml"
container_name="${HARBORNAVI_MEDIAMTX_CONTAINER:-harbornavi-p1-mediamtx}"
rtsp_host="${HARBORNAVI_REPLAY_RTSP_HOST:-192.168.3.82}"
rtsp_port="${HARBORNAVI_REPLAY_RTSP_PORT:-8554}"
sample_path="${HARBORNAVI_REPLAY_SAMPLE:-${sample_dir}/tp231-sub-640x480.mp4}"
paths=(${HARBORNAVI_REPLAY_PATHS:-p1-sim-1 p1-sim-2 p1-sim-3})
offsets=(${HARBORNAVI_REPLAY_OFFSETS:-0 37 74})

usage() {
  cat <<'USAGE'
Usage: harbornavi_p1_replay_fixture.sh <command>

Commands:
  record-sample   Record a short H.264 sample from CAM_REAL_231_RTSP.
  start           Start MediaMTX and publish three looped RTSP replay streams.
  stop            Stop replay publishers and MediaMTX.
  status          Print local fixture process/container status.
  print-k3-env    Print non-secret K3 env exports for CAM_SIM_*_RTSP.

Required for record-sample:
  CAM_REAL_231_RTSP=rtsp://...      Source camera URL, kept out of files.

Optional:
  HARBORNAVI_FIXTURE_ROOT=/var/tmp/harbornavi-p1
  HARBORNAVI_REPLAY_SAMPLE=/var/tmp/harbornavi-p1/samples/tp231-sub-640x480.mp4
  HARBORNAVI_REPLAY_RTSP_HOST=192.168.3.82
  HARBORNAVI_REPLAY_RTSP_PORT=8554
  HARBORNAVI_REPLAY_RECORD_SECONDS=180
  HARBORNAVI_MEDIAMTX_BIN=/var/tmp/harbornavi-p1/mediamtx
USAGE
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 2
  }
}

prepare_dirs() {
  mkdir -p "$run_dir" "$log_dir" "$sample_dir"
}

write_config() {
  prepare_dirs
  {
    cat <<EOF
logLevel: warn
rtsp: yes
rtspAddress: :${rtsp_port}
rtspTransports: [tcp]
rtmp: no
hls: no
webrtc: no
srt: no
paths:
EOF
    for path in "${paths[@]}"; do
      cat <<EOF
  ${path}:
    source: publisher
EOF
    done
  } > "$config_path"
}

start_mediamtx() {
  write_config
  if command -v docker >/dev/null 2>&1; then
    docker rm -f "$container_name" >/dev/null 2>&1 || true
    docker run -d --name "$container_name" --network host \
      -v "${config_path}:/mediamtx.yml:ro" \
      bluenviron/mediamtx:latest >/dev/null
    echo "docker:${container_name}" > "${run_dir}/mediamtx.pid"
    return
  fi

  local bin="${HARBORNAVI_MEDIAMTX_BIN:-${root}/mediamtx}"
  if [[ ! -x "$bin" ]]; then
    echo "error: docker not found and MediaMTX binary is not executable at ${bin}" >&2
    exit 2
  fi
  nohup "$bin" "$config_path" > "${log_dir}/mediamtx.log" 2>&1 &
  echo "$!" > "${run_dir}/mediamtx.pid"
}

start_publishers() {
  need_cmd ffmpeg
  if [[ ! -f "$sample_path" ]]; then
    echo "error: replay sample missing: ${sample_path}" >&2
    echo "hint: run record-sample first or set HARBORNAVI_REPLAY_SAMPLE" >&2
    exit 2
  fi
  rm -f "${run_dir}"/ffmpeg-*.pid
  local index=0
  for path in "${paths[@]}"; do
    local offset="${offsets[$index]:-0}"
    nohup ffmpeg -hide_banner -nostdin -loglevel warning \
      -stream_loop -1 -re -ss "$offset" -i "$sample_path" \
      -an -c:v copy -f rtsp -rtsp_transport tcp \
      "rtsp://127.0.0.1:${rtsp_port}/${path}" \
      > "${log_dir}/${path}.ffmpeg.log" 2>&1 &
    echo "$!" > "${run_dir}/ffmpeg-${path}.pid"
    index=$((index + 1))
  done
}

stop_fixture() {
  shopt -s nullglob
  for pid_file in "${run_dir}"/ffmpeg-*.pid; do
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
    rm -f "$pid_file"
  done
  if [[ -f "${run_dir}/mediamtx.pid" ]]; then
    local mediamtx_pid
    mediamtx_pid="$(cat "${run_dir}/mediamtx.pid")"
    if [[ "$mediamtx_pid" == docker:* ]]; then
      docker rm -f "${mediamtx_pid#docker:}" >/dev/null 2>&1 || true
    elif [[ -n "$mediamtx_pid" ]]; then
      kill "$mediamtx_pid" >/dev/null 2>&1 || true
    fi
    rm -f "${run_dir}/mediamtx.pid"
  fi
}

record_sample() {
  need_cmd ffmpeg
  if [[ -z "${CAM_REAL_231_RTSP:-}" ]]; then
    echo "error: CAM_REAL_231_RTSP is required for record-sample" >&2
    exit 2
  fi
  prepare_dirs
  local seconds="${HARBORNAVI_REPLAY_RECORD_SECONDS:-180}"
  ffmpeg -hide_banner -nostdin -loglevel warning \
    -rtsp_transport tcp -i "$CAM_REAL_231_RTSP" \
    -t "$seconds" -an -c:v copy -y "$sample_path"
  chmod 0600 "$sample_path"
  echo "sample=${sample_path}"
}

status_fixture() {
  echo "root=${root}"
  echo "config=${config_path}"
  echo "sample=${sample_path}"
  if [[ -f "${run_dir}/mediamtx.pid" ]]; then
    echo "mediamtx=$(cat "${run_dir}/mediamtx.pid")"
  else
    echo "mediamtx=stopped"
  fi
  shopt -s nullglob
  for pid_file in "${run_dir}"/ffmpeg-*.pid; do
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      echo "$(basename "$pid_file" .pid)=running"
    else
      echo "$(basename "$pid_file" .pid)=stopped"
    fi
  done
}

print_k3_env() {
  local index=1
  for path in "${paths[@]}"; do
    echo "export CAM_SIM_${index}_RTSP=rtsp://${rtsp_host}:${rtsp_port}/${path}"
    index=$((index + 1))
  done
}

command="${1:-}"
case "$command" in
  record-sample)
    record_sample
    ;;
  start)
    stop_fixture
    start_mediamtx
    sleep 1
    start_publishers
    print_k3_env
    ;;
  stop)
    stop_fixture
    ;;
  status)
    status_fixture
    ;;
  print-k3-env)
    print_k3_env
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
