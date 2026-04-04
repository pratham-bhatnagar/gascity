# Cost Analysis — Deepwork Intelligence

## The Problem: Repetitive LLM Work Is Expensive

AI agent orchestration systems (Gas Town, CrewAI, AutoGen, LangGraph, etc.) use frontier LLMs for EVERY task — including repetitive work like:
- Code reviews and scoring
- Release notes generation
- Documentation updates
- Issue/PR formatting
- Status reports

These tasks follow templates. They don't need GPT-4 or Claude Opus. They need structured, deterministic output.

## Cost Per Tool Call

| Provider | Cost/Call | vs Opus | vs Sonnet |
|----------|----------|---------|-----------|
| Claude Opus | $0.0675 | — | — |
| Claude Sonnet | $0.0135 | 80% cheaper | — |
| **DI + MiniMax API** | **$0.0018** | **97% cheaper** | **87% cheaper** |
| DI + Local LLM | $0.025 | 63% cheaper | — |

*Based on avg 2K input + 500 output tokens per tool call*

## Monthly Savings

### Small Team (1,000 calls/mo)

| Approach | Monthly Cost | Savings |
|----------|-------------|---------|
| Claude Opus for everything | $67.50 | — |
| Claude Sonnet for everything | $13.50 | $54/mo |
| **DI + MiniMax API** | **$1.75** | **$65.75/mo (97%)** |
| DI + Self-hosted LLM | $25.00 | $42.50/mo |

### Enterprise (10,000 calls/mo)

| Approach | Monthly Cost | Savings |
|----------|-------------|---------|
| Claude Opus | $675.00 | — |
| **DI + MiniMax API** | **$17.50** | **$657.50/mo (97%)** |
| DI + Self-hosted (H100) | $250.00 | $425/mo |

## But It's Not Just About Cost

DI doesn't just save money — it makes your system **more reliable**:

| Metric | Without DI | With DI |
|--------|-----------|---------|
| Output consistency | Varies per call | Same schema every time |
| Review scoring | Different each run | Calibrated, learning from feedback |
| Evidence quality | Depends on agent mood | Rich, structured from data sources |
| Failure rate | LLM can refuse/hallucinate | Pydantic validation catches errors |
| Latency | 5-15s (frontier model) | 1-3s (smaller model, local GPU) |

## Supported LLM Providers

DI is pluggable — use any OpenAI-compatible endpoint:

| Provider | Speed | Cost | Best For |
|----------|-------|------|----------|
| vLLM (self-hosted) | Fastest | GPU cost only | Teams with GPUs |
| OpenRouter | Fast | Per-token | Flexibility |
| Together AI | Fast | Per-token | Price/performance |
| Ollama (local) | Medium | Free | Development |
| LiteLLM | Varies | Varies | Multi-provider routing |
| Fireworks | Fast | Per-token | Production |

## Supported Orchestration Systems

DI works as a deterministic layer on top of ANY agent system:

| System | Integration | How |
|--------|-------------|-----|
| **Gas Town** | Native MCP | Built-in, hooks + crons |
| **Claude Code** | MCP server | .mcp.json config |
| **Cursor** | MCP server | Settings → MCP |
| **CrewAI** | Tool wrapper | `@tool` decorator |
| **AutoGen** | Function calling | Register as tool |
| **LangGraph** | Tool node | Add as graph node |
| **Gemini CLI** | MCP server | MCP config |
| **OpenCode** | MCP server | MCP config |
| **Codex CLI** | MCP server | MCP config |
| **Custom** | REST API or Python import | `from deepwork_intelligence.server import *` |
