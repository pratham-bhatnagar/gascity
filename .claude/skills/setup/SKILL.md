---
name: di-setup
description: Set up Deepwork Intelligence — deterministic AI tools for your agent workflows. Choose self-hosted (free) or Deepwork Cloud ($8/mo).
---

# Deepwork Intelligence — Setup

Welcome! Let's set up DI in under 60 seconds.

## Step 1: Choose Your Plan

Ask the user:

**How do you want to run DI?**

**A) Self-Hosted (Free forever)**
- Bring your own LLM (Ollama, vLLM, OpenRouter, etc.)
- You manage the server
- Unlimited tool calls
- Best for: developers with GPU access or API keys

**B) Deepwork Cloud ($8/month)**
- We run MiniMax M2.5 on H100 GPUs
- No setup needed — just an API key
- 10K tool calls/month included
- Best for: teams who want zero infrastructure

## Step 2: Install

```bash
pip install deepwork-intelligence
```

## Step 3: Configure

### Option A — Self-Hosted

Create `config.yaml`:
```yaml
llm:
  base_url: "http://localhost:11434/v1"  # Ollama
  # base_url: "http://localhost:8080/v1"  # vLLM
  # base_url: "https://openrouter.ai/api/v1"  # OpenRouter
  model: "your-model-name"
  api_key: "your-key-if-needed"
```

### Option B — Deepwork Cloud

```yaml
llm:
  base_url: "https://api.mind.deepwork.art/v1"
  api_key: "di_your_api_key_here"  # Get from mind.deepwork.art
  model: "minimax-m2.5"
```

## Step 4: Register with Claude Code

Add to your project's `.mcp.json`:
```json
{
  "mcpServers": {
    "deepwork-intelligence": {
      "command": "di-server",
      "env": {
        "DI_LLM_API_KEY": "your-key"
      }
    }
  }
}
```

## Step 5: Verify

Restart Claude Code. You should see DI tools available:
- `wasteland_*` — reputation and work tracking
- `docs_*` — documentation generation
- `github_*` — issue/PR/release automation
- `analytics_*` — usage monitoring
- `feedback_*` — self-improving scoring

## Done!

Your Claude Code agent now has 27+ deterministic tools. Try:
- "Generate a README for this project" → uses `docs_generate`
- "Create a GitHub release" → uses `github_create_release`
- "Check system health" → uses `health`

## Supported Tools

All tools work with ANY MCP-compatible AI coding tool:
- Claude Code, Cursor, VS Code Copilot, Gemini CLI, OpenCode, Codex CLI, Windsurf, Kimi
