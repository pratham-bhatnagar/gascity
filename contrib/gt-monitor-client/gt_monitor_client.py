"""Lightweight Python SDK for the Gas City memory API.

ADK agents call this to retrieve and persist memories across sessions.

Usage:
    from gt_monitor_client import MemoryClient

    client = MemoryClient("http://127.0.0.1:8080")

    # Retrieve relevant memories before starting work
    memories = client.recall(q="auth patterns", scope="rig")

    # Store a new memory after completing work
    client.remember("Always validate tokens before caching", kind="pattern", confidence=0.9)
"""

from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Memory:
    """A single memory bead returned from the API."""

    id: str = ""
    title: str = ""
    description: str = ""
    status: str = ""
    kind: str = ""
    confidence: str = ""
    scope: str = ""
    decay_at: str = ""
    source_bead: str = ""
    source_event: str = ""
    last_accessed: str = ""
    access_count: str = ""
    labels: list[str] = field(default_factory=list)
    created_at: str = ""

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> Memory:
        return cls(
            id=d.get("id", ""),
            title=d.get("title", ""),
            description=d.get("description", ""),
            status=d.get("status", ""),
            kind=d.get("kind", ""),
            confidence=d.get("confidence", ""),
            scope=d.get("scope", ""),
            decay_at=d.get("decay_at", ""),
            source_bead=d.get("source_bead", ""),
            source_event=d.get("source_event", ""),
            last_accessed=d.get("last_accessed", ""),
            access_count=d.get("access_count", ""),
            labels=d.get("labels") or [],
            created_at=d.get("created_at", ""),
        )


class MemoryClientError(Exception):
    """Raised when the API returns an error."""

    def __init__(self, status: int, message: str):
        self.status = status
        self.message = message
        super().__init__(f"HTTP {status}: {message}")


class MemoryClient:
    """Client for the Gas City memory API (GET/POST /v0/memories).

    Uses only stdlib (urllib) so it has zero dependencies beyond Python 3.10+.
    """

    def __init__(self, base_url: str, *, rig: str = "", timeout: float = 10.0):
        self.base_url = base_url.rstrip("/")
        self.rig = rig
        self.timeout = timeout

    def recall(
        self,
        *,
        q: str = "",
        scope: str = "",
        kind: str = "",
        min_confidence: str = "",
        limit: int = 0,
    ) -> list[Memory]:
        """Retrieve memories matching the given filters.

        Args:
            q: Keyword search across title and description.
            scope: Filter by scope (agent, rig, town, global).
            kind: Filter by kind (pattern, decision, incident, skill, context, anti-pattern).
            min_confidence: Minimum confidence threshold (0.0-1.0).
            limit: Maximum number of results (0 = server default).
        """
        params: dict[str, str] = {}
        if q:
            params["q"] = q
        if scope:
            params["scope"] = scope
        if kind:
            params["kind"] = kind
        if min_confidence:
            params["min_confidence"] = min_confidence
        if limit > 0:
            params["limit"] = str(limit)
        if self.rig:
            params["rig"] = self.rig

        url = f"{self.base_url}/v0/memories"
        if params:
            url += "?" + urllib.parse.urlencode(params)

        data = self._get(url)
        items = data.get("items") or []
        return [Memory.from_dict(item) for item in items]

    def remember(
        self,
        title: str,
        *,
        description: str = "",
        kind: str = "",
        confidence: float | str = "",
        scope: str = "",
        decay_at: str = "",
        source_bead: str = "",
        source_event: str = "",
        labels: list[str] | None = None,
    ) -> Memory:
        """Create a new memory bead.

        Args:
            title: Memory title (required).
            description: Optional longer description.
            kind: Memory kind (defaults to "context" server-side).
            confidence: Confidence score 0.0-1.0 (defaults to 0.5 server-side).
            scope: Scope level (defaults to "rig" server-side).
            decay_at: RFC 3339 timestamp for staleness detection.
            source_bead: Originating bead ID.
            source_event: Originating event ID.
            labels: Optional labels.
        """
        body: dict[str, Any] = {"title": title}
        if self.rig:
            body["rig"] = self.rig
        if description:
            body["description"] = description
        if kind:
            body["kind"] = kind
        if confidence != "":
            body["confidence"] = str(confidence)
        if scope:
            body["scope"] = scope
        if decay_at:
            body["decay_at"] = decay_at
        if source_bead:
            body["source_bead"] = source_bead
        if source_event:
            body["source_event"] = source_event
        if labels:
            body["labels"] = labels

        data = self._post(f"{self.base_url}/v0/memories", body)
        return Memory.from_dict(data)

    def get(self, memory_id: str) -> Memory:
        """Fetch a single memory by ID."""
        data = self._get(
            f"{self.base_url}/v0/memory/{urllib.parse.quote(memory_id, safe='')}"
        )
        return Memory.from_dict(data)

    def _get(self, url: str) -> dict[str, Any]:
        req = urllib.request.Request(url, method="GET")
        return self._do(req)

    def _post(self, url: str, body: dict[str, Any]) -> dict[str, Any]:
        data = json.dumps(body).encode()
        req = urllib.request.Request(url, data=data, method="POST")
        req.add_header("Content-Type", "application/json")
        req.add_header("X-GC-Request", "true")
        return self._do(req)

    def _do(self, req: urllib.request.Request) -> dict[str, Any]:
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            body = e.read().decode(errors="replace")
            try:
                err = json.loads(body)
                msg = err.get("message") or err.get("error") or body
            except (json.JSONDecodeError, KeyError):
                msg = body
            raise MemoryClientError(e.code, msg) from e
        except urllib.error.URLError as e:
            raise MemoryClientError(0, f"connection error: {e.reason}") from e
