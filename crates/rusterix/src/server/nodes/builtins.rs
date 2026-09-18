use super::*;
pub(super) fn register(registry: &mut Registry) {
    registry.register(Box::new(LookoutModule)).unwrap();
    registry.register(Box::new(EngageModule)).unwrap();
    registry.register(Box::new(TimeRangeModule)).unwrap();
    registry.register(Box::new(GoToModule)).unwrap();
    registry.register(Box::new(EventModule)).unwrap();
    registry.register(Box::new(SayModule)).unwrap();
    registry.register(Box::new(FilterModule)).unwrap();
    registry.register(Box::new(PlayerCameraModule)).unwrap();
    registry.register(Box::new(RoutineModule)).unwrap();
    registry.register(Box::new(RandomWalkModule)).unwrap();
    registry.register(Box::new(ResumeRoutineModule)).unwrap();
}
fn text(p: &BTreeMap<String, Value>, key: &str) -> Result<String, String> {
    p.get(key)
        .and_then(|v| v.get("Text"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("Missing text parameter {key}"))
}
struct EventModule;
impl NodeModule for EventModule {
    fn id(&self) -> &'static str {
        "event"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let value = p
            .get("event")
            .and_then(|v| v.get("Custom"))
            .ok_or("Missing event parameter")?;
        if value.get("kind").and_then(Value::as_str) != Some("event") {
            return Err("Invalid event parameter".into());
        }
        let name = value
            .get("data")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("Missing event name")?;
        Ok(Box::new(Event(name.into())))
    }
}
struct Event(String);
impl Operation for Event {
    fn event(&self) -> Option<&str> {
        Some(&self.0)
    }
    fn execute(
        &self,
        _: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok("out")
    }
}
struct SayModule;
impl NodeModule for SayModule {
    fn id(&self) -> &'static str {
        "say"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Say(text(p, "text")?)))
    }
}
struct Say(String);
impl Operation for Say {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let mut text = String::new();
        let mut rest = self.0.as_str();
        while let Some(start) = rest.find("{event.") {
            text.push_str(&rest[..start]);
            let value = &rest[start + 7..];
            let end = value.find('}').ok_or("Unclosed event reference")?;
            let key = &value[..end];
            text.push_str(
                &ctx.event
                    .fields
                    .get(key)
                    .ok_or_else(|| format!("Event has no value '{key}'"))?
                    .display(),
            );
            rest = &value[end + 1..];
        }
        text.push_str(rest);
        world.say(ctx.actor, text);
        Ok("out")
    }
}
struct FilterModule;
impl NodeModule for FilterModule {
    fn id(&self) -> &'static str {
        "filter"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["match", "no_match"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let field = text(p, "field")?;
        let key = field
            .strip_prefix("event.")
            .filter(|s| !s.is_empty())
            .ok_or("Only event fields are supported in this slice")?
            .to_owned();
        let op = p
            .get("operator")
            .and_then(|v| v.pointer("/Choice/selected"))
            .and_then(Value::as_u64)
            .filter(|n| *n < 5)
            .ok_or("Invalid filter operator")?;
        Ok(Box::new(Filter {
            key,
            op,
            expected: text(p, "expected")?,
        }))
    }
}
struct Filter {
    key: String,
    op: u64,
    expected: String,
}
impl Operation for Filter {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let value = ctx
            .event
            .fields
            .get(&self.key)
            .ok_or_else(|| format!("Event has no value '{}'", self.key))?;
        let matched = match self.op {
            0 => value.display() == self.expected,
            1 => value.display() != self.expected,
            2 => value.display().contains(&self.expected),
            _ => {
                let EventField::Number(n) = value else {
                    return Err("Numeric comparison requires a number".into());
                };
                let rhs: f64 = self.expected.parse().map_err(|_| "Expected a number")?;
                if self.op == 3 { *n > rhs } else { *n < rhs }
            }
        };
        Ok(if matched { "match" } else { "no_match" })
    }
}

struct PlayerCameraModule;
impl NodeModule for PlayerCameraModule {
    fn id(&self) -> &'static str {
        "player_camera"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let selected = p
            .get("camera")
            .and_then(|v| v.pointer("/Choice/selected"))
            .and_then(Value::as_u64)
            .ok_or("Missing camera selection")?;
        let camera = ["2d", "2d_grid", "iso", "firstp", "firstp_grid"]
            .get(selected as usize)
            .ok_or("Invalid camera selection")?;
        Ok(Box::new(PlayerCamera(*camera)))
    }
}
struct PlayerCamera(&'static str);
impl Operation for PlayerCamera {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.player_camera(ctx.actor, self.0)?;
        Ok("out")
    }
}

