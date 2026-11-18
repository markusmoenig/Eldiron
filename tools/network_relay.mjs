// Test-only opaque TCP pacing. TLS remains end-to-end; no packet-loss or
// TCP/WebSocket reordering claim. Socket backpressure bounds retained data.
import net from 'node:net';
import {once} from 'node:events';
import {setTimeout as sleep} from 'node:timers/promises';
export async function networkRelay(upstream, initial={latencyMs:45,jitterMs:45,bytesPerSecond:192*1024}) {
    let profile=initial, sequence=0, held=false, stopped=false;
    const peers=new Set(), jobs=new Set();
    const stats={connections:0,active:0,peakActive:0,bytes:0,fragments:0,peakWritableBytes:0,cuts:0};
    function setProfile(value) {
        if(!Number.isInteger(value.latencyMs)||value.latencyMs<0||value.latencyMs>500||!Number.isInteger(value.jitterMs)||value.jitterMs<0||value.jitterMs>500||!Number.isInteger(value.bytesPerSecond)||value.bytesPerSecond<4096)throw Error('Invalid bounded relay profile');
        profile={...value};
    }
    setProfile(initial);
    async function forward(input,output,signal) {
        const queue=[];let ended=false, finished=false, wake=null, space=null;
        const notify=()=>{wake?.();wake=null;};const available=()=>{space?.();space=null;};
        const reader=(async()=>{
            try {
                for await(const data of input) {
                    for(let offset=0;offset<data.length;offset+=8192) {
                        while(queue.length>=8&&!signal.aborted&&!finished)await new Promise(r=>{space=r;});
                        if(signal.aborted||finished)return;
                        const turn=sequence++,p=profile;
                        queue.push({bytes:Buffer.from(data.subarray(offset,offset+8192)),ready:performance.now()+p.latencyMs+(turn%5)*p.jitterMs/4,p,turn});notify();
                    }
                }
            }finally{ended=true;notify();}
        })();
        // Rejecting a reader is also observed when the writer is held.
        reader.catch(()=>{});
        const aborted=()=>{notify();available();};signal.addEventListener('abort',aborted,{once:true});
        let next=performance.now();
        try {
            while(!signal.aborted) {
                if(!queue.length){if(ended)break;await new Promise(r=>{wake=r;});continue;}
                const {bytes,ready,p,turn}=queue.shift();available();
                await sleep(Math.max(0,Math.max(ready,next)-performance.now()),undefined,{signal});
                const sent=performance.now();
                for(let start=0;start<bytes.length;start+=[127,509,1024][turn%3]) {
                    while(held)await sleep(10,undefined,{signal});
                    const fragment=bytes.subarray(start,start+[127,509,1024][turn%3]);
                    const flushed=output.write(fragment);
                    stats.bytes+=fragment.length;stats.fragments++;
                    stats.peakWritableBytes=Math.max(stats.peakWritableBytes,output.writableLength);
                    if(!flushed)await once(output,'drain',{signal});
                }
                next=sent+bytes.length*1000/p.bytesPerSecond;
            }
        }finally{finished=true;input.destroy();available();signal.removeEventListener('abort',aborted);await reader.catch(()=>{});}
    }
    const server=net.createServer(client=>{
        if(stopped||peers.size>=128){client.destroy();return;}
        const target=net.connect({host:'127.0.0.1',port:upstream});
        const abort=new AbortController();
        const peer={client,target,abort};peers.add(peer);
        stats.connections++;stats.active++;stats.peakActive=Math.max(stats.peakActive,stats.active);
        let connected=false;
        for(const socket of [client,target]){socket.setNoDelay(true);socket.on('error',()=>abort.abort());}
        // Graceful upstream close can precede delivery of buffered response
        // bytes. Let the forwarding iterator drain them before retiring peers.
        client.once('close',()=>{if(!connected)abort.abort();});
        const job=(async()=>{
            try {
                await once(target,'connect',{signal:abort.signal});connected=true;
                // Wait for both owned forwarding jobs even when one fails.
                const pump=async(a,b)=>{try{await forward(a,b,abort.signal);}finally{abort.abort();client.destroy();target.destroy();}};
                await Promise.allSettled([pump(client,target),pump(target,client)]);
            }catch{}finally{abort.abort();client.destroy();target.destroy();peers.delete(peer);stats.active--;}
        })();
        jobs.add(job);job.finally(()=>jobs.delete(job));
    });
    await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
    const cut=()=>{stats.cuts++;held=false;for(const p of peers){p.abort.abort();p.client.destroy();p.target.destroy();}};
    return {port:server.address().port,stats,setProfile,pause:()=>{held=true;},resume:()=>{held=false;},cut,
        close:async()=>{if(stopped)return;stopped=true;cut();await new Promise(r=>server.close(r));await Promise.allSettled([...jobs]);}};
}
