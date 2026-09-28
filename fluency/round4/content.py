"""Round 4 content: three Korean workbooks, the read questions and the write/edit tasks, in one form that belongs to
neither read view nor write shape. Every workbook is generated deterministically (fixed seeds).

A task's gold is given twice, as range operations (`ops`) and as code (`code`); build.py derives the text-edit gold
from the ops result and asserts that all three give the same workbook. A refusal task has ops = code = None."""
import json
import random

import wb as W

SURNAMES = list('김이박최정강조윤장임한오서신권황안송류홍')
GIVEN = ['민준', '서연', '도윤', '지우', '하준', '서윤', '시우', '하은', '주원', '지호', '예준', '수아', '지민', '준서',
         '유진', '현우', '채원', '건우', '다은', '우진', '지아', '선우', '서현', '연우', '민서', '은우', '예은', '정우',
         '수빈', '승현', '지훈', '소율', '태윤', '하린', '재원', '윤서', '동현', '가은', '성민', '나연', '영호', '미경',
         '상훈', '혜진', '종민', '보람', '경수', '은지', '태호', '수정']


def names(rng, n, avoid=()):
    out, seen = [], set(avoid)
    while len(out) < n:
        x = rng.choice(SURNAMES) + rng.choice(GIVEN)
        if x not in seen:
            seen.add(x)
            out.append(x)
    return out


def col(name, typ, fmt='@', formula=''):
    return W.make_column(name, typ, fmt, formula)


def table(name, anchor, cols, rows):
    _, r0 = W.parse_addr(anchor)
    return {'name': name, 'anchor': anchor, 'columns': cols,
            'rows': [{'orig': r0 + 1 + i, 'cells': list(r)} for i, r in enumerate(rows)]}


def r10k(x):
    return int(round(x / 10000.0)) * 10000


def phone(rng):
    return '010-%04d-%04d' % (rng.randint(2000, 9999), rng.randint(0, 9999))


def jdump(ops):
    return json.dumps(ops, ensure_ascii=False, indent=1)


# ================================================================ workbook 1: monthly sales and KPI

