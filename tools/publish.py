# Makes the public branch `main` from `dev`: only what builds and installs the app.
# Kept out: tests (files and #[cfg(test)] items), backups, specs, research notes, this
# machine's cargo config. The result must build (cargo build --release) before it is
# committed. `dev` stays local; `main` is what GitHub shows.
#
#   python tools/publish.py          # update main in target/public, commit, no push
#   python tools/publish.py --push   # same, then push main to origin

import os, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WT = ROOT / "target" / "public"
SRC_REF = "dev"
KEEP = [r"^src/", r"^Cargo\.(toml|lock)$", r"^\.gitignore$", r"^installer/(claude-usage\.iss|build\.ps1)$"]
DROP = [r"_tests\.rs$", r"(^|/)tests\.rs$", r"^src/tray/preview\.rs$"]
README = "tools/README.public.md"


def git(*args, cwd=ROOT, out=True):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=out, text=True, encoding="utf-8")
    if r.returncode:
        sys.exit(f"git {' '.join(args)} failed:\n{r.stderr}")
    return r.stdout if out else ""


def public_files():
    names = git("ls-tree", "-r", "--name-only", SRC_REF).splitlines()
    keep = lambda n: any(re.search(k, n) for k in KEEP) and not any(re.search(d, n) for d in DROP)
    return [n for n in names if keep(n)]


def strip_tests(text):
    # Drops each `#[cfg(test)]` item: a one-line `mod x;` (with its #[path]) or a block.
    lines, out, i = text.split("\n"), [], 0
    while i < len(lines):
        if lines[i].strip() != "#[cfg(test)]":
            out.append(lines[i])
            i += 1
            continue
        j = i + 1
        while lines[j].strip().startswith("#["):
            j += 1
        if not lines[j].rstrip().endswith(";"):
            depth = 0
            while True:
                depth += lines[j].count("{") - lines[j].count("}")
                if depth == 0 and "{" in "".join(lines[i:j + 1]):
                    break
                j += 1
        i = j + 1
        # No blank line left hanging at the end of the file or doubled.
        if out and out[-1] == "" and (i >= len(lines) or lines[i] == ""):
            out.pop()
    return "\n".join(out)


def ensure_worktree():
    if (WT / ".git").exists():
        return
    has_main = git("branch", "--list", "main").strip()
    if has_main:
        git("worktree", "add", str(WT), "main")
    else:
        git("worktree", "add", "--orphan", "-b", "main", str(WT))


def fill():
    for p in WT.iterdir():
        if p.name != ".git":
            git("rm", "-rq", "--cached", "--ignore-unmatch", p.name, cwd=WT)
            subprocess.run(["cmd", "/c", "rmdir", "/s", "/q", str(p)] if p.is_dir() else ["cmd", "/c", "del", "/q", str(p)])
    for name in public_files() + [README]:
        dest = WT / ("README.md" if name == README else name)
        dest.parent.mkdir(parents=True, exist_ok=True)
        if name.endswith(".rs"):
            dest.write_text(strip_tests(git("show", f"{SRC_REF}:{name}")), encoding="utf-8", newline="")
        else:
            # Byte for byte: images (the dialog's screenshots) are not text.
            dest.write_bytes(subprocess.run(["git", "show", f"{SRC_REF}:{name}"], cwd=ROOT, capture_output=True, check=True).stdout)


def check():
    left = [str(p.relative_to(WT)) for p in WT.rglob("*.rs") if "cfg(test)" in p.read_text(encoding="utf-8")]
    if left:
        sys.exit(f"test code left in: {left}")
    env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target" / "public-build"))
    r = subprocess.run(["cargo", "build", "--release", "--quiet"], cwd=WT, env=env)
    if r.returncode:
        sys.exit("the public tree does not build")


def commit(push):
    git("add", "-A", cwd=WT)
    if not git("status", "--porcelain", cwd=WT).strip():
        print("main is already up to date")
    else:
        sha = git("rev-parse", "--short", SRC_REF).strip()
        msg = f"Public snapshot of dev {sha}\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
        git("commit", "-q", "-m", msg, cwd=WT)
        print(git("log", "--oneline", "-1", cwd=WT).strip())
    if push:
        git("push", "-u", "origin", "main", cwd=WT, out=False)


if __name__ == "__main__":
    ensure_worktree()
    fill()
    check()
    commit("--push" in sys.argv)
