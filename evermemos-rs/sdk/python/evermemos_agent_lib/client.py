from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional
from uuid import uuid4

import httpx


@dataclass
class EverMemOSConfig:
    base_url: str = "http://localhost:8080"
    org_id: str = "default-org"
    user_id: Optional[str] = None
    group_id: Optional[str] = None
    api_key: Optional[str] = None
    timeout: float = 30.0


class _EnvelopeError(RuntimeError):
    pass


class _BaseClient:
    def __init__(self, config: EverMemOSConfig) -> None:
        self.config = config

    def _headers(self) -> Dict[str, str]:
        headers = {
            "X-Organization-Id": self.config.org_id,
            "Content-Type": "application/json",
        }
        if self.config.api_key:
            headers["Authorization"] = f"Bearer {self.config.api_key}"
        return headers

    @staticmethod
    def _unwrap(body: Dict[str, Any]) -> Dict[str, Any]:
        if body.get("status") not in {"success", "ok"}:
            raise _EnvelopeError(body.get("message", "EverMemOS returned non-success response"))
        return body.get("result") or {}

    def _effective_scope(
        self,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
    ) -> Dict[str, Optional[str]]:
        return {
            "user_id": user_id if user_id is not None else self.config.user_id,
            "group_id": group_id if group_id is not None else self.config.group_id,
        }

    def _memorize_payload(
        self,
        *,
        content: str,
        sender: str,
        role: str,
        sender_name: Optional[str],
        message_id: Optional[str],
        create_time: Optional[str],
        user_id: Optional[str],
        group_id: Optional[str],
        history: Optional[List[Dict[str, Any]]],
    ) -> Dict[str, Any]:
        scope = self._effective_scope(user_id=user_id, group_id=group_id)
        payload: Dict[str, Any] = {
            "message_id": message_id or str(uuid4()),
            "create_time": create_time or datetime.now(timezone.utc).isoformat(),
            "sender": sender,
            "sender_name": sender_name or sender,
            "content": content,
            "role": role,
            "user_id": scope["user_id"],
            "group_id": scope["group_id"],
        }
        if history:
            payload["history"] = history
        return payload


