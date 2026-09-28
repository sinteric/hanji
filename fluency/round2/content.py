# Round 2 seed content, tasks and intended results, in one syntax-neutral form.
#
# Document blocks (lists, so edit functions can change them in place):
#   ['h', level, text]  ['p', text]  ['div', style, text]  ['ul', text]  ['table', style_or_None, rows]
#   rows is a dense grid: rows[0] is the header row, every row has one entry per column;
#   '^^' = merged into the cell above, '<<' = merged into the cell to the left, '' = an empty cell.
# Presentation: [[layout, [[slot, [lines]], ...], [[shape_id, shape_name, text], ...]], ...]
# An edit is (instruction, fn) where fn changes a copy of the seed, or (instruction, None, reason) when the
# intended answer is a refusal.
import copy

DOC_FM = {'type': 'document', 'format': 'docx', 'template': 'org/report', 'schema': '1'}
FORM_FM = {'type': 'document', 'format': 'hwpx', 'template': 'org/gov-form', 'schema': '1'}
DECK_FM = {'type': 'presentation', 'format': 'pptx', 'template': 'org/sales-deck', 'schema': '1'}

EN_LAYOUTS = {'Title Slide': ['title', 'body'], 'Title and Content': ['title', 'body'],
              'Two Content': ['title', 'left', 'right'], 'Section Header': ['title', 'body'],
              'Title Only': ['title']}
KO_LAYOUTS = {'제목 슬라이드': ['title', 'body'], '제목 및 내용': ['title', 'body'],
              '콘텐츠 2개': ['title', 'left', 'right'], '구역 머리글': ['title', 'body'], '제목만': ['title']}


def cp(x):
    return copy.deepcopy(x)


def H(level, text):
    return ['h', level, text]


def P(text):
    return ['p', text]


def D(style, text):
    return ['div', style, text]


def UL(text):
    return ['ul', text]


def TB(style, rows):
    return ['table', style, rows]


def split_rows(spec):
    """'a | b | c' lines -> dense rows."""
    return [[c.strip() for c in line.split('|')] for line in spec.strip().split('\n')]


def block_index(blocks, kind, text):
    for i, b in enumerate(blocks):
        if b[0] == kind and b[-1] == text:
            return i
    raise KeyError((kind, text))


def table_after(blocks, heading, nth=0):
    """The nth table after the heading with this text."""
    i = block_index(blocks, 'h', heading)
    k = 0
    for b in blocks[i + 1:]:
        if b[0] == 'table':
            if k == nth:
                return b
            k += 1
    raise KeyError(heading)


def row_index(rows, col, text):
    for i, r in enumerate(rows):
        if r[col] == text:
            return i
    raise KeyError(text)


def slide_index(deck, title):
    for i, s in enumerate(deck):
        for name, lines in s[1]:
            if name == 'title' and lines == [title]:
                return i
    raise KeyError(title)


def slot(slide, name):
    for n, lines in slide[1]:
        if n == name:
            return lines
    raise KeyError(name)


UNITS = {}

# ======================================================================= merge
# ----------------------------------------------------------------------- merge 1: quarterly sales report
M1 = [
    H(1, '2026년 3분기 지역별 영업 실적 보고'),
    P('작성: 영업기획팀 / 보고일: 2026-10-05 / 배포: 영업본부장, 각 권역장'),
    H(2, '1. 요약'),
    P('3분기 전사 매출은 6,203백만 원으로 전년 동기 대비 4.7% 증가했다. 수도권과 충청권이 성장을 이끌었고, '
      '호남권은 전년 수준에 머물렀다.'),
    UL('수도권: 2,637백만 원 (전년 동기 대비 +6.5%)'),
    UL('영남권: 1,946백만 원 (전년 동기 대비 +3.6%)'),
    UL('호남권: 788백만 원 (전년 동기 대비 +0.6%)'),
    UL('충청권: 832백만 원 (전년 동기 대비 +6.3%)'),
    H(2, '2. 권역별 월별 매출 (단위: 백만 원)'),
    TB(None, split_rows('''
권역 | 지점 | 7월 | 8월 | 9월 | 합계
수도권 | 서울 본점 | 412 | 398 | 455 | 1,265
^^ | 경기 지점 | 287 | 301 | 296 | 884
^^ | 인천 지점 | 156 | 162 | 170 | 488
영남권 | 부산 지점 | 301 | 287 | 312 | 900
^^ | 대구 지점 | 198 | 205 | 211 | 614
^^ | 울산 지점 | 143 | 150 | 139 | 432
호남권 | 광주 지점 | 162 | 156 | 170 | 488
^^ | 전주 지점 | 97 | 104 | 99 | 300
충청권 | 대전 지점 | 176 | 181 | 190 | 547
^^ | 청주 지점 | 88 | 95 | 102 | 285
전사 합계 | << | 2,020 | 2,039 | 2,144 | 6,203''')),
    P('8월 수치는 반품 정산 후 확정치이다.'),
    P('9월에는 추석 선물 세트 판매로 수도권과 영남권 매출이 크게 늘었다.'),
    P('권역별 수치는 지점 확정 매출 기준이며, 본사 직판 매출은 서울 본점에 포함했다.'),
    H(2, '3. 권역별 전년 대비 실적 (단위: 백만 원)'),
    TB(None, split_rows('''
권역 | 지점 | 3분기 매출 | << | 증감률
^^ | ^^ | 전년 | 금년 | ^^
수도권 | 서울 본점 | 1,180 | 1,265 | +7.2%
^^ | 경기 지점 | 842 | 884 | +5.0%
^^ | 인천 지점 | 455 | 488 | +7.3%
영남권 | 부산 지점 | 861 | 900 | +4.5%
^^ | 대구 지점 | 590 | 614 | +4.1%
^^ | 울산 지점 | 428 | 432 | +0.9%
호남권 | 광주 지점 | 482 | 488 | +1.2%
^^ | 전주 지점 | 301 | 300 | -0.3%
충청권 | 대전 지점 | 512 | 547 | +6.8%
^^ | 청주 지점 | 271 | 285 | +5.2%
전사 합계 | << | 5,922 | 6,203 | +4.7%''')),
    P('증감률은 전년 동기 대비이며 소수점 둘째 자리에서 반올림했다.'),
    P('울산 지점은 부산 지점과의 통합을 앞두고 영업 인력을 재배치하고 있다.'),
    H(2, '4. 제품군별 매출 (단위: 백만 원)'),
    TB(None, split_rows('''
제품군 | 품목 | 수도권 | 영남권 | 호남권 | 충청권
생활용품 | 세제 | 820 | 610 | 240 | 250
^^ | 주방용품 | 612 | 488 | 170 | 180
식품 | 음료 | 540 | 402 | 150 | 160
^^ | 간편식 | 455 | 301 | 120 | 130
신제품 | 건강기능식품 | 128 | 88 | 64 | 72
^^ | 반려동물용품 | 82 | 57 | 44 | 40''')),
    P('신제품은 7월 출시 품목으로, 권역별 수치는 잠정치이다.'),
    P('생활용품과 식품이 전체 매출의 75%를 차지했으며, 신제품 비중은 7% 수준이다.'),
    H(2, '5. 주요 거래처 현황'),
    TB(None, split_rows('''
거래처 | 담당 | 계약 상태 | 비고
한빛유통 | 김민수 | 갱신 완료 | 3년 계약
대성상사 | ^^ | 갱신 협의 중 | 단가 조정 요청
동해물산 | 이서연 | 신규 | 9월 첫 발주
서해식품 | ^^ | 갱신 협의 중 | 단가 조정 요청
남부상회 | 박준호 | 갱신 완료 | 2년 계약''')),
    P('갱신 협의 중인 2곳은 모두 단가 조정을 요청했으며, 10월 말까지 대응 방안을 마련한다.'),
    H(2, '6. 권역별 영업 인력 (9월 말 기준)'),
    TB(None, split_rows('''
권역 | 지점 | 정원 | 현원 | 비고
수도권 | 서울 본점 | 18 | 17 | 충원 예정
^^ | 경기 지점 | 12 | 12 | -
^^ | 인천 지점 | 8 | 7 | 충원 예정
영남권 | 부산 지점 | 11 | 11 | -
^^ | 대구 지점 | 8 | 8 | -
^^ | 울산 지점 | 6 | 5 | 충원 예정
호남권 | 광주 지점 | 7 | 7 | -
^^ | 전주 지점 | 5 | 5 | -
충청권 | 대전 지점 | 7 | 6 | 충원 예정
^^ | 청주 지점 | 5 | 5 | -
합계 | << | 87 | 83 | 4명 충원 예정''')),
    P('10월 중 수도권 2명, 영남권 1명, 충청권 1명을 충원한다.'),
    H(2, '7. 권역장 의견'),
    UL('수도권: 대형마트 입점 확대로 4분기에도 성장세 유지 전망'),
    UL('영남권: 부산 지점 물류 거점 이전 후 배송 지연 감소'),
    UL('호남권: 판촉 행사 효과는 10월 이후 반영될 전망'),
    UL('충청권: 청주 지점 신규 거래처 2곳 계약 진행 중'),
    UL('공통: 4분기 물류비 인상분의 단가 반영 여부 결정 필요'),
    H(2, '8. 향후 계획'),
    UL('4분기 수도권 신규 거래처 5곳 확보'),
    UL('호남권 판촉 행사 2회 진행 (광주, 전주)'),
    UL('단가 조정 요청 거래처 대응 방안 수립 (10월 말)'),
    UL('청주 지점 신규 거래처 계약 마무리 (11월)'),
    UL('신제품 권역별 판매 목표 수립 (11월 초)'),
    P('첨부: 지점별 월간 실적표 1부.'),
    P('문의: 영업기획팀 김지현 (내선 2314)'),
]


def m1_e1(d):
    t = table_after(d, '2. 권역별 월별 매출 (단위: 백만 원)')[2]
    i = row_index(t, 1, '인천 지점')
    t.insert(i + 1, ['^^', '수원 지점', '64', '71', '83', '218'])


def m1_e2(d):
    t = table_after(d, '3. 권역별 전년 대비 실적 (단위: 백만 원)')[2]
    i = row_index(t, 1, '부산 지점')
    assert t[i][0] == '영남권' and t[i + 1][0] == '^^'
    del t[i]
    t[i][0] = '영남권'


def m1_e3(d):
    t = table_after(d, '3. 권역별 전년 대비 실적 (단위: 백만 원)')[2]
    vals = {'서울 본점': '1,250', '경기 지점': '900', '인천 지점': '480', '부산 지점': '920', '대구 지점': '600',
            '울산 지점': '450', '광주 지점': '500', '전주 지점': '310', '대전 지점': '530', '청주 지점': '290',
            '<<': '6,230'}
    t[0].insert(4, '<<')
    t[1].insert(4, '목표')
    for r in t[2:]:
        r.insert(4, vals[r[1]])


def m1_e4(d):
    t = table_after(d, '4. 제품군별 매출 (단위: 백만 원)')[2]
    i = row_index(t, 1, '건강기능식품')
    t[i][4:6] = ['합산 집계 중', '<<']
    t[i + 1][4:6] = ['^^', '<<']


UNITS['merge', 1] = dict(
    doc=M1, fm=DOC_FM, stem='report',
    write=dict(
        instruction=(
            'Write a new document: a heading level 1 "2026년 하반기 신입사원 교육 일정"; a paragraph '
            '"대상: 2026년 하반기 입사자 24명 / 주관: 인사팀"; then one table with the columns 구분, 과정, 일시, '
            '장소 for these sessions, in this order (구분 / 과정 / 일시 / 장소):\n'
            '- 공통 교육 / 회사 소개 / 10월 6일 오전 / 본사 대강당\n'
            '- 공통 교육 / 정보보안 / 10월 6일 오후 / 본사 대강당\n'
            '- 공통 교육 / 윤리 경영 / 10월 7일 오전 / 본사 3층 교육장\n'
            '- 직무 교육 / 영업 실무 / 10월 8일 / 영업본부 회의실\n'
            '- 직무 교육 / 품질 관리 / 10월 9일 / 생산본부 회의실\n'
            'In the table, each 구분 should appear once for all of its sessions, and where consecutive sessions '
            'are in the same place, that 장소 should appear once for them; no other cells are combined. The table '
            'ends with one more row whose only content, "교육 불참 시 팀장 승인 필요", runs across the whole width '
            'of the table. After the table, a paragraph "문의: 인사팀 교육담당 (내선 1102)".'),
        doc=[
            H(1, '2026년 하반기 신입사원 교육 일정'),
            P('대상: 2026년 하반기 입사자 24명 / 주관: 인사팀'),
            TB(None, split_rows('''
구분 | 과정 | 일시 | 장소
공통 교육 | 회사 소개 | 10월 6일 오전 | 본사 대강당
^^ | 정보보안 | 10월 6일 오후 | ^^
^^ | 윤리 경영 | 10월 7일 오전 | 본사 3층 교육장
직무 교육 | 영업 실무 | 10월 8일 | 영업본부 회의실
^^ | 품질 관리 | 10월 9일 | 생산본부 회의실
교육 불참 시 팀장 승인 필요 | << | << | <<''')),
            P('문의: 인사팀 교육담당 (내선 1102)'),
        ]),
    edits=[
        ('수도권 opened a new branch. In the table under "2. 권역별 월별 매출 (단위: 백만 원)", add 수원 지점 as the '
         'last 수도권 branch, right after 인천 지점, with 7월 64, 8월 71, 9월 83 and 합계 218. It belongs to '
         '수도권 like the other 수도권 branches. Leave the 전사 합계 row as it is.', m1_e1),
        ('In the table under "3. 권역별 전년 대비 실적 (단위: 백만 원)", remove the row of 부산 지점 (the branch was '
         'merged into 울산 지점). 영남권 must still label its remaining branches. Change nothing else.', m1_e2),
        ('In the table under "3. 권역별 전년 대비 실적 (단위: 백만 원)", add a 목표 column to the 3분기 매출 group, '
         'as its last column right after 금년, so that 3분기 매출 stands over 전년, 금년 and 목표. The 목표 values: '
         '서울 본점 1,250; 경기 지점 900; 인천 지점 480; 부산 지점 920; 대구 지점 600; 울산 지점 450; 광주 지점 500; '
         '전주 지점 310; 대전 지점 530; 청주 지점 290; 전사 합계 6,230.', m1_e3),
        ('In the table under "4. 제품군별 매출 (단위: 백만 원)", the 호남권 and 충청권 figures of the two 신제품 items '
         'are not reported separately yet. Replace those four figures with a single cell reading "합산 집계 중". '
         'Change nothing else.', m1_e4),
    ])

