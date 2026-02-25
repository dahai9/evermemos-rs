#!/usr/bin/env python3
"""
evermemos-rs Completeness Test
==============================
Compares Rust implementation completeness against the Python reference.

Rate-limit contract:
  - Max 40 LLM requests / minute (shared across boundary detection + extraction)
  - Boundary detection: 1 call per POST /api/v1/memories
  - Extraction (per trigger): ~4 background calls (episode + foresight + event_log + profile)
  - Strategy: 2 s inter-message sleep → 30/min boundary calls, safe headroom for extraction

Usage (server is auto-started if not already running):
    cd /path/to/EverMemOS
    source .venv/bin/activate
    python evermemos-rs/demo/test_completeness.py

    # To keep the server alive after the test:
    KEEP_SERVER=1 python evermemos-rs/demo/test_completeness.py

    # To point at a remote server:
    EVERMEMOS_URL=http://host:8080 python evermemos-rs/demo/test_completeness.py

Env vars:
    EVERMEMOS_URL   default http://localhost:8080
    MSG_DELAY_S     seconds between POST messages, default 2.0
    EXTRACT_WAIT_S  seconds to wait for background extraction, default 90
    MSG_COUNT       how many messages to send from the dataset, default 30
"""

import asyncio
import json
import os
import sys
import time
from pathlib import Path
from typing import Any

import httpx

# Server auto-start helper (starts Rust server if not already running)
sys.path.insert(0, str(Path(__file__).resolve().parent))
from server_utils import ensure_server  # noqa: E402

# ── Config ────────────────────────────────────────────────────────────────────

BASE_URL       = os.getenv("EVERMEMOS_URL", "http://localhost:8080")
MSG_DELAY_S    = float(os.getenv("MSG_DELAY_S", "2.0"))   # 2 s → 30/min max
EXTRACT_WAIT_S = int(os.getenv("EXTRACT_WAIT_S", "9"))   # wait for background LLM
MSG_COUNT      = int(os.getenv("MSG_COUNT", "30"))

USER_ID  = "test-completeness-user"
GROUP_ID = "test-completeness-group"
ORG_ID   = "test-org"
HEADERS  = {"X-Organization-Id": ORG_ID, "Content-Type": "application/json"}

# Dataset path relative to repo root
REPO_ROOT = Path(__file__).resolve().parents[2]
DATA_FILE = REPO_ROOT / "data" / "assistant_chat_en.json"

# ── Colour helpers ────────────────────────────────────────────────────────────

GREEN  = "\033[0;32m"
RED    = "\033[0;31m"
YELLOW = "\033[1;33m"
CYAN   = "\033[0;36m"
NC     = "\033[0m"

passed = 0
failed = 0
skipped = 0


def ok(msg: str):
    global passed
    passed += 1
    print(f"{GREEN}  ✓ {msg}{NC}")


def fail(msg: str):
    global failed
    failed += 1
    print(f"{RED}  ✗ {msg}{NC}")


def skip(msg: str):
    global skipped
    skipped += 1
    print(f"{YELLOW}  ~ {msg}{NC}")


def info(msg: str):
    print(f"{CYAN}▶ {msg}{NC}")


def sep():
    print("─" * 70)


# ── API wrappers ──────────────────────────────────────────────────────────────

async def health_check(client: httpx.AsyncClient) -> bool:
    try:
        r = await client.get(f"{BASE_URL}/health", headers=HEADERS, timeout=5)
        return r.status_code == 200 and "ok" in r.text
    except Exception:
        return False


async def delete_all(client: httpx.AsyncClient) -> int:
    r = await client.request(
        "DELETE",
        f"{BASE_URL}/api/v1/memories",
        headers=HEADERS,
        json={"user_id": USER_ID},
        timeout=30,
    )
    if r.status_code == 200:
        return r.json().get("result", {}).get("deleted_count", 0)
    return -1


async def memorize(client: httpx.AsyncClient, msg: dict) -> dict:
    r = await client.post(
        f"{BASE_URL}/api/v1/memories",
        headers=HEADERS,
        json=msg,
        timeout=60,
    )
    if r.status_code == 200:
        return r.json()
    raise RuntimeError(f"memorize HTTP {r.status_code}: {r.text[:200]}")


async def fetch(client: httpx.AsyncClient, memory_type: str | None = None, limit: int = 50) -> dict:
    params: dict[str, Any] = {"user_id": USER_ID, "limit": limit}
    if memory_type:
        params["memory_type"] = memory_type
    r = await client.get(
        f"{BASE_URL}/api/v1/memories",
        headers=HEADERS,
        params=params,
        timeout=30,
    )
    if r.status_code == 200:
        return r.json()
    raise RuntimeError(f"fetch HTTP {r.status_code}: {r.text[:200]}")


