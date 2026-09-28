# Seed content, tasks and intended results, in one syntax-neutral form.
# Document blocks: ('h', level, text) ('p', text) ('div', style, text) ('ul', text) ('ol', text)
#                  ('table', style_or_None, rows)   rows[0] is the header row; a cell is str or (text, rowspan, colspan)
# Presentation:    [(layout, [(slot, [lines])])]
import copy

DOC_FM = {'type': 'document', 'format': 'docx', 'template': 'org/report', 'schema': '1'}
DECK_FM = {'type': 'presentation', 'format': 'pptx', 'template': 'org/sales-deck', 'schema': '1'}

EN_LAYOUTS = {'Title Slide': ['title', 'body'], 'Title and Content': ['title', 'body'],
              'Two Content': ['title', 'left', 'right'], 'Section Header': ['title', 'body'],
              'Title Only': ['title']}
KO_LAYOUTS = {'제목 슬라이드': ['title', 'body'], '제목 및 내용': ['title', 'body'],
              '콘텐츠 2개': ['title', 'left', 'right'], '구역 머리글': ['title', 'body'], '제목만': ['title']}


def cp(x):
    return copy.deepcopy(x)


def find_table(blocks, header0):
    for b in blocks:
        if b[0] == 'table' and (b[2][0][0] if isinstance(b[2][0][0], str) else b[2][0][0][0]) == header0:
            return b
    raise KeyError(header0)


def find_block(blocks, text):
    for i, b in enumerate(blocks):
        if b[0] in ('p', 'ul', 'ol', 'h') and b[-1] == text:
            return i
        if b[0] == 'div' and b[2] == text:
            return i
    raise KeyError(text)


def cell_text(c):
    return c if isinstance(c, str) else c[0]


UNITS = {}

# =============================================================== merge
# ---------------------------------------------------------------- merge 1: quarterly sales report
m1 = [
    ('h', 1, '2026년 3분기 지역별 영업 실적 보고'),
    ('p', '작성: 영업기획팀 / 보고일: 2026-10-05'),
    ('h', 2, '1. 요약'),
    ('p', '3분기 전사 매출은 5,012백만 원으로 전년 동기 대비 4.8% 증가했다.'),
    ('p', '수도권과 영남권이 성장을 이끌었고, 호남권과 제주는 전년 수준에 머물렀다.'),
    ('h', 2, '2. 지역별 실적 (단위: 백만 원)'),
    ('table', None, [
        [('구분', 1, 2), '7월', '8월', '9월', '합계'],
        [('수도권', 3, 1), '서울 본점', '412', '398', '455', '1,265'],
        ['경기 지점', '287', '301', '296', '884'],
        ['인천 지점', '156', '162', '170', '488'],
        [('영남권', 2, 1), '부산 지점', '301', '287', '312', '900'],
        ['대구 지점', '198', '205', '211', '614'],
        ['호남권', '광주 지점', '162', '156', '170', '488'],
        ['제주', '제주 지점', '124', '131', '118', '373'],
        [('전사 합계', 1, 2), '1,640', '1,640', '1,732', '5,012'],
    ]),
    ('p', '8월 수치는 반품 정산 후 확정치이다.'),
    ('h', 2, '3. 주요 거래처 현황'),
    ('table', None, [
        ['거래처', '담당', '계약 상태', '비고'],
        ['한빛유통', '김민수', '갱신 완료', '3년 계약'],
        ['대성상사', '김민수', '갱신 협의 중', '단가 조정 요청'],
        ['동해물산', '이서연', '신규', '9월 첫 발주'],
        ['서해식품', '이서연', '갱신 협의 중', '단가 조정 요청'],
    ]),
    ('h', 2, '4. 향후 계획'),
    ('ul', '4분기 수도권 신규 거래처 5곳 확보'),
    ('ul', '호남권 판촉 행사 2회 진행'),
    ('ul', '단가 조정 요청 거래처 대응 방안 수립'),
]


def m1_e1(d):
    t = find_table(d, '거래처')[2]
    t[1][1] = ('김민수', 2, 1)
    del t[2][1]


def m1_e2(d):
    t = find_table(d, '구분')[2]
    vals = ['전년 동기', '1,180', '842', '455', '861', '590', '502', '351', '4,781']
    for row, v in zip(t, vals):
        row.append(v)


def m1_e3(d):
    t = find_table(d, '구분')[2]
    assert t[4][3] == '287'
    t[4][3] = '293'


