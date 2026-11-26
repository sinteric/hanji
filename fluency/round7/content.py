"""Round 7 seeds and tasks, written once in a form that belongs to neither candidate.

Seeds are Korean office files modelled on the corpus samples that today produce these keeps (docs/keep-census.md):
docx `break` in tdf124637_sectionMargin.docx (a `w:br type=column` before repeated "test") and tdf154481.docx
(inline `w:br type=page` mixed with heading text); xlsx `shape` in simple-monthly-budget.xlsx (the "Data Entry
Tip" callout at B25:D27) and WithDrawing.xlsx (TextBox 8); xlsx `group` in ExcelWithAttachments.xlsm (Group 1 of
a rectangle and a text box); xlsx `conditional-format` dataBar in simple-monthly-budget.xlsx (E4, min num 0, max
a named formula, Office blue). Repeated text is on purpose so an edit must hit one span.
"""
import copy

PAGE, COL, BR = 'page', 'col', 'br'

DOC_FM = {'type': 'document', 'format': 'docx', 'template': 'org/report', 'schema': '1'}
XLS_FM = {'type': 'spreadsheet', 'format': 'xlsx', 'schema': '1'}

DECISIONS = ('breaks', 'shapes', 'databar')


def cp(x):
    return copy.deepcopy(x)


def I(*parts):
    """Build inlines: strings, or PAGE / COL / BR."""
    out = []
    for p in parts:
        if p in (PAGE, COL, BR):
            out.append((p,))
        else:
            out.append(('t', p))
    return out


def H(level, *parts):
    return {'k': 'h', 'level': level, 'text': I(*parts)}


def P(*parts):
    return {'k': 'p', 'style': None, 'text': I(*parts)}


def D(style, *parts):
    return {'k': 'p', 'style': style, 'text': I(*parts)}


def UL(*parts):
    return {'k': 'ul', 'text': I(*parts)}


def UL2(*parts):
    return {'k': 'ul2', 'text': I(*parts)}


def PB():
    return {'k': 'pagebreak'}


def KEEP(i, kind, summary):
    return {'k': 'keep', 'id': i, 'kind': kind, 'summary': summary}


def TB(style, header, rows):
    return {'k': 'table', 'style': style, 'header': list(header), 'rows': [list(r) for r in rows]}


def plain(inlines):
    return ''.join(t if k == 't' else '' for (k, *rest), t in
                   ((p, p[1] if p[0] == 't' else '') for p in inlines))


def set_text(block, *parts):
    block['text'] = I(*parts)


def find_p(blocks, needle):
    for b in blocks:
        if b.get('k') in ('p', 'h', 'ul', 'ul2') and needle in plain(b['text']):
            return b
    raise KeyError(needle)


# ====================================================================== breaks

BREAK_NAMES = {
    'paragraph_styles': [('Note', '작은 회색 참고 문단'), ('표 제목', '표 바로 위의 제목 줄')],
    'table_styles': [('표 눈금', '모든 칸에 가는 검은 실선')],
}

# Seed 1: quarterly sales report. Corpus: tdf154481 page-break mixed into heading/paragraph text;
# tdf124637 column-break before a repeated word. Repeated "전년 대비 +12%" and "서울 14곳".
B1 = [
    H(1, '3분기 영업 보고'),
    P('영업본부는 2026년 3분기 실적을 다음과 같이 보고합니다. 매출은 전년 대비 +12% 증가했고, 신규 고객 34곳 중 21곳이 수도권입니다.'),
    H(2, '핵심 지표'),
    D('표 제목', '표 1. 지역별 매출 (억 원)'),
    TB('표 눈금',
       ['지역', '지점', '매출', '증감'],
       [['서울', '강남', '120', '+12%'],
        ['서울', '종로', '95', '+8%'],
        ['부산', '해운대', '88', '+4%'],
        ['합계', '—', '303', '+9%']]),
    P('수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.', PAGE, '첨부 1의 지점별 표는 10월 말 확정치이며, 잠정치와 다를 수 있습니다.'),
    H(2, '지역별 현황'),
    P('서울 14곳, 경기 7곳, 인천 0곳.', COL, '부산 5곳, 대구 3곳, 광주 2곳, 그 외 3곳.'),
    P('신규 고객 중 서울 14곳은 강남·종로에 집중되어 있고, 지방 13곳 중 부산이 5곳입니다.'),
    UL('전년 대비 +12% 증가는 수도권 대형 거래 3건이 이끌었습니다.'),
    UL('지방은 전년 대비 +4%로, 부산 해운대 지점이 가장 컸습니다.'),
    UL2('해운대 지점 매출 88억은 잠정치입니다.'),
    D('Note', '표의 +12%와 본문의 전년 대비 +12%는 같은 내부 집계입니다.'),
    KEEP('k3', 'drawing', '조직도, 상자 5개'),
    PB(),
    H(1, '부록'),
    P('첨부 1. 지점별 매출 원표는 영업지원팀에 있습니다. 서울 14곳의 명단은 별도 파일입니다.'),
    P('문의는 영업본부 기획팀 (☎ 02-000-1234)으로 하시기 바랍니다.'),
]

