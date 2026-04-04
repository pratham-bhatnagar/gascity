"""Tests for the gt_monitor_client Python SDK.

Runs against a fake HTTP server to verify request construction and
response parsing without needing a real Gas City instance.
"""

from __future__ import annotations

import json
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from threading import Thread
from urllib.parse import parse_qs, urlparse

from gt_monitor_client import Memory, MemoryClient, MemoryClientError


class FakeHandler(BaseHTTPRequestHandler):
    """Minimal fake for the /v0/memories endpoints."""

    memories: list[dict] = []

    def log_message(self, format, *args):  # noqa: A002
        pass  # suppress test output

    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path == "/v0/memories":
            params = parse_qs(parsed.query)
            items = list(self.memories)

            # Apply keyword filter.
            if "q" in params:
                kw = params["q"][0].lower()
                items = [
                    m
                    for m in items
                    if kw in m.get("title", "").lower()
                    or kw in m.get("description", "").lower()
                ]

            # Apply scope filter.
            if "scope" in params:
                scope = params["scope"][0]
                items = [m for m in items if m.get("scope") == scope]

            resp = json.dumps({"items": items, "total": len(items)}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(resp)
        elif parsed.path.startswith("/v0/memory/"):
            mid = parsed.path.split("/")[-1]
            for m in self.memories:
                if m.get("id") == mid:
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.end_headers()
                    self.wfile.write(json.dumps(m).encode())
                    return
            self.send_response(404)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": "not_found"}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        parsed = urlparse(self.path)
        if parsed.path == "/v0/memories":
            length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(length)) if length else {}

            memory = {
                "id": f"gc-{len(self.memories) + 1}",
                "title": body.get("title", ""),
                "description": body.get("description", ""),
                "kind": body.get("kind", "context"),
                "confidence": body.get("confidence", "0.5"),
                "scope": body.get("scope", "rig"),
                "status": "open",
            }
            self.memories.append(memory)

            self.send_response(201)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(memory).encode())
        else:
            self.send_response(404)
            self.end_headers()


class TestMemoryClient(unittest.TestCase):
    server: HTTPServer
    thread: Thread

    @classmethod
    def setUpClass(cls):
        FakeHandler.memories = []
        cls.server = HTTPServer(("127.0.0.1", 0), FakeHandler)
        cls.thread = Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()

    def setUp(self):
        FakeHandler.memories.clear()
        host, port = self.server.server_address
        self.client = MemoryClient(f"http://{host}:{port}")

    def test_remember_creates_memory(self):
        mem = self.client.remember("Always run tests", kind="pattern", confidence=0.9)
        self.assertEqual(mem.title, "Always run tests")
        self.assertEqual(mem.kind, "pattern")
        self.assertNotEqual(mem.id, "")

    def test_recall_returns_memories(self):
        self.client.remember("First memory")
        self.client.remember("Second memory")
        memories = self.client.recall()
        self.assertEqual(len(memories), 2)

    def test_recall_keyword_search(self):
        self.client.remember("TDD is important")
        self.client.remember("Auth tokens expire")
        memories = self.client.recall(q="TDD")
        self.assertEqual(len(memories), 1)
        self.assertIn("TDD", memories[0].title)

    def test_recall_scope_filter(self):
        self.client.remember("Rig memory", scope="rig")
        self.client.remember("Town memory", scope="town")
        memories = self.client.recall(scope="rig")
        self.assertEqual(len(memories), 1)
        self.assertEqual(memories[0].scope, "rig")

    def test_get_memory(self):
        created = self.client.remember("Specific memory")
        fetched = self.client.get(created.id)
        self.assertEqual(fetched.title, "Specific memory")

    def test_get_not_found(self):
        with self.assertRaises(MemoryClientError) as ctx:
            self.client.get("gc-nonexistent")
        self.assertEqual(ctx.exception.status, 404)

    def test_memory_from_dict(self):
        d = {
            "id": "gc-1",
            "title": "Test",
            "kind": "pattern",
            "confidence": "0.8",
            "scope": "rig",
            "labels": ["test"],
        }
        mem = Memory.from_dict(d)
        self.assertEqual(mem.id, "gc-1")
        self.assertEqual(mem.kind, "pattern")
        self.assertEqual(mem.labels, ["test"])

    def test_remember_with_defaults(self):
        mem = self.client.remember("Minimal memory")
        self.assertEqual(mem.confidence, "0.5")
        self.assertEqual(mem.scope, "rig")
        self.assertEqual(mem.kind, "context")

    def test_client_with_rig(self):
        host, port = self.server.server_address
        client = MemoryClient(f"http://{host}:{port}", rig="myrig")
        client.remember("Rig-scoped memory")
        memories = client.recall()
        self.assertEqual(len(memories), 1)


if __name__ == "__main__":
    unittest.main()