UNITS['merge', 1] = dict(
    doc=m1, fm=DOC_FM,
    write=dict(
        instruction=(
            'Write a new document. Its content is a heading level 1 "하반기 신입사원 교육 일정", followed by one '
            'table with the columns 구분, 과정, 일시, 장소 and these five rows (cells separated by " / "):\n'
            '1. 공통 교육 / 회사 소개 / 10월 6일 오전 / 본사 대강당\n'
            '2. 공통 교육 / 정보보안 / 10월 6일 오후 / 본사 대강당\n'
            '3. 직무 교육 / 영업 실무 / 10월 7일 / 영업본부 회의실\n'
            '4. 직무 교육 / 품질 관리 / 10월 8일 / 생산본부 회의실\n'
            '5. 비고: 교육 불참 시 팀장 승인 필요\n'
            'Merged cells: the two 공통 교육 cells are one cell; the two 본사 대강당 cells are one cell; the two '
            '직무 교육 cells are one cell; row 5 is a single cell covering all four columns.'),
        doc=[
            ('h', 1, '하반기 신입사원 교육 일정'),
            ('table', None, [
                ['구분', '과정', '일시', '장소'],
                [('공통 교육', 2, 1), '회사 소개', '10월 6일 오전', ('본사 대강당', 2, 1)],
                ['정보보안', '10월 6일 오후'],
                [('직무 교육', 2, 1), '영업 실무', '10월 7일', '영업본부 회의실'],
                ['품질 관리', '10월 8일', '생산본부 회의실'],
                [('비고: 교육 불참 시 팀장 승인 필요', 1, 4)],
            ]),
        ]),
    edits=[
        ('In the table under "3. 주요 거래처 현황", the 담당 cells of 한빛유통 and 대성상사 both read 김민수. '
         'Merge these two cells into one cell reading 김민수. Change nothing else.', m1_e1),
        ('In the table under "2. 지역별 실적", add a new last column with the header 전년 동기 and these values: '
         '서울 본점 1,180; 경기 지점 842; 인천 지점 455; 부산 지점 861; 대구 지점 590; 광주 지점 502; '
         '제주 지점 351; 전사 합계 row 4,781.', m1_e2),
        ('In the table under "2. 지역별 실적", correct the 8월 value of 부산 지점 from 287 to 293. '
         'Change nothing else.', m1_e3),
    ])

# ---------------------------------------------------------------- merge 2: weekly update
m2 = [
    ('h', 1, '개발본부 주간 업무 보고 (2026년 9월 4주차)'),
    ('p', '작성: 개발기획파트 / 기간: 9월 21일 ~ 9월 25일'),
    ('h', 2, '1. 금주 요약'),
    ('ul', '모바일 앱 4.2 버전 출시 완료'),
    ('ul', '결제 모듈 장애 1건 발생, 2시간 내 복구'),
    ('ul', '추석 연휴 대비 비상 연락망 점검'),
    ('h', 2, '2. 팀별 실적 및 계획'),
    ('table', None, [
        ['팀', '업무', '금주 실적', '차주 계획', '진척률'],
        [('개발1팀', 3, 1), '모바일 앱 4.2', '출시 완료', '모니터링', '100%'],
        ['푸시 알림 개선', '설계 검토', '개발 착수', '20%'],
        ['결제 모듈 안정화', '장애 원인 분석', '재발 방지 패치', '60%'],
        [('개발2팀', 2, 1), '관리자 웹 개편', '화면 개발', '화면 개발', '60%'],
        ['API 문서화', ('휴가로 일정 순연 (9/29~10/2)', 1, 2), '40%'],
        [('QA팀', 2, 1), '4.2 회귀 테스트', '완료', '없음', '100%'],
        ['결제 모듈 검증', '테스트 케이스 작성', '검증 수행', '20%'],
    ]),
    ('p', '진척률은 팀장 확인 기준이다.'),
    ('h', 2, '3. 이슈 및 요청 사항'),
    ('table', None, [
        ['구분', '내용', '요청 부서', '기한'],
        ['장애', '결제 모듈 타임아웃 재발 방지', '개발1팀', '10월 2일'],
        ['인력', 'QA 인력 1명 추가 지원', 'QA팀', '10월 10일'],
        ['일정', '관리자 웹 오픈 일정 확정', '개발2팀', '10월 2일'],
    ]),
    ('h', 2, '4. 차주 일정'),
    ('ul', '9월 29일(월) 결제 모듈 패치 배포'),
    ('ul', '10월 1일(수) 관리자 웹 중간 리뷰'),
]


def m2_e1(d):
    t = find_table(d, '팀')[2]
    row = t[4]
    assert row[2] == '화면 개발' and row[3] == '화면 개발'
    row[2:4] = [('화면 개발 (계속)', 1, 2)]


def m2_e2(d):
    t = find_table(d, '팀')[2]
    vals = ['비고', '-', '디자인팀 협업', '장애 보고서 첨부', '-', '-', '-', 'QA 인력 요청 중']
    for row, v in zip(t, vals):
        row.insert(len(row) - 1, v)


def m2_e3(d):
    t = find_table(d, '팀')[2]
    assert t[3][-1] == '60%'
    t[3][-1] = '70%'


UNITS['merge', 2] = dict(
    doc=m2, fm=DOC_FM,
    write=dict(
        instruction=(
            'Write a new document. Its content is a heading level 1 "10월 당직 근무표", then one table with the '
            'columns 주차, 요일, 주간 담당, 야간 담당 and these three rows (cells separated by " / "):\n'
            '1. 1주차 / 월~수 / 박지훈 / 최유진\n'
            '2. 1주차 / 목~금 / 박지훈 / 정하늘\n'
            '3. 2주차 / 월~금 / 외부 위탁 (한결시큐리티)\n'
            'Merged cells: the two 1주차 cells are one cell; the two 박지훈 cells are one cell; in row 3, '
            '"외부 위탁 (한결시큐리티)" is one cell covering the 주간 담당 and 야간 담당 columns. '
            'After the table, a paragraph "비상 연락: 경영지원팀 내선 2020".'),
        doc=[
            ('h', 1, '10월 당직 근무표'),
            ('table', None, [
                ['주차', '요일', '주간 담당', '야간 담당'],
                [('1주차', 2, 1), '월~수', ('박지훈', 2, 1), '최유진'],
                ['목~금', '정하늘'],
                ['2주차', '월~금', ('외부 위탁 (한결시큐리티)', 1, 2)],
            ]),
            ('p', '비상 연락: 경영지원팀 내선 2020'),
        ]),
    edits=[
        ('In the table under "2. 팀별 실적 및 계획", the row 관리자 웹 개편 has 화면 개발 in both the 금주 실적 '
         'and 차주 계획 columns. Merge these two cells into one cell reading 화면 개발 (계속).', m2_e1),
        ('In the table under "2. 팀별 실적 및 계획", add a column 비고 between 차주 계획 and 진척률, with these '
         'values: 모바일 앱 4.2 "-"; 푸시 알림 개선 "디자인팀 협업"; 결제 모듈 안정화 "장애 보고서 첨부"; '
         '관리자 웹 개편 "-"; API 문서화 "-"; 4.2 회귀 테스트 "-"; 결제 모듈 검증 "QA 인력 요청 중".', m2_e2),
        ('In the table under "2. 팀별 실적 및 계획", change the 진척률 of 결제 모듈 안정화 from 60% to 70%. '
         'Change nothing else.', m2_e3),
    ])

