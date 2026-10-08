#!/usr/bin/env python3
"""Attribution ledger: which model / harness / session made each file.

Joins two sources:
  - git history: first commit adding each path, its author and Co-Authored-By trailers
  - boop agent_cmd: the session whose `git commit` ran within 5 min before that commit
  - boop agent_lane / agent_turn: that session's brief or first user message (init prompt), and
    the last user message before the file was written, for --annotate
  - boop db (~/.agent/boop.db): agent_touch write/edit rows for this repo and its
    worktrees, with harness (agent_session), model (agent_usage per turn, else the
    session's dominant model, else the lane's spawn model) and effort (agent_session_attr)

Usage:
  scripts/30_attribution_ledger.py [--format md|tsv] [--prefix src/modes/] [--all-editors]
  scripts/30_attribution_ledger.py --annotate   # rewrite the header block on every mode entrypoint

boop's touch log starts 2026-05-20 and only covers harnesses it ingests; older files
show git attribution only.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import textwrap
from collections import defaultdict
from datetime import datetime, timezone

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# main checkout, sibling worktrees (ascii-renderer-maha), and boop lanes (.boop-worktrees/<kind>/<name>/)
PATH_RE = re.compile(r".*/projects/ascii-renderer[^/]*/(?:\.boop-worktrees/[^/]+/[^/]+/)?(.+)$")

CTE = """
WITH turn_model AS (
  SELECT session_id, turn, max(model_id) AS model_id
  FROM agent_usage WHERE model_id IS NOT NULL GROUP BY 1, 2
), session_model AS (
  SELECT session_id, model_id FROM (
    SELECT session_id, model_id,
           row_number() OVER (PARTITION BY session_id ORDER BY count(*) DESC) AS rn
    FROM agent_usage WHERE model_id IS NOT NULL GROUP BY 1, 2
  ) WHERE rn = 1
), effort AS (
  SELECT session_id, value FROM (
    SELECT session_id, value,
           row_number() OVER (PARTITION BY session_id ORDER BY set_ts DESC) AS rn
    FROM agent_session_attr WHERE key = 'effort'
  ) WHERE rn = 1
)
"""

WHO_JOINS = """
LEFT JOIN agent_session s ON s.session_id = t.session_id
LEFT JOIN dict_branch b ON b.id = s.branch_id
LEFT JOIN turn_model tm ON tm.session_id = t.session_id AND tm.turn = t.turn
LEFT JOIN dict_model tm_d ON tm_d.id = tm.model_id
LEFT JOIN session_model sm ON sm.session_id = t.session_id
LEFT JOIN dict_model sm_d ON sm_d.id = sm.model_id
LEFT JOIN agent_lane l ON l.lane_id = t.session_id
LEFT JOIN dict_model lm_d ON lm_d.id = l.model_id
LEFT JOIN effort e ON e.session_id = t.session_id
LEFT JOIN v_turn_cwd v ON v.session_id = t.session_id AND v.turn = t.turn
"""

WHO_COLS = """s.harness, ds.value AS session,
       coalesce(tm_d.value, sm_d.value, lm_d.value) AS model,
       e.value AS effort, b.value AS branch"""

# relative paths are resolved against the turn's cwd
TOUCH_SQL = CTE + f"""
SELECT CASE WHEN p.value LIKE '/%' THEN p.value ELSE v.cwd || '/' || p.value END AS path,
       t.ts, t.verb, {WHO_COLS}
FROM agent_touch t
JOIN dict_path p ON p.id = t.path_id
JOIN dict_session ds ON ds.id = t.session_id
{WHO_JOINS}
WHERE t.verb IN ('edit', 'write')
  AND (p.value LIKE '%/projects/ascii-renderer%' OR v.cwd LIKE '%/projects/ascii-renderer%')
ORDER BY t.ts
"""

# shell commands that ran `git commit` in this repo, matched to commits by time
COMMIT_SQL = CTE + f"""
SELECT t.ts, {WHO_COLS}
FROM agent_cmd t
JOIN dict_session ds ON ds.id = t.session_id
{WHO_JOINS}
WHERE t.argline LIKE '%git%commit%'
  AND (v.cwd LIKE '%/projects/ascii-renderer%' OR t.argline LIKE '%ascii-renderer%')
