#!/usr/bin/env python3
"""Bounded local TCP/TLS deployment soak; no external host or packet-loss claim."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import time

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--seconds',type=int,default=600)
parser.add_argument('--output',default='concepts/networking-evidence/deployment-soak.json')
args=parser.parse_args()
if os.name!='posix':parser.error('The deployment drill currently requires Unix SIGTERM and ps')
if not 180<=args.seconds<=86400:parser.error('--seconds must be 180..86400')
output=Path(args.output);output.parent.mkdir(parents=True,exist_ok=True)
log=output.with_suffix('.log')
command=['cargo','test','--release','-p','eldiron-server','--test','deployment_soak','--offline','--','--ignored','--nocapture','--test-threads=1']
environment=os.environ.copy();environment['ELDIRON_DEPLOYMENT_SECONDS']=str(args.seconds)
started=time.monotonic();process=None
fd=os.open(log,os.O_WRONLY|os.O_CREAT|os.O_TRUNC|os.O_NOFOLLOW,0o600)
try:
    with os.fdopen(fd,'w') as stream:
        process=subprocess.Popen(command,stdout=stream,stderr=subprocess.STDOUT,env=environment,start_new_session=True)
        offset=0
        while process.poll() is None:
            if time.monotonic()-started>args.seconds+600:raise TimeoutError('Deployment soak exceeded build/startup/run deadline')
            with log.open() as reader:
                reader.seek(offset)
                for line in reader:
                    if 'DEPLOYMENT_PROGRESS ' in line:print(line.strip(),flush=True)
                offset=reader.tell()
            time.sleep(1)
    text=log.read_text()
    markers=[json.loads(line.split('DEPLOYMENT_SOAK ',1)[1]) for line in text.splitlines() if 'DEPLOYMENT_SOAK ' in line]
    passed=process.returncode==0 and len(markers)==1
    report={'schema':1,'generated_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'machine':platform.platform(),'command':command,'requested_seconds':args.seconds,'elapsed_seconds':round(time.monotonic()-started,3),'passed':passed,'scope':'Local paced, fragmented ordered TCP/TLS with per-world process/restart/backup drills; not WAN packet loss or production capacity.','head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'working_tree_dirty':bool(subprocess.check_output(['git','status','--porcelain'],text=True)),'scenarios':markers}
    files=['servers/dedicated/tests/deployment_soak.rs','servers/dedicated/tests/support/relay.rs','servers/dedicated/tests/support/process.rs','servers/dedicated/tests/support/peer.rs','tools/verify_network_deployment.py','test_projects/3DStarter.eldiron','target/release/eldiron-server','target/release/eldiron-world']
    def digest(path):
        value=hashlib.sha256()
        with Path(path).open('rb') as file:
            while True:
                data=file.read(1024*1024)
                if not data:break
                value.update(data)
        return value.hexdigest()
    report['sha256']={p:digest(p) for p in files if Path(p).is_file()}
    output.write_text(json.dumps(report,indent=2)+'\n')
    if not passed:print(text[-12000:]);raise SystemExit(f'Deployment soak failed; private log: {log}')
    print(f'Passed: {output}',flush=True)
finally:
    if process is not None:
        # A cancelled runner owns its whole disposable process group, including
        # cargo, test binaries and temporary hosts. Do not leave owners behind.
        try:os.killpg(process.pid,signal.SIGTERM)
        except ProcessLookupError:pass
        try:process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            try:os.killpg(process.pid,signal.SIGKILL)
            except ProcessLookupError:pass
            process.wait()