# Seed 2: ministry work report. Corpus: tdf154481 page break in a heading-like line; repeated "다음".
B2 = [
    H(1, '2026년 하반기 고용노동 업무보고'),
    P('장관 주재 업무보고입니다. 핵심은 청년 일자리, 산업안전, 노동시간 단축 세 가지이며, 수치는 내부 집계 기준입니다.'),
    H(2, '1. 청년 일자리'),
    P('청년 고용률은 45.2%로 전년 동기 44.1%보다 1.1%p 올랐습니다. 일경험 프로그램 참여자는 12,400명입니다.'),
    P('다음 단계로 대학일자리+센터 30곳을 추가 지정합니다.', PAGE, '지정 공고는 11월 둘째 주에 나갑니다.'),
    D('표 제목', '표 1. 일경험 프로그램 (명)'),
    TB('표 눈금',
       ['구분', '상반기', '하반기', '합계'],
       [['공공', '3,200', '3,400', '6,600'],
        ['민간', '2,800', '3,000', '5,800'],
        ['합계', '6,000', '6,400', '12,400']]),
    H(2, '2. 산업안전'),
    P('사고사망자는 3분기 누적 457명으로 전년 동기 443명보다 14명 늘었습니다. 50인 미만 사업장에서 26명이 늘었습니다.'),
    P('다음 단계로 초소규모 사업장 점검 횟수를 두 배로 늘립니다.', COL, '업종은 지붕·축사, 벌목, 질식 위험 작업부터입니다.'),
    P('다음 단계 일정은 12월 초 관계부처 회의에서 확정합니다. 다음 주 내부 검토가 있습니다.'),
    UL('50인 이상 사업장은 12명 감소, 50인 미만은 26명 증가.'),
    UL('한파·온열 등 새로운 위험 요인을 점검 항목에 넣습니다.'),
    D('Note', '「다음 단계」는 각 과제의 후속 조치이며, 문단마다 가리키는 과제가 다릅니다.'),
    PB(),
    H(2, '3. 노동시간'),
    P('실노동시간 단축 지원법 입법은 3월, 패키지 입법은 5월을 목표로 합니다.'),
    P('다음 단계로 야간노동자 실태조사를 6월까지 마칩니다.'),
    KEEP('k1', 'drawing', '조직도, 본부 5개'),
    P('문의는 기획조정실 (☎ 044-000-1234)입니다.'),
]

# Seed 3: product-launch proposal. Repeated "출시일 11월 20일" and "핵심 기능 3가지".
B3 = [
    H(1, '한빛 노트 출시 제안서'),
    P('제품기획팀은 한빛 노트의 11월 출시를 제안합니다. 핵심 기능 3가지는 공동 편집, 오프라인 저장, 한글 교정입니다.'),
    H(2, '일정'),
    P('출시일 11월 20일을 기준으로 마케팅은 11월 6일부터, CS 교육은 11월 13일부터입니다.', PAGE,
      '인쇄물 마감은 출시일 11월 20일 일주일 전인 11월 13일입니다.'),
    D('표 제목', '표 1. 채널별 목표'),
    TB('표 눈금',
       ['채널', '1주', '4주', '비고'],
       [['자사몰', '1,200', '4,800', '얼리버드'],
        ['오픈마켓', '800', '3,200', '쿠폰'],
        ['합계', '2,000', '8,000', '—']]),
    H(2, '메시지'),
    P('왼쪽 칼럼은 기업 고객용입니다.', COL, '오른쪽 칼럼은 개인 고객용입니다. 핵심 기능 3가지를 같은 순서로 적습니다.'),
    P('핵심 기능 3가지를 포스터 상단에도 반복합니다. 출시일 11월 20일을 모든 인쇄물에 넣습니다.'),
    UL('공동 편집은 최대 20명.'),
    UL('오프라인 저장은 문서당 200쪽.'),
    UL('한글 교정은 맞춤법과 띄어쓰기.'),
    D('Note', '출시일 11월 20일은 이사회가 승인한 날짜이며, 바꾸려면 이사회를 다시 엽니다.'),
    KEEP('k2', 'drawing', '제품 렌더링, 각도 3개'),
    PB(),
    H(1, '첨부'),
    P('첨부 1. 가격표. 첨부 2. 경쟁 제품 비교. 핵심 기능 3가지의 상세 설명은 별도 위키에 있습니다.'),
]


def b1_add(d):
    """Add a page break in the opening paragraph, after the first sentence (the +12% sentence stays)."""
    p = find_p(d, '영업본부는 2026년 3분기')
    set_text(p, '영업본부는 2026년 3분기 실적을 다음과 같이 보고합니다. ', PAGE,
             '매출은 전년 대비 +12% 증가했고, 신규 고객 34곳 중 21곳이 수도권입니다.')


def b1_move(d):
    """Move the column break from the 서울/부산 line to after 인천 0곳 → after 대구 3곳."""
    p = find_p(d, '인천 0곳')
    set_text(p, '서울 14곳, 경기 7곳, 인천 0곳. 부산 5곳, 대구 3곳.', COL, '광주 2곳, 그 외 3곳.')


def b1_remove(d):
    """Remove the inline page break in the 수도권 매출 paragraph; text stays one paragraph."""
    p = find_p(d, '첨부 1의 지점별 표는 10월 말')
    set_text(p, '수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다. 첨부 1의 지점별 표는 10월 말 확정치이며, 잠정치와 다를 수 있습니다.')


