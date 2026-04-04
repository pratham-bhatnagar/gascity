"""Stamp agent — scores wasteland completions with Q/R/C."""
from __future__ import annotations

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from shared.llm import generate_structured
from shared.schemas import StampResult

SYSTEM_PROMPT = """You are a code review evaluator for Deepwork Labs.

Given completion evidence for a wasteland work item, score three dimensions:

- **Quality** (1-5): Code correctness, test coverage, documentation completeness, PR quality
  5 = PR merged with tests + docs, 4 = branch merged with evidence, 3 = completed with description, 2 = minimal evidence, 1 = empty

- **Reliability** (1-5): Evidence of delivery, on-time completion, evidence richness
  5 = PR URL + branch + merged confirmation, 4 = branch + bead closed, 3 = bead closed with text, 2 = minimal, 1 = empty

- **Creativity** (1-5): Problem-solving approach, innovation relative to effort level
  5 = novel approach to hard problem, 4 = solid solution to complex task, 3 = standard solution, 2 = minimal effort, 1 = copy-paste

Be fair but strict. Provide 1-2 sentence reasoning.
Set should_reject=true only if evidence is completely empty or says "closed locally" with no detail."""


def score_completion(evidence: str, title: str, effort: str, project: str) -> StampResult | None:
    """Score a completion and return structured Q/R/C result."""
    user_prompt = f"""Score this wasteland completion:

Title: {title}
Project: {project}
Effort level: {effort}
Evidence: {evidence}"""

    return generate_structured(SYSTEM_PROMPT, user_prompt, StampResult)


if __name__ == "__main__":
    # CLI usage: python stamp.py "<evidence>" "<title>" "<effort>" "<project>"
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
