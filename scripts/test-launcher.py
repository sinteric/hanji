#!/usr/bin/env python3
"""Offline launcher regressions; no Rust build or release downloads required."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
LAUNCHER = ROOT / "plugins/hanji/skills/office-documents/scripts/hanji"
VERSION = re.search(r"^version=(\S+)$", LAUNCHER.read_text(), re.M).group(1)
TARGET = "aarch64-apple-darwin"
ARCHIVE = f"hanji-{VERSION}-{TARGET}.tar.gz"
BASE = f"https://github.com/sinteric/hanji/releases/download/v{VERSION}"

# Only these fake downloaders are on PATH. The response metadata follows
# curl --write-out and wget -S, including a redirect before
# the final HTTP response. No command in this script accesses the network.
DOWNLOADER = r'''
import json
import os
from pathlib import Path
import sys

args = sys.argv[1:]
curl = Path(sys.argv[0]).name == "curl"
url = args[-1]
name = url.rsplit("/", 1)[-1]
with open(os.environ["HANJI_TEST_LOG"], "a") as log:
    log.write(json.dumps(url) + "\n")
failed = name == os.environ.get("HANJI_TEST_FAIL_FILE")
status = os.environ.get("HANJI_TEST_STATUS", "404") if failed else "200"
if curl and "--write-out" in args:
    print(status, end="")
elif not curl and "-S" in args and status != "000":
    print("  HTTP/1.1 302 Found\n  HTTP/2 " + status + " Response", file=sys.stderr)
if failed:
    sys.exit(int(os.environ.get("HANJI_TEST_EXIT", "22" if curl else "8")))
destination = Path(args[args.index("-o" if curl else "-O") + 1])
destination.write_bytes((Path(os.environ["HANJI_TEST_FIXTURES"]) / name).read_bytes())
'''


class LauncherChecks:
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="hanji-launcher-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.tools = self.root / "tools"
        self.tools.mkdir()
        # Isolate PATH from any real hanji, curl or wget installation.
        for name in ("awk", "chmod", "gzip", "mkdir", "mktemp", "mv", "rm", "tar"):
            source = shutil.which(name)
            self.assertIsNotNone(source, f"test needs {name}")
            (self.tools / name).symlink_to(source)
        checksum = "sha256sum" if shutil.which("sha256sum") else "shasum"
        (self.tools / checksum).symlink_to(shutil.which(checksum))
        self.executable(self.tools / "uname", '#!/bin/sh\ncase "$1" in -s) echo Darwin;; -m) echo arm64;; esac\n')
        self.executable(self.tools / self.downloader, f"#!{sys.executable}\n" + DOWNLOADER)
        self.fixtures = self.root / "fixtures"
        self.fixtures.mkdir()
        program = b'#!/bin/sh\nprintf "fixture\\n"\nprintf "%s\\n" "$@"\n'
        with tarfile.open(self.fixtures / ARCHIVE, "w:gz") as archive:
            entry = tarfile.TarInfo(f"hanji-{VERSION}-{TARGET}/hanji")
            entry.size = len(program)
            entry.mode = 0o755
            archive.addfile(entry, io.BytesIO(program))
        digest = hashlib.sha256((self.fixtures / ARCHIVE).read_bytes()).hexdigest()
        (self.fixtures / "SHA256SUMS").write_text(f"{digest}  {ARCHIVE}\n")
        self.log = self.root / "requests.jsonl"
        self.tmp = self.root / "tmp"
        self.tmp.mkdir()
        self.cache = self.root / "cache"
        self.env = dict(os.environ)
        for name in ("HANJI_BIN", "HANJI_DOWNLOAD_BASE"):
            self.env.pop(name, None)
        self.env.update(
            PATH=str(self.tools),
            XDG_CACHE_HOME=str(self.cache),
            TMPDIR=str(self.tmp),
            HANJI_TEST_LOG=str(self.log),
            HANJI_TEST_FIXTURES=str(self.fixtures),
        )

    @staticmethod
    def executable(path, text):
        path.write_text(text)
        path.chmod(0o755)

    def run_launcher(self, *args):
        return subprocess.run(
            [shutil.which("sh"), str(LAUNCHER), *args],
            env=self.env, cwd=self.root, text=True, capture_output=True, timeout=10,
        )

    def requests(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []

    def assert_failed_cleanly(self, result):
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "", result.stdout)
        self.assertFalse(self.cache.exists(), "failed downloads must not populate the cache")
        self.assertEqual(list(self.tmp.iterdir()), [], "temporary downloads must be removed")

    def missing_asset(self, filename):
        self.env["HANJI_TEST_FAIL_FILE"] = filename
        result = self.run_launcher("--version")
        self.assert_failed_cleanly(result)
        self.assertIn("release asset not found (HTTP 404)", result.stderr)
        self.assertIn(f"{BASE}/{filename}", result.stderr)
        self.assertIn(f"v{VERSION}", result.stderr)
        self.assertIn("HANJI_BIN", result.stderr)
        self.assertIn("cargo build --locked -p hanji-cli", result.stderr)
        self.assertNotIn("allow network access", result.stderr)
        expected = [f"{BASE}/{ARCHIVE}"]
        if filename == "SHA256SUMS":
            expected.append(f"{BASE}/SHA256SUMS")
        self.assertEqual(self.requests(), expected, "never try a different release version")

    def test_missing_archive_identifies_the_unpublished_release(self):
        self.missing_asset(ARCHIVE)

    def test_missing_checksums_also_refuses_the_release(self):
        self.missing_asset("SHA256SUMS")

    def test_other_http_errors_keep_their_status(self):
        self.env.update(HANJI_TEST_FAIL_FILE=ARCHIVE, HANJI_TEST_STATUS="403")
        result = self.run_launcher("--version")
        self.assert_failed_cleanly(result)
        self.assertIn("HTTP 403", result.stderr)
        self.assertNotIn("not published", result.stderr)
        self.assertNotIn("allow network access", result.stderr)

    def test_transport_failure_keeps_network_guidance(self):
        self.env.update(HANJI_TEST_FAIL_FILE=ARCHIVE, HANJI_TEST_STATUS="000", HANJI_TEST_EXIT="6")
        result = self.run_launcher("--version")
        self.assert_failed_cleanly(result)
        self.assertIn("allow network access", result.stderr)
        self.assertNotIn("HTTP 404", result.stderr)

    def test_verified_binary_is_cached_and_arguments_are_preserved(self):
        result = self.run_launcher("--version", "two words")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "fixture\n--version\ntwo words\n")
        self.assertEqual(self.requests(), [f"{BASE}/{ARCHIVE}", f"{BASE}/SHA256SUMS"])
        self.env["HANJI_TEST_FAIL_FILE"] = ARCHIVE
        cached = self.run_launcher("guide")
        self.assertEqual(cached.returncode, 0, cached.stderr)
        self.assertEqual(cached.stdout, "fixture\nguide\n")
        self.assertEqual(len(self.requests()), 2, "a cached binary needs no download")

    def test_checksum_mismatch_never_runs_or_caches_the_download(self):
        (self.fixtures / "SHA256SUMS").write_text(f"{'0' * 64}  {ARCHIVE}\n")
        result = self.run_launcher("--version")
        self.assert_failed_cleanly(result)
        self.assertIn("checksum mismatch", result.stderr)

    def test_explicit_source_build_override_needs_no_download(self):
        binary = self.root / "source build" / "hanji"
        binary.parent.mkdir()
        self.executable(binary, '#!/bin/sh\nprintf "source build\\n"\nprintf "%s\\n" "$@"\n')
        self.env["HANJI_BIN"] = str(binary)
        result = self.run_launcher("--version", "two words")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "source build\n--version\ntwo words\n")
        self.assertEqual(self.requests(), [])


class CurlLauncherTests(LauncherChecks, unittest.TestCase):
    downloader = "curl"


class WgetLauncherTests(LauncherChecks, unittest.TestCase):
    downloader = "wget"


if __name__ == "__main__":
    unittest.main()