def b2_add(d):
    p = find_p(d, '청년 고용률은 45.2%')
    set_text(p, '청년 고용률은 45.2%로 전년 동기 44.1%보다 1.1%p 올랐습니다. ', PAGE,
             '일경험 프로그램 참여자는 12,400명입니다.')


def b2_move(d):
    p = find_p(d, '초소규모 사업장')
    set_text(p, '다음 단계로 초소규모 사업장 점검 횟수를 두 배로 늘립니다. 업종은 지붕·축사, 벌목, ', COL, '질식 위험 작업부터입니다.')


def b2_remove(d):
    p = find_p(d, '지정 공고는 11월')
    set_text(p, '다음 단계로 대학일자리+센터 30곳을 추가 지정합니다. 지정 공고는 11월 둘째 주에 나갑니다.')


def b3_add(d):
    p = find_p(d, '제품기획팀은 한빛 노트')
    set_text(p, '제품기획팀은 한빛 노트의 11월 출시를 제안합니다. ', PAGE,
             '핵심 기능 3가지는 공동 편집, 오프라인 저장, 한글 교정입니다.')


def b3_move(d):
    p = find_p(d, '왼쪽 칼럼은 기업')
    set_text(p, '왼쪽 칼럼은 기업 고객용입니다. 오른쪽 칼럼은 개인 고객용입니다.', COL, ' 핵심 기능 3가지를 같은 순서로 적습니다.')


def b3_remove(d):
    p = find_p(d, '인쇄물 마감은')
    set_text(p, '출시일 11월 20일을 기준으로 마케팅은 11월 6일부터, CS 교육은 11월 13일부터입니다. '
             '인쇄물 마감은 출시일 11월 20일 일주일 전인 11월 13일입니다.')


B1_WRITE = [
    H(1, '주간 영업 메모'),
    P('강남 지점 매출은 잠정치 12억입니다.', PAGE, '종로 지점은 같은 주 9억입니다.'),
    P('서울 안건.', COL, '부산 안건.'),
    PB(),
    P('다음 회의는 금요일 10시입니다.'),
]

B2_WRITE = [
    H(1, '산업안전 주간 메모'),
    P('이번 주 점검은 50인 미만 3곳입니다.', PAGE, '결과는 금요일에 올립니다.'),
    P('지붕·축사.', COL, '벌목.'),
    P('다음 단계 안내는 별도 공지입니다.'),
]

B3_WRITE = [
    H(1, '출시 당일 공지'),
    P('한빛 노트 판매를 시작합니다.', PAGE, '자사몰 쿠폰은 오늘 자정까지입니다.'),
    P('기업 고객 안내.', COL, '개인 고객 안내.'),
]


