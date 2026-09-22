//! Branch discovery for behavior graphs.
//!
//! A branch is one trigger (`Event`, `On Event`, `Routine`, `On Area`, …) and
//! everything reachable downstream from it. The editor filters the canvas to one
//! branch at a time so a large graph stays readable, and uses the same model to
//! add and remove branches.

use super::*;
use std::collections::{HashMap, HashSet};

/// One trigger and everything reachable from it.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphBranch {
    pub root: GraphId,
    pub nodes: HashSet<GraphId>,
}

/// The branches of a graph, plus nodes no trigger reaches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphBranches {
    pub branches: Vec<GraphBranch>,
    pub detached: HashSet<GraphId>,
}

/// Map every port to the node that owns it.
fn port_owners(doc: &GraphDocument) -> HashMap<GraphId, GraphId> {
    let mut owners = HashMap::new();
    for node in &doc.nodes {
        for port in &node.ports {
            owners.insert(port.id, node.id);
        }
    }
    owners
}

/// Node-to-node adjacency along connections.
fn adjacency(doc: &GraphDocument, owners: &HashMap<GraphId, GraphId>) -> HashMap<GraphId, Vec<GraphId>> {
    let mut edges: HashMap<GraphId, Vec<GraphId>> = HashMap::new();
    for connection in &doc.connections {
        if let (Some(from), Some(to)) = (
            owners.get(&connection.from),
            owners.get(&connection.to),
        ) {
            edges.entry(*from).or_default().push(*to);
        }
    }
    edges
}

/// True when a node's definition has no inputs, so it can only start a branch.
pub fn is_branch_trigger(node: &GraphNode, definitions: &GraphDefinitions) -> bool {
    node.definition
        .as_deref()
        .and_then(|id| definitions.node(id))
        .is_some_and(|definition| {
            !definition
                .ports
                .iter()
                .any(|port| port.direction == PortDirection::Input)
        })
}

/// Every branch in the document, in document order.
pub fn graph_branches(doc: &GraphDocument, definitions: &GraphDefinitions) -> GraphBranches {
    let owners = port_owners(doc);
    let edges = adjacency(doc, &owners);
    let roots: Vec<GraphId> = doc
        .nodes
        .iter()
        .filter(|node| is_branch_trigger(node, definitions))
        .map(|node| node.id)
        .collect();

    let mut branches = Vec::new();
    let mut reached: HashSet<GraphId> = HashSet::new();
    for root in roots {
        let mut seen = HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(next) = edges.get(&id) {
                stack.extend(next.iter().copied());
            }
        }
        reached.extend(seen.iter().copied());
        branches.push(GraphBranch { root, nodes: seen });
    }

    let detached = doc
        .nodes
        .iter()
        .map(|node| node.id)
        .filter(|id| !reached.contains(id))
        .collect();
    GraphBranches { branches, detached }
}

impl GraphBranches {
    /// Nodes reachable from more than one trigger. Branches are meant to be
    /// separate, so this is what the editor warns about and cleanup targets.
    pub fn shared_nodes(&self) -> HashSet<GraphId> {
        let mut seen = HashSet::new();
        let mut shared = HashSet::new();
        for branch in &self.branches {
            for id in &branch.nodes {
                if !seen.insert(*id) {
                    shared.insert(*id);
                }
            }
        }
        shared
    }

    pub fn branch(&self, root: GraphId) -> Option<&GraphBranch> {
        self.branches.iter().find(|branch| branch.root == root)
    }
}

/// Horizontal gap between execution columns.
const COLUMN_GAP: f32 = 150.;
/// Vertical gap between nodes in one column.
const ROW_GAP: f32 = 60.;

