"""Every Python snippet in the user-facing docs pages must run.

Pages that need network access or API keys (function-calling tutorials) are excluded.
"""

import re
from pathlib import Path

import pytest

DOCS = Path(__file__).resolve().parents[2] / "docs"
PAGES = [
    DOCS / "index.md",
    DOCS / "getting-started" / "quickstart.md",
    *sorted((DOCS / "methods").glob("*.md")),
]
FENCE = re.compile(r"```python\n(.*?)```", re.DOTALL)


@pytest.mark.parametrize("page", PAGES, ids=lambda p: str(p.relative_to(DOCS)))
def test_python_snippets_run(page: Path) -> None:
    namespace: dict[str, object] = {}
    for snippet in FENCE.findall(page.read_text()):
        exec(compile(snippet, str(page), "exec"), namespace)