# ---------------------------------------------------------------- merge 3: proposal
m3 = [
    ('h', 1, "신제품 '하루견과' 출시 마케팅 제안서"),
    ('p', '제안 부서: 마케팅전략팀 / 제안일: 2026-09-28'),
    ('h', 2, '1. 제안 배경'),
    ('p', '간편 건강 간식 시장은 최근 3년간 연평균 11% 성장했다.'),
    ('p', '당사 견과류 제품군은 대형마트 매출 비중이 높아 온라인 채널 보완이 필요하다.'),
    ('h', 2, '2. 소요 예산 (단위: 만 원)'),
    ('table', None, [
        ['항목', '세부 내역', '금액', '집행 시기'],
        [('마케팅', 3, 1), '온라인 광고', '1,500', ('11월', 2, 1)],
        ['오프라인 시식 행사', '800'],
        ['인플루언서 협업', '500', '12월'],
        [('제작', 2, 1), '패키지 디자인', '300', '10월'],
        ['초도 생산', '2,400', '10월'],
        ['운영', '물류 대행', '500', '11월~12월'],
        [('합계', 1, 2), '6,000', '-'],
    ]),
    ('p', '예산은 2026년 하반기 마케팅 예비비에서 집행한다.'),
    ('h', 2, '3. 추진 일정'),
    ('table', None, [
        ['단계', '기간', '주요 내용'],
        ['준비', '10월 1일 ~ 10월 31일', '패키지 확정, 초도 생산'],
        ['출시', '11월 1일 ~ 11월 15일', '온라인 광고, 시식 행사'],
        ['확산', '11월 16일 ~ 12월 31일', '인플루언서 협업, 재구매 쿠폰'],
    ]),
    ('h', 2, '4. 기대 효과'),
    ('ul', '출시 3개월 매출 4억 원'),
    ('ul', '온라인 매출 비중 25%까지 확대'),
]


def m3_e1(d):
    t = find_table(d, '항목')[2]
    assert t[4][3] == '10월' and t[5][2] == '10월'
    t[4][3] = ('10월', 2, 1)
    del t[5][2]


def m3_e2(d):
    t = find_table(d, '항목')[2]
    t[0].insert(2, '담당 부서')
    t[1].insert(2, ('마케팅전략팀', 3, 1))
    t[4].insert(2, '디자인팀')
    t[5].insert(1, '생산관리팀')
    t[6].insert(2, 'SCM팀')
    t[7][0] = ('합계', 1, 3)


def m3_e3(d):
    t = find_table(d, '항목')[2]
    assert t[3][1] == '500'
    t[3][1] = '650'


UNITS['merge', 3] = dict(
    doc=m3, fm=DOC_FM,
    write=dict(
        instruction=(
            'Write a new document. Its content is a heading level 1 "교육비 지원 기준", a paragraph '
            '"2027년 1월 1일부터 적용한다.", and then one table with the columns 구분, 대상, 지원 한도, 비고 and '
            'these four rows (cells separated by " / "):\n'
            '1. 직무 교육 / 전 직원 / 연 200만 원 / 사전 승인 필요\n'
            '2. 직무 교육 / 팀장 이상 / 연 300만 원 / 사전 승인 필요\n'
            '3. 자격증 / 전 직원 / 응시료 전액 / 합격 시 지급\n'
            '4. 어학 / 전 직원 / 월 10만 원 / 합격 시 지급\n'
            'Merged cells: the two 직무 교육 cells are one cell; the two 사전 승인 필요 cells are one cell; the two '
            '합격 시 지급 cells are one cell. The 전 직원 cells stay separate.'),
        doc=[
            ('h', 1, '교육비 지원 기준'),
            ('p', '2027년 1월 1일부터 적용한다.'),
            ('table', None, [
                ['구분', '대상', '지원 한도', '비고'],
                [('직무 교육', 2, 1), '전 직원', '연 200만 원', ('사전 승인 필요', 2, 1)],
                ['팀장 이상', '연 300만 원'],
                ['자격증', '전 직원', '응시료 전액', ('합격 시 지급', 2, 1)],
                ['어학', '전 직원', '월 10만 원'],
            ]),
        ]),
    edits=[
        ('In the table under "2. 소요 예산", the 집행 시기 cells of 패키지 디자인 and 초도 생산 both read 10월. '
         'Merge these two cells into one cell reading 10월. Change nothing else.', m3_e1),
        ('In the table under "2. 소요 예산", add a column 담당 부서 right after 세부 내역. For the three 마케팅 '
         'rows it is one merged cell reading 마케팅전략팀; 패키지 디자인: 디자인팀; 초도 생산: 생산관리팀; '
         '물류 대행: SCM팀. The 합계 cell in the last row then covers the 항목, 세부 내역 and 담당 부서 '
         'columns.', m3_e2),
        ('In the table under "2. 소요 예산", change the 금액 of 인플루언서 협업 from 500 to 650. '
         'Change nothing else.', m3_e3),
    ])

