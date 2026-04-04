"""Map beads agent — semantic matching of beads to wasteland items."""
from __future__ import annotations

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from shared.llm import generate_structured
from shared.schemas import MapResult

SYSTEM_PROMPT = """You are a work tracker for Deepwork Labs.

Given a list of internal tasks (beads) and external work packages (wasteland items), determine which beads contribute to which wasteland items.

KEY RULES:
- A wasteland item is a BIG task (work package). It maps to MULTIPLE beads (small tasks).
- Match by SEMANTIC similarity, not exact string match.
- A bead like "WhatsApp webhook endpoint" maps to wasteland item "WhatsApp Gateway for ADK Agent".
- A bead like "Fix CORS" maps to wasteland item "Security: Fix CORS — restrict allowed origins".
- Some beads are internal (HANDOFF, patrol, witness) — they don't map to any wasteland item.
- confidence: 0.9+ for near-exact match, 0.7-0.9 for strong semantic match, 0.5-0.7 for partial.
- is_complete: true ONLY if ALL beads that would contribute to this wasteland item are closed.
- If unsure whether something is complete, set is_complete=false.

Return ONLY confident matches (confidence >= 0.6)."""


def map_beads(
    wasteland_items: list[dict],
    beads: list[dict],
    rig: str,
) -> MapResult | None:
    """Match beads to wasteland items semantically."""

    # Format for LLM
    wl_text = "\n".join(
        f"- {item['id']}: {item['title']} (project: {item.get('project', '?')})"
        for item in wasteland_items
    )

    bead_text = "\n".join(
        f"- {bead['id']}: {bead['title']} [status: {bead.get('status', '?')}]"
        for bead in beads
    )

    user_prompt = f"""Match these beads to wasteland items for rig: {rig}

WASTELAND ITEMS (work packages):
{wl_text}

BEADS (internal tasks):
{bead_text}

Return mappings with confidence scores. Skip beads that are clearly internal (HANDOFF, patrol, witness, escalation)."""

    return generate_structured(SYSTEM_PROMPT, user_prompt, MapResult)


if __name__ == "__main__":
    import json
    if len(sys.argv) >= 2:
        # Read JSON input from file or stdin
        input_data = json.loads(sys.argv[1]) if len(sys.argv) > 1 else json.load(sys.stdin)
        result = map_beads(
            input_data.get("wasteland_items", []),
            input_data.get("beads", []),
            input_data.get("rig", "unknown"),
        )
        if result:
            print(result.model_dump_json(indent=2))
        else:
            print('{"error": "LLM call failed"}', file=sys.stderr)
            sys.exit(1)