struct RoutineModule;
impl NodeModule for RoutineModule {
    fn id(&self) -> &'static str {
        "routine"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, _: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Routine))
    }
}
struct Routine;
impl Operation for Routine {
    fn event(&self) -> Option<&str> {
        Some("routine")
    }
    fn execute(
        &self,
        _: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok("out")
    }
}
fn number(p: &BTreeMap<String, Value>, key: &str, min: f32, max: f32) -> Result<f32, String> {
    let value = p
        .get(key)
        .and_then(|v| v.pointer("/Number/value"))
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("Missing number parameter {key}"))? as f32;
    if !value.is_finite() || value < min || value > max {
        return Err(format!("{key} is outside its supported range"));
    }
    Ok(value)
}
struct RandomWalkModule;
impl NodeModule for RandomWalkModule {
    fn id(&self) -> &'static str {
        "random_walk"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["started", "outside_area"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(RandomWalk {
            area: text(p, "area")?,
            distance: number(p, "distance", 0.1, 10.)?,
            speed: number(p, "speed", 0.1, 10.)?,
            pause: number(p, "pause", 0., 60.)? as i32,
        }))
    }
}
struct RandomWalk {
    area: String,
    distance: f32,
    speed: f32,
    pause: i32,
}
impl Operation for RandomWalk {
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        (!world.random_walk_active(ctx.actor)).then_some(Ok("stopped"))
    }
    fn retains_activity(&self, output: &str) -> bool {
        output == "started"
    }
    fn refresh(&self, ctx: &EventContext<'_>, world: &mut dyn WorldServices) -> Result<(), String> {
        world.refresh_random_walk(ctx.actor, &self.area, self.distance, self.speed, self.pause)
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok(
            if world.random_walk(ctx.actor, &self.area, self.distance, self.speed, self.pause)? {
                "started"
            } else {
                "outside_area"
            },
        )
    }
}
struct ResumeRoutineModule;
impl NodeModule for ResumeRoutineModule {
    fn id(&self) -> &'static str {
        "resume_routine"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, _: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(ResumeRoutine))
    }
}
struct ResumeRoutine;
impl Operation for ResumeRoutine {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        if ctx.event.name == "routine" {
            return Err("Resume Routine cannot call itself from a routine chain".into());
        }
        world.resume_routine(ctx.actor)?;
        Ok("out")
    }
}

/// Shared by runtime evaluation and Creator context feedback.
pub fn time_range_contains(
    start: &str,
    end: &str,
    time: theframework::prelude::TheTime,
) -> Result<bool, String> {
    fn minutes(value: &str) -> Result<u16, String> {
        let (h, m) = value.trim().split_once(':').ok_or("Use HH:MM for time")?;
        let h: u16 = h.parse().map_err(|_| "Invalid hour")?;
        let m: u16 = m.parse().map_err(|_| "Invalid minute")?;
        if h > 23 || m > 59 {
            return Err("Time must be between 00:00 and 23:59".into());
        }
        Ok(h * 60 + m)
    }
    let (start, end) = (minutes(start)?, minutes(end)?);
    let now = time.hours as u16 * 60 + time.minutes as u16;
    Ok(if start == end {
        true
    } else if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    })
}
struct TimeRangeModule;
impl NodeModule for TimeRangeModule {
    fn id(&self) -> &'static str {
        "time_range"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["inside", "outside"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let range = TimeRange {
            start: text(p, "start")?,
            end: text(p, "end")?,
        };
        time_range_contains(&range.start, &range.end, Default::default())?;
        Ok(Box::new(range))
    }
}
struct TimeRange {
    start: String,
    end: String,
}
impl Operation for TimeRange {
    fn condition(&self, time: theframework::prelude::TheTime) -> Option<bool> {
        time_range_contains(&self.start, &self.end, time).ok()
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok(if self.condition(ctx.time) == Some(true) {
            "inside"
        } else {
            "outside"
        })
    }
}
struct GoToModule;
impl NodeModule for GoToModule {
    fn id(&self) -> &'static str {
        "go_to"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["done", "error"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(GoTo {
            destination: text(p, "destination")?,
            speed: number(p, "speed", 0.1, 10.)?,
        }))
    }
}
struct GoTo {
    destination: String,
    speed: f32,
}
impl Operation for GoTo {
    fn retains_activity(&self, output: &str) -> bool {
        output == "running"
    }
    fn error_output(&self) -> Option<&'static str> {
        Some("error")
    }
    fn refresh(&self, ctx: &EventContext<'_>, world: &mut dyn WorldServices) -> Result<(), String> {
        let result = world.go_to(ctx.actor, &self.destination, self.speed);
        if result.is_err() {
            world.cancel_activity(ctx.actor);
        }
        result
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        world
            .go_to_result(ctx.actor)
            .map(|result| result.map(|()| "done"))
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.go_to(ctx.actor, &self.destination, self.speed)?;
        Ok("running")
    }
}

fn profile(p: &BTreeMap<String, Value>) -> Result<String, String> {
    let value = p
        .get("profile")
        .and_then(|v| v.get("Choice"))
        .ok_or("Missing ruleset profile")?;
    let index = value
        .get("selected")
        .and_then(Value::as_u64)
        .ok_or("Missing profile selection")? as usize;
    value
        .get("options")
        .and_then(Value::as_array)
        .and_then(|options| options.get(index))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Select a ruleset profile".into())
}
struct LookoutModule;
impl NodeModule for LookoutModule {
    fn id(&self) -> &'static str {
        "lookout"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["found", "watching"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Lookout(profile(p)?)))
    }
}
struct Lookout(String);
impl Operation for Lookout {
    fn watch_event(&self) -> Option<&'static str> {
        Some("lookout")
    }
    fn interrupts_on(&self, output: &str) -> bool {
        output == "found"
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok(if world.lookout(ctx.actor, &self.0)? {
            "found"
        } else {
            "watching"
        })
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        match world.lookout(ctx.actor, &self.0) {
            Ok(true) => Some(Ok("found")),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        }
    }
}
struct EngageModule;
impl NodeModule for EngageModule {
    fn id(&self) -> &'static str {
        "engage"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["defeated", "lost", "cannot_engage"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Engage(profile(p)?)))
    }
}
struct Engage(String);
impl Operation for Engage {
    fn retains_activity(&self, output: &str) -> bool {
        output == "running"
    }
    fn error_output(&self) -> Option<&'static str> {
        Some("cannot_engage")
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.engage_start(ctx.actor, &self.0)?;
        Ok("running")
    }
    fn refresh(&self, ctx: &EventContext<'_>, world: &mut dyn WorldServices) -> Result<(), String> {
        world.engage_start(ctx.actor, &self.0)
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        match world.engage_tick(ctx.actor, &self.0) {
            Ok(Some(output)) => Some(Ok(output)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}