# =============================================================== styleattr
S1_NAMES = {
    'paragraph_styles': [('Note', '회색 배경의 참고 문단'), ('Decision', '굵은 글씨의 결정 사항 문단'),
                         ('Alert Box', '빨간색 굵은 큰 글씨의 경고 문단'), ('Body Text Indent', '들여 쓴 본문'),
                         ('Caption', '작은 회색 글씨의 설명 문단'), ('Intense Quote', '위아래 테두리가 있는 강조 인용 문단')],
    'table_styles': [('Table Grid', '기본 격자'), ('Grid Table 4', '머리글 행이 진한 격자'),
                     ('List Table 3 Accent 1', '파란 머리글의 목록형 표')],
}
s1 = [
    ('h', 1, '제38차 주간 경영회의 회의록'),
    ('div', 'Caption', '일시: 2026-09-21(월) 09:00 / 장소: 본사 12층 대회의실'),
    ('h', 2, '1. 참석자'),
    ('p', '대표이사, 경영지원본부장, 영업본부장, 연구소장, 재무팀장'),
    ('h', 2, '2. 안건별 논의'),
    ('h', 3, '안건 1. 4분기 채용 계획'),
    ('div', 'Body Text Indent', '연구소 개발 인력 6명, 영업본부 2명 충원을 요청함.'),
    ('div', 'Note', '인건비 증가분은 4분기 예산 범위 내에서 집행.'),
    ('div', 'Decision', '결정: 연구소 6명 우선 채용, 영업본부는 11월 재검토.'),
    ('h', 3, '안건 2. 판교 사무실 이전'),
    ('div', 'Body Text Indent', '판교 사무실 임대 계약이 2027년 2월 만료됨.'),
    ('div', 'Note', '이전 비용은 4분기 예산 범위 내에서 집행.'),
    ('div', 'Decision', '결정: 후보지 3곳 실사 후 10월 회의에서 확정.'),
    ('h', 3, '안건 3. 보안 점검 결과'),
    ('p', '정보보안 점검 결과 외부 반출 의심 건 1건이 확인됨.'),
    ('p', '재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.'),
    ('h', 2, '3. 후속 조치'),
    ('table', 'Grid Table 4', [
        ['조치 사항', '담당', '기한'],
        ['채용 공고 게시', '경영지원본부', '9월 25일'],
        ['후보지 실사', '경영지원본부', '10월 10일'],
        ['보안 교육 계획 수립', '정보보안팀', '9월 30일'],
    ]),
    ('div', 'Caption', '다음 회의: 2026-09-28(월) 09:00'),
]


def s1_e1(d):
    i = find_block(d, '이전 비용은 4분기 예산 범위 내에서 집행.')
    d[i] = ('div', 'Body Text Indent', d[i][2])


def s1_e2(d):
    i = find_block(d, '재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.')
    d[i] = ('div', 'Alert Box', d[i][1])


def s1_e3(d):
    i = find_block(d, '재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.')
    d.insert(i + 1, ('div', 'Intense Quote', '보안 교육 미이수자는 사내 시스템 접속이 제한된다.'))


UNITS['styleattr', 1] = dict(
    doc=s1, fm=DOC_FM, names=S1_NAMES,
    write=dict(
        instruction=(
            'Write a new document with, in this order: a heading level 1 "10월 전사 워크숍 안내"; a paragraph in the '
            'default style "10월 17일(금) 전 직원 대상 워크숍을 진행합니다."; a paragraph in style Note '
            '"장소: 가평 한빛연수원 / 버스 출발: 본사 정문 08:00"; a paragraph in style Alert Box '
            '"불참자는 10월 10일까지 팀장에게 사유를 제출해야 합니다."; and a table in style List Table 3 Accent 1 '
            'with the header row 시간 / 프로그램 and the rows 10:00 / 대표이사 인사말, 11:00 / 팀별 발표, '
            '14:00 / 팀 빌딩.'),
        doc=[
            ('h', 1, '10월 전사 워크숍 안내'),
            ('p', '10월 17일(금) 전 직원 대상 워크숍을 진행합니다.'),
            ('div', 'Note', '장소: 가평 한빛연수원 / 버스 출발: 본사 정문 08:00'),
            ('div', 'Alert Box', '불참자는 10월 10일까지 팀장에게 사유를 제출해야 합니다.'),
            ('table', 'List Table 3 Accent 1', [['시간', '프로그램'], ['10:00', '대표이사 인사말'],
                                                ['11:00', '팀별 발표'], ['14:00', '팀 빌딩']]),
        ]),
    edits=[
        ('Under 안건 2, the paragraph "이전 비용은 4분기 예산 범위 내에서 집행." should have the style '
         'Body Text Indent instead of its current style. Change nothing else.', s1_e1),
        ('The CEO asked: make the sentence "재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다." red, bold and '
         'large so that it stands out. Change nothing else.', s1_e2),
        ('Right after the paragraph "재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.", add a new paragraph '
         'in style Intense Quote reading "보안 교육 미이수자는 사내 시스템 접속이 제한된다."', s1_e3),
    ])

