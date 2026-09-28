# Round 3 seed content, tasks and intended results, in one syntax-neutral form (DESIGN.md §10 item 8).
#
# Document blocks (lists, so edit functions can change them in place):
#   ['h', level, text]      a heading
#   ['p', text]             a paragraph with the default style
#   ['div', style, text]    a paragraph with a style
#   ['e', style_or_None]    an empty paragraph (default style when None)
#   ['ul', text] / ['ul2', text]   a list item / a nested list item
#   ['table', style_or_None, rows]
#   ['pagebreak']
# rows is a dense grid: rows[0] is the header row and every row has one entry per column. An entry is
#   '^^' (merged into the cell above), '<<' (merged into the cell to the left), a string (one paragraph with the
#   default style; '' is an empty cell), or a list of paragraphs [(style_or_None, text), ...].
# In the table specs below, ' // ' separates the paragraphs of a cell and «Name» in front of a paragraph gives it
# the paragraph style Name. This notation belongs to neither candidate; it is only how this file writes cells.
# An edit is (instruction, fn), where fn changes a copy of the seed, or (instruction, None, reason) when the
# intended answer is a refusal.
import copy
import re

DOC_FM = {'type': 'document', 'format': 'docx', 'template': 'org/report', 'schema': '1'}
FORM_FM = {'type': 'document', 'format': 'hwpx', 'template': 'org/gov-form', 'schema': '1'}


def cp(x):
    return copy.deepcopy(x)


def H(level, text):
    return ['h', level, text]


def P(text):
    return ['p', text]


def D(style, text):
    return ['div', style, text]


def E(style=None):
    return ['e', style]


def EN(k, style=None):
    return [E(style) for _ in range(k)]


def UL(text):
    return ['ul', text]


def UL2(text):
    return ['ul2', text]


def TB(style, rows):
    return ['table', style, rows]


def PB():
    return ['pagebreak']


def cellv(s):
    s = s.strip()
    if s in ('^^', '<<'):
        return s
    out = []
    for part in s.split(' // '):
        part = part.strip()
        m = re.match(r'^«([^»]+)»(.*)$', part)
        out.append((m.group(1), m.group(2).strip()) if m else (None, part))
    if len(out) == 1 and out[0][0] is None:
        return out[0][1]
    return out


def rows(spec):
    """'a | b // «Style»c | d' lines -> dense rows."""
    return [[cellv(c) for c in line.split('|')] for line in spec.strip().split('\n')]


def paras(cell):
    """A cell entry as a list of (style, text)."""
    if isinstance(cell, str):
        return [(None, cell)]
    return list(cell)


def block_index(blocks, kind, text):
    for i, b in enumerate(blocks):
        if b[0] == kind and b[-1] == text:
            return i
    raise KeyError((kind, text))


def div_index(blocks, text):
    for i, b in enumerate(blocks):
        if b[0] in ('p', 'div') and b[-1] == text:
            return i
    raise KeyError(text)


def table_after(blocks, text, nth=0):
    """The nth table after the block whose text is `text`."""
    i = [k for k, b in enumerate(blocks) if b[0] != 'e' and b[-1] == text][0]
    k = 0
    for b in blocks[i + 1:]:
        if b[0] == 'table':
            if k == nth:
                return b
            k += 1
    raise KeyError(text)


def row_index(rws, col, text):
    for i, r in enumerate(rws):
        if r[col] == text or (isinstance(r[col], list) and r[col][0][1] == text):
            return i
    raise KeyError(text)


def col_index(rws, text):
    return rws[0].index(text)


UNITS = {}

# ======================================================================= cellpara
# Cell shapes borrowed from the multi-paragraph cells of prototype/remainder/corpus (see README.md):
#   label + value lines in one cell (form_footnotes.docx: "03 - Date of birth" / "Day Month Year"),
#   a cell of options, one per paragraph (form_footnotes.docx t1 r12 c0: the education check list),
#   a styled first line followed by plain items (form_footnotes.docx t5: "INSTRUCTIONS" + 4 items),
#   a notes cell whose second line has another style (form_footnotes.docx t2 r2: "F - Validade" + a Header line),
#   multi-paragraph cells inside or next to merged cells (fdo76098.docx: a vMerge cell and a gridSpan cell with
#   two paragraphs each; form_footnotes.docx: most multi-paragraph cells span columns).

# ----------------------------------------------------------------------- cellpara 1: government notice
C1_NAMES = {
    'paragraph_styles': [
        ('표 제목', '표 바로 위의 표 제목 줄'),
        ('참고', '본문 아래의 작은 회색 글씨 참고 사항'),
        ('표 참고', '표 칸 안의 작은 글씨 참고 사항'),
        ('표 강조', '표 칸 안에서 굵은 글씨로 강조하는 줄'),
        ('발신 명의', '가운데 정렬된 크고 굵은 기관장 명의'),
    ],
    'table_styles': [
        ('표 눈금', '모든 칸에 가는 검은 실선'),
        ('눈금 표 4', '회색 머리글 행, 회색 줄무늬 행'),
        ('일반 표 1', '가는 가로선만, 세로선 없음'),
    ]}

C1 = [
    H(1, '2027년 청년 창업 지원사업 참여자 모집 공고'),
    P('○○시는 지역 청년의 창업을 돕기 위하여 「2027년 청년 창업 지원사업」에 참여할 예비 창업자와 초기 창업자를 '
      '다음과 같이 모집합니다.'),
    P('2027년 1월 13일'),
    D('발신 명의', '○○시장'),
    H(2, '1. 사업 개요'),
    UL('사업 기간: 2027. 3. 1. ~ 2027. 11. 30. (9개월)'),
    UL('지원 규모: 40개 팀 내외, 팀당 최대 3,000만 원'),
    UL('지원 분야: 제조, 지식서비스, 콘텐츠, 바이오, 친환경 소재 등 전 분야'),
    UL('지원 내용: 사업화 자금, 창업 교육, 전문가 멘토링, 입주 공간'),
    UL2('입주 공간은 ○○청년창업센터 2~4층에 있으며 최대 20개 팀이 입주할 수 있습니다.'),
    UL('신청 기간: 2027. 1. 20.(수) ~ 2. 10.(수) 18:00'),
    P('올해는 지역 특화 산업(바이오, 콘텐츠, 친환경 소재) 분야 창업 팀을 전체 선정 인원의 30% 이상 선발하며, 선정된 팀은 '
      '협약 기간 동안 분기마다 사업 진행 상황을 보고해야 합니다.'),
    H(2, '2. 신청 자격'),
    P('신청일 현재 다음 요건을 모두 갖춘 사람이 신청할 수 있습니다.'),
    D('표 제목', '표 1. 신청 자격'),
    TB('표 눈금', rows('''
구분 | 요건 | 비고
연령 | 공고일 기준 만 19세 이상 39세 이하 // 병역을 마친 사람은 복무 기간(최대 6년)을 연령 계산에서 뺍니다. | «표 참고»1987. 1. 13. 이후 출생자
거주 | 공고일 현재 ○○시에 주민등록을 둔 사람 | «표 참고»공동 대표는 전원이 요건을 갖추어야 합니다. // «표 참고»법인은 본점 소재지 기준
창업 | 예비 창업자 또는 창업 3년 이내 기업의 대표 // 창업일은 사업자등록증의 개업일(법인은 법인 등기일) 기준 | ^^
제외 대상 | 국세·지방세 체납자 // 같은 아이템으로 다른 정부 창업 지원사업을 수행 중인 사람 // 휴업 또는 폐업 중인 기업의 대표 | «표 참고»선정 후에 확인되어도 선정을 취소합니다.''')),
    D('참고', '자격 요건은 공고일 기준으로 판단하며, 제출한 증빙 서류로 확인합니다. 요건을 갖추지 못한 것이 확인되면 '
                '평가 대상에서 제외합니다.'),
    H(2, '3. 지원 내용'),
    D('표 제목', '표 2. 지원 내용'),
    TB('눈금 표 4', rows('''
구분 | 지원 내용 | 지원 한도 | 비고
사업화 자금 | 시제품 제작·마케팅 비용, 지식재산권 출원 비용 // 자부담 10% 이상 | 팀당 최대 3,000만 원 | «표 참고»인건비는 총액의 30% 이내
창업 교육 | 창업 기초 과정 (20시간) // 분야별 심화 과정 (16시간) | 전액 지원 | «표 참고»기초 과정은 80% 이상 출석해야 합니다.
멘토링 | 분야별 전문가 1:1 멘토링 (월 2회) // 투자 유치 설명회 참가 | ^^ | «표 참고»멘토는 센터가 배정합니다.
입주 공간 | ○○청년창업센터 개별 사무실 // 회의실과 장비실 공동 이용 | 최대 20개 팀 | «표 참고»관리비 월 5만 원은 입주 기업이 부담합니다.''')),
    D('참고', '사업화 자금은 협약을 체결한 뒤 두 번에 나누어 지급합니다. 2차 자금은 중간 점검을 통과한 팀에만 지급합니다.'),
    H(2, '4. 신청 방법 및 제출 서류'),
    UL('신청 방법: ○○시 청년포털에서 온라인으로 접수 (방문·우편 접수 불가)'),
    UL('제출 서류는 모두 PDF 파일로 올립니다.'),
    D('표 제목', '표 3. 제출 서류'),
    TB('표 눈금', rows('''
서류 | 대상 | 비고
사업 신청서 | 전원 | 지정 서식
사업계획서 | 전원 | 지정 서식, 20쪽 이내
주민등록초본 | 전원 | 공고일 이후 발급분
사업자등록증 사본 | 기창업자 | 해당자만''')),
    H(2, '5. 선정 절차'),
    TB('일반 표 1', rows('''
단계 | 일정 | 내용
서류 평가 | 2월 중 | 사업계획서 평가 (모집 인원의 2배수 선정)
발표 평가 | 3월 초 | 대면 발표 및 질의응답
최종 선정 | 3월 중 | 선정 결과 개별 통보''')),
    D('참고', '일정은 사정에 따라 바뀔 수 있으며, 바뀐 일정은 청년포털에 공지합니다.'),
    D('표 제목', '표 4. 평가 기준'),
    TB('표 눈금', rows('''
평가 항목 | 세부 내용 | 배점
사업 아이템 | 아이템의 독창성과 시장성 // 경쟁 제품과의 차별성 | 40
사업화 역량 | 대표자와 팀원의 전문성 // 사업화 추진 계획의 구체성 | 30
지역 기여 | 지역 고용 창출 계획 // «표 참고»○○시 거주자를 채용할 계획이 있으면 2점을 더합니다. | 20
사업비 계획 | 사업비 편성의 적정성 | 10
합계 | << | 100''')),
    D('참고', '발표 평가는 서류 평가 점수와 합산하지 않고 발표 평가 점수만으로 최종 선정합니다.'),
    H(2, '6. 유의 사항'),
    UL('신청서와 사업계획서의 내용이 사실과 다르면 선정을 취소하고 지급한 지원금을 돌려받습니다.'),
    UL('한 사람은 한 팀으로만 신청할 수 있으며, 공동 대표의 중복 신청도 허용하지 않습니다.'),
    UL('사업화 자금은 협약서에 정한 용도로만 써야 하며, 다른 용도로 쓰면 환수합니다.'),
    UL('선정된 팀은 협약 기간이 끝난 뒤 2년 동안 매년 경영 현황 조사에 응해야 합니다.'),
    H(2, '7. 문의'),
    P('○○시청 일자리정책과 청년창업팀 (☎ 031-000-1234, 평일 09:00~18:00)'),
    P('온라인 접수 시스템 오류는 ○○시 청년포털 운영팀 (☎ 031-000-5678)으로 문의하시기 바랍니다.'),
]


