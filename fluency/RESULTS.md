# Fluency test results

## Round 1 — 2026-09-28

**Subjects:** Claude Opus (`claude-opus-5-5`) and Claude Sonnet (`claude-sonnet-5`). Each did all 18 blind
units: 3 decisions × A/B × 3 Korean seeds × 4 tasks = 72 tasks per model, 144 answers in all. No tools. One
fix round on validator errors was allowed; it was never needed.

Answers: `runs/<unit>/opus-first.json` and `runs/<unit>/sonnet-first.json`. Re-score them with
`python3 score.py <unit> <file>`.

### Validity and landing

Both models: **72/72 valid and landed on the first try, under every candidate.** No flags on any answer.

| model | merge A | merge B | styleattr A | styleattr B | slides A | slides B |
|---|---|---|---|---|---|---|
| Opus | 12/12 | 12/12 | 12/12 | 12/12 | 12/12 | 12/12 |
| Sonnet | 12/12 | 12/12 | 12/12 | 12/12 | 12/12 | 12/12 |

### Size

Answer size in characters, summed over the 12 tasks of each cell (`chars` from the scorer: the length of a
written file, or the sum of `old` + `new` of an edit), A / B:

| model | merge | styleattr | slides |
|---|---|---|---|
| Opus | 3,435 / 2,893 | 2,527 / 2,527 | 2,410 / 1,887 |
| Sonnet | 6,689 / 3,383 | 2,578 / 2,551 | 2,837 / 2,256 |

### The tempting styling request

On the "make it red, bold and large" task (`document1-e2`), both models chose the named `Alert Box` style,
under both `style` and `class`. Neither wrote CSS anywhere in the styleattr answers.

### Decisions

| decision | winner | why |
|---|---|---|
| merged cells | **B**: local markers `^^` / `\|\|` in pipe tables | smaller (Opus −16%, Sonnet −49%); nothing counted (§5.1) |
| style attribute | **A**: `style="Name"` | a tie on every measure; kept by default |
| Presentation | **B**: Slidev-style | smaller (Opus −22%, Sonnet −20%); plain Markdown lines, no closing tags |

### Limits

- **The tasks hit the ceiling.** The briefs spelled out every merge, and the style list offered a style
  that matched the tempting request. Correctness did not discriminate between candidates, so the winners
  rest on size and design fit, not on errors avoided.
- Only two Claude models. GPT and Gemini were not run (no API keys), so §6's "at least Claude, GPT,
  Gemini" is not met yet.
- Small files (seeds of 662–1,305 characters), the fix round never exercised, tokens measured as
  characters.

### Next round

- Larger files.
- Merges implied by the content, not spelled out in the brief.
- A tempting styling request with no matching named style to escape to.
- Weaker models (Haiku).
- GPT (with `apply_patch`) and Gemini when keys exist.
