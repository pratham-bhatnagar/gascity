#!/bin/bash
# on_bead_close.sh — PostToolUse hook for Claude Code
#
# Fires after `bd close` and triggers the Deepwork Intelligence wasteland
# flywheel for the affected rig. Claude Code pipes the tool call JSON to stdin.
#
# Must complete in <5s — runs the flywheel in the background.

set -uo pipefail

GT_ROOT="${GT_ROOT:-$HOME/gt}"
DI_ROOT="$GT_ROOT/deepwork_intelligence"
LOGFILE="$DI_ROOT/logs/hooks.log"
mkdir -p "$(dirname "$LOGFILE")"

log() { echo "$(date +%Y-%m-%dT%H:%M:%S) [hook:bead-close] $*" >> "$LOGFILE"; }

# Read the tool call JSON from stdin
INPUT=$(cat)

# Extract the command that was run
COMMAND=$(echo "$INPUT" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    # Claude Code PostToolUse passes tool_input with 'command' for Bash calls
    cmd = data.get('tool_input', {}).get('command', '')
    print(cmd)
except:
    print('')
" 2>/dev/null)

if [ -z "$COMMAND" ]; then
    log "SKIP — could not extract command from input"
    exit 0
fi

# Verify this is actually a bd close command
if [[ "$COMMAND" != *"bd close"* ]]; then
    log "SKIP — not a bd close command: $COMMAND"
    exit 0
fi

log "Detected bd close: $COMMAND"

# Extract bead ID — bd close <bead-id> [flags]
BEAD_ID=$(echo "$COMMAND" | sed -n 's/.*bd close[[:space:]]\+\([^ ]*\).*/\1/p')

if [ -z "$BEAD_ID" ]; then
    log "WARN — could not extract bead ID from: $COMMAND"
    exit 0
fi

log "Bead ID: $BEAD_ID"

# Extract rig from the bead prefix (e.g., vap-123 -> villa_ai_planogram)
# The bead prefix maps to a rig name
RIG=$(python3 -c "
import sys, os
sys.path.insert(0, os.path.expanduser('~/gt/deepwork_intelligence'))
try:
    from config import load_config
    cfg = load_config()
    prefix = '$BEAD_ID'.split('-')[0] if '-' in '$BEAD_ID' else '$BEAD_ID'
    # Check rig prefixes from config
    for rig in cfg.get('rigs', {}):
        rig_cfg = cfg['rigs'][rig]
        if rig_cfg.get('prefix', '') == prefix:
            print(rig)
            sys.exit(0)
    # Fallback: try common prefix mappings
    mapping = {
        'vap': 'villa_ai_planogram',
        'vaa': 'villa_alc_ai',
        'of': 'officeworld',
        'ds': 'deepwork_site',
        'cc': 'command_center',
        'gta': 'officeworld',
        'gtm': 'gtm',
    }
    print(mapping.get(prefix, ''))
except Exception as e:
    # Hardcoded fallback
    mapping = {
        'vap': 'villa_ai_planogram',
        'vaa': 'villa_alc_ai',
        'of': 'officeworld',
        'ds': 'deepwork_site',
        'cc': 'command_center',
        'gta': 'officeworld',
        'gtm': 'gtm',
    }
    prefix = '$BEAD_ID'.split('-')[0] if '-' in '$BEAD_ID' else '$BEAD_ID'
    print(mapping.get(prefix, ''))
" 2>/dev/null)

if [ -z "$RIG" ]; then
    log "WARN — could not resolve rig for bead $BEAD_ID, skipping flywheel"
    exit 0
fi

log "Rig resolved: $RIG — launching flywheel in background"

# Run the flywheel in the background so we don't block the tool call
nohup python3 "$DI_ROOT/hooks/run_flywheel.py" "$RIG" "$BEAD_ID" >> "$LOGFILE" 2>&1 &

log "Flywheel launched (pid=$!) for rig=$RIG bead=$BEAD_ID"
exit 0
