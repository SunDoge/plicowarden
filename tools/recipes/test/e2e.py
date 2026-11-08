#!/usr/bin/env python3
"""Check generated plans against the prebuilt Rust runner; no network or real vault."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
TARGET = Path(os.environ.get('CARGO_TARGET_DIR', str(ROOT / 'target')))
if not TARGET.is_absolute():
    TARGET = ROOT / TARGET
BINARY = TARGET / 'debug/examples/recipes'


def run(plan, output, semantic=True, expected=0):
    args = [str(BINARY), '--plan', str(plan), '--output', str(output)]
    if semantic:
        args.append('--semantic')
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, timeout=60)
    assert result.returncode == expected, result.stdout + result.stderr
    return json.loads((output / 'results.json').read_text())


def main():
    reports = ROOT / 'reports'
    reports.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='recipe-e2e-', dir=reports) as temporary:
        directory = Path(temporary)
        plan_path = directory / 'plan.json'
        subprocess.run(['bun', 'tools/recipes/src/generate.ts', 'recipes/suite.ts', str(plan_path)], cwd=ROOT, check=True, timeout=60)
        original = json.loads(plan_path.read_text())
        success = run(plan_path, directory / 'success')
        assert len(success['cases']) == 8 and all(c['passed'] for c in success['cases'])
        # Fail inside a named flow, then verify a clean later case still passes.
        changed = json.loads(plan_path.read_text())
        case = changed['cases'][0]
        failed_index = next(i for i, step in enumerate(case['steps']) if step['flow'] == ['search-vault'])
        case['steps'][failed_index]['operation']['label'] = 'missing-search'
        changed['cases'] = [case, changed['cases'][2]]
        bad_path = directory / 'bad.json'
        bad_path.write_text(json.dumps(changed))
        failed = run(bad_path, directory / 'failed', semantic=False, expected=1)
        first, second = failed['cases']
        assert not first['passed'] and second['passed']
        failure = first['steps'][-1]
        assert failure['index'] == failed_index + 1 and failure['flow'] == ['search-vault']
        assert failure['source']['file'].endswith('flows/search.ts')
        assert failure['source']['line'] > 0
        assert 'missing-search' in failure['error']
        assert first['evidence'] and first['rendered_text']
        assert (directory / 'failed' / first['failure_screenshot']).is_file()
        # Unknown service payloads fail; there is no arbitrary app field mutation.
        changed = json.loads(json.dumps(original))
        changed['cases'] = [changed['cases'][0]]
        changed['cases'][0]['steps'] = [{'flow': [], 'operation': {'op': 'event', 'event': {'kind': 'arbitrary'}}}]
        bad_path.write_text(json.dumps(changed))
        rejected = run(bad_path, directory / 'event-rejected', expected=1)
        assert 'unknown variant' in rejected['cases'][0]['error']
        # Unknown versions, fields and unsafe artifact names fail before running cases.
        for change in ['version', 'unknown', 'path']:
            value = json.loads(json.dumps(original))
            if change == 'version':
                value['version'] = 2
            elif change == 'unknown':
                value['cases'][0]['typo'] = True
            else:
                value['cases'][0]['name'] = '../escape'
            bad_path.write_text(json.dumps(value))
            result = subprocess.run([str(BINARY), '--plan', str(bad_path), '--output', str(directory / change)], cwd=ROOT, capture_output=True, timeout=60)
            assert result.returncode != 0
            assert not (directory / change).exists()
        assert all(not Path(step['source']['file']).is_absolute() for c in original['cases'] for step in c['steps'] if step.get('source'))
    print('TS → JSON → Rust: all cases, source mapping, failure PNG, continuation, event validation and schema rejection passed.')


if __name__ == '__main__':
    main()
