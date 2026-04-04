# DI Wasteland Flywheel — Claude Code Hook

PostToolUse hook that triggers the Deepwork Intelligence wasteland flywheel
whenever a bead is closed via `bd close`.

## How it works

1. Claude Code fires `on_bead_close.sh` after any `bd close` Bash call
2. The script extracts the bead ID and resolves the rig from the bead prefix
3. It launches `run_flywheel.py` in the background (non-blocking)
4. The flywheel runs: map beads -> complete matched -> review/stamp
5. Results are logged to `logs/hooks.log`

## Installation

Add this to your Claude Code `settings.json` (either project-level at
`.claude/settings.json` or user-level at `~/.claude/settings.json`):

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash(bd close*)",
        "hooks": [
          {
            "type": "command",
            "command": "bash ~/gt/deepwork_intelligence/hooks/on_bead_close.sh"
          }
        ]
      }
    ]
  }
}
```

## Files

- `on_bead_close.sh` — Entry point. Parses tool call JSON, extracts bead/rig, launches flywheel.
- `run_flywheel.py` — Runs `wasteland_flywheel(rig)` from the DI server module.
- `../logs/hooks.log` — All hook activity is logged here.

## Debugging

```bash
# Check recent hook activity
tail -50 ~/gt/deepwork_intelligence/logs/hooks.log

# Test the flywheel manually for a rig
python3 ~/gt/deepwork_intelligence/hooks/run_flywheel.py villa_ai_planogram

# Test the hook with simulated input
echo '{"tool_input":{"command":"bd close vap-42"}}' | bash ~/gt/deepwork_intelligence/hooks/on_bead_close.sh
```
