#!/usr/bin/env python3
"""Compare one compiled Warden binary across isolated/batched recipe strategies.

Run from any directory. Uses synthetic fixtures only. Outputs stay under reports/.
"""
import argparse
import os
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import random
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rounds', type=int, default=5)
    parser.add_argument('--output', type=Path, default=ROOT / 'reports/recipe-experiment')
    args = parser.parse_args()
    if args.rounds < 1:
        parser.error('--rounds must be positive')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(ROOT / 'target')))
    if not target.is_absolute():
        target = ROOT / target
    binary = target / 'debug/examples/recipe_probe'
    start = time.perf_counter()
    build = subprocess.run(['cargo', 'build', '--locked', '--offline', '-p', 'plicowarden', '--example', 'recipe_probe'], cwd=ROOT, capture_output=True, text=True, timeout=600)
    (out / 'build.log').write_text(build.stdout + build.stderr)
    build.check_returncode()
    build_seconds = time.perf_counter() - start
    cases = json.loads((ROOT / 'scenarios/recipes.json').read_text())
    for case in cases:
        case['scenario'] = str(ROOT / case['scenario'])
    manifests = out / 'manifests'
    manifests.mkdir(exist_ok=True)

    def manifest(name, entries):
        path = manifests / f'{name}.json'
        path.write_text(json.dumps(entries))
        return path

    all_cases = manifest('all', cases)
    single_cases = [manifest(c['name'], [c]) for c in cases]

    def invoke(path, mode, destination, expected=0):
        result = subprocess.run([str(binary), '--manifest', str(path), '--mode', mode, '--output', str(destination)], cwd=ROOT, capture_output=True, text=True, timeout=60)
        if result.returncode != expected:
            raise RuntimeError(f'{mode}: exit {result.returncode}: {result.stdout}\n{result.stderr}')
        return json.loads(result.stdout)

    methods = ['process-sequential', 'process-parallel-2', 'batch-fresh', 'batch-shared', 'batch-semantic']

    def run(method, destination):
        start = time.perf_counter()
        if method == 'process-sequential':
            results = [r for path in single_cases for r in invoke(path, 'fresh', destination)]
        elif method == 'process-parallel-2':
            with ThreadPoolExecutor(max_workers=2) as pool:
                groups = list(pool.map(lambda path: invoke(path, 'fresh', destination), single_cases))
            results = [r for group in groups for r in group]
        else:
            results = invoke(all_cases, method.removeprefix('batch-'), destination)
        elapsed = time.perf_counter() - start
        assert len(results) == len(cases) and all(r['passed'] for r in results)
        return {'method': method, 'seconds': elapsed, 'cases': results}

    # Warm up every strategy separately; startup inside each timed invocation remains included.
    for method in methods:
        run(method, out / 'warmup' / method)
    records = []
    rng = random.Random(20261108)
    reference = None
    for round_index in range(args.rounds):
        order = methods.copy()
        rng.shuffle(order)
        for method in order:
            destination = out / f'round-{round_index}' / method
            result = run(method, destination)
            result['round'] = round_index
            records.append(result)
            if method != 'batch-semantic':
                hashes = {c['name']: hashlib.sha256((destination / f"{c['name']}.png").read_bytes()).hexdigest() for c in cases}
                if reference is None:
                    reference = hashes
                assert hashes == reference, f'PNG mismatch in {method}, round {round_index}'
            print(f"{round_index + 1}/{args.rounds} {method}: {result['seconds']:.3f}s", flush=True)

    # A shared renderer must not make recipes order-dependent.
    reverse = invoke(manifest('reverse', list(reversed(cases))), 'shared', out / 'reverse')
    assert all(r['passed'] for r in reverse)
    assert {c['name']: hashlib.sha256((out / 'reverse' / f"{c['name']}.png").read_bytes()).hexdigest() for c in cases} == reference
    # A failed command/assertion must be named, exit nonzero, and allow later cases to run.
    bad_scenario = out / 'bad-scenario.json'
    bad_scenario.write_text(json.dumps({'steps': [{'command': {'action': 'click', 'role': 'button', 'label': 'does-not-exist'}}]}))
    bad_command = {'name': 'bad-command', 'scenario': str(bad_scenario)}
    bad_assertion = dict(cases[0], name='bad-assertion', present=['does-not-exist'])
    failure = invoke(manifest('failures', [bad_command, bad_assertion, cases[0]]), 'shared', out / 'failures', expected=1)
    assert [r['passed'] for r in failure] == [False, False, True]
    assert 'does-not-exist' in failure[0]['error'] and 'does-not-exist' in failure[1]['error']
    assert hashlib.sha256((out / 'failures/vault.png').read_bytes()).hexdigest() == reference['vault']
    summary = {
        'build_seconds': build_seconds,
        'note': 'Build invocation is reported separately; it may be a no-op, not a cold build.',
        'recipe_count': len(cases),
        'rounds': args.rounds,
        'medians_seconds': {m: statistics.median(r['seconds'] for r in records if r['method'] == m) for m in methods},
        'all_pngs_identical': True,
        'reverse_order_identical': True,
        'failure_continues_and_exits_nonzero': True,
        'records': records,
        'failure_results': failure,
    }
    (out / 'results.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({k: v for k, v in summary.items() if k not in ['records', 'failure_results']}, indent=2))


if __name__ == '__main__':
    main()
