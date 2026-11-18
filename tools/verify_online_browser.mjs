// Actual WASM WebSocket + HTTPS content test in an isolated Chromium profile.
// Build eldiron-server and eldiron-network (wasm32), then run matching wasm-bindgen
// --target web --out-dir /tmp/eldiron-online-web/pkg. ELDIRON_WEB overrides path.
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import {spawn,execFileSync} from 'node:child_process';
import {createHash,randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
import {networkRelay} from './network_relay.mjs';
const temp=await fs.mkdtemp(path.join(os.tmpdir(),'eldiron-online-'));
const web=process.env.ELDIRON_WEB??'/tmp/eldiron-online-web';
const contentDelay=Number(process.env.ELDIRON_CONTENT_DELAY_MS??3000);
const wan=process.env.ELDIRON_WAN==='1';
const walking=wan&&process.env.ELDIRON_WALK!=='0';
const relays=[];
const soakSeconds=Number(process.env.ELDIRON_SOAK_SECONDS??(wan?300:60));
assert(Number.isFinite(contentDelay)&&contentDelay>=0&&contentDelay<=4000);
assert(Number.isFinite(soakSeconds)&&soakSeconds>=0&&soakSeconds<=86400);
const contentRequests=new Set(), delayedTimers=new Set();
let delayedRequests=0, canceledRequests=0;
const profile=path.join(temp,'profile');let chrome,host;let hostlog='',stderr='';
const staticServer=http.createServer(async(req,res)=>{
    try {const file=req.url==='/pkg/eldiron_network.js'?'eldiron_network.js':req.url==='/pkg/eldiron_network_bg.wasm'?'eldiron_network_bg.wasm':null;
        res.setHeader('Content-Type',file?.endsWith('.wasm')?'application/wasm':file?'text/javascript':'text/html');
        res.end(file?await fs.readFile(path.join(web,'pkg',file)):'<!doctype html><title>Online WASM test</title>');
    }catch(e){res.writeHead(500);res.end(String(e));}
});
let id=0,buffer='',interrupted=false;const pending=new Map();
const interrupt=()=>{interrupted=true;for(const p of pending.values()){clearTimeout(p.timer);p.reject(Error('Browser soak interrupted'));}pending.clear();chrome?.kill('SIGTERM');host?.kill('SIGTERM');};
process.once('SIGINT',interrupt);process.once('SIGTERM',interrupt);
async function stopChild(child,signal){
    if(!child||child.exitCode!==null||child.signalCode!==null)return;
    const exited=new Promise(r=>child.once('exit',r));
    const timer=setTimeout(()=>child.kill('SIGKILL'),10000);timer.unref();
    try{child.kill(signal);await exited;}finally{clearTimeout(timer);}
}

function call(method,params={},sessionId){if(interrupted)return Promise.reject(Error('Browser soak interrupted'));return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(request);reject(Error(`CDP timeout: ${stderr.slice(-500)} ${hostlog.slice(-500)}`));},45000);const request=++id;pending.set(request,{resolve,reject,timer});chrome.stdio[3].write(JSON.stringify({id:request,method,params,sessionId})+'\0');});}
async function evaluate(session,expression){const result=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true},session);assert(!result.exceptionDetails,JSON.stringify(result.exceptionDetails));return result.result.value;}
try {
    await new Promise((resolve,reject)=>{staticServer.once('error',reject);staticServer.listen(0,'127.0.0.1',resolve);});
    const origin=`http://127.0.0.1:${staticServer.address().port}`;
    const cert=path.join(temp,'cert.pem'),key=path.join(temp,'key.pem');
    execFileSync('openssl',['req','-x509','-newkey','rsa:2048','-sha256','-nodes','-days','1','-subj','/CN=localhost','-addext','subjectAltName=DNS:localhost','-keyout',key,'-out',cert],{stdio:'ignore'});
    const pub=execFileSync('openssl',['x509','-in',cert,'-pubkey','-noout']);
    const der=execFileSync('openssl',['pkey','-pubin','-outform','DER'],{input:pub});const pin=createHash('sha256').update(der).digest('base64');
    const project=JSON.parse(await fs.readFile('test_projects/3DStarter.eldiron','utf8'));
    const preload=structuredClone(project.regions[0]);preload.id=randomUUID();preload.map.id=randomUUID();preload.name='Browser preload';preload.map.name=preload.name;project.regions.push(preload);
    project.config+='\n[multiplayer.spatial]\nprefetch_regions=["Browser preload"]\n';
    const fixture=path.join(temp,'spatial.eldiron');await fs.writeFile(fixture,JSON.stringify(project));
    if(interrupted)throw Error('Browser soak interrupted');
    host=spawn(process.env.ELDIRON_SERVER??'target/debug/eldiron-server',['--project',fixture,'--demo','--bind','127.0.0.1:0','--tls-cert',cert,'--tls-key',key,'--origin',origin],{stdio:['ignore','pipe','pipe']});
    host.stderr.on('data',b=>hostlog=(hostlog+b).slice(-512*1024));host.stdout.on('data',b=>hostlog=(hostlog+b).slice(-512*1024));
    let endpoint=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error(`Server startup timeout: ${hostlog}`)),180000);const check=()=>{const match=hostlog.match(/host listening on 127\.0\.0\.1:(\d+)/);if(match){clearTimeout(timer);resolve(`wss://localhost:${match[1]}/play`);}};host.stdout.on('data',check);host.once('exit',code=>{clearTimeout(timer);reject(Error(`Server exited ${code}: ${hostlog}`));});check();});
    const directEndpoint=endpoint;
    if(wan){const upstream=Number(new URL(endpoint).port);for(let i=0;i<2;i++)relays.push(await networkRelay(upstream));endpoint=`wss://localhost:${relays[0].port}/play`;}
    if(interrupted)throw Error('Browser soak interrupted');
    chrome=spawn(process.env.ELDIRON_CHROME??'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless','--disable-gpu','--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows','--no-first-run','--no-default-browser-check','--remote-debugging-pipe',`--user-data-dir=${profile}`,`--ignore-certificate-errors-spki-list=${pin}`,'about:blank'],{stdio:['ignore','ignore','pipe','pipe','pipe']});
    chrome.stderr.on('data',b=>stderr=(stderr+b).slice(-512*1024));chrome.stdio[4].on('data',chunk=>{buffer+=chunk.toString();let end;while((end=buffer.indexOf('\0'))>=0){const msg=JSON.parse(buffer.slice(0,end));buffer=buffer.slice(end+1);if(msg.method==='Fetch.requestPaused') {
            if(msg.params.networkId)contentRequests.add(msg.params.networkId);
            delayedRequests++;
            const timer=setTimeout(()=>{delayedTimers.delete(timer);call('Fetch.continueRequest',{requestId:msg.params.requestId},msg.sessionId).catch(()=>{});},contentDelay);
            delayedTimers.add(timer);
        }
        if(msg.method==='Network.loadingFailed'&&contentRequests.has(msg.params.requestId)&&msg.params.canceled)canceledRequests++;
        if(msg.id&&pending.has(msg.id)){const p=pending.get(msg.id);pending.delete(msg.id);clearTimeout(p.timer);msg.error?p.reject(Error(JSON.stringify(msg.error))):p.resolve(msg.result);}}});
    const browserVersion=await call('Browser.getVersion');
    const sessions=[];
    for(const name of ['Alice','Bob']) {
        const clientEndpoint=wan?`wss://localhost:${relays[sessions.length].port}/play`:directEndpoint;
        const {targetId}=await call('Target.createTarget',{url:origin});const {sessionId}=await call('Target.attachToTarget',{targetId,flatten:true});sessions.push(sessionId);
        await call('Page.enable',{},sessionId);await call('Network.enable',{},sessionId);await call('Fetch.enable',{patterns:[{urlPattern:'*/content/*',requestStage:'Request'}]},sessionId);await call('Page.navigate',{url:origin},sessionId);
        await evaluate(sessionId,`(async()=>{const mod=await import(${JSON.stringify(origin+'/pkg/eldiron_network.js')});window.wasm=await mod.default();window.mod=mod;window.client=new mod.DemoClient(${JSON.stringify(clientEndpoint)},${JSON.stringify(name)},false);window.error=null;window.busy=false;window.last=performance.now();window.cached={actors:[]};window.pumps=0;window.step=async()=>{if(window.busy)return;window.busy=true;const now=performance.now();try{const count=window.fast?4:1;for(let i=0;i<count;i++){await client.pump((now-last)/count);window.pumps++;}if(!window.fast||window.pumps%20===0)window.cached=JSON.parse(client.state());}catch(e){window.error=String(e);}finally{window.last=now;window.busy=false;}};window.startPump=ms=>{clearInterval(window.timer);window.timer=setInterval(window.step,ms);};startPump(20);return true;})()`);
    }
    // Drop a client while required fetch is suspended. Its AbortController must
    // cancel the browser request and socket; the replacement gets one new body.
    if(contentDelay>=2000) {
        await evaluate(sessions[0],`(async()=>{const deadline=performance.now()+10000;while(performance.now()<deadline){if(window.error)throw Error(window.error);if(window.cached.content_delivery?.downloading)break;await new Promise(r=>setTimeout(r,20));}if(!window.cached.content_delivery?.downloading)throw Error('Missing pending content');clearInterval(timer);while(busy)await new Promise(r=>setTimeout(r,5));client.free();window.client=new window.mod.DemoClient(${JSON.stringify(endpoint)},'Alice',false);window.cached={actors:[]};window.last=performance.now();startPump(20);return true;})()`);
    }
    const contentQueues=[];
    for(const session of sessions) {
        const result=await evaluate(session,`(async()=>{const deadline=performance.now()+20000;while(performance.now()<deadline){if(window.error)throw Error(window.error);const state=window.cached;if(state.actors.filter(a=>['Alice','Bob'].includes(a.name)).length===2)return state;await new Promise(r=>setTimeout(r,50));}throw Error('Missing shared actors '+JSON.stringify(window.cached));})()`);
        assert.equal(result.actors.filter(a=>a.owned).length,1);assert.equal(result.status,'Playing demo');
        const d=result.content_delivery;contentQueues.push(d);
        assert(d.peak_packets<=256&&d.peak_bytes<=4*1024*1024,JSON.stringify(d));
        if(contentDelay>=2000)assert(d.peak_packets>64,JSON.stringify(d));
    }
    for(const session of sessions) {
        const cached=await evaluate(session,`(async()=>{const deadline=performance.now()+20000;while(performance.now()<deadline){if(window.error)throw Error(window.error);if(window.cached.cached_regions>=2)return window.cached;await new Promise(r=>setTimeout(r,50));}throw Error('Missing content preload '+JSON.stringify(window.cached));})()`);
        assert(cached.cached_bytes<=16*1024*1024);assert(cached.cached_regions<=4);
    }
    // Requests use the Rust sequencer and codec inside WASM, including denial.
    for(const [request,expected] of [[{Intent:'look'},{Outcome:'Processed'}],[{Intent:'admin.grant'},{Outcome:{Denied:'Unsupported'}}]]) {
        const result=await evaluate(sessions[0],`(async()=>{while(busy)await new Promise(r=>setTimeout(r,5));client.request(${JSON.stringify(JSON.stringify(request))});window.cached=JSON.parse(client.state());const deadline=performance.now()+10000;while(performance.now()<deadline){if(window.error)throw Error(window.error);if(window.cached.action)return window.cached.action;await new Promise(r=>setTimeout(r,25));}throw Error('Missing action reply');})()`);
        assert.deepEqual(result,expected);
    }
    // Drive real fixed inputs and verify both tabs remain synchronized/connected.
    await evaluate(sessions[0],`(async()=>{while(busy)await new Promise(r=>setTimeout(r,5));client.intent('w');await new Promise(r=>setTimeout(r,400));while(busy)await new Promise(r=>setTimeout(r,5));client.intent('x');return true;})()`);
    for(const session of sessions) assert.equal(await evaluate(session,'window.error'),null);
    // Keep the measured tab foreground; hidden-tab timer throttling is not a
    // high-refresh test. Several substeps split real elapsed time, without
    // inventing extra simulation time or running concurrent Rust borrows.
    await call('Page.bringToFront',{},sessions[0]);
    const fast=await evaluate(sessions[0],`(async()=>{clearInterval(timer);while(busy)await new Promise(r=>setTimeout(r,5));const raw=WebSocket.prototype.send;window.sent=0;WebSocket.prototype.send=function(...args){window.sent++;return raw.apply(this,args);};window.pumps=0;window.fast=true;startPump(4);await new Promise(r=>setTimeout(r,3000));clearInterval(timer);while(busy)await new Promise(r=>setTimeout(r,5));window.fast=false;window.cached=JSON.parse(client.state());if(window.error)throw Error(window.error);const result={pumps:window.pumps,sent:window.sent,status:window.cached.status};startPump(20);return result;})()`);
    assert(fast.pumps>360,JSON.stringify(fast));assert(fast.sent<=210,JSON.stringify(fast));assert.equal(fast.status,'Playing demo');
    // Reload Alice alone. Bob must retain its body and see one replacement
    // Alice, without an old guest body left in the public world.
    const before=await evaluate(sessions[0],`window.cached.actors.find(a=>a.owned).id`);
    const bob=await evaluate(sessions[1],`window.cached.actors.find(a=>a.owned).id`);
    await evaluate(sessions[0],`(async()=>{clearInterval(timer);while(busy)await new Promise(r=>setTimeout(r,5));client.free();window.client=new window.mod.DemoClient(${JSON.stringify(endpoint)},'Alice',false);window.cached={actors:[]};window.last=performance.now();window.error=null;startPump(20);return true;})()`);
    const reloaded=[];
    for(const session of sessions){
        reloaded.push(await evaluate(session,`(async()=>{const deadline=performance.now()+20000;while(performance.now()<deadline){if(window.error)throw Error(window.error);if(window.cached.actors.filter(a=>a.name==='Alice').length===1&&window.cached.actors.filter(a=>a.name==='Bob').length===1&&window.cached.actors.filter(a=>a.owned).length===1)return window.cached;await new Promise(r=>setTimeout(r,50));}throw Error('Reload failed '+JSON.stringify(window.cached));})()`));
    }
    assert.notEqual(reloaded[0].actors.find(a=>a.owned).id,before);
    assert.equal(reloaded[1].actors.find(a=>a.owned).id,bob);
    // Use real elapsed time, and periodically exercise a sequenced request
    // during a bounded soak. No synthetic accelerated server clock is used.
    const soakStarted=Date.now();let checks=0;const incidents=[];const lastTicks=new Map();const actionRtts=[];const browserSamples=[];const memoryBaselines=new Map();
    let shortOutage=false,longOutage=false,distant=false;let nextWalk=15;const walks=[];
    const sample=async(index)=>{
        const state=await evaluate(sessions[index],`(()=>{if(window.error)throw Error(window.error);return window.cached;})()`);
        assert.equal(state.status,'Playing demo');assert.equal(state.actors.filter(a=>a.owned).length,1);
        assert(state.content_delivery.queued_packets<=256&&state.content_delivery.queued_bytes<=4*1024*1024);
        assert(state.replica.motion.pending_frames<=128);
        const last=lastTicks.get(index);if(last!==undefined)assert(state.replica.tick>last,`Replica stopped advancing: ${JSON.stringify(state.replica)}`);lastTicks.set(index,state.replica.tick);
        const memory=await evaluate(sessions[index],'window.wasm.memory.buffer.byteLength');
        if(!memoryBaselines.has(index))memoryBaselines.set(index,memory);
        assert(memory-memoryBaselines.get(index)<=256*1024*1024,'WASM memory grew more than 256 MiB');
        browserSamples.push({seconds:(Date.now()-soakStarted)/1000,client:index,memory_bytes:memory,replica:state.replica,content:state.content_delivery});
        checks++;return state;
    };
    const look=async(intent='look')=>{
        const started=Date.now();
        const result=await evaluate(sessions[0],`(async()=>{while(busy)await new Promise(r=>setTimeout(r,5));client.request(${JSON.stringify(JSON.stringify({Intent:intent}))});window.cached=JSON.parse(client.state());const deadline=performance.now()+10000;while(performance.now()<deadline){if(window.error)throw Error(window.error);if(window.cached.action)return window.cached.action;await new Promise(r=>setTimeout(r,25));}throw Error('Soak action was not acknowledged');})()`);
        assert.deepEqual(result,{Outcome:'Processed'},JSON.stringify({result,samples:browserSamples.slice(-2),relays:relays.map(r=>r.stats)}));actionRtts.push(Date.now()-started);
    };
    const walk=async()=>{
        // The D2 controller treats a selected look/action intent as targeting.
        // Explicitly clear it through the authority before locomotion probes.
        await look('');
        const result=await evaluate(sessions[0],`(async()=>{while(busy)await new Promise(r=>setTimeout(r,5));const before=JSON.parse(client.state()).actors.find(a=>a.owned);client.intent('w');await new Promise(r=>setTimeout(r,350));while(busy)await new Promise(r=>setTimeout(r,5));const during=JSON.parse(client.state()).actors.find(a=>a.owned);client.intent('s');await new Promise(r=>setTimeout(r,350));while(busy)await new Promise(r=>setTimeout(r,5));client.intent('x');await new Promise(r=>setTimeout(r,1500));while(busy)await new Promise(r=>setTimeout(r,5));if(window.error)throw Error(window.error);const state=JSON.parse(client.state()),view=state.actors.find(a=>a.owned),actual=state.replica.authority_position;return {displacement:Math.hypot(during.x-before.x,during.z-before.z),settled_error:Math.hypot(view.x-actual[0],view.z-actual[2]),motion:state.replica.motion,snapshot:state.replica.snapshot,mode:view.mode};})()`);
        assert(Number.isFinite(result.displacement));assert(result.settled_error<0.1,JSON.stringify(result));console.error('WALK_PROBE '+JSON.stringify(result));walks.push({seconds:(Date.now()-soakStarted)/1000,...result});
    };
    if(walking){await walk();assert(walks.at(-1).displacement>0.01,'Initial movement probe did not move');}
    while(Date.now()-soakStarted<soakSeconds*1000) {
        await new Promise(r=>setTimeout(r,Math.min(5000,soakSeconds*1000-(Date.now()-soakStarted))));
        const elapsed=(Date.now()-soakStarted)/1000;
        if(wan&&!distant&&elapsed>=soakSeconds*0.1){
            distant=true;for(const relay of relays)relay.setProfile({latencyMs:120,jitterMs:80,bytesPerSecond:96*1024});
            incidents.push({kind:'increased-latency-and-pacing',at_seconds:elapsed});
        }
        if(wan&&!shortOutage&&elapsed>=soakSeconds*0.2){
            shortOutage=true;relays[0].pause();await new Promise(r=>setTimeout(r,2000));relays[0].resume();
            await new Promise(r=>setTimeout(r,3000));incidents.push({kind:'two-second-blackout',at_seconds:elapsed});
        }
        if(wan&&!longOutage&&elapsed>=soakSeconds*0.5){
            longOutage=true;
            const bobBefore=await sample(1);
            if(walking){await look('');await evaluate(sessions[0],`(async()=>{while(busy)await new Promise(r=>setTimeout(r,5));client.intent('w');await new Promise(r=>setTimeout(r,1000));return true;})()`);}
            relays[0].pause();let heldPosition=null;
            for(let i=0;i<7;i++){
                await new Promise(r=>setTimeout(r,5000));const bobNow=await sample(1);assert.equal(bobNow.actors.find(a=>a.owned).id,bobBefore.actors.find(a=>a.owned).id);
                const actor=bobNow.actors.find(a=>a.name==='Alice');
                if(walking&&actor&&heldPosition)assert(Math.hypot(actor.x-heldPosition.x,actor.z-heldPosition.z)<0.05,'Disconnected body kept walking');
                if(actor)heldPosition={x:actor.x,z:actor.z};
            }
            relays[0].cut();for(const relay of relays)relay.setProfile({latencyMs:45,jitterMs:45,bytesPerSecond:192*1024});
            await evaluate(sessions[0],`(async()=>{clearInterval(timer);while(busy)await new Promise(r=>setTimeout(r,5));client.free();window.client=new window.mod.DemoClient(${JSON.stringify(endpoint)},'Alice',false);window.cached={actors:[]};window.last=performance.now();window.error=null;startPump(20);return true;})()`);
            lastTicks.delete(0);
            await evaluate(sessions[0],`(async()=>{const deadline=performance.now()+20000;while(performance.now()<deadline){if(window.error)throw Error(window.error);if(window.cached.actors?.filter(a=>a.owned).length===1)return true;await new Promise(r=>setTimeout(r,50));}throw Error('WAN reconnect failed');})()`);
            incidents.push({kind:'35-second-blackout-and-explicit-reconnect',at_seconds:elapsed,healthy_peer_retained:true,runaway_motion:false});
        }
        for(let index=0;index<sessions.length;index++) {
            const state=await sample(index);
            assert.equal(state.actors.filter(a=>a.name==='Alice').length,1);assert.equal(state.actors.filter(a=>a.name==='Bob').length,1);
        }
        await look();
        if(walking&&elapsed>=nextWalk){await walk();nextWalk=(Date.now()-soakStarted)/1000+20;}
        console.error('WASM_PROGRESS '+JSON.stringify({seconds:(Date.now()-soakStarted)/1000,checks,incidents:incidents.length,action_rtt_ms:actionRtts.at(-1),replicas:browserSamples.slice(-2).map(s=>s.replica)}));
    }
    if(contentDelay>=2000)assert(canceledRequests>=1,'Dropping the WASM client did not cancel its pending fetch');
    if(walking&&soakSeconds>=20)assert(walks.some(w=>w.displacement>0.01),'Movement probe never left its starting position');
    const relayReport=[];for(const relay of relays){await relay.close();relayReport.push({...relay.stats});assert.equal(relay.stats.active,0);}
    actionRtts.sort((a,b)=>a-b);
    console.log(JSON.stringify({transport:'WSS (temporary certificate pinned)',content:'HTTPS compiled physics',clients:2,wasm:'spatial replicas, asynchronous content preloads, fixed inputs and sequenced requests',accounts:'none',saveStore:'none',high_refresh:fast,reload:true,content_delay_ms:contentDelay,content_queues:contentQueues,canceled_content_requests:canceledRequests,delayed_requests:delayedRequests,soak:{elapsed_seconds:(Date.now()-soakStarted)/1000,checks,incidents,walks,action_rtt_ms:{p50:actionRtts[Math.floor(actionRtts.length/2)],p95:actionRtts[Math.floor((actionRtts.length-1)*0.95)],max:actionRtts.at(-1)},samples:browserSamples},browser:browserVersion.product,wan_emulation:wan?relayReport:null}));
} finally {
    for(const timer of delayedTimers)clearTimeout(timer);
    for(const p of pending.values())clearTimeout(p.timer);
    await stopChild(chrome,'SIGTERM');
    for(const relay of relays)await relay.close();
    await stopChild(host,'SIGINT');
    await new Promise(r=>staticServer.close(r));await fs.rm(temp,{recursive:true,force:true});
    process.removeListener('SIGINT',interrupt);process.removeListener('SIGTERM',interrupt);
}