S2_NAMES = {
    'paragraph_styles': [('Subtitle', '제목 아래 회색 부제'), ('Key Message', '파란색 굵은 16pt 강조 문단'),
                         ('Note', '회색 배경의 참고 문단'), ('Block Text', '테두리 상자 안의 본문'),
                         ('Source Note', '작은 기울임 글씨의 출처 표기')],
    'table_styles': [('Table Grid', '기본 격자'), ('Grid Table 4 Accent 5', '청록 머리글의 격자'),
                     ('Plain Table 1', '가는 선만 있는 표')],
}
s2 = [
    ('h', 1, '스마트 물류센터 구축 제안서'),
    ('div', 'Subtitle', '경기 이천 제2물류센터 자동화 사업'),
    ('h', 2, '1. 추진 배경'),
    ('p', '온라인 주문량이 최근 2년간 2.3배 증가해 기존 센터의 처리 한계에 도달했다.'),
    ('p', '피크 시즌 출고 지연률은 7.8%로 목표치 3%를 크게 웃돌았다.'),
    ('div', 'Source Note', '출처: 2026년 상반기 물류 운영 보고서'),
    ('h', 2, '2. 제안 내용'),
    ('div', 'Block Text', '자동 분류기(소터) 2기와 AMR 40대를 도입해 출고 공정을 자동화한다.'),
    ('div', 'Block Text', 'WMS를 클라우드형으로 전환해 재고 정확도를 99.5% 이상으로 유지한다.'),
    ('p', '도입 후 시간당 처리량은 4,000박스에서 9,000박스로 늘어난다.'),
    ('table', 'Grid Table 4 Accent 5', [
        ['구분', '현재', '도입 후'],
        ['시간당 처리량', '4,000박스', '9,000박스'],
        ['출고 지연률', '7.8%', '2% 이하'],
        ['필요 인력', '120명', '85명'],
    ]),
    ('div', 'Source Note', '출처: 설비 공급사 제안 자료 (2026-08)'),
    ('h', 2, '3. 투자 및 일정'),
    ('p', '총 투자비는 86억 원이며, 2027년 3월 착공해 2027년 10월 가동을 목표로 한다.'),
    ('p', '투자비 회수 기간은 약 4.2년으로 예상된다.'),
    ('div', 'Note', '세부 견적은 공급사 2곳의 최종 제안서 접수 후 확정한다.'),
]


def s2_e1(d):
    t = find_table(d, '구분')
    i = d.index(t)
    d[i] = ('table', 'Plain Table 1', t[2])


def s2_e2(d):
    i = find_block(d, '도입 후 시간당 처리량은 4,000박스에서 9,000박스로 늘어난다.')
    d[i] = ('div', 'Key Message', d[i][1])


def s2_e3(d):
    i = find_block(d, '출처: 2026년 상반기 물류 운영 보고서')
    d.insert(i + 1, ('div', 'Block Text', '센터 확장 없이 처리 능력을 두 배 이상 늘리는 것이 이번 제안의 목표이다.'))


UNITS['styleattr', 2] = dict(
    doc=s2, fm=DOC_FM, names=S2_NAMES,
    write=dict(
        instruction=(
            'Write a new document with, in this order: a heading level 1 "물류센터 견학 일정 안내"; a paragraph in '
            'style Subtitle "협력사 대상 / 2026년 11월"; a paragraph in the default style '
            '"견학은 회차별 20명 이내로 운영합니다."; a table in style Grid Table 4 Accent 5 with the header row '
            '회차 / 일시 / 대상 and the rows 1회차 / 11월 5일 14:00 / 신규 협력사 and 2회차 / 11월 12일 14:00 / '
            '기존 협력사; and a paragraph in style Source Note "문의: 물류기획팀 내선 3120".'),
        doc=[
            ('h', 1, '물류센터 견학 일정 안내'),
            ('div', 'Subtitle', '협력사 대상 / 2026년 11월'),
            ('p', '견학은 회차별 20명 이내로 운영합니다.'),
            ('table', 'Grid Table 4 Accent 5', [['회차', '일시', '대상'], ['1회차', '11월 5일 14:00', '신규 협력사'],
                                                ['2회차', '11월 12일 14:00', '기존 협력사']]),
            ('div', 'Source Note', '문의: 물류기획팀 내선 3120'),
        ]),
    edits=[
        ('Give the comparison table (구분 / 현재 / 도입 후) the table style Plain Table 1 instead of its current '
         'style. Change nothing else.', s2_e1),
        ('Make the sentence "도입 후 시간당 처리량은 4,000박스에서 9,000박스로 늘어난다." stand out in bold, blue, '
         '16pt text. Change nothing else.', s2_e2),
        ('Right after the first 출처 paragraph ("출처: 2026년 상반기 물류 운영 보고서"), add a new paragraph in '
         'style Block Text reading "센터 확장 없이 처리 능력을 두 배 이상 늘리는 것이 이번 제안의 목표이다."', s2_e3),
    ])

