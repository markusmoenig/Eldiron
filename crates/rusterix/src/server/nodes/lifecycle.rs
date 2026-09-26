//! Lifecycle and targeting nodes use the same world state as native combat.
use super::*;
pub(super) fn register(registry: &mut Registry) {
    registry
        .register(Box::new(Module { target: false }))
        .unwrap();
    registry
        .register(Box::new(Module { target: true }))
        .unwrap();
}
struct Module {
    target: bool,
}
impl NodeModule for Module {
    fn id(&self) -> &'static str {
        if self.target {
            "set_target"
        } else {
            "respawn_character"
        }
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["done", "failed"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Action {
            target: self.target,
            field: p
                .get("field")
                .and_then(|v| v.get("Text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .into(),
        }))
    }
}
struct Action {
    target: bool,
    field: String,
}
impl Operation for Action {
    fn error_output(&self) -> Option<&'static str> {
        Some("failed")
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        if self.target {
            let target = if self.field.trim().is_empty() {
                None
            } else {
                match ctx.event.fields.get(
                    self.field
                        .trim()
                        .strip_prefix("event.")
                        .unwrap_or(self.field.trim()),
                ) {
                    Some(EventField::Entity(id)) => Some(*id),
                    _ => return Err("Set Target needs an event entity field".into()),
                }
            };
            world.set_target(ctx.actor, target)?;
        } else {
            world.respawn(ctx.actor)?;
        }
        Ok("done")
    }
}