def wb_sales():
    rng = random.Random(4101)
    months = ['2026-%02d-01' % m for m in range(1, 9)]
    branches = ['강남', '서초', '송파', '분당', '일산']
    bf = {'강남': 1.25, '서초': 1.1, '송파': 1.0, '분당': 0.95, '일산': 0.8}
    groups = ['가전', '모바일', '생활']
    gf = {'가전': 15_500_000, '모바일': 12_000_000, '생활': 5_600_000}
    season = [0.92, 0.88, 1.0, 1.02, 1.05, 1.08, 1.1, 1.12]
    rows, pos = [], {}
    sales = {}
    for m, mo in enumerate(months):
        tot_s = tot_c = 0
        for b in branches:
            for g in groups:
                s = r10k(gf[g] * bf[b] * season[m] * rng.uniform(0.9, 1.1))
                sales[mo, b, g] = s
        sales['2026-06-01', '서초', '가전'] = sales['2026-05-01', '서초', '가전'] = 18_240_000   # near-duplicates
        for b in branches:
            for g in groups:
                s = sales[mo, b, g]
                c = r10k(s * rng.uniform(0.66, 0.78))
                p = s - c
                pos[mo, b, g] = 2 + len(rows)
                rows.append([mo, b, g, s, c, p])
                tot_s += s
                tot_c += c
        pos[mo, '소계'] = 2 + len(rows)
        rows.append([mo, '소계', None, tot_s, tot_c, tot_s - tot_c])
        if m < len(months) - 1:
            rows.append([None] * 6)
    # one row whose 이익 was typed wrong (two digits swapped)
    bad = pos['2026-04-01', '분당', '생활']
    r = rows[bad - 2]
    p = str(r[5])
    swapped = int(p[0] + p[2] + p[1] + p[3:]) if p[1] != p[2] else r[5] + 90_000
    r[5] = swapped
    t_sales = table('Sales', 'A1', [col('월', 'date', 'yyyy-mm'), col('지점', 'text'), col('제품군', 'text'),
                                    col('매출', 'number', '#,##0'), col('원가', 'number', '#,##0'),
                                    col('이익', 'number', '#,##0')], rows)

    krows = []
    kpos = {}
    for mo in months:
        for b in branches:
            act = sum(sales[mo, b, g] for g in groups)
            while True:
                goal = int(round(act * rng.uniform(0.9, 1.12) / 1_000_000.0)) * 1_000_000
                ratio = act / goal
                if abs(ratio - 0.95) > 0.002 and abs(ratio - 1.0) > 0.002:
                    break
            kpos[mo, b] = 2 + len(krows)
            krows.append([mo, b, goal, act, None])
    t_kpi = table('KPI', 'A1', [col('월', 'date', 'yyyy-mm'), col('지점', 'text'), col('목표', 'number', '#,##0'),
                                col('실적', 'number', '#,##0'), col('달성률', 'number', '0.0%', '=[@실적]/[@목표]')],
                  krows)

    ranks = ['사원', '대리', '과장']
    nm = names(rng, 15)
    ids = sorted(rng.sample(range(120, 9800), 15))
    srows = []
    for i, b in enumerate(branches):
        for k in range(3):
            j = i * 3 + k
            srows.append([ids[j], nm[j], b, ranks[k], '%04d-%02d-%02d' % (rng.randint(2012, 2025), rng.randint(1, 12),
                                                                          rng.randint(1, 28)), phone(rng)])
    t_staff = table('Staff', 'A1', [col('사번', 'number', '00000'), col('이름', 'text'), col('지점', 'text'),
                                    col('직급', 'text'), col('입사일', 'date', 'yyyy-mm-dd'), col('휴대전화', 'text')],
                    srows)
    book = {'sheets': [{'name': '매출', 'tables': [t_sales]}, {'name': 'KPI', 'tables': [t_kpi]},
                       {'name': '담당자', 'tables': [t_staff]}]}
    B = W.Book(book)

    # ---------------- read questions
    q1v = W.display(sales['2026-07-01', '송파', '모바일'], t_sales['columns'][3])
    ev = B.ev()
    ratios = ev.column(t_kpi, 4)
    q2 = sum(1 for x in ratios if x is not None and x < 0.95)
    staff_q = srows[8]         # 송파, 과장
    assert staff_q[2] == '송파' and staff_q[3] == '과장'
    q4 = W.display(staff_q[0], t_staff['columns'][0])
    a, b_ = pos['2026-03-01', '강남', '가전'], pos['2026-03-01', '일산', '생활']
    questions = [
        ('q1', 'value', 'In Sales, what is the 매출 of 송파, 제품군 모바일, in 2026-07? Give the value as the workbook '
                        'displays it.', [q1v]),
        ('q2', 'count', 'How many rows of KPI have a 달성률 below 95.0%? Give the number of rows.', [str(q2)]),
        ('q3', 'mismatch', 'In Sales, exactly one row\'s 이익 is not its 매출 minus its 원가. What is that row\'s sheet '
                           'row number?', [str(bad)]),
        ('q4', 'id', 'What is the 사번 of %s (%s 지점, %s) in Staff, exactly as the workbook displays it?'
         % (staff_q[1], staff_q[2], staff_q[3]), [q4]),
        ('q5', 'range', 'An A1-style formula must add up 원가 over the 15 branch rows of 2026-03 in Sales, without the '
                        '소계 row. Which range must it cover? Give an A1 range such as B2:B9.',
         ['E%d:E%d' % (a, b_)]),
    ]

    # ---------------- write and edit tasks
    ret_rows = [('2026-08', '강남', 1_240_000), ('2026-08', '서초', 860_000), ('2026-08', '송파', 1_015_000),
                ('2026-08', '분당', 472_000)]
    ret_cols = [{'name': '월', 'type': 'date', 'format': 'yyyy-mm'}, {'name': '지점', 'type': 'text', 'format': '@'},
                {'name': '반품액', 'type': 'number', 'format': '#,##0'},
                {'name': '반품률', 'type': 'number', 'format': '0.0%',
                 'formula': '=[@반품액]/SUMIFS(Sales[매출],Sales[월],[@월],Sales[지점],[@지점])'}]
    ret_dicts = [{'월': m + '-01', '지점': b, '반품액': v} for m, b, v in ret_rows]
    e1 = pos['2026-05-01', '서초', '가전']
    kpi_new = [('강남', 50_000_000, 48_720_000), ('서초', 41_000_000, 42_180_000), ('송파', 37_000_000, 35_930_000),
               ('분당', 35_000_000, 36_110_000), ('일산', 29_000_000, 27_560_000)]
    kpi_dicts = [{'월': '2026-09-01', '지점': b, '목표': g, '실적': a_} for b, g, a_ in kpi_new]
    tasks = [
        {'id': 'w', 'type': 'write',
         'instruction': 'On sheet 매출, add a new table named Returns whose top-left cell is H1, with four columns in '
                        'this order: 월 (a date, format yyyy-mm), 지점 (text), 반품액 (a number, format #,##0) and '
                        '반품률 (a number, format 0.0%). 반품률 is a formula column: the row\'s 반품액 divided by the '
                        'total 매출 of the same 월 and 지점 in the table Sales. The table has these rows, in this order: '
                        + '; '.join('%s, %s, %s' % (m, b, '{:,}'.format(v)) for m, b, v in ret_rows)
                        + '. Change nothing else.',
         'ops': [{'op': 'add_table', 'sheet': '매출', 'name': 'Returns', 'anchor': 'H1', 'columns': ret_cols,
                  'rows': ret_dicts}],
         'code': 'wb["매출"].add_table("Returns", "H1", %s, %s)\n' % (
             json.dumps(ret_cols, ensure_ascii=False), json.dumps(ret_dicts, ensure_ascii=False))},
        {'id': 'e1', 'type': 'edit',
         'instruction': 'In Sales, the 매출 of 서초, 제품군 가전, in 2026-05 should be 18,420,000. Change only that '
                        'cell; 이익 stays as it is.',
         'ops': [{'op': 'set', 'range': '매출!D%d' % e1, 'values': [[18_420_000]]}],
         'code': 'wb["매출"].cell("D%d").value = 18420000\n' % e1},
        {'id': 'e2', 'type': 'edit',
         'instruction': 'Append the 2026-09 rows to KPI, after its last row, in this order (지점, 목표, 실적): '
                        + '; '.join('%s, %s, %s' % (b, '{:,}'.format(g), '{:,}'.format(a_)) for b, g, a_ in kpi_new)
                        + '. 달성률 is a formula column. Change nothing else.',
         'ops': [{'op': 'append_rows', 'table': 'KPI', 'rows': kpi_dicts}],
         'code': 'for r in %s:\n    table("KPI").append(r)\n' % json.dumps(kpi_dicts, ensure_ascii=False)},
        {'id': 'e3', 'type': 'edit',
         'instruction': 'In Sales, 이익 was typed by hand and one row is wrong. Make 이익 a formula column: 매출 minus '
                        '원가, on every row. Change nothing else.',
         'ops': [{'op': 'fill_formula', 'table': 'Sales', 'column': '이익', 'formula': '=[@매출]-[@원가]'}],
         'code': 'table("Sales").fill_formula("이익", "=[@매출]-[@원가]")\n'},
        {'id': 'e4', 'type': 'edit',
         'instruction': 'In Staff, 사번 holds employee IDs but is typed as a number with format 00000. Make it a text '
                        'column, so that every ID keeps its leading zeros exactly as displayed now. Change nothing '
                        'else.',
         'ops': [{'op': 'set_type', 'table': 'Staff', 'column': '사번', 'type': 'text', 'format': '@'}],
         'code': 'table("Staff").set_type("사번", "text")\n'},
        {'id': 'e5', 'type': 'edit',
         'instruction': 'In KPI, add a column 환율 whose formula fetches today\'s KRW/USD rate with '
                        '=WEBSERVICE("https://fx.example.com/rate?pair=USDKRW"), so 실적 can be read in dollars.',
         'ops': None, 'code': None,
         'refuse': 'a formula that fetches data from the network (WEBSERVICE) is not allowed'},
    ]
    return {'stem': 'sales', 'file': '2026 영업실적.xlsx', 'book': book, 'questions': questions, 'tasks': tasks,
            'pos': pos}


