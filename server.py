"""Deepwork Intelligence — MCP Server.

Deterministic intelligence layer for Gas Town.
Small ADK agents powered by MiniMax M2.5 on local H100 GPUs.

Tools:
  wasteland.stamp       — Score a completion (Q/R/C)
  wasteland.map_beads   — Semantic bead-to-wasteland matching
  wasteland.complete    — Submit completion for matched items
  wasteland.status      — Current wasteland board status
  reports.overseer      — Generate daily overseer summary
  reports.board         — Generate wasteland leaderboard
"""
from __future__ import annotations

import hashlib
import json
import logging
import os
import subprocess
import sys
import time
from datetime import datetime, timezone

from mcp.server.fastmcp import FastMCP

# Add agents to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "agents"))

from shared.dolt import DoltClient
from shared.llm import generate_structured, generate_text
from shared.schemas import StampResult, MapResult, OverseerReport, BoardReport

logging.basicConfig(level=logging.INFO, stream=sys.stderr,
                    format="%(asctime)s [DI] %(message)s")
logger = logging.getLogger(__name__)

GT_ROOT = os.environ.get("GT_ROOT", os.path.expanduser("~/gt"))
WASTELAND_DB = "wl_commons"

# Project → rig DB mapping
PROJECT_TO_DB = {
    "ai-planogram": "villa_ai_planogram",
    "alc-ai-villa": "villa_alc_ai",
    "officeworld": "officeworld",
    "deepwork-site": "deepwork_site",
    "gt-monitor": "gtm",
    "products": "prd",
    "media-studio": "media_studio",
    "command-center": "command_center",
    "deepwork-intelligence": "di",
}

dolt = DoltClient()

# ─── MCP Server ────────────────────────────────────────────

mcp = FastMCP(
    "deepwork-intelligence",
    instructions="""Deepwork Intelligence — deterministic AI tools for Gas Town.

These tools use MiniMax M2.5 (local GPU) for smart structured output.
Use them for wasteland operations, reports, and content generation.

IMPORTANT: wasteland data is in the 'wl_commons' Dolt database.
Bead data is in per-rig databases (villa_ai_planogram, villa_alc_ai, etc.).""",
)


# ─── Wasteland Tools ──────────────────────────────────────

@mcp.tool()
async def wasteland_stamp(completion_id: str) -> str:
    """Score a wasteland completion with Q/R/C using MiniMax.

    Args:
        completion_id: The completion ID to score (e.g. c-abc123)
    """
    from wasteland.stamp import score_completion

    # Get completion details from Dolt
    rows = dolt.query(WASTELAND_DB,
        "SELECT c.id, c.evidence, c.completed_by, w.title, w.effort_level, w.project "
        "FROM completions c JOIN wanted w ON w.id = c.wanted_id "
        "WHERE c.id = %s AND c.validated_by IS NULL", (completion_id,))

    if not rows:
        return json.dumps({"error": f"Completion {completion_id} not found or already reviewed"})

    row = rows[0]
    result = score_completion(
        evidence=row["evidence"] or "",
        title=row["title"] or "",
        effort=row["effort_level"] or "medium",
        project=row["project"] or "",
    )

    if not result:
        return json.dumps({"error": "MiniMax scoring failed — vLLM may be down"})

    if result.should_reject:
        dolt.execute(WASTELAND_DB,
            "DELETE FROM completions WHERE id = %s", (completion_id,))
        dolt.execute(WASTELAND_DB,
            "UPDATE wanted SET status='open', claimed_by=NULL WHERE id = "
            "(SELECT wanted_id FROM completions WHERE id = %s)", (completion_id,))
        dolt.commit_and_push(WASTELAND_DB, f"rejected {completion_id}: {result.reject_reason}")
        return json.dumps({"action": "rejected", "reason": result.reject_reason})

    # Create stamp
    stamp_id = f"s-{hashlib.sha256(f'{completion_id}-{time.time_ns()}'.encode()).hexdigest()[:16]}"
    valence = json.dumps({"quality": result.quality, "reliability": result.reliability,
                          "creativity": result.creativity})
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S")

    dolt.execute(WASTELAND_DB,
        "INSERT INTO stamps (id, author, subject, valence, confidence, severity, "
        "context_id, context_type, stamp_type, message, created_at) "
        "VALUES (%s, 'deepwork-intelligence', %s, %s, 0.9, 'leaf', %s, 'completion', "
        "'work', %s, %s)",
        (stamp_id, row["completed_by"], valence, completion_id,
         f"DI Review: Q:{result.quality} R:{result.reliability} C:{result.creativity} — {result.reasoning}",
         now))

    dolt.execute(WASTELAND_DB,
        "UPDATE completions SET validated_by='deepwork-intelligence', stamp_id=%s, "
        "validated_at=NOW() WHERE id = %s", (stamp_id, completion_id))

    dolt.execute(WASTELAND_DB,
        "UPDATE wanted SET status='completed', updated_at=NOW() WHERE id = "
        "(SELECT wanted_id FROM completions WHERE id = %s)", (completion_id,))

    dolt.commit_and_push(WASTELAND_DB,
        f"DI stamp: {completion_id} Q:{result.quality} R:{result.reliability} C:{result.creativity}")

    return json.dumps({
        "stamp_id": stamp_id,
        "quality": result.quality,
        "reliability": result.reliability,
        "creativity": result.creativity,
        "reasoning": result.reasoning,
    })