# ----------------------------------------------------------------------- merge 2: weekly update
M2 = [
    H(1, '개발본부 주간 업무 보고 (2026년 9월 4주차)'),
    P('작성: 개발기획파트 / 기간: 9월 21일 ~ 9월 25일 / 보고 대상: 개발본부장'),
    H(2, '1. 금주 요약'),
    UL('모바일 앱 4.2 버전 출시 완료 (9월 23일)'),
    UL('결제 모듈 장애 1건 발생, 2시간 내 복구'),
    UL('추석 연휴 대비 비상 연락망 점검 완료'),
    UL('데이터팀 결원 2명 채용 공고 게시'),
    UL('관리자 웹 개편 화면 개발 60% 진행'),
    UL('검색 속도 개선을 위한 인덱스 재설계 착수'),
    H(2, '2. 팀별 실적 및 계획'),
    TB(None, split_rows('''
팀 | 업무 | 금주 실적 | 차주 계획 | 진척률
개발1팀 | 모바일 앱 4.2 | 출시 완료 | 모니터링 | 100%
^^ | 푸시 알림 개선 | 설계 검토 | 개발 착수 | 20%
^^ | 결제 모듈 안정화 | 장애 원인 분석 | 재발 방지 패치 | 60%
개발2팀 | 관리자 웹 개편 | 화면 개발 | 화면 개발 | 60%
^^ | API 문서화 | 휴가로 일정 순연 (9/29~10/2) | << | 40%
^^ | 검색 속도 개선 | 인덱스 재설계 | 성능 테스트 | 50%
QA팀 | 4.2 회귀 테스트 | 완료 | 없음 | 100%
^^ | 결제 모듈 검증 | 테스트 케이스 작성 | 검증 수행 | 20%''')),
    P('진척률은 팀장 확인 기준이다.'),
    P('관리자 웹 개편은 화면 개발이 2주 연속 이어지고 있어 오픈 일정 재검토가 필요하다.'),
    H(2, '3. 팀별 인력 현황 (9월 25일 기준)'),
    TB(None, split_rows('''
본부 | 팀 | 정원 | 현원 | 결원
개발본부 | 개발1팀 | 12 | 11 | 1
^^ | 개발2팀 | 10 | 10 | 0
^^ | QA팀 | 6 | 5 | 1
기술지원본부 | 인프라팀 | 8 | 7 | 1
^^ | 보안팀 | 5 | 5 | 0
^^ | 데이터팀 | 6 | 4 | 2
합계 | << | 47 | 42 | 5''')),
    P('정원은 2026년 조직 개편 기준이며, 파견 인력은 현원에 포함하지 않았다.'),
    P('데이터팀 결원 2명은 10월 채용으로 충원할 예정이며, 그동안 개발1팀이 데이터 추출 업무를 지원한다.'),
    H(2, '4. 이슈 및 요청 사항'),
    TB(None, split_rows('''
구분 | 내용 | 요청 부서 | 기한
장애 | 결제 모듈 타임아웃 재발 방지 | 개발1팀 | 10월 2일
인력 | QA 인력 1명 추가 지원 | QA팀 | 10월 10일
^^ | 데이터팀 결원 2명 채용 | 데이터팀 | ^^
일정 | 관리자 웹 오픈 일정 확정 | 개발2팀 | 10월 2일''')),
    P('인력 요청 2건은 10월 10일 인사위원회에서 함께 검토한다.'),
    H(2, '5. 차주 일정'),
    TB(None, split_rows('''
일자 | 시간 | 내용 | 참석
9월 29일(월) | 10:00 | 결제 모듈 패치 배포 | 개발1팀, QA팀
^^ | 15:00 | 주간 회의 | 전 팀장
10월 1일(수) | 14:00 | 관리자 웹 중간 리뷰 | 개발2팀, 기획파트
10월 2일(목) | 종일 | 연휴 전 시스템 점검 | 전 팀
^^ | 16:00 | 연휴 비상 연락망 최종 확인 | 전 팀장''')),
    P('10월 2일 점검은 전 팀이 참여하며, 점검 결과는 10월 5일까지 공유한다.'),
    H(2, '6. 9월 장애 및 배포 현황'),
    TB(None, split_rows('''
주차 | 구분 | 서비스 | 건수 | 비고
1주차 | 장애 | 결제 모듈 | 0 | -
^^ | 배포 | 모바일 앱 | 2 | 4.1.3, 4.1.4
2주차 | 장애 | 관리자 웹 | 1 | 30분 내 복구
^^ | 배포 | 관리자 웹 | 1 | 긴급 패치
3주차 | 장애 | 검색 | 1 | 인덱스 재구축
^^ | 배포 | 모바일 앱 | 1 | 4.2 사전 배포
4주차 | 장애 | 결제 모듈 | 1 | 2시간 내 복구
^^ | 배포 | 모바일 앱 | 1 | 4.2 정식 출시
합계 | << | << | 8 | ''')),
    P('장애 건수는 서비스 영향 30분 이상 기준이다.'),
    P('4주차 결제 모듈 장애는 외부 결제 대행사의 응답 지연이 원인으로 확인되었다.'),
    H(2, '7. 10월 인력 운영 계획'),
    TB(None, split_rows('''
팀 | 구분 | 인원 | 기간 | 비고
개발1팀 | 휴가 | 2 | 10월 5일 ~ 9일 | 추석 연휴 연계
^^ | 교육 | 1 | 10월 14일 ~ 16일 | 보안 과정
개발2팀 | 휴가 | 1 | 9월 29일 ~ 10월 2일 | API 문서화 담당
QA팀 | 지원 | 1 | 10월 한 달 | 외부 인력 (파견)
데이터팀 | 채용 | 2 | 10월 중 | 면접 진행 중''')),
    P('휴가 및 교육 일정은 팀장이 조정하며, 변경 시 개발기획파트에 알린다.'),
    H(2, '8. 코드 리뷰 현황'),
    TB(None, split_rows('''
팀 | 요청 | 완료 | 대기
개발1팀 | 42 | 39 | 3
개발2팀 | 35 | 30 | 5
QA팀 | 8 | 8 | 0
합계 | 85 | 77 | 8''')),
    P('대기 중인 리뷰 8건은 10월 1일까지 처리한다.'),
    H(2, '9. 건의 사항'),
    UL('결제 모듈 장애 대응 매뉴얼 개정 (개발1팀, 10월 중)'),
    UL('QA 자동화 도구 라이선스 추가 구매 검토'),
    UL('관리자 웹 오픈 전 사용자 교육 일정 확정 필요'),
    UL('야간 배포 시 QA팀 대기 인력 수당 지급 기준 마련'),
    H(2, '10. 참고 사항'),
    UL('다음 주간 보고는 추석 연휴로 10월 2일(목)에 제출한다.'),
    UL('팀별 실적은 9월 25일 18시 기준이다.'),
    UL('장애 보고서는 사내 위키 장애 게시판에 게시했다.'),
    UL('코드 리뷰 대기 건수는 매주 금요일 17시 기준으로 집계한다.'),
    UL('인력 운영 계획은 10월 1일 팀장 회의에서 확정한다.'),
    P('비상 연락: 개발기획파트 내선 3030 (연휴 기간 당직자 휴대전화로 전환)'),
]


def m2_e1(d):
    t = table_after(d, '2. 팀별 실적 및 계획')[2]
    i = row_index(t, 1, 'API 문서화')
    t[i][2:4] = ['초안 50% 작성', '휴가로 일정 순연 (9/29~10/2)']


def m2_e2(d):
    t = table_after(d, '3. 팀별 인력 현황 (9월 25일 기준)')[2]
    i = row_index(t, 1, '인프라팀')
    assert t[i + 1][1] == '보안팀'
    t[i][3:5] = ['조직 개편 중', '<<']
    t[i + 1][3:5] = ['^^', '<<']


def m2_e3(d):
    t = table_after(d, '2. 팀별 실적 및 계획')[2]
    i = row_index(t, 1, '관리자 웹 개편')
    t.insert(i + 1, ['^^', 'SSO 연동', '요건 정의', '설계 착수', '10%'])


def m2_e4(d):
    t = table_after(d, '2. 팀별 실적 및 계획')[2]
    i = row_index(t, 1, '모바일 앱 4.2')
    del t[i]
    t[i][0] = '개발1팀'


UNITS['merge', 2] = dict(
    doc=M2, fm=DOC_FM, stem='report',
    write=dict(
        instruction=(
            'Write a new document: a heading level 1 "2026년 10월 당직 근무표"; then one table with the columns '
            '주차, 요일, 주간 담당, 야간 담당 for these rows, in this order (주차 / 요일 / 주간 담당 / 야간 담당):\n'
            '- 1주차 / 월~수 / 박지훈 / 최유진\n'
            '- 1주차 / 목~금 / 박지훈 / 정하늘\n'
            '- 2주차 / 월~수 / 이도윤 / 최유진\n'
            '- 2주차 / 목~금 / 이도윤 / 최유진\n'
            '- 3주차 / 월~금 / 외부 위탁 (한결시큐리티), for both the day and the night duty\n'
            'In the table, each 주차 should appear once for its rows, and within a 주차 a name that has the same '
            'duty on consecutive rows should appear once for those rows. In the 3주차 row, "외부 위탁 (한결시큐리티)" '
            'is written once and takes up both the 주간 담당 and 야간 담당 columns. After the table, a paragraph '
            '"비상 연락: 경영지원팀 내선 2020".'),
        doc=[
            H(1, '2026년 10월 당직 근무표'),
            TB(None, split_rows('''
주차 | 요일 | 주간 담당 | 야간 담당
1주차 | 월~수 | 박지훈 | 최유진
^^ | 목~금 | ^^ | 정하늘
2주차 | 월~수 | 이도윤 | 최유진
^^ | 목~금 | ^^ | ^^
3주차 | 월~금 | 외부 위탁 (한결시큐리티) | <<''')),
            P('비상 연락: 경영지원팀 내선 2020'),
        ]),
    edits=[
        ('In the table under "2. 팀별 실적 및 계획", the API 문서화 task now has separate entries for this week and '
         'next week: its 금주 실적 is "초안 50% 작성" and its 차주 계획 is "휴가로 일정 순연 (9/29~10/2)". Change '
         'nothing else.', m2_e1),
        ('In the table under "3. 팀별 인력 현황 (9월 25일 기준)", the 현원 and 결원 figures of 인프라팀 and 보안팀 '
         'are not final because of a reorganization. Show a single cell reading "조직 개편 중" in place of those '
         'four figures. Leave the 합계 row as it is.', m2_e2),
        ('In the table under "2. 팀별 실적 및 계획", 개발2팀 has a new task "SSO 연동" (금주 실적: 요건 정의, '
         '차주 계획: 설계 착수, 진척률: 10%). Add it right after 관리자 웹 개편, as a task of 개발2팀.', m2_e3),
        ('The 모바일 앱 4.2 task is closed. Remove its row from the table under "2. 팀별 실적 및 계획"; 개발1팀 must '
         'still label its remaining tasks. Change nothing else.', m2_e4),
    ])

# ----------------------------------------------------------------------- merge 3: budget request (public agency)
M3 = [
    H(1, '2027년도 정보화사업 예산 요구서'),
    P('제출 부서: ○○시 스마트도시국 / 제출일: 2026. 9. 30. / 담당: 정보화기획팀'),
    H(2, '1. 요구 개요'),
    P('2027년도 정보화사업 예산은 총 4,860백만 원으로, 2026년 대비 370백만 원(8.2%) 증액을 요구한다.'),
    UL('증액 사유: 노후 장비 교체, 스마트 교통 확대, 행정정보 DB 이중화'),
    UL('감액 사업: 민원 키오스크 유지보수 (계약 만료), 돌봄 서비스 플랫폼 (구축 완료)'),
    P('요구액 중 국비 보조는 1,210백만 원, 시비는 3,650백만 원이다.'),
    H(2, '2. 분야별 사업 예산 (단위: 백만 원)'),
    TB(None, split_rows('''
분야 | 사업명 | 담당 부서 | 2026 예산 | 2027 요구 | 증감
행정 | 전자결재 고도화 | 기획예산과 | 320 | 410 | +90
^^ | 민원 키오스크 유지보수 | ^^ | 180 | 120 | -60
^^ | 행정정보 DB 이중화 | ^^ | 250 | 300 | +50
교통 | 스마트 교차로 확대 | 교통정책과 | 900 | 1,150 | +250
^^ | 버스정보시스템 교체 | ^^ | 620 | 700 | +80
안전 | CCTV 통합관제 증설 | 안전총괄과 | 1,050 | 1,120 | +70
^^ | 재난문자 연계 개선 | 재난대응과 | 170 | 160 | -10
복지 | 돌봄 서비스 플랫폼 | 복지정책과 | 1,000 | 900 | -100
합계 | << | << | 4,490 | 4,860 | +370''')),
    P('증감은 2027 요구액에서 2026 예산을 뺀 금액이다.'),
    H(2, '3. 분기별 집행 계획 (단위: 백만 원)'),
    TB(None, split_rows('''
분야 | 상반기 | << | 하반기 | << | 연간
^^ | 1분기 | 2분기 | 3분기 | 4분기 | ^^
행정 | 210 | 250 | 230 | 140 | 830
교통 | 450 | 520 | 480 | 400 | 1,850
안전 | 300 | 340 | 360 | 280 | 1,280
복지 | 200 | 250 | 250 | 200 | 900
합계 | 1,160 | 1,360 | 1,320 | 1,020 | 4,860''')),
    P('집행 계획은 조달 일정과 국비 교부 시기를 반영했다.'),
    P('교통 분야는 2분기에 스마트 교차로 공사가 몰려 집행액이 가장 크다.'),
    P('복지 분야는 운영비만 남아 분기별 집행액이 고르게 나뉜다.'),
    H(2, '4. 인건비 및 운영비 (단위: 백만 원)'),
    TB(None, split_rows('''
직급 | 구분 | 인원 | 금액
5급 | 정규직 | 2 | 180
6급 | 정규직 | 4 | 280
6급 | 기간제 | 1 | 55
7급 | 정규직 | 3 | 180
7급 | 기간제 | 2 | 96
운영비 | << | << | 145''')),
    P('기간제 인원은 2027년 1월 채용 예정이며, 인건비는 12개월분으로 산정했다.'),
    H(2, '5. 추진 일정'),
    TB(None, split_rows('''
단계 | 기간 | 내용
준비 | 2027. 1. ~ 2. | 사업계획 수립, 발주 준비
^^ | 2027. 3. | 조달 발주
구축 | 2027. 4. ~ 9. | 시스템 구축 및 시험
운영 | 2027. 10. ~ 12. | 안정화 및 성과 점검''')),
    P('단계별 일정은 조달 발주 결과에 따라 조정될 수 있다.'),
    H(2, '6. 성과 지표'),
    TB(None, split_rows('''
분야 | 지표 | 2026 목표 | 2027 목표 | 측정 방법
행정 | 전자결재 처리 시간 | 2.0일 | 1.5일 | 시스템 로그
^^ | 민원 온라인 처리율 | 75% | 80% | 민원 통계
교통 | 교차로 평균 대기 시간 | 90초 | 80초 | 신호 제어 데이터
^^ | 버스 도착 정보 정확도 | 92% | 95% | 표본 조사
안전 | 관제 CCTV 대수 | 1,800대 | 2,100대 | 관제센터 집계
복지 | 돌봄 플랫폼 이용자 | 3,000명 | 4,000명 | 가입자 통계''')),
    P('성과 지표는 2027년 12월 말 기준으로 측정해 2028년 2월에 공개한다.'),
    H(2, '7. 사업별 요구 사유'),
    P('전자결재 고도화: 2016년 도입한 전자결재 시스템을 클라우드로 전환하고 모바일 결재 기능을 추가한다.'),
    P('스마트 교차로 확대: 2026년 시범 운영한 12개 교차로에서 대기 시간이 평균 14% 줄어, 2027년 30개소로 확대한다.'),
    P('CCTV 통합관제 증설: 어린이 보호구역과 하천변 사각지대 해소를 위해 300대를 추가한다.'),
    P('민원 키오스크 유지보수: 2026년 12월에 계약이 끝나고, 2027년에는 무상 유지보수 기간이 적용된다.'),
    P('행정정보 DB 이중화: 재해 복구 센터에 실시간 복제 DB를 구축한다.'),
    P('버스정보시스템 교체: 2015년 설치한 정류장 안내기 180대 중 120대를 교체한다.'),
    P('재난문자 연계 개선: 기존 연계 모듈의 유지보수 계약을 단가 인하 조건으로 갱신한다.'),
    P('돌봄 서비스 플랫폼: 구축이 완료되어 2027년에는 운영비만 요구한다.'),
    H(2, '8. 연차별 투자 계획 (단위: 백만 원)'),
    TB(None, split_rows('''
분야 | 사업명 | 2027 | 2028 | 2029
행정 | 전자결재 고도화 | 410 | 150 | 150
^^ | 행정정보 DB 이중화 | 300 | 80 | 80
교통 | 스마트 교차로 확대 | 1,150 | 1,200 | 600
^^ | 버스정보시스템 교체 | 700 | 100 | 100
안전 | CCTV 통합관제 증설 | 1,120 | 900 | 500
합계 | << | 3,680 | 2,430 | 1,430''')),
    P('2028년 이후 금액은 중기지방재정계획 반영 전 추정치이다.'),
    P('연차별 투자 계획은 2027년 3월 정보화위원회 심의 후 확정한다.'),
    P('붙임: 사업별 산출 내역서 1부. 끝.'),
]


