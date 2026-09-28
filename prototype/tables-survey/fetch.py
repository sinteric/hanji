"""Fetch the survey corpus listed in manifest.csv into ./corpus (not committed).

    python3 fetch.py [corpus_dir]

Every file is checked against its sha256. GovDocs1 files come out of one
29 MB zip on Digital Corpora's S3 bucket, downloaded once.
"""
import csv, hashlib, io, os, sys, urllib.request, zipfile

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "corpus")
HERE = os.path.dirname(os.path.abspath(__file__))


def get(url):
    with urllib.request.urlopen(url, timeout=120) as r:
        return r.read()


zips = {}
bad = 0
for row in csv.DictReader(open(os.path.join(HERE, "manifest.csv"))):
    dest = os.path.join(OUT, row["format"], row["id"] + "." + row["format"])
    if os.path.exists(dest) and hashlib.sha256(open(dest, "rb").read()).hexdigest() == row["sha256"]:
        continue
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    if row["member"]:
        if row["fetch_url"] not in zips:
            zips[row["fetch_url"]] = zipfile.ZipFile(io.BytesIO(get(row["fetch_url"])))
        data = zips[row["fetch_url"]].read(row["member"])
    else:
        data = get(row["fetch_url"])
    if hashlib.sha256(data).hexdigest() != row["sha256"]:
        print("sha256 mismatch:", row["id"])
        bad += 1
        continue
    open(dest, "wb").write(data)
print("done, %d mismatches" % bad)
