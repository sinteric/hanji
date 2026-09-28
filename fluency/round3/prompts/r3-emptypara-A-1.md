You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items (a nested item is indented two spaces), `**bold**` and `*italic*`. A blank line only separates blocks; it is never a paragraph itself. Inside a paragraph, `<br/>` is a line break: the text goes on in the same paragraph. A line holding only `<pagebreak/>` is a page break. Besides Markdown, only the tags and markers described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. Colours, fonts, sizes, spacing, alignment, borders and shading cannot be written directly. A paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. A style name is written exactly as listed, spaces included.

### Tables

A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column, merged or not. Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column. A span over three columns is `|||`, with no space between the pipes.
- A merged area is a rectangle; for two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. The text of a merged cell is written once, in its top-left cell. An empty cell that is not merged is written with a space, `|  |`. `^^` never appears in the first row, and `||` never starts a row.

A table with a style has a line `{style="Name"}` directly before it, with no blank line between; the braces hold nothing but `style="Name"`, and nothing closes the table. A table without that line has the default table style.

Example ("서울" covers two rows, "합계" covers two columns; the table has the style Grid Table 4):

```
{style="Grid Table 4"}
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```

### Empty paragraphs

Documents use empty paragraphs for spacing, often several in a row and sometimes with a style. An empty paragraph with the default style is a line holding only `<p/>`; an empty paragraph with a paragraph style is a line holding only `<p style="Name"/>`.

- Each empty paragraph is its own line: three empty paragraphs are three such lines, even when they are identical. Consecutive empty paragraphs are on consecutive lines, with no blank line between them; a blank line separates the group from the blocks before and after it, as for any block.
- `<p/>` and `<p style="Name"/>` are single tags; there is no `</p>`, and `<p>` takes no other attribute. The tag stands alone on its line, never inside a line of text or a table cell.
- A blank line is not an empty paragraph, and neither is a line holding only `<br/>`.

Example (two empty paragraphs with the default style after the heading, and one in the style 좁은 간격 after the table):

```
# 알림

<p/>
<p/>

행사 일정은 다음과 같다.

| 구분 | 일시 |
|---|---|
| 1차 | 3월 |

<p style="좁은 간격"/>

문의는 총무과로 한다.
```

## Names available in this file

Paragraph styles:
- `기관명` — 문서 맨 위 가운데의 큰 기관 이름
- `항목` — 번호(1., 2., …)가 붙은 공문 본문 항목
- `붙임` — 붙임 목록 줄
- `발신 명의` — 가운데 정렬된 크고 굵은 기관장 명의
- `결재란` — 문서 끝의 결재·시행 정보 줄 (작은 글씨)
- `좁은 간격` — 표 앞뒤에 두는 높이가 낮은 빈 줄
- `붙임 제목` — 붙임 쪽 맨 위의 굵은 제목 줄

Table styles:
- `표 눈금` — 모든 칸에 가는 검은 실선
- `눈금 표 4` — 회색 머리글 행, 회색 줄무늬 행

## The file: letter-1.hj.md

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---

<div style="기관명">○○시교육청</div>

<p/>
<p/>

수신: 수신자 참조

(경유)

제목: 2027학년도 교원 직무 연수 운영 계획 안내

<p/>

<div style="항목">1. 관련: ○○시교육청 교원인사과-4521(2026. 12. 3.)</div>

<div style="항목">2. 2027학년도 교원 직무 연수를 아래와 같이 운영하오니, 각 학교에서는 소속 교원이 기한 안에 신청하도록 안내하여 주시기 바랍니다.</div>

가. 연수 기간: 2027. 3. 2. ~ 2027. 11. 30.

나. 연수 대상: 관내 초·중·고 교원 (과정별 정원 있음)

다. 연수 과정: 4개 과정 (과정별 세부 일정은 붙임 1 참고)

<p style="좁은 간격"/>