def m3_e1(d):
    t = table_after(d, '2. 분야별 사업 예산 (단위: 백만 원)')[2]
    notes = {'전자결재 고도화': '클라우드 전환', '민원 키오스크 유지보수': '계약 만료', '스마트 교차로 확대': '국비 50%',
             'CCTV 통합관제 증설': '국비 30%'}
    t[0].insert(2, '비고')
    for r in t[1:]:
        r.insert(2, '<<' if r[0] == '합계' else notes.get(r[1], ''))


def m3_e2(d):
    t = table_after(d, '2. 분야별 사업 예산 (단위: 백만 원)')[2]
    i = row_index(t, 1, '행정정보 DB 이중화')
    assert t[i][2] == '^^'
    t[i][2] = '정보통신과'


def m3_e3(d):
    t = table_after(d, '4. 인건비 및 운영비 (단위: 백만 원)')[2]
    for i in range(len(t) - 1, 1, -1):
        if t[i][0] == t[i - 1][0] and t[i][0].endswith('급'):
            t[i][0] = '^^'


def m3_e4(d):
    t = table_after(d, '2. 분야별 사업 예산 (단위: 백만 원)')[2]
    i = row_index(t, 1, '스마트 교차로 확대')
    t[i][0] = '^^'
    t[i][2] = '^^'
    t.insert(i, ['교통', '교통 빅데이터 분석', '교통정책과', '0', '230', '+230'])


UNITS['merge', 3] = dict(
    doc=M3, fm=FORM_FM, stem='report',
    write=dict(
        instruction=(
            'Write a new document: a heading level 1 "2026년 하반기 청사 시설 점검 계획"; then one table with the '
            'columns 구역, 층, 점검 항목, 점검일, 담당 for these rows, in this order '
            '(구역 / 층 / 점검 항목 / 점검일 / 담당):\n'
            '- 본관 / 1~3층 / 소방 설비 / 10월 12일 / 시설관리팀\n'
            '- 본관 / 1~3층 / 전기 설비 / 10월 13일 / 시설관리팀\n'
            '- 본관 / 4~6층 / 소방 설비 / 10월 14일 / 시설관리팀\n'
            '- 별관 / 전층 / 소방 설비 / 10월 19일 / 외부 위탁\n'
            '- 별관 / 전층 / 승강기 / 10월 20일 / 외부 위탁\n'
            'In the table, each 구역 should appear once for its rows; within a 구역, a 층 should appear once for '
            'consecutive rows with the same 층, and a 담당 once for the rows of that 구역 with the same 담당. The '
            '점검 항목 and 점검일 cells are never combined. After the table, a paragraph '
            '"점검 결과는 10월 30일까지 시설관리팀에 제출한다."'),
        doc=[
            H(1, '2026년 하반기 청사 시설 점검 계획'),
            TB(None, split_rows('''
구역 | 층 | 점검 항목 | 점검일 | 담당
본관 | 1~3층 | 소방 설비 | 10월 12일 | 시설관리팀
^^ | ^^ | 전기 설비 | 10월 13일 | ^^
^^ | 4~6층 | 소방 설비 | 10월 14일 | ^^
별관 | 전층 | 소방 설비 | 10월 19일 | 외부 위탁
^^ | ^^ | 승강기 | 10월 20일 | ^^''')),
            P('점검 결과는 10월 30일까지 시설관리팀에 제출한다.'),
        ]),
    edits=[
        ('In the table under "2. 분야별 사업 예산 (단위: 백만 원)", add a column 비고 right after 사업명. Its entries: '
         '전자결재 고도화 "클라우드 전환"; 민원 키오스크 유지보수 "계약 만료"; 스마트 교차로 확대 "국비 50%"; '
         'CCTV 통합관제 증설 "국비 30%". The other 사업 have no 비고: leave their cells empty. In the 합계 row, the '
         '합계 label keeps covering every column to the left of 2026 예산, the new 비고 column included.', m3_e1),
        ('In the table under "2. 분야별 사업 예산 (단위: 백만 원)", the 담당 부서 of 행정정보 DB 이중화 is now '
         '정보통신과. 기획예산과 stays the 담당 부서 of the other two 행정 사업. Change nothing else.', m3_e2),
        ('In the table under "4. 인건비 및 운영비 (단위: 백만 원)", the 직급 column should show each 직급 once for '
         'its rows. Change nothing else.', m3_e3),
        ('In the table under "2. 분야별 사업 예산 (단위: 백만 원)", add the 사업 "교통 빅데이터 분석" (2026 예산 0, '
         '2027 요구 230, 증감 +230) as the first 교통 사업, before 스마트 교차로 확대. It is handled by 교통정책과 '
         'like the other 교통 사업: 교통 and 교통정책과 should each still appear once for all 교통 사업. Leave the '
         '합계 row as it is.', m3_e4),
    ])

# ======================================================================= styleattr
# ----------------------------------------------------------------------- styleattr 1: executive meeting minutes
S1_NAMES = {
    'paragraph_styles': [
        ('Body Text', 'ordinary body paragraph'),
        ('Body Text Indent', 'body paragraph indented 10 mm, for sub-points under the paragraph before it'),
        ('Body Text 2', 'body paragraph with double line spacing, for draft wording under review'),
        ('Quote', 'italic quotation, indented on both sides'),
        ('Intense Quote', 'bold italic quotation between two thin rules, for decisions recorded word for word'),
        ('Note', 'small gray text for side remarks'),
        ('Note Heading', 'bold label line that introduces a note'),
        ('Caption', '9 pt label directly above a table'),
        ('Signature', 'right-aligned sign-off line at the end of a document'),
    ],
    'table_styles': [
        ('Table Grid', 'thin black lines around every cell'),
        ('Grid Table 4', 'gray header row, gray banded rows'),
        ('Grid Table 4 Accent 1', 'blue header row, light blue banded rows'),
        ('Plain Table 1', 'light horizontal lines only'),
    ]}

S1 = [
    H(1, '2026년 9월 경영회의 회의록'),
    D('Body Text', '일시: 2026년 9월 24일(목) 10:00 ~ 11:40 / 장소: 본사 12층 대회의실'),
    H(2, '1. 참석자'),
    D('Caption', '표 1. 참석자'),
    TB('Table Grid', split_rows('''
구분 | 성명 | 소속
의장 | 김정호 | 대표이사
참석 | 이수민 | 경영지원본부
참석 | 박태윤 | 영업본부
참석 | 최은영 | 생산본부
배석 | 박소영 | 경영지원팀''')),
    D('Note', '생산본부장은 10시 30분부터 참석했다.'),
    H(2, '2. 안건 1: 3분기 실적 점검'),
    D('Body Text', '3분기 매출은 6,203백만 원으로 목표 대비 97%를 달성했다.'),
    D('Body Text Indent', '수도권 매출은 목표를 넘었으나 호남권은 목표 대비 89%에 그쳤다.'),
    D('Body Text Indent', '서비스본부는 유지보수 계약 해지 2건의 영향으로 목표 대비 94%에 머물렀다.'),
    D('Body Text 2', '(초안) 4분기 목표는 3분기 실적을 반영하여 10월 경영회의에서 재조정한다.'),
    D('Note Heading', '참고'),
    D('Note', '세부 내용은 첨부 자료 참조.'),
    D('Caption', '표 2. 3분기 본부별 실적 (단위: 백만 원)'),
    TB('Grid Table 4', split_rows('''
본부 | 목표 | 실적 | 달성률
영업본부 | 4,200 | 4,080 | 97%
생산본부 | 1,450 | 1,420 | 98%
서비스본부 | 750 | 703 | 94%''')),
    D('Body Text', '4분기에는 호남권 판촉과 서비스본부 계약 갱신에 집중한다.'),
    H(2, '3. 안건 2: 2027년 예산 편성 방향'),
    D('Body Text', '2027년 예산은 2026년 대비 5% 이내 증액을 원칙으로 한다.'),
    P('인건비는 동결하되, 신규 채용 5명분은 별도로 반영한다.'),
    D('Body Text', '마케팅 예산은 온라인 채널 중심으로 재배분한다.'),
    D('Body Text 2', '(초안) 시설 예산은 본사 노후 설비 교체를 2028년으로 미루는 안을 전제로 한다.'),
    D('Note Heading', '참고'),
    D('Note', '세부 내용은 첨부 자료 참조.'),
    D('Caption', '표 3. 2027년 예산 편성 방향 (단위: 백만 원)'),
    TB('Grid Table 4', split_rows('''
항목 | 2026년 | 2027년 안 | 증감
인건비 | 2,100 | 2,180 | +80
마케팅 | 640 | 700 | +60
시설 | 380 | 350 | -30
합계 | 3,120 | 3,230 | +110''')),
    H(2, '4. 결정 사항'),
    D('Intense Quote', '호남권 영업 강화를 위해 10월 중 권역 TF를 구성한다.'),
    D('Intense Quote', '2027년 예산 초안은 10월 20일까지 경영지원본부에 제출한다.'),
    D('Intense Quote', '4분기 목표 재조정안은 10월 경영회의에 상정한다.'),
    P('보안 교육 미이수자는 10월 말까지 이수한다.'),
    H(2, '5. 후속 조치'),
    D('Caption', '표 4. 후속 조치'),
    TB('Grid Table 4', split_rows('''
조치 | 담당 | 기한
권역 TF 구성 | 영업본부 | 10월 15일
예산 초안 제출 | 각 본부 | 10월 20일
보안 교육 이수 점검 | 경영지원팀 | 10월 31일
4분기 목표 재조정안 작성 | 경영지원본부 | 10월 27일''')),
    D('Note Heading', '참고'),
    D('Note', '세부 내용은 첨부 자료 참조.'),
    H(2, '6. 기타 보고'),
    D('Body Text', '정보보안팀은 9월 모의 해킹 점검 결과를 보고했다.'),
    D('Body Text Indent', '외부 공개 서버 12대 중 2대에서 취약점이 발견되어 9월 30일까지 조치한다.'),
    D('Body Text Indent', '보안 교육 이수율은 9월 말 기준 87%이다.'),
    D('Caption', '표 5. 본부별 보안 교육 이수율'),
    TB('Grid Table 4', split_rows('''
본부 | 대상 | 이수 | 이수율
영업본부 | 120 | 98 | 82%
생산본부 | 140 | 126 | 90%
경영지원본부 | 45 | 41 | 91%
합계 | 305 | 265 | 87%''')),
    D('Quote', '보안은 한 사람의 실수로 무너진다. (대표이사 당부 사항)'),
    D('Note Heading', '참고'),
    D('Note', '세부 내용은 첨부 자료 참조.'),
    D('Body Text', '다음 회의: 2026년 10월 29일(목) 10:00, 본사 12층 대회의실'),
    D('Note', '회의 자료는 사내 게시판 경영회의 폴더에 게시했다.'),
    P('작성: 경영지원팀 박소영 / 확인: 경영지원본부장 이수민'),
]


def s1_e1(d):
    table_after(d, '5. 후속 조치')[1] = 'Grid Table 4 Accent 1'


def s1_e3(d):
    for text in ('인건비는 동결하되, 신규 채용 5명분은 별도로 반영한다.', '마케팅 예산은 온라인 채널 중심으로 재배분한다.'):
        for i, b in enumerate(d):
            if b[-1] == text:
                d[i] = D('Body Text Indent', text)


def s1_e4(d):
    i = block_index(d, 'p', '작성: 경영지원팀 박소영 / 확인: 경영지원본부장 이수민')
    d[i] = D('Signature', d[i][1])


UNITS['styleattr', 1] = dict(
    doc=S1, fm=DOC_FM, stem='memo', names=S1_NAMES,
    write=dict(
        instruction=(
            'Write the minutes of a short meeting as a new document, in this order: a heading level 1 '
            '"2026년 10월 영업전략회의 회의록"; the line "일시: 2026년 10월 8일(목) 14:00 ~ 15:00" as an ordinary '
            'body paragraph; the label "표 1. 참석자" in the file\'s style for a label above a table; directly after '
            'it a table with thin black lines around every cell, with the columns 구분, 성명, 소속 and the rows '
            '의장 / 박태윤 / 영업본부, 참석 / 정다은 / 영업기획팀, 참석 / 한도윤 / 온라인영업팀; the decision '
            '"온라인영업팀은 11월 프로모션 계획을 10월 22일까지 보고한다." recorded word for word in the file\'s style '
            'for that; the side remark "세부 일정은 별도 공지." in small gray text; and the sign-off '
            '"작성: 영업기획팀 정다은", right-aligned as the last line.'),
        doc=[
            H(1, '2026년 10월 영업전략회의 회의록'),
            D('Body Text', '일시: 2026년 10월 8일(목) 14:00 ~ 15:00'),
            D('Caption', '표 1. 참석자'),
            TB('Table Grid', split_rows('''
구분 | 성명 | 소속
의장 | 박태윤 | 영업본부
참석 | 정다은 | 영업기획팀
참석 | 한도윤 | 온라인영업팀''')),
            D('Intense Quote', '온라인영업팀은 11월 프로모션 계획을 10월 22일까지 보고한다.'),
            D('Note', '세부 일정은 별도 공지.'),
            D('Signature', '작성: 영업기획팀 정다은'),
        ]),
    edits=[
        ('Give the table under "5. 후속 조치" the version of its current style that has a blue header row. '
         'Change nothing else.', s1_e1),
        ('Make the sentence "보안 교육 미이수자는 10월 말까지 이수한다." red, so that nobody misses it.', None,
         'no style of this file makes text red'),
        ('Under "3. 안건 2: 2027년 예산 편성 방향", the paragraphs "인건비는 동결하되, 신규 채용 5명분은 별도로 '
         '반영한다." and "마케팅 예산은 온라인 채널 중심으로 재배분한다." are sub-points of the paragraph before '
         'them. Give each of them the file\'s indented style for sub-points. Change nothing else.', s1_e3),
        ('Right-align the last line, "작성: 경영지원팀 박소영 / 확인: 경영지원본부장 이수민", as the sign-off of the '
         'minutes.', s1_e4),
    ])

# ----------------------------------------------------------------------- styleattr 2: logistics-center proposal
S2_NAMES = {
    'paragraph_styles': [
        ('Body Text', 'ordinary body paragraph'),
        ('Key Message', 'large bold statement of the main point of a section'),
        ('Key Message Small', 'the same statement at body size, for a secondary main point'),
        ('Callout', 'boxed paragraph on a light gray background'),
        ('Callout Warning', 'boxed paragraph with an orange border, for risks'),
        ('Footnote Text', 'small text for sources and remarks'),
        ('Caption', '9 pt label directly above a table'),
    ],
    'table_styles': [
        ('Light List', 'only a thin line under the header row'),
        ('Light List Accent 2', 'only a thin orange line under the header row'),
        ('Medium Shading 1', 'dark header row, shaded rows'),
        ('Medium Shading 1 Accent 2', 'orange header row, shaded rows'),
    ]}

