use super::NodeOp;

fn _peephole_locals(ops: &mut Vec<NodeOp>) {
    let mut i = 0;
    while i + 1 < ops.len() {
        match (&ops[i], &ops[i + 1]) {
            (NodeOp::StoreLocal(a), NodeOp::LoadLocal(b)) if a == b => {
                ops.drain(i..=i + 1);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
}

pub fn optimize(_ops: &mut Vec<NodeOp>) {
    // peephole_locals(ops);
}
