"""Pydantic output schemas for all Deepwork Intelligence agents."""
from __future__ import annotations

from pydantic import BaseModel, Field


# === Wasteland Agents ===

class StampResult(BaseModel):
    """Output from the stamp agent."""
    quality: int = Field(ge=1, le=5, description="Code quality, test coverage, documentation")
    reliability: int = Field(ge=1, le=5, description="On-time delivery, evidence richness")
    creativity: int = Field(ge=1, le=5, description="Problem-solving approach, innovation")
    reasoning: str = Field(description="1-2 sentence justification for scores")
    should_reject: bool = Field(default=False, description="True if evidence is too thin")
    reject_reason: str | None = Field(default=None)


class BeadMapping(BaseModel):
    """A single bead-to-wasteland mapping."""
    wasteland_id: str
    bead_ids: list[str]
    rig: str
    confidence: float = Field(ge=0.0, le=1.0)
    is_complete: bool = Field(description="True if all mapped beads are closed")
    progress: str = Field(description="e.g. '3/5 beads closed'")
    evidence: str = Field(description="Summary for wasteland completion")


class MapResult(BaseModel):
    """Output from the map_beads agent."""
    mappings: list[BeadMapping]


class WastelandItem(BaseModel):
    """Output from the sync agent — a rich wasteland item."""
    title: str = Field(description="Category: clear outsider-readable title")
    project: str
    type: str = Field(description="feature, bug, or docs")
    priority: int = Field(ge=0, le=4)
    effort: str = Field(description="trivial, small, medium, large, or epic")
    description: str = Field(description="Full markdown with Context, Repo, Criteria")
    acceptance_criteria: list[str]
    key_files: list[str]
    skip: bool = Field(default=False, description="True if internal/infra work")
    skip_reason: str | None = Field(default=None)


# === Report Agents ===

class RigSummary(BaseModel):
    name: str
    closed: int
    opened: int
    highlights: list[str]


class ContributorSummary(BaseModel):
    handle: str
    items_completed: int
    stamps: int
    avg_quality: float
    avg_reliability: float
    avg_creativity: float
    tier: str


class HealthInfo(BaseModel):
    dolt_status: str
    thread_usage: str
    agents_alive: int
    agents_stuck: int
    disk_usage: str


class Incident(BaseModel):
    severity: str
    description: str


class OverseerReport(BaseModel):
    """Output from the overseer agent."""
    date: str
    summary: str = Field(description="2-3 sentence executive summary")
    beads_closed: int
    beads_opened: int
    polecats_slung: int
    rigs_active: list[RigSummary]
    stamps_today: int
    stamps_total: int
    items_completed: int
    items_open: int
    reputation: dict
    contributor_activity: list[ContributorSummary]
    health: HealthInfo
    incidents: list[Incident]
    recommendations: list[str] = Field(description="What to focus on today")


class ContributorEntry(BaseModel):
    handle: str
    items_claimed: int
    items_completed: int
    stamps: int
    avg_quality: float
    avg_reliability: float
    avg_creativity: float
    tier: str


class BoardReport(BaseModel):
    """Output from the wasteland board report agent."""
    date: str
    summary: str
    leaderboard: list[ContributorEntry]
    recent_completions: list[dict]
    items_needing_attention: list[str]


# === Content Agents ===

class ReleaseSection(BaseModel):
    type: str = Field(description="features, fixes, improvements, docs")
    entries: list[str]


class ReleaseNotes(BaseModel):
    """Output from the release notes agent."""
    version: str
    title: str
    highlights: list[str]
    sections: list[ReleaseSection]
    breaking_changes: list[str]
    contributors: list[str]


class ChangelogEntry(BaseModel):
    """Output from the changelog agent."""
    type: str = Field(description="decision, deploy, fix, incident, milestone, infra")
    rigs: list[str]
    title: str
    description: str
    impact: str


# === GitHub/Gitea Agents ===

class GitIssue(BaseModel):
    """Output from the issue creation agent."""
    title: str = Field(description="Clear, actionable issue title")
    body: str = Field(description="Markdown body with context, acceptance criteria, etc.")
    labels: list[str] = Field(description="Labels e.g. bug, feature, docs, P0, P1")
    priority: str = Field(description="P0, P1, P2, or P3")


class GitPR(BaseModel):
    """Output from the PR creation agent."""
    title: str = Field(description="Short PR title under 70 chars")
    body: str = Field(description="Markdown PR body with summary, changes, test plan")


class GitRelease(BaseModel):
    """Output from the release notes agent (Gitea)."""
    tag_name: str = Field(description="Semantic version tag e.g. v1.2.0")
    name: str = Field(description="Human-readable release title")
    body: str = Field(description="Markdown release notes with highlights and changes")