S2 = [
    H(1, '이천 통합 물류센터 구축 제안서'),
    D('Body Text', '제안 부서: 물류혁신TF / 제출일: 2026년 9월 30일 / 보고 대상: 경영위원회'),
    H(2, '1. 추진 배경'),
    D('Key Message', '분산된 3개 창고를 하나로 통합해 연간 물류비 18%를 절감한다.'),
    D('Body Text', '현재 수도권 물량은 이천, 용인, 광주(경기) 3개 임차 창고에서 나누어 처리하고 있다.'),
    D('Body Text', '창고 간 재고 이동이 월 1,200건에 달하고, 출고 리드타임은 평균 2일이다.'),
    D('Body Text', '온라인 주문 증가로 2025년 이후 일 출고 건수가 해마다 15%씩 늘고 있다.'),
    D('Callout', '임차 계약 3건 중 2건이 2027년 6월에 만료된다.'),
    D('Caption', '표 1. 현행 창고 현황'),
    TB('Light List', split_rows('''
창고 | 면적 (㎡) | 일 출고 (건) | 계약 만료
이천 | 9,900 | 8,000 | 2027년 6월
용인 | 6,600 | 5,500 | 2027년 6월
광주(경기) | 4,300 | 3,200 | 2028년 12월''')),
    H(2, '2. 추진 방안'),
    D('Key Message Small', '이천 부지에 연면적 23,000㎡ 규모의 통합 센터를 신축한다.'),
    D('Body Text', '자동 분류기와 WMS를 도입하고, 용인·광주 창고는 계약 만료 시 반납한다.'),
    D('Body Text', '신축 센터는 지상 4층 규모로, 1~2층은 입출고 구역, 3~4층은 보관 구역으로 쓴다.'),
    D('Footnote Text', '연면적은 기본 설계 전 추정치이다.'),
    P('공사 기간 중 성수기 물량이 몰리면 출고 지연이 생길 수 있다.'),
    D('Caption', '표 2. 투자 계획 (단위: 억 원)'),
    TB('Medium Shading 1', split_rows('''
항목 | 2027년 | 2028년 | 합계
토지·건축 | 180 | 60 | 240
설비 | 40 | 55 | 95
시스템 | 12 | 18 | 30
합계 | 232 | 133 | 365''')),
    D('Footnote Text', '금액은 VAT 별도, 설계 전 추정치이다.'),
    H(2, '3. 기대 효과'),
    D('Body Text', '통합 후 연간 물류비 18% 절감과 출고 리드타임 단축이 예상된다.'),
    D('Caption', '표 3. 기대 효과'),
    TB('Medium Shading 1', split_rows('''
항목 | 현행 | 통합 후
연간 물류비 | 142억 원 | 116억 원
출고 리드타임 | 2일 | 1일
창고 간 재고 이동 | 월 1,200건 | 없음''')),
    D('Callout', '물류비 절감액은 연간 약 26억 원으로 추정된다.'),
    D('Footnote Text', '금액은 VAT 별도, 설계 전 추정치이다.'),
    H(2, '4. 추진 일정'),
    D('Key Message Small', '2028년 6월 통합 센터 가동을 목표로 한다.'),
    D('Caption', '표 4. 추진 일정'),
    TB('Light List', split_rows('''
단계 | 기간 | 내용
설계 | 2027년 1월 ~ 4월 | 기본·실시 설계
시공 | 2027년 5월 ~ 2028년 4월 | 건축 및 설비 공사
이전 | 2028년 5월 ~ 6월 | 재고 이전 및 시범 운영''')),
    D('Body Text', '이전 기간에는 기존 이천 창고 운영을 유지해 출고 공백을 막는다.'),
    D('Callout', '이사회 승인 후 설계 발주까지 약 2개월이 소요된다.'),
    H(2, '5. 위험 요인 및 대응'),
    D('Callout Warning', '인허가가 지연되면 착공이 최대 3개월 늦어질 수 있다.'),
    D('Body Text', '인허가는 설계 단계부터 이천시와 사전 협의를 진행해 지연 가능성을 줄인다.'),
    D('Caption', '표 5. 위험 요인별 대응'),
    TB('Light List', split_rows('''
위험 요인 | 영향 | 대응
인허가 지연 | 착공 지연 | 설계 단계 사전 협의
공사비 상승 | 투자비 증가 | 주요 자재 단가 계약
성수기 출고 지연 | 고객 불만 | 임시 창고 확보''')),
    H(2, '6. 요청 사항'),
    D('Key Message Small', '경영위원회에 투자 계획 승인을 요청한다.'),
    D('Body Text', '승인 시 10월 중 설계 용역을 발주하고, 11월에 이천시와 인허가 사전 협의를 시작한다.'),
    H(2, '7. 검토 의견'),
    D('Body Text', '재무팀: 2027년 차입 한도 안에서 조달할 수 있다.'),
    D('Body Text', '법무팀: 임차 계약에 중도 해지 조항은 없으며, 만료 시 원상 복구 의무가 있다.'),
    D('Body Text', '인사팀: 용인 창고 인력 32명은 이천 센터로 전환 배치한다.'),
    D('Footnote Text', '금액은 VAT 별도, 설계 전 추정치이다.'),
]


def s2_e1(d):
    i = block_index(d, 'p', '공사 기간 중 성수기 물량이 몰리면 출고 지연이 생길 수 있다.')
    d[i] = D('Callout Warning', d[i][1])


def s2_e2(d):
    table_after(d, '2. 추진 방안')[1] = 'Medium Shading 1 Accent 2'


def s2_e4(d):
    i = block_index(d, 'h', '3. 기대 효과')
    d.insert(i + 1, D('Key Message Small', '출고 리드타임을 2일에서 1일로 줄인다.'))


UNITS['styleattr', 2] = dict(
    doc=S2, fm=DOC_FM, stem='memo', names=S2_NAMES,
    write=dict(
        instruction=(
            'Write a new document, in this order: a heading level 1 "용인 창고 반납 계획"; the main point of the '
            'document, "용인 창고는 2027년 6월 계약 만료와 함께 반납한다.", as a large bold statement; the ordinary '
            'body paragraph "재고는 2027년 4월부터 이천 센터로 단계적으로 옮긴다."; the label "표 1. 반납 일정" in '
            'the file\'s style for a label above a table; directly after it a table whose only line is a thin orange '
            'line under the header row, with the columns 단계, 시기 and the rows 재고 이전 / 2027년 4월 ~ 5월 and '
            '원상 복구 / 2027년 6월; the source line "자료: 물류혁신TF" in small text for sources; and the risk '
            '"원상 복구 비용이 보증금을 넘을 수 있다." in a box with an orange border.'),
        doc=[
            H(1, '용인 창고 반납 계획'),
            D('Key Message', '용인 창고는 2027년 6월 계약 만료와 함께 반납한다.'),
            D('Body Text', '재고는 2027년 4월부터 이천 센터로 단계적으로 옮긴다.'),
            D('Caption', '표 1. 반납 일정'),
            TB('Light List Accent 2', split_rows('''
단계 | 시기
재고 이전 | 2027년 4월 ~ 5월
원상 복구 | 2027년 6월''')),
            D('Footnote Text', '자료: 물류혁신TF'),
            D('Callout Warning', '원상 복구 비용이 보증금을 넘을 수 있다.'),
        ]),
    edits=[
        ('The sentence "공사 기간 중 성수기 물량이 몰리면 출고 지연이 생길 수 있다." describes a risk. Give it the '
         'file\'s style for risks. Change nothing else.', s2_e1),
        ('Give the table under "2. 추진 방안" the orange version of its current style. Change nothing else.', s2_e2),
        ('In the paragraph under "3. 기대 효과", highlight the phrase "연간 물류비 18% 절감" in yellow.', None,
         'no style of this file highlights text'),
        ('Right after the heading "3. 기대 효과", add the paragraph "출고 리드타임을 2일에서 1일로 줄인다." as a '
         'secondary main-point statement, at body size.', s2_e4),
    ])

# ----------------------------------------------------------------------- styleattr 3: official notice (Korean names)
S3_NAMES = {
    'paragraph_styles': [
        ('본문', '기본 본문'),
        ('본문 들여쓰기', '10mm 들여 쓴 본문: 번호 항목 아래의 가., 나. 항목'),
        ('본문 들여쓰기 2', '20mm 들여 쓴 본문: 가., 나. 항목 아래의 1), 2) 항목'),
        ('강조 문단', '위아래 가는 선 사이의 굵은 문단: 공고의 핵심 문장'),
        ('참고 문단', '작은 회색 글씨의 참고 사항'),
        ('붙임', '문서 끝의 붙임 목록 줄'),
        ('발신 명의', '가운데 정렬된 큰 글씨: 공고 끝의 발신 기관장 명의'),
    ],
    'table_styles': [
        ('표 눈금', '모든 칸에 가는 검은 실선'),
        ('눈금 표 4', '회색 머리글 행, 회색 줄무늬 행'),
        ('눈금 표 4 - 강조색 1', '파란 머리글 행, 연한 파란 줄무늬 행'),
        ('일반 표 1', '가는 가로선만'),
    ]}

S3 = [
    H(1, '2026년 하반기 소상공인 디지털 전환 지원사업 공고'),
    D('본문', '○○시는 소상공인의 디지털 전환을 돕기 위해 다음과 같이 지원사업 참여자를 모집합니다.'),
    D('강조 문단', '신청 기간: 2026. 10. 5.(월) ~ 10. 23.(금) 18:00'),
    H(2, '1. 지원 대상'),
    D('본문', '공고일 현재 ○○시에 사업자 등록을 하고 영업 중인 소상공인'),
    D('본문 들여쓰기', '가. 상시 근로자 5인 미만 (제조업 등은 10인 미만)'),
    D('본문 들여쓰기', '나. 2025년 매출액 10억 원 이하'),
    D('본문 들여쓰기', '다. 지방세 체납이 없는 사업자'),
    D('참고 문단', '휴업 또는 폐업 중인 사업자는 제외합니다.'),
    D('참고 문단', '공동 대표 사업자는 대표자 1명이 신청합니다.'),
    H(2, '2. 지원 내용'),
    D('본문', '선정된 사업자에게 다음 중 하나를 지원합니다.'),
    D('본문', '가. 키오스크·테이블 주문 기기 도입비 (최대 300만 원)'),
    D('본문', '나. 온라인 쇼핑몰 입점 및 상세페이지 제작 (최대 200만 원)'),
    D('본문 들여쓰기 2', '1) 자부담 20%는 선정 후 납부'),
    D('본문 들여쓰기 2', '2) 동일 품목을 이미 지원받은 사업자는 제외'),
    D('본문 들여쓰기 2', '3) 기기 설치 후 3개월 안에 사용 실적 제출'),
    D('참고 문단', '지원 한도를 넘는 금액은 사업자가 부담합니다.'),
    P('표 1. 지원 유형별 선정 규모'),
    TB('눈금 표 4', split_rows('''
지원 유형 | 선정 규모 | 지원 한도 | 자부담
키오스크·테이블 주문 | 120곳 | 300만 원 | 20%
온라인 쇼핑몰 입점 | 80곳 | 200만 원 | 20%
합계 | 200곳 | - | -''')),
    H(2, '3. 신청 방법'),
    D('본문', '○○시 누리집에서 온라인으로 신청하거나, 시청 민원실을 방문하여 신청합니다.'),
    D('본문', '신청서 서식은 ○○시 누리집 공고 게시물에 첨부되어 있습니다.'),
    D('본문 들여쓰기', '가. 온라인: ○○시 누리집 > 알림마당 > 공고'),
    D('본문 들여쓰기', '나. 방문: 시청 본관 1층 민원실 (평일 09:00 ~ 18:00)'),
    D('참고 문단', '방문 신청 시 대리인은 위임장과 신분증을 함께 제출합니다.'),
    P('신청 기한을 넘긴 신청서는 접수하지 않습니다.'),
    P('표 2. 제출 서류'),
    TB('눈금 표 4', split_rows('''
구분 | 서류 | 비고
필수 | 참여 신청서 | 붙임 서식 1
필수 | 사업자등록증 사본 | -
필수 | 개인정보 수집 동의서 | 붙임 서식 2
해당자 | 부가가치세 과세표준증명 | 2025년 귀속''')),
    H(2, '4. 선정 및 발표'),
    D('본문', '서류 심사 후 2026. 11. 13.(금) ○○시 누리집에 발표합니다.'),
    D('본문 들여쓰기', '가. 평가 항목: 디지털 활용 계획, 매출 규모, 사업 지속성'),
    D('본문 들여쓰기', '나. 동점 시 매출액이 적은 사업자를 우선 선정'),
    P('표 3. 평가 배점'),
    TB('일반 표 1', split_rows('''
평가 항목 | 배점
디지털 활용 계획 | 50
매출 규모 | 30
사업 지속성 | 20
합계 | 100''')),
    D('참고 문단', '선정 결과는 개별 문자로도 안내합니다.'),
    D('참고 문단', '선정 결과에 이의가 있는 경우 발표일로부터 7일 안에 신청할 수 있습니다.'),
    H(2, '5. 유의 사항'),
    D('본문', '다음에 해당하는 경우 선정을 취소하고 지원금을 환수합니다.'),
    D('본문 들여쓰기', '가. 거짓이나 부정한 방법으로 지원을 받은 경우'),
    D('본문 들여쓰기', '나. 지원받은 기기를 1년 안에 처분하거나 폐업한 경우'),
    D('본문 들여쓰기 2', '1) 폐업 시에는 남은 기간에 비례하여 환수'),
    D('본문 들여쓰기 2', '2) 천재지변 등 불가피한 사유는 심의 후 면제'),
    D('참고 문단', '지원 기기는 선정일로부터 1년간 사후 관리 대상입니다.'),
    P('표 4. 추진 일정'),
    TB('눈금 표 4', split_rows('''
단계 | 기간 | 내용
신청 접수 | 10. 5. ~ 10. 23. | 온라인·방문 접수
서류 심사 | 10. 26. ~ 11. 6. | 자격 요건 확인
결과 발표 | 11. 13. | 누리집 게시, 개별 문자 안내
협약 체결 | 11. 16. ~ 11. 27. | 협약서 작성, 자부담금 납부''')),
    H(2, '6. 문의'),
    TB('일반 표 1', split_rows('''
구분 | 담당 부서 | 연락처
사업 내용 | 일자리경제과 | 031-000-1234
온라인 신청 | 정보통신과 | 031-000-5678''')),
    D('참고 문단', '문의 전화는 평일 09:00 ~ 18:00에 받습니다.'),
    D('붙임', '붙임  1. 참여 신청서 1부.'),
    D('붙임', '2. 개인정보 수집 동의서 1부.  끝.'),
    P('○○시장'),
]


def s3_e1(d):
    for text in ('가. 키오스크·테이블 주문 기기 도입비 (최대 300만 원)', '나. 온라인 쇼핑몰 입점 및 상세페이지 제작 (최대 200만 원)'):
        i = block_index(d, 'div', text)
        d[i][1] = '본문 들여쓰기'


def s3_e3(d):
    table_after(d, '3. 신청 방법')[1] = '눈금 표 4 - 강조색 1'


def s3_e4(d):
    i = block_index(d, 'p', '○○시장')
    d[i] = D('발신 명의', '○○시장')


