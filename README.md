# Deepwork Intelligence

> Deterministic AI intelligence layer for [Gas Town](https://github.com/steveyegge/gastown). Small, specialized agents powered by local LLMs for structured, reliable automation.

## What It Does

Deepwork Intelligence (DI) adds a smart deterministic layer on top of Gas Town's multi-agent orchestration. Instead of using expensive LLM agents for repetitive tasks (reviewing, matching, reporting), DI uses small specialized agents running on a local LLM (via vLLM) that produce structured JSON output every time.

**The pattern:** `Cron trigger → Python agent → Local LLM (structured output) → Pydantic validation → Action`

## Features

### Wasteland Flywheel
Autonomous reputation cycle for the [Wasteland federation](https://github.com/steveyegge/gastown/blob/main/docs/wasteland.md):

- **`wasteland_map_beads`** — Semantically match internal beads to wasteland items using LLM understanding (not keyword matching)
- **`wasteland_complete_matched`** — Auto-submit completions when all mapped beads are closed
- **`wasteland_stamp`** — Score completions with Q/R/C (quality, reliability, creativity) using intelligent evaluation
- **`wasteland_flywheel`** — Run the full cycle: map → complete → stamp

### Documentation Agents
Generate and maintain rig documentation:

- **`docs_create`** — Generate a new doc from context (architecture, API reference, runbook, etc.)
- **`docs_append`** — Add a section to existing docs, matching the existing style
- **`docs_update`** — Rewrite a specific section with new information
- **`docs_generate`** — Auto-generate by reading rig state (beads, git log, code structure)
- **`docs_index`** — List all docs for a rig

### Feedback & Learning
Self-improving scoring through human feedback:

- **`feedback_submit`** — Correct a stamp's scores (what quality SHOULD have been)
- **`feedback_summary`** — Analyze patterns in scoring errors
- **`feedback_apply`** — Generate calibration notes that adjust future scoring

### System Health
- **`health`** — Check Dolt and vLLM status

## Architecture

```
┌─────────────────────────────────────┐
│         Claude Code Agents          │
│   (Mayor, Witness, Crew, Polecats)  │
│         call MCP tools              │
└──────────────┬──────────────────────┘
               │ MCP (stdio)
               ▼
┌──────────────────────────────────────┐
│      Deepwork Intelligence Server    │
│         (FastMCP, Python)            │
│                                      │
│  ┌──────────┐  ┌──────────┐         │
│  │ Wasteland│  │   Docs   │         │
│  │  Agents  │  │  Agents  │         │
│  └────┬─────┘  └────┬─────┘         │
│       │              │               │
│  ┌────┴──────────────┴────┐          │
│  │    Shared Layer        │          │
│  │  LLM Client │ Dolt DB │          │
│  └────┬──────────────┬────┘          │
└───────┼──────────────┼───────────────┘
        │              │
   ┌────┴────┐    ┌────┴────┐
   │  vLLM   │    │  Dolt   │
   │ (local) │    │ (3307)  │
   └─────────┘    └─────────┘
```

## Quick Start

### Prerequisites

- [Gas Town](https://github.com/steveyegge/gastown) workspace
- Python 3.12+
- vLLM serving any OpenAI-compatible model (tested with MiniMax M2.5)
- Dolt SQL server (Gas Town manages this)
- Python packages: `mcp`, `openai`, `pymysql`, `pydantic`, `jinja2`

### Installation

```bash
# Clone into your Gas Town workspace
cd ~/gt
git clone <your-repo-url> deepwork_intelligence

# Install dependencies
pip install mcp openai pymysql pydantic jinja2

# Register as Gas Town rig
gt rig add deepwork_intelligence --adopt --prefix di --force
```

### Configuration

Create `.mcp.json` in your Gas Town root:

```json
{
  "mcpServers": {
    "deepwork-intelligence": {
      "command": "python3",
      "args": ["<path-to>/deepwork_intelligence/server.py"],
      "env": {
        "GT_ROOT": "<path-to-gas-town>"
      }
    }
  }
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GT_ROOT` | `~/gt` | Gas Town workspace root |
| `VLLM_BASE_URL` | `http://localhost:8080/v1` | vLLM OpenAI-compatible endpoint |
| `VLLM_MODEL` | `MiniMaxAI/MiniMax-M2.5` | Model name on vLLM |
| `DOLT_HOST` | `127.0.0.1` | Dolt server host |
| `DOLT_PORT` | `3307` | Dolt server port |
| `WASTELAND_DB` | `wl_commons` | Wasteland database name in Dolt |

### Customize for Your Setup

Edit `agents/shared/llm.py` to point to your LLM:

```python
VLLM_BASE_URL = "http://localhost:8080/v1"  # Your vLLM endpoint
VLLM_MODEL = "your-model-name"              # Your model
```

Edit `server.py` to configure your rig→database mapping:

```python
PROJECT_TO_DB = {
    "my-project": "my_rig_db",
    # Add your project→database mappings
}
```

### Cron Setup

Add the wasteland flywheel to your crontab:

```bash
# Run wasteland flywheel every 20 minutes
*/20 * * * * /path/to/deepwork_intelligence/crons/flywheel-test.sh >> /path/to/logs/di-flywheel.log 2>&1
```

## Usage

Once configured, any Claude Code agent in your workspace can use DI tools:

```
"Run the wasteland flywheel for my-rig"
→ Claude calls wasteland_flywheel("my_rig")
→ Maps beads, completes matched items, stamps reviews

"Generate architecture docs for my-rig"
→ Claude calls docs_generate("my_rig", "architecture")
→ Reads code, beads, git log → generates doc → commits to git

"That stamp was too harsh, quality should be 4"
→ Claude calls feedback_submit("s-abc123", 4, 4, 3, "Branch was merged with tests")
→ Feedback saved, future stamps calibrated
```

## Feedback Loop

DI learns from corrections:

```
1. DI stamps a completion: Q:2 R:3 C:2
2. Human reviews: "That should be Q:4 — the branch was merged with full test suite"
3. feedback_submit(stamp_id, actual_quality=4, ...)
4. feedback_apply() → generates calibration notes
5. Next stamp reads calibration → adjusts scoring behavior
```

The calibration file (`feedback/calibration.md`) is human-readable and editable.

## Project Structure

```
deepwork_intelligence/
├── server.py                  # FastMCP server — all tools registered here
├── agents/
│   ├── wasteland/
│   │   ├── stamp.py           # Score completions (reads calibration)
│   │   ├── map_beads.py       # Semantic bead→wasteland matching
│   │   └── sync.py            # Beads → wasteland items (future)
│   ├── content/               # Release notes, changelog (future)
│   ├── reports/               # Overseer summary, board report (future)
│   └── shared/
│       ├── llm.py             # LLM client (vLLM OpenAI-compatible)
│       ├── dolt.py            # Dolt database helper
│       └── schemas.py         # Pydantic output models
├── templates/                 # Jinja2 templates for reports (future)
├── feedback/                  # Feedback loop data + calibration
│   ├── stamp_feedback.jsonl   # Raw feedback entries
│   └── calibration.md         # Generated calibration notes
├── crons/
│   └── flywheel-test.sh       # Cron wrapper for wasteland flywheel
└── README.md
```

## Supported LLMs

Tested with:
- **MiniMax M2.5** via vLLM (recommended — good structured output, fast)

Should work with any OpenAI-compatible endpoint:
- Any vLLM-served model
- Ollama (with OpenAI compatibility)
- Together AI, Fireworks, etc.

**Note:** Some models wrap output in `<think>` tags. DI handles this automatically via `_strip_think_tags()`.

## Future Scope

- **Overseer daily reports** — automated summary of Gas Town activity
- **Wasteland board reports** — contributor leaderboards
- **Release notes** — auto-generate from git log
- **Changelog entries** — structured event logging
- **Org config pack updates** — sync knowledge to shared pack
- **gt-monitor integration** — analytics reports on top of monitoring data

## License

MIT — same as Gas Town.
