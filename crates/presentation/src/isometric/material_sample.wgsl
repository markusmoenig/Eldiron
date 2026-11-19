// Include with read-only material_a/material_b arrays of vec4<u32>. Their
// immutable format is owned by material_sources.rs; no viewport bytes enter it.
fn material_word(which:u32,index:u32)->vec4<u32> {
    if which==0u {return material_a[index];}
    return material_b[index];
}