ORDER BY t.ts
"""


def boop(sql):
    out = subprocess.run(["boop", "db", sql, "--format", "ndjson"],
                         capture_output=True, text=True, check=True).stdout
    return [json.loads(line) for line in out.splitlines() if line.strip()]


def boop_touches():
    by_path = defaultdict(list)
    for row in boop(TOUCH_SQL):
        m = PATH_RE.match(row["path"] or "")
        if m:
            by_path[m.group(1)].append(row)
    return by_path


def git_first_commits():
    fmt = "\x1e%h\x1f%ct\x1f%aI\x1f%an\x1f%s\x1f%(trailers:key=Co-Authored-By,valueonly,separator=%x3B)"
    out = subprocess.run(["git", "-C", REPO, "log", "--reverse", "--no-renames",
                          "--diff-filter=A", "--name-only", f"--format={fmt}"],
                         capture_output=True, text=True, check=True).stdout
    first = {}
    for rec in out.split("\x1e")[1:]:
        head, _, files = rec.partition("\n")
        sha, ct, date, author, subject, trailers = (head.split("\x1f") + [""] * 6)[:6]
        coauth = "; ".join(re.sub(r"\s*<[^>]*>", "", c).strip()
                           for c in trailers.split(";") if c.strip())
        for f in files.splitlines():
            f = f.strip()
            if f and f not in first:
                first[f] = dict(sha=sha, ct=int(ct) * 1000, date=date[:10], author=author,
                                coauthor=coauth, subject=subject)
    return first


def session_prompts(sessions):
    """Per session: the lane brief if boop spawned it, and every real user message with its ts."""
    if not sessions:
        return {}, {}
    ids = ",".join("'" + x.replace("'", "''") + "'" for x in sessions)
    turns = defaultdict(list)
    for r in boop(f"""
        SELECT ds.value AS session, t.ts, t.said FROM agent_turn t
        JOIN dict_session ds ON ds.id = t.session_id
        WHERE ds.value IN ({ids}) AND t.role = 'user' AND t.source_class != 'harness'
          AND t.said IS NOT NULL AND ltrim(t.said) NOT LIKE '<%'
          AND t.said NOT LIKE 'This is a transport readiness probe%'
        ORDER BY t.session_id, t.turn"""):
        turns[r["session"]].append(r)
    briefs = {r["session"]: r["body"] for r in boop(f"""
        SELECT ds.value AS session, m.body FROM agent_lane l
        JOIN dict_session ds ON ds.id = l.lane_id
        JOIN markdown_cache m ON m.markdown_id = l.brief_markdown_id
        WHERE ds.value IN ({ids})""")}
    return turns, briefs


def pick_prompts(origin, ts, turns, briefs):
    """(init, trigger): the session's opening prompt, and the last one before the piece was written."""
    msgs = turns.get(origin, [])
    init = briefs.get(origin) or (msgs[0]["said"] if msgs else None)
    before = [m for m in msgs if ts is None or m["ts"] <= ts]
    trigger = before[-1]["said"] if before else None
    return init, (trigger if trigger and trigger.strip() != (init or "").strip() else None)


def commit_sessions(commits, window_ms=300_000):
    """For each commit, the boop session whose `git commit` ran closest before it."""
    cmds = boop(COMMIT_SQL)
    hit = {}
    for c in commits:
        best = None
        for r in cmds:
            dt = c["ct"] - r["ts"]
            if -30_000 <= dt <= window_ms and (best is None or abs(dt) < abs(c["ct"] - best["ts"])):
                best = r
        if best:
            hit[c["sha"]] = best
    return hit


def repo_files():
    out = subprocess.run(["git", "-C", REPO, "ls-files", "--cached", "--others",
                          "--exclude-standard"], capture_output=True, text=True, check=True).stdout
    return [f for f in out.splitlines() if f]


def who(row):
    model = row.get("model") or "?"
    if row.get("effort"):
        model += f"@{row['effort']}"
    return f"{row.get('harness') or '?'}:{model}"


def day(ms):
    return datetime.fromtimestamp(ms / 1000, tz=timezone.utc).strftime("%Y-%m-%d")


BEGIN = "// attribution (generated by scripts/30_attribution_ledger.py --annotate; edits are overwritten)"
END = "// end attribution"
MODE_RE = re.compile(r"impl\s+(?:crate::registry::)?Mode\s+for\b")