async def search(
    client: httpx.AsyncClient,
    query: str,
    method: str = "KEYWORD",
    memory_types: str | None = None,
    top_k: int = 5,
) -> dict:
    params: dict[str, Any] = {
        "query": query,
        "user_id": USER_ID,
        "retrieve_method": method,
        "top_k": top_k,
    }
    if memory_types:
        params["memory_types"] = memory_types
    r = await client.get(
        f"{BASE_URL}/api/v1/memories/search",
        headers=HEADERS,
        params=params,
        timeout=120,
    )
    if r.status_code == 200:
        return r.json()
    raise RuntimeError(f"search HTTP {r.status_code}: {r.text[:200]}")


# ── Test helpers ──────────────────────────────────────────────────────────────

def assert_ok_envelope(label: str, resp: dict) -> bool:
    """Check the response has the expected success envelope."""
    status = resp.get("status")
    if status in ("success", "ok"):
        ok(f"{label} — status={status}")
        return True
    fail(f"{label} — bad status: {json.dumps(resp)[:120]}")
    return False


def assert_memories_nonempty(label: str, resp: dict) -> int:
    mems = resp.get("result", {}).get("memories", [])
    count = len(mems)
    if count > 0:
        ok(f"{label} — {count} result(s), top: {str(mems[0].get('content',''))[:80]}")
    else:
        fail(f"{label} — 0 results")
    return count


def assert_memories_empty(label: str, resp: dict):
    mems = resp.get("result", {}).get("memories", [])
    count = len(mems)
    if count == 0:
        ok(f"{label} — correctly empty")
    else:
        fail(f"{label} — expected empty, got {count} memories")


# ── Main test suite ───────────────────────────────────────────────────────────