BREAKS = {
    1: dict(stem='report', fm=DOC_FM, names=BREAK_NAMES, doc=B1,
            write_instruction=(
                'Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 '
                '"주간 영업 메모"; a default-style paragraph "강남 지점 매출은 잠정치 12억입니다." then an inline page break then '
                '"종로 지점은 같은 주 9억입니다."; a default-style paragraph "서울 안건." then an inline column break then '
                '"부산 안건."; a page break between paragraphs; a default-style paragraph "다음 회의는 금요일 10시입니다."'),
            write_doc=B1_WRITE,
            read=('report1-q1',
                  'In the file, the paragraph that reports 수도권 매출 and 지방 증감 has an inline page break. '
                  'Quote the five words immediately after that page break, then the five words immediately after '
                  'the inline column break in the 지역별 현황 paragraph. Answer as `ANSWER: <page-break words> | '
                  '<column-break words>`.'),
            read_expect='ANSWER: 첨부 1의 지점별 표 | 부산 5곳, 대구',
            edits=[
                ('report1-e1',
                 'In the opening paragraph (the one that begins 영업본부는), insert an inline page break after the first '
                 'sentence (after "보고합니다."). Do not change the other page break, and do not touch the other '
                 '"전년 대비 +12%" sentence.',
                 b1_add),
                ('report1-e2',
                 'In the paragraph that lists 서울 14곳, 경기 7곳, 인천 0곳 and then 부산 5곳, move the inline column break so '
                 'it falls after "대구 3곳." instead of after "인천 0곳.". Keep every city count. Do not edit the other '
                 'paragraph that also says 서울 14곳.',
                 b1_move),
                ('report1-e3',
                 'Remove the inline page break from the paragraph that begins 수도권 매출은 전년 대비 +12%. Keep that '
                 'paragraph as one paragraph (the two stretches become one, with a space where the break was). '
                 'Do not remove the page break between paragraphs before 부록, and do not edit the opening paragraph '
                 'that also says 전년 대비 +12%.',
                 b1_remove),
            ],
            refuse=('report1-r',
                    'Insert a next-page section break before the 부록 heading so that heading starts a new section '
                    'with 2.5 cm margins and two newspaper columns for the rest of the file.',
                    'a section break and a change of margins or column count cannot be written')),
    2: dict(stem='ministry', fm=DOC_FM, names=BREAK_NAMES, doc=B2,
            write_instruction=(
                'Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 '
                '"산업안전 주간 메모"; a default-style paragraph "이번 주 점검은 50인 미만 3곳입니다." then an inline page break then '
                '"결과는 금요일에 올립니다."; a default-style paragraph "지붕·축사." then an inline column break then "벌목."; a '
                'default-style paragraph "다음 단계 안내는 별도 공지입니다."'),
            write_doc=B2_WRITE,
            read=('ministry2-q1',
                  'The file uses the words 다음 단계 in several paragraphs. Which 다음 단계 paragraph has an inline page '
                  'break, and which has an inline column break? Answer as `ANSWER: page: <the words after the page '
                  'break, up to the period> | column: <the words after the column break, up to the period>`.'),
            read_expect='ANSWER: page: 지정 공고는 11월 둘째 주에 나갑니다 | column: 업종은 지붕·축사, 벌목, 질식 위험 작업부터입니다',
            edits=[
                ('ministry2-e1',
                 'In the paragraph that begins 청년 고용률은 45.2%, insert an inline page break after the first sentence '
                 '(after "올랐습니다."). Do not touch the 다음 단계 paragraph that already has a page break.',
                 b2_add),
                ('ministry2-e2',
                 'In the paragraph about 초소규모 사업장 점검, move the inline column break so it falls just before '
                 '"질식 위험 작업부터입니다." (after "벌목, "). Keep the rest of that paragraph. Do not edit the other '
                 '다음 단계 paragraphs.',
                 b2_move),
                ('ministry2-e3',
                 'Remove the inline page break from the 다음 단계로 대학일자리+센터 paragraph. Keep it as one paragraph, with '
                 'a space where the break was. Do not remove the page break between paragraphs before "3. 노동시간".',
                 b2_remove),
            ],
            refuse=('ministry2-r',
                    'Insert an odd-page section break before "3. 노동시간" so that heading always starts on a right-hand '
                    'page, and set that section to three columns.',
                    'an odd-page or section break and a change of column count cannot be written')),
    3: dict(stem='launch', fm=DOC_FM, names=BREAK_NAMES, doc=B3,
            write_instruction=(
                'Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 '
                '"출시 당일 공지"; a default-style paragraph "한빛 노트 판매를 시작합니다." then an inline page break then '
                '"자사몰 쿠폰은 오늘 자정까지입니다."; a default-style paragraph "기업 고객 안내." then an inline column break then '
                '"개인 고객 안내."'),
            write_doc=B3_WRITE,
            read=('launch3-q1',
                  'The file repeats 출시일 11월 20일. In the paragraph that also names 마케팅 and CS 교육, what are the first '
                  'four words after the inline page break? In the paragraph that names 왼쪽 칼럼, what are the first '
                  'four words after the inline column break? Answer as `ANSWER: <page> | <column>`.'),
            read_expect='ANSWER: 인쇄물 마감은 출시일 | 오른쪽 칼럼은 개인',
            edits=[
                ('launch3-e1',
                 'In the opening paragraph (제품기획팀은), insert an inline page break after the first sentence '
                 '(after "제안합니다."). Do not touch the other paragraph that contains 출시일 11월 20일 and already has a '
                 'page break.',
                 b3_add),
                ('launch3-e2',
                 'In the paragraph that begins 왼쪽 칼럼은 기업 고객용입니다, move the inline column break so it falls after '
                 '"개인 고객용입니다." (just before "핵심 기능 3가지를 같은 순서로"). Do not edit the other 핵심 기능 3가지 sentence.',
                 b3_move),
                ('launch3-e3',
                 'Remove the inline page break from the paragraph that begins 출시일 11월 20일을 기준으로 마케팅은. Keep it as '
                 'one paragraph, with a space where the break was. Do not remove the page break between paragraphs '
                 'before 첨부.',
                 b3_remove),
            ],
            refuse=('launch3-r',
                    'Add a continuous section break after 메시지 so the rest of the file is two newspaper columns with '
                    'a 1 cm gap, without starting a new page.',
                    'a section break and a change of column count cannot be written')),
}


# ====================================================================== shapes

def sheet(name, rng, items):
    return {'name': name, 'range': rng, 'items': items}


def table(name, rng, cols):
    return {'k': 'table', 'name': name, 'range': rng, 'cols': cols}


def col(n, ty, fmt='', formula=''):
    return {'name': n, 'type': ty, 'format': fmt, 'formula': formula}


def merge(rng):
    return {'k': 'merge', 'range': rng}


def note(rng, author, text):
    return {'k': 'note', 'range': rng, 'author': author, 'text': text}


def hyperlink(rng, target):
    return {'k': 'hyperlink', 'range': rng, 'target': target}


def validation(rng, formula1):
    return {'k': 'validation', 'range': rng, 'type': 'list', 'formula1': formula1}


def cellis(rng, op, formula, dxf):
    return {'k': 'cf', 'type': 'cellIs', 'range': rng, 'operator': op, 'formula': formula, 'dxf': dxf}


def databar(rng, mn, mx, fill):
    return {'k': 'cf', 'type': 'dataBar', 'range': rng, 'min': mn, 'max': mx, 'fill': fill}


def shape(name, anchor, text, group=None):
    return {'k': 'shape', 'name': name, 'anchor': anchor, 'text': text, 'group': group}


def group(name, children):
    return {'k': 'group', 'name': name, 'children': children}


def chart(ty, data, title):
    return {'k': 'chart', 'type': ty, 'data': data, 'title': title}


def xkeep(i, kind, summary):
    return {'k': 'keep', 'id': i, 'kind': kind, 'summary': summary}


def book(sheets):
    return {'fm': XLS_FM, 'sheets': sheets}


