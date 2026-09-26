//! A conversation authored inside a `Talk` node.
//!
//! The whole dialogue tree lives in the node as data, so the canvas stays a flow
//! chart instead of a drawing of the script. A step is a line and the choices are
//! the answers the listener may give.
//!
//! What a choice *does* is not a script. It either moves inside the tree, ends the
//! conversation, or leaves through one of the node's output ports, where ordinary
//! nodes — Say, Set Attribute, Add Item, Offer Inventory and so on — do the work.
//! A condition is the one small language, because it is a predicate that decides
//! which answers to offer, not flow.

use serde::{Deserialize, Serialize};

/// Output ports a Talk node offers for choice consequences.
pub const CONSEQUENCE_SLOTS: usize = 6;
/// The keys of those ports, in slot order.
pub const CONSEQUENCE_PORTS: [&str; CONSEQUENCE_SLOTS] =
    ["out:0", "out:1", "out:2", "out:3", "out:4", "out:5"];

/// Steps, and where the conversation starts. Stored in the node's
/// `conversation` row as `Custom { kind: "conversation" }`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// One thing the speaker says, and what the listener may answer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// Unique name, which a `go` jumps to.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    #[serde(default)]
    pub label: String,
    /// Empty means always offered. Uses the dialogue condition language,
    /// including `has("Item")` for inventory.
    #[serde(default)]
    pub condition: String,
    /// Where picking this answer leads.
    #[serde(default)]
    pub then: Then,
}

/// What a choice does when it is picked.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Then {
    /// The conversation ends here.
    #[default]
    End,
    /// Continue at another step of this conversation, without leaving the node.
    Go {
        #[serde(default)]
        step: String,
    },
    /// Leave through `out:slot`, so the nodes wired to that port run. A choice's
    /// consequence is graph work, not text. When the chain hands the flow back to
    /// this node, `resume` is the step it continues at; empty ends the
    /// conversation.
    Out {
        #[serde(default)]
        slot: usize,
        #[serde(default)]
        resume: String,
    },
}

impl Conversation {
    /// Read the conversation out of the control value held in a node row.
    pub fn from_control(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.get("Custom")?.get("data")?.clone()).ok()
    }

    pub fn step(&self, name: &str) -> Option<&Step> {
        self.steps.iter().find(|step| step.name == name)
    }

    /// The step a conversation starts on: the named entry, or the first one.
    pub fn entry_step(&self) -> Option<&Step> {
        self.step(self.entry.trim()).or_else(|| self.steps.first())
    }

    /// A minimal conversation, so a freshly dropped Talk node still compiles
    /// and its author has something to edit.
    pub fn starter() -> Self {
        Self {
            entry: "start".into(),
            steps: vec![Step {
                name: "start".into(),
                text: "Hello.".into(),
                choices: vec![Choice {
                    label: "Leave".into(),
                    condition: String::new(),
                    then: Then::End,
                }],
            }],
        }
    }

    /// Names that a `go` could mean, for the editor and for diagnostics.
    pub fn step_names(&self) -> Vec<&str> {
        self.steps.iter().map(|step| step.name.as_str()).collect()
    }

    /// A conversation as one readable line, for the node body and diagnostics.
    pub fn describe(&self) -> String {
        let choices: usize = self.steps.iter().map(|step| step.choices.len()).sum();
        let plural = |count: usize, one: &str, many: &str| {
            if count == 1 {
                format!("{count} {one}")
            } else {
                format!("{count} {many}")
            }
        };
        format!(
            "{}, {}",
            plural(self.steps.len(), "step", "steps"),
            plural(choices, "choice", "choices")
        )
    }

    /// Check the tree, so a broken jump or a bad port shows as a node diagnostic
    /// instead of failing for one unlucky player later.
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("Talk conversation has no steps".into());
        }
        if !self.entry.trim().is_empty() && self.step(self.entry.trim()).is_none() {
            return Err(format!(
                "Talk entry step '{}' does not exist",
                self.entry.trim()
            ));
        }
        let mut names = std::collections::HashSet::new();
        for step in &self.steps {
            let name = step.name.trim();
            if name.is_empty() {
                return Err("Talk has a step without a name".into());
            }
            if !names.insert(name) {
                return Err(format!("Talk has duplicate step '{name}'"));
            }
            if step.choices.is_empty() {
                return Err(format!("Talk step '{name}' needs a way out"));
            }
            for choice in &step.choices {
                if choice.label.trim().is_empty() {
                    return Err(format!("Talk step '{name}' has a choice without a label"));
                }
                match &choice.then {
                    Then::Go { step } if self.step(step.trim()).is_none() => {
                        return Err(format!("Talk jump to unknown step '{step}'"));
                    }
                    Then::Out { slot, .. } if *slot >= CONSEQUENCE_SLOTS => {
                        return Err(format!("Talk has no output 'out:{slot}'"));
                    }
                    Then::Out { resume, .. }
                        if !resume.trim().is_empty() && self.step(resume.trim()).is_none() =>
                    {
                        return Err(format!("Talk resumes at unknown step '{resume}'"));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
