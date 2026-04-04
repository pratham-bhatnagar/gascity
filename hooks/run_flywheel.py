#!/usr/bin/env python3
"""run_flywheel.py — Run the DI wasteland flywheel for a single rig.

Called by on_bead_close.sh after a `bd close` event.
Imports wasteland_flywheel from server.py and runs it.

Usage:
    python3 run_flywheel.py <rig_name>
"""

import sys
import os
import json
import asyncio
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


async def run(rig: str):
    from server import wasteland_flywheel

    log(f"Starting flywheel for rig={rig}")
    try:
        result_json = await wasteland_flywheel(rig)
        data = json.loads(result_json)

        steps = data.get("steps", [])
        maps = steps[0]["result"].get("total", 0) if len(steps) > 0 else 0
        completes = steps[1]["result"].get("completed", 0) if len(steps) > 1 else 0
        reviews = steps[2]["result"].get("reviewed", 0) if len(steps) > 2 else 0

        log(f"Flywheel complete for rig={rig}: maps={maps} completes={completes} reviews={reviews}")
    except Exception as e:
        log(f"Flywheel ERROR for rig={rig}: {e}")
        sys.exit(1)


def main():
    if len(sys.argv) < 2:
        print("Usage: run_flywheel.py <rig_name>", file=sys.stderr)
        sys.exit(1)

    rig = sys.argv[1]
    asyncio.run(run(rig))


if __name__ == "__main__":
    main()