UNITS['styleattr', 3] = dict(
    doc=S3, fm=FORM_FM, stem='memo', names=S3_NAMES,
    write=dict(
        instruction=(
            'Write a short notice as a new document, in this order: a heading level 1 "2026년 하반기 전통시장 '
            '주차장 무료 개방 안내"; the ordinary body paragraph "○○시는 전통시장 이용 편의를 위해 공영주차장을 무료로 '
            '개방합니다."; the key sentence of the notice, "개방 기간: 2026. 11. 1. ~ 12. 31.", in the file\'s style '
            'for it; the ordinary body paragraph "대상 주차장은 다음과 같습니다."; the items "가. 중앙시장 공영주차장" '
            'and "나. 동부시장 공영주차장" in the style for items under a numbered point; the item "1) 1일 최대 3시간" '
            'in the style for items under 가. and 나.; a table with thin black lines around every cell, with the '
            'columns 주차장, 주차면 and the rows 중앙시장 / 120면 and 동부시장 / 85면; the attachment line '
            '"붙임  주차장 위치도 1부.  끝." in the file\'s style for attachment lines; and the sender\'s name '
            '"○○시장" as the last line, in the file\'s style for it.'),
        doc=[
            H(1, '2026년 하반기 전통시장 주차장 무료 개방 안내'),
            D('본문', '○○시는 전통시장 이용 편의를 위해 공영주차장을 무료로 개방합니다.'),
            D('강조 문단', '개방 기간: 2026. 11. 1. ~ 12. 31.'),
            D('본문', '대상 주차장은 다음과 같습니다.'),
            D('본문 들여쓰기', '가. 중앙시장 공영주차장'),
            D('본문 들여쓰기', '나. 동부시장 공영주차장'),
            D('본문 들여쓰기 2', '1) 1일 최대 3시간'),
            TB('표 눈금', split_rows('''
주차장 | 주차면
중앙시장 | 120면
동부시장 | 85면''')),
            D('붙임', '붙임  주차장 위치도 1부.  끝.'),
            D('발신 명의', '○○시장'),
        ]),
    edits=[
        ('Under "2. 지원 내용", the lines "가. 키오스크·테이블 주문 기기 도입비 (최대 300만 원)" and "나. 온라인 쇼핑몰 '
         '입점 및 상세페이지 제작 (최대 200만 원)" are items under the numbered point. Give both the file\'s style for '
         'such items. Change nothing else.', s3_e1),
        ('Put the sentence "신청 기한을 넘긴 신청서는 접수하지 않습니다." in a red box so that applicants notice it.',
         None, 'no style of this file draws a red box'),
        ('Give the table of 제출 서류 (under "3. 신청 방법") the version of its current style with a blue header '
         'row. Change nothing else.', s3_e3),
        ('Center the last line, "○○시장", and make it large: it is the sender\'s name at the end of the notice.',
         s3_e4),
    ])

# ======================================================================= tablestyle
# ----------------------------------------------------------------------- tablestyle 1: supplier evaluation report
T1_NAMES = {
    'paragraph_styles': [
        ('Body Text', 'ordinary body paragraph'),
        ('Note', 'small gray text for side remarks'),
        ('Caption', '9 pt label directly above a table'),
    ],
    'table_styles': [
        ('Table Grid', 'thin black lines around every cell'),
        ('Grid Table 4', 'gray header row, gray banded rows'),
        ('Grid Table 4 Accent 1', 'blue header row, light blue banded rows'),
        ('List Table 3', 'dark header row, no vertical lines'),
    ]}

T1 = [
    H(1, '2026년 하반기 협력업체 평가 결과 보고'),
    D('Body Text', '작성: 구매팀 / 평가 기간: 2026. 7. 1. ~ 9. 30. / 대상: 협력업체 7곳'),
    H(2, '1. 평가 기준'),
    D('Body Text', '평가는 정량 항목 80점과 정성 항목 20점으로 구성하며, 정성 항목은 평가위원 3인의 평균으로 산정한다.'),
    D('Caption', '표 1. 평가 항목과 배점'),
    TB('Table Grid', split_rows('''
구분 | 항목 | 배점 | 비고
품질 | 불량률 | 30 | 월 평균
^^ | 품질 인증 | 10 | ISO 9001 등
납기 | 납기 준수율 | 25 | 월 평균
^^ | 긴급 대응 | 10 | 요청 후 48시간
가격 | 단가 경쟁력 | 15 | 시장가 대비
협력 | 개선 제안 | 10 | 연간 건수
합계 | << | 100 | ''')),
    D('Note', '배점은 2026년 구매위원회 의결 기준이다.'),
    H(2, '2. 업체별 점수'),
    D('Body Text', '업체별 점수는 평가위원회 심의를 거쳐 10월 8일 확정했다.'),
    D('Caption', '표 2. 업체별 평가 점수'),
    TB('Grid Table 4', split_rows('''
분야 | 업체 | 품질 | 납기 | 가격 | 협력 | 총점 | 등급
제조 | 대한정밀 | 36 | 33 | 13 | 8 | 90 | A
^^ | 성진테크 | 34 | 30 | 12 | 7 | 83 | B
^^ | 우림산업 | 30 | 28 | 14 | 6 | 78 | B
^^ | 한결부품 | 25 | 22 | 11 | 5 | 63 | C
물류 | 동방로지스 | 35 | 34 | 12 | 9 | 90 | A
^^ | 서해운송 | 28 | 27 | 13 | 7 | 75 | B
^^ | 누리물류 | 22 | 20 | 10 | 4 | 56 | D''')),
    D('Note', '총점 90점 이상 A, 75점 이상 B, 60점 이상 C, 60점 미만 D.'),
    D('Body Text', '제조 분야 평균은 78.5점, 물류 분야 평균은 73.7점이다.'),
    D('Body Text', '등급별로는 A 등급 2곳, B 등급 3곳, C 등급 1곳, D 등급 1곳이다.'),
    H(2, '3. 등급별 조치'),
    TB(None, split_rows('''
등급 | 조치 | 시기 | 담당
A | 우수 협력업체 지정 | 11월 | 구매팀
^^ | 2027년 물량 우선 배정 | 12월 | ^^
B | 정기 모니터링 | 분기 1회 | ^^
C | 개선 계획서 제출 요구 | 10월 | 품질팀
^^ | 현장 점검 | 11월 | ^^
D | 거래 중단 검토 | 12월 | 구매위원회''')),
    D('Body Text', 'C, D 등급 업체는 개선 계획 이행 여부를 다음 평가에 반영한다.'),
    D('Note', '우수 협력업체 지정 기간은 1년이며, 다음 평가에서 A 등급을 유지하면 연장한다.'),
    H(2, '4. 전년 대비 점수 비교'),
    D('Caption', '표 3. 업체별 전년 대비 총점'),
    TB('Grid Table 4', split_rows('''
분야 | 업체 | 2025년 하반기 | 2026년 하반기 | 증감
제조 | 대한정밀 | 88 | 90 | +2
^^ | 성진테크 | 85 | 83 | -2
^^ | 우림산업 | 74 | 78 | +4
^^ | 한결부품 | 70 | 63 | -7
물류 | 동방로지스 | 87 | 90 | +3
^^ | 서해운송 | 76 | 75 | -1
^^ | 누리물류 | 61 | 56 | -5''')),
    D('Note', '한결부품과 누리물류는 2회 연속 점수가 하락했다.'),
    D('Body Text', '전년 대비 점수가 오른 업체는 3곳, 내린 업체는 4곳이다.'),
    H(2, '5. 향후 일정'),
    TB('Grid Table 4', split_rows('''
일정 | 내용 | 담당
10월 15일 | 결과 통보 | 구매팀
10월 31일 | 개선 계획서 접수 | 품질팀
11월 중 | 현장 점검 | ^^
12월 중 | 구매위원회 상정 | 구매팀
2027년 1월 | 상반기 평가 계획 수립 | ^^''')),
    D('Note', '평가 결과에 이의가 있는 업체는 통보일로부터 7일 안에 재심을 신청할 수 있다.'),
    D('Note', '업체별 평가표는 구매팀 공유 폴더에 있다.'),
    H(2, '6. 평가위원'),
    TB('Table Grid', split_rows('''
구분 | 성명 | 소속
위원장 | 김태호 | 구매본부장
위원 | 이지은 | 품질팀장
^^ | 박성우 | 구매팀장
^^ | 최민정 | 재무팀 과장''')),
    D('Note', '평가위원은 평가 대상 업체와 이해관계가 없음을 서약했다.'),
    H(2, '7. 개선 계획서 제출 현황'),
    TB('List Table 3', split_rows('''
분야 | 업체 | 등급 | 제출 기한 | 상태
제조 | 한결부품 | C | 10월 31일 | 검토 중
물류 | 누리물류 | D | 10월 31일 | 보완 요청''')),
    D('Note', '개선 계획서는 구매팀과 품질팀이 함께 검토한다.'),
    D('Body Text', '보완 요청을 받은 업체는 11월 7일까지 다시 제출한다.'),
    D('Body Text', '첨부: 업체별 평가표 7부.'),
]


def t1_e1(d):
    table_after(d, '3. 등급별 조치')[1] = 'Grid Table 4'


def t1_e3(d):
    i = block_index(d, 'h', '2. 업체별 점수')
    tb = table_after(d, '2. 업체별 점수')
    j = d.index(tb)
    rows = tb[2]
    k = row_index(rows, 0, '물류')
    first, second = rows[:k], [rows[0][:]] + rows[k:]
    d[j:j + 1] = [TB(tb[1], first), D('Caption', '물류 분야'), TB(tb[1], second)]
    assert i < j


def t1_e4(d):
    table_after(d, '1. 평가 기준')[1] = None


UNITS['tablestyle', 1] = dict(
    doc=T1, fm=DOC_FM, stem='form', names=T1_NAMES,
    write=dict(
        instruction=(
            'Write a new document, in this order: a heading level 1 "협력업체 현장 점검 계획"; the ordinary body '
            'paragraph "점검 기간: 2026. 11. 2. ~ 11. 20."; the label "표 1. 점검 일정" in the file\'s style for a '
            'label above a table; directly after it a table with a blue header row and light blue banded rows, with '
            'the columns 분야, 업체, 점검일, 점검자 for these rows, in this order (분야 / 업체 / 점검일 / 점검자):\n'
            '- 제조 / 성진테크 / 11월 3일 / 품질팀 김도현\n'
            '- 제조 / 우림산업 / 11월 5일 / 품질팀 김도현\n'
            '- 제조 / 한결부품 / 11월 10일 / 품질팀 오세린\n'
            '- 물류 / 서해운송 / 11월 12일 / 구매팀 정민재\n'
            '- 물류 / 누리물류 / 11월 19일 / 구매팀 정민재\n'
            'In the table, each 분야 should appear once for its rows, and a 점검자 once for consecutive rows with the '
            'same 점검자; no other cells are combined. After the table, the side remark "점검 결과는 11월 27일까지 '
            '보고한다." in small gray text.'),
        doc=[
            H(1, '협력업체 현장 점검 계획'),
            D('Body Text', '점검 기간: 2026. 11. 2. ~ 11. 20.'),
            D('Caption', '표 1. 점검 일정'),
            TB('Grid Table 4 Accent 1', split_rows('''
분야 | 업체 | 점검일 | 점검자
제조 | 성진테크 | 11월 3일 | 품질팀 김도현
^^ | 우림산업 | 11월 5일 | ^^
^^ | 한결부품 | 11월 10일 | 품질팀 오세린
물류 | 서해운송 | 11월 12일 | 구매팀 정민재
^^ | 누리물류 | 11월 19일 | ^^''')),
            D('Note', '점검 결과는 11월 27일까지 보고한다.'),
        ]),
    edits=[
        ('Give the table under "3. 등급별 조치" the table style with a gray header row and gray banded rows. '
         'Change nothing else.', t1_e1),
        ('Make the header row of the table under "2. 업체별 점수" red.', None,
         'no table style of this file has a red header row'),
        ('Split the table under "2. 업체별 점수" into two tables: the 제조 rows stay in the first table, and the '
         '물류 rows move to a second table that starts with a copy of the same header row. Between the two tables '
         'put the label "물류 분야" in the file\'s style for a label above a table. Both tables have the style of the '
         'original table. Everything else stays as it is.', t1_e3),
        ('The table under "1. 평가 기준" should have the default table style instead of its current style. Change '
         'nothing else.', t1_e4),
    ])

# ----------------------------------------------------------------------- tablestyle 2: training plan (Korean names)
T2_NAMES = {
    'paragraph_styles': [
        ('본문', '기본 본문'),
        ('표 제목', '표 바로 위의 표 제목 줄'),
        ('참고', '작은 회색 글씨의 참고 사항'),
    ],
    'table_styles': [
        ('표 눈금', '모든 칸에 가는 검은 실선'),
        ('눈금 표 4', '회색 머리글 행, 회색 줄무늬 행'),
        ('눈금 표 4 - 강조색 1', '파란 머리글 행, 연한 파란 줄무늬 행'),
        ('일반 표 1', '가는 가로선만, 세로선 없음'),
    ]}

T2 = [
    H(1, '2027년 임직원 교육 훈련 계획'),
    D('본문', '주관: 인재개발팀 / 시행: 2027. 1. 1. ~ 12. 31. / 대상: 전 임직원 420명'),
    H(2, '1. 기본 방향'),
    D('본문', '2027년 교육은 법정 의무 교육, 직무 교육, 리더십 교육, 외부 위탁 교육으로 나누어 운영한다.'),
    UL('법정 의무 교육 이수율 100% 유지'),
    UL('직무 교육 1인당 연 20시간 이상'),
    UL('리더십 교육 대상 확대 (팀장 → 파트장)'),
    D('본문', '교육 시간은 1인 기준 연간 최소 이수 시간으로 관리하며, 부서장은 분기마다 이수 현황을 점검한다.'),
    D('본문', '교육 과정별 세부 일정은 매 분기 시작 2주 전에 공지한다.'),
    D('본문', '2026년 이수율은 법정 의무 교육 100%, 직무 교육 87%, 리더십 교육 92%였다.'),
    H(2, '2. 법정 의무 교육'),
    D('본문', '법정 의무 교육은 관련 법령에 따라 전 직원이 이수해야 하며, 온라인 과정으로도 운영한다.'),
    D('표 제목', '표 1. 법정 의무 교육'),
    TB('눈금 표 4', split_rows('''
분야 | 과정명 | 시기 | 대상 | 시간
안전 | 산업안전보건 교육 | 분기별 | 전 직원 | 12
^^ | 소방 안전 교육 | 5월 | ^^ | 2
인권 | 성희롱 예방 교육 | 3월 | ^^ | 1
^^ | 장애인 인식 개선 교육 | 9월 | ^^ | 1
정보 | 개인정보 보호 교육 | 6월 | ^^ | 2''')),
    D('참고', '법정 의무 교육 미이수자는 인사 평가에 반영한다.'),
    D('본문', '신규 입사자는 입사 후 1개월 안에 법정 의무 교육을 이수한다.'),
    H(2, '3. 직무 교육'),
    D('본문', '직무 교육은 본부별 수요 조사 결과를 반영해 편성했으며, 공통 과정은 전 직원이 신청할 수 있다.'),
    D('표 제목', '표 2. 직무 교육'),
    TB('눈금 표 4', split_rows('''
분야 | 과정명 | 시기 | 대상 | 시간
영업 | 협상 실무 | 4월 | 영업본부 | 16
^^ | 고객 데이터 분석 | 7월 | ^^ | 12
생산 | 품질 관리 심화 | 5월 | 생산본부 | 16
^^ | 설비 예방 정비 | 8월 | ^^ | 8
공통 | 보고서 작성 | 수시 | 전 직원 | 4''')),
    D('참고', '직무 교육 시간은 1인 기준이다.'),
    D('본문', '영업본부와 생산본부 과정은 본부별 교육 담당자가 운영을 지원한다.'),
    H(2, '4. 리더십 및 외부 위탁 교육'),
    D('표 제목', '표 3. 리더십 교육'),
    TB('일반 표 1', split_rows('''
과정명 | 대상 | 시기 | 시간
신임 팀장 과정 | 신임 팀장 | 3월 | 24
팀장 역량 심화 | 팀장 | 6월 | 16
파트장 코칭 과정 | 파트장 | ^^ | ^^''')),
    D('참고', '리더십 과정은 사외 강사와 사내 임원이 함께 진행한다.'),
    D('본문', '팀장 역량 심화와 파트장 코칭 과정은 6월에 함께 운영하며, 시간은 과정별 16시간이다.'),
    D('본문', '리더십 교육은 사내에서, 전문 과정은 외부 기관에 위탁해 운영한다. 외부 위탁 과정은 다음과 같다.'),
    H(2, '5. 교육 예산'),
    D('표 제목', '표 4. 교육 예산 (단위: 천 원)'),
    TB('눈금 표 4 - 강조색 1', split_rows('''
구분 | 항목 | 금액
법정 의무 교육 | 강사료 | 12,000
^^ | 교재비 | 3,000
직무 교육 | 강사료 | 48,000
^^ | 교재비 | 9,500
리더십 교육 | 위탁 교육비 | 36,000
합계 | << | 108,500''')),
    D('참고', '예산은 2027년 사업계획 확정 시 조정될 수 있다.'),
    D('참고', '외부 위탁 과정의 교육비는 직무 교육 예산에서 집행한다.'),
    D('본문', '2027년 교육 예산은 2026년 대비 6% 늘어난 108,500천 원이다.'),
    H(2, '6. 교육 평가 및 사후 관리'),
    D('본문', '모든 과정은 만족도와 현업 적용도를 평가하고, 결과를 다음 연도 계획에 반영한다.'),
    D('표 제목', '표 5. 평가 방법'),
    TB('눈금 표 4', split_rows('''
구분 | 평가 항목 | 시기 | 방법
과정 평가 | 만족도 | 과정 종료 직후 | 설문
^^ | 학습 성취도 | ^^ | 사후 시험
현업 적용 | 적용도 | 종료 3개월 후 | 설문
^^ | 성과 기여 | 종료 6개월 후 | 팀장 면담''')),
    D('참고', '평가 결과는 인재개발팀이 분기별로 보고한다.'),
    D('본문', '현업 적용도는 교육 대상자와 소속 팀장이 함께 평가한다.'),
    D('본문', '만족도 4.0점 미만 과정은 다음 연도에 내용과 강사를 다시 검토한다.'),
    H(2, '7. 행정 사항'),
    UL('교육 신청은 사내 교육 시스템에서 받는다.'),
    UL('외부 교육 참가자는 결과 보고서를 1주일 안에 제출한다.'),
    UL('교육 시간은 근무 시간으로 인정한다.'),
    UL('교육 불참 시 부서장 승인을 받아 다음 차수로 이월한다.'),
    UL('사외 교육 비용은 사전 승인된 과정만 지원한다.'),
    UL('교육 관련 문의: 인재개발팀 (내선 4120)'),
    D('본문', '끝.'),
]


