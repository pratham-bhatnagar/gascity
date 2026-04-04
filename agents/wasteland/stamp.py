"""Stamp agent — scores wasteland completions with Q/R/C.

Reads calibration notes from feedback/calibration.md to adjust scoring
based on past feedback. The feedback loop:
  1. DI stamps a completion
  2. Human reviews and submits feedback via feedback_submit()
  3. feedback_apply() generates calibration notes
  4. This agent reads calibration notes before scoring
"""
from __future__ import annotations

import os
import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from shared.llm import generate_structured
from shared.schemas import StampResult

BASE_PROMPT = """You are a code review evaluator for Deepwork Labs.

Given completion evidence for a wasteland work item, score three dimensions:

- **Quality** (1-5): Code correctness, test coverage, documentation completeness, PR quality
  5 = PR merged with tests + docs, 4 = branch merged with evidence, 3 = completed with description, 2 = minimal evidence, 1 = empty

- **Reliability** (1-5): Evidence of delivery, on-time completion, evidence richness
  5 = PR URL + branch + merged confirmation, 4 = branch + bead closed, 3 = bead closed with text, 2 = minimal, 1 = empty

- **Creativity** (1-5): Problem-solving approach, innovation relative to effort level
  5 = novel approach to hard problem, 4 = solid solution to complex task, 3 = standard solution, 2 = minimal effort, 1 = copy-paste

Be fair but strict. Provide 1-2 sentence reasoning.
Set should_reject=true only if evidence is completely empty or says "closed locally" with no detail."""


def _load_calibration() -> str:
    """Load calibration notes from feedback loop."""
    cal_path = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "feedback", "calibration.md")
    if os.path.exists(cal_path):
        return open(cal_path).read()
    return ""


def score_completion(evidence: str, title: str, effort: str, project: str) -> StampResult | None:
    """Score a completion and return structured Q/R/C result."""
    # Build prompt with calibration
    calibration = _load_calibration()
    system_prompt = BASE_PROMPT
    if calibration:
        system_prompt += f"\n\n## CALIBRATION (from feedback)\n{calibration}"

    user_prompt = f"""Score this wasteland completion:

Title: {title}
Project: {project}
Effort level: {effort}
Evidence: {evidence}"""

    return generate_structured(system_prompt, user_prompt, StampResult)


if __name__ == "__main__":
    if len(sys.argv) >= 5:
        result = score_completion(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4])
        if result:
            print(result.model_dump_json(indent=2))
        else:
            print('{"error": "LLM call failed"}', file=sys.stderr)
            sys.exit(1)
    else:
        print("Usage: stamp.py <evidence> <title> <effort> <project>", file=sys.stderr)
        sys.exit(1)
