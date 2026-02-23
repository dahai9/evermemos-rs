#!/usr/bin/env python3
"""
evermemos-rs Demo — mirrors the original simple_demo.py

Demonstrates the Rust rewrite of EverMemOS:
1. Store conversation messages via POST /api/v1/memories
2. Wait for the LLM extraction pipeline
3. Search memories via GET /api/v1/memories/search
4. Fetch all memories via GET /api/v1/memories

Prerequisites:
    # Start the server in another terminal:
    cd evermemos-rs && cargo run --bin evermemos

Run:
    python3 demo/simple_demo.py
    # or with a custom server URL:
    EVERMEMOS_URL=http://localhost:8080 python3 demo/simple_demo.py
"""

import asyncio
import json
import os
import sys
import time
import uuid
from datetime import datetime, timezone
from typing import Optional

try:
    import httpx
except ImportError:
    print("httpx not found — install it:  pip install httpx")
    sys.exit(1)


BASE_URL = os.environ.get("EVERMEMOS_URL", "http://localhost:8080")
DEFAULT_USER_ID = "demo-user-001"
DEFAULT_ORG_ID = "demo-org"


# ─────────────────────────────────────────────────────────────────────────────
# Client wrapper
# ─────────────────────────────────────────────────────────────────────────────

class MemClient:
    def __init__(
        self,
        base_url: str = BASE_URL,
        user_id: str = DEFAULT_USER_ID,
        org_id: str = DEFAULT_ORG_ID,
        api_key: Optional[str] = None,
    ):
        self.base_url = base_url.rstrip("/")
        self.user_id = user_id
        headers = {"X-Organization-Id": org_id, "Content-Type": "application/json"}
        if api_key and api_key != "none":
            headers["Authorization"] = f"Bearer {api_key}"
        self.client = httpx.AsyncClient(base_url=self.base_url, headers=headers, timeout=30.0)

    async def health(self) -> dict:
        r = await self.client.get("/health")
        r.raise_for_status()
        return r.json()

    async def memorize(
        self,
        content: str,
        sender: str = "User",
        role: str = "user",
        sender_name: Optional[str] = None,
    ) -> dict:
        payload = {
            "message_id": str(uuid.uuid4()),
            "create_time": datetime.now(timezone.utc).isoformat(),
            "sender": sender,
            "sender_name": sender_name or sender,
            "user_id": self.user_id,
            "content": content,
            "role": role,
        }
        r = await self.client.post("/api/v1/memories", json=payload)
        r.raise_for_status()
        return r.json()

    async def search(
        self,
        query: str,
        method: str = "KEYWORD",
        top_k: int = 5,
    ) -> dict:
        r = await self.client.get(
            "/api/v1/memories/search",
            params={
                "query": query,
                "user_id": self.user_id,
                "retrieve_method": method,
                "top_k": top_k,
            },
        )
        r.raise_for_status()
        return r.json()

    async def fetch(self, limit: int = 20) -> dict:
        r = await self.client.get(
            "/api/v1/memories",
            params={"user_id": self.user_id, "limit": limit},
        )
        r.raise_for_status()
        return r.json()

    async def delete_all(self) -> dict:
        r = await self.client.request(
            "DELETE", "/api/v1/memories", json={"user_id": self.user_id}
        )
        r.raise_for_status()
        return r.json()

    async def close(self):
        await self.client.aclose()


# ─────────────────────────────────────────────────────────────────────────────
# Pretty print helpers
# ─────────────────────────────────────────────────────────────────────────────

def sep(label: str = ""):
    width = 64
    if label:
        pad = (width - len(label) - 2) // 2
        print("─" * pad + f" {label} " + "─" * pad)
    else:
        print("─" * width)


def print_memories(memories: list, max_content: int = 120):
    if not memories:
        print("  (no memories returned)")
        return
    for i, m in enumerate(memories, 1):
        score = m.get("score", 0.0)
        content = m.get("content", "")[:max_content]
        ts = m.get("timestamp", "")[:19] if m.get("timestamp") else ""
        print(f"  [{i}] score={score:.3f}  ts={ts}")
        print(f"       {content}")


# ─────────────────────────────────────────────────────────────────────────────
# Main demo
# ─────────────────────────────────────────────────────────────────────────────

