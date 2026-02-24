"""
server_utils.py — evermemos-rs server lifecycle helpers
========================================================
Provides ensure_server() which can be embedded in any test/demo script so that
the script is self-contained: it auto-starts the Rust server when not running,
waits for readiness, and (optionally) kills it when done.

Typical usage::

    from server_utils import ensure_server
    proc = ensure_server()           # starts server if needed; None if already up
    try:
        ...
    finally:
        if proc:
            proc.terminate()
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

# ── Paths ─────────────────────────────────────────────────────────────────────

# demo/ → evermemos-rs/
_RS_DIR = Path(__file__).resolve().parent.parent

_BINARY_CANDIDATES = [
    # env override
    os.environ.get("EVERMEMOS_BIN", ""),
    # cargo default target dir (set by .envrc)
    str(Path.home() / ".cargo/target/debug/evermemos"),
    # standard in-tree location
    str(_RS_DIR / "target/debug/evermemos"),
    # installed in PATH
    shutil.which("evermemos") or "",
]


def _find_binary() -> str | None:
    for candidate in _BINARY_CANDIDATES:
        if candidate and Path(candidate).is_file():
            return candidate
    return None


def _health_ok(url: str, timeout: float = 3.0) -> bool:
    try:
        with urllib.request.urlopen(f"{url}/health", timeout=timeout) as r:
            return r.status == 200
    except Exception:
        return False


def ensure_server(
    url: str = "http://localhost:8080",
    *,
    wait_seconds: int = 20,
    log_file: str = "/tmp/evermemos.log",
    auto_kill_existing: bool = False,
) -> subprocess.Popen | None:
    """Ensure the evermemos-rs server is running at *url*.

    Parameters
    ----------
    url:
        Server base URL to health-check.
    wait_seconds:
        How long to wait for the server to become healthy after starting it.
    log_file:
        Where to redirect server stdout/stderr when auto-starting.
    auto_kill_existing:
        If True, kill any existing evermemos process first (useful for clean
        state between test runs). Default False.

    Returns
    -------
    subprocess.Popen | None
        The Popen object if *this call* started the server, else None.
        Callers can call `proc.terminate()` in a finally block.
    """
    if auto_kill_existing:
        subprocess.run(
            ["pkill", "-f", "debug/evermemos"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        time.sleep(1.5)

    if _health_ok(url):
        print(f"  ✓ server already running at {url}")
        return None

    binary = _find_binary()
    if not binary:
        print(
            f"  ✗ evermemos binary not found.\n"
            f"    Build with:  cd {_RS_DIR} && cargo build --bin evermemos\n"
            f"    Or set EVERMEMOS_BIN=/path/to/evermemos",
            file=sys.stderr,
        )
        sys.exit(1)

    print(f"  ⚡ Starting server: {binary}")
    print(f"     cwd={_RS_DIR}  log={log_file}")

    log_fh = open(log_file, "w")
    proc = subprocess.Popen(
        [binary],
        cwd=_RS_DIR,          # rocksdb://./data/surreal needs to resolve here
        stdout=log_fh,
        stderr=log_fh,
    )

    deadline = time.monotonic() + wait_seconds
    dots = 0
    while time.monotonic() < deadline:
        if _health_ok(url, timeout=2.0):
            elapsed = wait_seconds - (deadline - time.monotonic())
            print(f"\r  ✓ server ready in {elapsed:.1f}s (PID={proc.pid})" + " " * 10)
            return proc
        if proc.poll() is not None:
            log_fh.flush()
            snippet = Path(log_file).read_text()[-800:]
            print(f"  ✗ server exited early (rc={proc.returncode})\n{snippet}", file=sys.stderr)
            sys.exit(1)
        sys.stdout.write(f"\r  waiting for server{'.' * (dots % 4)}   ")
        sys.stdout.flush()
        dots += 1
        time.sleep(1)

    print(f"\n  ✗ server not ready after {wait_seconds}s. Check {log_file}", file=sys.stderr)
    proc.terminate()
    sys.exit(1)