# Seed 1: monthly KPI workbook. Corpus: simple-monthly-budget Data Entry Tip at B25:D27; ExcelWithAttachments group.
S1 = book([sheet('월별수지', 'A1:G30', [
    table('수지', 'A1:D20', [
        col('월', 'date', 'yyyy-mm'),
        col('수입', 'number', '#,##0'),
        col('지출', 'number', '#,##0'),
        col('잔액', 'number', '#,##0', '=[@수입]-[@지출]'),
    ]),
    merge('A1:D1'),
    note('C3', '작성자', '지출은 카드 승인 기준입니다.'),
    hyperlink('G2', 'https://example.com/budget'),
    validation('B2:B20', '&quot;급여,부수입,환급&quot;'),
    cellis('D2:D20', 'lessThan', '0', 'fill=#FFC7CE'),
    chart('column', '수지[월],수지[잔액]', '월별 잔액'),
    shape('입력 안내', 'B25:D27', '아래에 입력하면 표가 늘어납니다. 마지막 행에서 Enter.'),
    group('신호등', [
        shape('매출 배지', 'F2:G3', '매출 +12%'),
        shape('이익 배지', 'F4:G5', '이익 +8%'),
    ]),
    xkeep('k1', 'picture', '로고'),
])])

# Seed 2: regional sales dashboard. Repeated "목표 미달" on a callout and in a note.
S2 = book([sheet('지역매출', 'A1:H28', [
    table('지역', 'A1:E16', [
        col('지역', 'text'),
        col('지점', 'text'),
        col('매출', 'number', '#,##0'),
        col('목표', 'number', '#,##0'),
        col('달성', 'number', '0%'),
    ]),
    note('E2', '작성자', '부산 해운대는 목표 미달 (잠정).'),
    shape('서울 콜아웃', 'B18:D20', '서울 14곳, 전년 대비 +12%'),
    shape('부산 콜아웃', 'B22:D24', '목표 미달'),
    group('권역 배지', [
        shape('수도권 배지', 'F2:G3', '수도권 21곳'),
        shape('지방 배지', 'F4:G5', '지방 13곳'),
        shape('합계 배지', 'F6:G7', '합계 34곳'),
    ]),
    xkeep('k2', 'chart', '지역별 막대'),
])])

# Seed 3: budget execution. Tip + title shape + group of two arrows. Repeated "집행률 82%".
S3 = book([sheet('집행', 'A1:G24', [
    table('사업', 'A1:D14', [
        col('사업', 'text'),
        col('예산', 'number', '#,##0'),
        col('집행', 'number', '#,##0'),
        col('집행률', 'number', '0%'),
    ]),
    note('D2', '작성자', '집행률 82%는 9월 말 기준.'),
    shape('표 제목', 'A16:C17', '2026 사업별 집행'),
    shape('입력 안내', 'B20:D22', '행을 추가하려면 마지막 사업명 아래칸에 입력합니다.'),
    group('화살표', [
        shape('증가 화살표', 'F2:G3', '집행률 82%'),
        shape('주의 화살표', 'F4:G5', '인건비 초과'),
    ]),
    xkeep('k3', 'picture', '부서 직인'),
])])


def items(book_, sheet_name):
    for s in book_['sheets']:
        if s['name'] == sheet_name:
            return s['items']
    raise KeyError(sheet_name)


def find_item(book_, kind, name):
    for s in book_['sheets']:
        for it in s['items']:
            if it.get('k') == kind and it.get('name') == name:
                return it
            if it.get('k') == 'group':
                if kind == 'group' and it.get('name') == name:
                    return it
                for ch in it['children']:
                    if ch.get('k') == kind and ch.get('name') == name:
                        return ch
    raise KeyError((kind, name))


def s1_add(b):
    items(b, '월별수지').append(shape('환급 안내', 'B28:D29', '환급은 다음 달 수입에 넣습니다.'))


def s1_move(b):
    find_item(b, 'shape', '입력 안내')['anchor'] = 'B28:D30'


def s1_retext(b):
    find_item(b, 'shape', '매출 배지')['text'] = '매출 +15%'


def s1_resize(b):
    find_item(b, 'shape', '이익 배지')['anchor'] = 'F4:H6'


def s2_add(b):
    items(b, '지역매출').append(shape('대구 콜아웃', 'B26:D28', '대구 3곳, 전년 대비 +4%'))


def s2_move(b):
    find_item(b, 'shape', '부산 콜아웃')['anchor'] = 'E22:G24'


def s2_retext(b):
    find_item(b, 'shape', '부산 콜아웃')['text'] = '목표 달성'


def s2_resize(b):
    find_item(b, 'shape', '서울 콜아웃')['anchor'] = 'B18:E21'


def s3_add(b):
    items(b, '집행').append(shape('마감 안내', 'B23:D24', '10월 마감은 11월 5일입니다.'))


def s3_move(b):
    find_item(b, 'shape', '표 제목')['anchor'] = 'A18:C19'


def s3_retext(b):
    find_item(b, 'shape', '증가 화살표')['text'] = '집행률 85%'


def s3_resize(b):
    find_item(b, 'shape', '입력 안내')['anchor'] = 'B20:E23'


S1_WRITE = book([sheet('메모', 'A1:D8', [
    table('메모표', 'A1:B4', [col('항목', 'text'), col('값', 'text')]),
    shape('오늘', 'C2:D3', '오늘은 마감일입니다.'),
    group('뱃지', [
        shape('통과', 'C5:D6', '통과'),
        shape('보류', 'C7:D8', '보류'),
    ]),
])])

