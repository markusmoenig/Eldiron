#!/usr/bin/env python3
from pathlib import Path
import json,re
r=Path(__file__).resolve().parents[1];m=json.loads((r/'shaders/library.json').read_text())
lines=['// Generated from the curated source manifest by scripts/generate-library.py.','pub(crate) const TILES: &[(&str, &str, bool, &str)] = &[']
for x in m:lines.append(f'    (include_str!("../shaders/tiles/{x["stem"]}.tile"), include_str!("../shaders/original/{x["file"]}"), {str(x["input"]).lower()}, "{x.get("id",x["stem"])}"),')
lines.append('];');(r/'src/library.rs').write_text('\n'.join(lines)+'\n')
p=r/'src/metadata.rs';s=p.read_text();a=s.index('    pub const ALL:');b=s.index('    pub fn original',a);base=['Perlin','Disc','Multiply','ColorBlend','Transform','Circle','RoundedRectangle','Color','Border','PeriodicProfile','CompareSelect','CellData','DirectionalGrain','HeightToNormal'];s=s[:a]+f'    pub const ALL: [Self; {len(base)+len(m)}] = [\n'+''.join(f'        Self::{x},\n' for x in base)+''.join(f'        Self::Library({i}),\n' for i in range(len(m)))+'    ];\n'+s[b:];p.write_text(s)