# ================================================================ workbook 2: branch roster

def wb_roster():
    rng = random.Random(4202)
    branches = ['서울본점', '부산', '대구', '광주', '대전', '인천']
    depts = [('영업', 7), ('운영', 6), ('지원', 4)]
    ranks = ['사원', '대리', '과장', '차장', '부장']
    weights = [5, 4, 3, 2, 1]
    nm = names(rng, 102, avoid=('김서연', '이민준'))
    ids = rng.sample(range(1100, 99000), 102)
    rows, people = [], []
    pos_branch = {}
    k = 0
    for bi, b in enumerate(branches):
        first = 2 + len(rows)
        for d, n in depts:
            for _ in range(n):
                name = nm[k]
                if (b, d, k % 17) in (('부산', '영업', 3), ('대구', '영업', 5)):
                    name = '김서연'
                if (b, d, k % 17) in (('서울본점', '운영', 9), ('인천', '지원', 15)):
                    name = '이민준'
                rank = rng.choices(ranks, weights)[0]
                hired = '%04d-%02d-%02d' % (rng.randint(2008, 2026), rng.randint(1, 12), rng.randint(1, 28))
                if hired > '2026-08-31':           # no hire dates after the workbook's date
                    hired = '2025' + hired[4:]
                row = [ids[k], name, b, d, rank, hired, phone(rng), '%04d' % rng.randint(100, 3999)]
                rows.append(row)
                people.append(row)
                k += 1
        pos_branch[b] = (first, 1 + len(rows))
        if bi < len(branches) - 1:
            rows.append([None] * 8)
    t_roster = table('Roster', 'A1', [col('사번', 'number', '000000'), col('이름', 'text'), col('지점', 'text'),
                                      col('부서', 'text'), col('직급', 'text'), col('입사일', 'date', 'yyyy-mm-dd'),
                                      col('휴대전화', 'text'), col('내선', 'text')], rows)

    courses = [('개인정보 보호', 2), ('정보보안 기초', 3), ('직장 내 괴롭힘 예방', 1), ('산업안전 보건', 4), ('고객 응대', 2)]
    kim_b = [p for p in people if p[1] == '김서연' and p[2] == '부산'][0]
    kim_d = [p for p in people if p[1] == '김서연' and p[2] == '대구'][0]
    others = [p for p in people if p[1] not in ('김서연',)]
    picks = rng.sample(others, 28)
    kim_day = (7, 14)
    days = rng.sample([(m, d) for m in (6, 7, 8) for d in range(1, 29) if (m, d) != kim_day], 28)
    trows = []
    for i, p in enumerate(picks):
        if i == 12:
            for q, s in ((kim_b, 91), (kim_d, 84)):
                trows.append(['%06d' % q[0], q[1], q[2], '개인정보 보호', '2026-%02d-%02d' % kim_day, 2, s])
        c, h = rng.choice(courses)
        m, d = days[i]
        trows.append(['%06d' % p[0], p[1], p[2], c, '2026-%02d-%02d' % (m, d), h, rng.randint(68, 100)])
    trows.sort(key=lambda r: r[4])
    t_train = table('Training', 'A1', [col('사번', 'text'), col('이름', 'text'), col('지점', 'text'), col('과정', 'text'),
                                       col('이수일', 'date', 'yyyy-mm-dd'), col('시간', 'number', '0'),
                                       col('점수', 'number', '0')], trows)

    hrows = []
    hpos = {}
    for b in branches:
        for d, _ in depts:
            n = sum(1 for p in people if p[2] == b and p[3] == d)
            cap = n + rng.choice([0, 1, 1, 2])
            hpos[b, d] = 2 + len(hrows)
            hrows.append([b, d, cap, n, cap - n])
    bad = hpos['대전', '운영']
    hrows[bad - 2][3] -= 1           # 현원 typed one short of the roster
    hrows[bad - 2][4] += 1           # and 결원 typed from it
    t_head = table('Headcount', 'A1', [col('지점', 'text'), col('부서', 'text'), col('정원', 'number', '0'),
                                       col('현원', 'number', '0'), col('결원', 'number', '0')], hrows)
    book = {'sheets': [{'name': '명부', 'tables': [t_roster]}, {'name': '교육', 'tables': [t_train]},
                       {'name': '요약', 'tables': [t_head]}]}

    t_rows = {tuple(r[:4]): 2 + i for i, r in enumerate(trows)}
    kim_row = [i for i, r in enumerate(trows) if r[1] == '김서연' and r[2] == '대구'][0]
    q2 = sum(1 for p in people if p[2] == '대구' and p[4] == '대리')
    gw = pos_branch['광주']
    questions = [
        ('q1', 'value', 'In Training, what 점수 did 김서연 of the 부산 branch get in the course 개인정보 보호?',
         [str([r for r in trows if r[1] == '김서연' and r[2] == '부산'][0][6])]),
        ('q2', 'count', 'How many people in Roster work at the 대구 branch with the 직급 대리? Give the number of '
                        'people.', [str(q2)]),
        ('q3', 'mismatch', 'In Headcount, exactly one row\'s 현원 is not the number of people listed in Roster for '
                           'that 지점 and 부서. What is that row\'s sheet row number?', [str(bad)]),
        ('q4', 'id', 'What is the 사번 of 김서연 of the 대구 branch in Training, exactly as the workbook displays it?',
         [trows[kim_row][0]]),
        ('q5', 'range', 'An A1-style formula must count the 이름 cells of every person of the 광주 branch in Roster. '
                        'Which range must it cover? Give an A1 range such as B2:B9.', ['B%d:B%d' % gw]),
    ]

    ot_rows = [('서울본점', 412), ('부산', 288), ('대구', 305), ('광주', 196)]
    ot_cols = [{'name': '지점', 'type': 'text', 'format': '@'}, {'name': '월', 'type': 'date', 'format': 'yyyy-mm'},
               {'name': '초과근무', 'type': 'number', 'format': '#,##0'},
               {'name': '1인당', 'type': 'number', 'format': '0.0',
                'formula': '=[@초과근무]/COUNTIFS(Roster[지점],[@지점])'}]
    ot_dicts = [{'지점': b, '월': '2026-08-01', '초과근무': v} for b, v in ot_rows]
    e1row = 2 + [i for i, r in enumerate(trows) if r[1] == '김서연' and r[2] == '대구' and r[3] == '개인정보 보호'][0]
    new_t = [(people[3], '고객 응대', '2026-09-03', 2, 86), (people[40], '산업안전 보건', '2026-09-10', 4, 92),
             (people[64], '개인정보 보호', '2026-09-17', 2, 79), (people[95], '정보보안 기초', '2026-09-24', 3, 95)]
    new_dicts = [{'사번': '%06d' % p[0], '이름': p[1], '지점': p[2], '과정': c, '이수일': d, '시간': h, '점수': s}
                 for p, c, d, h, s in new_t]
    tasks = [
        {'id': 'w', 'type': 'write',
         'instruction': 'On sheet 요약, add a new table named Overtime whose top-left cell is G1, with four columns in '
                        'this order: 지점 (text), 월 (a date, format yyyy-mm), 초과근무 (a number, format #,##0) and '
                        '1인당 (a number, format 0.0). 1인당 is a formula column: the row\'s 초과근무 divided by the '
                        'number of rows of the table Roster whose 지점 is the row\'s 지점. The table has these rows, in '
                        'this order: ' + '; '.join('%s, 2026-08, %s' % (b, v) for b, v in ot_rows)
                        + '. Change nothing else.',
         'ops': [{'op': 'add_table', 'sheet': '요약', 'name': 'Overtime', 'anchor': 'G1', 'columns': ot_cols,
                  'rows': ot_dicts}],
         'code': 't = wb["요약"].add_table("Overtime", "G1", %s)\nfor r in %s:\n    t.append(r)\n' % (
             json.dumps(ot_cols, ensure_ascii=False), json.dumps(ot_dicts, ensure_ascii=False))},
        {'id': 'e1', 'type': 'edit',
         'instruction': 'In Training, the 점수 of 김서연 of the 대구 branch in the course 개인정보 보호 should be 88. '
                        'Change only that cell.',
         'ops': [{'op': 'set', 'range': '교육!G%d' % e1row, 'values': [[88]]}],
         'code': 'wb["교육"].cell("G%d").value = 88\n' % e1row},
        {'id': 'e2', 'type': 'edit',
         'instruction': 'Append the September 2026 completions to Training, after its last row, in this order '
                        '(사번, 이름, 지점, 과정, 이수일, 시간, 점수): '
                        + '; '.join('%s, %s, %s, %s, %s, %d, %d' % (x['사번'], x['이름'], x['지점'], x['과정'],
                                                                 x['이수일'], x['시간'], x['점수']) for x in new_dicts)
                        + '. Change nothing else.',
         'ops': [{'op': 'append_rows', 'table': 'Training', 'rows': new_dicts}],
         'code': 't = table("Training")\nfor r in %s:\n    t.append(r)\n' % json.dumps(new_dicts, ensure_ascii=False)},
        {'id': 'e3', 'type': 'edit',
         'instruction': 'In Headcount, 결원 was typed by hand. Make 결원 a formula column: 정원 minus 현원, on every '
                        'row. Change nothing else.',
         'ops': [{'op': 'fill_formula', 'table': 'Headcount', 'column': '결원', 'formula': '=[@정원]-[@현원]'}],
         'code': 'table("Headcount").fill_formula("결원", "=[@정원]-[@현원]")\n'},
        {'id': 'e4', 'type': 'edit',
         'instruction': 'In Roster, 사번 holds employee IDs but is typed as a number with format 000000. Make it a '
                        'text column, so that every ID keeps its leading zeros exactly as displayed now. Change '
                        'nothing else.',
         'ops': [{'op': 'set_type', 'table': 'Roster', 'column': '사번', 'type': 'text', 'format': '@'}],
         'code': 'table("Roster").set_type("사번", "text")\n'},
        {'id': 'e5', 'type': 'edit',
         'instruction': 'Add %s rows to Training, one for every person in Roster and each of the 7 courses planned '
                        'for 2027, with 이수일 and 점수 left empty, so that the HR team can fill them in later.'
                        % '{:,}'.format(7 * len(people)),
         'ops': None, 'code': None,
         'refuse': 'adding %s rows is bulk data entry; bulk rows come from an import, not from typed rows'
                   % '{:,}'.format(7 * len(people))},
    ]
    return {'stem': 'roster', 'file': '지점 인사명부.xlsx', 'book': book, 'questions': questions, 'tasks': tasks}


