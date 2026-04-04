#!/bin/bash
# flywheel-test.sh — Run the full wasteland flywheel via DI MCP
#
# Tests: map beads → complete matched → stamp → push
# For each active rig with beads.

set -uo pipefail

export PATH="$HOME/go/bin:$HOME/.local/bin:$PATH"
GT_ROOT="${GT_ROOT:-$HOME/gt}"
LOGFILE="$GT_ROOT/logs/di-flywheel.log"
LOCKFILE="/tmp/di-flywheel.lock"

mkdir -p "$(dirname "$LOGFILE")"
log() { echo "$(date +%Y-%m-%dT%H:%M:%S) [di-flywheel] $*" >> "$LOGFILE"; }

exec 200>"$LOCKFILE"
flock -n 200 || { log "SKIP — another run"; exit 0; }

log "=== Starting DI flywheel cycle ==="

# Run the flywheel for each active rig
RESULT=$(python3 << 'PYEOF'
import sys, asyncio, json, os
sys.path.insert(0, os.path.expanduser("~/gt/deepwork_intelligence"))
sys.path.insert(0, os.path.expanduser("~/gt/deepwork_intelligence/agents"))

from server import wasteland_flywheel, wasteland_review_all, wasteland_status, _log_tool_call
import time

async def run():
    results = {}

    # Run flywheel for ALL rigs that have beads
    for rig in ["villa_ai_planogram", "villa_alc_ai", "officeworld", "deepwork_site",
                "command_center", "products", "media_studio", "content_studio", "gtm"]:
        try:
            t0 = int(time.time() * 1000)
            r = await wasteland_flywheel(rig)
            latency = int(time.time() * 1000) - t0
            data = json.loads(r)
            maps = data["steps"][0]["result"].get("total", 0) if data["steps"] else 0
            completes = data["steps"][1]["result"].get("completed", 0) if len(data["steps"]) > 1 else 0
            reviews = data["steps"][2]["result"].get("reviewed", 0) if len(data["steps"]) > 2 else 0
            results[rig] = {"maps": maps, "completes": completes, "reviews": reviews}
            _log_tool_call("wasteland_flywheel", "cron/flywheel", {"rig": rig}, r, latency)
            print(f"{rig}: maps={maps} completes={completes} reviews={reviews}", file=sys.stderr)
        except Exception as e:
            results[rig] = {"error": str(e)[:100]}
            _log_tool_call("wasteland_flywheel", "cron/flywheel", {"rig": rig}, "", 0, str(e)[:200])
            print(f"{rig}: ERROR {e}", file=sys.stderr)

    # Also run standalone review for anything missed
    try:
        review = await wasteland_review_all()
        review_data = json.loads(review)
        results["review_pass"] = review_data.get("reviewed", 0)
    except Exception as e:
        results["review_pass"] = f"error: {e}"

    # Get status
    try:
        status = await wasteland_status()
        status_data = json.loads(status)
        results["status"] = status_data.get("items_by_status", {})
        results["stamps"] = status_data.get("total_stamps", 0)
    except Exception as e:
        results["status"] = f"error: {e}"

    print(json.dumps(results))

asyncio.run(run())
PYEOF
)

log "$RESULT"

# Push wasteland to DoltHub
bash "$GT_ROOT/mayor/scripts/wasteland-push.sh" 2>/dev/null

log "=== DI flywheel cycle complete ==="