def c1_e1(d):
    t = table_after(d, '표 3. 제출 서류')[2]
    r = row_index(t, 0, '사업계획서')
    t[r][2] = [(None, '지정 서식, 20쪽 이내'), ('표 참고', '사업비 집행 계획 포함')]


def c1_e2(d):
    t = table_after(d, '표 2. 지원 내용')[2]
    r = row_index(t, 0, '사업화 자금')
    t[r][1] = [(None, '시제품 제작·마케팅 비용'), (None, '지식재산권 출원 비용'), (None, '자부담 10% 이상')]


def c1_e4(d):
    t = table_after(d, '표 2. 지원 내용')[2]
    a = row_index(t, 0, '창업 교육')
    t[a][1] = paras(t[a][1]) + paras(t[a + 1][1])
    t[a + 1][1] = '^^'


UNITS['cellpara', 1] = dict(
    doc=C1, fm=FORM_FM, stem='notice', names=C1_NAMES,
    write=dict(
        instruction=(
            'Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 '
            '"2027년 청년 창업 교육 수강생 모집"; the paragraph "창업 교육 과정별 내용은 다음과 같습니다." with the '
            'default style; the line "표 1. 과정별 내용" in the file\'s style for a table\'s title line; directly after '
            'it a table with thin black lines around all cells, with the columns 과정, 내용, 비고 and these rows, in '
            'this order:\n'
            '- 기초 과정: the 내용 cell has two paragraphs, "사업계획서 작성 (8시간)" and "창업 법률·세무 (12시간)"; '
            'the 비고 cell has one paragraph, "온라인 병행", in the style for small notes inside a table.\n'
            '- 심화 과정: the 내용 cell has two paragraphs, "분야별 실습 (12시간)" and, in the style for small notes '
            'inside a table, "분야는 신청할 때 선택"; its 비고 is the same cell as the 비고 of 기초 과정 (one cell '
            'covering both rows).\n'
            '- 수료 기준: "80% 이상 출석", one paragraph written once across the 내용 and 비고 columns.\n'
            'After the table, the note "교육 장소는 ○○청년창업센터 3층입니다." in the style for small gray notes below '
            'body text. Paragraphs in table cells have the default style unless a style is named above.'),
        doc=[
            H(1, '2027년 청년 창업 교육 수강생 모집'),
            P('창업 교육 과정별 내용은 다음과 같습니다.'),
            D('표 제목', '표 1. 과정별 내용'),
            TB('표 눈금', rows('''
과정 | 내용 | 비고
기초 과정 | 사업계획서 작성 (8시간) // 창업 법률·세무 (12시간) | «표 참고»온라인 병행
심화 과정 | 분야별 실습 (12시간) // «표 참고»분야는 신청할 때 선택 | ^^
수료 기준 | 80% 이상 출석 | <<''')),
            D('참고', '교육 장소는 ○○청년창업센터 3층입니다.'),
        ]),
    edits=[
        ('In 표 3 (제출 서류), give the 비고 cell of 사업계획서 a second paragraph, "사업비 집행 계획 포함", in the style '
         'for small notes inside a table. Change nothing else.', c1_e1),
        ('In 표 2 (지원 내용), the first paragraph of the 지원 내용 cell of 사업화 자금 is "시제품 제작·마케팅 비용, '
         '지식재산권 출원 비용". Split it at the comma into two paragraphs, "시제품 제작·마케팅 비용" and "지식재산권 '
         '출원 비용" (the comma goes away); the cell\'s other paragraph stays after them. Change nothing else.',
         c1_e2),
        ('In 표 1 (신청 자격), shade the whole 제외 대상 row in light gray so that it stands out.', None,
         'no style of this file shades a table row or cell'),
        ('In 표 2 (지원 내용), merge the 지원 내용 cells of 창업 교육 and 멘토링 into one cell. The merged cell keeps all '
         'four paragraphs, those of 창업 교육 first. Change nothing else.', c1_e4),
    ])

# ----------------------------------------------------------------------- cellpara 2: weekly meeting minutes
C2_NAMES = {
    'paragraph_styles': [
        ('Caption', 'title line directly above a table'),
        ('Note', 'small gray note below a table'),
        ('Table Note', 'small gray note inside a table cell'),
        ('Table Emphasis', 'bold text inside a table cell'),
        ('Quote', 'indented italic quotation'),
    ],
    'table_styles': [
        ('Table Grid', 'thin black lines around all cells'),
        ('Grid Table 4', 'gray header row, gray banded rows'),
        ('Plain Table 1', 'thin horizontal lines only'),
    ]}

