import {test} from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import {once} from 'node:events';
import {setTimeout as sleep} from 'node:timers/promises';
import {randomBytes,createHash} from 'node:crypto';
import {networkRelay} from './network_relay.mjs';
test('paced fragmented stream preserves bytes and closes held connections',async()=>{
    const sockets=new Set();
    const server=net.createServer(s=>{sockets.add(s);s.on('error',()=>{});s.on('close',()=>sockets.delete(s));s.pipe(s);});
    server.listen(0,'127.0.0.1');await once(server,'listening');
    const relay=await networkRelay(server.address().port,{latencyMs:5,jitterMs:5,bytesPerSecond:1024*1024});
    const socket=net.connect(relay.port,'127.0.0.1');socket.on('error',()=>{});
    try {
        await once(socket,'connect');
        const data=randomBytes(128*1024), chunks=[];let count=0;
        const received=new Promise(resolve=>socket.on('data',b=>{chunks.push(b);count+=b.length;if(count===data.length)resolve(Buffer.concat(chunks));}));
        socket.write(data);
        const output=await Promise.race([received,sleep(5000,undefined,{ref:false}).then(()=>{throw Error('relay did not drain');})]);
        const hash=b=>createHash('sha256').update(b).digest('hex');assert.equal(hash(output),hash(data));
        relay.pause();socket.write('held');await sleep(25);relay.cut();await sleep(25);
        assert.equal(relay.stats.active,0);assert.equal(socket.destroyed,true);assert(relay.stats.fragments>100);assert(relay.stats.peakWritableBytes<128*1024);
    }finally{socket.destroy();await relay.close();for(const s of sockets)s.destroy();await new Promise(r=>server.close(r));}
});

test('graceful HTTP-style upstream close drains paced response bytes',async()=>{
    const data=randomBytes(64*1024);
    const server=net.createServer(s=>{s.on('error',()=>{});s.end(data);});
    server.listen(0,'127.0.0.1');await once(server,'listening');
    const relay=await networkRelay(server.address().port,{latencyMs:10,jitterMs:10,bytesPerSecond:256*1024});
    const client=net.connect(relay.port,'127.0.0.1');client.on('error',()=>{});
    try {
        const chunks=[];for await(const chunk of client)chunks.push(chunk);
        assert.deepEqual(Buffer.concat(chunks),data);
    }finally{client.destroy();await relay.close();await new Promise(r=>server.close(r));}
});

test('propagation delay pipelines instead of reducing stream bandwidth per read',async()=>{
    const sockets=new Set();const data=randomBytes(1024*1024);
    const server=net.createServer(s=>{sockets.add(s);s.on('error',()=>{});s.once('close',()=>sockets.delete(s));s.pipe(s);});
    server.listen(0,'127.0.0.1');await once(server,'listening');
    const relay=await networkRelay(server.address().port,{latencyMs:100,jitterMs:20,bytesPerSecond:512*1024});
    const client=net.connect(relay.port,'127.0.0.1');client.on('error',()=>{});const started=performance.now();
    try {
        await once(client,'connect');const chunks=[];let count=0;
        const received=new Promise(r=>client.on('data',b=>{chunks.push(b);count+=b.length;if(count===data.length)r();}));
        client.write(data);await Promise.race([received,sleep(8000,undefined,{ref:false}).then(()=>{throw Error('pipeline timed out');})]);
        assert.deepEqual(Buffer.concat(chunks),data);assert(performance.now()-started<4000,'propagation latency serialized each chunk');
    }finally{client.destroy();await relay.close();for(const socket of sockets)socket.destroy();await new Promise(r=>server.close(r));}
});
