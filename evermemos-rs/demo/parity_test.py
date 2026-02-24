#!/usr/bin/env python3
"""
evermemos-rs  Parity Test
=========================
Measures: does the Rust implementation reach Python-level completeness?

Tests performed
---------------
  P1  Health check
  P2  Memorize — store assistant_chat_en.json (104 msgs)
  P3  Wait for async extraction to finish
  P4  Fetch memories (paginated)
  P5  Search — KEYWORD  (episodic_memory)
  P6  Search — VECTOR   (episodic_memory)
  P7  Search — HYBRID   (episodic_memory)
  P8  Search — RRF      (episodic_memory)
  P9  Search — AGENTIC  (episodic_memory)
  P10 Search — foresight_record
  P11 Search — event_log_record
  P12 Multi-type search (all)
  P13 Date-range filter
  P14 top_k boundary (1 and 50)
  P15 Delete memories and verify empty
  P16 Missing-feature detection (conversation-meta, profile)

Usage
-----
  # terminal A – start Rust server
  cd evermemos-rs && cargo run --bin evermemos

  # terminal B – run parity test
  cd evermemos-rs
  source ../.venv/bin/activate   # or just use python from PATH
  python demo/parity_test.py [--url http://localhost:8080]
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import httpx

# ──────────────────────────────────────────────────────────────────────────────
DATA_FILE = Path(__file__).resolve().parents[2] / "data" / "assistant_chat_en.json"
USER_ID   = "parity-test-user"
GROUP_ID  = "parity-test-group"
ORG_ID    = "parity-test-org"

# Retrieval queries tuned to the assistant_chat_en conversation content
QUERIES = {
    "sports":  "What sports does the user like?",
    "travel":  "Did the user travel anywhere?",
    "health":  "What are the user's health conditions?",
}

# ──────────────────────────────────────────────────────────────────────────────

@dataclass
class Result:
    id: str
    ok: bool
    detail: str = ""
    extra: dict = field(default_factory=dict)


class ParityTester:
    def __init__(self, base_url: str):
        self.base = base_url.rstrip("/")
        self.results: list[Result] = []
        self.client: httpx.AsyncClient | None = None

    # ── helpers ───────────────────────────────────────────────────────────────

    def record(self, id: str, ok: bool, detail: str = "", **extra):
        r = Result(id, ok, detail, extra)
        flag = "✅" if ok else "❌"
        print(f"  {flag} {id}: {detail}")
        self.results.append(r)
        return r

    async def get(self, path: str, **params) -> tuple[int, Any]:
        r = await self.client.get(f"{self.base}{path}", params=params or None)
        return r.status_code, r.json() if r.content else {}

    async def post(self, path: str, body: dict) -> tuple[int, Any]:
        r = await self.client.post(f"{self.base}{path}", json=body)
        return r.status_code, r.json() if r.content else {}

    async def delete(self, path: str, body: dict) -> tuple[int, Any]:
        r = await self.client.request("DELETE", f"{self.base}{path}", json=body)
        return r.status_code, r.json() if r.content else {}

    def _headers(self):
        return {"X-Organization-Id": ORG_ID}

    # ═══════════════════════════════════════════════════════════════════════════
    # P1 — Health
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_health(self):
        print("\n── P1  Health check ──")
        try:
            code, body = await self.get("/health")
            ok = code == 200 and body.get("status") in ("ok", "success")
            self.record("P1", ok, f"HTTP {code} status={body.get('status')}")
        except Exception as e:
            self.record("P1", False, f"connection failed: {e}")

    # ═══════════════════════════════════════════════════════════════════════════
    # P2 — Memorize (all 104 messages)
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_memorize(self) -> tuple[int, int]:
        """Returns (accumulated_count, extracted_count)."""
        print("\n── P2  Memorize 104 messages ──")
        data = json.loads(DATA_FILE.read_text())
        messages = data["conversation_list"]

        accumulated = extracted = errors = 0
        for msg in messages:
            body = {
                "message_id": msg["message_id"],
                "create_time": msg["create_time"],
                "sender":      msg["sender"],
                "sender_name": msg.get("sender_name", msg["sender"]),
                "content":     msg["content"],
                "user_id":     USER_ID,
                "group_id":    GROUP_ID,
            }
            try:
                code, resp = await self.post("/api/v1/memories", body)
                status = resp.get("result", {}).get("status", "")
                if code == 200 and status == "accumulating":
                    accumulated += 1
                elif code == 200 and status == "extracted":
                    extracted += 1
                else:
                    errors += 1
                    print(f"  ⚠️  msg {msg['message_id']}: HTTP {code} {resp}")
            except Exception as e:
                errors += 1
                print(f"  ⚠️  msg {msg['message_id']}: {e}")

        ok = extracted > 0
        self.record(
            "P2", ok,
            f"{len(messages)} sent — accumulated={accumulated} extracted={extracted} errors={errors}",
            accumulated=accumulated, extracted=extracted, errors=errors,
        )
        return accumulated, extracted

    # ═══════════════════════════════════════════════════════════════════════════
    # P3 — Wait for async extraction
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_wait_extraction(self, seconds: int = 15):
        print(f"\n── P3  Wait {seconds}s for async extraction ──")
        for remaining in range(seconds, 0, -1):
            print(f"  ⏳ {remaining}s …", end="\r", flush=True)
            await asyncio.sleep(1)
        print()

        # Verify something is actually in the DB
        code, body = await self.get(
            "/api/v1/memories",
            user_id=USER_ID, group_id=GROUP_ID, limit=5,
        )
        count = body.get("result", {}).get("total_count", 0)
        ok = code == 200 and count > 0
        self.record("P3", ok, f"fetch after wait: HTTP {code}, memories in DB = {count}")

    # ═══════════════════════════════════════════════════════════════════════════
    # P4 — Fetch (pagination)
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_fetch(self):
        print("\n── P4  Fetch memories (pagination) ──")

        code, body = await self.get(
            "/api/v1/memories",
            user_id=USER_ID, group_id=GROUP_ID, limit=20, offset=0,
        )
        memories = body.get("result", {}).get("memories", [])
        has_more = body.get("result", {}).get("has_more", False)

        ok = code == 200 and len(memories) > 0
        self.record(
            "P4", ok,
            f"HTTP {code}, count={len(memories)}, has_more={has_more}",
            count=len(memories), has_more=has_more,
        )
        if memories:
            m = memories[0]
            print(f"     sample: [{m.get('memory_type')}] {m.get('content','')[:80]}…")

    # ═══════════════════════════════════════════════════════════════════════════
    # P5–P9 — Search by retrieval method
    # ═══════════════════════════════════════════════════════════════════════════

    async def _search(self, label: str, method: str, query: str,
                      memory_types: str | None = None,
                      extra_params: dict | None = None) -> int:
        params: dict[str, Any] = dict(
            query=query,
            user_id=USER_ID,
            group_id=GROUP_ID,
            retrieve_method=method,
            top_k=5,
        )
        if memory_types:
            params["memory_types"] = memory_types
        if extra_params:
            params.update(extra_params)

        t0 = time.monotonic()
        try:
            code, body = await self.get("/api/v1/memories/search", **params)
            elapsed_ms = (time.monotonic() - t0) * 1000
            memories = body.get("result", {}).get("memories", [])
            total = body.get("result", {}).get("total_count", 0)
            ok = code == 200 and total > 0
            self.record(
                label, ok,
                f"HTTP {code}, total={total}, returned={len(memories)}, {elapsed_ms:.0f}ms",
                total=total, count=len(memories), latency_ms=elapsed_ms,
            )
            if memories:
                m = memories[0]
                print(f"     top-1 [{m.get('memory_type')}] score={m.get('score',0):.4f} "
                      f"— {m.get('content','')[:70]}…")
            return total
        except Exception as e:
            self.record(label, False, str(e))
            return 0

    async def test_search_keyword(self):
        print("\n── P5  Search KEYWORD ──")
        await self._search("P5", "KEYWORD", QUERIES["sports"])

    async def test_search_vector(self):
        print("\n── P6  Search VECTOR ──")
        await self._search("P6", "VECTOR", QUERIES["sports"])

    async def test_search_hybrid(self):
        print("\n── P7  Search HYBRID ──")
        await self._search("P7", "HYBRID", QUERIES["health"])

    async def test_search_rrf(self):
        print("\n── P8  Search RRF ──")
        await self._search("P8", "RRF", QUERIES["travel"])

    async def test_search_agentic(self):
        print("\n── P9  Search AGENTIC ──")
        await self._search("P9", "AGENTIC", QUERIES["sports"])

    # ═══════════════════════════════════════════════════════════════════════════
    # P10–P11 — Memory type coverage
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_foresight(self):
        print("\n── P10 Search foresight_record ──")
        await self._search("P10", "VECTOR", QUERIES["health"],
                           memory_types="foresight_record")

    async def test_event_log(self):
        print("\n── P11 Search event_log_record ──")
        await self._search("P11", "KEYWORD", QUERIES["travel"],
                           memory_types="event_log_record")

    # ═══════════════════════════════════════════════════════════════════════════
    # P12 — Multi-type search
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_multi_type(self):
        print("\n── P12 Multi-type search ──")
        await self._search("P12", "RRF", QUERIES["sports"],
                           memory_types="episodic_memory,foresight_record,event_log_record")

    # ═══════════════════════════════════════════════════════════════════════════
    # P13 — Date-range filter
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_date_filter(self):
        print("\n── P13 Date-range filter ──")
        # Data spans ~2025; use a valid window that should include messages
        await self._search(
            "P13", "KEYWORD", QUERIES["sports"],
            extra_params={
                "start_time": "2025-01-01T00:00:00Z",
                "end_time":   "2026-12-31T23:59:59Z",
            },
        )

    # ═══════════════════════════════════════════════════════════════════════════
    # P14 — top_k boundaries
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_topk_boundaries(self):
        print("\n── P14 top_k boundaries ──")
        for topk, label in [(1, "P14a"), (50, "P14b")]:
            params: dict[str, Any] = dict(
                query=QUERIES["sports"],
                user_id=USER_ID,
                group_id=GROUP_ID,
                retrieve_method="KEYWORD",
                top_k=topk,
            )
            try:
                code, body = await self.get("/api/v1/memories/search", **params)
                memories = body.get("result", {}).get("memories", [])
                ok = code == 200 and len(memories) <= topk
                self.record(label, ok, f"top_k={topk} → returned {len(memories)}")
            except Exception as e:
                self.record(label, False, str(e))

    # ═══════════════════════════════════════════════════════════════════════════
    # P15 — Delete + verify
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_delete(self):
        print("\n── P15 Delete and verify ──")
        code, body = await self.delete(
            "/api/v1/memories",
            {"user_id": USER_ID, "group_id": GROUP_ID},
        )
        deleted = body.get("result", {}).get("deleted_count", -1)
        ok_del = code == 200 and deleted >= 0
        self.record("P15a", ok_del, f"DELETE HTTP {code}, deleted_count={deleted}")

        # Now fetch — should be empty (allow small delay for consistency)
        await asyncio.sleep(2)
        code2, body2 = await self.get(
            "/api/v1/memories",
            user_id=USER_ID, group_id=GROUP_ID, limit=5,
        )
        count = body2.get("result", {}).get("total_count", 0)
        ok_empty = code2 == 200 and count == 0
        self.record("P15b", ok_empty, f"after delete: fetch count={count}")

    # ═══════════════════════════════════════════════════════════════════════════
    # P16 — Feature gap detection (things Python has, Rust may not)
    # ═══════════════════════════════════════════════════════════════════════════

    async def test_feature_gaps(self):
        print("\n── P16 Feature gap detection ──")

        gaps: list[str] = []

        # conversation-meta save endpoint
        code, body = await self.post("/api/v1/memories/conversation-meta", {
            "group_id": GROUP_ID,
            "scene": "assistant",
            "name": "test",
        })
        if code == 404:
            gaps.append("conversation-meta POST (404)")
            self.record("P16a", False, "conversation-meta POST not implemented (404)")
        else:
            self.record("P16a", True, f"conversation-meta POST exists (HTTP {code})")

        # conversation-meta get endpoint
        code2, _ = await self.get("/api/v1/memories/conversation-meta",
                                  group_id=GROUP_ID)
        if code2 == 404:
            gaps.append("conversation-meta GET (404)")
            self.record("P16b", False, "conversation-meta GET not implemented (404)")
        else:
            self.record("P16b", True, f"conversation-meta GET exists (HTTP {code2})")

        # profile memory type
        code3, body3 = await self.get(
            "/api/v1/memories/search",
            query="user preference",
            user_id=USER_ID,
            retrieve_method="KEYWORD",
            memory_types="profile",
            top_k=3,
        )
        memories = body3.get("result", {}).get("memories", [])
        profile_supported = code3 == 200 and len(memories) > 0
        if not profile_supported:
            gaps.append("profile memory type")
            self.record("P16c", False,
                        f"profile memory: HTTP {code3}, returned={len(memories)}")
        else:
            self.record("P16c", True, f"profile memory: {len(memories)} results")

        # request status endpoint
        code4, _ = await self.get("/api/v1/memories/status", request_id="dummy")
        if code4 == 404:
            gaps.append("request status endpoint (404)")
            self.record("P16d", False, "status endpoint not implemented (404)")
        else:
            self.record("P16d", True, f"status endpoint exists (HTTP {code4})")

        return gaps

    # ═══════════════════════════════════════════════════════════════════════════
    # Summary
    # ═══════════════════════════════════════════════════════════════════════════

    def print_summary(self):
        passed  = [r for r in self.results if r.ok]
        failed  = [r for r in self.results if not r.ok]
        total   = len(self.results)
        pct     = 100 * len(passed) / total if total else 0

        width = 70
        print()
        print("═" * width)
        print(f"  Parity Test Results — evermemos-rs vs Python baseline")
        print("═" * width)
        print(f"  Total : {total}")
        print(f"  ✅ Pass: {len(passed)}")
        print(f"  ❌ Fail: {len(failed)}")
        print(f"  Score : {pct:.1f}%")
        print()

        if failed:
            print("  Failed tests:")
            for r in failed:
                print(f"    {r.id}: {r.detail}")
            print()

        # Parity verdict
        print("  Feature coverage vs Python implementation:")
        core_tests = [r for r in self.results if not r.id.startswith("P16")]
        core_pass  = [r for r in core_tests if r.ok]
        core_pct   = 100 * len(core_pass) / len(core_tests) if core_tests else 0

        gap_tests  = [r for r in self.results if r.id.startswith("P16")]
        gap_pass   = [r for r in gap_tests if r.ok]
        gap_pct    = 100 * len(gap_pass) / len(gap_tests) if gap_tests else 0

        print(f"    Core pipeline (P1–P15) : {len(core_pass)}/{len(core_tests)}  ({core_pct:.0f}%)")
        print(f"    Extended features (P16): {len(gap_pass)}/{len(gap_tests)}  ({gap_pct:.0f}%)")
        print()

        if core_pct >= 90 and gap_pct == 100:
            verdict = "🟢  FULL PARITY — Rust implementation matches Python baseline"
        elif core_pct >= 80:
            verdict = "🟡  NEAR PARITY — Core pipeline works, gaps in extended features"
        elif core_pct >= 50:
            verdict = "🟠  PARTIAL — Major features work but retrieval has gaps"
        else:
            verdict = "🔴  BELOW PARITY — Rust implementation needs significant work"

        print(f"  {verdict}")
        print("═" * width)


async def run(base_url: str):
    async with httpx.AsyncClient(
        headers={"X-Organization-Id": ORG_ID},
        timeout=60.0,
    ) as client:
        tester = ParityTester(base_url)
        tester.client = client

        print(f"\n🦀  evermemos-rs Parity Test")
        print(f"    Target : {base_url}")
        print(f"    Data   : {DATA_FILE}")
        print(f"    User   : {USER_ID}")
        print(f"    Group  : {GROUP_ID}")

        # Pre-clean
        print("\n── Pre-clean ──")
        try:
            await tester.delete("/api/v1/memories", {"user_id": USER_ID, "group_id": GROUP_ID})
            print("  previous test data cleared")
        except Exception:
            pass

        await tester.test_health()
        _, extracted = await tester.test_memorize()

        # Only wait when extraction actually happened
        wait_secs = 20 if extracted > 0 else 5
        await tester.test_wait_extraction(wait_secs)

        await tester.test_fetch()
        await tester.test_search_keyword()
        await tester.test_search_vector()
        await tester.test_search_hybrid()
        await tester.test_search_rrf()
        await tester.test_search_agentic()
        await tester.test_foresight()
        await tester.test_event_log()
        await tester.test_multi_type()
        await tester.test_date_filter()
        await tester.test_topk_boundaries()
        await tester.test_delete()
        await tester.test_feature_gaps()

        tester.print_summary()
        return tester.results


def main():
    parser = argparse.ArgumentParser(description="evermemos-rs parity test")
    parser.add_argument("--url", default="http://localhost:8080",
                        help="Rust server base URL (default: http://localhost:8080)")
    args = parser.parse_args()

    results = asyncio.run(run(args.url))
    # Exit non-zero if any core test failed
    core_failed = [r for r in results if not r.id.startswith("P16") and not r.ok]
    sys.exit(1 if core_failed else 0)


if __name__ == "__main__":
    main()