C2 = [
    H(1, '제품개발본부 주간 업무 회의록'),
    P('일시: 2026. 10. 12.(월) 09:30 ~ 11:00 / 장소: 본관 7층 대회의실 / 주재: 본부장 김도현'),
    D('Caption', '표 1. 회의 개요'),
    TB('Table Grid', rows('''
항목 | 내용
참석자 | 본부장 김도현, 기획팀장 이수진 // 개발1팀장 박준영, 개발2팀장 정하늘 // 품질팀장 오세린, 디자인팀 선임 한지우
불참자 | 디자인팀장 한승민 (해외 출장)
작성자 | 기획팀 윤서아
배포 | 참석자 전원 // «Table Note»경영지원본부장에게는 요약본만 보냅니다.''')),
    H(2, '1. 지난 회의 조치 사항'),
    UL('모바일 앱 3.2 출시 일정 확정: 완료 (10. 8.)'),
    UL('고객 문의 분류 기준 개정: 진행 중'),
    UL2('품질팀 초안을 검토한 뒤 10. 16.까지 확정'),
    UL('개발2팀 신규 채용 2명: 완료'),
    UL('결제 모듈 보안 점검: 완료, 지적 사항 없음'),
    UL('사내 위키 개편: 보류 (4분기 사업계획 확정 후 재검토)'),
    P('고객 문의 분류 기준 개정은 품질팀 초안에 대한 개발팀 의견이 늦게 모여 일정이 1주 늦어졌다. 품질팀장은 의견 수렴 '
      '기한을 10. 14.로 다시 정하였다.'),
    H(2, '2. 안건별 논의'),
    P('본부장은 3분기 실적을 짧게 공유한 뒤 안건별 논의를 진행하였다. 3분기 본부 목표 달성률은 94%로, 앱 3.2 출시가 '
      '2주 늦어진 영향이 컸다. 본부장은 4분기에는 출시 일정 관리를 우선하고, 일정이 바뀌면 바로 공유해 달라고 '
      '당부하였다.'),
    P('안건은 사전에 공유한 네 가지를 순서대로 다루었으며, 각 안건의 논의 내용과 결정 사항은 다음과 같다.'),
    D('Caption', '표 2. 안건별 논의 결과'),
    TB('Grid Table 4', rows('''
안건 | 논의 내용 | 결정 사항 | 담당
앱 3.3 기능 범위 | 간편 로그인 추가 요청이 가장 많음 // 오프라인 모드는 개발 기간이 6주 이상 필요 // «Table Note»고객 설문 1,204명 기준 | 간편 로그인만 3.3에 포함 // 오프라인 모드는 3.4로 연기 | 개발1팀
앱 디자인 개편 | 메인 화면 시안 A, B 두 가지 검토 // B안 선호가 우세하나 글자 대비가 낮다는 의견 있음 | B안으로 진행 // «Table Note»접근성 검토 결과는 다음 회의에 보고 | ^^
결제 오류 대응 | 9월 결제 실패율 2.1% (8월 0.8%) // 원인은 PG사 인증서 갱신 지연 | 인증서 만료 30일 전 알림 자동화 // 월 1회 결제 모니터링 보고 | 개발2팀 // 품질팀
채용 계획 | 하반기 채용 2명 추가 필요 | 11월 중 공고 | 기획팀''')),
    D('Note', '담당 팀은 결정 사항의 진행 상황을 다음 회의에서 보고한다.'),
    P('앱 디자인 개편은 B안으로 진행하되, 디자인팀이 글자 대비를 높인 수정안을 10. 16.까지 공유하기로 하였다. 결제 오류 '
      '대응은 인증서 만료 알림이 구축될 때까지 개발2팀이 매주 월요일 인증서 상태를 직접 확인한다.'),
    P('채용 계획은 기획팀이 인사팀과 협의하여 공고안을 작성하며, 채용 분야는 안드로이드 개발 1명과 QA 1명으로 한다.'),
    H(2, '3. 팀별 공유 사항'),
    P('각 팀장은 지난주 주요 업무와 이번 주 계획을 공유하였다. 이슈가 있는 팀은 이슈 칸에 적었다.'),
    D('Caption', '표 3. 팀별 공유 사항'),
    TB('Table Grid', rows('''
팀 | 지난주 주요 업무 | 이번 주 계획 | 이슈
기획팀 | 4분기 사업계획 초안 작성 // 3분기 실적 보고서 배포 | 사업계획 본부 검토 | 없음
개발1팀 | 앱 3.2 출시 후 안정화 // 크래시 비율 0.3%로 감소 | 간편 로그인 설계 착수 | «Table Emphasis»iOS 18 대응 인력 부족 // 외주 1명 투입 검토 중
개발2팀 | 결제 오류 원인 분석 // PG사와 인증서 갱신 절차 협의 | 알림 자동화 개발 | 없음
품질팀 | 회귀 테스트 자동화 범위 확대 (62% → 70%) | 고객 문의 분류 기준 초안 보완 | 테스트 단말 3대 교체 필요
디자인팀 | 메인 화면 시안 A, B 제작 // 아이콘 세트 1차 정리 | B안 글자 대비 수정 // 접근성 검토 자료 준비 | 없음''')),
    D('Note', '이슈 칸의 인력과 장비 요청은 기획팀이 모아 경영지원본부에 전달한다.'),
    P('본부장은 개발1팀의 외주 인력 투입을 10. 16.까지 결정하기로 하고, 필요하면 개발2팀 인력을 2주 동안 지원하도록 '
      '하였다. 품질팀의 테스트 단말 교체는 이번 달 예산으로 처리한다.'),
    H(2, '4. 조치 사항'),
    D('Caption', '표 4. 조치 사항'),
    TB('Plain Table 1', rows('''
번호 | 조치 내용 | 담당 | 기한
1 | 간편 로그인 개발 일정 수립 | 개발1팀 | 10. 16.
2 | 접근성 검토 의뢰 | 개발1팀 | 10. 19.
3 | 인증서 만료 알림 구축 | 개발2팀 | 10. 30.
4 | 채용 공고안 작성 | 기획팀 | 10. 26.
5 | 테스트 단말 교체 요청 | 품질팀 | 10. 21.''')),
    H(2, '5. 기타'),
    UL('다음 회의: 2026. 10. 19.(월) 09:30, 본관 7층 대회의실'),
    UL('회의록 수정 요청은 10. 13.까지 기획팀 윤서아에게 보낸다.'),
    UL('3분기 실적 자료는 본부 공유 폴더에 올려 두었다.'),
    UL('10. 15.(목) 오후에는 본관 전산실 점검으로 사내 테스트 서버를 쓸 수 없다.'),
    UL('다음 회의 안건 (예정)'),
    UL2('간편 로그인 개발 일정 확정'),
    UL2('접근성 검토 결과 보고'),
    UL2('4분기 사업계획 본부안 확정'),
    P('회의록은 참석자 확인을 거쳐 10. 14.에 확정한다.'),
    P('끝.'),
]


def c2_e1(d):
    t = table_after(d, '표 2. 안건별 논의 결과')[2]
    r = row_index(t, 0, '앱 3.3 기능 범위')
    t[r][2] = [('Table Emphasis', '간편 로그인만 3.3에 포함'), (None, '오프라인 모드는 3.4로 연기')]


def c2_e2(d):
    t = table_after(d, '표 2. 안건별 논의 결과')[2]
    r = row_index(t, 0, '앱 3.3 기능 범위')
    moved = (None, '오프라인 모드는 개발 기간이 6주 이상 필요')
    t[r][1] = [p for p in t[r][1] if p != moved]
    t[r][2] = paras(t[r][2]) + [moved]


def c2_e4(d):
    t = table_after(d, '표 2. 안건별 논의 결과')[2]
    r = row_index(t, 0, '결제 오류 대응')
    t.insert(r + 1, rows('''
고객 문의 분류 기준 | 현행 12개 분류 중 4개가 겹침 // 품질팀 초안은 8개 분류 | 품질팀 초안 채택 // «Table Note»11월 1일부터 시행 | 품질팀''')[0])


UNITS['cellpara', 2] = dict(
    doc=C2, fm=DOC_FM, stem='minutes', names=C2_NAMES,
    write=dict(
        instruction=(
            'Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 '
            '"품질팀 주간 회의록"; the paragraph "일시: 2026. 10. 13.(화) 14:00 / 장소: 본관 5층 소회의실" with the '
            'default style; the line "표 1. 논의 결과" in the file\'s style for a table\'s title line; directly after it '
            'a table with a gray header row and gray banded rows, with the columns 안건, 논의 내용, 결정 사항 and these '
            'rows, in this order:\n'
            '- 보안 점검 결과: the 논의 내용 cell has two paragraphs, "지적 사항 3건" and "모두 경미한 설정 오류"; the '
            '결정 사항 cell has one paragraph, "10월 중 조치".\n'
            '- 서버 이전: the 논의 내용 cell has two paragraphs, "11월 둘째 주 이전 예정" and, as a small gray note '
            'inside the table, "주말 작업"; the 결정 사항 cell has two paragraphs, "이전 계획 승인" and "고객 공지는 '
            '1주 전".\n'
            '- 장애 대응 훈련: the 논의 내용 cell has one paragraph, "분기 1회 실시"; its 결정 사항 is the same cell '
            'as the 결정 사항 of 서버 이전 (one cell covering both rows).\n'
            'After the table, the small gray note below a table "다음 회의는 10. 20.에 연다." Paragraphs in table '
            'cells have the default style unless a style is named above.'),
        doc=[
            H(1, '품질팀 주간 회의록'),
            P('일시: 2026. 10. 13.(화) 14:00 / 장소: 본관 5층 소회의실'),
            D('Caption', '표 1. 논의 결과'),
            TB('Grid Table 4', rows('''
안건 | 논의 내용 | 결정 사항
보안 점검 결과 | 지적 사항 3건 // 모두 경미한 설정 오류 | 10월 중 조치
서버 이전 | 11월 둘째 주 이전 예정 // «Table Note»주말 작업 | 이전 계획 승인 // 고객 공지는 1주 전
장애 대응 훈련 | 분기 1회 실시 | ^^''')),
            D('Note', '다음 회의는 10. 20.에 연다.'),
        ]),
    edits=[
        ('In 표 2 (안건별 논의 결과), the paragraph "간편 로그인만 3.3에 포함" should be bold, in the file\'s style for '
         'bold text inside a table. Change nothing else.', c2_e1),
        ('In 표 2 (안건별 논의 결과), move the paragraph "오프라인 모드는 개발 기간이 6주 이상 필요" from the 논의 내용 '
         'cell of 앱 3.3 기능 범위 to the end of the 결정 사항 cell of the same row, directly after "오프라인 모드는 '
         '3.4로 연기". Change nothing else.', c2_e2),
        ('In 표 2 (안건별 논의 결과), center the text of the 담당 column vertically and horizontally in its cells.',
         None, 'no style of this file aligns the text of a table cell'),
        ('In 표 2 (안건별 논의 결과), add a row between 결제 오류 대응 and 채용 계획: 안건 "고객 문의 분류 기준"; '
         '논의 내용 with two paragraphs, "현행 12개 분류 중 4개가 겹침" and "품질팀 초안은 8개 분류"; 결정 사항 with '
         'two paragraphs, "품질팀 초안 채택" and, as a small gray note inside the table, "11월 1일부터 시행"; 담당 '
         '"품질팀". Paragraphs have the default style unless a style is named.', c2_e4),
    ])