@mcp.tool()
async def wasteland_map_beads(rig: str) -> str:
    """Semantically match beads to wasteland items for a rig using MiniMax.

    Args:
        rig: The rig name (e.g. villa_ai_planogram)
    """
    from wasteland.map_beads import map_beads

    # Find the project name for this rig
    project = None
    for p, db in PROJECT_TO_DB.items():
        if db == rig:
            project = p
            break
    if not project:
        return json.dumps({"error": f"Unknown rig: {rig}"})

    # Get open wasteland items for this project
    items = dolt.query(WASTELAND_DB,
        "SELECT id, title, project FROM wanted "
        "WHERE status IN ('open', 'claimed') AND project = %s", (project,))

    if not items:
        return json.dumps({"mappings": [], "message": f"No open items for {project}"})

    # Get all beads for this rig
    beads = dolt.query(rig,
        "SELECT id, title, status FROM issues ORDER BY updated_at DESC LIMIT 100")

    if not beads:
        return json.dumps({"mappings": [], "message": f"No beads in {rig}"})

    result = map_beads(items, beads, rig)

    if not result:
        return json.dumps({"error": "MiniMax mapping failed — vLLM may be down"})

    # Store mappings in Dolt
    for m in result.mappings:
        for bead_id in m.bead_ids:
            dolt.execute(WASTELAND_DB,
                "INSERT IGNORE INTO bead_mappings (wasteland_id, bead_id, rig, confidence, mapped_at) "
                "VALUES (%s, %s, %s, %s, NOW())",
                (m.wasteland_id, bead_id, m.rig, m.confidence))

    dolt.commit_and_push(WASTELAND_DB, f"DI map: {len(result.mappings)} mappings for {rig}")

    return json.dumps({
        "mappings": [m.model_dump() for m in result.mappings],
        "total": len(result.mappings),
    })


