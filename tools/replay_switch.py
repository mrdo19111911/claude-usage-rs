"""Replay a usage log; compare auto-switch rules "room" (old, spec Rule step 4) and
"deadline" (EDF, steps 1-3) on the same demand. Spec: docs/specs/auto-switch-deadline.md.
Usage: python -I -X utf8 tools/replay_switch.py LOG.jsonl [--no-resets] [--spread]
  --no-resets  ignore all weekly reset times (sanity check: rules should be close)
  --spread     spread a rise evenly over a polling gap instead of putting it in one step
"""
import json
import sys
from collections import defaultdict
from datetime import datetime

K, TH, WTH, STEP, HOUR, EPS = 6.7, 95.0, 99.0, 600_000, 3_600_000, 1e-9
FIVE_H, CAP5 = 5 * HOUR, TH / K  # 5-hour cap in weekly % units


def parse(line):
    r = json.loads(line)
    if r.get("seed"):
        return None
    if "t" in r:  # live format
        f, w = r["five"], r["week"]
        return (r["t"], r["id"], bool(r["active"]), f["u"], f["resetsAt"], w["u"], w["resetsAt"])
    t = int(datetime.fromisoformat(r["ts"].replace("Z", "+00:00")).timestamp() * 1000)
    f, w = r["five_hour"], r["seven_day"]
    ms = lambda s: None if s is None else int(s) * 1000
    return (t, r["account"], bool(r["active"]), f["used_percentage"], ms(f["resets_at"]),
            w["used_percentage"], ms(w["resets_at"]))


def load(path):
    with open(path, encoding="utf-8") as fh:
        rows = [p for p in (parse(x) for x in fh if x.strip()) if p and p[5] is not None]
    return sorted(rows, key=lambda r: r[0])


def cluster(values):
    """Distinct reset times; values within 1 h of each other are one reset (ms jitter)."""
    out = []
    for v in sorted(values):
        if not out or v - out[-1] > HOUR:
            out.append(v)
    return out


class Acc:
    def __init__(self, aid, week, five, five_reset, t0, resets):
        self.id, self.week, self.resets = aid, float(week), resets
        open_ = five_reset is not None and five_reset > t0
        self.five = (five or 0) / K if open_ else 0.0
        self.five_end = five_reset if open_ else None

    def deadline(self, t):
        return next((r for r in self.resets if r > t), None)

    def eligible(self):
        return self.five * K < TH - EPS and self.week < WTH - EPS

    def room_old(self):
        return min(100 - self.five * K, 100 - self.week)

    def room_now(self):
        return min(100 - self.week, (100 - self.five * K) / K)


def rank_edf(accs, t):
    el = [a for a in accs if a.eligible()]
    known = sorted((a for a in el if a.deadline(t) is not None), key=lambda a: a.deadline(t))
    groups, start = [], None
    for a in known:  # deadlines within 1 h of the group's first are equal
        if start is None or a.deadline(t) - start > HOUR:
            groups.append([])
            start = a.deadline(t)
        groups[-1].append(a)
    groups.append([a for a in el if a.deadline(t) is None])
    return [a for g in groups for a in sorted(g, key=lambda a: (-a.room_now(), a.id))]


def pick(rule, accs, cur, t):
    """Return the account to use (may be cur)."""
    if rule == "room":
        el = sorted((a for a in accs if a.eligible()), key=lambda a: (-a.room_old(), a.id))
        return cur if cur.eligible() or not el else el[0]
    ranked = rank_edf(accs, t)
    if not ranked or not cur.eligible():
        return ranked[0] if ranked else cur
    c = ranked[0]
    dc, du = c.deadline(t), cur.deadline(t)
    if c is cur or dc is None:
        return cur
    sooner = du is None or dc + HOUR <= du
    return c if sooner and c.five * K <= TH - 10 + EPS else cur