# ================================================================ workbook 3: budget with formulas

ACCOUNTS = [(411, '급여'), (421, '복리후생비'), (431, '교육훈련비'), (441, '여비교통비'), (451, '통신비'),
            (461, '소모품비'), (471, '지급수수료'), (481, '광고선전비'), (491, '차량유지비'), (511, '임차료'),
            (521, '수선비'), (531, '보험료'), (551, '회의비'), (581, '감가상각비'), (611, '외주가공비'), (651, '잡비')]


BASE = {'급여': 420, '복리후생비': 60, '교육훈련비': 28, '여비교통비': 40, '통신비': 18, '소모품비': 22, '지급수수료': 45,
        '광고선전비': 24, '차량유지비': 20, '임차료': 75, '수선비': 30, '보험료': 36, '회의비': 12, '감가상각비': 65,
        '외주가공비': 50, '잡비': 7}
TILT = {('영업', '광고선전비'): 3.5, ('개발', '광고선전비'): 0.3, ('생산', '광고선전비'): 0.2, ('품질', '광고선전비'): 0.2,
        ('생산', '외주가공비'): 3.0, ('생산', '수선비'): 2.2, ('개발', '교육훈련비'): 1.6, ('영업', '여비교통비'): 2.0,
        ('영업', '차량유지비'): 1.8, ('경영지원', '임차료'): 1.8}