S2_WRITE = book([sheet('메모', 'A1:D8', [
    table('메모표', 'A1:B3', [col('지역', 'text'), col('상태', 'text')]),
    shape('경고', 'C2:D3', '목표 미달'),
    group('등', [
        shape('녹', 'C5:D6', '녹'),
        shape('적', 'C7:D8', '적'),
    ]),
])])

S3_WRITE = book([sheet('메모', 'A1:D8', [
    table('메모표', 'A1:B3', [col('사업', 'text'), col('집행률', 'number', '0%')]),
    shape('제목', 'C1:D2', '주간 집행'),
    group('표시', [
        shape('양호', 'C4:D5', '양호'),
        shape('주의', 'C6:D7', '주의'),
    ]),
])])


SHAPES = {
    1: dict(stem='kpi', fm=XLS_FM, names={}, doc=S1,
            write_instruction=(
                'Write a new workbook with exactly one sheet 메모, range A1:D8, containing: a table 메모표 on A1:B4 '
                'with columns 항목 (text) and 값 (text); a shape named 오늘, anchor C2:D3, text "오늘은 마감일입니다."; a group '
                'named 뱃지 of two shapes, 통과 at C5:D6 text "통과" and 보류 at C7:D8 text "보류". No other items. Use the '
                'front matter of a spreadsheet (type spreadsheet, format xlsx, schema 1).'),
            write_doc=S1_WRITE,
            read=('kpi1-q1',
                  'What is the anchor of the shape 입력 안내, and which two shape names are in the group 신호등? '
                  'Answer as `ANSWER: <anchor> | <name>, <name>` in the order they are written.'),
            read_expect='ANSWER: B25:D27 | 매출 배지, 이익 배지',
            edits=[
                ('kpi1-e1',
                 'Add a shape named 환급 안내, anchored B28:D29, with text "환급은 다음 달 수입에 넣습니다." Leave the 입력 안내 '
                 'shape where it is.',
                 s1_add),
                ('kpi1-e2',
                 'Move the 입력 안내 shape so its anchor is B28:D30. Do not change its text. Do not move the 신호등 group.',
                 s1_move),
                ('kpi1-e3',
                 'Change the text of 매출 배지 to "매출 +15%". Do not edit 이익 배지, and do not change any cell note.',
                 s1_retext),
                ('kpi1-e4',
                 'Resize 이익 배지 so its anchor is F4:H6. Keep its text and keep it in the 신호등 group.',
                 s1_resize),
            ],
            refuse=('kpi1-r',
                    'Add a picture from files/org.png as a drawing over H2:J10, and rotate the 입력 안내 shape 15 degrees.',
                    'a picture and a rotation cannot be written')),
    2: dict(stem='region', fm=XLS_FM, names={}, doc=S2,
            write_instruction=(
                'Write a new workbook with exactly one sheet 메모, range A1:D8, containing: a table 메모표 on A1:B3 '
                'with columns 지역 (text) and 상태 (text); a shape named 경고 anchored C2:D3 text "목표 미달"; a group named '
                '등 of two shapes, 녹 at C5:D6 text "녹" and 적 at C7:D8 text "적". No other items.'),
            write_doc=S2_WRITE,
            read=('region2-q1',
                  'Which shape is named 부산 콜아웃, what is its anchor, and what is its text? Answer as '
                  '`ANSWER: <anchor> | <text>`.'),
            read_expect='ANSWER: B22:D24 | 목표 미달',
            edits=[
                ('region2-e1',
                 'Add a shape named 대구 콜아웃, anchored B26:D28, with text "대구 3곳, 전년 대비 +4%". Do not change 서울 콜아웃.',
                 s2_add),
                ('region2-e2',
                 'Move 부산 콜아웃 so its anchor is E22:G24. Keep its text "목표 미달". Do not edit the note that also says '
                 '목표 미달.',
                 s2_move),
                ('region2-e3',
                 'Change the text of 부산 콜아웃 to "목표 달성". Do not edit the note that says 목표 미달 (잠정).',
                 s2_retext),
                ('region2-e4',
                 'Resize 서울 콜아웃 so its anchor is B18:E21. Keep its text.',
                 s2_resize),
            ],
            refuse=('region2-r',
                    'Connect 서울 콜아웃 to 부산 콜아웃 with an elbow connector, and give 수도권 배지 a drop shadow.',
                    'a connector and a shadow cannot be written')),
    3: dict(stem='exec', fm=XLS_FM, names={}, doc=S3,
            write_instruction=(
                'Write a new workbook with exactly one sheet 메모, range A1:D8, containing: a table 메모표 on A1:B3 '
                'with columns 사업 (text) and 집행률 (number, format 0%); a shape named 제목 anchored C1:D2 text "주간 집행"; '
                'a group named 표시 of two shapes, 양호 at C4:D5 text "양호" and 주의 at C6:D7 text "주의". No other items.'),
            write_doc=S3_WRITE,
            read=('exec3-q1',
                  'The file repeats 집행률 82%. Which object is the shape (not the note) that holds those words, and '
                  'what group is it in? Answer as `ANSWER: <shape name> | <group name>`.'),
            read_expect='ANSWER: 증가 화살표 | 화살표',
            edits=[
                ('exec3-e1',
                 'Add a shape named 마감 안내, anchored B23:D24, with text "10월 마감은 11월 5일입니다."',
                 s3_add),
                ('exec3-e2',
                 'Move 표 제목 so its anchor is A18:C19. Keep its text "2026 사업별 집행".',
                 s3_move),
                ('exec3-e3',
                 'Change the text of 증가 화살표 to "집행률 85%". Do not edit the note that says 집행률 82%.',
                 s3_retext),
                ('exec3-e4',
                 'Resize 입력 안내 so its anchor is B20:E23. Keep its text.',
                 s3_resize),
            ],
            refuse=('exec3-r',
                    'Turn 증가 화살표 into a right-arrow preset (kind=rightArrow) filled accent1, with a 2 pt outline.',
                    'a preset kind, a fill and an outline cannot be written')),
}


