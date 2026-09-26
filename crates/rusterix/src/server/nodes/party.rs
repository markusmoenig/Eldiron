//! Party and health nodes use world services; they do not dispatch scripts.
use super::*;
pub(super) fn register(registry: &mut Registry) {
    registry
        .register(Box::new(PartyModule { leave: false }))
        .unwrap();
    registry
        .register(Box::new(PartyModule { leave: true }))
        .unwrap();
    registry.register(Box::new(HealthModule)).unwrap();
}
struct PartyModule {
    leave: bool,
}
impl NodeModule for PartyModule {
    fn id(&self) -> &'static str {
        if self.leave {
            "leave_party"
        } else {
            "join_party"
        }
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["done", "failed"]
    }
    fn compile(&self, _: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Party { leave: self.leave }))
    }
}
struct Party {
    leave: bool,
}
impl Operation for Party {
    fn error_output(&self) -> Option<&'static str> {
        Some("failed")
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let done = if self.leave {
            world.leave_party(ctx.actor)?
        } else {
            let Some(EventField::Entity(leader)) = ctx.event.fields.get("subject") else {
                return Err("Join Party needs event.subject as the leader".into());
            };
            world.join_party(ctx.actor, *leader)?
        };
        Ok(if done { "done" } else { "failed" })
    }
}
struct HealthModule;
impl NodeModule for HealthModule {
    fn id(&self) -> &'static str {
        "health_check"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["match", "not_match"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let percent = p
            .get("percent")
            .and_then(|v| v.get("Number"))
            .and_then(|v| v.get("value"))
            .and_then(Value::as_f64)
            .ok_or("Health Check needs a percentage")?;
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            return Err("Health percentage must be between 0 and 100".into());
        }
        Ok(Box::new(Health(percent)))
    }
}
struct Health(f64);
impl Operation for Health {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let Some(EventField::Entity(subject)) = ctx.event.fields.get("subject") else {
            return Err("Health Check needs event.subject".into());
        };
        Ok(if world.health_below(*subject, self.0)? {
            "match"
        } else {
            "not_match"
        })
    }
}
