---
name: office-documents
description: Read, edit, fill or create Word, PowerPoint, Excel and Hancom files (.docx, .pptx, .xlsx, .hwpx) with the hanji command line, keeping their formatting, pictures and comments. Use when the person asks to summarise, review, change, translate, fill in or produce an office file, or when a task's input or output is one.
---

# Office files with hanji

The `hanji` command turns an office file into text you can read and edit
exactly; everything the text does not show (layout, formatting, pictures,
comments) is kept and put back on export. This skill is the workflow around
its commands.

Formats: `.docx`, `.pptx`, `.xlsx`, `.hwpx`. A `.hwp` (HWP 5.0), `.doc`,
`.ppt` or `.xls` file is not supported: ask the person to save it as the
newer format first, rather than converting it another way.

## Running it

Run hanji through this skill's launcher, `scripts/hanji` in the directory
that holds this `SKILL.md`:

```sh
sh "${CLAUDE_SKILL_DIR}/scripts/hanji" guide
```

Claude Code fills in `${CLAUDE_SKILL_DIR}`. If you see the variable's name
instead of a path, put the absolute path of this file's directory in its
place. Below, `hanji` stands for that whole command. The launcher runs a
`hanji` on `PATH` if there is one; otherwise its first run downloads the
release binary for this machine (a few seconds; it says so on stderr), and
later runs start at once.

- Run `hanji guide` once before the first edit: it prints the format (front
  matter, styles, tables, slides, spreadsheet operations). Follow it.
- Add `--json` to any command (before or after the subcommand) for
  machine-readable output: `doc_id`, `revision`, the `report`, and
  `{"error": …}` on a refusal. For `read`, plain output is easier to copy
  `old` from: the text alone is on stdout, the revision, line range, outline
  and where to read on are on stderr.
- Give every file path as an absolute path.
- Run every command from the same working directory, the project's.
  Documents and their revisions are kept in `.hanji/` there (or in
  `$HANJI_STORE`, or `--store DIR`); `hanji list` shows them. `.hanji/` is
  hanji's working store, not part of the person's project: do not commit it.
- Pass text and JSON through files or stdin, never as shell-quoted
  arguments: write it to a file with your file-writing tool and give the
  path, or use a quoted heredoc (`<<'HANJI'`) with `-` for stdin. Markers,
  quotes, `$`, backslashes and line breaks then arrive exactly.

## Workflow

1. `hanji --json open /abs/file.docx`, or `hanji --json new document`
   (`presentation`, `spreadsheet`; `--format hwpx`, `--template
   /abs/template.docx`). Note the `doc_id`. Tell the person what
   `report.neutralised` lists (macros, remote content, embedded objects:
   removed) and what `report.surfaced` lists (comments, hidden text, tracked
   deletions, author metadata).
2. `hanji read DOC` before every edit, and note the revision it reports.
   Large files come in parts: follow the outline and `read on:`, and read by
   `--lines 120:180`, `--section "heading text"` or `--slides 3:5`. For a
   spreadsheet, read the structure, then a row window: `--table Sales --rows
   2:101`, or `--sheet 매출 --range A1:F50`.
3. Change it, always against the revision you read (`--rev N`):
   - small changes: `hanji edit DOC --rev N --edits /abs/edits.json`, a JSON
     list of `{"old": …, "new": …}` applied in order (or `--edits -` with the
     list on stdin; `--old-file` and `--new-file` for one edit). Copy each
     `old` exactly from the read, and make it unique in the whole text;
   - large changes or a new document: `hanji write DOC --rev N
     /abs/text.md` (or `-` for stdin), the whole text;
   - spreadsheet cells: `hanji ops DOC --rev N /abs/ops.json`, range
     operations only.
   Check a large text first with `hanji validate /abs/text.md --doc DOC`.
   If the result says the text was stored in canonical form, read again
   before the next edit.
4. A refusal changes nothing, exits with status 1 and says why (line,
   column, the allowed names, or how often `old` occurs). Fix that and retry;
   never resend it unchanged. If the revision is stale, read again.
5. For a presentation, check the look: `hanji preview DOC --format png --out
   /abs/scratch/dir` writes one PNG per slide (`<doc>-r<rev>-slide-<n>.png`)
   and says which fonts it substituted. Open the slides you need with your
   file or image reader. Without `--out` the files go next to the original,
   so pass a scratch directory. Mention substituted fonts that change the
   look.
   For a Word or HWPX document, the same command writes one PNG per page
   (`<doc>-r<rev>-page-<n>.png`). These page renderers are experimental:
   inspect tables, page breaks, headers/footers and any rendering diagnostics;
   do not treat a successful render as native Word/Hancom fidelity.
   For a spreadsheet, `hanji preview DOC --sheet NAME --range A1:H40
   --format png --out /abs/scratch/dir` writes one bounded worksheet window.
   `--sheet-index N` is 1-based; the default is the first visible worksheet
   and A1:L40. File paths are accepted and stay unstored. Formula results are
   stored caches only; missing caches show #UNEVALUATED. Use `--json` for
   cell cache status, fonts and unsupported-feature diagnostics. A range
   cutting a merged cell is refused with the complete merge address. Mention
   omitted charts, images, conditional formatting and print layout when those
   diagnostics occur.
6. `hanji export DOC /abs/out.docx` to a path the person chose, next to the
   original unless they said otherwise; do not overwrite the original
   without asking. If export is refused over surfaced items (comments,
   hidden text, tracked deletions, metadata), show them and export again
   with `--acknowledge-surfaced` only once the person agrees. For a Word
   file a person will review, offer `--tracked-changes`.
7. If the person edits the exported file in Office or Hancom and wants you to
   continue, `hanji reimport DOC /abs/edited.docx` and read again.

`hanji history DOC` lists the revisions and `hanji diff DOC 1 4` diffs any
two of them: use them to show the person what changed.

## Leave alone

`<keep …/>` lines stand for pictures, charts, footnotes and other objects the
text does not model. Keep them as they are; you may move a line or delete it
when the person asks, never write a new one.

## If the launcher fails

The launcher says on stderr, in one line, what failed and how to fix it.
Show that line to the person as it is, and do not work around it. Most often
the first run could not download the binary. For HTTP 404, the plugin's
release or the requested asset is unavailable: use a matching source
checkout with `HANJI_BIN` as described in the plugin README, or a plugin
version with published release assets. For connection failures, the network
may be off (Codex's default sandbox) or the release host blocked. If your
tool can rerun a command with network access, ask the person and rerun it
once that way; otherwise the fixes are theirs: allow github.com and
release-assets.githubusercontent.com, set `HANJI_BIN` to a `hanji` binary,
or install hanji on `PATH` (`cargo install --locked --git
https://github.com/sinteric/hanji hanji-cli`).

Point the person to https://github.com/sinteric/hanji/tree/main/plugins/hanji
for details. Do not fall back to unzipping and rewriting the XML yourself.
