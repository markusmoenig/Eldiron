"""Create an offline, synchronized comparison after baking styles/{preset}."""
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
data = {name: json.loads((root / 'styles' / name / 'atlas.json').read_text())
        for name in ('blocky', 'stylized', 'natural')}
page = '''<!doctype html><html lang="en"><meta charset="utf-8">
<title>Eldiron character styles</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
body{margin:32px auto;padding:0 24px;max-width:1040px;background:#171b23;color:#e4e8ef;font:15px system-ui}h1{font-size:25px}p{line-height:1.6;color:#b5c0d0}label{display:inline-block;margin:12px 18px 16px 0}select,button{padding:8px;background:#29313e;color:inherit;border:1px solid #526078;border-radius:5px}.grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:18px}canvas{width:100%;image-rendering:pixelated;background:#242d3a;border:1px solid #526078}h2{font-size:18px}a{color:#a7ceff}small{color:#b5c0d0}@media(max-width:650px){.grid{grid-template-columns:1fr}}
</style>
<h1>One character, three creation styles</h1>
<p>Style is chosen by the generator/app. All three use the same character definition, equipment, rig and animations.</p>
<label>Animation <select id="motion"></select></label>
<label>Direction <select id="direction"></select></label><button id="pause">Pause</button><button id="restart">Restart</button><label><input id="repeat" type="checkbox" checked> Repeat actions</label><label>Frame <input id="frame" type="range" min="0" max="7" step="1" value="0"></label>
<div class="grid" id="grid"></div><p id="status"></p>
<p>Blocky retains box forms. Stylized uses faceted rounded shapes. Natural uses a narrower head, shaped torso and smoother surfaces. Natural is an early low-poly form; detailed anatomy and realistic faces still need work.</p>
<script>
const data=__DATA__, motion=document.getElementById('motion'), direction=document.getElementById('direction');
for(const name of Object.keys(data.blocky.durations)){const o=document.createElement('option');o.value=name;o.textContent=name;motion.append(o);}motion.value='walk';
for(const name of [...new Set(data.blocky.frames.map(f=>f.direction))]){const o=document.createElement('option');o.value=name;o.textContent=name.replaceAll('_',' ');direction.append(o);}direction.value='front_right';
const panels=Object.entries(data).map(([name,meta])=>{
 const section=document.createElement('section');section.innerHTML=`<h2>${name[0].toUpperCase()+name.slice(1)}</h2><canvas width="288" height="288"></canvas><p><small>${meta.style.segments} sides · ${meta.style.shading} shading</small></p><a href="styles/${name}/viewer3d.html">3D viewer</a> · <a href="styles/${name}/character.glb" download>GLB</a> · <a href="styles/${name}/preview.html">Full preview</a>`;document.getElementById('grid').append(section);
 const canvas=section.querySelector('canvas'),ctx=canvas.getContext('2d');ctx.imageSmoothingEnabled=false;const image=new Image();image.src=`styles/${name}/atlas.png`;return {meta,canvas,ctx,image};
});
let elapsed=0,last=null,running=true;
const scrub=document.getElementById('frame');scrub.oninput=()=>{const frames=data.blocky.frames.filter(f=>f.motion===motion.value&&f.direction===direction.value);const loop=(data.blocky.looping?.[motion.value] ?? true)||document.getElementById('repeat').checked;elapsed=(loop?Number(scrub.value)*data.blocky.durations[motion.value]/frames.length:frames[Number(scrub.value)].time)+0.00001;running=false;document.getElementById('pause').textContent='Play';};
document.getElementById('pause').onclick=()=>{running=!running;document.getElementById('pause').textContent=running?'Pause':'Play';};document.getElementById('restart').onclick=()=>{elapsed=0;running=true;document.getElementById('pause').textContent='Pause';};document.getElementById('repeat').onchange=motion.onchange=direction.onchange=()=>elapsed=0;
function draw(now){if(last!==null&&running)elapsed+=Math.min((now-last)/1000,0.1);last=now;let current=0;
 for(const {meta,canvas,ctx,image} of panels){const frames=meta.frames.filter(f=>f.motion===motion.value&&f.direction===direction.value);const loop=(meta.looping?.[motion.value] ?? true)||document.getElementById('repeat').checked;const index=loop?Math.floor(elapsed/meta.durations[motion.value]*frames.length)%frames.length:Math.min(frames.length-1,Math.floor(elapsed/meta.durations[motion.value]*(frames.length-1)));const frame=frames[index];current=frame.frame+1;scrub.max=frames.length-1;scrub.value=index;ctx.clearRect(0,0,canvas.width,canvas.height);if(image.complete&&image.naturalWidth){const [x,y,w,h]=frame.rect;ctx.drawImage(image,x,y,w,h,0,0,canvas.width,canvas.height);}}
 document.getElementById('status').textContent=`${motion.value} · ${direction.value.replaceAll('_',' ')} · frame ${current}/${data.blocky.frames.filter(f=>f.motion===motion.value&&f.direction===direction.value).length}`;requestAnimationFrame(draw);
}requestAnimationFrame(draw);
</script></html>'''
(root / 'style-comparison.html').write_text(page.replace('__DATA__', json.dumps(data).replace('<', '\\u003c')))
print(root / 'style-comparison.html')
