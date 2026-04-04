"""Deepwork Intelligence Dashboard — API Server.

Lightweight HTTP server serving the dashboard and analytics endpoints.
No dependencies beyond stdlib + pymysql (for Dolt/wasteland queries).

Usage:
    python api.py [--port 8090]
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from collections import defaultdict
from datetime import datetime, timezone, timedelta
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from statistics import mean

DASHBOARD_DIR = Path(__file__).parent
LOGS_DIR = DASHBOARD_DIR.parent / "logs"
TOOL_CALLS_FILE = LOGS_DIR / "tool_calls.jsonl"

# Wasteland Dolt config (optional — graceful if unavailable)
DOLT_HOST = os.environ.get("DI_DOLT_HOST", "127.0.0.1")
DOLT_PORT = int(os.environ.get("DI_DOLT_PORT", "3307"))
DOLT_USER = os.environ.get("DI_DOLT_USER", "root")
DOLT_PASS = os.environ.get("DI_DOLT_PASSWORD", "")
WASTELAND_DB = os.environ.get("DI_WASTELAND_DB", "wl_commons")


def _read_jsonl() -> list[dict]:
    """Read all entries from tool_calls.jsonl."""
    entries = []
    if not TOOL_CALLS_FILE.exists():
        return entries
    try:
        with open(TOOL_CALLS_FILE, "r") as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        entries.append(json.loads(line))
                    except json.JSONDecodeError:
                        continue
    except OSError:
        pass
    return entries


def _compute_analytics() -> dict:
    """Compute analytics from JSONL entries."""
    entries = _read_jsonl()

    if not entries:
        return {
            "total_calls": 0,
            "calls_24h": 0,
            "error_count": 0,
            "avg_latency_ms": 0,
            "p95_latency_ms": 0,
            "active_tools": 0,
            "active_callers": 0,
            "by_tool": {},
            "by_caller": {},
            "timeline": {},
            "recent_calls": [],
            "recent_errors": [],
        }

    now = datetime.now(timezone.utc)
    cutoff_24h = now - timedelta(hours=24)

    total = len(entries)
    errors = [e for e in entries if e.get("error")]
    error_count = len(errors)

    # Calls in last 24h
    calls_24h = 0
    for e in entries:
        try:
            ts = datetime.fromisoformat(e["timestamp"])
            if ts > cutoff_24h:
                calls_24h += 1
        except (KeyError, ValueError):
            pass

    # Latency stats
    latencies = [e["latency_ms"] for e in entries if isinstance(e.get("latency_ms"), (int, float))]
    avg_lat = mean(latencies) if latencies else 0
    p95_lat = 0
    if latencies:
        sorted_lat = sorted(latencies)
        idx = int(len(sorted_lat) * 0.95)
        p95_lat = sorted_lat[min(idx, len(sorted_lat) - 1)]

    # By tool
    by_tool: dict[str, int] = defaultdict(int)
    for e in entries:
        by_tool[e.get("tool", "unknown")] += 1

    # By caller
    by_caller: dict[str, int] = defaultdict(int)
    for e in entries:
        by_caller[e.get("caller", "unknown")] += 1

    # Timeline: group by date + tool
    timeline: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for e in entries:
        try:
            date_str = e["timestamp"][:10]  # YYYY-MM-DD
            tool = e.get("tool", "unknown")
            timeline[date_str][tool] += 1
        except (KeyError, TypeError):
            pass

    # Recent calls (last 20, newest first)
    recent = list(reversed(entries[-20:]))

    # Recent errors (last 10, newest first)
    recent_errors = list(reversed(errors[-10:]))

    return {
        "total_calls": total,
        "calls_24h": calls_24h,
        "error_count": error_count,
        "avg_latency_ms": round(avg_lat, 1),
        "p95_latency_ms": round(p95_lat, 1),
        "active_tools": len(by_tool),
        "active_callers": len(by_caller),
        "by_tool": dict(by_tool),
        "by_caller": dict(by_caller),
        "timeline": {k: dict(v) for k, v in sorted(timeline.items())},
        "recent_calls": recent,
        "recent_errors": recent_errors,
    }


def _query_wasteland() -> dict:
    """Query wl_commons from Dolt for wasteland board status."""
    try:
        import pymysql
    except ImportError:
        return {"error": "pymysql not installed", "items": []}

    try:
        conn = pymysql.connect(
            host=DOLT_HOST,
            port=DOLT_PORT,
            user=DOLT_USER,
            password=DOLT_PASS,
            database=WASTELAND_DB,
            connect_timeout=5,
            read_timeout=10,
            cursorclass=pymysql.cursors.DictCursor,
        )
        with conn:
            with conn.cursor() as cur:
                # Get recent wasteland items
                cur.execute("""
                    SELECT id, project, title, status, priority, quality_score,
                           relevance_score, completeness_score, submitted_at, completed_at
                    FROM wl_items
                    ORDER BY submitted_at DESC
                    LIMIT 50
                """)
                items = cur.fetchall()

                # Convert datetime objects to strings
                for item in items:
                    for k, v in item.items():
                        if isinstance(v, datetime):
                            item[k] = v.isoformat()

                # Summary stats
                cur.execute("""
                    SELECT status, COUNT(*) as cnt
                    FROM wl_items
                    GROUP BY status
                """)
                status_counts = {row["status"]: row["cnt"] for row in cur.fetchall()}

                cur.execute("""
                    SELECT project, COUNT(*) as cnt
                    FROM wl_items
                    GROUP BY project
                    ORDER BY cnt DESC
                """)
                project_counts = {row["project"]: row["cnt"] for row in cur.fetchall()}

        return {
            "items": items,
            "by_status": status_counts,
            "by_project": project_counts,
            "total": sum(status_counts.values()),
        }
    except Exception as e:
        return {"error": str(e), "items": []}


class DashboardHandler(SimpleHTTPRequestHandler):
    """HTTP handler for dashboard and API endpoints."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(DASHBOARD_DIR), **kwargs)

    def do_GET(self):
        if self.path == "/api/analytics":
            self._json_response(_compute_analytics())
        elif self.path == "/api/wasteland":
            self._json_response(_query_wasteland())
        elif self.path == "/" or self.path == "/index.html":
            self.path = "/index.html"
            super().do_GET()
        else:
            super().do_GET()

    def _json_response(self, data: dict, status: int = 200):
        body = json.dumps(data, default=str).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        # Compact logging
        ts = time.strftime("%H:%M:%S")
        sys.stderr.write(f"[{ts}] {args[0]}\n")


def main():
    parser = argparse.ArgumentParser(description="Deepwork Intelligence Dashboard")
    parser.add_argument("--port", type=int, default=int(os.environ.get("DI_DASHBOARD_PORT", "8090")))
    parser.add_argument("--bind", default="0.0.0.0")
    args = parser.parse_args()

    server = HTTPServer((args.bind, args.port), DashboardHandler)
    print(f"[DI Dashboard] Serving on http://{args.bind}:{args.port}")
    print(f"[DI Dashboard] JSONL source: {TOOL_CALLS_FILE}")
    print(f"[DI Dashboard] Wasteland DB: {WASTELAND_DB}@{DOLT_HOST}:{DOLT_PORT}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[DI Dashboard] Stopped.")
        server.server_close()


if __name__ == "__main__":
    main()
