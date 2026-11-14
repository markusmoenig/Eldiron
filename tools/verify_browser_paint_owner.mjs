// Drive the real Rust pressure acceptance executable in a task-owned WebGPU tab.
// Package creator/examples/paint_pressure_web.rs and serve at the URL below.
import fs from 'node:fs';
const endpoint=`http://127.0.0.1:${process.env.ELDIRON_CDP_PORT??'9267'}`;
const url=process.env.ELDIRON_PAINT_OWNER_URL??'http://127.0.0.1:8898/paint-pressure/';
const page=await(await fetch(endpoint+'/json/new?about:blank',{method:'PUT'})).json();
const ws=new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{ws.addEventListener('open',resolve,{once:true});ws.addEventListener('error',reject,{once:true});});
let next=1;const pending=new Map();
ws.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p?.reject(Error(JSON.stringify(m.error))):p?.resolve(m.result);}});
function call(method,params={}){return new Promise((resolve,reject)=>{const id=next++;const timer=setTimeout(()=>{pending.delete(id);reject(Error(method+' timed out'));},30000);pending.set(id,{resolve:r=>{clearTimeout(timer);resolve(r);},reject:e=>{clearTimeout(timer);reject(e);}});ws.send(JSON.stringify({id,method,params}));});}
try {
 await call('Page.enable');
 await call('Network.setCacheDisabled',{cacheDisabled:true});
 await call('Page.addScriptToEvaluateOnNewDocument',{source:`
 window.__paintOwnerProbe={devices:[],gpuErrors:[],errors:[],contexts2D:0,textureCopies:{},metadataBytes:0};
 window.addEventListener('error',event=>window.__paintOwnerProbe.errors.push(event.message));
 window.addEventListener('unhandledrejection',event=>window.__paintOwnerProbe.errors.push(String(event.reason)));
 const canvasContext=HTMLCanvasElement.prototype.getContext;
 HTMLCanvasElement.prototype.getContext=function(type,...args){if(type==='2d')window.__paintOwnerProbe.contexts2D++;return canvasContext.call(this,type,...args);};
 const request=GPUAdapter.prototype.requestDevice;
 GPUAdapter.prototype.requestDevice=async function(...args){
  const device=await request.apply(this,args);const probe=window.__paintOwnerProbe;
  probe.devices.push({maxStorageBuffersPerShaderStage:device.limits.maxStorageBuffersPerShaderStage,features:[...device.features],requestedFeatures:Array.from(args[0]?.requiredFeatures??[])});
  device.addEventListener('uncapturederror',event=>probe.gpuErrors.push(event.error.message));
  const create=device.createCommandEncoder.bind(device);
  device.createCommandEncoder=function(descriptor){const encoder=create(descriptor);const copy=encoder.copyTextureToBuffer.bind(encoder);encoder.copyTextureToBuffer=function(...args){const label=descriptor?.label||'(unlabelled)';probe.textureCopies[label]=(probe.textureCopies[label]||0)+1;return copy(...args);};const bufferCopy=encoder.copyBufferToBuffer.bind(encoder);encoder.copyBufferToBuffer=function(...args){if(args[2]?.label==='SceneVM paint dependency status')probe.metadataBytes+=args[4];return bufferCopy(...args);};return encoder;};
  return device;
 };
 `});
 await call('Page.navigate',{url});
 const end=Date.now()+180000;let state;
 while(Date.now()<end){
  await new Promise(resolve=>setTimeout(resolve,200));
  const response=await call('Runtime.evaluate',{expression:`JSON.stringify({state:document.documentElement.getAttribute('data-paint-pressure-state'),result:JSON.parse(document.documentElement.getAttribute('data-paint-pressure-result')||'null'),probe:window.__paintOwnerProbe})`,returnByValue:true});
  if(response.exceptionDetails)throw Error(JSON.stringify(response.exceptionDetails));
  state=JSON.parse(response.result.value);
  if(state.probe?.errors.length||state.probe?.gpuErrors.length||state.state==='failed')throw Error(JSON.stringify(state));
  if(state.state==='passed')break;
 }
 if(state?.state!=='passed')throw Error('Rust paint owner timed out '+JSON.stringify(state));
 if(state.probe.contexts2D||state.probe.devices.length!==2||state.probe.devices.some(device=>device.maxStorageBuffersPerShaderStage!==8||device.requestedFeatures.length)||Object.keys(state.probe.textureCopies).some(label=>!['renderkit-asset-capture','asset surface depth capture'].includes(label)))throw Error('WebGPU architecture contract failed '+JSON.stringify(state));
 console.log('browser-rust-paint-pressure-owner',JSON.stringify({url,targetId:page.id,...state}));
 if(process.env.ELDIRON_PAINT_OWNER_EVIDENCE)fs.writeFileSync(process.env.ELDIRON_PAINT_OWNER_EVIDENCE,JSON.stringify({url,...state},null,2)+'\n');
} finally {ws.close();}