S3_NAMES = {
    'paragraph_styles': [('본문', '기본 본문'), ('참고 문단', '회색 상자의 참고 사항'),
                         ('강조 문단', '빨간색 굵은 큰 글씨의 강조 문단'), ('인용 문단', '들여 쓴 기울임 인용'),
                         ('붙임 목록', '첨부 목록 문단'), ('발신 명의', '가운데 정렬 큰 글씨의 발신 명의')],
    'table_styles': [('기본 표', '기본 격자'), ('격자 표 4', '머리글 행이 진한 격자'),
                     ('목록 표 강조 2', '주황 머리글의 목록형 표')],
}
s3 = [
    ('h', 1, '개인정보 처리방침 개정 안내'),
    ('p', '수신: 전 부서장 / 참조: 정보보호위원회'),
    ('h', 2, '1. 개정 사유'),
    ('p', '개인정보 보호법 시행령 개정(2026. 9. 15. 시행)에 따라 처리방침을 개정합니다.'),
    ('div', '인용 문단', '개인정보처리자는 처리 목적이 달성된 개인정보를 지체 없이 파기하여야 한다.'),
    ('h', 2, '2. 주요 개정 내용'),
    ('table', None, [
        ['구분', '현행', '개정'],
        ['보유 기간', '회원 탈퇴 후 1년', '회원 탈퇴 후 6개월'],
        ['파기 절차', '분기별 일괄 파기', '월별 일괄 파기'],
        ['위탁 업체', '3곳', '4곳'],
    ]),
    ('div', '참고 문단', '위탁 업체 추가: 한결고객센터(고객 상담)'),
    ('div', '참고 문단', '개정 방침은 2026년 10월 1일부터 적용합니다.'),
    ('h', 2, '3. 부서별 조치 사항'),
    ('ol', '보유 중인 탈퇴 회원 정보 점검 (9월 30일까지)'),
    ('ol', '위탁 계약서 개인정보 조항 갱신 (10월 15일까지)'),
    ('ol', '부서 내 교육 실시 후 결과 보고 (10월 31일까지)'),
    ('p', '기한 내 미조치 부서는 정보보호위원회에 보고됩니다.'),
    ('div', '붙임 목록', '붙임: 개정 처리방침 전문 1부. 끝.'),
    ('div', '발신 명의', '정보보호책임자'),
]


def s3_e1(d):
    t = find_table(d, '구분')
    i = d.index(t)
    d[i] = ('table', '목록 표 강조 2', t[2])


def s3_e2(d):
    i = find_block(d, '기한 내 미조치 부서는 정보보호위원회에 보고됩니다.')
    d[i] = ('div', '강조 문단', d[i][1])


def s3_e3(d):
    i = find_block(d, '개인정보처리자는 처리 목적이 달성된 개인정보를 지체 없이 파기하여야 한다.')
    d.insert(i + 1, ('div', '참고 문단', '관련 조항: 개인정보 보호법 제21조'))


UNITS['styleattr', 3] = dict(
    doc=s3, fm=DOC_FM, names=S3_NAMES,
    write=dict(
        instruction=(
            'Write a new document with, in this order: a heading level 1 "사내 동호회 지원 안내"; a paragraph in style '
            '본문 "2027년 동호회 활동비 지원 신청을 받습니다."; a paragraph in style 강조 문단 '
            '"신청 마감: 12월 5일(금) 18:00"; a table in style 격자 표 4 with the header row 구분 / 지원 금액 and '
            'the rows 정기 활동 / 월 10만 원 and 대회 참가 / 회당 30만 원; and a paragraph in style 발신 명의 '
            '"경영지원팀장".'),
        doc=[
            ('h', 1, '사내 동호회 지원 안내'),
            ('div', '본문', '2027년 동호회 활동비 지원 신청을 받습니다.'),
            ('div', '강조 문단', '신청 마감: 12월 5일(금) 18:00'),
            ('table', '격자 표 4', [['구분', '지원 금액'], ['정기 활동', '월 10만 원'], ['대회 참가', '회당 30만 원']]),
            ('div', '발신 명의', '경영지원팀장'),
        ]),
    edits=[
        ('Give the table under "2. 주요 개정 내용" the table style 목록 표 강조 2. Change nothing else.', s3_e1),
        ('Make the sentence "기한 내 미조치 부서는 정보보호위원회에 보고됩니다." red, bold and big. '
         'Change nothing else.', s3_e2),
        ('Right after the quoted paragraph "개인정보처리자는 처리 목적이 달성된 개인정보를 지체 없이 파기하여야 '
         '한다.", add a new paragraph in style 참고 문단 reading "관련 조항: 개인정보 보호법 제21조".', s3_e3),
    ])

# =============================================================== slides
d1 = [
    ('Title Slide', [('title', ['2026 하반기 영업 전략']), ('body', ['영업본부 / 2026년 7월'])]),
    ('Title and Content', [('title', ['상반기 실적 요약']),
                           ('body', ['- 매출 482억 원, 전년 대비 +12%', '- 신규 고객 34곳 확보', '- 재계약률 91%']),
                           ('notes', ['전년 대비 성장률을 먼저 강조'])]),
    ('Section Header', [('title', ['시장 환경']), ('body', ['경쟁 심화와 가격 압박'])]),
    ('Two Content', [('title', ['경쟁사 비교']),
                     ('left', ['- 당사: 전년 대비 +12%', '- 평균 납기 3일', '- 전담 CS 운영']),
                     ('right', ['- A사: 전년 대비 +8%', '- 평균 납기 5일', '- 가격 할인 공세'])]),
    ('Title and Content', [('title', ['하반기 목표']),
                           ('body', ['- 매출 530억 원, 전년 대비 +12%', '- 신규 고객 40곳', '- 재계약률 93%']),
                           ('notes', ['목표치는 경영회의 승인 완료'])]),
    ('Title Only', [('title', ['감사합니다'])]),
]