async def main():
    client = MemClient()

    # ── 0. Health check ───────────────────────────────────────────────────────
    sep("evermemos-rs Demo")
    print(f"  Server: {BASE_URL}")
    try:
        health = await client.health()
        print(f"  Health: {health}")
    except Exception as e:
        print(f"  ✗ Server not reachable: {e}")
        print("  Start it with:  cargo run --bin evermemos")
        return

    # ── 1. Clean previous test data ───────────────────────────────────────────
    sep("Step 1 — Clean test data")
    try:
        r = await client.delete_all()
        print(f"  Deleted: {r.get('result', {}).get('deleted_count', 0)} memories")
    except Exception as e:
        print(f"  (skip cleanup: {e})")

    # ── 2. Store conversations ────────────────────────────────────────────────
    sep("Step 2 — Store conversation messages")

    conversations = [
        ("User",      "user",      "I love playing soccer, often go to the field on weekends"),
        ("Assistant", "assistant", "Soccer is a great sport! Which team do you like?"),
        ("User",      "user",      "I love Barcelona the most, Messi is my idol"),
        ("User",      "user",      "I also enjoy watching basketball, NBA is my favorite"),
        ("User",      "user",      "I will sleep now"),
        ("User",      "user",      "The weather is good today"),
        ("User",      "user",      "The universe is expanding"),
    ]

    for sender, role, content in conversations:
        r = await client.memorize(content, sender=sender, role=role)
        status = r.get("result", {}).get("status", "?")
        print(f"  [{sender}] {status} — {content[:60]}")
        await asyncio.sleep(0.3)

    # ── 3. Wait for LLM pipeline ──────────────────────────────────────────────
    sep("Step 3 — Wait for extraction pipeline (12s)")
    print("  ", end="", flush=True)
    for _ in range(12):
        print(".", end="", flush=True)
        await asyncio.sleep(1)
    print()

    # ── 4. Fetch all memories ─────────────────────────────────────────────────
    sep("Step 4 — Fetch stored memories")
    resp = await client.fetch(limit=20)
    result = resp.get("result", {})
    total = result.get("total_count", 0)
    print(f"  Total memories stored: {total}")
    print_memories(result.get("memories", [])[:3])

    # ── 5. Keyword search (mirrors original demo queries) ─────────────────────
    sep("Step 5 — Keyword Search (BM25)")

    queries = [
        "What sports does the user like?",
        "What is the user's favorite team?",
        "What are the user's hobbies?",
    ]

    for q in queries:
        print(f"\n  Query: {q}")
        resp = await client.search(q, method="KEYWORD", top_k=5)
        mems = resp.get("result", {}).get("memories", [])
        print(f"  Results: {len(mems)}")
        print_memories(mems, max_content=100)

    # ── 6. Vector search ──────────────────────────────────────────────────────
    sep("Step 6 — Vector Search (ANN)")
    print("  (requires embedding service at VECTORIZE_BASE_URL)")

    for q in ["soccer football sports Messi", "NBA basketball"]:
        print(f"\n  Query: {q}")
        try:
            resp = await client.search(q, method="VECTOR", top_k=5)
            mems = resp.get("result", {}).get("memories", [])
            if mems:
                print(f"  Results: {len(mems)}")
                print_memories(mems, max_content=100)
            else:
                print("  No results (embedding service may be offline)")
        except Exception as e:
            print(f"  ✗ {e}")

    # ── 7. Hybrid search ──────────────────────────────────────────────────────
    sep("Step 7 — Hybrid Search (BM25 + Vector RRF)")
    q = "favorite sports hobbies"
    print(f"\n  Query: {q}")
    try:
        resp = await client.search(q, method="HYBRID", top_k=5)
        mems = resp.get("result", {}).get("memories", [])
        print(f"  Results: {len(mems)}")
        print_memories(mems, max_content=100)
    except Exception as e:
        print(f"  ✗ {e}")

    # ── Done ──────────────────────────────────────────────────────────────────
    sep("Done")
    print("  evermemos-rs demo complete ✓")
    print(f"  Server: {BASE_URL}  |  User: {client.user_id}")
    sep()

    await client.close()


if __name__ == "__main__":
    asyncio.run(main())
