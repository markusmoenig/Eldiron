import { test } from 'node:test';
import assert from 'node:assert/strict';
import './world.js';
const w = globalThis.MZIso;

test('ground picking roundtrips across fractional and negative coordinates', () => {
  for (let x=-2;x<=12;x+=.37) for(let z=-2;z<=12;z+=.53) {
    const s=w.project(x,z), p=w.unproject(s.x,s.y);
    assert.ok(Math.abs(p.x-x)<1e-9 && Math.abs(p.z-z)<1e-9);
  }
});
test('anchors order the character behind and in front of each obstacle', () => {
  for(const prop of w.props) {
    const behind=w.ordered({x:prop.x,z:prop.z-1}), front=w.ordered({x:prop.x,z:prop.z+1});
    assert.ok(behind.findIndex(p=>p.id==='character')<behind.findIndex(p=>p.id===prop.id));
    assert.ok(front.findIndex(p=>p.id==='character')>front.findIndex(p=>p.id===prop.id));
  }
});
test('all automatic patrol legs have collision-free routes around wall and trunk', () => {
  for(let i=0;i<w.patrol.length;i++) {
    const from=w.patrol[i], to=w.patrol[(i+1)%w.patrol.length], route=w.path(from,to);
    assert.ok(route.length, `No path for leg ${i}`);
    let last=from;
    for(const p of route) {assert.ok(w.segmentClear(last,p));last=p;}
    assert.ok(Math.hypot(last.x-to.x,last.z-to.z)<.001);
  }
});
test('a click across the wall detours instead of walking through or clipping corners', () => {
  const from={x:2,z:5},to={x:8,z:5};
  assert.equal(w.segmentClear(from,to),false);
  const route=w.path(from,to);assert.ok(route.length>1);
  let last=from;for(const p of route) {assert.ok(w.segmentClear(last,p));last=p;}
});
test('obstacles and out-of-bounds clicks are rejected', () => {
  for(const to of [{x:4,z:3},w.TREE,{x:-1,z:3},{x:12,z:3}]) {
    assert.deepEqual(w.path({x:2,z:2},to),[]);
  }
});
test('four joined walls fully enclose the room, including every corner', () => {
  for(const outside of [{x:2,z:5},{x:8,z:5},{x:5,z:2},{x:5,z:8},{x:2,z:2},{x:8,z:8}]) {
    assert.deepEqual(w.path({x:5,z:5},outside),[]);
  }
  for(const [x,z] of [[3,3],[7,3],[7,7],[3,7]])assert.equal(w.blocked(x,z),true);
  for(let t=3;t<=7;t+=.05)for(const [x,z] of [[t,3],[t,7],[3,t],[7,t]])assert.equal(w.blocked(x,z),true);
});