def slot(slide, name):
    for k, v in slide[1]:
        if k == name:
            return v
    raise KeyError(name)


def d1_e1(d):
    d.insert(5, ('Two Content', [('title', ['실행 과제']),
                                 ('left', ['- 수도권 대리점 3곳 추가', '- 온라인 견적 채널 오픈']),
                                 ('right', ['- 재계약 고객 할인 5%', '- 분기별 고객 세미나'])]))


def d1_e2(d):
    body = slot(d[1], 'body')
    body.remove('- 재계약률 91%')
    slot(d[1], 'notes').append('- 재계약률 91%')


def d1_e3(d):
    body = slot(d[4], 'body')
    body[0] = '- 매출 530억 원, 전년 대비 +10%'


UNITS['slides', 1] = dict(
    deck=d1, fm=DECK_FM, names={'layouts': EN_LAYOUTS},
    write=dict(
        instruction=(
            'Write a new presentation with exactly three slides:\n'
            '1. Layout Title Slide; title "4분기 영업 회의"; body text "영업기획팀".\n'
            '2. Layout Title and Content; title "4분기 중점 과제"; body with the bullets "연말 재고 소진", '
            '"대형 거래처 재계약", "신규 대리점 교육"; speaker notes "재고 소진이 최우선".\n'
            '3. Layout Two Content; title "지역별 담당"; left column bullets "수도권: 김민수", "영남권: 이서연"; '
            'right column bullets "호남권: 박지훈", "제주: 최유진".'),
        deck=[
            ('Title Slide', [('title', ['4분기 영업 회의']), ('body', ['영업기획팀'])]),
            ('Title and Content', [('title', ['4분기 중점 과제']),
                                   ('body', ['- 연말 재고 소진', '- 대형 거래처 재계약', '- 신규 대리점 교육']),
                                   ('notes', ['재고 소진이 최우선'])]),
            ('Two Content', [('title', ['지역별 담당']), ('left', ['- 수도권: 김민수', '- 영남권: 이서연']),
                             ('right', ['- 호남권: 박지훈', '- 제주: 최유진'])]),
        ]),
    edits=[
        ('After slide 5 ("하반기 목표") and before the last slide, add a slide with layout Two Content: title '
         '"실행 과제"; left column bullets "수도권 대리점 3곳 추가", "온라인 견적 채널 오픈"; right column bullets '
         '"재계약 고객 할인 5%", "분기별 고객 세미나".', d1_e1),
        ('On slide 2 ("상반기 실적 요약"), move the bullet "- 재계약률 91%" out of the body and into the speaker '
         'notes, as a new last line of the notes (keep it as the bullet line "- 재계약률 91%").', d1_e2),
        ('On slide 5 ("하반기 목표"), change "전년 대비 +12%" to "전년 대비 +10%". The same phrase on other '
         'slides stays as it is.', d1_e3),
    ])

d2 = [
    ('제목 슬라이드', [('title', ['2026년 3분기 KPI 리뷰']), ('body', ['경영기획실 / 2026-10-06'])]),
    ('제목 및 내용', [('title', ['전사 KPI 달성률']),
                  ('body', ['- 매출: 목표 대비 96%', '- 영업이익: 목표 대비 102%', '- 고객 만족도: 목표 대비 99%']),
                  ('notes', ['영업이익 초과 달성 강조'])]),
    ('콘텐츠 2개', [('title', ['사업부별 달성률']),
                ('left', ['- 가전사업부: 목표 대비 104%', '- 부품사업부: 목표 대비 91%']),
                ('right', ['- 서비스사업부: 목표 대비 96%', '- 해외사업부: 목표 대비 88%', '- 부품사업부 원가 상승 영향'])]),
    ('구역 머리글', [('title', ['4분기 계획']), ('body', ['미달 KPI 집중 관리'])]),
    ('제목 및 내용', [('title', ['4분기 중점 관리 KPI']),
                  ('body', ['- 매출: 목표 대비 96% → 100%', '- 해외사업부: 목표 대비 88% → 95%',
                            '- 고객 만족도: 목표 대비 99% → 100%']),
                  ('notes', ['해외사업부 담당 임원 별도 보고'])]),
    ('제목만', [('title', ['Q&A'])]),
]


def d2_e1(d):
    d.insert(2, ('제목 및 내용', [('title', ['고객 만족도 세부']),
                              ('body', ['- 콜센터 응답률 92%', '- 불만 처리 기간 2.1일']),
                              ('notes', ['콜센터 인력 충원 효과'])]))


def d2_e2(d):
    right = slot(d[2], 'right')
    right.remove('- 부품사업부 원가 상승 영향')
    slot(d[2], 'left').append('- 부품사업부 원가 상승 영향')


def d2_e3(d):
    body = slot(d[1], 'body')
    body[0] = '- 매출: 목표 대비 97%'