def add_rise(demand, t0, a, b, rise, spread):
    if not spread:
        demand[(b - t0) // STEP] += rise
        return
    for k in range((max(a, t0) - t0) // STEP, (b - t0) // STEP + 1):  # gap clipped to sim
        lo, hi = max(a, t0 + k * STEP), min(b, t0 + (k + 1) * STEP)
        if hi > lo:
            demand[k] += rise * (hi - lo) / (b - a)


def build(rows, no_resets, spread):
    ids = sorted({r[1] for r in rows})
    by = defaultdict(list)
    for r in rows:
        by[r[1]].append(r)
    have, t0 = set(), None
    for r in rows:  # first time every account has a non-null week resetsAt
        if r[6] is not None:
            have.add(r[1])
        if len(have) == len(ids):
            t0 = r[0]
            break
    if t0 is None:
        return None
    t_end = rows[-1][0]
    nsteps = (t_end - t0) // STEP + 1
    demand = [0.0] * nsteps
    for rs in by.values():
        for a, b in zip(rs, rs[1:]):
            if b[0] < t0 or b[5] <= a[5]:  # drop = weekly reset (or no rise)
                continue
            add_rise(demand, t0, a[0], b[0], b[5] - a[5], spread)
    init = {r[1]: r for r in rows if r[0] <= t0}  # rows are time-sorted: last one wins
    active = next((i for i in ids if init[i][2]), ids[0])
    resets = {i: [] if no_resets else cluster({r[6] for r in by[i] if r[6]}) for i in ids}
    return dict(ids=ids, t0=t0, t_end=t_end, demand=demand, init=init,
                active=active, resets=resets)


def simulate(rule, d):
    t0 = d["t0"]
    accs = [Acc(i, d["init"][i][5], d["init"][i][3], d["init"][i][4], t0, d["resets"][i])
            for i in d["ids"]]
    cur = next(a for a in accs if a.id == d["active"])
    served = unmet = lost = 0.0
    switches = nres = 0
    for s, dem in enumerate(d["demand"]):
        t = t0 + s * STEP
        for a in accs:
            for r in a.resets:
                if t - STEP < r <= t and r > t0:  # weekly reset inside this step
                    lost += 100 - a.week
                    a.week = 0.0
                    nres += 1
            if a.five_end is not None and a.five_end <= t:
                a.five, a.five_end = 0.0, None
        nxt = pick(rule, accs, cur, t)
        if nxt is not cur:
            switches, cur = switches + 1, nxt
        rem = dem
        for _ in range(len(accs) + 2):
            if rem <= EPS:
                break
            if cur.eligible():
                amt = min(rem, WTH - cur.week, CAP5 - cur.five)
                if amt > 0:
                    if cur.five_end is None:
                        cur.five_end = t + FIVE_H  # window opens at first use
                    cur.week += amt
                    cur.five += amt
                    served += amt
                    rem -= amt
            if rem <= EPS:
                break
            nxt = pick(rule, accs, cur, t)  # cap hit mid-step: re-pick
            if nxt is cur:
                break
            switches, cur = switches + 1, nxt
        unmet += max(rem, 0.0)
    return dict(demand=sum(d["demand"]), served=served, unmet=unmet, lost=lost,
                switches=switches, nres=nres)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    flags = [a for a in sys.argv[1:] if a.startswith("--")]
    rows = load(args[0])
    d = build(rows, "--no-resets" in flags, "--spread" in flags)
    print(f"log: {args[0]}  non-seed rows: {len(rows)}  accounts: "
          f"{len({r[1] for r in rows})}  {' '.join(flags)}")
    if d is None:
        print("no time where every account has a weekly resetsAt; nothing to replay")
        return
    span = (d["t_end"] - d["t0"]) / 86_400_000
    res = {rule: simulate(rule, d) for rule in ("room", "deadline")}
    print(f"{'rule':<9}{'demand':>9}{'served':>9}{'unmet':>8}{'lost@reset':>11}"
          f"{'switches':>9}{'days':>7}")
    for rule, r in res.items():
        print(f"{rule:<9}{r['demand']:>9.1f}{r['served']:>9.1f}{r['unmet']:>8.1f}"
              f"{r['lost']:>11.1f}{r['switches']:>9}{span:>7.2f}")
        assert abs(r["served"] + r["unmet"] - r["demand"]) < 1e-6, "served+unmet != demand"
    print(f"weekly resets inside the sim: {res['room']['nres']}")


if __name__ == "__main__":
    main()
