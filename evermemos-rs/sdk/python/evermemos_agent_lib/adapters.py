from __future__ import annotations

from typing import Dict, List, Optional

from .client import EverMemOSClient


def _render_memories(memories: List[Dict]) -> str:
    if not memories:
        return "No relevant memory found."
    lines: List[str] = []
    for idx, memory in enumerate(memories, 1):
        content = str(memory.get("content", "")).strip()
        score = memory.get("score")
        memory_type = memory.get("memory_type") or memory.get("memoryType") or "unknown"
        if score is None:
            lines.append(f"{idx}. [{memory_type}] {content}")
        else:
            lines.append(f"{idx}. [{memory_type}] (score={float(score):.3f}) {content}")
    return "\n".join(lines)


class MemoryContextBuilder:
    def __init__(
        self,
        client: EverMemOSClient,
        *,
        retrieve_method: str = "HYBRID",
        top_k: int = 5,
        memory_types: Optional[List[str]] = None,
    ) -> None:
        self.client = client
        self.retrieve_method = retrieve_method
        self.top_k = top_k
        self.memory_types = memory_types

    async def build(self, query: str) -> str:
        memories = await self.client.search(
            query=query,
            retrieve_method=self.retrieve_method,
            top_k=self.top_k,
            memory_types=self.memory_types,
        )
        return _render_memories(memories)


def compose_system_prompt(base_prompt: str, memory_context: str) -> str:
    if not memory_context.strip():
        return base_prompt
    return (
        f"{base_prompt.strip()}\n\n"
        "Long-term memory context:\n"
        f"{memory_context.strip()}"
    )


def build_openai_messages(
    *,
    user_input: str,
    memory_context: str,
    base_system_prompt: str = "You are a helpful assistant.",
) -> List[Dict[str, str]]:
    system_prompt = compose_system_prompt(base_system_prompt, memory_context)
    return [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_input},
    ]


def build_langchain_messages(
    *,
    user_input: str,
    memory_context: str,
    base_system_prompt: str = "You are a helpful assistant.",
) -> List[Dict[str, str]]:
    system_prompt = compose_system_prompt(base_system_prompt, memory_context)
    return [
        {"role": "system", "content": system_prompt},
        {"role": "human", "content": user_input},
    ]


def build_llamaindex_chat_history(
    *,
    user_input: str,
    memory_context: str,
    base_system_prompt: str = "You are a helpful assistant.",
) -> List[Dict[str, str]]:
    system_prompt = compose_system_prompt(base_system_prompt, memory_context)
    return [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_input},
    ]
