#!/usr/bin/env python3
"""Measure warm Warden UI edits with the normal Cargo wrapper; restore source.

Run after other builds finish. Generated logs/results stay in reports/warden-rust.
No account workers or credentials are involved. Does not benchmark startup/GPU.
"""
import json
from pathlib import Path
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "reports/warden-rust"
SOURCE = ROOT / "src/ui.rs"
CMD = ["cargo", "build", "--locked", "--offline", "-p", "plicowarden", "--bin", "plicowarden", "--message-format=json"]


def run(name):
    start = time.perf_counter()
    result = subprocess.run(CMD, cwd=ROOT, capture_output=True, text=True)
    elapsed = time.perf_counter() - start
    (OUT / f"build-{name}.log").write_text(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f"build failed: {name}")
    changed = []
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") == "compiler-artifact" and not message["fresh"]:
            changed.append((message["target"]["name"], message["target"]["kind"]))
    return dict(seconds=elapsed, artifacts=changed)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    original = SOURCE.read_bytes()
    title = b'text(ui, "Warden")'
    layout = b'.height_percent(100.).padding(12).gap(18)'
    assert original.count(title) == 1 and original.count(layout) == 1
    rows = []
    try:
        run("warmup")
        SOURCE.write_bytes(original.replace(title, f'text(ui, "Warden warmup {time.time_ns()}")'.encode(), 1))
        run("edit-warmup")
        for sample in range(5):
            for work in (["text", "layout"] if sample % 2 == 0 else ["layout", "text"]):
                if work == "text":
                    source = original.replace(title, f'text(ui, "Warden benchmark {time.time_ns()}")'.encode(), 1)
                else:
                    source = original.replace(layout, f'.height_percent(100.).padding(12).gap({19 + sample})'.encode(), 1)
                SOURCE.write_bytes(source)
                row = run(f"{work}-{sample}")
                assert len(row["artifacts"]) == 2 and all(name == "plicowarden" for name, _ in row["artifacts"]), row["artifacts"]
                row.update(workload=work, sample=sample)
                rows.append(row)
                print(f'{work} {sample + 1}/5: {row["seconds"]:.3f}s', flush=True)
    finally:
        SOURCE.write_bytes(original)
        run("restore")
        (OUT / "build-results.json").write_text(json.dumps(rows, indent=2) + "\n")
    print({work: statistics.median(r["seconds"] for r in rows if r["workload"] == work) for work in ("text", "layout")})


if __name__ == "__main__":
    main()