def t2_e1(d):
    table_after(d, '3. 직무 교육')[1] = '눈금 표 4 - 강조색 1'


def t2_e2(d):
    i = block_index(d, 'div', '리더십 교육은 사내에서, 전문 과정은 외부 기관에 위탁해 운영한다. 외부 위탁 과정은 다음과 같다.')
    d.insert(i + 1, TB('일반 표 1', split_rows('''
과정명 | 위탁 기관 | 기간
재무 회계 실무 | 한국생산성본부 | 5월 (3일)
데이터 분석 입문 | ^^ | 6월 (2일)
프로젝트 관리 | 대한상공회의소 | 9월 (3일)''')))


def t2_e4(d):
    i = block_index(d, 'h', '5. 교육 예산')
    k = i + 1
    while d[k][0] != 'h':
        k += 1
    section = d[i:k]
    del d[i:k]
    j = block_index(d, 'h', '2. 법정 의무 교육')
    d[j:j] = section


UNITS['tablestyle', 2] = dict(
    doc=T2, fm=FORM_FM, stem='form', names=T2_NAMES,
    write=dict(
        instruction=(
            'Write a new document, in this order: a heading level 1 "2027년 신입사원 입문 교육"; the ordinary body '
            'paragraph "대상: 2027년 상반기 신입사원 / 장소: 연수원"; the line "표 1. 입문 교육 일정" in the file\'s '
            'style for a table\'s title line; directly after it a table with a gray header row and gray banded rows, '
            'with the columns 일차, 시간, 과정, 강사 for these rows, in this order (일차 / 시간 / 과정 / 강사):\n'
            '- 1일차 / 오전 / 회사 소개 / 인재개발팀\n'
            '- 1일차 / 오후 / 조직 문화 / 인재개발팀\n'
            '- 2일차 / 오전 / 법정 의무 교육 / 외부 강사\n'
            '- 2일차 / 오후 / 법정 의무 교육 / 외부 강사\n'
            '- 3일차 / 종일 / 현업 부서 견학, written once across the 과정 and 강사 columns\n'
            'In the table, each 일차 should appear once for its rows, and within an 일차 a 과정 or a 강사 that repeats '
            'on its rows should appear once (each column on its own). After the table, the side remark "교육 일정은 '
            '인원에 따라 조정될 수 있다." in small gray text.'),
        doc=[
            H(1, '2027년 신입사원 입문 교육'),
            D('본문', '대상: 2027년 상반기 신입사원 / 장소: 연수원'),
            D('표 제목', '표 1. 입문 교육 일정'),
            TB('눈금 표 4', split_rows('''
일차 | 시간 | 과정 | 강사
1일차 | 오전 | 회사 소개 | 인재개발팀
^^ | 오후 | 조직 문화 | ^^
2일차 | 오전 | 법정 의무 교육 | 외부 강사
^^ | 오후 | ^^ | ^^
3일차 | 종일 | 현업 부서 견학 | <<''')),
            D('참고', '교육 일정은 인원에 따라 조정될 수 있다.'),
        ]),
    edits=[
        ('Give the 직무 교육 table (under "3. 직무 교육") the blue-header version of its current style. Change '
         'nothing else.', t2_e1),
        ('Right after the paragraph that ends "외부 위탁 과정은 다음과 같다.", add a table with light horizontal '
         'lines only, with the columns 과정명, 위탁 기관, 기간 and these rows, in this order: 재무 회계 실무 / '
         '한국생산성본부 / 5월 (3일); 데이터 분석 입문 / 한국생산성본부 / 6월 (2일); 프로젝트 관리 / 대한상공회의소 / '
         '9월 (3일). The 위탁 기관 should appear once for consecutive rows with the same 기관; no other cells are '
         'combined.', t2_e2),
        ('Make the lines of the table under "2. 법정 의무 교육" thicker so that it prints clearly.', None,
         'no table style of this file has thick lines'),
        ('Move the heading "5. 교육 예산" together with everything under it, up to the heading "6. 교육 평가 및 사후 '
         '관리" (its title line, table, notes and paragraph), so that it comes right before the heading "2. 법정 의무 '
         '교육". The table keeps its style. Do not change any text, including the heading numbers.', t2_e4),
    ])

# ----------------------------------------------------------------------- tablestyle 3: monthly complaint report
T3_NAMES = {
    'paragraph_styles': [
        ('Body Text', 'ordinary body paragraph'),
        ('Note', 'small gray text for side remarks'),
        ('Caption', '9 pt label directly above a table'),
    ],
    'table_styles': [
        ('Light Grid', 'thin gray lines, bold header row'),
        ('Light Grid - Accent 1', 'thin blue lines, bold blue header row'),
        ('Medium Grid 3', 'dark gray header row, shaded rows, white lines'),
        ('Medium Grid 3 - Accent 1', 'dark blue header row, shaded blue rows, white lines'),
    ]}

T3 = [
    H(1, '2026년 9월 고객 불만 처리 현황'),
    D('Body Text', '작성: 고객지원센터 / 집계 기간: 9월 1일 ~ 9월 30일 / 보고 대상: 고객경험본부장'),
    H(2, '1. 요약'),
    D('Body Text', '9월 접수 건수는 412건으로 전월 대비 6% 감소했고, 평균 처리 기간은 2.4일이었다.'),
    D('Body Text', '처리 중인 건은 17건이며, 모두 10월 첫째 주 안에 종결할 예정이다.'),
    UL('배송 지연 불만이 전체의 49%'),
    UL('채팅 접수 비중 확대 (24% → 28%)'),
    UL('환불 요청 처리 기간 목표 미달 (목표 1.5일, 실적 1.8일)'),
    H(2, '2. 유형별 접수 현황'),
    D('Caption', '표 1. 오프라인 채널 접수 (단위: 건)'),
    TB('Light Grid - Accent 1', split_rows('''
채널 | 유형 | 접수 | 처리 완료 | 처리 중
매장 | 배송 지연 | 42 | 40 | 2
^^ | 제품 불량 | 35 | 35 | 0
^^ | 환불 요청 | 28 | 26 | 2
콜센터 | 배송 지연 | 61 | 58 | 3
^^ | 제품 불량 | 30 | 29 | 1
^^ | 환불 요청 | 22 | 22 | 0''')),
    D('Body Text', '온라인 채널 접수는 다음과 같다.'),
    D('Caption', '표 2. 온라인 채널 접수 (단위: 건)'),
    TB('Light Grid - Accent 1', split_rows('''
채널 | 유형 | 접수 | 처리 완료 | 처리 중
홈페이지 | 배송 지연 | 38 | 36 | 2
^^ | 제품 불량 | 25 | 25 | 0
^^ | 환불 요청 | 17 | 15 | 2
채팅 | 배송 지연 | 60 | 57 | 3
^^ | 제품 불량 | 32 | 32 | 0
^^ | 환불 요청 | 22 | 20 | 2''')),
    D('Note', '처리 중 건수는 9월 30일 18시 기준이다.'),
    D('Body Text', '오프라인 접수는 218건, 온라인 접수는 194건이다.'),
    D('Body Text', '채팅 접수는 114건으로, 처음으로 콜센터 접수(113건)를 넘었다.'),
    H(2, '3. 처리 기간'),
    D('Caption', '표 3. 유형별 평균 처리 기간 (단위: 일)'),
    TB('Light Grid', split_rows('''
유형 | 채널 구분 | 8월 | 9월 | 목표
배송 지연 | 오프라인 | 2.8 | 2.5 | 2.0
^^ | 온라인 | 2.2 | 1.9 | ^^
제품 불량 | 오프라인 | 3.1 | 2.9 | 2.5
^^ | 온라인 | 2.7 | 2.6 | ^^
환불 요청 | 오프라인 | 2.0 | 2.1 | 1.5
^^ | 온라인 | 1.6 | 1.5 | ^^''')),
    D('Note', '목표는 2026년 고객 서비스 기준에 따른 값이다.'),
    D('Body Text', '환불 요청은 오프라인 처리 기간이 늘어 목표를 넘었다.'),
    D('Body Text', '배송 지연과 제품 불량은 온라인 채널에서 목표 대비 0.1일 이내로 근접했다.'),
    H(2, '4. 재발 방지 대책'),
    D('Body Text', '배송 지연 대책은 물류팀과 고객지원센터가 함께 추진한다.'),
    TB(None, split_rows('''
유형 | 대책 | 담당 | 기한
배송 지연 | 권역별 배송 협력사 추가 | 물류팀 | 10월 31일
^^ | 지연 예상 시 사전 문자 안내 | 고객지원센터 | 10월 15일
제품 불량 | 입고 검사 기준 강화 | 품질팀 | 11월 15일
환불 요청 | 환불 처리 단계 축소 (5단계 → 3단계) | 고객지원센터 | 10월 31일
공통 | 상담 품질 교육 (전 상담원) | ^^ | 11월 30일''')),
    D('Note', '대책별 진행 상황은 매주 월요일 점검한다.'),
    D('Body Text', '10월 말에 대책별 효과를 1차 점검해 11월 보고에 포함한다.'),
    H(2, '5. 채널별 고객 만족도'),
    D('Caption', '표 4. 채널별 만족도 (5점 만점)'),
    TB('Medium Grid 3', split_rows('''
채널 구분 | 채널 | 8월 | 9월 | 증감
오프라인 | 매장 | 4.1 | 4.2 | +0.1
^^ | 콜센터 | 3.8 | 3.9 | +0.1
온라인 | 홈페이지 | 3.9 | 3.9 | 0.0
^^ | 채팅 | 4.0 | 4.3 | +0.3''')),
    D('Note', '만족도는 처리 완료 고객 대상 문자 설문 결과이다.'),
    D('Body Text', '채팅 만족도가 가장 크게 올랐으며, 콜센터 만족도는 여전히 가장 낮다.'),
    H(2, '6. 10월 중점 관리'),
    UL('추석 연휴 전후 배송 지연 문의 대응 인력 20% 증원'),
    UL('채팅 상담 응답 시간 1분 이내 유지'),
    UL('환불 처리 단계 축소안 10월 31일 시행'),
    UL('콜센터 상담원 대상 불만 고객 응대 교육 (10월 2주차)'),
    H(2, '7. 우수 상담원'),
    TB('Light Grid', split_rows('''
채널 | 상담원 | 처리 건수 | 만족도
콜센터 | 김하늘 | 312 | 4.8
^^ | 이준서 | 298 | 4.7
채팅 | 박서윤 | 405 | 4.9''')),
    D('Note', '우수 상담원은 10월 월례 조회에서 시상한다.'),
    D('Body Text', '다음 보고는 11월 첫째 주에 한다.'),
]


def t3_e1(d):
    t = table_after(d, '3. 처리 기간')[2]
    t.append(['^^', '모바일 앱', '-', '1.2', '^^'])


def t3_e2(d):
    table_after(d, '4. 재발 방지 대책')[1] = 'Medium Grid 3 - Accent 1'


def t3_e4(d):
    first = table_after(d, '2. 유형별 접수 현황', 0)
    second = table_after(d, '2. 유형별 접수 현황', 1)
    i = d.index(first)
    j = d.index(second)
    assert j == i + 3
    first[2].extend(second[2][1:])
    del d[i + 1:j + 1]
    d[i - 1] = D('Caption', '표 1. 채널별 접수 (단위: 건)')


UNITS['tablestyle', 3] = dict(
    doc=T3, fm=DOC_FM, stem='form', names=T3_NAMES,
    write=dict(
        instruction=(
            'Write a new document, in this order: a heading level 1 "2026년 10월 상담원 교육 일정"; the label '
            '"표 1. 교육 일정" in the file\'s style for a label above a table; directly after it a table with thin '
            'gray lines and a bold header row, with the columns 주차, 과정, 대상, 시간 for these rows, in this order '
            '(주차 / 과정 / 대상 / 시간):\n'
            '- 1주차 / 응대 화법 / 신입 상담원 / 4시간\n'
            '- 1주차 / 불만 고객 응대 / 신입 상담원 / 4시간\n'
            '- 2주차 / 환불 규정 개정 안내 / 전 상담원 / 2시간\n'
            '- 2주차 / 채팅 상담 도구 / 전 상담원 / 2시간\n'
            '- 3주차 / 사례 발표회 (전 상담원 참석), written once across the 과정, 대상 and 시간 columns\n'
            'In the table, each 주차 should appear once for its rows, and within a 주차 a 대상 or a 시간 that repeats '
            'should appear once (each column on its own). After the table, the side remark "교육은 매주 수요일 14시에 '
            '시작한다." in small gray text.'),
        doc=[
            H(1, '2026년 10월 상담원 교육 일정'),
            D('Caption', '표 1. 교육 일정'),
            TB('Light Grid', split_rows('''
주차 | 과정 | 대상 | 시간
1주차 | 응대 화법 | 신입 상담원 | 4시간
^^ | 불만 고객 응대 | ^^ | ^^
2주차 | 환불 규정 개정 안내 | 전 상담원 | 2시간
^^ | 채팅 상담 도구 | ^^ | ^^
3주차 | 사례 발표회 (전 상담원 참석) | << | <<''')),
            D('Note', '교육은 매주 수요일 14시에 시작한다.'),
        ]),
    edits=[
        ('In the table under "3. 처리 기간", add a row for the new 채널 구분 "모바일 앱" of 환불 요청, right after its '
         '온라인 row, with 8월 "-" and 9월 1.2. It belongs to 환불 요청, and its 목표 is the same 1.5 as the other '
         '환불 요청 rows, shown once for all of them.', t3_e1),
        ('Give the table under "4. 재발 방지 대책" the table style with a dark blue header row. Change nothing '
         'else.', t3_e2),
        ('Make the table under "3. 처리 기간" span the full page width, with equal column widths.', None,
         'no table style of this file sets the table width or column widths'),
        ('Join the two tables under "2. 유형별 접수 현황" into one table in which the 온라인 rows directly follow the '
         '오프라인 rows. Remove the paragraph "온라인 채널 접수는 다음과 같다.", the label "표 2. 온라인 채널 접수 '
         '(단위: 건)" and the header row of the second table. Change the first label to "표 1. 채널별 접수 (단위: '
         '건)". The joined table keeps the style of the first table.', t3_e4),
    ])

