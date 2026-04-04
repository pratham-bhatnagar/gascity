"""LLM client — pluggable, works with any OpenAI-compatible endpoint.

Supports: vLLM, OpenRouter, Together AI, LiteLLM, Ollama, Fireworks, etc.
Configure via config.yaml or environment variables.
"""
from __future__ import annotations

import logging
import re
import sys

from openai import OpenAI
from pydantic import BaseModel

logger = logging.getLogger(__name__)

# Defaults — overridden by config at runtime
_base_url = "http://localhost:8080/v1"
_model = "MiniMaxAI/MiniMax-M2.5"
_api_key = "not-needed"
_temperature = 0.1


def configure(base_url: str, model: str, api_key: str = "not-needed",
              temperature: float = 0.1):
    """Set LLM configuration. Called by server.py on startup."""
    global _base_url, _model, _api_key, _temperature
    _base_url = base_url
    _model = model
    _api_key = api_key
    _temperature = temperature


def _strip_think_tags(text: str) -> str:
    """Strip <think>...</think> tags that some models wrap around output."""
    cleaned = re.sub(r'<think>.*?</think>\s*', '', text, flags=re.DOTALL).strip()
    start = cleaned.find('{')
    end = cleaned.rfind('}')
    if start >= 0 and end > start:
        cleaned = cleaned[start:end + 1]
    return cleaned


def get_client(base_url: str | None = None) -> OpenAI:
    """Get OpenAI client."""
    return OpenAI(base_url=base_url or _base_url, api_key=_api_key)


def generate_structured(
    system_prompt: str,
    user_prompt: str,
    output_schema: type[BaseModel],
    temperature: float | None = None,
    max_tokens: int = 4096,
    base_url: str | None = None,
    model: str | None = None,
) -> BaseModel | None:
    """Call LLM and parse response into a Pydantic model.

    Uses tool calling for structured output. Falls back to JSON extraction.
    Works with any OpenAI-compatible endpoint.
    """
    client = get_client(base_url)
    _temp = temperature if temperature is not None else _temperature
    _mdl = model or _model

    schema = output_schema.model_json_schema()
    tool = {
        "type": "function",
        "function": {
            "name": "submit_result",
            "description": f"Submit the structured result as {output_schema.__name__}",
            "parameters": schema,
        },
    }

    try:
        response = client.chat.completions.create(
            model=_mdl,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            tools=[tool],
            tool_choice={"type": "function", "function": {"name": "submit_result"}},
            temperature=_temp,
            max_tokens=max_tokens,
        )

        message = response.choices[0].message
        if message.tool_calls:
            args_json = _strip_think_tags(message.tool_calls[0].function.arguments)
            return output_schema.model_validate_json(args_json)

        if message.content:
            content = _strip_think_tags(message.content)
            return output_schema.model_validate_json(content)

    except Exception as e:
        logger.error(f"LLM call failed: {e}")

    return None


def generate_text(
    system_prompt: str,
    user_prompt: str,
    temperature: float | None = None,
    max_tokens: int = 4096,
    base_url: str | None = None,
    model: str | None = None,
) -> str:
    """Simple text generation."""
    client = get_client(base_url)
    _temp = temperature if temperature is not None else _temperature
    _mdl = model or _model

    try:
        response = client.chat.completions.create(
            model=_mdl,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            temperature=_temp,
            max_tokens=max_tokens,
        )
        content = response.choices[0].message.content or ""
        return _strip_think_tags(content)
    except Exception as e:
        logger.error(f"LLM call failed: {e}")
        return ""