# ====================================================================== databar

# Seed 1: monthly budget, modelled on simple-monthly-budget E4 dataBar (num 0 → named formula, Office blue).
# Two similar bars so the edit must hit one.
D1 = book([sheet('예산', 'A1:G20', [
    table('월별', 'A1:G16', [
        col('월', 'date', 'yyyy-mm'),
        col('수입', 'number', '#,##0'),
        col('지출', 'number', '#,##0'),
        col('잔액', 'number', '#,##0', '=[@수입]-[@지출]'),
        col('실적', 'number', '#,##0'),
        col('목표', 'number', '#,##0'),
        col('달성률', 'number', '0%'),
    ]),
    cellis('D2:D16', 'lessThan', '0', 'fill=#FFC7CE'),
    databar('E4:E16', ['num', '0'], ['num', '월간수입합계'], '#638EC6'),
    databar('G4:G16', ['min', None], ['max', None], '#FFC7CE'),
    xkeep('k1', 'chart', '수입·지출 막대'),
])])

# Seed 2: regional attainment, percent bars on two similar columns.
D2 = book([sheet('달성', 'A1:F18', [
    table('지역', 'A1:F14', [
        col('지역', 'text'),
        col('매출', 'number', '#,##0'),
        col('목표', 'number', '#,##0'),
        col('달성률', 'number', '0%'),
        col('전년', 'number', '0%'),
        col('증감', 'number', '0%'),
    ]),
    databar('D2:D14', ['num', '0'], ['percent', '100'], '#638EC6'),
    databar('E2:E14', ['num', '0'], ['percent', '100'], '#63BE7B'),
    cellis('F2:F14', 'lessThan', '0', 'fill=#FFC7CE'),
])])

# Seed 3: KPI scores, percentile bars. Repeated 80점.
D3 = book([sheet('KPI', 'A1:E16', [
    table('점수', 'A1:E12', [
        col('팀', 'text'),
        col('점수', 'number', '0'),
        col('목표', 'number', '0'),
        col('가중', 'number', '0'),
        col('등급', 'text'),
    ]),
    databar('B2:B12', ['percentile', '10'], ['percentile', '90'], '#638EC6'),
    databar('C2:C12', ['num', '0'], ['num', '100'], '#FFEB84'),
    cellis('B2:B12', 'greaterThan', '80', 'fill=#C6EFCE'),
])])


def find_bar(b, rng):
    for s in b['sheets']:
        for it in s['items']:
            if it.get('k') == 'cf' and it.get('type') == 'dataBar' and it.get('range') == rng:
                return it
    raise KeyError(rng)


def d1_range(b):
    find_bar(b, 'E4:E16')['range'] = 'E4:E18'


def d1_color(b):
    find_bar(b, 'E4:E16')['fill'] = '#63BE7B'


def d1_minmax(b):
    it = find_bar(b, 'E4:E16')
    it['min'] = ['min', None]
    it['max'] = ['max', None]


def d2_range(b):
    find_bar(b, 'D2:D14')['range'] = 'D2:D16'


def d2_color(b):
    find_bar(b, 'D2:D14')['fill'] = '#FFC7CE'


def d2_minmax(b):
    it = find_bar(b, 'D2:D14')
    it['min'] = ['percent', '0']
    it['max'] = ['percent', '100']


def d3_range(b):
    find_bar(b, 'B2:B12')['range'] = 'B2:B14'


def d3_color(b):
    find_bar(b, 'B2:B12')['fill'] = '#FFC7CE'


def d3_minmax(b):
    it = find_bar(b, 'B2:B12')
    it['min'] = ['num', '0']
    it['max'] = ['num', '100']


D1_WRITE = book([sheet('요약', 'A1:C6', [
    table('요약표', 'A1:C4', [col('지표', 'text'), col('값', 'number', '#,##0'), col('비율', 'number', '0%')]),
    databar('B2:B4', ['num', '0'], ['max', None], '#638EC6'),
])])

D2_WRITE = book([sheet('요약', 'A1:C6', [
    table('요약표', 'A1:C4', [col('지역', 'text'), col('달성률', 'number', '0%'), col('전년', 'number', '0%')]),
    databar('B2:B4', ['num', '0'], ['percent', '100'], '#63BE7B'),
])])

D3_WRITE = book([sheet('요약', 'A1:C6', [
    table('요약표', 'A1:C4', [col('팀', 'text'), col('점수', 'number', '0'), col('목표', 'number', '0')]),
    databar('B2:B4', ['min', None], ['max', None], '#FFEB84'),
])])


