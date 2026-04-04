"""Deepwork Intelligence — Configuration.

All settings are configurable via:
  1. Environment variables (highest priority)
  2. config.yaml file in DI root
  3. Defaults (lowest priority)

This makes DI pluggable — any LLM provider, any database, any wasteland setup.
"""
from __future__ import annotations

import os
import yaml
from dataclasses import dataclass, field
from typing import Any


@dataclass
class LLMConfig:
    """LLM provider configuration. Works with any OpenAI-compatible endpoint."""
    base_url: str = "http://localhost:8080/v1"
    model: str = "MiniMaxAI/MiniMax-M2.5"
    api_key: str = "not-needed"
    temperature: float = 0.1
    max_tokens: int = 4096
    # Provider hint — for logging and analytics
    provider: str = "vllm-local"


@dataclass
class DoltConfig:
    """Dolt database connection."""
    host: str = "127.0.0.1"
    port: int = 3307
    user: str = "root"
    password: str = ""


@dataclass
class WastelandConfig:
    """Wasteland federation configuration."""
    database: str = "wl_commons"
    handle: str = "deepwork"
    # Project → rig DB mapping (customizable per deployment)
    project_to_db: dict[str, str] = field(default_factory=lambda: {})


@dataclass
class GiteaConfig:
    """Gitea (internal git platform) configuration."""
    url: str = "http://localhost:3300"
    token: str = ""
    org: str = "Deepwork-AI"


@dataclass
class Config:
    """Root configuration for Deepwork Intelligence."""
    llm: LLMConfig = field(default_factory=LLMConfig)
    dolt: DoltConfig = field(default_factory=DoltConfig)
    wasteland: WastelandConfig = field(default_factory=WastelandConfig)
    gitea: GiteaConfig = field(default_factory=GiteaConfig)
    gt_root: str = ""
    logs_dir: str = ""
    feedback_dir: str = ""

    @classmethod
    def load(cls, config_path: str | None = None) -> "Config":
        """Load config from env vars, then yaml file, then defaults."""
        cfg = cls()

        # 1. Defaults
        di_root = os.path.dirname(os.path.abspath(__file__))
        cfg.gt_root = os.path.expanduser("~/gt")
        cfg.logs_dir = os.path.join(di_root, "logs")
        cfg.feedback_dir = os.path.join(di_root, "feedback")

        # 2. YAML file
        if config_path is None:
            config_path = os.path.join(di_root, "config.yaml")

        if os.path.exists(config_path):
            with open(config_path) as f:
                data = yaml.safe_load(f) or {}
            _merge_config(cfg, data)

        # 3. Environment variables (highest priority)
        _apply_env(cfg)

        # Ensure dirs exist
        os.makedirs(cfg.logs_dir, exist_ok=True)
        os.makedirs(cfg.feedback_dir, exist_ok=True)

        return cfg


def _merge_config(cfg: Config, data: dict[str, Any]):
    """Merge yaml data into config."""
    if "llm" in data:
        for k, v in data["llm"].items():
            if hasattr(cfg.llm, k):
                setattr(cfg.llm, k, v)

    if "dolt" in data:
        for k, v in data["dolt"].items():
            if hasattr(cfg.dolt, k):
                setattr(cfg.dolt, k, v)

    if "wasteland" in data:
        for k, v in data["wasteland"].items():
            if hasattr(cfg.wasteland, k):
                setattr(cfg.wasteland, k, v)

    if "gitea" in data:
        for k, v in data["gitea"].items():
            if hasattr(cfg.gitea, k):
                setattr(cfg.gitea, k, v)

    if "gt_root" in data:
        cfg.gt_root = data["gt_root"]
    if "logs_dir" in data:
        cfg.logs_dir = data["logs_dir"]
    if "feedback_dir" in data:
        cfg.feedback_dir = data["feedback_dir"]


def _apply_env(cfg: Config):
    """Override config from environment variables."""
    # LLM
    if v := os.environ.get("DI_LLM_BASE_URL"):
        cfg.llm.base_url = v
    if v := os.environ.get("DI_LLM_MODEL"):
        cfg.llm.model = v
    if v := os.environ.get("DI_LLM_API_KEY"):
        cfg.llm.api_key = v
    if v := os.environ.get("DI_LLM_TEMPERATURE"):
        cfg.llm.temperature = float(v)
    if v := os.environ.get("DI_LLM_PROVIDER"):
        cfg.llm.provider = v

    # Dolt
    if v := os.environ.get("DI_DOLT_HOST"):
        cfg.dolt.host = v
    if v := os.environ.get("DI_DOLT_PORT"):
        cfg.dolt.port = int(v)
    if v := os.environ.get("DI_DOLT_USER"):
        cfg.dolt.user = v
    if v := os.environ.get("DI_DOLT_PASSWORD"):
        cfg.dolt.password = v

    # Wasteland
    if v := os.environ.get("DI_WASTELAND_DB"):
        cfg.wasteland.database = v
    if v := os.environ.get("DI_WASTELAND_HANDLE"):
        cfg.wasteland.handle = v

    # Gitea
    if v := os.environ.get("DI_GITEA_URL"):
        cfg.gitea.url = v
    if v := os.environ.get("DI_GITEA_TOKEN"):
        cfg.gitea.token = v
    if v := os.environ.get("DI_GITEA_ORG"):
        cfg.gitea.org = v

    # Paths
    if v := os.environ.get("GT_ROOT"):
        cfg.gt_root = v
    if v := os.environ.get("DI_LOGS_DIR"):
        cfg.logs_dir = v