/// Lay a node set out left to right: one column per execution step, stacked so
/// nodes never overlap, ordered to keep wires short. Returns true when anything
/// moved. Unrelated nodes keep their positions.
pub fn layout_branch(doc: &mut GraphDocument, nodes: &HashSet<GraphId>) -> bool {
    let ids: Vec<GraphId> = doc
        .nodes
        .iter()
        .map(|node| node.id)
        .filter(|id| nodes.contains(id))
        .collect();
    if ids.is_empty() {
        return false;
    }
    let index: HashMap<GraphId, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    // Port-to-node, restricted to the set.
    let mut owner: HashMap<GraphId, usize> = HashMap::new();
    for node in doc.nodes.iter() {
        if let Some(slot) = index.get(&node.id) {
            for port in &node.ports {
                owner.insert(port.id, *slot);
            }
        }
    }
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); ids.len()];
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); ids.len()];
    for connection in &doc.connections {
        if let (Some(from), Some(to)) = (owner.get(&connection.from), owner.get(&connection.to)) {
            if from != to && !successors[*from].contains(to) {
                successors[*from].push(*to);
                predecessors[*to].push(*from);
            }
        }
    }

    // Column = longest path from a trigger. A dialogue tree loops back to an
    // earlier node when a choice re-opens the menu, and following that loop
    // would push its nodes further right on every pass, so the back edges are
    // found and dropped first. The rest is a DAG and settles in one sweep per
    // node.
    let back = back_edges(&successors);
    let mut depth = vec![0usize; ids.len()];
    for _ in 0..ids.len() {
        let mut changed = false;
        for from in 0..ids.len() {
            for &to in &successors[from] {
                if back[from].contains(&to) {
                    continue;
                }
                if depth[to] < depth[from] + 1 {
                    depth[to] = depth[from] + 1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); max_depth + 1];
    for (i, d) in depth.iter().enumerate() {
        columns[*d].push(i);
    }

    // Order each column by the average position of its parents, which keeps the
    // left-to-right wires from crossing more than necessary.
    let mut rank: Vec<f32> = (0..ids.len()).map(|i| i as f32).collect();
    for column in columns.iter_mut().skip(1) {
        let mut keyed: Vec<(usize, f32)> = column
            .iter()
            .map(|&i| {
                let parents = &predecessors[i];
                let key = if parents.is_empty() {
                    rank[i]
                } else {
                    parents.iter().map(|&p| rank[p]).sum::<f32>() / parents.len() as f32
                };
                (i, key)
            })
            .collect();
        keyed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        column.clear();
        column.extend(keyed.into_iter().map(|(i, _)| i));
        for (position, &i) in column.iter().enumerate() {
            rank[i] = position as f32;
        }
    }

    let slot: HashMap<GraphId, usize> = doc
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id, i))
        .collect();
    let metrics = doc.metrics();
    let height = |i: usize| doc.nodes[slot[&ids[i]]].height(&metrics);
    let width = |i: usize| doc.nodes[slot[&ids[i]]].width;

    // Each column sits just past the widest node of the one before it, so mixed
    // sizes pack tightly instead of leaving the gap the widest node needs.
    let mut column_x = vec![0.; columns.len()];
    for column in 1..columns.len() {
        let previous = columns[column - 1]
            .iter()
            .map(|&i| width(i))
            .fold(0., f32::max);
        column_x[column] = column_x[column - 1] + previous + COLUMN_GAP;
    }

    let mut positions = vec![[0., 0.]; ids.len()];
    for (column, members) in columns.iter().enumerate() {
        let total = members.iter().map(|&i| height(i)).sum::<f32>()
            + ROW_GAP * members.len().saturating_sub(1) as f32;
        let mut y = -total * 0.5;
        for &i in members {
            positions[i] = [column_x[column], y];
            y += height(i) + ROW_GAP;
        }
    }

    let mut moved = false;
    for (i, id) in ids.iter().enumerate() {
        if let Some(node) = doc.nodes.iter_mut().find(|node| node.id == *id)
            && node.position != positions[i]
        {
            node.position = positions[i];
            moved = true;
        }
    }
    moved
}

/// Edges that close a cycle: found by a depth first walk, an edge to a node that
/// is still on the walk stack is a back edge. `back[from]` lists them.
fn back_edges(successors: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut back = vec![Vec::new(); successors.len()];
    // 0 = unvisited, 1 = on the walk stack, 2 = finished.
    let mut state = vec![0u8; successors.len()];
    for start in 0..successors.len() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, cursor)) = stack.pop() {
            if cursor < successors[node].len() {
                stack.push((node, cursor + 1));
                let next = successors[node][cursor];
                match state[next] {
                    0 => {
                        state[next] = 1;
                        stack.push((next, 0));
                    }
                    1 => back[node].push(next),
                    _ => {}
                }
            } else {
                state[node] = 2;
            }
        }
    }
    back
}