class EverMemOSClient(_BaseClient):
    def __init__(
        self,
        *,
        base_url: str = "http://localhost:8080",
        org_id: str = "default-org",
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ) -> None:
        super().__init__(
            EverMemOSConfig(
                base_url=base_url.rstrip("/"),
                org_id=org_id,
                user_id=user_id,
                group_id=group_id,
                api_key=api_key,
                timeout=timeout,
            )
        )
        self._client = httpx.AsyncClient(
            base_url=self.config.base_url,
            headers=self._headers(),
            timeout=self.config.timeout,
        )

    async def health(self) -> Dict[str, Any]:
        response = await self._client.get("/health")
        response.raise_for_status()
        return response.json()

    async def memorize(
        self,
        *,
        content: str,
        sender: str = "User",
        role: str = "user",
        sender_name: Optional[str] = None,
        message_id: Optional[str] = None,
        create_time: Optional[str] = None,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
        history: Optional[List[Dict[str, Any]]] = None,
    ) -> Dict[str, Any]:
        payload = self._memorize_payload(
            content=content,
            sender=sender,
            role=role,
            sender_name=sender_name,
            message_id=message_id,
            create_time=create_time,
            user_id=user_id,
            group_id=group_id,
            history=history,
        )
        response = await self._client.post("/api/v1/memories", json=payload)
        response.raise_for_status()
        return self._unwrap(response.json())

    async def add_conversation(
        self,
        *,
        user_message: str,
        assistant_message: str,
        user_name: str = "User",
        assistant_name: str = "Assistant",
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        user_result = await self.memorize(
            content=user_message,
            sender=user_name,
            role="user",
            sender_name=user_name,
            user_id=user_id,
            group_id=group_id,
        )
        assistant_result = await self.memorize(
            content=assistant_message,
            sender=assistant_name,
            role="assistant",
            sender_name=assistant_name,
            user_id=user_id,
            group_id=group_id,
            history=[{"role": "user", "content": user_message}],
        )
        return {
            "user": user_result,
            "assistant": assistant_result,
        }

    async def search(
        self,
        query: str,
        *,
        retrieve_method: str = "HYBRID",
        memory_types: Optional[List[str]] = None,
        top_k: int = 5,
        radius: Optional[float] = None,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        scope = self._effective_scope(user_id=user_id, group_id=group_id)
        params: Dict[str, Any] = {
            "query": query,
            "retrieve_method": retrieve_method,
            "top_k": top_k,
            "user_id": scope["user_id"],
            "group_id": scope["group_id"],
        }
        if memory_types:
            params["memory_types"] = ",".join(memory_types)
        if radius is not None:
            params["radius"] = radius
        response = await self._client.get("/api/v1/memories/search", params=params)
        response.raise_for_status()
        result = self._unwrap(response.json())
        return result.get("memories", [])

    async def fetch(
        self,
        *,
        memory_type: Optional[str] = None,
        limit: int = 20,
        offset: int = 0,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        scope = self._effective_scope(user_id=user_id, group_id=group_id)
        params: Dict[str, Any] = {
            "limit": limit,
            "offset": offset,
            "user_id": scope["user_id"],
            "group_id": scope["group_id"],
        }
        if memory_type:
            params["memory_type"] = memory_type
        response = await self._client.get("/api/v1/memories", params=params)
        response.raise_for_status()
        return self._unwrap(response.json())

    async def get_profile(
        self,
        memory_type: str = "profile",
        *,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
        limit: int = 20,
    ) -> List[Dict[str, Any]]:
        result = await self.fetch(
            memory_type=memory_type,
            user_id=user_id,
            group_id=group_id,
            limit=limit,
        )
        return result.get("memories", [])

    async def delete_memories(
        self,
        *,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
        memory_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        scope = self._effective_scope(user_id=user_id, group_id=group_id)
        payload: Dict[str, Any] = {
            "user_id": scope["user_id"],
            "group_id": scope["group_id"],
        }
        if memory_id:
            payload["memory_id"] = memory_id
        response = await self._client.request("DELETE", "/api/v1/memories", json=payload)
        response.raise_for_status()
        return self._unwrap(response.json())

    async def aclose(self) -> None:
        await self._client.aclose()


class EverMemOSSyncClient(_BaseClient):
    def __init__(
        self,
        *,
        base_url: str = "http://localhost:8080",
        org_id: str = "default-org",
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ) -> None:
        super().__init__(
            EverMemOSConfig(
                base_url=base_url.rstrip("/"),
                org_id=org_id,
                user_id=user_id,
                group_id=group_id,
                api_key=api_key,
                timeout=timeout,
            )
        )
        self._client = httpx.Client(
            base_url=self.config.base_url,
            headers=self._headers(),
            timeout=self.config.timeout,
        )

    def memorize(self, **kwargs: Any) -> Dict[str, Any]:
        payload = self._memorize_payload(
            content=kwargs["content"],
            sender=kwargs.get("sender", "User"),
            role=kwargs.get("role", "user"),
            sender_name=kwargs.get("sender_name"),
            message_id=kwargs.get("message_id"),
            create_time=kwargs.get("create_time"),
            user_id=kwargs.get("user_id"),
            group_id=kwargs.get("group_id"),
            history=kwargs.get("history"),
        )
        response = self._client.post("/api/v1/memories", json=payload)
        response.raise_for_status()
        return self._unwrap(response.json())

    def search(
        self,
        query: str,
        *,
        retrieve_method: str = "HYBRID",
        top_k: int = 5,
        memory_types: Optional[List[str]] = None,
        user_id: Optional[str] = None,
        group_id: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        scope = self._effective_scope(user_id=user_id, group_id=group_id)
        params: Dict[str, Any] = {
            "query": query,
            "retrieve_method": retrieve_method,
            "top_k": top_k,
            "user_id": scope["user_id"],
            "group_id": scope["group_id"],
        }
        if memory_types:
            params["memory_types"] = ",".join(memory_types)
        response = self._client.get("/api/v1/memories/search", params=params)
        response.raise_for_status()
        result = self._unwrap(response.json())
        return result.get("memories", [])

    def close(self) -> None:
        self._client.close()
