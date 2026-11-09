/* Shared, DOM-free experiment logic. The browser and Node checks use this file. */
globalThis.MZIso = (() => {
  const HALF_W = 32, HALF_H = 16, VERTICAL = 39.1918359, SIZE = 11;
  const ROOM = { x0: 3, x1: 7, z0: 3, z1: 7 };
  const WALLS = [
    { x0: 2.89, x1: 7.11, z0: 2.89, z1: 3.11 },
    { x0: 2.89, x1: 7.11, z0: 6.89, z1: 7.11 },
    { x0: 2.89, x1: 3.11, z0: 2.89, z1: 7.11 },
    { x0: 6.89, x1: 7.11, z0: 2.89, z1: 7.11 }
  ];
  const TREE = { x: 8.6, z: 7.4, radius: .33 };
  const RADIUS = .22, STEP = .25;
  function project(x, z, height = 0) {
    return { x: (x + z) * HALF_W, y: (z - x) * HALF_H - height * VERTICAL };
  }
  function unproject(x, y) {
    return { x: (x / HALF_W - y / HALF_H) / 2, z: (x / HALF_W + y / HALF_H) / 2 };
  }
  function blocked(x, z) {
    if (x < .3 || z < .3 || x > SIZE - .3 || z > SIZE - .3) return true;
    return WALLS.some(wall => {
      const dx = x - Math.max(wall.x0, Math.min(wall.x1, x));
      const dz = z - Math.max(wall.z0, Math.min(wall.z1, z));
      return dx * dx + dz * dz < RADIUS * RADIUS;
    }) ||
      Math.hypot(x - TREE.x, z - TREE.z) < RADIUS + TREE.radius;
  }
  function segmentClear(a, b) {
    const n = Math.max(1, Math.ceil(Math.hypot(a.x - b.x, a.z - b.z) / .06));
    for (let i = 0; i <= n; ++i) {
      const t = i / n;
      if (blocked(a.x + (b.x - a.x) * t, a.z + (b.z - a.z) * t)) return false;
    }
    return true;
  }
  function path(from, to) {
    if (blocked(from.x, from.z) || blocked(to.x, to.z)) return [];
    if (segmentClear(from, to)) return [to];
    const key = (x, z) => `${x},${z}`;
    const sx = Math.round(from.x / STEP), sz = Math.round(from.z / STEP);
    const tx = Math.round(to.x / STEP), tz = Math.round(to.z / STEP);
    const target = key(tx, tz), start = key(sx, sz);
    const open = [{ x: sx, z: sz, g: 0, f: Math.hypot(tx - sx, tz - sz), id: start }];
    const scores = new Map([[start, 0]]), parents = new Map(), closed = new Set();
    const dirs = [[1,0],[-1,0],[0,1],[0,-1],[1,1],[1,-1],[-1,1],[-1,-1]];
    while (open.length) {
      open.sort((a, b) => a.f - b.f);
      const current = open.shift();
      if (closed.has(current.id)) continue;
      if (current.id === target) {
        const result = [{ x: tx * STEP, z: tz * STEP }];
        let id = current.id;
        while (id !== start) {
          const parent = parents.get(id);
          if (!parent) return [];
          id = parent.id;
          result.unshift({ x: parent.x * STEP, z: parent.z * STEP });
        }
        const full = [from, ...result, to];
        // A rounded start/target node can fall behind a thin obstacle. Validate
        // the exact endpoints as well as each graph edge before returning it.
        if (full.slice(1).some((p, i) => !segmentClear(full[i], p))) return [];
        const smooth = [];
        let anchor = 0;
        while (anchor < full.length - 1) {
          let next = full.length - 1;
          while (next > anchor + 1 && !segmentClear(full[anchor], full[next])) --next;
          smooth.push(full[next]); anchor = next;
        }
        return smooth;
      }
      closed.add(current.id);
      for (const [dx, dz] of dirs) {
        const x = current.x + dx, z = current.z + dz, id = key(x, z);
        if (closed.has(id) || blocked(x * STEP, z * STEP)) continue;
        const a = { x: current.x * STEP, z: current.z * STEP }, b = { x: x * STEP, z: z * STEP };
        if (!segmentClear(a, b)) continue;
        const g = current.g + Math.hypot(dx, dz);
        if (g >= (scores.get(id) ?? Infinity)) continue;
        scores.set(id, g); parents.set(id, current);
        open.push({ x, z, g, f: g + Math.hypot(tx - x, tz - z), id });
      }
    }
    return [];
  }
  const props = [
    ...[4,5,6].flatMap(x => [
      {id:`north-${x}`,sprite:'wall-x',x,z:3},
      {id:`south-${x}`,sprite:'wall-x',x,z:7}
    ]),
    ...[4,5,6].flatMap(z => [
      {id:`west-${z}`,sprite:'wall-z',x:3,z},
      {id:`east-${z}`,sprite:'wall-z',x:7,z}
    ]),
    {id:'corner-NW',sprite:'corner-0',x:3,z:3},
    {id:'corner-NE',sprite:'corner-270',x:7,z:3},
    {id:'corner-SE',sprite:'corner-180',x:7,z:7},
    {id:'corner-SW',sprite:'corner-90',x:3,z:7},
    { id: 'tree', sprite: 'tree', x: TREE.x, z: TREE.z }
  ];
  function ordered(hero) {
    return [...props, { ...hero, id: 'character', sprite: 'character' }]
      .sort((a,b) => (a.z - a.x) - (b.z - b.x) || a.id.localeCompare(b.id));
  }
  const patrol = [
    { x: 4.6, z: 2.1 }, { x: 2.1, z: 2.1 },
    { x: 2.1, z: 7.9 }, { x: 5.8, z: 8.0 },
    { x: 8.2, z: 8.7 }, { x: 9.6, z: 7.5 },
    { x: 8.1, z: 6.5 }, { x: 8.2, z: 2.1 }
  ];
  return { HALF_W, HALF_H, VERTICAL, SIZE, ROOM, WALLS, TREE, RADIUS,
    project, unproject, blocked, segmentClear, path, props, ordered, patrol };
})();