def is_entrypoint(f):
    if not (f.startswith("src/modes/") and f.endswith(".rs")):
        return False
    with open(os.path.join(REPO, f)) as fh:
        return bool(MODE_RE.search(fh.read()))


PROMPT_LINES = 40


def quote(label, text, origin, cap):
    body = []
    for para in text.strip().splitlines():
        body += textwrap.wrap(para, 96) or [""]
    if len(body) > cap:
        body = body[:cap] + [f"... ({len(body) - cap} more lines: boop db chat {origin})"]
    return [f"//   {label} (session {origin[:8]}):"] + [f"//     {b}".rstrip() for b in body]


def annotate(f, row, editors, origin=None, init=None, trigger=None):
    _, sha, gdate, coauthor, commit_by, creator, session, bdate, branch = row[:9]
    lines = [BEGIN]
    if creator:
        lines.append(f"//   written by: {creator} (session {session}, {bdate}"
                     + (f", branch {branch})" if branch else ")"))
    if sha != "untracked":
        lines.append(f"//   first commit: {sha} {gdate}"
                     + (f" by {commit_by}" if commit_by else "")
                     + (f"; co-author {coauthor}" if coauthor else ""))
    else:
        lines.append("//   first commit: none (untracked)")
    if editors:
        lines.append(f"//   editors: {editors}")
    if init:
        lines += quote("init prompt", init, origin, PROMPT_LINES)
    if trigger:
        lines += quote("prompt before first write", trigger, origin, 15)
    lines.append(END)
    path = os.path.join(REPO, f)
    with open(path) as fh:
        text = fh.read()
    if text.startswith(BEGIN):
        text = text[text.index(END) + len(END):].lstrip("\n")
    with open(path, "w") as fh:
        fh.write("\n".join(lines) + "\n" + text)
    print(f"annotated {f}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--format", choices=["md", "tsv"], default="md")
    ap.add_argument("--prefix", default="", help="only paths starting with this")
    ap.add_argument("--all-editors", action="store_true",
                    help="list every harness:model that edited the file, not just the creator")
    ap.add_argument("--annotate", action="store_true",
                    help="write the attribution block atop each mode entrypoint (src/modes/*.rs with `impl Mode for`)")
    args = ap.parse_args()

    touches = boop_touches()
    first = git_first_commits()
    by_commit = commit_sessions({g["sha"]: g for g in first.values()}.values())
    cols = ["file", "git_commit", "git_date", "git_coauthor", "commit_by", "boop_creator",
            "boop_session", "boop_date", "boop_branch"]
    if args.all_editors:
        cols.append("boop_editors")

    rows, pending = [], []
    for f in sorted(repo_files()):
        if not f.startswith(args.prefix):
            continue
        g = first.get(f, {})
        t = touches.get(f, [])
        writes = [r for r in t if r["verb"] == "write"] or t
        c = writes[0] if writes else {}
        row = [f, g.get("sha", "untracked"), g.get("date", ""),
               g.get("coauthor") or (g.get("author", "") if g else ""),
               who(by_commit[g["sha"]]) if g.get("sha") in by_commit else "",
               who(c) if c else "", (c.get("session") or "")[:8], day(c["ts"]) if c else "",
               c.get("branch") or ""]
        editors = ", ".join(sorted({who(r) for r in t}))
        cb = by_commit.get(g.get("sha"), {})
        origin = c.get("session") or cb.get("session")
        origin_ts = c.get("ts") if c.get("session") else cb.get("ts")
        if args.all_editors:
            row.append(editors)
        rows.append(row)
        if args.annotate and is_entrypoint(f):
            pending.append((f, row, editors, origin, origin_ts))

    if args.annotate:
        turns, briefs = session_prompts({p[3] for p in pending if p[3]})
        for f, row, editors, origin, ts in pending:
            annotate(f, row, editors, origin, *pick_prompts(origin, ts, turns, briefs))
        return

    if args.format == "tsv":
        print("\t".join(cols))
        for r in rows:
            print("\t".join(r))
    else:
        print("| " + " | ".join(cols) + " |")
        print("|" + "---|" * len(cols))
        for r in rows:
            print("| " + " | ".join(x.replace("|", "\\|") for x in r) + " |")


if __name__ == "__main__":
    sys.exit(main())