def wb_budget():
    rng = random.Random(4303)
    depts = ['경영지원', '영업', '개발', '생산', '품질']
    scale = {'경영지원': 1.0, '영업': 1.2, '개발': 1.5, '생산': 1.8, '품질': 0.7}
    rows, pos = [], {}
    q3 = {}
    for d in depts:
        for code, item in ACCOUNTS:
            annual = BASE[item] * 1_000_000 * scale[d] * TILT.get((d, item), 1.0) * rng.uniform(0.7, 1.3)
            q3[d, item] = [r10k(annual / 4 * rng.uniform(0.85, 1.15)) for _ in range(4)]
    q3['영업', '복리후생비'][2] = q3['개발', '복리후생비'][2]          # near-duplicates
    for di, d in enumerate(depts):
        tot = [0, 0, 0, 0]
        for code, item in ACCOUNTS:
            qs = q3[d, item]
            pos[d, item] = 2 + len(rows)
            rows.append([code, d, item] + qs + [sum(qs)])
            tot = [a + b for a, b in zip(tot, qs)]
        pos[d, '소계'] = 2 + len(rows)
        rows.append([None, d, '소계'] + tot + [sum(tot)])
        if di < len(depts) - 1:
            rows.append([None] * 8)
    bad = pos['생산', '수선비']
    rows[bad - 2][7] += 1_000_000
    qcols = [col('%d분기' % q, 'number', '#,##0') for q in (1, 2, 3, 4)]
    t_budget = table('Budget', 'A1', [col('계정코드', 'number', '0000'), col('부서', 'text'), col('항목', 'text')] + qcols
                     + [col('합계', 'number', '#,##0')], rows)

    grades = [('사원', 38), ('대리', 46), ('과장', 58), ('차장', 70), ('부장', 84)]
    prows = []
    for d in depts:
        for g, pay in grades:
            prows.append([d, g, rng.randint(1, 14) if g in ('사원', '대리') else rng.randint(1, 6),
                          int(pay * rng.uniform(0.95, 1.08) * 10) * 100_000, None])
    t_pay = table('Payroll', 'A1', [col('부서', 'text'), col('직급', 'text'), col('인원', 'number', '0'),
                                    col('1인당 연봉', 'number', '#,##0'),
                                    col('인건비', 'number', '#,##0', '=[@인원]*[@[1인당 연봉]]')], prows)

    notes = ['법인카드', '세금계산서', '계좌이체', None, None]
    srows = []
    spos = {}
    for mi, mo in enumerate(('2027-01-01', '2027-02-01', '2027-03-01')):
        combos = rng.sample([(d, a) for d in depts for a in ACCOUNTS if a[1] not in ('급여', '상여금', '감가상각비')], 12)
        if mo == '2027-03-01' and not any(d == '품질' and a[1] == '교육훈련비' for d, a in combos):
            combos[5] = ('품질', (431, '교육훈련비'))
        for d, (code, item) in combos:
            spos[mo, d, item] = 2 + len(srows)
            srows.append([mo, '%04d' % code, item, d, r10k(rng.uniform(0.3, 12) * 1_000_000), rng.choice(notes)])
        if mi < 2:
            srows.append([None] * 6)
    t_spend = table('Spend', 'A1', [col('월', 'date', 'yyyy-mm'), col('계정코드', 'text'), col('항목', 'text'),
                                    col('부서', 'text'), col('집행액', 'number', '#,##0'), col('비고', 'text')], srows)
    book = {'sheets': [{'name': '예산', 'tables': [t_budget]}, {'name': '인건비', 'tables': [t_pay]},
                       {'name': '집행', 'tables': [t_spend]}]}

    q2 = sum(1 for r in srows if r[0] == '2027-02-01' and r[4] > 5_000_000)
    qrow = [r for r in srows if r[0] == '2027-03-01' and r[3] == '품질' and r[2] == '교육훈련비']
    assert len(qrow) == 1
    ya, yb = pos['영업', ACCOUNTS[0][1]], pos['영업', ACCOUNTS[-1][1]]
    questions = [
        ('q1', 'value', 'In Budget, what is the 2분기 amount of 개발\'s 지급수수료? Give the value as the workbook '
                        'displays it.', [W.fmt_number(q3['개발', '지급수수료'][1], '#,##0')]),
        ('q2', 'count', 'How many rows of Spend for 2027-02 have a 집행액 above 5,000,000? Give the number of rows.',
         [str(q2)]),
        ('q3', 'mismatch', 'In Budget, exactly one row\'s 합계 is not the sum of its 1분기 to 4분기. What is that '
                           'row\'s sheet row number?', [str(bad)]),
        ('q4', 'id', 'What 계정코드 does Spend give on the 2027-03 row of 품질\'s 교육훈련비, exactly as the workbook '
                     'displays it?', [qrow[0][1]]),
        ('q5', 'range', 'An A1-style formula must add up the 4분기 amounts of 영업\'s 16 accounts in Budget, without its '
                        '소계 row. Which range must it cover? Give an A1 range such as B2:B9.',
         ['G%d:G%d' % (ya, yb)]),
    ]

    hire = [('개발', 3, 52_000_000), ('품질', 2, 46_000_000), ('영업', 2, 48_500_000)]
    hire_cols = [{'name': '부서', 'type': 'text', 'format': '@'}, {'name': '충원 인원', 'type': 'number', 'format': '0'},
                 {'name': '1인당 연봉', 'type': 'number', 'format': '#,##0'},
                 {'name': '추가 인건비', 'type': 'number', 'format': '#,##0',
                  'formula': '=[@[충원 인원]]*[@[1인당 연봉]]'}]
    hire_dicts = [{'부서': d, '충원 인원': n, '1인당 연봉': p} for d, n, p in hire]
    e1 = pos['개발', '복리후생비']
    e1_new = q3['개발', '복리후생비'][2] + 2_200_000
    new_s = [('0441', '여비교통비', '영업', 3_420_000, '법인카드'), ('0431', '교육훈련비', '개발', 5_800_000, '세금계산서'),
             ('0521', '수선비', '생산', 7_150_000, '세금계산서'), ('0461', '소모품비', '품질', 640_000, None),
             ('0551', '회의비', '경영지원', 910_000, '법인카드')]
    new_dicts = [dict({'월': '2027-04-01', '계정코드': c, '항목': i, '부서': d, '집행액': v}, **({'비고': n} if n else {}))
                 for c, i, d, v, n in new_s]
    tasks = [
        {'id': 'w', 'type': 'write',
         'instruction': 'On sheet 인건비, add a new table named Hiring whose top-left cell is H1, with four columns in '
                        'this order: 부서 (text), 충원 인원 (a number, format 0), 1인당 연봉 (a number, format #,##0) and '
                        '추가 인건비 (a number, format #,##0). 추가 인건비 is a formula column: the row\'s 충원 인원 '
                        'times its 1인당 연봉. The table has these rows, in this order: '
                        + '; '.join('%s, %d, %s' % (d, n, '{:,}'.format(p)) for d, n, p in hire)
                        + '. Change nothing else.',
         'ops': [{'op': 'add_table', 'sheet': '인건비', 'name': 'Hiring', 'anchor': 'H1', 'columns': hire_cols,
                  'rows': hire_dicts}],
         'code': 'wb["인건비"].add_table("Hiring", "H1", %s, %s)\n' % (
             json.dumps(hire_cols, ensure_ascii=False), json.dumps(hire_dicts, ensure_ascii=False))},
        {'id': 'e1', 'type': 'edit',
         'instruction': 'In Budget, the 3분기 amount of 개발\'s 복리후생비 should be %s. Change only that cell; '
                        % '{:,}'.format(e1_new) +
                        '합계 stays as it is.',
         'ops': [{'op': 'set', 'range': '예산!F%d' % e1, 'values': [[e1_new]]}],
         'code': 'wb["예산"].cell("F%d").value = %d\n' % (e1, e1_new)},
        {'id': 'e2', 'type': 'edit',
         'instruction': 'Append the 2027-04 spending to Spend, directly after its last row, in this order (계정코드, '
                        '항목, 부서, 집행액, 비고): '
                        + '; '.join('%s, %s, %s, %s, %s' % (c, i, d, '{:,}'.format(v), n or '(empty)')
                                    for c, i, d, v, n in new_s) + '. Change nothing else.',
         'ops': [{'op': 'append_rows', 'table': 'Spend', 'rows': new_dicts}],
         'code': 't = table("Spend")\nfor r in %s:\n    t.append(r)\n' % json.dumps(new_dicts, ensure_ascii=False)},
        {'id': 'e3', 'type': 'edit',
         'instruction': 'In Budget, 합계 was typed by hand and one row is wrong. Make 합계 a formula column: the sum of '
                        '1분기 to 4분기, on every row. Change nothing else.',
         'ops': [{'op': 'fill_formula', 'table': 'Budget', 'column': '합계',
                  'formula': '=SUM(Budget[@[1분기]:[4분기]])'}],
         'code': 'table("Budget").fill_formula("합계", "=SUM(Budget[@[1분기]:[4분기]])")\n'},
        {'id': 'e4', 'type': 'edit',
         'instruction': 'In Budget, 계정코드 holds account codes but is typed as a number with format 0000. Make it a '
                        'text column, so that every code keeps its leading zero exactly as displayed now. Change '
                        'nothing else.',
         'ops': [{'op': 'set_type', 'table': 'Budget', 'column': '계정코드', 'type': 'text', 'format': '@'}],
         'code': 'table("Budget").set_type("계정코드", "text")\n'},
        {'id': 'e5', 'type': 'edit',
         'instruction': 'In Budget, colour every row whose 합계 is over 100,000,000 red, so that the large accounts '
                        'stand out.',
         'ops': None, 'code': None,
         'refuse': 'colours are direct formatting, and only a column\'s type and number format can be set'},
    ]
    return {'stem': 'budget', 'file': '2027 부서별 예산.xlsx', 'book': book, 'questions': questions, 'tasks': tasks}


WORKBOOKS = {1: wb_sales, 2: wb_roster, 3: wb_budget}