# ======================================================================= slides
# ----------------------------------------------------------------------- slides 1: H2 sales strategy deck
K1 = [
    ['Title Slide', [['title', ['2026년 하반기 영업 전략']], ['body', ['영업기획팀 | 2026년 7월 3일']],
                     ['notes', ['인사 후 30초 안에 핵심 요약']]], []],
    ['Title and Content', [['title', ['목차']],
                           ['body', ['- 상반기 실적 요약', '- 시장 환경', '- 하반기 목표', '- 실행 계획',
                                     '- 요청 사항']]], []],
    ['Section Header', [['title', ['상반기 실적 요약']], ['body', ['1월 ~ 6월 누계']]], []],
    ['Title and Content', [['title', ['상반기 핵심 지표']],
                           ['body', ['- 매출 1조 2,480억 원 (전년 대비 +12%)', '- 영업이익 1,340억 원 (전년 대비 +9%)',
                                     '- 신규 고객 412곳', '- 고객 유지율 91%']],
                           ['notes', ['전년 대비 수치는 재무팀 확정치']]],
     [['s4', '출처', '출처: 재무팀 결산 (2026. 7. 1.)']]],
    ['Two Content', [['title', ['권역별 매출']],
                     ['left', ['- 수도권 5,120억 원', '- 영남권 3,260억 원', '- 충청권 1,480억 원']],
                     ['right', ['- 호남권 1,390억 원', '- 강원권 610억 원', '- 제주 620억 원']],
                     ['notes', ['권역 구분은 2026년 조직 기준']]],
     [['s5', '단위', '단위: 억 원, 상반기 누계']]],
    ['Title and Content', [['title', ['상반기 성과 요인']],
                           ['body', ['- 수도권 대형 거래처 3곳 신규 계약', '- 온라인 채널 매출 +31%',
                                     '- 생활용품 매출 전년 대비 +12%']]], []],
    ['Two Content', [['title', ['상반기 채널별 매출']],
                     ['left', ['- 오프라인 7,740억 원 (전년 대비 +5%)', '- 대형마트 3,900억 원', '- 대리점 3,840억 원']],
                     ['right', ['- 온라인 4,740억 원 (전년 대비 +31%)', '- 자사몰 2,100억 원', '- 오픈마켓 2,640억 원']],
                     ['notes', ['온라인 비중 38%로 확대']]], []],
    ['Section Header', [['title', ['시장 환경']], ['body', ['하반기 전망']]], []],
    ['Title and Content', [['title', ['하반기 시장 전망']],
                           ['body', ['- 소비 심리 완만한 회복 예상', '- 원자재 가격 안정세', '- 온라인 채널 경쟁 심화',
                                     '- 경쟁사 A 가격 인하 (평균 -5%)', '- 경쟁사 B 신제품 2종 출시 예정',
                                     '- 물류비 상승 지속 (+4%)', '- 환율 변동성 확대',
                                     '- 정부 소비 촉진 정책 (10월)']],
                           ['notes', ['경쟁사 동향은 질문 시 설명']]], []],
    ['Title and Content', [['title', ['경쟁사 동향']],
                           ['body', ['- 경쟁사 A: 가격 인하 (평균 -5%), 대형마트 행사 확대',
                                     '- 경쟁사 B: 신제품 2종 출시 예정 (9월)', '- 경쟁사 C: 온라인 전용 브랜드 출시']],
                           ['notes', ['경쟁사 동향은 질문 시 설명']]], []],
    ['Section Header', [['title', ['하반기 계획']], ['body', ['목표와 실행']]], []],
    ['Title and Content', [['title', ['하반기 목표']],
                           ['body', ['- 매출 1조 3,600억 원 (전년 대비 +12%)', '- 신규 고객 450곳',
                                     '- 고객 유지율 92%']],
                           ['notes', ['목표는 이사회 승인 전 잠정치']]], []],
    ['Title and Content', [['title', ['권역별 하반기 목표']],
                           ['body', ['- 수도권 5,600억 원 (전년 대비 +12%)', '- 영남권 3,500억 원 (전년 대비 +9%)',
                                     '- 기타 권역 4,500억 원 (전년 대비 +14%)']],
                           ['notes', ['목표는 이사회 승인 전 잠정치']]], []],
    ['Two Content', [['title', ['채널별 전략']],
                     ['left', ['- 오프라인: 대형마트 입점 확대', '- 오프라인: 지역 대리점 교육']],
                     ['right', ['- 온라인: 자사몰 멤버십 개편', '- 온라인: 라이브 커머스 월 2회']],
                     ['notes', ['질문 시 설명']]], []],
    ['Title and Content', [['title', ['고객 유지 전략']],
                           ['body', ['- 멤버십 등급 개편 (4단계 → 3단계)', '- 재구매 고객 쿠폰 자동 발송',
                                     '- 이탈 고객 전화 상담 (월 500곳)']],
                           ['notes', ['질문 시 설명']]], []],
    ['Title and Content', [['title', ['실행 계획']],
                           ['body', ['- 7월: 조직 개편 및 목표 배분', '- 8월: 신규 거래처 제안',
                                     '- 9월: 추석 프로모션', '- 10월 ~ 12월: 연말 성수기 대응']],
                           ['notes', ['질문 시 설명']]], []],
    ['Title and Content', [['title', ['하반기 프로모션']],
                           ['body', ['- 8월: 여름 정기 세일 (온라인 단독)', '- 9월: 추석 선물 세트 사전 예약',
                                     '- 11월: 블랙프라이데이 기획전', '- 12월: 연말 멤버십 감사 행사']],
                           ['notes', ['질문 시 설명']]], []],
    ['Title Only', [['title', ['요청 사항 요약']]],
     [['s12', '요약표', '요청 사항 3건: 인력 2명, 예산 5억 원, 시스템 1건']]],
    ['Title and Content', [['title', ['요청 사항']],
                           ['body', ['- 영업 인력 2명 충원', '- 판촉 예산 5억 원 추가', '- CRM 시스템 고도화']],
                           ['notes', ['질문 시 설명']]], []],
    ['Two Content', [['title', ['기대 효과']],
                     ['left', ['- 매출 전년 대비 +12%', '- 신규 고객 450곳']],
                     ['right', ['- 고객 유지율 92%', '- 영업이익률 11%']]],
     [['s16', '주석', '하반기 목표 기준 추정치']]],
    ['Title Slide', [['title', ['감사합니다']], ['body', ['문의: 영업기획팀 (내선 2314)']]], []],
]


def k1_e1(d):
    i = slide_index(d, '하반기 시장 전망')
    s = d[i]
    body = slot(s, 'body')
    first = ['Title and Content', [['title', ['하반기 시장 전망']], ['body', body[:4]], ['notes', slot(s, 'notes')]], []]
    second = ['Title and Content', [['title', ['하반기 시장 전망 (계속)']], ['body', body[4:]]], []]
    d[i:i + 1] = [first, second]


def k1_e2(d):
    s = d[slide_index(d, '권역별 매출')]
    left, right = slot(s, 'left'), slot(s, 'right')
    left.remove('- 충청권 1,480억 원')
    right.append('- 충청권 1,480억 원')


def k1_e3(d):
    s = d[slide_index(d, '실행 계획')]
    notes = slot(s, 'notes')
    notes[:] = ['9월 추석 프로모션은 마케팅팀과 공동 진행']


def k1_e4(d):
    i = slide_index(d, '하반기 목표')
    d.insert(i + 1, ['Two Content', [['title', ['목표 대비 리스크']],
                                     ['left', ['- 경쟁사 가격 인하', '- 물류비 상승']],
                                     ['right', ['- 대응: 번들 상품 확대', '- 대응: 물류 거점 통합']],
                                     ['notes', ['리스크 대응 방안은 8월 중 확정']]], []])


UNITS['slides', 1] = dict(
    deck=K1, fm=DECK_FM, stem='deck', names={'layouts': EN_LAYOUTS},
    write=dict(
        instruction=(
            'Write a new presentation with exactly these four slides, in this order:\n'
            '1. Layout Title Slide: title "2026년 4분기 영업 계획"; body "영업기획팀 | 2026년 10월 2일".\n'
            '2. Layout Title and Content: title "4분기 목표"; body bullets "매출 3,900억 원" and "신규 고객 120곳"; '
            'speaker notes "목표는 10월 경영회의에서 확정".\n'
            '3. Layout Two Content: title "채널별 과제"; left column bullets "오프라인: 연말 매대 확보" and '
            '"오프라인: 대리점 판촉 지원"; right column bullets "온라인: 블랙프라이데이 기획전" and '
            '"온라인: 멤버십 전환 캠페인".\n'
            '4. Layout Title Only: title "질의응답".'),
        deck=[
            ['Title Slide', [['title', ['2026년 4분기 영업 계획']], ['body', ['영업기획팀 | 2026년 10월 2일']]], []],
            ['Title and Content', [['title', ['4분기 목표']], ['body', ['- 매출 3,900억 원', '- 신규 고객 120곳']],
                                   ['notes', ['목표는 10월 경영회의에서 확정']]], []],
            ['Two Content', [['title', ['채널별 과제']],
                             ['left', ['- 오프라인: 연말 매대 확보', '- 오프라인: 대리점 판촉 지원']],
                             ['right', ['- 온라인: 블랙프라이데이 기획전', '- 온라인: 멤버십 전환 캠페인']]], []],
            ['Title Only', [['title', ['질의응답']]], []],
        ]),
    edits=[
        ('Split the slide "하반기 시장 전망" into two slides with the same layout. The first keeps the title and the '
         'first four bullets; the second, right after it, has the title "하반기 시장 전망 (계속)" and the other four '
         'bullets. The speaker notes stay with the first slide only.', k1_e1),
        ('On the slide "권역별 매출", the left column should list only 수도권 and 영남권. Move the bullet '
         '"충청권 1,480억 원" to the right column, as its last bullet. Change nothing else.', k1_e2),
        ('On the slide "실행 계획", replace the speaker notes with "9월 추석 프로모션은 마케팅팀과 공동 진행". Leave '
         'the notes of every other slide as they are.', k1_e3),
        ('Right after the slide "하반기 목표", add a slide with the two-column layout: title "목표 대비 리스크"; left '
         'column bullets "경쟁사 가격 인하" and "물류비 상승"; right column bullets "대응: 번들 상품 확대" and '
         '"대응: 물류 거점 통합"; speaker notes "리스크 대응 방안은 8월 중 확정".', k1_e4),
    ])

# ----------------------------------------------------------------------- slides 2: Q3 KPI review
K2 = [
    ['Title Slide', [['title', ['2026년 3분기 KPI 리뷰']], ['body', ['경영기획실 | 2026. 10. 8.']]], []],
    ['Title and Content', [['title', ['리뷰 순서']],
                           ['body', ['- 전사 KPI 요약', '- 사업부별 실적', '- 고객 지표', '- 4분기 과제']]], []],
    ['Title and Content', [['title', ['리뷰 기준']],
                           ['body', ['- 기간: 2026년 7월 ~ 9월', '- 실적: 사업부 보고 잠정치',
                                     '- 목표: 2026년 사업계획 (1월 확정)']]], []],
    ['Section Header', [['title', ['전사 KPI 요약']], ['body', ['3분기 누계 기준']]], []],
    ['Title and Content', [['title', ['전사 KPI']],
                           ['body', ['- 매출: 목표 대비 96%', '- 영업이익: 목표 대비 102%',
                                     '- 신규 수주: 목표 대비 88%', '- 고객 만족도: 4.3 / 5.0']]],
     [['s4', '기준', '기준: 3분기 누계, 잠정치']]],
    ['Two Content', [['title', ['목표 대비 실적']],
                     ['left', ['- 목표 달성: 영업이익, 고객 만족도']],
                     ['right', ['- 목표 미달: 매출, 신규 수주']],
                     ['notes', ['미달 항목은 사업부별 실적에서 설명']]], []],
    ['Title and Content', [['title', ['3분기 주요 이슈']],
                           ['body', ['- 원자재 가격 상승 (+7%)', '- 완성품 신제품 2종 조기 출시',
                                     '- 서비스 계약 갱신률 94% 유지', '- 부품 수주 2건 4분기로 이월']],
                           ['notes', ['이월 수주는 4분기 매출에 반영']]], []],
    ['Section Header', [['title', ['사업부별 실적']], ['body', ['3분기 누계 기준']]], []],
    ['Title and Content', [['title', ['부품사업부']],
                           ['body', ['- 매출: 목표 대비 91%', '- 원가 상승 영향 (원자재 +7%)',
                                     '- 신규 수주: 목표 대비 80%']],
                           ['notes', ['원가 상승은 4분기까지 지속 전망']]], []],
    ['Title and Content', [['title', ['완성품사업부']],
                           ['body', ['- 매출: 목표 대비 101%', '- 신제품 2종 조기 출시',
                                     '- 신규 수주: 목표 대비 95%']],
                           ['notes', ['질문 시 상세 설명']]], []],
    ['Title and Content', [['title', ['서비스사업부']],
                           ['body', ['- 매출: 목표 대비 98%', '- 유지보수 계약 갱신률 94%',
                                     '- 신규 수주: 목표 대비 90%']],
                           ['notes', ['질문 시 상세 설명']]], []],
    ['Two Content', [['title', ['사업부별 영업이익']],
                     ['left', ['- 부품: 목표 대비 94%', '- 완성품: 목표 대비 108%']],
                     ['right', ['- 서비스: 목표 대비 103%', '- 전사: 목표 대비 102%']],
                     ['notes', ['영업이익은 원가 절감 효과 반영']]], []],
    ['Title and Content', [['title', ['사업부별 인력']],
                           ['body', ['- 부품사업부 412명 (전년 대비 -3%)', '- 완성품사업부 538명 (전년 대비 +2%)',
                                     '- 서비스사업부 297명 (전년 대비 +5%)']],
                           ['notes', ['질문 시 상세 설명']]], []],
    ['Title and Content', [['title', ['재고 지표']],
                           ['body', ['- 재고 회전율: 목표 6.0회, 실적 5.2회', '- 장기 재고: 전년 대비 +8%',
                                     '- 결품률: 1.2% (목표 1.0%)']],
                           ['notes', ['질문 시 상세 설명']]], []],
    ['Section Header', [['title', ['고객 지표']], ['body', ['3분기 누계 기준']]], []],
    ['Two Content', [['title', ['고객 만족도']],
                     ['left', ['- 3분기: 4.3 / 5.0', '- 2분기: 4.1 / 5.0']],
                     ['right', ['- 불만 접수: 132건', '- 평균 처리: 2.4일']],
                     ['notes', ['질문 시 상세 설명']]],
     [['s11', '출처', '출처: 고객지원센터 월간 보고']]],
    ['Title and Content', [['title', ['불만 유형']],
                           ['body', ['- 배송 지연 49%', '- 제품 불량 22%', '- 환불 요청 21%', '- 기타 8%']],
                           ['notes', ['질문 시 상세 설명']]],
     [['s14', '출처', '출처: 고객지원센터 월간 보고']]],
    ['Two Content', [['title', ['채널별 불만 처리']],
                     ['left', ['- 오프라인: 평균 2.5일', '- 오프라인 목표: 2.0일']],
                     ['right', ['- 온라인: 평균 1.9일', '- 온라인 목표: 1.5일', '- 채팅 비중 28%']],
                     ['notes', ['질문 시 상세 설명']]], []],
    ['Section Header', [['title', ['4분기 계획']], ['body', ['과제와 일정']]], []],
    ['Title and Content', [['title', ['4분기 과제']],
                           ['body', ['- 부품사업부 원가 절감 TF 운영', '- 신규 수주 파이프라인 점검 (주 1회)',
                                     '- 완성품 신제품 판촉 강화', '- 서비스 계약 갱신 캠페인',
                                     '- 고객 불만 처리 1.5일 이내', '- 재고 회전율 개선', '- 인력 재배치 검토']],
                           ['notes', ['과제별 담당 임원은 다음 주 확정']]], []],
    ['Title and Content', [['title', ['4분기 일정']],
                           ['body', ['- 10월: 원가 절감 TF 구성', '- 11월: 수주 파이프라인 중간 점검',
                                     '- 12월: 연간 실적 예비 집계']],
                           ['notes', ['일정은 경영회의 후 확정']]], []],
    ['Two Content', [['title', ['요청 사항']],
                     ['left', ['- 원가 절감 TF 인력 3명']],
                     ['right', ['- 재고 관리 시스템 개선 예산 2억 원']],
                     ['notes', ['질문 시 상세 설명']]], []],
    ['Title Only', [['title', ['Q&A']]], []],
]