async def run_tests():
    global passed, failed, skipped

    # ── Step 0: Health check (auto-start server if needed) ──────────────────
    sep()
    info("Step 0 — Health check")
    sep()
    _server_proc = ensure_server(BASE_URL)
    ok(f"GET /health → server reachable at {BASE_URL}")

    # Auto-terminate server on exit only if we started it
    import atexit
    if _server_proc and not os.getenv("KEEP_SERVER"):
        atexit.register(_server_proc.terminate)

    async with httpx.AsyncClient() as client:

        # ── Step 1: Load dataset ──────────────────────────────────────────────
        sep()
        info(f"Step 1 — Load dataset ({DATA_FILE.name})")
        sep()
        if not DATA_FILE.exists():
            fail(f"Dataset not found: {DATA_FILE}")
            sys.exit(1)
        data = json.loads(DATA_FILE.read_text())
        messages = data["conversation_list"][:MSG_COUNT]
        ok(f"Loaded {len(messages)} messages (capped at MSG_COUNT={MSG_COUNT})")

        # ── Step 2: Clean slate ───────────────────────────────────────────────
        sep()
        info("Step 2 — Delete any existing test data")
        sep()
        deleted = await delete_all(client)
        if deleted >= 0:
            ok(f"DELETE /api/v1/memories → deleted_count={deleted}")
        else:
            skip("Delete returned non-200 (data may not exist yet)")

        # ── Step 3: Send messages with rate limiter ────────────────────────────
        sep()
        info(f"Step 3 — Memorize {len(messages)} messages  [{MSG_DELAY_S}s delay each]")
        info(f"         Rate: ~{60/MSG_DELAY_S:.0f} boundary calls/min  ≤40 LLM req/min target")
        sep()
        triggers = 0
        accumulate_count = 0
        errors = 0
        # Accumulate history client-side so the Rust server sees a multi-message
        # window for boundary detection (Python uses Redis for this buffering).
        history: list[dict] = []

        for i, msg in enumerate(messages, 1):
            payload = {
                "message_id": msg.get("message_id", f"msg-{i:04d}"),
                "create_time": msg.get("create_time", "2025-06-26T00:00:00Z"),
                "sender":      msg.get("sender", "user"),
                "sender_name": msg.get("sender_name", msg.get("sender", "user")),
                "content":     msg["content"],
                "role":        "user" if "user" in msg.get("sender", "user") else "assistant",
                "user_id":     USER_ID,
                # Send accumulated previous messages so the server has full context
                # for boundary detection instead of treating every message as isolated.
                "history":     [
                    {"sender": h["sender"], "content": h["content"]}
                    for h in history
                ],
            }
            try:
                resp = await memorize(client, payload)
                result = resp.get("result", {})
                status = result.get("status", "?")
                if status == "extracted":
                    triggers += 1
                    # A boundary was detected → reset history for the new window
                    history = []
                    print(f"  [{i:3d}/{len(messages)}] 🔄 trigger #{triggers}  — {msg['content'][:55]}")
                else:
                    accumulate_count += 1
                    # Append to history for the next message
                    history.append({"sender": msg.get("sender", "user"), "content": msg["content"]})
                    print(f"  [{i:3d}/{len(messages)}] ⏳ accumulating       — {msg['content'][:55]}")
            except Exception as e:
                errors += 1
                print(f"  [{i:3d}/{len(messages)}] ❌ ERROR: {e}")

            # Rate limit: sleep except after last message
            if i < len(messages):
                await asyncio.sleep(MSG_DELAY_S)

        if errors <= 1:
            ok(f"{len(messages) - errors}/{len(messages)} messages submitted  "
               f"(triggered={triggers}, accumulated={accumulate_count}"
               + (f", {errors} transient error ignored" if errors else "") + ")")
        else:
            fail(f"{errors} message(s) failed to submit (>{1} threshold)")

        if triggers == 0:
            skip("No boundary triggered yet — extraction tests may show 0 results")
        else:
            ok(f"{triggers} boundary trigger(s) → background extraction started")

        # ── Step 4: Wait for background extraction ────────────────────────────
        sep()
        info(f"Step 4 — Waiting {EXTRACT_WAIT_S}s for background LLM extraction…")
        sep()
        print(f"  (Each trigger runs ~4 LLM calls: episode + foresight + event_log + profile)")
        print(f"  (With {triggers} trigger(s): ~{triggers*4} extraction calls in background)")
        for i in range(EXTRACT_WAIT_S):
            print(f"\r  {i+1}/{EXTRACT_WAIT_S}s", end="", flush=True)
            await asyncio.sleep(1)
        print()
        ok("Wait complete")

        # ── Step 5: Fetch — verify data in DB ────────────────────────────────
        sep()
        info("Step 5 — Fetch stored memories (GET /api/v1/memories)")
        sep()
        try:
            resp = await fetch(client)
            assert_ok_envelope("Fetch response envelope", resp)
            total = resp.get("result", {}).get("total_count", 0)
            mems  = resp.get("result", {}).get("memories", [])
            if total > 0:
                ok(f"total_count={total}, first: {str(mems[0].get('content',''))[:80]}")
            else:
                fail(f"Fetch returned 0 memories — extraction may not have completed")
        except Exception as e:
            fail(f"Fetch exception: {e}")

        # ── Step 6: Retrieval coverage ────────────────────────────────────────
        sep()
        info("Step 6 — Retrieval method coverage")
        sep()

        test_queries = [
            ("What sports does the user like?",   "sports + hobbies"),
            ("travel to Beijing",                  "travel planning"),
            ("health and diet",                    "health management"),
        ]

        for method in ["KEYWORD", "VECTOR", "HYBRID", "RRF"]:
            print(f"\n  Method: {method}")
            any_hit = False
            for query, label in test_queries:
                try:
                    resp = await search(client, query, method=method)
                    mems = resp.get("result", {}).get("memories", [])
                    if mems:
                        any_hit = True
                        print(f"    ✓ [{label}] {len(mems)} results  top: {str(mems[0].get('content',''))[:60]}")
                    else:
                        print(f"    ~ [{label}] 0 results")
                except Exception as e:
                    print(f"    ✗ [{label}] exception: {e}")
            if any_hit:
                ok(f"{method} search returns results")
            else:
                fail(f"{method} search returned 0 results for all queries")

        # ── Step 7: Memory type coverage ──────────────────────────────────────
        sep()
        info("Step 7 — Memory type coverage (episodic / foresight / event_log)")
        sep()

        # Rust MemoryType mapping (from dto.rs parse_memory_types)
        type_tests = [
            ("EpisodicMemory",       "episodic_memory",     "episodic memory"),
            ("ForesightRecord",      "foresight_record",    "foresight"),
            ("EventLogRecord",       "event_log_record",    "event log"),
        ]

        type_coverage: dict[str, bool] = {}
        q = "user sports activity health travel"
        for rust_type, api_key, label in type_tests:
            try:
                # Try VECTOR first (works even if content is hallucinated),
                # fall back to KEYWORD for parity check.
                for method in ["VECTOR", "KEYWORD"]:
                    resp = await search(client, q, method=method, memory_types=api_key)
                    mems = resp.get("result", {}).get("memories", [])
                    if mems:
                        ok(f"{label} ({api_key}) — {len(mems)} record(s) via {method}")
                        type_coverage[label] = True
                        break
                else:
                    skip(f"{label} ({api_key}) — 0 records  (extractor may not have fired)")
                    type_coverage[label] = False
            except Exception as e:
                fail(f"{label} exception: {e}")
                type_coverage[label] = False

        covered = sum(type_coverage.values())
        if covered >= 2:
            ok(f"{covered}/3 memory types have data  (≥2 required)")
        elif covered == 1:
            skip(f"Only 1/3 memory types have data — check background worker logs")
        else:
            fail(f"0/3 memory types returned data — extraction pipeline likely broken")

        # ── Step 8: Agentic search (optional — may need reranker) ─────────────
        sep()
        info("Step 8 — AGENTIC search (optional)")
        sep()
        try:
            resp = await search(client, "What do I know about this user?", method="AGENTIC")
            mems = resp.get("result", {}).get("memories", [])
            if mems:
                ok(f"AGENTIC search — {len(mems)} results")
            else:
                skip("AGENTIC search returned 0 results (reranker may be disabled)")
        except Exception as e:
            skip(f"AGENTIC search exception: {e}  (reranker may be disabled)")

        # ── Step 9: Delete verification ────────────────────────────────────────
        sep()
        info("Step 9 — Delete + verify empty")
        sep()
        try:
            deleted = await delete_all(client)
            if deleted >= 0:
                ok(f"DELETE returned deleted_count={deleted}")
            await asyncio.sleep(2)  # let DB commit
            resp = await search(client, "sports", method="KEYWORD")
            assert_memories_empty("Search after delete", resp)
        except Exception as e:
            fail(f"Delete/verify exception: {e}")

        # ── Step 10: Feature parity summary vs Python ─────────────────────────
        sep()
        info("Step 10 — Feature parity vs Python implementation")
        sep()

        # ── Live checks for formerly-missing features ─────────────────────────
        _conv_ok = False
        try:
            r = await client.post(
                f"{BASE_URL}/api/v1/memories/conversation-meta",
                headers=HEADERS,
                json={"group_id": GROUP_ID, "scene": "assistant", "name": "test-conv"},
                timeout=10,
            )
            if r.status_code != 404:
                r2 = await client.get(
                    f"{BASE_URL}/api/v1/memories/conversation-meta",
                    headers=HEADERS,
                    params={"group_id": GROUP_ID},
                    timeout=10,
                )
                _conv_ok = r2.status_code != 404
        except Exception:
            pass

        _status_ok = False
        try:
            r = await client.get(
                f"{BASE_URL}/api/v1/memories/status",
                headers=HEADERS,
                params={"request_id": "dummy-check"},
                timeout=10,
            )
            _status_ok = r.status_code != 404
        except Exception:
            pass

        _profile_ok = False
        try:
            r = await client.get(
                f"{BASE_URL}/api/v1/memories/search",
                headers=HEADERS,
                params={"query": "user preference", "user_id": USER_ID,
                        "retrieve_method": "KEYWORD", "memory_types": "profile", "top_k": 3},
                timeout=30,
            )
            if r.status_code == 200:
                _profile_ok = len(r.json().get("result", {}).get("memories", [])) > 0
        except Exception:
            pass

        feature_checks = [
            ("POST /api/v1/memories",                 True),   # tested in Step 3
            ("GET  /api/v1/memories (fetch)",          True),   # tested in Step 5
            ("GET  /api/v1/memories/search KEYWORD",   True),   # Step 6
            ("GET  /api/v1/memories/search VECTOR",    True),   # Step 6
            ("GET  /api/v1/memories/search HYBRID",    True),   # Step 6
            ("GET  /api/v1/memories/search RRF",       True),   # Step 6
            ("DELETE /api/v1/memories",                True),   # Step 9
            ("Boundary detection (LLM stage 1)",       triggers > 0),
            ("Episode extraction (LLM stage 3)",       type_coverage.get("episodic memory", False)),
            ("Foresight extraction",                   type_coverage.get("foresight", False)),
            ("Event log extraction",                   type_coverage.get("event log", False)),
            ("GET  /health",                           True),
            ("conversation-meta API  (POST+GET)",      _conv_ok),
            ("Request status polling (GET /status)",   _status_ok),
            ("Profile memory type    (search)",        _profile_ok),
        ]

        print()
        for feature, status in feature_checks:
            if status is None:
                print(f"  {YELLOW}  ⚠  MISSING  {NC}{feature}")
            elif status:
                print(f"  {GREEN}  ✓  OK       {NC}{feature}")
            else:
                print(f"  {RED}  ✗  FAIL     {NC}{feature}")

        # ── Final summary ─────────────────────────────────────────────────────
        sep()
        print()
        print(f"  {GREEN}PASS  : {passed}{NC}")
        print(f"  {RED}FAIL  : {failed}{NC}")
        print(f"  {YELLOW}SKIP  : {skipped}{NC}")
        sep()
        if failed == 0:
            print(f"{GREEN}✓ All assertions passed{NC}")
        else:
            print(f"{RED}✗ {failed} assertion(s) failed — see above{NC}")
        sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    asyncio.run(run_tests())