{style="표 눈금"}
| 과정명 | 대상 | 기간 | 정원 | 학점 |
|---|---|---|---|---|
| 디지털 수업 설계 | 초·중등 교원 | 3~5월 | 120명 | 2 |
| 학생 상담 실무 | 담임 교원 | 4~6월 | 80명 | 2 |
| 학교 안전 관리 | 전 교원 | 연중 | 200명 | 1 |
| 교육과정 평가 | 중·고등 교원 | 9~11월 | 60명 | 2 |

<p style="좁은 간격"/>

라. 신청 방법: 교원연수 누리집에서 개인별로 신청 (2027. 2. 10.까지). 누리집 신청이 어려운 교원은 붙임 2의 신청서를 학교 연수 담당자에게 제출합니다.

마. 이수 기준: 출석 80% 이상, 과정 평가 60점 이상

바. 연수비: 전액 교육청 부담 (숙박이 필요한 과정은 숙박비 포함)

사. 유의 사항: 같은 기간에 두 과정 이상을 신청할 수 없으며, 신청 후 취소는 개강 7일 전까지 누리집에서 합니다.

<div style="항목">3. 각 학교에서는 연수 대상 교원의 수업 결손이 생기지 않도록 시간표를 조정하고, 연수 기간 중 복무는 출장으로 처리하여 주시기 바랍니다.</div>

<div style="항목">4. 연수 결과는 개인별 연수 이력에 반영되며, 미이수자는 다음 연도에 같은 과정을 신청할 수 없습니다.</div>

<div style="항목">5. 올해부터 디지털 수업 설계 과정은 수업 나눔 발표를 이수 요건에 포함하며, 발표 자료는 연수 종료 후 누리집의 수업 나눔 게시판에 공개합니다. 학교에서는 발표 교원의 수업 공개 일정을 미리 협의하여 주시기 바랍니다.</div>

<div style="항목">6. 연수와 관련한 문의는 교원인사과 연수 담당 장학사(031-000-2345)에게 하여 주시기 바랍니다.</div>

<p/>
<p/>

<div style="붙임">붙임 1. 연수 과정별 세부 일정 1부.</div>

<div style="붙임">2. 연수 신청서 서식 1부. 끝.</div>

<p/>
<p/>
<p/>

<div style="발신 명의">○○시교육감</div>

<p/>
<p/>

<div style="결재란">장학사 김민지 / 장학관 박성훈 / 교원인사과장 최현우</div>

<div style="결재란">시행: 교원인사과-4602 (2026. 12. 15.) / 접수:</div>

<div style="결재란">우 12345 ○○시 ○○로 100 / 전화 031-000-2345 / 전송 031-000-2399 / 공개</div>

<p/>
<p/>
<p/>
<p/>
<p/>

<div style="붙임 제목">붙임 1. 연수 과정별 세부 일정</div>

<p/>

{style="눈금 표 4"}
| 과정명 | 차시 | 일정 | 장소 |
|---|---|---|---|
| 디지털 수업 설계 | 1~4차시 | 3. 16. ~ 4. 6. (매주 월) | 교육연수원 301호 |
| ^^ | 5~8차시 | 4. 13. ~ 5. 4. (매주 월) | ^^ |
| 학생 상담 실무 | 1~6차시 | 4. 7. ~ 5. 12. (매주 화) | 교육연수원 205호 |
| 학교 안전 관리 | 1~2차시 | 온라인 상시 | 원격연수 누리집 |
| 교육과정 평가 | 1~6차시 | 9. 8. ~ 10. 13. (매주 화) | 교육연수원 302호 |
| ^^ | 7~8차시 | 10. 20. ~ 10. 27. (매주 화) | ^^ |

<p/>

※ 차시별 세부 내용은 연수 시작 2주 전에 누리집에 게시합니다.

※ 연수 시간은 모두 16:30 ~ 18:30이며, 첫 차시에는 30분 일찍 와서 등록을 마쳐야 합니다.

