// Full WebGPU Player smoke: a public release is the only game document served.
// Usage: node tools/verify_public_client_browser.mjs WEB_DIR PRIVATE_PROJECT
// The project/package must target ws://127.0.0.1:18843/play (explicit loopback).
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {startIdentityProvider} from './oidc_test_provider.mjs';
const [webArg,project]=process.argv.slice(2);
assert(webArg&&project,'Supply a packaged WebGPU directory and its private multiplayer source');
const web=path.resolve(webArg),temp=await fs.mkdtemp(path.join(os.tmpdir(),'eldiron-public-player-'));
const metrics=path.join(temp,'metrics.json'),requests=[],errors=[];
let chrome,host,session,buffer='',counter=0,hostlog='',chromelog='';
const manifest=JSON.parse(execFileSync('python3',['-c','import zipfile,json,sys;print(zipfile.ZipFile(sys.argv[1]).read("manifest.json").decode())',path.join(web,'game.eldiron-client')],{encoding:'utf8'}));
const regionPaths=new Set((manifest.packs??[]).filter(p=>p.map).map(p=>`/packs/${p.sha256}.eldiron-pack`));
const globalPaths=new Set((manifest.packs??[]).filter(p=>!p.map).map(p=>`/packs/${p.sha256}.eldiron-pack`));
const identityTest=process.env.ELDIRON_IDENTITY_TEST==='1';
const accountTest=process.env.ELDIRON_ACCOUNT_TEST==='1'||identityTest;
let identityProvider;
const assetDelay=Number(process.env.ELDIRON_ASSET_DELAY_MS??1500);
let activePacks=0,peakPacks=0,frameGateObserved=false;
const pending=new Map();
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const staticServer=http.createServer(async(req,res)=>{
    const pathname=new URL(req.url,'http://localhost').pathname;
    const relative=decodeURIComponent(pathname).replace(/^\//,'')||'index.html';
    requests.push(pathname);
    if(relative.includes('..')||relative.endsWith('.eldiron')){res.writeHead(404);res.end();return;}
    const file=path.resolve(web,relative);
    if(!file.startsWith(web+path.sep)){res.writeHead(404);res.end();return;}
    try {const data=await fs.readFile(file);
        if(pathname.startsWith('/packs/')){
            activePacks++;peakPacks=Math.max(peakPacks,activePacks);let settled=false;const done=()=>{if(!settled){settled=true;activePacks--;}};res.once('finish',done);res.once('close',done);
            res.setHeader('Cache-Control','public,max-age=31536000,immutable');
            if(regionPaths.has(pathname))await sleep(assetDelay);
        }
        res.setHeader('Content-Type',file.endsWith('.wasm')?'application/wasm':file.endsWith('.js')?'text/javascript':file.endsWith('.html')?'text/html':'application/octet-stream');res.end(data);}
    catch {res.writeHead(404);res.end();}
});
function call(method,params={},sessionId){return new Promise((resolve,reject)=>{
    const id=++counter,timer=setTimeout(()=>{pending.delete(id);reject(Error('CDP timeout: '+method));},45000);
    pending.set(id,{resolve,reject,timer});chrome.stdio[3].write(JSON.stringify({id,method,params,sessionId})+'\0');
});}
async function evaluate(expression){const r=await call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true},session);assert(!r.exceptionDetails,JSON.stringify(r.exceptionDetails));return r.result.value;}
async function until(check,label,ms=180000){const end=Date.now()+ms;while(Date.now()<end){const result=await check();if(result)return result;await sleep(500);}const state=session?await evaluate('({...document.documentElement.dataset})'):null;throw Error(label+': '+JSON.stringify(state)+' '+hostlog.slice(-1200)+' '+chromelog.slice(-1200));}
try {
    await fs.access(path.join(web,'game.eldiron-client'));
    await new Promise((resolve,reject)=>{staticServer.once('error',reject);staticServer.listen(0,'127.0.0.1',resolve);});
    const origin=`http://127.0.0.1:${staticServer.address().port}`;
    if(identityTest)identityProvider=await startIdentityProvider(temp);
    host=spawn(process.env.ELDIRON_SERVER??'target/debug/eldiron-server',['--project',project,...(accountTest?['--state',path.join(temp,'private.save')]:['--demo']),...(identityProvider?['--identity-config',identityProvider.config]:[]),'--insecure-loopback','--bind','127.0.0.1:18843','--origin',origin,'--metrics-file',metrics],{stdio:['ignore','pipe','pipe']});
    host.stdout.on('data',b=>hostlog=(hostlog+b).slice(-64000));host.stderr.on('data',b=>hostlog=(hostlog+b).slice(-64000));
    await until(()=>{assert(host.exitCode===null,hostlog);return hostlog.includes('host listening on');},'Host startup');
    chrome=spawn(process.env.ELDIRON_CHROME??'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless','--enable-unsafe-webgpu','--use-angle=metal','--disable-background-timer-throttling','--no-first-run','--no-default-browser-check','--remote-debugging-pipe',`--user-data-dir=${path.join(temp,'profile')}`,'about:blank'],{stdio:['ignore','ignore','pipe','pipe','pipe']});
    chrome.stderr.on('data',b=>chromelog=(chromelog+b).slice(-64000));
    chrome.stdio[4].on('data',chunk=>{buffer+=chunk;let at;while((at=buffer.indexOf('\0'))>=0){const text=buffer.slice(0,at);buffer=buffer.slice(at+1);if(!text)continue;const msg=JSON.parse(text),p=pending.get(msg.id);if(p){clearTimeout(p.timer);pending.delete(msg.id);msg.error?p.reject(Error(JSON.stringify(msg.error))):p.resolve(msg.result);}else if(msg.method==='Runtime.exceptionThrown')errors.push(msg.params.exceptionDetails);}});
    const {targetId}=await call('Target.createTarget',{url:'about:blank'});
    ({sessionId:session}=await call('Target.attachToTarget',{targetId,flatten:true}));
    await call('Runtime.enable',{},session);
    await call('Emulation.setDeviceMetricsOverride',{width:960,height:540,deviceScaleFactor:1,mobile:false},session);
    await call('Page.navigate',{url:origin+'/index.html?project=game.eldiron-client'},session);
    const initial=await until(async()=>{const d=await evaluate('({...document.documentElement.dataset})');assert(d.eldironState!=='error',JSON.stringify(d));return d.eldironState==='playing'?d:false;},'Public package GPU startup');
    if(manifest.version===2){assert(!requests.some(p=>regionPaths.has(p)),'region art downloaded before entering the world');assert([...globalPaths].every(p=>requests.includes(p)),'required global artwork missing');assert(peakPacks<=4,'global fetch concurrency exceeded');}
    async function flushInput(){await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');}
    async function click(x,y){for(const type of ['mouseMoved','mousePressed','mouseReleased'])await call('Input.dispatchMouseEvent',{type,x,y,...(type==='mouseMoved'?{}:{button:'left',clickCount:1})},session);await flushInput();}
    async function text(x,y,value){await click(x,y);assert(await evaluate('document.activeElement?.tagName==="TEXTAREA"&&!document.activeElement.disabled'),'GPU field did not focus its browser text service');await call('Input.insertText',{text:value},session);await flushInput();}
    async function state(){return evaluate('({...document.documentElement.dataset})');}
    if(accountTest){
        await until(async()=>{const d=await state();assert(!d.eldironOnlineStatus?.includes('differ'),JSON.stringify(d));return d.eldironOnlineStatus?.includes('Enter username');},'Account server handshake',30000);
        console.error('Account test: registering');
        await text(480,153,'browser_tester');await text(480,209,'original browser password');await click(480,344);
        await until(async()=>(await state()).eldironOnlineAccount==='true','Account registration',30000);
        console.error('Account test: changing password');
        await click(610,504);
        await text(480,153,'original browser password');await text(480,209,'replacement browser password');await text(480,265,'replacement browser password');await click(480,304);
        await until(async()=>{const d=await state();return d.eldironOnlineAccount==='false'&&d.eldironOnlineStatus?.includes('Password changed');},'Password change',30000);
        console.error('Account test: logging in again');
        await click(370,504);
        await until(async()=>(await state()).eldironOnlineStatus?.includes('Enter username'),'Reconnect after password change');
        await text(480,209,'replacement browser password');await click(480,304);
        await until(async()=>(await state()).eldironOnlineAccount==='true','Login with changed password');
    }
    // The built-in lobby uses a 960x540 responsive layout; Play is centered here.
    await sleep(2000);
    await call('Input.dispatchMouseEvent',{type:'mouseMoved',x:480,y:384},session);
    await call('Input.dispatchMouseEvent',{type:'mousePressed',x:480,y:384,button:'left',clickCount:1},session);
    await call('Input.dispatchMouseEvent',{type:'mouseReleased',x:480,y:384,button:'left',clickCount:1},session);
    if(manifest.version===2){await until(()=>requests.some(p=>regionPaths.has(p)),'Required region request',10000);const d=await evaluate('({...document.documentElement.dataset})');assert.equal(Number(d.eldironTicks),0,'frame escaped before delayed artwork');frameGateObserved=true;}
    let joined=await until(async()=>{let m;try{m=JSON.parse(await fs.readFile(metrics,'utf8'));}catch{return false;}return m.players===1?m:false;},'World admission');
    await until(async()=>Number((await evaluate('({...document.documentElement.dataset})')).eldironTicks)>0,'Remote frame presentation');
    if(identityTest){
        async function account(){
            await call('Input.dispatchKeyEvent',{type:'keyDown',key:'F10',code:'F10',windowsVirtualKeyCode:121,nativeVirtualKeyCode:121},session);
            await call('Input.dispatchKeyEvent',{type:'keyUp',key:'F10',code:'F10',windowsVirtualKeyCode:121,nativeVirtualKeyCode:121},session);await flushInput();
        }
        async function approve(){
            const {targetId:providerTarget}=await call('Target.createTarget',{url:identityProvider.state.issuer+'/verify'});
            const {sessionId:providerSession}=await call('Target.attachToTarget',{targetId:providerTarget,flatten:true});
            await until(async()=>{const r=await call('Runtime.evaluate',{expression:'!!document.getElementById("approve")',returnByValue:true},providerSession);return r.result.value;},'Provider authorization form',10000);
            await call('Runtime.evaluate',{expression:'document.getElementById("approve").click()'},providerSession);
            await until(()=>identityProvider.state.approved,'Provider approval',10000);
            await call('Target.closeTarget',{targetId:providerTarget});await call('Target.activateTarget',{targetId});
        }
        console.error('Identity test: linking existing saved character');
        await account();await click(480,464);await text(480,279,'replacement browser password');await click(370,220);
        await until(async()=>(await state()).eldironOnlineStatus?.includes('Visit the link'),'Provider link challenge',30000);await approve();
        await until(async()=>{const d=await state();return d.eldironOnlineAccount==='false'&&d.eldironOnlineStatus?.includes('Login method linked');},'Durable provider linking',30000);
        await click(370,504);await until(async()=>(await state()).eldironOnlineStatus?.includes('Enter username'),'Reconnect for provider login',30000);
        await click(610,464);await click(370,220);
        await until(async()=>(await state()).eldironOnlineStatus?.includes('Visit the link'),'Provider login challenge',30000);
        await click(480,492);const requestsAtCancel=identityProvider.state.tokenRequests;await sleep(2200);
        assert.equal(identityProvider.state.tokenRequests,requestsAtCancel,'Cancelled provider flow kept polling');
        assert.equal((await state()).eldironOnlineAccount,'false');
        console.error('Identity test: provider login after cancellation');
        await click(610,464);await click(370,220);
        await until(async()=>(await state()).eldironOnlineStatus?.includes('Visit the link'),'New provider login challenge',30000);await approve();
        await until(async()=>(await state()).eldironOnlineAccount==='true','Provider login',30000);
        await click(480,384);
        joined=await until(async()=>{const m=JSON.parse(await fs.readFile(metrics,'utf8'));return m.players===1?m:false;},'Rejoin saved body after provider login',30000);
        assert.equal(joined.entities,1,'Provider login created a duplicate body/account');
        await until(async()=>Number((await state()).eldironTicks)>0,'Provider-authenticated remote frame',30000);
        console.error('Identity test: unlinking provider and returning to password login');
        await account();await click(480,464);await text(480,279,'replacement browser password');await click(610,220);
        await until(async()=>{const d=await state();return d.eldironOnlineAccount==='false'&&d.eldironOnlineStatus?.includes('Login method removed');},'Durable provider unlink',30000);
        await click(370,504);await until(async()=>(await state()).eldironOnlineStatus?.includes('Enter username'),'Reconnect after unlink',30000);
        await text(480,209,'replacement browser password');await click(480,304);
        await until(async()=>(await state()).eldironOnlineAccount==='true','Local password retained after unlink',30000);await click(480,384);
        joined=await until(async()=>{const m=JSON.parse(await fs.readFile(metrics,'utf8'));return m.players===1?m:false;},'Rejoin after unlink',30000);
        assert.equal(joined.entities,1);await until(async()=>Number((await state()).eldironTicks)>0,'Remote frame after unlink',30000);
        assert.equal(identityProvider.state.errors.length,0,JSON.stringify(identityProvider.state.errors));
    }
    if(accountTest){
        await call('Input.dispatchKeyEvent',{type:'keyDown',key:'F10',code:'F10',windowsVirtualKeyCode:121,nativeVirtualKeyCode:121},session);
        await call('Input.dispatchKeyEvent',{type:'keyUp',key:'F10',code:'F10',windowsVirtualKeyCode:121,nativeVirtualKeyCode:121},session);
        await click(480,344);
        await until(async()=>{const d=await state();return d.eldironOnlineAccount==='false'&&d.eldironOnlineStatus?.includes('All sessions');},'Log out everywhere');
    }
    const before=Number(await evaluate('document.documentElement.dataset.eldironPresentations'));
    await sleep(1500);
    const final=await evaluate('({...document.documentElement.dataset})');
    assert(final.eldironState==='playing',JSON.stringify(final));assert(Number(final.eldironPresentations)>before);
    assert.equal(Number(final.eldironReadbackBytes),0);assert.equal(errors.length,0,JSON.stringify(errors));
    assert(requests.includes('/game.eldiron-client'));assert(!requests.some(p=>p.endsWith('.eldiron')));
    assert(joined.transport.active===1,JSON.stringify(joined.transport));
    if(identityTest)joined.identity={linked_existing_character:true,provider_login:true,cancellation_stops_polling:true,unlinked:true,password_retained:true,duplicate_characters:0};
    if(manifest.version===2&&regionPaths.size>1)assert.equal(new Set(requests.filter(p=>regionPaths.has(p))).size,1,'unvisited region artwork downloaded');
    console.log(JSON.stringify({identity:joined.identity??null,account_management:accountTest?{registration:true,password_changed:true,new_password_login:true,global_logout:true}:null,streaming:{version:manifest.version,global_packs:globalPaths.size,regional_packs:regionPaths.size,requested_regions:[...new Set(requests.filter(p=>regionPaths.has(p)))].length,peak_fetches:peakPacks,frame_gate_observed:frameGateObserved},public_package:true,transport:'explicit loopback WS',game_documents_served:[...new Set(requests.filter(p=>p.endsWith('.eldiron-client')))],gpu:final,host:{mode:joined.mode,players:joined.players,entities:joined.entities},transport_metrics:joined.transport,exceptions:errors.length,initial_gpu_state:initial.eldironState}));
} finally {
    for(const p of pending.values()){clearTimeout(p.timer);p.reject(Error('Test closed'));}pending.clear();
    if(chrome){chrome.kill('SIGTERM');await new Promise(r=>chrome.exitCode!==null?r():chrome.once('exit',r));}
    if(host){host.kill('SIGTERM');await Promise.race([new Promise(r=>host.exitCode!==null?r():host.once('exit',r)),sleep(10000)]);if(host.exitCode===null)host.kill('SIGKILL');}
    await new Promise(r=>staticServer.close(r));if(identityProvider)await identityProvider.close();await fs.rm(temp,{recursive:true,force:true});
}
