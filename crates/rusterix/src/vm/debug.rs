use crate::vm::VMValue;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EldrinDebugTarget {
    World,
    Region(u32),
    Entity(u32),
    Item(u32),
}

#[derive(Clone, Debug)]
pub enum EldrinDebugEntry {
    ExecutedLine {
        line: usize,
    },
    Value {
        line: usize,
        name: String,
        value: VMValue,
    },
    Branch {
        line: usize,
        taken: bool,
    },
}

#[derive(Clone, Debug)]
pub struct EldrinDebugFrame {
    pub target: EldrinDebugTarget,
    pub function: String,
    pub entries: Vec<EldrinDebugEntry>,
}

#[derive(Clone, Debug, Default)]
pub struct EldrinDebugModule {
    pub frames: Vec<EldrinDebugFrame>,
}

impl EldrinDebugModule {
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn merge(&mut self, other: &EldrinDebugModule) {
        self.frames.extend(other.frames.iter().cloned());
    }

    pub fn begin_invocation(&mut self, target: EldrinDebugTarget, function: &str) {
        self.frames.push(EldrinDebugFrame {
            target,
            function: function.to_string(),
            entries: vec![],
        });
    }

    pub fn latest_frame_for(&self, target: &EldrinDebugTarget) -> Option<&EldrinDebugFrame> {
        self.frames
            .iter()
            .rev()
            .find(|frame| &frame.target == target)
    }

    pub fn mark_executed(&mut self, target: EldrinDebugTarget, function: &str, line: usize) {
        self.frame_mut(target, function)
            .entries
            .push(EldrinDebugEntry::ExecutedLine { line });
    }

    pub fn add_value(
        &mut self,
        target: EldrinDebugTarget,
        function: &str,
        line: usize,
        name: impl Into<String>,
        value: VMValue,
    ) {
        self.frame_mut(target, function)
            .entries
            .push(EldrinDebugEntry::Value {
                line,
                name: name.into(),
                value,
            });
    }

    pub fn mark_branch(
        &mut self,
        target: EldrinDebugTarget,
        function: &str,
        line: usize,
        taken: bool,
    ) {
        self.frame_mut(target, function)
            .entries
            .push(EldrinDebugEntry::Branch { line, taken });
    }

    pub fn latest_line_for(&self, target: &EldrinDebugTarget) -> Option<usize> {
        self.latest_frame_for(target)?
            .entries
            .iter()
            .filter_map(|entry| match entry {
                EldrinDebugEntry::ExecutedLine { line }
                | EldrinDebugEntry::Value { line, .. }
                | EldrinDebugEntry::Branch { line, .. } => Some(*line),
            })
            .last()
    }

    pub fn latest_values_for(&self, target: &EldrinDebugTarget) -> Vec<(usize, String, VMValue)> {
        self.latest_frame_for(target)
            .map(|frame| {
                frame
                    .entries
                    .iter()
                    .filter_map(|entry| match entry {
                        EldrinDebugEntry::Value { line, name, value } => {
                            Some((*line, name.clone(), value.clone()))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn frame_mut(&mut self, target: EldrinDebugTarget, function: &str) -> &mut EldrinDebugFrame {
        if let Some(index) = self
            .frames
            .iter()
            .rposition(|frame| frame.target == target && frame.function == function)
        {
            return self.frames.get_mut(index).unwrap();
        }
        self.frames.push(EldrinDebugFrame {
            target,
            function: function.to_string(),
            entries: vec![],
        });
        self.frames.last_mut().unwrap()
    }
}