※ 천재지변 등으로 연수를 할 수 없으면 원격 연수로 바꾸고, 바뀐 내용은 누리집과 문자 메시지로 알립니다.

※ 원격 연수는 개인 계정으로 접속하며, 대리 수강이 확인되면 이수를 취소합니다.

<p/>

연수 장소 안내

<p style="좁은 간격"/>

{style="표 눈금"}
| 장소 | 주소 | 교통 |
|---|---|---|
| 교육연수원 | ○○시 ○○구 연수로 45 | 지하철 2호선 연수원역 3번 출구, 도보 5분 |
| 원격연수 누리집 | 누리집 주소는 학교로 따로 안내 | 개인 인증서로 접속 |

<p style="좁은 간격"/>

※ 교육연수원에는 주차 공간이 부족하니 대중교통을 이용하여 주시기 바랍니다.

※ 장애가 있는 교원은 신청서 비고란에 필요한 지원(수어 통역, 이동 지원 등)을 적어 주시면 준비하겠습니다.

<p/>
<p/>
<p/>
<p/>
<p/>

<div style="붙임 제목">붙임 2. 연수 신청서</div>

<p/>

{style="표 눈금"}
| 항목 | 내용 |
|---|---|
| 성명 |  |
| 소속 학교 |  |
| 담당 교과 |  |
| 신청 과정 |  |
| 희망 차시 |  |
| 연락처 |  |
| 비고 |  |

<p/>

개인정보 수집·이용 동의: 연수 운영을 위하여 성명, 소속 학교, 연락처를 수집하며, 연수 종료 후 5년 동안 보관합니다. 동의하지 않으면 연수를 신청할 수 없습니다.

동의 여부: [ ] 동의함 [ ] 동의하지 않음

<p/>

위와 같이 2027학년도 교원 직무 연수를 신청합니다.

<p/>

2027년 __월 __일

신청인: ________ (서명)

<p/>
<p/>

○○학교장 확인: ________ (직인)

※ 학교 연수 담당자는 제출받은 신청서를 모아 2027. 2. 12.까지 공문으로 보내 주시기 바랍니다.
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### letter1-w (write)

Write a new document with exactly these blocks, in this order, and nothing else: the line "○○시교육청" in the style for the organisation name at the top; two empty paragraphs with the default style; the paragraphs "수신: 관내 초·중·고등학교장" and "제목: 2027학년도 교원 연수 신청 기간 연장 안내", both with the default style; one empty paragraph with the default style; the paragraphs "1. 교원 연수 신청 기간을 2027. 2. 17.까지 연장합니다." and "2. 연장 기간에도 과정별 정원을 넘으면 신청을 마감합니다.", each in the style for numbered items of the letter, with one empty paragraph in the low spacing style between them; two empty paragraphs with the default style; the attachment-list line "붙임 1. 변경된 연수 일정 1부. 끝."; three empty paragraphs with the default style; the head-of-organisation line "○○시교육감".

Start the new file with this front matter:

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---
````

### letter1-e1 (edit)

There are three empty paragraphs directly before the line "○○시교육감". Keep two of them (delete one). Change nothing else.

### letter1-e2 (edit)

In 붙임 1, the empty paragraph directly after the table should be in the low spacing style used around tables. Change nothing else.

### letter1-e3 (edit)

Replace the empty paragraphs before "○○시교육감" with exactly 24pt of space above that line.

### letter1-e4 (edit)

붙임 2 is pushed to a new page by the run of empty paragraphs directly before its title line "붙임 2. 연수 신청서". Replace that whole run with one page break. The run before "붙임 1. 연수 과정별 세부 일정" stays as it is. Change nothing else.

## Tasks that cannot be done

Do only what the syntax documentation and the names above can express. If an edit task asks for something they cannot express, do not approximate it, and do not invent a name, tag or attribute: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its edits.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]},
  {"task_id": "<id of an edit task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```
