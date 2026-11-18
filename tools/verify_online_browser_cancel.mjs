// Opt-in real-browser harness lifetime check. Owns a private temporary root and
// one process group; never touches unrelated Chrome profiles or world slots.
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {spawn,execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
const temp=await fs.mkdtemp(path.join(os.tmpdir(),'eldiron-cancel-'));
const child=spawn(process.execPath,['tools/verify_online_browser.mjs'],{detached:true,env:{...process.env,TMPDIR:temp,ELDIRON_WAN:'1',ELDIRON_SOAK_SECONDS:'300',ELDIRON_CONTENT_DELAY_MS:'0'},stdio:['ignore','pipe','pipe']});
let log='',signaled=false,pids=[];
const deadline=setTimeout(()=>{try{process.kill(-child.pid,'SIGKILL');}catch{}},180000);
const observe=b=>{
    log=(log+b).slice(-32000);
    if(!signaled&&log.includes('WASM_PROGRESS ')){
        signaled=true;
        try{pids=execFileSync('pgrep',['-P',String(child.pid)],{encoding:'utf8'}).trim().split(/\s+/).map(Number);}catch{}
        child.kill('SIGTERM');
    }
};
child.stdout.on('data',observe);child.stderr.on('data',observe);
try {
    const code=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
    assert(signaled,log);assert.notEqual(code,0,'canceled soak reported success');assert(pids.length>=2,'missing temporary Chrome/host children');
    for(const pid of pids){let alive=false;try{process.kill(pid,0);alive=true;}catch{}assert(!alive,`temporary child ${pid} survived cancellation`);}
    assert.deepEqual(await fs.readdir(temp),[],'temporary profile/world/certificate directory survived cancellation');
    console.log(JSON.stringify({canceled:true,signal:'SIGTERM',owned_children:pids.length,children_exited:true,temporary_files_removed:true}));
} finally {
    clearTimeout(deadline);try{process.kill(-child.pid,'SIGKILL');}catch{}
    await fs.rm(temp,{recursive:true,force:true});
}