# ----------------------------------------------------------------------- cellpara 3: facility rental application form
C3_NAMES = {
    'paragraph_styles': [
        ('서식 번호', '서식 맨 위의 작은 서식 번호 줄'),
        ('안내', '서식 제목 아래와 표 아래의 작은 안내 문구'),
        ('표 참고', '표 칸 안의 작은 글씨 참고 사항'),
        ('표 소제목', '표 칸 안의 굵은 소제목 줄'),
        ('서명', '오른쪽 정렬된 날짜·서명 줄'),
        ('수신', '왼쪽 정렬된 크고 굵은 수신 기관 줄'),
    ],
    'table_styles': [
        ('서식 표', '굵은 바깥 테두리, 안쪽은 가는 실선'),
        ('표 눈금', '모든 칸에 가는 검은 실선'),
        ('일반 표 1', '가는 가로선만, 세로선 없음'),
    ]}

C3 = [
    D('서식 번호', '[별지 제3호서식] (개정 2026. 7. 1.)'),
    H(1, '공공시설 대관 신청서'),
    D('안내', '※ 뒤쪽의 작성 방법을 읽고 작성하시기 바라며, [ ]에는 해당되는 곳에 √ 표시를 합니다.'),
    TB('서식 표', rows('''
구분 | 항목 | 내용
신청인 | 성명(단체명) | 한빛문화예술협회
^^ | 대표자 | 이서연
^^ | 주소 | 서울특별시 ○○구 ○○로 12<br/>한빛빌딩 3층
^^ | 연락처 | 전화: 02-000-1234 // 휴대전화: 010-0000-5678
이용 계획 | 시설명 | [ ] 대공연장 // [√] 소공연장 // [ ] 전시실
^^ | 이용 일시 | 2026. 11. 21.(토) 13:00 ~ 18:00 // «표 참고»리허설과 정리 시간 포함
^^ | 행사명 | 제5회 시민 합창 발표회
^^ | 행사 내용 | 시민 합창단 4개 팀 공연 // 관람객 약 250명 예상 // «표 참고»입장료 없음
^^ | 부대 설비 | [√] 음향 [√] 조명 [ ] 영상 // «표 참고»※ 해당 시 작성
준수 사항 | << | «표 소제목»신청인은 다음 사항을 지킵니다. // ① 시설물을 훼손하거나 잃어버린 경우 원래대로 복구합니다. // ② 허가받은 용도 외에는 시설을 사용하지 않습니다. // ③ 행사가 끝난 뒤 1시간 안에 정리를 마칩니다.''')),
    P('위와 같이 공공시설 대관을 신청하며, 준수 사항을 지킬 것을 서약합니다.'),
    D('서명', '2026년 10월 20일'),
    D('서명', '신청인 이서연 (서명 또는 인)'),
    D('수신', '○○시 문화예술회관장 귀하'),
    H(2, '첨부 서류 및 수수료'),
    TB('표 눈금', rows('''
구분 | 서류 | 수수료
신청인 제출 서류 | 가. 행사 계획서 1부 // 나. 단체 소개서 1부 (단체인 경우만) // «표 참고»※ 해당 시 작성 | 「○○시 문화예술회관 관리 조례」 별표 2에 따른 사용료
담당 공무원 확인 사항 | 법인 등기사항증명서 (법인인 경우만) | ^^''')),
    D('안내', '※ 담당 공무원 확인 사항은 신청인이 확인에 동의하지 않으면 해당 서류를 직접 제출해야 합니다.'),
    H(2, '개인정보 수집·이용 동의'),
    P('문화예술회관은 대관 신청을 처리하기 위하여 다음과 같이 개인정보를 수집·이용합니다.'),
    TB('서식 표', rows('''
구분 | 내용
수집 항목 | 성명, 주소, 전화번호, 휴대전화번호 // «표 참고»단체는 대표자의 정보를 수집합니다.
이용 목적 | 대관 신청의 접수와 허가 // 사용료 부과와 환불 // 시설 이용 안내
보유 기간 | 대관 종료 후 3년 // «표 참고»관계 법령에 따라 보존해야 하는 경우에는 그 기간
제3자 제공 | 제공하지 않습니다. // «표 참고»법령에 특별한 규정이 있는 경우는 예외로 합니다.
동의 거부 | 동의를 거부할 수 있습니다. 다만, 동의하지 않으면 대관 신청을 처리할 수 없습니다.''')),
    D('서명', '위 내용에 동의합니다. [√] 동의함 [ ] 동의하지 않음'),
    H(2, '사용료 감면'),
    P('다음에 해당하는 행사는 사용료를 감면합니다. 감면을 받으려면 신청서와 함께 증빙 서류를 내야 합니다.'),
    TB('표 눈금', rows('''
감면 구분 | 대상 | 감면율
전액 면제 | 시가 주최하거나 주관하는 행사 // 국가기관·공공기관이 공익 목적으로 여는 행사 | 100%
일부 감면 | 관내 등록 예술 단체의 정기 공연 // 관내 학교의 교육 행사 // «표 참고»단체별로 연 2회까지 | 50%
^^ | 장애인·노인 단체가 회원을 위해 여는 행사 | 30%''')),
    D('안내', '※ 감면 대상이 둘 이상에 해당하면 감면율이 가장 높은 하나만 적용합니다.'),
    D('안내', '※ 감면을 받은 뒤 입장료를 받거나 영리 목적으로 시설을 사용한 것이 확인되면 감면한 사용료를 다시 내야 '
            '합니다.'),
    D('안내', '※ 부대 설비 사용료는 감면 대상에 포함되지 않습니다.'),
    H(2, '작성 방법'),
    UL('성명(단체명)란에는 단체가 신청하는 경우 단체 이름을 적습니다.'),
    UL('이용 일시에는 리허설과 정리 시간을 포함하여 적습니다.'),
    UL('부대 설비는 사용할 설비에 √ 표시를 합니다.'),
    UL2('영상 설비는 사용 7일 전까지 따로 협의해야 합니다.'),
    UL('사용료는 허가 통지를 받은 날부터 5일 안에 냅니다.'),
    UL('사용 허가를 받은 뒤 취소하면 사용일 10일 전까지는 사용료 전액을, 그 뒤에는 절반을 돌려받습니다.'),
    UL('허가받은 사용자는 사용 권리를 다른 사람에게 넘길 수 없습니다.'),
    UL('시설 안에서는 음식물을 먹을 수 없으며, 무대 장치를 설치하려면 미리 허가를 받아야 합니다.'),
    UL('행사 계획서에는 행사 목적, 프로그램, 예상 관람 인원, 안전 관리 계획을 적습니다.'),
    UL('관람객이 300명 이상이면 안전 관리 요원 배치 계획을 함께 냅니다.'),
    UL('신청서는 사용일 60일 전부터 10일 전까지 문화예술회관 누리집이나 방문으로 냅니다.'),
    H(2, '처리 절차'),
    TB('일반 표 1', rows('''
신청서 작성 | 접수 | 검토 | 허가 | 사용료 납부
신청인 | 문화예술회관 | 문화예술회관 | 문화예술회관장 | 신청인''')),
    D('안내', '※ 처리 기간은 접수일부터 7일입니다. 다만, 같은 날짜에 신청이 겹치면 추첨으로 정합니다.'),
    D('안내', '※ 허가 결과는 신청서에 적은 휴대전화로 문자 메시지를 보내 알려 드립니다.'),
]


