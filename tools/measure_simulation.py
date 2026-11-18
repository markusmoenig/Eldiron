#!/usr/bin/env python3
"""Sequential simulation profiles, using the real host and synthetic crowds."""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import platform
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', default='target/release/eldiron-server-bench')
parser.add_argument('--scheduling-pairs', action='store_true', help='Compare one extra unoccupied region ticking versus paused')
parser.add_argument('--ticks', type=int, default=200)
parser.add_argument('--label', required=True)
parser.add_argument('--output', required=True)
args = parser.parse_args()
report = {'schema': 1, 'label': args.label,
          'generated_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'binary_sha256': hashlib.sha256(Path(args.binary).read_bytes()).hexdigest(),
          'machine': platform.platform(), 'rows': [],
          'scope': 'In-process real host; moving synthetic crowds, not scripted AI, TLS, rendering or production capacity. No competing builds/tests.'}
for project in ['3DStarter.eldiron', 'Hideout2D.eldiron']:
    rows = [(4, 100), (4, 100)] if args.scheduling_pairs else [(8, 300), (32, 1000)]
    for row_index, (players, entities) in enumerate(rows):
        command = [args.binary, '--project', f'test_projects/{project}', '--players', str(players),
                   '--entities', str(entities), '--ticks', str(args.ticks), '--moving-entities']
        if args.scheduling_pairs:
            command += ['--idle-entities', '1000']
            if row_index == 1:
                command.append('--pause-idle')
        print(f'{project}: {players} players / {entities} actors', flush=True)
        result = subprocess.run(command, capture_output=True, text=True, timeout=300)
        if result.returncode:
            raise RuntimeError(result.stdout[-2000:] + result.stderr[-2000:])
        line = next(l for l in result.stdout.splitlines() if l.startswith('ELDIRON_BENCH '))
        row = json.loads(line.removeprefix('ELDIRON_BENCH '))
        if row['build'] != 'optimized' or row['diagnostics']['players'] != players or row['players_in_crowd_region_at_end'] != players:
            raise RuntimeError('Optimized build and full requested population required')
        if args.scheduling_pairs and row_index == 1 and row['content_revision'] != report['rows'][-1]['content_revision']:
            raise RuntimeError('Scheduling comparison content differs')
        row['command'] = command
        report['rows'].append(row)
        path = Path(args.output)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(report, indent=2) + '\n')
        print(f"  p95={row['server_slice']['p95_us']/1000:.2f} ms max={row['server_slice']['max_us']/1000:.2f} ms", flush=True)
