// Run the generated page's real scripts without a browser. The recording context
// also exports sprite draw calls for preview.py; it is not a browser screenshot.
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import vm from 'node:vm';
import assert from 'node:assert/strict';

const out = resolve(process.argv[2] ?? 'generated/mz-iso2d');
const html = await readFile(`${out}/index.html`, 'utf8');
const manifest = JSON.parse(await readFile(`${out}/manifest.json`, 'utf8'));
let calls = [], raf;
const listeners = {}, elements = new Map();
const context = {
  createLinearGradient: () => ({ addColorStop() {} }),
  fillRect(x,y,w,h) { if(w===1000 && h===660) calls=[]; },
  drawImage(img,...rect) { calls.push({ image:img.key, rect }); },
};
for(const key of ['beginPath','moveTo','lineTo','closePath','fill','stroke',
  'save','restore','translate','rotate','scale','arc','ellipse','setLineDash'])context[key]=()=>{};
function element(id) {
  if(!elements.has(id))elements.set(id, { textContent:'', setAttribute(){}, focus(){} });
  return elements.get(id);
}
Object.assign(element('scene'), {width:1000,height:660,getContext:()=>context,
  getBoundingClientRect:()=>({left:0,top:0,width:1000,height:660})});
const cases=['wall-behind','wall-front','room-inside','tree-behind','tree-front']
  .map(id=>({dataset:{case:id},textContent:id}));
class Image {
  set src(src) {
    this.key=Object.keys(manifest.images).find(k=>manifest.images[k]===src);
    assert.ok(this.key,'Unexpected external image');
    queueMicrotask(()=>this.onload());
  }
}
const sandbox=vm.createContext({console,Image,
  document:{getElementById:element,querySelectorAll:()=>cases,
    addEventListener:(name,fn)=>listeners[name]=fn},
  window:{addEventListener(){}},requestAnimationFrame:fn=>raf=fn});
for(const script of html.matchAll(/<script>([\s\S]*?)<\/script>/g))vm.runInContext(script[1],sandbox);
await new Promise(setImmediate);
const run=code=>vm.runInContext(code,sandbox);
assert.equal(run('ready'),true);
assert.ok(raf);
const snapshots=[];
function capture(name) {
  run('draw()');
  snapshots.push({name,calls:structuredClone(calls)});
  assert.equal(calls.filter(c=>c.image==='character').length,1);
  assert.equal(calls.filter(c=>c.image.startsWith('corner')).length,4);
  assert.equal(calls.filter(c=>c.image.startsWith('wall')).length,12);
}
for(const button of cases) {
  button.onclick();
  assert.equal(run('paused && !auto && !moving'),true);
  capture(button.dataset.case);
}
// Inside the closed room, pointing outside must not create a route.
cases[2].onclick();
let target=run('screen(8,5)');
element('scene').onpointerdown({clientX:target.x,clientY:target.y});
assert.equal(run('route.length'),0);
// A legal interior click is reachable; execute real frame updates to completion.
target=run('screen(6,6)');
element('scene').onpointerdown({clientX:target.x,clientY:target.y});
for(let i=0;i<80;i++)run('update(.02)');
assert.ok(run('Math.hypot(hero.x-6,hero.z-6)<.05'));
// Drive into the east wall. No tunnelling, and the walk cycle stops when blocked.
listeners.keydown({key:'d',preventDefault(){}});
listeners.keydown({key:'s',preventDefault(){}});
for(let i=0;i<120;i++)run('update(.02)');
assert.ok(run('hero.z<6.68 && !W.blocked(hero.x,hero.z)'));
assert.equal(run('moving'),false);
listeners.keyup({key:'d'});listeners.keyup({key:'s'});
// Automatic movement must circle the entire room and pass both sides of the tree.
run('hero.x=4.6;hero.z=2.1;patrolIndex=1;clock=0;setAuto(true);setPaused(false)');
let minX=Infinity,maxX=0,minZ=Infinity,maxZ=0;
const walk=[];
for(let i=0;i<2400;i++) {
  run('update(.025)');
  const p=run('({...hero})');
  assert.equal(run('W.blocked(hero.x,hero.z)'),false,`Collision at step ${i}`);
  minX=Math.min(minX,p.x);maxX=Math.max(maxX,p.x);minZ=Math.min(minZ,p.z);maxZ=Math.max(maxZ,p.z);
  if(i%8===0&&walk.length<100){run('draw()');walk.push(structuredClone(calls));}
}
assert.ok(minX<2.2&&maxX>9.5&&minZ<2.2&&maxZ>8.6);
await writeFile(`${out}/preview-frames.json`,JSON.stringify({snapshots,walk,
  props:run('W.props'),projection:manifest.projection,origin:run('origin')}));
console.log('Page scripts verified: closed-room picking, keyboard collision, all overlap cases, and 60 seconds of automatic walking.');
console.log('Saved draw calls for optional sprite-render previews.');