def c3_e1(d):
    t = table_after(d, '공공시설 대관 신청서')[2]
    r = row_index(t, 1, '부대 설비')
    t[r][2] = '[√] 음향 [√] 조명 [ ] 영상'


def c3_e2(d):
    t = table_after(d, '첨부 서류 및 수수료')[2]
    r = row_index(t, 0, '신청인 제출 서류')
    t[r][1] = '가. 행사 계획서 1부, 나. 단체 소개서 1부 (단체인 경우만)'


def c3_e4(d):
    t = table_after(d, '공공시설 대관 신청서')[2]
    r = row_index(t, 0, '준수 사항')
    ps = t[r][2]
    t[r][2] = ps[:2]
    t.insert(r + 1, ['^^', '^^', [ps[2]]])
    t.insert(r + 2, ['^^', '^^', [ps[3]]])


UNITS['cellpara', 3] = dict(
    doc=C3, fm=FORM_FM, stem='form', names=C3_NAMES,
    write=dict(
        instruction=(
            'Write a new document with exactly these blocks, in this order, and nothing else: the line "[별지 제5호서식]" '
            'in the style for the small form number at the top of a form; a heading level 1 "공공시설 사용 결과 '
            '보고서"; a table with a thick outer border and thin inner lines, with the columns 구분, 항목, 내용 and '
            'these rows, in this order:\n'
            '- 사용자 / 단체명 / "한빛문화예술협회".\n'
            '- the same 구분 cell as the row above (사용자 covers both rows) / 사용 일시 / a 내용 cell with two '
            'paragraphs, "2026. 11. 21.(토) 13:00 ~ 18:00" and, in the style for small notes inside a table, "실제 '
            '사용 시간 기준".\n'
            '- 결과, written once across the 구분 and 항목 columns / a 내용 cell with three paragraphs: "관람객 238명", '
            '"시설 훼손 없음" and, in the style for small notes inside a table, "사진 5매 첨부".\n'
            'After the table, the line "2026년 11월 24일" in the style for right-aligned date and signature lines. '
            'Paragraphs in table cells have the default style unless a style is named above.'),
        doc=[
            D('서식 번호', '[별지 제5호서식]'),
            H(1, '공공시설 사용 결과 보고서'),
            TB('서식 표', rows('''
구분 | 항목 | 내용
사용자 | 단체명 | 한빛문화예술협회
^^ | 사용 일시 | 2026. 11. 21.(토) 13:00 ~ 18:00 // «표 참고»실제 사용 시간 기준
결과 | << | 관람객 238명 // 시설 훼손 없음 // «표 참고»사진 5매 첨부''')),
            D('서명', '2026년 11월 24일'),
        ]),
    edits=[
        ('Remove the note "※ 해당 시 작성" from the 부대 설비 cell of the application table. The same note elsewhere '
         'stays. Change nothing else.', c3_e1),
        ('In the table under "첨부 서류 및 수수료", make the 서류 cell of 신청인 제출 서류 a single paragraph with the '
         'default style: "가. 행사 계획서 1부, 나. 단체 소개서 1부 (단체인 경우만)". The note in that cell goes away. '
         'Change nothing else.', c3_e2),
        ('In the 준수 사항 cell of the application table, put 6pt of space between the pledges ①, ② and ③.', None,
         'spacing in points cannot be written, and no style of this file adds space between paragraphs'),
        ('In the application table, split the 준수 사항 row into three rows: the first keeps the cell\'s bold heading '
         'line and ①, the second holds ② and the third holds ③, each as the only paragraph of its cell. The 준수 사항 '
         'label stays one cell across the 구분 and 항목 columns and now covers all three rows. Change nothing else.',
         c3_e4),
    ])

# ======================================================================= emptypara
# Empty-paragraph patterns borrowed from prototype/remainder/corpus (see README.md): 226 of 471 body paragraphs
# are empty; runs of 2-4 are common and runs of 7-19 occur (testWORD_2006ml.docx, tdf154481.docx), mostly as
# spacing or to push content to the next page; some are styled (fdo76098.docx, testWORD_various.docx) and some
# carry a section break (tdf124637_sectionMargin.docx).

# ----------------------------------------------------------------------- emptypara 1: official letter (공문)
E1_NAMES = {
    'paragraph_styles': [
        ('기관명', '문서 맨 위 가운데의 큰 기관 이름'),
        ('항목', '번호(1., 2., …)가 붙은 공문 본문 항목'),
        ('붙임', '붙임 목록 줄'),
        ('발신 명의', '가운데 정렬된 크고 굵은 기관장 명의'),
        ('결재란', '문서 끝의 결재·시행 정보 줄 (작은 글씨)'),
        ('좁은 간격', '표 앞뒤에 두는 높이가 낮은 빈 줄'),
        ('붙임 제목', '붙임 쪽 맨 위의 굵은 제목 줄'),
    ],
    'table_styles': [
        ('표 눈금', '모든 칸에 가는 검은 실선'),
        ('눈금 표 4', '회색 머리글 행, 회색 줄무늬 행'),
    ]}

E1 = [
    D('기관명', '○○시교육청'),
    *EN(2),
    P('수신: 수신자 참조'),
    P('(경유)'),
    P('제목: 2027학년도 교원 직무 연수 운영 계획 안내'),
    E(),
    D('항목', '1. 관련: ○○시교육청 교원인사과-4521(2026. 12. 3.)'),
    D('항목', '2. 2027학년도 교원 직무 연수를 아래와 같이 운영하오니, 각 학교에서는 소속 교원이 기한 안에 신청하도록 안내하여 '
      '주시기 바랍니다.'),
    P('가. 연수 기간: 2027. 3. 2. ~ 2027. 11. 30.'),
    P('나. 연수 대상: 관내 초·중·고 교원 (과정별 정원 있음)'),
    P('다. 연수 과정: 4개 과정 (과정별 세부 일정은 붙임 1 참고)'),
    E('좁은 간격'),
    TB('표 눈금', rows('''
과정명 | 대상 | 기간 | 정원 | 학점
디지털 수업 설계 | 초·중등 교원 | 3~5월 | 120명 | 2
학생 상담 실무 | 담임 교원 | 4~6월 | 80명 | 2
학교 안전 관리 | 전 교원 | 연중 | 200명 | 1
교육과정 평가 | 중·고등 교원 | 9~11월 | 60명 | 2''')),
    E('좁은 간격'),
    P('라. 신청 방법: 교원연수 누리집에서 개인별로 신청 (2027. 2. 10.까지). 누리집 신청이 어려운 교원은 붙임 2의 '
      '신청서를 학교 연수 담당자에게 제출합니다.'),
    P('마. 이수 기준: 출석 80% 이상, 과정 평가 60점 이상'),
    P('바. 연수비: 전액 교육청 부담 (숙박이 필요한 과정은 숙박비 포함)'),
    P('사. 유의 사항: 같은 기간에 두 과정 이상을 신청할 수 없으며, 신청 후 취소는 개강 7일 전까지 누리집에서 합니다.'),
    D('항목', '3. 각 학교에서는 연수 대상 교원의 수업 결손이 생기지 않도록 시간표를 조정하고, 연수 기간 중 복무는 출장으로 '
      '처리하여 주시기 바랍니다.'),
    D('항목', '4. 연수 결과는 개인별 연수 이력에 반영되며, 미이수자는 다음 연도에 같은 과정을 신청할 수 없습니다.'),
    D('항목', '5. 올해부터 디지털 수업 설계 과정은 수업 나눔 발표를 이수 요건에 포함하며, 발표 자료는 연수 종료 후 누리집의 '
      '수업 나눔 게시판에 공개합니다. 학교에서는 발표 교원의 수업 공개 일정을 미리 협의하여 주시기 바랍니다.'),
    D('항목', '6. 연수와 관련한 문의는 교원인사과 연수 담당 장학사(031-000-2345)에게 하여 주시기 바랍니다.'),
    *EN(2),
    D('붙임', '붙임 1. 연수 과정별 세부 일정 1부.'),
    D('붙임', '2. 연수 신청서 서식 1부. 끝.'),
    *EN(3),
    D('발신 명의', '○○시교육감'),
    *EN(2),
    D('결재란', '장학사 김민지 / 장학관 박성훈 / 교원인사과장 최현우'),
    D('결재란', '시행: 교원인사과-4602 (2026. 12. 15.) / 접수:'),
    D('결재란', '우 12345 ○○시 ○○로 100 / 전화 031-000-2345 / 전송 031-000-2399 / 공개'),
    *EN(5),
    D('붙임 제목', '붙임 1. 연수 과정별 세부 일정'),
    E(),
    TB('눈금 표 4', rows('''
과정명 | 차시 | 일정 | 장소
디지털 수업 설계 | 1~4차시 | 3. 16. ~ 4. 6. (매주 월) | 교육연수원 301호
^^ | 5~8차시 | 4. 13. ~ 5. 4. (매주 월) | ^^
학생 상담 실무 | 1~6차시 | 4. 7. ~ 5. 12. (매주 화) | 교육연수원 205호
학교 안전 관리 | 1~2차시 | 온라인 상시 | 원격연수 누리집
교육과정 평가 | 1~6차시 | 9. 8. ~ 10. 13. (매주 화) | 교육연수원 302호
^^ | 7~8차시 | 10. 20. ~ 10. 27. (매주 화) | ^^''')),
    E(),
    P('※ 차시별 세부 내용은 연수 시작 2주 전에 누리집에 게시합니다.'),
    P('※ 연수 시간은 모두 16:30 ~ 18:30이며, 첫 차시에는 30분 일찍 와서 등록을 마쳐야 합니다.'),
    P('※ 천재지변 등으로 연수를 할 수 없으면 원격 연수로 바꾸고, 바뀐 내용은 누리집과 문자 메시지로 알립니다.'),
    P('※ 원격 연수는 개인 계정으로 접속하며, 대리 수강이 확인되면 이수를 취소합니다.'),
    E(),
    P('연수 장소 안내'),
    E('좁은 간격'),
    TB('표 눈금', rows('''
장소 | 주소 | 교통
교육연수원 | ○○시 ○○구 연수로 45 | 지하철 2호선 연수원역 3번 출구, 도보 5분
원격연수 누리집 | 누리집 주소는 학교로 따로 안내 | 개인 인증서로 접속''')),
    E('좁은 간격'),
    P('※ 교육연수원에는 주차 공간이 부족하니 대중교통을 이용하여 주시기 바랍니다.'),
    P('※ 장애가 있는 교원은 신청서 비고란에 필요한 지원(수어 통역, 이동 지원 등)을 적어 주시면 준비하겠습니다.'),
    *EN(5),
    D('붙임 제목', '붙임 2. 연수 신청서'),
    E(),
    TB('표 눈금', rows('''
항목 | 내용
성명 |
소속 학교 |
담당 교과 |
신청 과정 |
희망 차시 |
연락처 |
비고 | ''')),
    E(),
    P('개인정보 수집·이용 동의: 연수 운영을 위하여 성명, 소속 학교, 연락처를 수집하며, 연수 종료 후 5년 동안 '
      '보관합니다. 동의하지 않으면 연수를 신청할 수 없습니다.'),
    P('동의 여부: [ ] 동의함 [ ] 동의하지 않음'),
    E(),
    P('위와 같이 2027학년도 교원 직무 연수를 신청합니다.'),
    E(),
    P('2027년 __월 __일'),
    P('신청인: ________ (서명)'),
    *EN(2),
    P('○○학교장 확인: ________ (직인)'),
    P('※ 학교 연수 담당자는 제출받은 신청서를 모아 2027. 2. 12.까지 공문으로 보내 주시기 바랍니다.'),
]


