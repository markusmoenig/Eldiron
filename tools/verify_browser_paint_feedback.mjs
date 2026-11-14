// Run against an instrumented, task-owned Creator WebGPU page. The page must
// expose its injected device as window.__eldironDevice. This tests shader and
// four-byte sampler receipts, not the full Rust bake producer in a browser.
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
 const device=window.__eldironDevice;
 device.pushErrorScope('validation');
 const full=device.createShaderModule({label:'current complete Compute3D shader',code:shader});
 const info=await full.getCompilationInfo();
 const messages=[...info.messages].map(m=>({type:m.type,message:m.message,line:m.lineNum}));
 await device.createComputePipelineAsync({label:'complete Compute3D eight-storage pipeline',layout:'auto',compute:{module:full,entryPoint:'cs_main'}});
 const receipts=[];
 // Exercise the actual paint sampler/header layout, keeping this appended
 // entry distinct from the full renderer's geometry/transport entry above.
 for(const [name,key,resident] of [['missing',7,false],['unused',8,false],['resident',7,true]]) {
  const code=shader+`
@compute @workgroup_size(1) fn paint_status_contract() {
    let paint=sample_surface_paint_nearest(vec2<f32>(0.5),vec4<u32>(${key}u,0u,0u,1u));
    textureStore(color_out,vec2<i32>(0),paint.overlay+vec4<f32>(paint.material)/255.0);
  }`;
  const module=device.createShaderModule({label:'paint status contract '+name,code});
  const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module,entryPoint:'paint_status_contract'}});
  const words=new Uint32Array(5+24);words.set([0xffffffff,4,2,28,0]);
  let hash=2166136261;for(const value of [7,0,0,1,0,0])hash=Math.imul(hash^value,16777619)>>>0;
  const slot=hash&1;
  words.set([7,0,0,1,0,0,64,64,0,0,resident?64:0,resident?64:0],5+slot*12);
  const source=device.createBuffer({label:'paint dependency auxiliary contract',size:words.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(source,0,words);
  const result=device.createBuffer({label:'paint dependency four-byte contract receipt',size:4,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST});
  const output=device.createTexture({label:'paint status contract output',size:[1,1],format:'rgba8unorm',usage:GPUTextureUsage.STORAGE_BINDING});
  const color=device.createTexture({size:[64,64],format:'rgba8unorm',usage:GPUTextureUsage.TEXTURE_BINDING});
  const material=device.createTexture({size:[64,64],format:'rgba8uint',usage:GPUTextureUsage.TEXTURE_BINDING});
  const bind=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:1,resource:output.createView()},{binding:16,resource:color.createView()},{binding:17,resource:material.createView()},{binding:18,resource:{buffer:source}}]});
  const encoder=device.createCommandEncoder({label:'paint dependency metadata-only contract'});
  const pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,bind);pass.dispatchWorkgroups(1);pass.end();
  encoder.copyBufferToBuffer(source,0,result,0,4);device.queue.submit([encoder.finish()]);
  await result.mapAsync(GPUMapMode.READ);
  const actual=new Uint32Array(result.getMappedRange())[0];result.unmap();
  const expected=name==='missing'?slot:0xffffffff;
  receipts.push({name,expected,actual,bytes:4});
  for(const resource of [source,result,output,color,material])resource.destroy();
  if(actual!==expected)throw Error('Invalid '+name+' paint receipt '+JSON.stringify(receipts));
 }
 const error=await device.popErrorScope();
 if(error||messages.some(m=>m.type==='error'))throw Error('GPU validation '+JSON.stringify({error:error?.message,messages}));
 return {messages,error:null,maxStorageBuffersPerShaderStage:device.limits.maxStorageBuffersPerShaderStage,receipts,probe:window.__eldironProbe};
};
try {
 const result=await call('Runtime.evaluate',{expression:`(${exercise.toString()})(${JSON.stringify(shader)})`,awaitPromise:true,returnByValue:true});
 if(result.exceptionDetails)throw Error(JSON.stringify(result.exceptionDetails));
 console.log(JSON.stringify(result.result.value));
} finally {ws.close();}
