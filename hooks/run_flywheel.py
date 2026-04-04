#!/usr/bin/env python3
"""run_flywheel.py — Run the DI wasteland flywheel for a single rig.

Called by on_bead_close.sh after a `bd close` event.
Logs to BOTH hooks.log AND tool_calls.jsonl (dashboard visibility).

Usage:
    python3 run_flywheel.py <rig_name> [bead_id]
"""

import sys
import os
import json
import asyncio
import time
from datetime import datetime

DI_ROOT = os.path.expanduser("~/gt/deepwork_intelligence")
LOGFILE = os.path.join(DI_ROOT, "logs", "hooks.log")

sys.path.insert(0, DI_ROOT)
sys.path.insert(0, os.path.join(DI_ROOT, "agents"))


def log(msg: str):
    os.makedirs(os.path.dirname(LOGFILE), exist_ok=True)
    ts = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
    with open(LOGFILE, "a") as f:
        f.write(f"{ts} [flywheel] {msg}\n")


async def run(rig: str, bead_id: str = ""):
    from server import wasteland_flywheel, _log_tool_call

    log(f"Starting flywheel for rig={rig} bead={bead_id}")
    t0 = int(time.time() * 1000)

    try:
        result_json = await wasteland_flywheel(rig)
        latency = int(time.time() * 1000) - t0
        data = json.loads(result_json)

        steps = data.get("steps", [])
        maps = steps[0]["result"].get("total", 0) if len(steps) > 0 else 0
        completes = steps[1]["result"].get("completed", 0) if len(steps) > 1 else 0
        reviews = steps[2]["result"].get("reviewed", 0) if len(steps) > 2 else 0

        # Log to hooks.log
        log(f"Flywheel complete for rig={rig}: maps={maps} completes={completes} reviews={reviews} ({latency}ms)")

        # Log to tool_calls.jsonl (dashboard visibility)
        _log_tool_call(
            "wasteland_flywheel",
            f"hook/bead-close/{bead_id}" if bead_id else "hook/bead-close",
            {"rig": rig, "bead_id": bead_id, "trigger": "bd_close_hook"},
            result_json,
            latency,
        )

    except Exception as e:
        latency = int(time.time() * 1000) - t0
        log(f"Flywheel ERROR for rig={rig}: {e}")
        _log_tool_call(
            "wasteland_flywheel",
            f"hook/bead-close/{bead_id}" if bead_id else "hook/bead-close",
            {"rig": rig, "bead_id": bead_id, "trigger": "bd_close_hook"},
            "",
            latency,
            str(e)[:200],
        )
        sys.exit(1)


def main():
    if len(sys.argv) < 2:
        print("Usage: run_flywheel.py <rig_name> [bead_id]", file=sys.stderr)
        sys.exit(1)

    rig = sys.argv[1]
    bead_id = sys.argv[2] if len(sys.argv) > 2 else ""
    asyncio.run(run(rig, bead_id))


if __name__ == "__main__":
    main()
