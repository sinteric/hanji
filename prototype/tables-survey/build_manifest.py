"""Build manifest.csv once (needs network + scratch inputs; not needed to rerun the survey).

Inputs, all fetched by hand into SCRATCH (see SURVEY.md, "Sources"):
  gp/            treeless clone of github.com/wavelen-jw/GovPress_PDF_MD at GP_SHA
  gd_meta.json   GovDocs1 rows (url, retrieved) for the ids in by_type/docx.zip,
                 cut out of corpora/files/govdocs1/dump.sql
  corpus/        the fetched files (for sha256)
"""
import csv, hashlib, json, os, subprocess, sys

SCRATCH = sys.argv[1]
GP_SHA = "ed432313a05c8717e9a0bdc686edf0159cc0e501"
GP_DIR = "artifacts/policy_briefing_benchmark/corpus_latest100"
GD_ZIP = "https://digitalcorpora.s3.amazonaws.com/corpora/files/govdocs1/by_type/docx.zip"
GITHUB = [  # (repo, sha, path, licence)
    ("18F/handbook", "c220f93896b39e9e4ab0f486a1d5bf346ac06e3e", "downloads/TEMPLATE_Internal_kickoff_agenda.docx"),
    ("18F/handbook", "c220f93896b39e9e4ab0f486a1d5bf346ac06e3e", "downloads/TEMPLATE_Project_README.docx"),
    ("18F/handbook", "c220f93896b39e9e4ab0f486a1d5bf346ac06e3e", "downloads/TEMPLATE_Team_charter.docx"),
    ("18F/handbook", "c220f93896b39e9e4ab0f486a1d5bf346ac06e3e", "downloads/TEMPLATE_Weekly_ship_template.docx"),
    ("usds/playbook", "f5d71f1939efe87db9f0be414bcfa7c489e01c19", "assets/TechFAR Handbook_2014-08-07.docx"),
]


def sha(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()


rows = []
meta = json.loads(subprocess.check_output(
    ["git", "-C", os.path.join(SCRATCH, "gp"), "show", "%s:%s/manifest.json" % (GP_SHA, GP_DIR)]))
by_id = {d["news_item_id"]: d for d in meta["documents"]}
for f in sorted(os.listdir(os.path.join(SCRATCH, "corpus/hwpx"))):
    nid = f.split("-", 1)[1].rsplit(".", 1)[0]
    d = by_id[nid]
    m, dd, rest = d["approve_date"].split("/")
    rows.append(dict(
        id=f.rsplit(".", 1)[0], format="hwpx", lang="ko",
        collection="korea.kr press releases (via GovPress_PDF_MD corpus_latest100)",
        department=d["department"],
        original_url="https://www.korea.kr/briefing/pressReleaseView.do?newsId=%s" % nid,
        fetch_url="https://raw.githubusercontent.com/wavelen-jw/GovPress_PDF_MD/%s/%s/hwpxs/%s.hwpx" % (GP_SHA, GP_DIR, nid),
        member="",
        licence="KOGL Type 1 (공공누리 제1유형, attribution) for press-release text per korea.kr; "
                "images may belong to third parties; mirror repo is MIT (code). Not redistributed here.",
        date="%s-%s-%s" % (rest[:4], m, dd), sha256=sha(os.path.join(SCRATCH, "corpus/hwpx", f))))
gd = json.load(open(os.path.join(SCRATCH, "gd_meta.json")))
for f in sorted(os.listdir(os.path.join(SCRATCH, "corpus/docx"))):
    if not f.startswith("govdocs1-"):
        continue
    did = f[len("govdocs1-"):-5]
    g = gd[str(int(did))]
    rows.append(dict(
        id=f[:-5], format="docx", lang="en", collection="GovDocs1 (Digital Corpora), by_type/docx.zip",
        department=g["url"].split("/")[2], original_url=g["url"], fetch_url=GD_ZIP,
        member="%s/%s.docx" % (did[:3], did),
        licence="US government web document (GovDocs1 usg=YES); US federal works are public domain "
                "(17 U.S.C. 105), some .gov hosts are contractors/states. Not redistributed here.",
        date=g["retrieved"][:10] + " (retrieved)", sha256=sha(os.path.join(SCRATCH, "corpus/docx", f))))
for repo, rsha, path in GITHUB:
    f = "github-%s-%s" % (repo.replace("/", "_"), os.path.basename(path).replace(" ", "_"))
    rows.append(dict(
        id=f[:-5], format="docx", lang="en", collection="GitHub " + repo, department=repo.split("/")[0],
        original_url="https://github.com/%s/blob/%s/%s" % (repo, rsha, path.replace(" ", "%20")),
        fetch_url="https://raw.githubusercontent.com/%s/%s/%s" % (repo, rsha, path.replace(" ", "%20")),
        member="", licence="US Government work, public domain; 18F/handbook adds CC0 1.0 (LICENSE.md)",
        date=subprocess.check_output(["git", "-C", os.path.join(SCRATCH, repo.replace("/", "_")), "log", "-1",
                                      "--format=%cs", rsha]).decode().strip() + " (commit)",
        sha256=sha(os.path.join(SCRATCH, "corpus/docx", f))))
with open("manifest.csv", "w", newline="") as fh:
    w = csv.DictWriter(fh, fieldnames=list(rows[0]))
    w.writeheader()
    w.writerows(rows)
print(len(rows), "rows")