def k2_e1(d):
    i = slide_index(d, '고객 만족도')
    s = d.pop(i)
    j = slide_index(d, '전사 KPI')
    d.insert(j + 1, s)


def k2_e2(d):
    d.append(['Two Content', [['title', ['부록: 사업부별 수주 현황']],
                              ['left', ['- 부품: 목표 대비 80%', '- 완성품: 목표 대비 95%']],
                              ['right', ['- 서비스: 목표 대비 90%', '- 전사: 목표 대비 88%']]], []])


def k2_e3(d):
    i = slide_index(d, '4분기 과제')
    s = d[i]
    body = slot(s, 'body')
    first = ['Title and Content', [['title', ['4분기 과제']], ['body', body[:4]]], []]
    second = ['Title and Content', [['title', ['4분기 과제 (계속)']], ['body', body[4:]],
                                    ['notes', slot(s, 'notes')]], []]
    d[i:i + 1] = [first, second]


def k2_e4(d):
    s = d[slide_index(d, '전사 KPI')]
    s[1].append(['notes', ['수치는 잠정치, 10월 20일 확정']])


UNITS['slides', 2] = dict(
    deck=K2, fm=DECK_FM, stem='deck', names={'layouts': EN_LAYOUTS},
    write=dict(
        instruction=(
            'Write a new presentation with exactly these four slides, in this order:\n'
            '1. Layout Section Header: title "4분기 과제 점검"; body "11월 경영회의".\n'
            '2. Layout Two Content: title "과제 진행 현황"; left column bullets "완료: 원가 절감 TF 구성" and '
            '"완료: 갱신 캠페인 착수"; right column bullets "진행 중: 파이프라인 점검" and "지연: 재고 회전율 개선"; '
            'speaker notes "지연 과제는 담당 임원이 설명".\n'
            '3. Layout Title and Content: title "다음 달 계획"; body bullets "재고 회전율 개선안 보고", '
            '"인력 재배치안 확정" and "12월 경영회의 준비".\n'
            '4. Layout Title Only: title "Q&A".'),
        deck=[
            ['Section Header', [['title', ['4분기 과제 점검']], ['body', ['11월 경영회의']]], []],
            ['Two Content', [['title', ['과제 진행 현황']],
                             ['left', ['- 완료: 원가 절감 TF 구성', '- 완료: 갱신 캠페인 착수']],
                             ['right', ['- 진행 중: 파이프라인 점검', '- 지연: 재고 회전율 개선']],
                             ['notes', ['지연 과제는 담당 임원이 설명']]], []],
            ['Title and Content', [['title', ['다음 달 계획']],
                                   ['body', ['- 재고 회전율 개선안 보고', '- 인력 재배치안 확정',
                                             '- 12월 경영회의 준비']]], []],
            ['Title Only', [['title', ['Q&A']]], []],
        ]),
    edits=[
        ('Move the slide "고객 만족도" (with everything on it) so that it comes right after the slide "전사 KPI", '
         'before "목표 대비 실적". Change nothing else.', k2_e1),
        ('At the very end of the deck, after "Q&A", add a slide with the two-column layout: title "부록: 사업부별 '
         '수주 현황"; left column bullets "부품: 목표 대비 80%" and "완성품: 목표 대비 95%"; right column bullets '
         '"서비스: 목표 대비 90%" and "전사: 목표 대비 88%".', k2_e2),
        ('Split the slide "4분기 과제" into two slides with the same layout. The first keeps the title and the first '
         'four bullets; the second, right after it, has the title "4분기 과제 (계속)" and the remaining three '
         'bullets. The speaker notes move to the second slide; the first has none.', k2_e3),
        ('Give the slide "전사 KPI" the speaker notes "수치는 잠정치, 10월 20일 확정". Change nothing else.', k2_e4),
    ])

# ----------------------------------------------------------------------- slides 3: customer proposal (Korean layouts)
K3 = [
    ['제목 슬라이드', [['title', ['한빛유통 스마트 물류 솔루션 제안']], ['body', ['㈜누리시스템 | 2026년 10월']]], []],
    ['제목 및 내용', [['title', ['제안 개요']],
                     ['body', ['- 현황 진단', '- 제안 솔루션', '- 기대 효과', '- 추진 일정', '- 투자 비용']]], []],
    ['제목 및 내용', [['title', ['제안사 소개']],
                     ['body', ['- ㈜누리시스템: 물류 자동화 전문 기업 (2009년 설립)', '- 물류센터 자동화 구축 42건',
                               '- 유통·제조 고객 28곳']],
                     ['notes', ['회사 소개는 1분 이내']]], []],
    ['구역 머리글', [['title', ['현황 진단']], ['body', ['한빛유통 물류센터 3곳']]], []],
    ['콘텐츠 2개', [['title', ['현황과 과제']],
                  ['left', ['- 현황: 수작업 분류 60%', '- 현황: 출고 지연 월 120건']],
                  ['right', ['- 과제: 분류 자동화', '- 과제: 실시간 재고 가시성']],
                  ['notes', ['현장 인터뷰 결과 기반']]], []],
    ['제목 및 내용', [['title', ['분류 작업 현황']],
                     ['body', ['- 수작업 분류 인력 60명 (3교대)', '- 시간당 분류 1,800건', '- 오분류율 1.4%',
                               '- 성수기 초과 근무 월 평균 38시간']],
                     ['notes', ['현장 인터뷰 결과 기반']]], []],
    ['제목 및 내용', [['title', ['센터별 현황']],
                     ['body', ['- 이천 센터: 일 출고 8,000건', '- 용인 센터: 일 출고 5,500건',
                               '- 칠곡 센터: 일 출고 4,200건']]],
     [['s5', '출처', '출처: 한빛유통 내부 자료 (2026. 9.)']]],
    ['제목 및 내용', [['title', ['고객 요구 사항']],
                     ['body', ['- 분류 작업 자동화로 인력 의존도 축소', '- 센터 3곳 재고 실시간 통합 조회',
                               '- 기존 WMS와 연동 (추가 개발 최소화)', '- 성수기 전 1단계 가동',
                               '- 현장 작업자 교육 지원']],
                     ['notes', ['9월 워크숍에서 확인한 요구 사항']]], []],
    ['구역 머리글', [['title', ['제안 솔루션']], ['body', ['자동 분류 + 재고 관제']]], []],
    ['콘텐츠 2개', [['title', ['도입 전후 비교']],
                  ['left', ['- 도입 후', '- 분류 인력 25명', '- 출고 지연 월 20건 이하']],
                  ['right', ['- 도입 전', '- 분류 인력 60명', '- 출고 지연 월 120건']],
                  ['notes', ['인력 재배치 방안은 질문 시 설명']]], []],
    ['제목 및 내용', [['title', ['솔루션 구성']],
                     ['body', ['- 자동 분류기 (시간당 6,000건)', '- 재고 관제 대시보드', '- WMS 연동 모듈',
                               '- 모바일 검수 앱']],
                     ['notes', ['데모 영상 2분']]], []],
    ['콘텐츠 2개', [['title', ['적용 사례']],
                  ['left', ['- A사 물류센터', '- 분류 인력 45% 절감', '- 출고 지연 80% 감소']],
                  ['right', ['- B사 물류센터', '- 재고 정확도 99.7%', '- 구축 기간 4개월']],
                  ['notes', ['사례 고객사 이름은 공개 동의 후 사용']]],
     [['s9', '출처', '출처: ㈜누리시스템 구축 실적 (2025)']]],
    ['콘텐츠 2개', [['title', ['도입 방식 비교']],
                  ['left', ['- 구매형', '- 초기 투자 18억 원', '- 설비 자산 보유']],
                  ['right', ['- 임대형', '- 월 4,500만 원 (5년 약정)', '- 유지보수 포함']],
                  ['notes', ['고객 선호 방식 확인 필요']]], []],
    ['제목 및 내용', [['title', ['기대 효과']],
                     ['body', ['- 분류 인력 60명 → 25명', '- 출고 지연 월 120건 → 20건 이하',
                               '- 재고 정확도 97% → 99.5%']],
                     ['notes', ['인력 재배치 방안은 질문 시 설명']]], []],
    ['제목 및 내용', [['title', ['운영 지원 체계']],
                     ['body', ['- 전담 PM 1명, 현장 엔지니어 2명 상주', '- 24시간 원격 모니터링',
                               '- 장애 시 4시간 이내 현장 출동', '- 분기별 운영 보고서 제출']],
                     ['notes', ['인력 재배치 방안은 질문 시 설명']]], []],
    ['구역 머리글', [['title', ['추진 계획']], ['body', ['일정과 비용']]], []],
    ['제목 및 내용', [['title', ['추진 일정']],
                     ['body', ['- 1단계 (11월): 이천 센터 구축', '- 2단계 (1월): 용인 센터 확대',
                               '- 3단계 (3월): 칠곡 센터 확대 및 안정화']]], []],
    ['콘텐츠 2개', [['title', ['단계별 범위']],
                  ['left', ['- 1단계: 이천 센터', '- 자동 분류기 2대', '- 재고 관제 대시보드 구축']],
                  ['right', ['- 2~3단계: 용인·칠곡 센터', '- 자동 분류기 센터별 1대', '- 모바일 검수 앱 확대']]], []],
    ['콘텐츠 2개', [['title', ['투자 비용']],
                  ['left', ['- 초기 투자: 18억 원', '- 연간 운영비: 1.2억 원']],
                  ['right', ['- 회수 기간: 2.5년', '- 연간 절감: 7.4억 원']]],
     [['s11', '주석', 'VAT 별도, 3개 센터 기준']]],
    ['제목 및 내용', [['title', ['유지보수 조건']],
                     ['body', ['- 무상 유지보수 1년', '- 이후 연 1.2억 원 (연간 운영비에 포함)',
                               '- 소프트웨어 업데이트 연 2회', '- 부품 교체는 실비 청구']]], []],
    ['제목 및 내용', [['title', ['다음 단계']],
                     ['body', ['- 11월 1주: 현장 실사', '- 11월 3주: 상세 견적 제출', '- 12월: 계약 및 착수']],
                     ['notes', ['현장 실사 일정은 한빛유통 물류팀과 협의']]], []],
    ['콘텐츠 2개', [['title', ['협력 체계']],
                  ['left', ['- 한빛유통: 물류팀 (현장 운영)', '- 한빛유통: 정보팀 (WMS 연동)']],
                  ['right', ['- ㈜누리시스템: 구축 PM', '- ㈜누리시스템: 설비·SW 엔지니어']],
                  ['notes', ['주간 회의는 매주 화요일']]], []],
    ['제목만', [['title', ['감사합니다']]], []],
]


def k3_e1(d):
    s = d[slide_index(d, '도입 전후 비교')]
    left, right = slot(s, 'left'), slot(s, 'right')
    left[:], right[:] = right[:], left[:]


def k3_e2(d):
    i = slide_index(d, '추진 일정')
    s = d.pop(i)
    j = slide_index(d, '투자 비용')
    d.insert(j + 1, s)


def k3_e3(d):
    i = slide_index(d, '투자 비용')
    s = d[i]
    a = ['제목 및 내용', [['title', ['투자 비용']], ['body', slot(s, 'left')]], s[2]]
    b = ['제목 및 내용', [['title', ['투자 회수']], ['body', slot(s, 'right')]], []]
    d[i:i + 1] = [a, b]


def k3_e4(d):
    s = d[slide_index(d, '기대 효과')]
    s[1] = [x for x in s[1] if x[0] != 'notes']


UNITS['slides', 3] = dict(
    deck=K3, fm=DECK_FM, stem='deck', names={'layouts': KO_LAYOUTS},
    write=dict(
        instruction=(
            'Write a new presentation with exactly these four slides, in this order:\n'
            '1. Layout 제목 슬라이드: title "한빛유통 1단계 구축 결과 보고"; body "㈜누리시스템 | 2027년 1월".\n'
            '2. Layout 콘텐츠 2개: title "목표 대비 결과"; left column bullets "목표: 분류 인력 25명" and '
            '"목표: 출고 지연 월 20건 이하"; right column bullets "결과: 분류 인력 28명" and '
            '"결과: 출고 지연 월 17건"; speaker notes "인력은 2월까지 목표 달성 예정".\n'
            '3. Layout 제목 및 내용: title "2단계 계획"; body bullets "용인 센터 확대 (2월 착수)" and '
            '"교육 일정: 2월 첫째 주".\n'
            '4. Layout 제목만: title "감사합니다".'),
        deck=[
            ['제목 슬라이드', [['title', ['한빛유통 1단계 구축 결과 보고']], ['body', ['㈜누리시스템 | 2027년 1월']]], []],
            ['콘텐츠 2개', [['title', ['목표 대비 결과']],
                          ['left', ['- 목표: 분류 인력 25명', '- 목표: 출고 지연 월 20건 이하']],
                          ['right', ['- 결과: 분류 인력 28명', '- 결과: 출고 지연 월 17건']],
                          ['notes', ['인력은 2월까지 목표 달성 예정']]], []],
            ['제목 및 내용', [['title', ['2단계 계획']],
                             ['body', ['- 용인 센터 확대 (2월 착수)', '- 교육 일정: 2월 첫째 주']]], []],
            ['제목만', [['title', ['감사합니다']]], []],
        ]),
    edits=[
        ('On the slide "도입 전후 비교", the columns are the wrong way round: the 도입 전 bullets belong in the left '
         'column and the 도입 후 bullets in the right column. Swap the contents of the two columns. Change nothing '
         'else.', k3_e1),
        ('Move the slide "추진 일정" so that it comes right after the slide "투자 비용". Change nothing else.',
         k3_e2),
        ('Replace the slide "투자 비용" with two slides that use the layout 제목 및 내용: first "투자 비용", whose body '
         'is the bullets of the left column, then "투자 회수", whose body is the bullets of the right column. The '
         'shape of the original slide stays on the first of the two.', k3_e3),
        ('Remove the speaker notes from the slide "기대 효과". Every other slide keeps its notes.', k3_e4),
    ])