def _after_block(d, text, kind=None):
    for k, b in enumerate(d):
        if b[0] != 'e' and b[-1] == text and (kind is None or b[0] == kind):
            return k
    raise KeyError(text)


def e1_e1(d):
    k = _after_block(d, '○○시교육감')
    assert d[k - 3:k] == EN(3)
    del d[k - 1]


def e1_e2(d):
    k = _after_block(d, '붙임 1. 연수 과정별 세부 일정', 'div')
    j = k + 1
    while d[j][0] != 'table':
        j += 1
    assert d[j + 1] == E()
    d[j + 1] = E('좁은 간격')


def e1_e4(d):
    k = _after_block(d, '붙임 2. 연수 신청서')
    j = k
    while d[j - 1] == E():
        j -= 1
    assert k - j == 5
    d[j:k] = [PB()]


UNITS['emptypara', 1] = dict(
    doc=E1, fm=FORM_FM, stem='letter', names=E1_NAMES,
    write=dict(
        instruction=(
            'Write a new document with exactly these blocks, in this order, and nothing else: the line "○○시교육청" in '
            'the style for the organisation name at the top; two empty paragraphs with the default style; the '
            'paragraphs "수신: 관내 초·중·고등학교장" and "제목: 2027학년도 교원 연수 신청 기간 연장 안내", both with the '
            'default style; one empty paragraph with the default style; the paragraphs "1. 교원 연수 신청 기간을 '
            '2027. 2. 17.까지 연장합니다." and "2. 연장 기간에도 과정별 정원을 넘으면 신청을 마감합니다.", each in the '
            'style for numbered items of the letter, with one empty paragraph in the low spacing style between them; two empty '
            'paragraphs with the default style; the attachment-list line "붙임 1. 변경된 연수 일정 1부. 끝."; three '
            'empty paragraphs with the default style; the head-of-organisation line "○○시교육감".'),
        doc=[
            D('기관명', '○○시교육청'),
            *EN(2),
            P('수신: 관내 초·중·고등학교장'),
            P('제목: 2027학년도 교원 연수 신청 기간 연장 안내'),
            E(),
            D('항목', '1. 교원 연수 신청 기간을 2027. 2. 17.까지 연장합니다.'),
            E('좁은 간격'),
            D('항목', '2. 연장 기간에도 과정별 정원을 넘으면 신청을 마감합니다.'),
            *EN(2),
            D('붙임', '붙임 1. 변경된 연수 일정 1부. 끝.'),
            *EN(3),
            D('발신 명의', '○○시교육감'),
        ]),
    edits=[
        ('There are three empty paragraphs directly before the line "○○시교육감". Keep two of them (delete one). '
         'Change nothing else.', e1_e1),
        ('In 붙임 1, the empty paragraph directly after the table should be in the low spacing style used around '
         'tables. Change nothing else.', e1_e2),
        ('Replace the empty paragraphs before "○○시교육감" with exactly 24pt of space above that line.', None,
         'spacing in points cannot be written'),
        ('붙임 2 is pushed to a new page by the run of empty paragraphs directly before its title line "붙임 2. 연수 '
         '신청서". Replace that whole run with one page break. The run before "붙임 1. 연수 과정별 세부 일정" stays as '
         'it is. Change nothing else.', e1_e4),
    ])

# ----------------------------------------------------------------------- emptypara 2: board meeting minutes
E2_NAMES = {
    'paragraph_styles': [
        ('Title', 'large centered document title'),
        ('Caption', 'title line directly above a table'),
        ('Spacer', 'low empty line between a table and the text after it'),
        ('Signature', 'right-aligned date and signature line'),
        ('Note', 'small gray note'),
    ],
    'table_styles': [
        ('Table Grid', 'thin black lines around all cells'),
        ('Grid Table 4', 'gray header row, gray banded rows'),
    ]}

