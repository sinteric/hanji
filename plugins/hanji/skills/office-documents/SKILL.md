---
name: office-documents
description: Read, edit, fill or create Word, PowerPoint, Excel and Hancom files (.docx, .pptx, .xlsx, .hwpx) with the hanji MCP tools, keeping their formatting, pictures and comments. Use when the person asks to summarise, review, change, translate, fill in or produce an office file, or when a task's input or output is one.
---

# Office files with hanji

The `hanji` MCP server turns an office file into text you can read and edit
exactly; everything the text does not show (layout, formatting, pictures,
comments) is kept and put back on export. Its instructions carry the full
format: follow them. This skill is the workflow around the tools.

Formats: `.docx`, `.pptx`, `.xlsx`, `.hwpx`. A `.hwp` (HWP 5.0), `.doc`,
`.ppt` or `.xls` file is not supported: ask the person to save it as the
newer format first, rather than converting it another way.

## Paths

Give every path to the tools as an absolute path. The server's working
directory may be the plugin's own directory, not the project's.

## Workflow

1. `hanji_open` the file (or `hanji_new`, optionally from a template). Tell the
   person what the result lists under `neutralised` (macros, remote content,
   embedded objects: removed) and `surfaced` (comments, hidden text, tracked
   deletions, author metadata).
2. `hanji_read` before every edit. Large files come in parts: follow
   `outline` and `next`, and read by `lines`, `section` or `slides`. For a
   spreadsheet, read the structure, then a row window (`table` with `rows`, or
   `sheet` with `range`).
3. Change it:
   - small changes: `hanji_edit`, with `old` copied exactly from the read and
     unique in the whole text, and the `revision` you read;
   - large changes or a new document: `hanji_write` the whole text;
   - spreadsheet cells: `hanji_ops` range operations only.
   Check a large text first with `hanji_validate`.
4. A refusal changes nothing and says why (line, column, the allowed names, or
   how often `old` occurs). Fix that and retry; never resend it unchanged. If
   the revision is stale, read again.
5. `hanji_export` to a path the person chose, next to the original unless
   they said otherwise; do not overwrite the original without asking. If the
   export lists `surfaced` items, show them and export with
   `acknowledge_surfaced` only once the person agrees. For a Word file a person
   will review, offer `tracked_changes`.
6. If the person edits the exported file in Office or Hancom and wants you to
   continue, `hanji_reimport` it and read again.

`hanji_history` lists the revisions and diffs any two of them: use it to
show the person what changed.

## Leave alone

`<keep …/>` lines stand for pictures, charts, footnotes and other objects the
text does not model. Keep them as they are; you may move a line or delete it
when the person asks, never write a new one.

## Without the MCP server

If the `hanji_*` tools are missing, the server did not start: say so and
point the person to https://github.com/sinteric/hanji/tree/main/plugins/hanji
(install `hanji-mcp` with `cargo install --locked --git
https://github.com/sinteric/hanji hanji-mcp`, or set `HANJI_MCP_BIN`). Do not
fall back to unzipping and rewriting the XML yourself.
