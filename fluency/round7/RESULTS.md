# Fluency test results, round 7

## Round 7 — (to be filled after the runs)

**Subjects:** Claude Sonnet 5.5 (`claude-sonnet-5-5`, files `sonnet55-*.json`), Claude Haiku 5.5
(`claude-haiku-5-5`, files `haiku55-*.json`), Claude Sonnet 4.6 (`claude-sonnet-4-6`, files `sonnet46-*.json`).
Blind, each unit in a fresh `claude -p` conversation whose only tool was Read (the prompt read from its own file,
[README.md](README.md#runs)). 18 units, 114 tasks × 3 models. One fix round on validator errors.

Re-score any answer with `python3 score.py <unit> <file>`; `python3 summarize.py` reproduces every table here.
The self-test must pass (`python3 build.py && python3 selftest.py` ends with `SELFTEST PASSED`).

Place the owner's run tables below. Until then, `python3 summarize.py` prints the empty per-model grid.

### What round 7 decides

Three keep-reducing syntaxes, each by a fluency test first ([CANDIDATES.md](CANDIDATES.md)):

| decision | A | B |
|---|---|---|
| `breaks` | inline `<pagebreak/>` / `<columnbreak/>` inside the paragraph | `<br type="page"/>` / `<br type="column"/>` (reusing `<br/>`) |
| `shapes` | `<shape name="…" anchor="B2:D5">text</shape>` and `<group name="…">…</group>` | `<shape name="…" anchor="B2:D5" text="…"/>` with `group="Name"` (xlsx line style) |
| `databar` | `<conditional-format type="dataBar" min="min" max="max" fill="#638EC6"/>` | `<conditional-format type="dataBar" min-type="min" max-type="num" max="100" color="#638EC6"/>` |

### Landed

*(tables from `python3 summarize.py` after the runs)*