E2 = [
    D('Title', '제12기 제4차 이사회 의사록'),
    *EN(2),
    P('일시: 2026년 10월 27일(화) 16:00'),
    P('장소: 본사 12층 이사회실'),
    P('출석 이사: 5명 중 5명 (사외이사 2명 포함) / 출석 감사: 1명'),
    E(),
    H(2, '1. 개회'),
    P('의장인 대표이사 김정훈은 정관 제31조에 따라 이사회가 적법하게 성립되었음을 알리고 개회를 선언하였다.'),
    E(),
    H(2, '2. 보고 사항'),
    P('재무 담당 이사 박현주는 2026년 3분기 경영 실적을 다음과 같이 보고하였다.'),
    D('Caption', '표 1. 3분기 경영 실적 (단위: 억 원)'),
    TB('Table Grid', rows('''
구분 | 3분기 | 전년 동기 | 증감률
매출액 | 412 | 385 | +7.0%
영업이익 | 38 | 31 | +22.6%
당기순이익 | 27 | 24 | +12.5%''')),
    E('Spacer'),
    P('매출액은 신규 거래처 확대로 늘었으나, 원자재 가격 상승으로 매출원가율이 1.2%포인트 높아졌다. 영업이익은 '
      '판매관리비 절감 효과로 전년 동기보다 7억 원 늘었다.'),
    P('이사들은 보고 내용을 확인하였으며, 사외이사 정유진은 원자재 가격 상승에 따른 4분기 원가 관리 방안을 다음 회의에서 '
      '보고해 줄 것을 요청하였다.'),
    E(),
    P('감사 오미경은 2026년 3분기 내부 감사 결과를 보고하였다. 구매 업무와 법인카드 사용을 점검한 결과 중대한 지적 '
      '사항은 없었으며, 구매 요청서 결재 누락 2건은 담당 부서에 시정을 요구하였다고 밝혔다.'),
    P('이사들은 감사 결과를 확인하고, 결재 누락이 반복되지 않도록 전자 결재 시스템의 필수 결재 설정을 점검할 것을 '
      '요청하였다.'),
    E(),
    H(2, '3. 의결 사항'),
    P('제1호 의안: 2027년 사업계획 승인의 건'),
    P('의장은 2027년 사업계획안의 주요 내용을 설명하고 심의를 요청하였다. 사업계획안은 매출액 1,750억 원, 영업이익 '
      '160억 원을 목표로 하며, 평택 공장 3라인 증설에 120억 원을 투자하는 내용을 담고 있다.'),
    P('사외이사 정유진은 설비 투자 증가에 따른 차입금 규모를 물었으며, 재무 담당 이사 박현주는 투자금의 절반은 자체 '
      '자금으로, 나머지는 시설 자금 대출로 조달할 계획이라고 답변하였다.'),
    D('Caption', '표 2. 2027년 사업계획 주요 지표 (단위: 억 원)'),
    TB('Table Grid', rows('''
구분 | 2026년 전망 | 2027년 계획 | 증감률
매출액 | 1,610 | 1,750 | +8.7%
영업이익 | 142 | 160 | +12.7%
설비 투자 | 85 | 120 | +41.2%''')),
    E('Spacer'),
    P('심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.'),
    E(),
    P('제2호 의안: 지점 개설의 건'),
    P('의장은 부산 지점 개설 안건을 상정하고 개설 개요를 설명하였다.'),
    D('Caption', '표 3. 지점 개설 개요'),
    TB('Grid Table 4', rows('''
항목 | 내용
지점명 | 부산 지점
소재지 | 부산광역시 해운대구 센텀중앙로 00
개설 예정일 | 2027년 1월 4일
인원 | 6명 (영업 4명, 지원 2명)''')),
    E('Spacer'),
    P('심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.'),
    E(),
    P('제3호 의안: 내부회계관리규정 개정의 건'),
    P('의장은 외부감사법 시행령 개정에 따라 내부회계관리규정 일부를 개정하는 안을 상정하였다.'),
    D('Caption', '표 4. 내부회계관리규정 주요 개정 내용'),
    TB('Table Grid', rows('''
조항 | 현행 | 개정안
제7조 | 운영 실태 보고: 연 1회 | 운영 실태 보고: 반기 1회
제12조 | 평가 주체: 내부감사팀 | 평가 주체: 감사위원회
부칙 | (신설) | 2027년 1월 1일부터 시행''')),
    E('Spacer'),
    P('심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.'),
    E(),
    P('제4호 의안: 2027년 임원 보수 한도 결정의 건'),
    P('의장은 2027년 임원 보수 한도를 2026년과 같은 18억 원으로 정하는 안을 상정하였으며, 출석 이사 전원의 찬성으로 '
      '원안대로 승인하였다. 이 안건은 정기 주주총회에 상정한다.'),
    E(),
    H(2, '4. 폐회'),
    P('의장은 기타 안건이 있는지 물었으며, 사외이사 한동우는 부산 지점 개설에 따른 인력 채용 계획을 다음 이사회에 '
      '보고해 줄 것을 요청하였다. 의장은 이를 받아들였다.'),
    P('의장은 이상으로 회의 목적 사항의 심의를 모두 마쳤음을 알리고 17시 20분에 폐회를 선언하였다.'),
    P('위 의사의 경과와 결과를 명확히 하기 위하여 이 의사록을 작성하고 출석한 이사와 감사가 기명날인한다.'),
    *EN(3),
    D('Signature', '2026년 10월 27일'),
    E(),
    D('Signature', '의장 대표이사 김정훈 (인)'),
    E(),
    D('Signature', '사내이사 박현주 (인)'),
    E(),
    D('Signature', '사내이사 이상민 (인)'),
    E(),
    D('Signature', '사외이사 정유진 (인)'),
    E(),
    D('Signature', '사외이사 한동우 (인)'),
    E(),
    D('Signature', '감사 오미경 (인)'),
    *EN(2),
    D('Note', '이 의사록은 원본 1부를 작성하여 본사 경영지원팀이 보관한다.'),
    D('Note', '첨부: 1. 2027년 사업계획서 1부. 2. 부산 지점 개설 계획서 1부. 3. 내부회계관리규정 개정안 1부.'),
]


def e2_e1(d):
    t = table_after(d, '표 2. 2027년 사업계획 주요 지표 (단위: 억 원)')
    j = d.index(t)
    assert d[j + 1] == E('Spacer')
    d.insert(j + 2, E())


def e2_e2(d):
    k = _after_block(d, '2026년 10월 27일')
    assert d[k - 3:k] == EN(3)
    d[k - 2] = P('(첨부: 2027년 사업계획서 1부)')


def e2_e3(d):
    a = block_index(d, 'h', '3. 의결 사항')
    b = block_index(d, 'h', '4. 폐회')
    d[a:b] = [x for x in d[a:b] if x[0] != 'e']


UNITS['emptypara', 2] = dict(
    doc=E2, fm=DOC_FM, stem='board', names=E2_NAMES,
    write=dict(
        instruction=(
            'Write a new document with exactly these blocks, in this order, and nothing else: the title "이사회 소집 '
            '통지서" in the style for a large centered document title; two empty paragraphs with the default style; '
            'the paragraphs "일시: 2026년 10월 27일(화) 16:00" and "장소: 본사 12층 이사회실", both with the default '
            'style; the line "표 1. 부의 안건" in the file\'s style for a table\'s title line; directly after it a table '
            'with thin black lines around all cells, with the columns 안건, 내용 and the rows 제1호 의안 / 2027년 '
            '사업계획 승인의 건 and 제2호 의안 / 지점 개설의 건; the low empty line used between a table and the text '
            'after it; the paragraph "참석이 어려운 이사는 10월 23일까지 알려 주시기 바랍니다." with the default style; '
            'three empty paragraphs with the default style; the line "2026년 10월 20일" in the style for right-aligned '
            'date and signature lines; one empty paragraph with the default style; the line "대표이사 김정훈" in the '
            'same signature style.'),
        doc=[
            D('Title', '이사회 소집 통지서'),
            *EN(2),
            P('일시: 2026년 10월 27일(화) 16:00'),
            P('장소: 본사 12층 이사회실'),
            D('Caption', '표 1. 부의 안건'),
            TB('Table Grid', rows('''
안건 | 내용
제1호 의안 | 2027년 사업계획 승인의 건
제2호 의안 | 지점 개설의 건''')),
            E('Spacer'),
            P('참석이 어려운 이사는 10월 23일까지 알려 주시기 바랍니다.'),
            *EN(3),
            D('Signature', '2026년 10월 20일'),
            E(),
            D('Signature', '대표이사 김정훈'),
        ]),
    edits=[
        ('Directly after the low spacing line that follows 표 2 (2027년 사업계획 주요 지표), add one empty paragraph '
         'with the default style, so that both come before the next paragraph. Change nothing else.', e2_e1),
        ('Of the three empty paragraphs directly before the date line "2026년 10월 27일", replace the middle one with '
         'the paragraph "(첨부: 2027년 사업계획서 1부)" with the default style; the first and the third stay empty. '
         'Change nothing else.', e2_e2),
        ('Remove every empty paragraph in section 3, between the heading "3. 의결 사항" and the heading "4. 폐회", '
         'including the low spacing lines after the tables. Empty paragraphs elsewhere stay. Change nothing else.',
         e2_e3),
        ('Make each empty paragraph between the signature lines exactly 5 mm high.', None,
         'a height in millimetres cannot be written'),
    ])

# ----------------------------------------------------------------------- emptypara 3: safety inspection report
E3_NAMES = {
    'paragraph_styles': [
        ('표지 제목', '표지 가운데의 크고 굵은 제목'),
        ('표지 정보', '표지 아래쪽의 작성 기관·날짜 줄'),
        ('표 제목', '표 바로 위의 표 제목 줄'),
        ('참고', '작은 회색 글씨의 참고 사항'),
        ('표 아래 간격', '표 바로 아래에 두는 높이가 낮은 빈 줄'),
    ],
    'table_styles': [
        ('표 눈금', '모든 칸에 가는 검은 실선'),
        ('눈금 표 4 - 강조색 1', '파란 머리글 행, 연한 파란 줄무늬 행'),
    ]}

