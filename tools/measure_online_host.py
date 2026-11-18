#!/usr/bin/env python3
"""Repeatable, sequential optimized host samples; no network load claim."""
import argparse
import datetime
import json
import hashlib
from pathlib import Path
import platform
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', default='target/release/eldiron-server-bench')
parser.add_argument('--output', default='concepts/networking-evidence/incremental-storage.json')
parser.add_argument('--storage-pairs-only', action='store_true', help='Compare snapshot-per-action and journaled groups with the same binary/content')
parser.add_argument('--ticks', type=int, default=200)
args = parser.parse_args()
rows = [(1, 0, False, False, 0), (4, 100, False, False, 0),
        (4, 100, True, False, 0), (8, 300, True, False, 0),
        (16, 500, True, False, 0), (32, 1000, True, False, 0),
        (4, 100, True, True, 2)]
report = {'schema': 1, 'generated_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'binary_sha256': hashlib.sha256(Path(args.binary).read_bytes()).hexdigest(),
          'machine': platform.platform(), 'processor': platform.processor(),
          'build_command': 'cargo build --release -p eldiron-server --bin eldiron-server-bench --offline',
          'scope': 'Sequential in-process samples. Synthetic actors are not full AI; excludes TLS, HTTP downloads, rendering, reconnect storms, and client input decoding. File commits use this local filesystem. Not a production capacity guarantee.',
          'rows': []}
try:
    report['hardware'] = subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True, stderr=subprocess.DEVNULL).strip()
except (OSError, subprocess.CalledProcessError):
    report['hardware'] = platform.machine()
output = Path(args.output)
output.parent.mkdir(parents=True, exist_ok=True)
if args.storage_pairs_only:
    rows = [(4, 100, True, True, 2)] * 3
for project in ['3DStarter.eldiron', 'Hideout2D.eldiron']:
    for row_index, (players, entities, moving, persistent, actions) in enumerate(rows):
        command = [str(Path(args.binary).resolve()), '--project', f'test_projects/{project}',
                   '--players', str(players), '--entities', str(entities), '--ticks', str(args.ticks)]
        if moving:
            command.append('--moving-entities')
        if persistent:
            command += ['--persistent', '--saved-actions', str(actions)]
        snapshot = args.storage_pairs_only and row_index == 0
        if snapshot:
            command.append('--snapshot-commits')
        elif args.storage_pairs_only and row_index == 1:
            command.append('--ungrouped-actions')
        print(f'{project}: {players} players, {entities} actors, moving={moving}, saved={persistent}, snapshot_reference={snapshot}', flush=True)
        result = subprocess.run(command, capture_output=True, text=True, timeout=300)
        if result.returncode:
            raise RuntimeError(result.stdout[-2000:] + result.stderr[-2000:])
        line = next(line for line in result.stdout.splitlines() if line.startswith('ELDIRON_BENCH '))
        row = json.loads(line.removeprefix('ELDIRON_BENCH '))
        if row['build'] != 'optimized':
            raise RuntimeError('Use an optimized benchmark binary')
        if row['diagnostics']['players'] != players or row['players_in_crowd_region_at_end'] != players:
            raise RuntimeError('The requested busy-region population was not sustained')
        if args.storage_pairs_only and not snapshot:
            reference = report['rows'][-row_index]
            if reference['content_revision'] != row['content_revision']:
                raise RuntimeError('Storage comparison content differs')
        row['command'] = [args.binary] + command[1:]
        report['rows'].append(row)
        output.write_text(json.dumps(report, indent=2) + '\n')
        print(f"  p95={row['server_slice']['p95_us']/1000:.2f}ms, max={row['server_slice']['max_us']/1000:.2f}ms; budget={row['within_test_budget']}", flush=True)
print(output, flush=True)
