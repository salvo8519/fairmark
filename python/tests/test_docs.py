"""Every Python snippet in the user-facing docs pages must run.

Pages that need network access or API keys (function-calling tutorials) are excluded.
"""

import re
from pathlib import Path

import pytest

DOCS = Path(__file__).resolve().parents[2] / "docs"
PAGES = [
    page
    for lang in ("en", "it")
    for page in [
        DOCS / lang / "index.md",
        DOCS / lang / "getting-started" / "quickstart.md",
        *sorted((DOCS / lang / "methods").glob("*.md")),
    ]
    if page.exists()
]
FENCE = re.compile(r"```python\n(.*?)```", re.DOTALL)


@pytest.mark.parametrize("page", PAGES, ids=lambda p: str(p.relative_to(DOCS)))
def test_python_snippets_run(page: Path) -> None:
    namespace: dict[str, object] = {}
    for snippet in FENCE.findall(page.read_text()):
        exec(compile(snippet, str(page), "exec"), namespace)
