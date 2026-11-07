"""Structured logging for adapters (the core never logs: it performs no I/O).

Policy (docs/en/project/adr/0007-structured-logging.md):
- logs go to stderr, as text or one JSON object per line;
- events carry identifiers, durations, outcomes and error codes, never financial values,
  inputs, outputs, file names or prompts. ``input_hash`` is allowed: it identifies a run
  without revealing it.
"""

from __future__ import annotations

import json
import logging
import sys
import time
from typing import Any

from ._engine import ENGINE_VERSION

_FORBIDDEN_KEYS = {"input", "inputs", "value", "values", "arguments", "result", "prompt"}


class JsonFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        payload: dict[str, Any] = {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(record.created))
            + f".{int(record.msecs):03d}Z",
            "level": record.levelname.lower(),
            "logger": record.name,
            "event": record.getMessage(),
            "engine_version": ENGINE_VERSION,
        }
        payload.update(getattr(record, "fields", {}))
        return json.dumps(payload, ensure_ascii=False, separators=(",", ":"))


class TextFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        fields = getattr(record, "fields", {})
        extra = " ".join(f"{k}={v}" for k, v in fields.items())
        return f"{record.levelname.lower():<5} {record.name} {record.getMessage()} {extra}".rstrip()


def configure(fmt: str = "text", level: str = "INFO") -> None:
    """Configure the ``fairmark`` loggers (stderr)."""
    handler = logging.StreamHandler(sys.stderr)
    handler.setFormatter(JsonFormatter() if fmt == "json" else TextFormatter())
    root = logging.getLogger("fairmark")
    root.handlers[:] = [handler]
    root.setLevel(level.upper())
    root.propagate = False


def log_event(logger: logging.Logger, event: str, **fields: Any) -> None:
    """Log a structured event. Refuses fields that could carry financial data."""
    leaked = _FORBIDDEN_KEYS & fields.keys()
    if leaked:
        raise ValueError(f"log fields not allowed (financial data policy): {sorted(leaked)}")
    logger.info(event, extra={"fields": {k: v for k, v in fields.items() if v is not None}})