@mcp.tool()
async def wasteland_complete_matched() -> str:
    """Check bead mappings and submit completions for fully-closed items.

    No args needed — reads from bead_mappings table automatically.
    """
    # Get all mappings where wasteland item is still open/claimed
    mappings = dolt.query(WASTELAND_DB,
        "SELECT DISTINCT bm.wasteland_id, w.title, w.project "
        "FROM bead_mappings bm JOIN wanted w ON w.id = bm.wasteland_id "
        "WHERE w.status IN ('open', 'claimed')")

    completed = 0
    for m in mappings:
        wl_id = m["wasteland_id"]
        project = m["project"]
        rig_db = PROJECT_TO_DB.get(project)
        if not rig_db:
            continue

        # Get all mapped beads for this wasteland item
        bead_rows = dolt.query(WASTELAND_DB,
            "SELECT bead_id FROM bead_mappings WHERE wasteland_id = %s", (wl_id,))
        bead_ids = [r["bead_id"] for r in bead_rows]

        if not bead_ids:
            continue

        # Check how many are closed
        placeholders = ",".join(["%s"] * len(bead_ids))
        closed = dolt.query(rig_db,
            f"SELECT id FROM issues WHERE id IN ({placeholders}) AND status = 'closed'",
            tuple(bead_ids))

        closed_ids = [r["id"] for r in closed]

        if len(closed_ids) == len(bead_ids):
            # All beads closed — submit completion
            evidence = f"All {len(bead_ids)} mapped beads closed: {', '.join(closed_ids)}"

            # Claim if needed
            dolt.execute(WASTELAND_DB,
                "UPDATE wanted SET claimed_by='deepwork' WHERE id=%s AND claimed_by IS NULL",
                (wl_id,))

            # Try gt wl done
            try:
                subprocess.run(
                    ["gt", "wl", "done", wl_id, "--evidence", evidence],
                    capture_output=True, text=True, cwd=GT_ROOT, timeout=30)
                completed += 1
            except Exception:
                # Fallback: write directly
                cid = f"c-{hashlib.sha256(f'{wl_id}{time.time()}'.encode()).hexdigest()[:16]}"
                dolt.execute(WASTELAND_DB,
                    "INSERT INTO completions (id, wanted_id, completed_by, evidence, completed_at) "
                    "VALUES (%s, %s, 'deepwork', %s, NOW())",
                    (cid, wl_id, evidence))
                dolt.execute(WASTELAND_DB,
                    "UPDATE wanted SET status='in_review', updated_at=NOW() WHERE id=%s", (wl_id,))
                completed += 1

    if completed > 0:
        dolt.commit_and_push(WASTELAND_DB, f"DI complete: {completed} items")

    return json.dumps({"completed": completed})


@mcp.tool()
async def wasteland_status() -> str:
    """Get current wasteland board status — items, stamps, reputation."""
    status = dolt.query(WASTELAND_DB,
        "SELECT status, COUNT(*) as cnt FROM wanted GROUP BY status")

    stamps = dolt.query(WASTELAND_DB, "SELECT COUNT(*) as cnt FROM stamps")
    completions = dolt.query(WASTELAND_DB, "SELECT COUNT(*) as cnt FROM completions")

    # Get charsheet via gt
    try:
        r = subprocess.run(["gt", "wl", "charsheet", "deepwork"],
                          capture_output=True, text=True, cwd=GT_ROOT, timeout=15)
        charsheet = r.stdout
    except Exception:
        charsheet = "unavailable"

    return json.dumps({
        "items_by_status": {r["status"]: r["cnt"] for r in status},
        "total_stamps": stamps[0]["cnt"] if stamps else 0,
        "total_completions": completions[0]["cnt"] if completions else 0,
        "charsheet": charsheet,
    })


@mcp.tool()
async def wasteland_review_all() -> str:
    """Review ALL pending in_review completions. Stamps each one via MiniMax.

    This is the autonomous review workflow — call it to process the review queue.
    """
    pending = dolt.query(WASTELAND_DB,
        "SELECT c.id FROM completions c "
        "JOIN wanted w ON w.id = c.wanted_id "
        "WHERE w.status = 'in_review' AND c.validated_by IS NULL")

    if not pending:
        return json.dumps({"message": "No items to review", "reviewed": 0})

    results = []
    for row in pending:
        result_json = await wasteland_stamp(row["id"])
        results.append(json.loads(result_json))

    return json.dumps({
        "reviewed": len(results),
        "results": results,
    })


@mcp.tool()
async def wasteland_flywheel(rig: str) -> str:
    """Run the full wasteland flywheel for a rig: map → complete → review.

    This is the main autonomous loop. Call it periodically.

    Args:
        rig: The rig database name (e.g. villa_ai_planogram)
    """
    steps = []

    # Step 1: Map beads
    map_result = await wasteland_map_beads(rig)
    steps.append({"step": "map", "result": json.loads(map_result)})

    # Step 2: Complete matched items
    complete_result = await wasteland_complete_matched()
    steps.append({"step": "complete", "result": json.loads(complete_result)})

    # Step 3: Review pending
    review_result = await wasteland_review_all()
    steps.append({"step": "review", "result": json.loads(review_result)})

    return json.dumps({"rig": rig, "steps": steps})


# ─── Docs Tools ───────────────────────────────────────────

DOCS_SYSTEM_PROMPT = """You are a technical documentation writer for Deepwork Labs.

Write clear, professional documentation following the DEEPWORK LABS style:
- Clean, data-first, no fluff
- Code examples where relevant
- Structured with clear headings
- Include setup instructions, API references, architecture overview as appropriate

Output valid markdown. Never include internal rig names, ports, tokens, or private info unless the doc is marked internal."""


