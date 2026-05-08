#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path


REQUIRED_MEDIA_FILES = [
    Path("media-tools/bin/ffmpeg"),
    Path("media-tools/bin/ffprobe"),
    Path("media-tools/NOTICE.txt"),
    Path("media-tools/provenance.json"),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Verify a HarborOS release bundle.")
    parser.add_argument("bundle", help="Path to an extracted harbor-release dir or .tar.gz bundle.")
    parser.add_argument(
        "--require-execute",
        action="store_true",
        help="Require bundled ffmpeg/ffprobe to execute with -version on this host.",
    )
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def extract_if_needed(bundle: Path, temp_dir: Path) -> Path:
    if bundle.is_dir():
        return bundle
    if not bundle.is_file():
        raise SystemExit(f"bundle path not found: {bundle}")
    if not tarfile.is_tarfile(bundle):
        raise SystemExit(f"unsupported bundle format: {bundle}")
    with tarfile.open(bundle, "r:*") as archive:
        archive.extractall(temp_dir)
    roots = [path for path in temp_dir.iterdir() if path.is_dir()]
    if len(roots) != 1:
        raise SystemExit(f"expected one extracted bundle root, found {len(roots)}")
    return roots[0]


def load_checksums(bundle_root: Path) -> dict[str, str]:
    checksums_path = bundle_root / "checksums.sha256"
    if not checksums_path.is_file():
        raise SystemExit("checksums.sha256 is missing")
    entries: dict[str, str] = {}
    for raw_line in checksums_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        digest, _, name = line.partition(" ")
        normalized = name.strip().lstrip("*").removeprefix("./")
        entries[normalized] = digest
    return entries


def verify_checksums(bundle_root: Path, checksums: dict[str, str]) -> None:
    for relative_name, expected in sorted(checksums.items()):
        path = bundle_root / relative_name
        if not path.is_file():
            raise SystemExit(f"checksum entry points at missing file: {relative_name}")
        actual = sha256(path)
        if actual != expected:
            raise SystemExit(
                f"checksum mismatch for {relative_name}: expected {expected}, got {actual}"
            )


def verify_executable_bit(path: Path) -> None:
    mode = path.stat().st_mode
    if not mode & stat.S_IXUSR:
        raise SystemExit(f"missing executable bit: {path}")


def should_execute(require_execute: bool) -> bool:
    if require_execute:
        return True
    return os.name == "posix" and platform.machine().lower() in {"x86_64", "amd64"}


def verify_runs(path: Path) -> str:
    result = subprocess.run(
        [str(path), "-version"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise SystemExit(f"{path.name} -version failed: {result.stderr.strip() or result.stdout.strip()}")
    return (result.stdout.splitlines() or [""])[0].strip()


def verify_manifest(bundle_root: Path) -> dict[str, object]:
    manifest_path = bundle_root / "manifest.json"
    if not manifest_path.is_file():
        raise SystemExit("manifest.json is missing")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    components = manifest.get("components")
    if not isinstance(components, dict):
        raise SystemExit("manifest components must be an object")
    media_tools = components.get("media_tools")
    if not isinstance(media_tools, dict):
        raise SystemExit("manifest components.media_tools is missing")
    for key in ["variant", "source_url", "archive_sha256", "license_profile", "binaries"]:
        if not media_tools.get(key):
            raise SystemExit(f"manifest media_tools.{key} is missing")
    binaries = media_tools.get("binaries")
    if not isinstance(binaries, dict):
        raise SystemExit("manifest media_tools.binaries must be an object")
    for name in ["ffmpeg", "ffprobe"]:
        entry = binaries.get(name)
        if not isinstance(entry, dict) or entry.get("path") != f"media-tools/bin/{name}":
            raise SystemExit(f"manifest media_tools.binaries.{name}.path is invalid")
    return manifest


def main() -> int:
    args = parse_args()
    with tempfile.TemporaryDirectory(prefix="harbor-release-verify-") as temp:
        bundle_root = extract_if_needed(Path(args.bundle), Path(temp))
        checksums = load_checksums(bundle_root)
        verify_checksums(bundle_root, checksums)
        manifest = verify_manifest(bundle_root)

        missing = [str(path) for path in REQUIRED_MEDIA_FILES if not (bundle_root / path).is_file()]
        if missing:
            raise SystemExit(f"missing required media files: {', '.join(missing)}")

        for binary_name in ["ffmpeg", "ffprobe"]:
            relative = f"media-tools/bin/{binary_name}"
            if relative not in checksums:
                raise SystemExit(f"{relative} missing from checksums.sha256")
            verify_executable_bit(bundle_root / relative)

        versions: dict[str, str] = {}
        if should_execute(args.require_execute):
            versions["ffmpeg"] = verify_runs(bundle_root / "media-tools/bin/ffmpeg")
            versions["ffprobe"] = verify_runs(bundle_root / "media-tools/bin/ffprobe")

        print(
            json.dumps(
                {
                    "ok": True,
                    "bundle": str(bundle_root),
                    "version": manifest.get("version", ""),
                    "media_tools": manifest["components"]["media_tools"],
                    "executed_versions": versions,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
