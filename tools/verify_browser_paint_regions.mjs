// Run against an instrumented, task-owned Creator WebGPU page. The page must
// expose its injected device as window.__eldironDevice. This tests shader and
// absolute subdispatch bounds, not the full Rust bake producer in a browser.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
const shader=fs.readFileSync(root+'crates/scenevm/embedded/3d_header.wgsl','utf8')+fs.readFileSync(root+'crates/scenevm/embedded/3d_body.wgsl','utf8');
const pages=await(await fetch(`http://127.0.0.1:${process.env.ELDIRON_CDP_PORT??'9267'}/json/list`)).json();const page=pages.find(p=>p.type==='page'&&p.url.startsWith(process.env.ELDIRON_BROWSER_PREFIX??'http://127.0.0.1:8898/creator/'));
if(!page)throw Error('No task-owned Creator page');
const ws=new WebSocket(page.webSocketDebuggerUrl);await new Promise(r=>ws.addEventListener('open',r,{once:true}));
let id=1;const pending=new Map();ws.addEventListener('message',e=>{const m=JSON.parse(e.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result);}});
function call(method,params){return new Promise((resolve,reject)=>{const n=id++;const timer=setTimeout(()=>{pending.delete(n);reject(Error(method+' timed out'));},60000);pending.set(n,{resolve:value=>{clearTimeout(timer);resolve(value);},reject:error=>{clearTimeout(timer);reject(error);}});ws.send(JSON.stringify({id:n,method,params}));});}
const exercise = async function(shader) {
 const device=window.__eldironDevice;device.pushErrorScope('validation');
 const module=device.createShaderModule({label:'complete current Compute3D region validation',code:shader});
 const info=await module.getCompilationInfo();
 const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module,entryPoint:'cs_main'}});
 const resources=[];
 function buffer(size,usage){const value=device.createBuffer({size,usage:usage|GPUBufferUsage.COPY_DST});resources.push(value);return value;}
 function texture(size,format,usage){const value=device.createTexture({size,format,usage});resources.push(value);return value;}
 const uniform=buffer(4480,GPUBufferUsage.UNIFORM),grid=buffer(80,GPUBufferUsage.UNIFORM);
 const storage=new Map([4,5,6,8,12,13,15,18].map(binding=>[binding,buffer(256,GPUBufferUsage.STORAGE)]));
 const atlas=texture([1,1],'rgba8unorm',GPUTextureUsage.TEXTURE_BINDING);
 const paint=texture([64,64],'rgba8unorm',GPUTextureUsage.TEXTURE_BINDING);
 const material=texture([64,64],'rgba8uint',GPUTextureUsage.TEXTURE_BINDING);
 const prior=texture([13,9],'rgba8unorm',GPUTextureUsage.TEXTURE_BINDING);
 const output=texture([13,9],'rgba8unorm',GPUTextureUsage.STORAGE_BINDING|GPUTextureUsage.COPY_SRC);
 const reference=texture([13,9],'rgba8unorm',GPUTextureUsage.STORAGE_BINDING|GPUTextureUsage.COPY_SRC);
 const sampler=device.createSampler();
 async function draw(target,color,origin,extent){
  const data=new ArrayBuffer(4480),f=new Float32Array(data),u=new Uint32Array(data);
  f.set([...color,1],0);u.set([13,9],4);u.set(origin,6);
  for(const at of [48,53,58,63])f[at]=1;
  f.set([0,0,4,0],68);f.set([0,0,-1,0],72);f.set([1,0,0,0],76);f.set([0,1,0,0],80);
  f.set([50,1,0.1,100],84);u.set(extent,92);
  device.queue.writeBuffer(uniform,0,data);
  const entries=[{binding:0,resource:{buffer:uniform}},{binding:1,resource:target.createView()},{binding:2,resource:atlas.createView()},{binding:3,resource:sampler},{binding:7,resource:{buffer:grid}},{binding:11,resource:atlas.createView()},{binding:14,resource:prior.createView()},{binding:16,resource:paint.createView()},{binding:17,resource:material.createView()},...[...storage].map(([binding,buffer])=>({binding,resource:{buffer}}))];
  const bind=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries});
  const encoder=device.createCommandEncoder();const pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,bind);pass.dispatchWorkgroups(Math.ceil((extent[0]||13)/8),Math.ceil((extent[1]||9)/8));pass.end();device.queue.submit([encoder.finish()]);
 }
 async function capture(target){
  const staging=buffer(256*9,GPUBufferUsage.MAP_READ);
  const encoder=device.createCommandEncoder({label:'renderkit-asset-capture'});
  encoder.copyTextureToBuffer({texture:target},{buffer:staging,bytesPerRow:256,rowsPerImage:9},[13,9]);device.queue.submit([encoder.finish()]);await staging.mapAsync(GPUMapMode.READ);
  const mapped=new Uint8Array(staging.getMappedRange());const result=new Uint8Array(13*9*4);
  for(let y=0;y<9;y++)result.set(mapped.subarray(y*256,y*256+52),y*52);
  staging.unmap();return result;
 }
 await draw(output,[0,1,0],[0,0],[0,0]);const before=await capture(output);
 await draw(reference,[1,0,0],[0,0],[0,0]);const red=await capture(reference);
 // Nonzero unaligned origin plus odd extent exposes writes from rounded groups.
 await draw(output,[1,0,0],[2,1],[7,5]);const actual=await capture(output);
 let changed=0,preserved=0;
 for(let y=0;y<9;y++)for(let x=0;x<13;x++){
  const inside=x>=2&&x<9&&y>=1&&y<6;const expected=inside?red:before;const at=(y*13+x)*4;
  for(let channel=0;channel<4;channel++)if(actual[at+channel]!==expected[at+channel])throw Error('Region pixel mismatch '+JSON.stringify({x,y,channel,actual:actual[at+channel],expected:expected[at+channel]}));
  if(inside)changed++;else preserved++;
 }
 if(before[1]<150||red[0]<150||before[0]||red[1])throw Error('Reference full-frame colors did not render');
 const error=await device.popErrorScope();if(error)throw Error(error.message);
 for(const resource of resources)resource.destroy();
 return {messages:[...info.messages].map(m=>({type:m.type,message:m.message})),maxStorageBuffersPerShaderStage:device.limits.maxStorageBuffersPerShaderStage,full_size:[13,9],region:{origin:[2,1],size:[7,5]},changed,preserved,exact_rgba:true};
};
try {
 const result=await call('Runtime.evaluate',{expression:`(${exercise.toString()})(${JSON.stringify(shader)})`,awaitPromise:true,returnByValue:true});
 if(result.exceptionDetails)throw Error(JSON.stringify(result.exceptionDetails));
 console.log(JSON.stringify(result.result.value));
} finally {ws.close();}