@mcp.tool()
async def docs_create(rig: str, title: str, doc_type: str, context: str) -> str:
    """Create a new documentation file for a rig using MiniMax.

    Args:
        rig: Rig name (e.g. gt_monitor, villa_ai_planogram)
        title: Document title
        doc_type: Type — readme, changelog, architecture, runbook, api-reference, onboarding
        context: All relevant context — beads, code structure, discussions, decisions
    """
    prompt = f"""Create a {doc_type} document for the {rig} project.

Title: {title}

Context provided:
{context}

Generate a complete, production-quality {doc_type} document in markdown."""

    content = generate_text(DOCS_SYSTEM_PROMPT, prompt, max_tokens=8192)

    if not content:
        return json.dumps({"error": "MiniMax doc generation failed"})

    # Write to rig docs directory
    docs_dir = os.path.join(GT_ROOT, rig, "docs")
    os.makedirs(docs_dir, exist_ok=True)

    filename = title.lower().replace(" ", "-").replace("/", "-")[:50] + ".md"
    filepath = os.path.join(docs_dir, filename)

    with open(filepath, "w") as f:
        f.write(content)

    # Git commit
    subprocess.run(["git", "add", filepath], cwd=os.path.join(GT_ROOT, rig),
                   capture_output=True, timeout=10)
    subprocess.run(["git", "commit", "-m", f"docs: add {doc_type} — {title}"],
                   cwd=os.path.join(GT_ROOT, rig), capture_output=True, timeout=10)

    return json.dumps({
        "path": filepath,
        "type": doc_type,
        "title": title,
        "size": len(content),
    })


@mcp.tool()
async def docs_append(rig: str, doc_path: str, section: str, content: str) -> str:
    """Append a new section to an existing doc. MiniMax formats it to match the doc style.

    Args:
        rig: Rig name
        doc_path: Relative path within the rig (e.g. docs/architecture.md)
        section: Section heading to add
        content: Raw content/context to format and append
    """
    filepath = os.path.join(GT_ROOT, rig, doc_path)

    if not os.path.exists(filepath):
        return json.dumps({"error": f"File not found: {filepath}"})

    existing = open(filepath).read()

    prompt = f"""Append a new section to this existing document.

EXISTING DOCUMENT:
{existing[:3000]}

NEW SECTION TO ADD:
Heading: {section}
Content: {content}

Write ONLY the new section (with heading). Match the style and formatting of the existing document."""

    new_section = generate_text(DOCS_SYSTEM_PROMPT, prompt, max_tokens=4096)

    if not new_section:
        return json.dumps({"error": "MiniMax generation failed"})

    with open(filepath, "a") as f:
        f.write(f"\n\n{new_section}")

    subprocess.run(["git", "add", filepath], cwd=os.path.join(GT_ROOT, rig),
                   capture_output=True, timeout=10)
    subprocess.run(["git", "commit", "-m", f"docs: append {section} to {doc_path}"],
                   cwd=os.path.join(GT_ROOT, rig), capture_output=True, timeout=10)

    return json.dumps({"path": filepath, "section": section, "appended": len(new_section)})


@mcp.tool()
async def docs_update(rig: str, doc_path: str, section: str, new_content: str) -> str:
    """Rewrite a specific section of an existing doc using MiniMax.

    Args:
        rig: Rig name
        doc_path: Relative path within the rig
        section: Section heading to rewrite (must exist in the doc)
        new_content: New context/information for this section
    """
    filepath = os.path.join(GT_ROOT, rig, doc_path)

    if not os.path.exists(filepath):
        return json.dumps({"error": f"File not found: {filepath}"})

    existing = open(filepath).read()

    prompt = f"""Rewrite the "{section}" section of this document with updated information.

FULL DOCUMENT:
{existing[:4000]}

SECTION TO REWRITE: {section}
NEW INFORMATION: {new_content}

Output the COMPLETE document with the section rewritten. Keep all other sections unchanged."""

    updated = generate_text(DOCS_SYSTEM_PROMPT, prompt, max_tokens=8192)

    if not updated:
        return json.dumps({"error": "MiniMax generation failed"})

    with open(filepath, "w") as f:
        f.write(updated)

    subprocess.run(["git", "add", filepath], cwd=os.path.join(GT_ROOT, rig),
                   capture_output=True, timeout=10)
    subprocess.run(["git", "commit", "-m", f"docs: update {section} in {doc_path}"],
                   cwd=os.path.join(GT_ROOT, rig), capture_output=True, timeout=10)

    return json.dumps({"path": filepath, "section": section, "size": len(updated)})


