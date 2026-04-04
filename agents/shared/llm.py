"""MiniMax M2.5 LLM client via vLLM (OpenAI-compatible)."""
from __future__ import annotations

import json
import logging
import sys
from typing import Any

from openai import OpenAI
from pydantic import BaseModel

logger = logging.getLogger(__name__)

# Default vLLM endpoint
VLLM_BASE_URL = "http://localhost:8080/v1"
VLLM_MODEL = "MiniMaxAI/MiniMax-M2.5"


def _strip_think_tags(text: str) -> str:
    """Strip <think>...</think> tags that MiniMax M2.5 wraps around output."""
    import re
    # Remove <think>...</think> blocks (greedy, handles multiline)
    cleaned = re.sub(r'<think>.*?</think>\s*', '', text, flags=re.DOTALL)
    # Also try to extract JSON if surrounded by other text
    cleaned = cleaned.strip()
    # Find first { and last } for JSON extraction
    start = cleaned.find('{')
    end = cleaned.rfind('}')
    if start >= 0 and end > start:
        cleaned = cleaned[start:end + 1]
    return cleaned


def get_client(base_url: str = VLLM_BASE_URL) -> OpenAI:
    """Get OpenAI client pointing to vLLM."""
    return OpenAI(base_url=base_url, api_key="not-needed")


def generate_structured(
    system_prompt: str,
    user_prompt: str,
    output_schema: type[BaseModel],
    temperature: float = 0.1,
    max_tokens: int = 4096,
    base_url: str = VLLM_BASE_URL,
    model: str = VLLM_MODEL,
) -> BaseModel | None:
    """Call MiniMax and parse response into a Pydantic model.

    Uses tool calling for structured output — MiniMax M2.5 supports this.
    Falls back to JSON parsing from text if tool calling fails.
    """
    client = get_client(base_url)

    # Build tool definition from Pydantic schema
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
            model=model,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            tools=[tool],
            tool_choice={"type": "function", "function": {"name": "submit_result"}},
            temperature=temperature,
            max_tokens=max_tokens,
        )

        # Extract tool call arguments
        message = response.choices[0].message
        if message.tool_calls:
            args_json = message.tool_calls[0].function.arguments
            # MiniMax may wrap JSON in <think> tags — strip them
            args_json = _strip_think_tags(args_json)
            return output_schema.model_validate_json(args_json)

        # Fallback: try parsing content as JSON
        if message.content:
            content = _strip_think_tags(message.content)
            return output_schema.model_validate_json(content)

    except Exception as e:
        logger.error(f"LLM call failed: {e}")

    return None


def generate_text(
    system_prompt: str,
    user_prompt: str,
    temperature: float = 0.1,
    max_tokens: int = 4096,
    base_url: str = VLLM_BASE_URL,
    model: str = VLLM_MODEL,
) -> str:
    """Simple text generation (no structured output)."""
    client = get_client(base_url)

    try:
        response = client.chat.completions.create(
            model=model,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            temperature=temperature,
            max_tokens=max_tokens,
        )
        content = response.choices[0].message.content or ""
        return _strip_think_tags(content)
    except Exception as e:
        logger.error(f"LLM call failed: {e}")
        return ""
