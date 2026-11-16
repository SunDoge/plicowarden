#!/usr/bin/env python3
"""Typecheck a TS recipe suite, generate a plan, build once, and run Warden fixtures."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--suite', type=Path, default=ROOT / 'recipes/suite.ts')
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/plicowarden-recipes/latest')
    parser.add_argument('--semantic', action='store_true', help='skip GPU and PNG output')
    parser.add_argument('--no-build', action='store_true', help='reuse the previously compiled Rust runner')
    parser.add_argument('--runner', type=Path, help='explicit prebuilt runner for --no-build')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    times = {}

    def command(name, argv, timeout=120):
        started = time.perf_counter()
        result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
        times[name] = time.perf_counter() - started
        (output / f'{name}.log').write_text(result.stdout + result.stderr)
        if result.returncode:
            print(f'{name} failed; see {output / f"{name}.log"}', file=sys.stderr)
            print(result.stdout + result.stderr, file=sys.stderr)
            raise SystemExit(result.returncode)
        return result

    package = ROOT / 'tools/recipes'
    command('dependencies', ['bun', 'install', '--frozen-lockfile'], 600)
    command('typecheck', ['bun', 'run', '--bun', '--cwd', str(package), 'check'])
    plan = output / 'input-plan.json'
    command('generate', ['bun', str(package / 'src/generate.ts'), str(args.suite.resolve()), str(plan)])
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(ROOT / 'target')))
    runner = args.runner.resolve() if args.runner else target / 'debug/examples/recipes'
    if not runner.is_absolute():
        runner = ROOT / runner
    if not args.no_build:
        result = command('build', ['cargo', 'build', '--locked', '-p', 'plicowarden', '--example', 'recipes', '--message-format=json'], 600)
        artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
        executables = [item['executable'] for item in artifacts if item.get('reason') == 'compiler-artifact' and item.get('target', {}).get('name') == 'recipes' and item.get('executable')]
        if not executables:
            raise SystemExit('Cargo did not report a recipes executable')
        runner = Path(executables[-1])
    if not runner.is_file():
        raise SystemExit(f'Runner missing: {runner}; run without --no-build first')
    argv = [str(runner), '--plan', str(plan), '--output', str(output)]
    if args.semantic:
        argv.append('--semantic')
    (output / 'results.json').unlink(missing_ok=True)
    started = time.perf_counter()
    result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=60)
    times['execute'] = time.perf_counter() - started
    (output / 'runner.log').write_text(result.stdout + result.stderr)
    (output / 'pipeline.json').write_text(json.dumps({'seconds': times, 'runner': str(runner), 'exit_code': result.returncode}, indent=2) + '\n')
    if result.returncode not in [0, 1] or not (output / 'results.json').exists():
        print(result.stdout + result.stderr, file=sys.stderr)
        raise SystemExit(result.returncode or 1)
    report = json.loads((output / 'results.json').read_text())
    for case in report['cases']:
        print(f"{'PASS' if case['passed'] else 'FAIL'} {case['name']} ({case['milliseconds']:.1f} ms)")
        for step in case['steps']:
            if not step['passed']:
                source = step.get('source')
                where = f"{source['file']}:{source['line']}:{source['column']}" if source else 'no source location'
                flow = ' / '.join(step['flow'])
                print(f"  step {step['index']} {where} {flow}\n  {step['error']}")
        if not case['passed'] and not case['steps']:
            print(f"  {case['error']}")
    print(' | '.join(f'{name}: {seconds:.3f}s' for name, seconds in times.items()))
    print(f'Report: {output / "results.json"}')
    raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