DATABAR = {
    1: dict(stem='budget', fm=XLS_FM, names={}, doc=D1,
            write_instruction=(
                'Write a new workbook with exactly one sheet 요약, range A1:C6, containing: a table 요약표 on A1:C4 '
                'with columns 지표 (text), 값 (number, format #,##0), 비율 (number, format 0%); one data-bar rule on '
                'B2:B4 whose minimum is the number 0, whose maximum is the largest value in the range, and whose '
                'fill is #638EC6. No other items.'),
            write_doc=D1_WRITE,
            read=('budget1-q1',
                  'The 실적 column (E) has a data bar. What are its min, max and fill? Answer as '
                  '`ANSWER: min=<…> max=<…> fill=<…>` using the file\'s own tokens.'),
            read_expect_a='ANSWER: min=num:0 max=num:월간수입합계 fill=#638EC6',
            read_expect_b='ANSWER: min-type=num min=0 max-type=num max=월간수입합계 color=#638EC6',
            edits=[
                ('budget1-e1',
                 'Change the data bar on the 실적 column so its range is E4:E18. Do not change the data bar on 달성률 (G), '
                 'and do not change the cellIs rule on 잔액.',
                 d1_range),
                ('budget1-e2',
                 'Change the fill colour of the 실적 data bar to #63BE7B. Leave the 달성률 bar\'s colour as it is.',
                 d1_color),
                ('budget1-e3',
                 'Change the 실적 data bar so its minimum is the smallest value in the range and its maximum is the '
                 'largest value in the range (the auto min/max types, no number). Keep its fill.',
                 d1_minmax),
            ],
            refuse=('budget1-r',
                    'Add a 3-color scale on F4:F16 (red-yellow-green) and an icon set on G4:G16.',
                    'a color scale and an icon set cannot be written')),
    2: dict(stem='rate', fm=XLS_FM, names={}, doc=D2,
            write_instruction=(
                'Write a new workbook with exactly one sheet 요약, range A1:C6, containing: a table 요약표 on A1:C4 '
                'with columns 지역 (text), 달성률 (number, format 0%), 전년 (number, format 0%); one data-bar rule on '
                'B2:B4 whose minimum is the number 0, whose maximum is 100 percent, and whose fill is #63BE7B. '
                'No other items.'),
            write_doc=D2_WRITE,
            read=('rate2-q1',
                  'Two columns have a data bar from 0 to 100 percent. What is the fill of the 달성률 column (D), and '
                  'what is the fill of the 전년 column (E)? Answer as `ANSWER: D=<…> | E=<…>`.'),
            read_expect_a='ANSWER: D=#638EC6 | E=#63BE7B',
            read_expect_b='ANSWER: D=#638EC6 | E=#63BE7B',
            edits=[
                ('rate2-e1',
                 'Change the 달성률 data bar (column D) so its range is D2:D16. Do not change the 전년 bar on E.',
                 d2_range),
                ('rate2-e2',
                 'Change the fill of the 달성률 data bar to #FFC7CE. Leave the 전년 bar green.',
                 d2_color),
                ('rate2-e3',
                 'Change the 달성률 data bar so its minimum is 0 percent and its maximum is 100 percent (percent '
                 'types, not num). Keep its fill and range.',
                 d2_minmax),
            ],
            refuse=('rate2-r',
                    'Make the 달성률 data bar a gradient bar (the bar fades) and hide the cell values (showValue off).',
                    'a gradient bar and showValue cannot be written')),
    3: dict(stem='score', fm=XLS_FM, names={}, doc=D3,
            write_instruction=(
                'Write a new workbook with exactly one sheet 요약, range A1:C6, containing: a table 요약표 on A1:C4 '
                'with columns 팀 (text), 점수 (number, format 0), 목표 (number, format 0); one data-bar rule on B2:B4 '
                'whose minimum is the smallest value in the range, whose maximum is the largest, and whose fill '
                'is #FFEB84. No other items.'),
            write_doc=D3_WRITE,
            read=('score3-q1',
                  'The 점수 column (B) has a data bar. What are its min and max types (and values, if any) and its '
                  'fill? Answer as `ANSWER: min=<…> max=<…> fill=<…>` using the file\'s own tokens.'),
            read_expect_a='ANSWER: min=percentile:10 max=percentile:90 fill=#638EC6',
            read_expect_b='ANSWER: min-type=percentile min=10 max-type=percentile max=90 color=#638EC6',
            edits=[
                ('score3-e1',
                 'Change the 점수 data bar so its range is B2:B14. Do not change the 목표 bar on C, and do not change '
                 'the cellIs rule that also sits on B2:B12.',
                 d3_range),
                ('score3-e2',
                 'Change the fill of the 점수 data bar to #FFC7CE. Leave the 목표 bar yellow.',
                 d3_color),
                ('score3-e3',
                 'Change the 점수 data bar so its minimum is the number 0 and its maximum is the number 100 (num '
                 'types). Keep its fill. Do not edit the cellIs rule that uses 80.',
                 d3_minmax),
            ],
            refuse=('score3-r',
                    'Add an icon set (3 arrows) on 등급 (E2:E12) and a data bar with a solid axis at the cell midpoint.',
                    'an icon set and a data-bar axis cannot be written')),
}


SPECS = {'breaks': BREAKS, 'shapes': SHAPES, 'databar': DATABAR}


def spec(dec, rep):
    return SPECS[dec][rep]