E3 = [
    *EN(7),
    D('표지 제목', '2026년 하반기 사업장 정기 안전 점검 결과 보고서'),
    *EN(5),
    D('표지 정보', '2026. 11.'),
    D('표지 정보', '안전환경팀'),
    *EN(6),
    H(1, '1. 점검 개요'),
    P('산업안전보건법에 따른 위험성 평가의 후속 조치로, 전 사업장을 대상으로 하반기 정기 안전 점검을 실시하였다.'),
    UL('점검 기간: 2026. 10. 12. ~ 10. 30.'),
    UL('점검 대상: 본사, 평택 공장, 이천 물류센터'),
    UL('점검 인원: 안전환경팀 4명, 외부 전문기관 2명'),
    UL('점검 방법: 현장 확인, 관리 대장 검토, 작업자 면담'),
    UL('점검 기준: 사내 안전 점검 기준서(2026년 개정판)와 외부 전문기관 점검표'),
    P('이번 점검은 상반기 점검에서 반복 지적된 항목(비상 대피로, 적재 상태)을 중점적으로 확인하였으며, 평택 공장은 '
      '신규 설치한 3라인 설비를 점검 대상에 추가하였다.'),
    E(),
    D('표 제목', '표 1. 사업장별 점검 항목'),
    TB('표 눈금', rows('''
사업장 | 점검 항목 | 점검일
본사 | 소방 설비, 비상 대피로, 전기 설비 | 10. 12.
평택 공장 | 기계 방호 장치, 화학물질 보관, 소방 설비 | 10. 19. ~ 10. 21.
이천 물류센터 | 지게차 운행, 적재 상태, 비상 대피로 | 10. 28. ~ 10. 30.''')),
    E('표 아래 간격'),
    H(1, '2. 점검 결과'),
    P('지적 사항은 모두 23건으로, 상반기(31건)보다 8건 줄었다. 중대 위험은 평택 공장에서 1건이 확인되어 즉시 조치하였다.'),
    D('표 제목', '표 2. 사업장별 지적 사항 (단위: 건)'),
    TB('눈금 표 4 - 강조색 1', rows('''
사업장 | 중대 | 일반 | 경미 | 합계
본사 | 0 | 2 | 3 | 5
평택 공장 | 1 | 6 | 4 | 11
이천 물류센터 | 0 | 3 | 4 | 7
합계 | 1 | 11 | 11 | 23''')),
    E('표 아래 간격'),
    P('상반기와 비교하면 소방 설비와 전기 설비 분야의 지적은 크게 줄었으나, 기계 방호 장치 분야는 3건에서 5건으로 '
      '늘었다. 신규 설비의 방호 장치 점검 절차가 아직 정착되지 않은 것이 원인으로 보인다.'),
    P('사업장별로는 평택 공장의 지적 사항이 가장 많았으며, 이 가운데 4건은 신규 설치한 3라인에서 나왔다. 이천 '
      '물류센터는 적재 관련 지적이 상반기 5건에서 3건으로 줄었다.'),
    P('주요 지적 사항은 다음과 같다.'),
    UL('평택 공장 2라인 프레스 방호 덮개 파손 (중대, 10. 19. 즉시 교체)'),
    UL('이천 물류센터 랙 상단 적재 높이 기준 초과 (일반)'),
    UL('본사 3층 비상구 앞 물품 적치 (일반)'),
    E(),
    H(1, '3. 조치 계획'),
    P('지적 사항은 위험 수준에 따라 조치 기한을 정하였다. 중대 위험은 발견 즉시, 일반은 30일, 경미는 45일 안에 '
      '조치한다.'),
    D('표 제목', '표 3. 지적 사항 조치 계획'),
    TB('표 눈금', rows('''
구분 | 조치 내용 | 담당 | 기한
중대 | 방호 덮개 교체 및 전 라인 점검 | 평택 공장 생산팀 | 완료
일반 | 적재 기준 재교육, 비상구 상시 점검 | 각 사업장 관리팀 | 11. 30.
경미 | 표지판 교체, 소화기 위치 조정 | 각 사업장 관리팀 | 12. 15.''')),
    D('참고', '조치 결과는 12월 안전보건위원회에 보고한다.'),
    P('일반 지적 사항 중 적재 기준 초과는 11월 첫 주에 해당 구역 작업자 전원을 대상으로 재교육을 실시하였고, 비상구 앞 '
      '적치는 관리팀이 매일 오후 순회 점검한다.'),
    E(),
    H(1, '4. 안전 교육 실적'),
    P('하반기 법정 안전 교육 이수율은 전 사업장 평균 97%로, 상반기(93%)보다 높아졌다. 미이수자는 11월 중 보충 교육을 '
      '받는다.'),
    D('표 제목', '표 4. 사업장별 안전 교육 이수율'),
    TB('표 눈금', rows('''
사업장 | 대상 인원 | 이수 인원 | 이수율
본사 | 120 | 118 | 98%
평택 공장 | 210 | 202 | 96%
이천 물류센터 | 85 | 83 | 98%''')),
    E('표 아래 간격'),
    P('평택 공장은 교대 근무자의 교육 시간을 맞추기 어려워 야간 교육반을 따로 운영하였다. 이천 물류센터는 신규 입사자 '
      '12명에게 지게차 안전 교육을 따로 실시하였다.'),
    P('2027년에는 사업장별 교육 담당자를 1명씩 지정하고, 교육 자료를 사내 교육 시스템에서 함께 쓰도록 할 계획이다.'),
    E(),
    H(1, '5. 향후 계획'),
    P('2027년 상반기 점검은 4월에 실시하며, 이천 물류센터는 지게차 운행 구역을 중점 점검한다. 평택 공장 3라인은 '
      '가동 6개월이 되는 3월에 설비 제조사와 함께 특별 점검을 실시한다.'),
    P('점검 결과는 사내 게시판에 공개하고, 사업장별 안전 교육 자료로 활용한다. 반복 지적된 항목은 사업장장 평가에 '
      '반영하는 방안을 안전보건위원회에서 논의한다.'),
    UL('지게차 운행 구역에 보행자 통로를 따로 표시한다.'),
    UL('랙 적재 높이 기준을 현장에 게시하고, 월 1회 자체 점검한다.'),
    UL('비상구 앞 적치 금지 구역을 바닥에 표시한다.'),
    UL('신규 설비는 설치 후 1개월 안에 방호 장치 점검표를 작성하여 안전환경팀에 제출한다.'),
    P('외부 전문기관의 점검 의견서는 안전환경팀이 보관하며, 요청하면 열람할 수 있다.'),
    *EN(2),
    P('붙임: 사업장별 점검표 3부. 끝.'),
    *EN(3),
    D('참고', '작성: 안전환경팀 대리 서지훈 / 검토: 안전환경팀장 문경수 / 배포: 각 사업장장, 안전보건위원회'),
]


def e3_e1(d):
    k = _after_block(d, '안전환경팀')
    assert d[k + 1:k + 7] == EN(6) and d[k + 7][0] == 'h'
    d[k + 1:k + 7] = [PB()]


def e3_e2(d):
    t = table_after(d, '표 3. 지적 사항 조치 계획')
    j = d.index(t)
    d.insert(j + 1, E('표 아래 간격'))


def e3_e3(d):
    assert d[:7] == EN(7)
    del d[:2]


UNITS['emptypara', 3] = dict(
    doc=E3, fm=DOC_FM, stem='report', names=E3_NAMES,
    write=dict(
        instruction=(
            'Write a new document with exactly these blocks, in this order, and nothing else: seven empty paragraphs '
            'with the default style; the cover title "2027년 상반기 사업장 정기 안전 점검 계획" in the style for the '
            'large cover title; five empty paragraphs with the default style; the lines "2027. 3." and "안전환경팀", '
            'each in the style for the cover\'s organisation and date lines; a page break; a heading level 1 "1. 점검 '
            '개요"; the paragraph "상반기 점검은 4월 한 달 동안 실시한다." with the default style; the line "표 1. 점검 '
            '일정" in the file\'s style for a table\'s title line; directly after it a table with a blue header row and '
            'light blue banded rows, with the columns 사업장, 점검일 and the rows 본사 / 4. 6. and 평택 공장 / 4. 13. ~ '
            '4. 15.; the low empty line used directly below a table.'),
        doc=[
            *EN(7),
            D('표지 제목', '2027년 상반기 사업장 정기 안전 점검 계획'),
            *EN(5),
            D('표지 정보', '2027. 3.'),
            D('표지 정보', '안전환경팀'),
            PB(),
            H(1, '1. 점검 개요'),
            P('상반기 점검은 4월 한 달 동안 실시한다.'),
            D('표 제목', '표 1. 점검 일정'),
            TB('눈금 표 4 - 강조색 1', rows('''
사업장 | 점검일
본사 | 4. 6.
평택 공장 | 4. 13. ~ 4. 15.''')),
            E('표 아래 간격'),
        ]),
    edits=[
        ('The run of empty paragraphs after the cover\'s "안전환경팀" line pushes the report body to the next page. '
         'Replace that whole run with one page break. Change nothing else.', e3_e1),
        ('표 3 (지적 사항 조치 계획) is followed directly by the 참고 paragraph. Put one empty paragraph in the style '
         'used directly below tables between them, as after the other tables. Change nothing else.', e3_e2),
        ('The cover page has seven empty paragraphs before its title. Delete two of them, so that five remain. Change '
         'nothing else.', e3_e3),
        ('Make the empty paragraphs on the cover page 6pt high each, so that the title sits higher on the page.', None,
         'a height in points cannot be written'),
    ])