UNITS['slides', 2] = dict(
    deck=d2, fm=DECK_FM, names={'layouts': KO_LAYOUTS},
    write=dict(
        instruction=(
            'Write a new presentation with exactly three slides:\n'
            '1. Layout 제목 슬라이드; title "신입사원 온보딩"; body text "인사팀 / 2026년 10월".\n'
            '2. Layout 구역 머리글; title "첫 주 일정"; body text "적응 교육과 부서 배치".\n'
            '3. Layout 콘텐츠 2개; title "담당자 안내"; left column bullets "교육: 인사팀 한지민", '
            '"장비: IT지원팀 오세훈"; right column bullets "급여: 재무팀 윤가은", "복지: 총무팀 서준호"; speaker '
            'notes "연락처는 사내 메신저로 공유".'),
        deck=[
            ('제목 슬라이드', [('title', ['신입사원 온보딩']), ('body', ['인사팀 / 2026년 10월'])]),
            ('구역 머리글', [('title', ['첫 주 일정']), ('body', ['적응 교육과 부서 배치'])]),
            ('콘텐츠 2개', [('title', ['담당자 안내']), ('left', ['- 교육: 인사팀 한지민', '- 장비: IT지원팀 오세훈']),
                        ('right', ['- 급여: 재무팀 윤가은', '- 복지: 총무팀 서준호']),
                        ('notes', ['연락처는 사내 메신저로 공유'])]),
        ]),
    edits=[
        ('After slide 2 ("전사 KPI 달성률"), add a slide with layout 제목 및 내용: title "고객 만족도 세부"; body '
         'bullets "콜센터 응답률 92%", "불만 처리 기간 2.1일"; speaker notes "콜센터 인력 충원 효과".', d2_e1),
        ('On slide 3 ("사업부별 달성률"), the bullet "부품사업부 원가 상승 영향" is in the right column. Move it to '
         'the end of the left column.', d2_e2),
        ('On slide 2 ("전사 KPI 달성률"), change the 매출 bullet from "목표 대비 96%" to "목표 대비 97%". The same '
         'figure on slides 3 and 5 stays as it is.', d2_e3),
    ])

d3 = [
    ('Title Slide', [('title', ['한빛유통 물류 자동화 제안']), ('body', ['스마트로지스 영업1팀 / 2026-10-12'])]),
    ('Title and Content', [('title', ['고객 현황']),
                           ('body', ['- 일 출고량 12,000박스', '- 피크 시즌 출고 지연 7%', '- 수작업 분류 인력 60명'])]),
    ('Title and Content', [('title', ['제안 개요']),
                           ('body', ['- 자동 분류기 2기 도입', '- 출고 지연 2% 이하', '- 분류 인력 60명 → 25명']),
                           ('notes', ['인력 재배치 방안은 질문 시 설명'])]),
    ('Two Content', [('title', ['도입 전후 비교']),
                     ('left', ['- 도입 전', '- 출고 지연 7%', '- 분류 인력 60명']),
                     ('right', ['- 도입 후', '- 출고 지연 2% 이하', '- 분류 인력 25명'])]),
    ('Title and Content', [('title', ['투자 및 회수']),
                           ('body', ['- 총 투자비 18억 원', '- 연간 절감액 5억 원', '- 회수 기간 3.6년']),
                           ('notes', ['리스 조건 별도 안내 가능'])]),
    ('Title Only', [('title', ['감사합니다'])]),
]


def d3_e1(d):
    d.insert(5, ('Section Header', [('title', ['다음 단계']), ('body', ['현장 실사 일정 협의 (10월 넷째 주)'])]))


def d3_e2(d):
    s = d[2]
    note = slot(s, 'notes')[0]
    slot(s, 'body').append('- ' + note)
    d[2] = (s[0], [(k, v) for k, v in s[1] if k != 'notes'])


def d3_e3(d):
    right = slot(d[3], 'right')
    right[1] = '- 출고 지연 1.5% 이하'


UNITS['slides', 3] = dict(
    deck=d3, fm=DECK_FM, names={'layouts': EN_LAYOUTS},
    write=dict(
        instruction=(
            'Write a new presentation with exactly three slides:\n'
            '1. Layout Title Slide; title "고객 세미나 결과 보고"; body text "마케팅팀".\n'
            '2. Layout Title and Content; title "참석 현황"; body bullets "참석 기업 42곳", "참석자 118명", '
            '"만족도 4.6점".\n'
            '3. Layout Title and Content; title "후속 조치"; body bullets "상담 요청 기업 12곳 방문", '
            '"발표 자료 공유"; speaker notes "방문 일정은 영업팀과 조율".'),
        deck=[
            ('Title Slide', [('title', ['고객 세미나 결과 보고']), ('body', ['마케팅팀'])]),
            ('Title and Content', [('title', ['참석 현황']),
                                   ('body', ['- 참석 기업 42곳', '- 참석자 118명', '- 만족도 4.6점'])]),
            ('Title and Content', [('title', ['후속 조치']), ('body', ['- 상담 요청 기업 12곳 방문', '- 발표 자료 공유']),
                                   ('notes', ['방문 일정은 영업팀과 조율'])]),
        ]),
    edits=[
        ('Before the last slide ("감사합니다"), add a slide with layout Section Header: title "다음 단계"; body text '
         '"현장 실사 일정 협의 (10월 넷째 주)".', d3_e1),
        ('On slide 3 ("제안 개요"), move the speaker note "인력 재배치 방안은 질문 시 설명" into the body as a new '
         'last bullet; the slide then has no notes.', d3_e2),
        ('On slide 4 ("도입 전후 비교"), change "출고 지연 2% 이하" in the right column to "출고 지연 1.5% 이하". '
         'Slide 3 keeps its figure.', d3_e3),
    ])
