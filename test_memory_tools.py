"""Functional tests for memory MCP tools against live Dolt.

Run: python3 test_memory_tools.py
Requires: Dolt server running on port 3307 with 'gc' database.
"""
from __future__ import annotations

import asyncio
import json
import sys
import os

# Add server directory to path
sys.path.insert(0, os.path.dirname(__file__))

# Minimal config override to avoid needing full config.yaml
os.environ.setdefault("DI_LLM_BASE_URL", "http://localhost:8080/v1")
os.environ.setdefault("DI_LLM_MODEL", "test")
os.environ.setdefault("DI_LLM_API_KEY", "test")

from server import memory_remember, memory_recall, memory_forget, dolt

MEMORY_DB = "gc"
_created_ids: list[str] = []


async def test_remember():
    """Create a memory bead and verify it exists."""
    result = json.loads(await memory_remember(
        content="Test memory: always run tests before pushing",
        kind="pattern",
        scope="rig",
        confidence=0.9,
        decay_days=7,
        source_bead="gc-test",
    ))
    assert "error" not in result, f"remember failed: {result}"
    assert result["kind"] == "pattern"
    assert result["scope"] == "rig"
    assert result["confidence"] == 0.9
    assert result["id"].startswith("mem-")
    assert result["decay_at"] is not None
    _created_ids.append(result["id"])
    print(f"  PASS: test_remember (id={result['id']})")
    return result["id"]


async def test_remember_validation():
    """Verify validation rejects bad inputs."""
    result = json.loads(await memory_remember(
        content="bad kind", kind="invalid"))
    assert "error" in result
    assert "Invalid kind" in result["error"]

    result = json.loads(await memory_remember(
        content="bad scope", scope="invalid"))
    assert "error" in result
    assert "Invalid scope" in result["error"]

    result = json.loads(await memory_remember(
        content="bad confidence", confidence=1.5))
    assert "error" in result
    assert "Confidence" in result["error"]
    print("  PASS: test_remember_validation")


async def test_recall_all():
    """Recall all memories (should include the one we just created)."""
    result = json.loads(await memory_recall(limit=50))
    assert "error" not in result, f"recall failed: {result}"
    assert result["count"] >= 1
    ids = [m["id"] for m in result["memories"]]
    assert _created_ids[0] in ids, f"expected {_created_ids[0]} in results"
    print(f"  PASS: test_recall_all (count={result['count']})")


async def test_recall_by_keyword():
    """Recall by keyword search."""
    result = json.loads(await memory_recall(query="always run tests"))
    assert "error" not in result
    assert result["count"] >= 1
    assert any("tests" in m["title"].lower() for m in result["memories"])
    print(f"  PASS: test_recall_by_keyword (count={result['count']})")


async def test_recall_by_scope():
    """Recall filtered by scope."""
    result = json.loads(await memory_recall(scope="rig"))
    assert "error" not in result
    for m in result["memories"]:
        assert m["scope"] == "rig", f"expected scope=rig, got {m['scope']}"
    print(f"  PASS: test_recall_by_scope (count={result['count']})")


async def test_recall_by_confidence():
    """Recall filtered by minimum confidence."""
    result = json.loads(await memory_recall(min_confidence=0.85))
    assert "error" not in result
    for m in result["memories"]:
        assert m["confidence"] >= 0.85, f"expected confidence>=0.85, got {m['confidence']}"
    print(f"  PASS: test_recall_by_confidence (count={result['count']})")


async def test_recall_bumps_access_count():
    """Verify recall increments the access counter."""
    mem_id = _created_ids[0]

    # First recall
    result = json.loads(await memory_recall(query="always run tests"))
    mem = next((m for m in result["memories"] if m["id"] == mem_id), None)
    assert mem is not None
    count1 = mem["access_count"]

    # Second recall
    result = json.loads(await memory_recall(query="always run tests"))
    mem = next((m for m in result["memories"] if m["id"] == mem_id), None)
    assert mem is not None
    assert mem["access_count"] > count1, "access_count should increment"
    print(f"  PASS: test_recall_bumps_access_count ({count1} -> {mem['access_count']})")


async def test_forget_by_id():
    """Forget a specific memory by ID."""
    # Create a throwaway memory
    result = json.loads(await memory_remember(
        content="Throwaway memory for forget test",
        kind="context",
        scope="agent",
    ))
    throwaway_id = result["id"]
    _created_ids.append(throwaway_id)

    # Forget it
    result = json.loads(await memory_forget(memory_id=throwaway_id))
    assert "error" not in result, f"forget failed: {result}"
    assert throwaway_id in result["archived"]
    assert result["count"] == 1

    # Verify it's gone from recall
    result = json.loads(await memory_recall(query="Throwaway memory for forget test"))
    ids = [m["id"] for m in result["memories"]]
    assert throwaway_id not in ids, "forgotten memory should not appear in recall"
    print(f"  PASS: test_forget_by_id ({throwaway_id})")


async def test_forget_validation():
    """Verify forget requires an argument."""
    result = json.loads(await memory_forget())
    assert "error" in result
    print("  PASS: test_forget_validation")


async def cleanup():
    """Clean up test memories."""
    for mem_id in _created_ids:
        try:
            dolt.execute(MEMORY_DB,
                "DELETE FROM issues WHERE id = %s AND issue_type = 'memory'",
                (mem_id,))
        except Exception:
            pass
    if _created_ids:
        try:
            dolt.commit_and_push(MEMORY_DB, "test: clean up memory test beads")
        except Exception:
            pass
    print(f"  Cleaned up {len(_created_ids)} test memories")


async def main():
    print("Memory MCP tools — functional tests")
    print("=" * 50)
    try:
        await test_remember()
        await test_remember_validation()
        await test_recall_all()
        await test_recall_by_keyword()
        await test_recall_by_scope()
        await test_recall_by_confidence()
        await test_recall_bumps_access_count()
        await test_forget_by_id()
        await test_forget_validation()
        print("=" * 50)
        print("ALL TESTS PASSED")
    finally:
        await cleanup()


if __name__ == "__main__":
    asyncio.run(main())