@mcp.tool()
async def docs_generate(rig: str, doc_type: str) -> str:
    """Auto-generate a doc by reading the rig's code, beads, and git history.

    MiniMax reads the rig state and generates a complete document.

    Args:
        rig: Rig name
        doc_type: readme, architecture, api-reference, changelog, onboarding
    """
    context_parts = []

    # Read beads
    rig_db = PROJECT_TO_DB.get(rig, rig)
    try:
        beads = dolt.query(rig_db,
            "SELECT id, title, status, priority FROM issues ORDER BY priority ASC LIMIT 30")
        context_parts.append("BEADS:\n" + "\n".join(
            f"  [{b['status']}] {b['id']}: {b['title']}" for b in beads))
    except Exception:
        pass

    # Read git log
    try:
        r = subprocess.run(
            ["git", "log", "--oneline", "-20"],
            cwd=os.path.join(GT_ROOT, rig), capture_output=True, text=True, timeout=10)
        if r.stdout:
            context_parts.append(f"GIT LOG:\n{r.stdout}")
    except Exception:
        pass

    # Read existing CLAUDE.md for project context
    claude_md = os.path.join(GT_ROOT, rig, "CLAUDE.md")
    if os.path.exists(claude_md):
        context_parts.append(f"CLAUDE.MD:\n{open(claude_md).read()[:2000]}")

    # Read directory structure
    try:
        r = subprocess.run(
            ["find", ".", "-maxdepth", "3", "-type", "f", "-name", "*.py",
             "-o", "-name", "*.rs", "-o", "-name", "*.ts", "-o", "-name", "*.toml"],
            cwd=os.path.join(GT_ROOT, rig), capture_output=True, text=True, timeout=10)
        if r.stdout:
            context_parts.append(f"CODE FILES:\n{r.stdout[:1000]}")
    except Exception:
        pass

    context = "\n\n".join(context_parts)

    return await docs_create(rig, f"{rig} {doc_type}", doc_type, context)


@mcp.tool()
async def docs_index(rig: str) -> str:
    """List all documentation files for a rig.

    Args:
        rig: Rig name
    """
    docs_dir = os.path.join(GT_ROOT, rig, "docs")
    rig_root = os.path.join(GT_ROOT, rig)

    files = []

    # Check docs/ directory
    if os.path.isdir(docs_dir):
        for f in os.listdir(docs_dir):
            if f.endswith(".md"):
                path = os.path.join(docs_dir, f)
                files.append({
                    "path": f"docs/{f}",
                    "size": os.path.getsize(path),
                    "title": f.replace("-", " ").replace(".md", "").title(),
                })

    # Check root level docs
    for name in ["README.md", "CLAUDE.md", "AGENTS.md", "CHANGELOG.md"]:
        path = os.path.join(rig_root, name)
        if os.path.exists(path):
            files.append({
                "path": name,
                "size": os.path.getsize(path),
                "title": name,
            })

    return json.dumps({"rig": rig, "docs": files, "total": len(files)})


# ─── Health ────────────────────────────────────────────────

@mcp.tool()
async def health() -> str:
    """Check Deepwork Intelligence health — Dolt, vLLM, system status."""
    checks = {}

    # Dolt
    try:
        dolt.query("wl_commons", "SELECT 1 as ok")
        checks["dolt"] = "ok"
    except Exception as e:
        checks["dolt"] = f"error: {e}"

    # vLLM
    try:
        import httpx
        async with httpx.AsyncClient() as client:
            r = await client.get("http://localhost:8080/v1/models", timeout=5)
            checks["vllm"] = "ok" if r.status_code == 200 else f"status {r.status_code}"
    except Exception as e:
        checks["vllm"] = f"error: {e}"

    checks["timestamp"] = datetime.now(timezone.utc).isoformat()
    return json.dumps(checks)


# ─── Entry Point ──────────────────────────────────────────

def main():
    logger.info("Deepwork Intelligence MCP server starting...")
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
