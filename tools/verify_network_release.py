#!/usr/bin/env python3
"""Repeat the optimized real-socket release gate and retain compact evidence."""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repeats', type=int, default=2)
parser.add_argument('--output', default='concepts/networking-evidence/release-hardening.json')
args = parser.parse_args()
if not 1 <= args.repeats <= 5:
    parser.error('--repeats must be 1..5')
command = ['cargo', 'test', '--release', '-p', 'eldiron-server', '--test', 'release_network',
           '--test', 'transport', '--test', 'transport_lifecycle', '--offline', '--', '--nocapture', '--test-threads=1']
report = {'schema': 1, 'generated_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'machine': platform.platform(), 'command': command,
          'scope': 'Actual child server processes and loopback TCP/TLS, deterministic read jitter/fragmentation and connection cuts. Not WAN packet loss, production capacity or a long deployment soak.',
          'runs': []}
output = Path(args.output)
output.parent.mkdir(parents=True, exist_ok=True)
for n in range(args.repeats):
    print(f'Release network gate {n+1}/{args.repeats}', flush=True)
    started = time.monotonic()
    result = subprocess.run(command, capture_output=True, text=True, timeout=600)
    text = result.stdout + result.stderr
    markers = [json.loads(line.split('RELEASE_NETWORK ', 1)[1]) for line in text.splitlines() if 'RELEASE_NETWORK ' in line]
    summaries = [line for line in text.splitlines() if line.startswith('test ') and (' ... ' in line or line.startswith('test result:'))]
    row = {'passed': result.returncode == 0 and len(markers) == 1, 'elapsed_seconds': round(time.monotonic()-started, 3),
           'server_sha256': hashlib.sha256(Path('target/release/eldiron-server').read_bytes()).hexdigest() if Path('target/release/eldiron-server').is_file() else None,
           'project_sha256': hashlib.sha256(Path('test_projects/3DStarter.eldiron').read_bytes()).hexdigest(),
           'scenarios': markers, 'tests': summaries}
    report['runs'].append(row)
    output.write_text(json.dumps(report, indent=2)+'\n')
    if result.returncode or len(markers) != 1:
        print(text[-12000:], flush=True)
        raise SystemExit('Release gate failed; inspect the test output above')
    print(f"  passed in {row['elapsed_seconds']} seconds", flush=True)
print(output, flush=True)
