// Exercise the actual IndexedDB adapter in a disposable browser profile.
// Run: node tools/verify_world_store_browser.mjs (ELDIRON_CHROME overrides binary).
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import {spawn} from 'node:child_process';
import assert from 'node:assert/strict';
const source=await fs.readFile(new URL('../crates/runtime/src/persistence/indexed_db.js',import.meta.url));
const profile=await fs.mkdtemp(path.join(os.tmpdir(),'eldiron-world-store-'));
const server=http.createServer((req,res)=>{
    res.setHeader('content-type',req.url==='/store.js'?'text/javascript':'text/html');
    res.end(req.url==='/store.js'?source:'<!doctype html><title>World store test</title>');
});
try { await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);}); }
catch(error) { await fs.rm(profile,{recursive:true,force:true}); throw error; }
const origin=`http://127.0.0.1:${server.address().port}`;
const chrome=spawn(process.env.ELDIRON_CHROME??'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[
    '--headless','--disable-gpu','--no-first-run','--no-default-browser-check',
    '--remote-debugging-pipe',`--user-data-dir=${profile}`,'about:blank',
],{stdio:['ignore','ignore','pipe','pipe','pipe']});
let id=0,buffer='';const pending=new Map();let stderr='';
chrome.stderr.on('data',chunk=>stderr+=chunk);
chrome.stdio[4].on('data',chunk=>{
    buffer+=chunk.toString();let end;
    while((end=buffer.indexOf('\0'))>=0){
        const message=JSON.parse(buffer.slice(0,end));buffer=buffer.slice(end+1);
        if(message.id&&pending.has(message.id)){
            const {resolve,reject,timer}=pending.get(message.id);pending.delete(message.id);clearTimeout(timer);
            message.error?reject(Error(JSON.stringify(message.error))):resolve(message.result);
        }
    }
});
chrome.on('error',error=>{for(const {reject} of pending.values())reject(error);});
function call(method,params={},sessionId){return new Promise((resolve,reject)=>{
    const next=++id;const timer=setTimeout(()=>reject(Error(`${method} timed out: ${stderr.slice(-1000)}`)),20000);
    pending.set(next,{resolve,reject,timer});chrome.stdio[3].write(JSON.stringify({id:next,method,params,sessionId})+'\0');
});}
try {
    const {targetId}=await call('Target.createTarget',{url:origin});
    const {sessionId}=await call('Target.attachToTarget',{targetId,flatten:true});
    await call('Page.enable',{},sessionId);
    await call('Page.navigate',{url:origin},sessionId);
    const result=await call('Runtime.evaluate',{
        expression:`(async()=>{
            const {loadWorld,commitWorld}=await import(${JSON.stringify(origin+'/store.js')});
            const database='durable-test';const bytes=new Uint8Array([1,2,3,4]);
            if(await loadWorld(database,'solo')!==null)throw Error('new slot not empty');
            await commitWorld(database,'solo','','1','world:content:solo',bytes);
            // Competing tabs/owners use one read-write transaction per CAS.
            const races=await Promise.allSettled([
                commitWorld(database,'solo','1','2','world:content:solo',new Uint8Array([5])),
                commitWorld(database,'solo','1','2','world:content:solo',new Uint8Array([6])),
            ]);
            if(races.filter(r=>r.status==='fulfilled').length!==1)throw Error('CAS did not choose one winner');
            const loaded=await loadWorld(database,'solo');
            if(loaded.length!==1 || ![5,6].includes(loaded[0]))throw Error('torn checkpoint');
            for(const [expected,identity] of [['1','world:content:solo'],['2','other-world']]){
                let rejected=false;
                try{await commitWorld(database,'solo',expected,'3',identity,bytes);}catch(e){rejected=e==='save-conflict';}
                if(!rejected)throw Error('stale/foreign identity was accepted');
            }
            await commitWorld(database,'creator-test','','1','world:content:test',bytes);
            if((await loadWorld(database,'solo'))[0]!==loaded[0])throw Error('test slot overwrote solo');
            if((await loadWorld(database,'creator-test')).join(',')!=='1,2,3,4')throw Error('reopen lost data');
            return {cas:'one winner',rollback:'old head retained',slots:'isolated',reopen:'preserved'};
        })()`,awaitPromise:true,returnByValue:true,
    },sessionId);
    assert(!result.exceptionDetails,JSON.stringify(result.exceptionDetails));
    console.log(JSON.stringify(result.result.value));
} finally {
    for(const {timer} of pending.values())clearTimeout(timer);
    chrome.kill('SIGTERM');server.close();
    await new Promise(resolve=>chrome.once('exit',resolve));
    await fs.rm(profile,{recursive:true,force:true});
}
