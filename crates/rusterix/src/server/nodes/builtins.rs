use super::*;
pub(super) fn register(registry: &mut Registry) {
    super::party::register(registry);
    super::lifecycle::register(registry);
    registry.register(Box::new(QuestStateModule)).unwrap();
    registry.register(Box::new(SetQuestModule)).unwrap();
    registry.register(Box::new(QuestGuardModule)).unwrap();
    registry.register(Box::new(ItemGuardModule)).unwrap();
    registry
        .register(Box::new(PlayerAttributeGuardModule))
        .unwrap();
    registry.register(Box::new(UseActionModule)).unwrap();
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
    registry.register(Box::new(SetAttributeModule)).unwrap();
    registry.register(Box::new(TeleportModule)).unwrap();
    registry.register(Box::new(MessageModule)).unwrap();
    registry.register(Box::new(StateModule)).unwrap();
    registry.register(Box::new(OnEnterAreaModule)).unwrap();
    registry.register(Box::new(OnAreaModule)).unwrap();
    registry.register(Box::new(SetEmitLightModule)).unwrap();
    registry.register(Box::new(OnEventModule)).unwrap();
    registry.register(Box::new(NotifyInModule)).unwrap();
    registry.register(Box::new(EntitiesInRadiusModule)).unwrap();
    registry.register(Box::new(DialogModule)).unwrap();
    registry.register(Box::new(DialogueModule)).unwrap();
    registry.register(Box::new(PromptModule)).unwrap();
    registry.register(Box::new(TalkModule)).unwrap();
    registry.register(Box::new(InventoryHasModule)).unwrap();
    registry.register(Box::new(OfferInventoryModule)).unwrap();
    registry.register(Box::new(AddItemModule)).unwrap();
    registry.register(Box::new(DropItemsModule)).unwrap();
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
        let text = render_event_template(ctx, &self.0)?;
        world.say(ctx.actor, text);
        Ok("out")
    }
}
/// Expands `{event.field}` references against the current observation.
fn render_event_template(ctx: &EventContext<'_>, template: &str) -> Result<String, String> {
    let mut text = String::new();
    let mut rest = template;
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
    Ok(text)
}
struct SetAttributeModule;
impl NodeModule for SetAttributeModule {
    fn id(&self) -> &'static str {
        "set_attribute"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let attribute = text(p, "attribute")?;
        if attribute.trim().is_empty() {
            return Err("Set Attribute needs an attribute name".into());
        }
        let selected = p
            .get("value_kind")
            .and_then(|v| v.pointer("/Choice/selected"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let raw = text(p, "value")?;
        let toggle = selected == 3;
        let value = match selected {
            1 => EventAttribute::Number(
                raw.trim()
                    .parse()
                    .map_err(|_| "Set Attribute expects a number".to_string())?,
            ),
            2 => EventAttribute::Bool(matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "true" | "yes" | "on" | "1"
            )),
            _ => EventAttribute::Text(raw),
        };
        Ok(Box::new(SetAttribute {
            attribute,
            value,
            toggle,
        }))
    }
}
struct SetAttribute {
    attribute: String,
    value: EventAttribute,
    /// Toggle ignores the value and flips the attribute's current boolean.
    toggle: bool,
}
impl Operation for SetAttribute {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        if self.toggle {
            world.toggle_attribute(ctx.actor, self.attribute.trim())?;
        } else {
            world.set_attribute(ctx.actor, self.attribute.trim(), self.value.clone())?;
        }
        Ok("out")
    }
}
/// Keeps an actor's light in step with its active state, which is what an on and
/// off item such as a torch needs.
struct SetEmitLightModule;
impl NodeModule for SetEmitLightModule {
    fn id(&self) -> &'static str {
        "set_emit_light"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(SetEmitLight {
            mode: selection(p, "emit").unwrap_or_else(|_| "Follow Active".into()),
        }))
    }
}
struct SetEmitLight {
    mode: String,
}
impl Operation for SetEmitLight {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.set_emit_light(ctx.actor, &self.mode)?;
        Ok("out")
    }
}
struct TeleportModule;
impl NodeModule for TeleportModule {
    fn id(&self) -> &'static str {
        "teleport"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let area = text(p, "area")?;
        if area.trim().is_empty() {
            return Err("Teleport needs a destination area".into());
        }
        Ok(Box::new(Teleport {
            area,
            sector: text(p, "sector").unwrap_or_default(),
        }))
    }
}
struct Teleport {
    area: String,
    sector: String,
}
impl Operation for Teleport {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.teleport(ctx.actor, self.area.trim(), self.sector.trim())?;
        Ok("out")
    }
}
/// Gives the character an item from the ruleset's item templates, so a kill
/// can leave something behind.
struct AddItemModule;
impl NodeModule for AddItemModule {
    fn id(&self) -> &'static str {
        "add_item"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(AddItem {
            item: text(p, "item")?,
        }))
    }
}
struct AddItem {
    item: String,
}
impl Operation for AddItem {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.add_item(ctx.actor, &self.item)?;
        Ok("out")
    }
}
/// Drops what the character carries, either into the ruleset loot container or
/// on the ground where it stands.
struct DropItemsModule;
impl NodeModule for DropItemsModule {
    fn id(&self) -> &'static str {
        "drop_items"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        // An empty filter means everything the character carries.
        Ok(Box::new(DropItems {
            filter: text(p, "filter").unwrap_or_default(),
        }))
    }
}
struct DropItems {
    filter: String,
}
impl Operation for DropItems {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.drop_items(ctx.actor, &self.filter)?;
        Ok("out")
    }
}
/// Entry for the graph a named place owns. The place is the owner, so this node
/// routes the whole event: the transition *and* who caused it. Only one terminal
/// fires, so a terminal is a conjunction — wire the ones you care about.
struct OnAreaModule;
impl NodeModule for OnAreaModule {
    fn id(&self) -> &'static str {
        "on_area"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["player_entered", "npc_entered", "player_left", "npc_left"]
    }
    fn compile(&self, _: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(OnArea))
    }
}
struct OnArea;
impl Operation for OnArea {
    fn event(&self) -> Option<&str> {
        Some("entered")
    }
    fn events(&self) -> &'static [&'static str] {
        &["left"]
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let player = world
            .attribute_display(ctx.actor, "player")
            .is_some_and(|value| value == "true");
        match (ctx.event.name.as_str(), player) {
            ("entered", true) => Ok("player_entered"),
            ("entered", false) => Ok("npc_entered"),
            ("left", true) => Ok("player_left"),
            ("left", false) => Ok("npc_left"),
            (other, _) => Err(format!("On Area does not handle '{other}'")),
        }
    }
}
/// Field the entered event carries: the area the character walked into.
const ENTERED_AREA_FIELD: &str = "area";
/// Event entry that continues on `match` only when the named area was entered.
/// The condition lives in this node; the actions live downstream in the graph.
struct OnEnterAreaModule;
impl NodeModule for OnEnterAreaModule {
    fn id(&self) -> &'static str {
        "on_enter_area"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["match", "no_match"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let area = text(p, "area")?;
        if area.trim().is_empty() {
            return Err("On Enter Area needs an area name".into());
        }
        Ok(Box::new(OnEnterArea { area }))
    }
}
struct OnEnterArea {
    area: String,
}
impl Operation for OnEnterArea {
    fn event(&self) -> Option<&str> {
        Some("entered")
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let entered = ctx
            .event
            .fields
            .get(ENTERED_AREA_FIELD)
            .ok_or_else(|| format!("The entered event carries no '{ENTERED_AREA_FIELD}'"))?
            .display();
        Ok(if entered.trim() == self.area.trim() {
            "match"
        } else {
            "no_match"
        })
    }
}
/// Life states, in picker order. The engine stores them in the `mode` attribute.
pub const STATES: [&str; 4] = ["Alive", "Dead", "Sleeping", "Unconscious"];
struct StateModule;
impl NodeModule for StateModule {
    fn id(&self) -> &'static str {
        "state"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let selected = p
            .get("state")
            .and_then(|v| v.pointer("/Choice/selected"))
            .and_then(Value::as_u64)
            .ok_or("Missing state selection")?;
        let state = STATES
            .get(selected as usize)
            .ok_or("Invalid state selection")?;
        Ok(Box::new(State(state)))
    }
}
struct State(&'static str);
impl Operation for State {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.set_state(ctx.actor, self.0)?;
        Ok("out")
    }
}
struct MessageModule;
impl NodeModule for MessageModule {
    fn id(&self) -> &'static str {
        "message"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(Message {
            text: text(p, "text")?,
            role: text(p, "role").unwrap_or_default(),
            recipient: text(p, "recipient").unwrap_or_default(),
        }))
    }
}
struct Message {
    text: String,
    role: String,
    recipient: String,
}
impl Operation for Message {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let text = render_event_template(ctx, &self.text)?;
        let target = if self.recipient.trim().is_empty() {
            None
        } else {
            match ctx.event.fields.get(
                self.recipient
                    .trim()
                    .strip_prefix("event.")
                    .unwrap_or(self.recipient.trim()),
            ) {
                Some(EventField::Entity(id)) => Some(*id),
                _ => return Err("Message recipient must reference an event entity".into()),
            }
        };
        world.message_to(ctx.actor, target, text, self.role.trim())?;
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
        let field = field.trim();
        let (source, key) = if let Some(key) = field.strip_prefix("event.") {
            (FilterSource::Event, key)
        } else if let Some(key) = field.strip_prefix("attr.") {
            (FilterSource::Attribute, key)
        } else {
            return Err("Filter fields use event.<field> or attr.<name>".into());
        };
        if key.is_empty() {
            return Err("Filter needs a field name".into());
        }
        let key = key.to_owned();
        let op = p
            .get("operator")
            .and_then(|v| v.pointer("/Choice/selected"))
            .and_then(Value::as_u64)
            .filter(|n| *n < 5)
            .ok_or("Invalid filter operator")?;
        Ok(Box::new(Filter {
            source,
            key,
            op,
            expected: text(p, "expected")?,
        }))
    }
}
#[derive(Clone, Copy)]
enum FilterSource {
    Event,
    Attribute,
}
struct Filter {
    source: FilterSource,
    key: String,
    op: u64,
    expected: String,
}
impl Operation for Filter {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let display = match self.source {
            FilterSource::Event => ctx
                .event
                .fields
                .get(&self.key)
                .ok_or_else(|| format!("Event has no value '{}'", self.key))?
                .display(),
            // A missing attribute means the guard does not apply to this entity,
            // not that the graph is broken: area graphs run for every entrant,
            // and only some of them carry a given flag.
            FilterSource::Attribute => match world.attribute_display(ctx.actor, self.key.trim()) {
                Some(value) => value,
                None => return Ok("no_match"),
            },
        };
        let matched = match self.op {
            0 => display == self.expected,
            1 => display != self.expected,
            2 => display.contains(&self.expected),
            _ => {
                let lhs: f64 = display
                    .trim()
                    .parse()
                    .map_err(|_| "Numeric comparison requires a number".to_string())?;
                let rhs: f64 = self.expected.parse().map_err(|_| "Expected a number")?;
                if self.op == 3 { lhs > rhs } else { lhs < rhs }
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

fn selection(p: &BTreeMap<String, Value>, key: &str) -> Result<String, String> {
    let value = p
        .get(key)
        .and_then(|v| v.get("Choice"))
        .ok_or_else(|| format!("Missing ruleset {key}"))?;
    let index = value
        .get("selected")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Missing {key} selection"))? as usize;
    value
        .get("options")
        .and_then(Value::as_array)
        .and_then(|options| options.get(index))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("Select a ruleset {key}"))
}
fn profile(p: &BTreeMap<String, Value>) -> Result<String, String> {
    selection(p, "profile")
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
        Ok(Box::new(Lookout {
            profile: profile(p)?,
            reaction: p
                .contains_key("reaction_distance")
                .then(|| {
                    number(p, "reaction_distance", 0.1, f32::MAX)
                        .map(|value| value.round().clamp(1., 20.))
                })
                .transpose()?,
            escape: p
                .contains_key("escape_distance")
                .then(|| {
                    number(p, "escape_distance", 0.1, f32::MAX)
                        .map(|value| value.round().clamp(1., 20.))
                })
                .transpose()?,
        }))
    }
}
struct Lookout {
    profile: String,
    reaction: Option<f32>,
    escape: Option<f32>,
}
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
        Ok(
            if world.lookout(ctx.actor, &self.profile, self.reaction, self.escape)? {
                "found"
            } else {
                "watching"
            },
        )
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        match world.lookout(ctx.actor, &self.profile, self.reaction, self.escape) {
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

struct UseActionModule;
impl NodeModule for UseActionModule {
    fn id(&self) -> &'static str {
        "use_action"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["done", "failed"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(UseAction(selection(p, "action")?)))
    }
}
struct UseAction(String);
impl Operation for UseAction {
    fn error_output(&self) -> Option<&'static str> {
        Some("failed")
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let subject = match ctx.event.fields.get("subject") {
            Some(EventField::Entity(id)) => Some(*id),
            Some(_) => return Err("Action subject must be an entity".into()),
            None => None,
        };
        world.use_action(ctx.actor, &self.0, subject)?;
        Ok("done")
    }
}

/// Entry for an event a graph names itself, such as one raised by a dialogue
/// option or by Notify In.
struct OnEventModule;
impl NodeModule for OnEventModule {
    fn id(&self) -> &'static str {
        "on_event"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let name = text(p, "event")?;
        if name.trim().is_empty() {
            return Err("On Event needs an event name".into());
        }
        Ok(Box::new(OnEvent { name }))
    }
}
struct OnEvent {
    name: String,
}
impl Operation for OnEvent {
    fn event(&self) -> Option<&str> {
        Some(self.name.as_str())
    }
    fn execute(
        &self,
        _: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok("out")
    }
}
/// The entity a graph should act on behalf of: whoever triggered the event.
/// An intent carries a `subject`, collisions and raised events carry `entity`.
fn acting_target(ctx: &EventContext<'_>) -> Option<u32> {
    match ctx
        .event
        .fields
        .get("subject")
        .or_else(|| ctx.event.fields.get("entity"))
    {
        Some(EventField::Entity(id)) => Some(*id),
        _ => None,
    }
}
fn valid_quest_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
fn selected(p: &BTreeMap<String, Value>, key: &str) -> Result<u64, String> {
    p.get(key)
        .and_then(|value| value.pointer("/Choice/selected"))
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Missing choice parameter {key}"))
}
struct QuestGuardModule;
impl NodeModule for QuestGuardModule {
    fn id(&self) -> &'static str {
        "quest_guard"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["condition"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let quest = text(p, "quest")?.trim().to_string();
        if !valid_quest_id(&quest) {
            return Err("Quest Guard needs a valid quest ID".into());
        }
        let state = match selected(p, "state")? {
            0 => "not_started",
            1 => "active",
            2 => "completed",
            3 => "not_completed",
            _ => return Err("Invalid quest state".into()),
        };
        Ok(Box::new(QuestGuard { quest, state }))
    }
}
struct QuestGuard {
    quest: String,
    state: &'static str,
}
impl Operation for QuestGuard {
    fn is_guard(&self) -> bool {
        true
    }
    fn evaluate_guard(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<bool, String> {
        let state = world.quest_state(ctx.actor, acting_target(ctx), &self.quest)?;
        Ok(if self.state == "not_completed" {
            state != "completed"
        } else {
            state == self.state
        })
    }
    fn execute(
        &self,
        _: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok("condition")
    }
}
struct ItemGuardModule;
impl NodeModule for ItemGuardModule {
    fn id(&self) -> &'static str {
        "item_guard"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["condition"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let item = text(p, "item")?.trim().to_string();
        if item.is_empty() {
            return Err("Item Guard needs an item".into());
        }
        let has = match selected(p, "presence")? {
            0 => true,
            1 => false,
            _ => return Err("Invalid item presence".into()),
        };
        Ok(Box::new(ItemGuard { item, has }))
    }
}
struct ItemGuard {
    item: String,
    has: bool,
}
impl Operation for ItemGuard {
    fn is_guard(&self) -> bool {
        true
    }
    fn evaluate_guard(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<bool, String> {
        let target = acting_target(ctx).unwrap_or(ctx.actor.render_id);
        Ok(world.inventory_has(target, &self.item)? == self.has)
    }
    fn execute(
        &self,
        _: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok("condition")
    }
}
struct PlayerAttributeGuardModule;
impl NodeModule for PlayerAttributeGuardModule {
    fn id(&self) -> &'static str {
        "player_attribute_guard"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["condition"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let name = text(p, "attribute")?.trim().to_string();
        if name.is_empty() {
            return Err("Player Attribute Guard needs an attribute".into());
        }
        let expected = match selected(p, "expected")? {
            0 => true,
            1 => false,
            _ => return Err("Invalid expected value".into()),
        };
        Ok(Box::new(PlayerAttributeGuard { name, expected }))
    }
}
struct PlayerAttributeGuard {
    name: String,
    expected: bool,
}
impl Operation for PlayerAttributeGuard {
    fn is_guard(&self) -> bool {
        true
    }
    fn evaluate_guard(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<bool, String> {
        Ok(
            world.player_attribute_bool(ctx.actor, acting_target(ctx), &self.name)?
                == self.expected,
        )
    }
    fn execute(
        &self,
        _: &EventContext<'_>,
        _: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok("condition")
    }
}

/// A quest is identified by an author-chosen ID; its state is stored on the
/// player who participates in the event.
struct QuestStateModule;
impl NodeModule for QuestStateModule {
    fn id(&self) -> &'static str {
        "quest_state"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["not_started", "active", "completed"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let quest = text(p, "quest")?.trim().to_owned();
        if !valid_quest_id(&quest) {
            return Err("Quest State needs an ID using letters, numbers, _ or -".into());
        }
        Ok(Box::new(QuestState(quest)))
    }
}
struct QuestState(String);
impl Operation for QuestState {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.quest_state(ctx.actor, acting_target(ctx), &self.0)
    }
}
struct SetQuestModule;
impl NodeModule for SetQuestModule {
    fn id(&self) -> &'static str {
        "set_quest"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["done", "unchanged", "failed"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let quest = text(p, "quest")?.trim().to_owned();
        if !valid_quest_id(&quest) {
            return Err("Set Quest needs an ID using letters, numbers, _ or -".into());
        }
        let state = p
            .get("state")
            .and_then(|v| v.pointer("/Choice/selected"))
            .and_then(Value::as_u64)
            .ok_or("Set Quest needs a state")?;
        let state = match state {
            0 => "active",
            1 => "completed",
            2 => "not_started",
            _ => return Err("Invalid quest state".into()),
        };
        Ok(Box::new(SetQuest { quest, state }))
    }
}
struct SetQuest {
    quest: String,
    state: &'static str,
}
impl Operation for SetQuest {
    fn error_output(&self) -> Option<&'static str> {
        Some("failed")
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        Ok(
            if world.set_quest_state(ctx.actor, acting_target(ctx), &self.quest, self.state)? {
                "done"
            } else {
                "unchanged"
            },
        )
    }
}
/// Opens a dialogue with whoever triggered this event.
struct DialogModule;
impl NodeModule for DialogModule {
    fn id(&self) -> &'static str {
        "dialog"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let node = text(p, "node")?;
        if node.trim().is_empty() {
            return Err("Dialog needs a dialogue node name".into());
        }
        Ok(Box::new(Dialog { node }))
    }
}
struct Dialog {
    node: String,
}
impl Operation for Dialog {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let target = acting_target(ctx).ok_or("Dialog has nobody to talk to")?;
        world.open_dialog(ctx.actor, target, &self.node)?;
        Ok("out")
    }
}
/// Number of choice outputs a Dialogue node exposes. Longer conversations chain
/// further Dialogue nodes; a subgraph keeps a big conversation readable.
pub(super) const DIALOGUE_CHOICE_SLOTS: usize = 6;
const DIALOGUE_CHOICE_PORTS: [&str; DIALOGUE_CHOICE_SLOTS] = [
    "choice:0", "choice:1", "choice:2", "choice:3", "choice:4", "choice:5",
];
fn choice_cell_text(value: &Value) -> String {
    value
        .get("Text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}
/// Reads the repeating `choices` list into `(label, condition)` pairs. A blank
/// label is an unused slot and is skipped.
pub(super) fn dialog_choices(value: Option<&Value>) -> Vec<(String, Option<String>)> {
    let Some(rows) = value
        .and_then(|value| value.pointer("/List/rows"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    rows.iter()
        .map(|row| {
            let Some(cells) = row.as_array() else {
                return (String::new(), None);
            };
            let label = cells.first().map(choice_cell_text).unwrap_or_default();
            let condition = cells.get(1).map(choice_cell_text).unwrap_or_default();
            let condition = (!condition.is_empty()).then_some(condition);
            (label, condition)
        })
        .collect()
}
/// A node-authored conversation step. It shows text with up to
/// `DIALOGUE_CHOICE_SLOTS` choices and waits; every choice has its own output
/// port, so selecting one resumes the flow down that branch. `done` is taken
/// when the dialogue is dismissed without a choice.
struct DialogueModule;
/// A single visible conversation step. Each answer follows a graph connection.
struct PromptModule;
impl NodeModule for PromptModule {
    fn id(&self) -> &'static str {
        "prompt"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &[
            "in", "when:0", "when:1", "when:2", "when:3", "when:4", "when:5",
        ]
    }
    fn outputs(&self) -> &'static [&'static str] {
        DialogueModule.outputs()
    }
    fn is_valid_output(&self, parameters: &BTreeMap<String, Value>, key: &str) -> bool {
        DialogueModule.is_valid_output(parameters, key)
    }
    fn compile(&self, parameters: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let dialogue = compiled_dialogue(parameters)?;
        if dialogue
            .choices
            .iter()
            .any(|(_, condition)| condition.is_some())
        {
            return Err("Prompt answer visibility belongs on guard inputs".into());
        }
        Ok(Box::new(Prompt(dialogue)))
    }
}
struct Prompt(Dialogue);
impl Operation for Prompt {
    fn retains_activity(&self, output: &str) -> bool {
        output == "waiting"
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        self.execute_with_guards(ctx, world, &[true; 6])
    }
    fn execute_with_guards(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
        visible: &[bool; 6],
    ) -> Result<&'static str, String> {
        let target = acting_target(ctx).ok_or("Prompt has nobody to talk to")?;
        let choices: Vec<_> = self
            .0
            .choices
            .iter()
            .enumerate()
            .map(|(index, (label, _))| {
                (
                    if visible[index] {
                        label.clone()
                    } else {
                        String::new()
                    },
                    None,
                )
            })
            .collect();
        if world.present_dialog(ctx.actor, target, &self.0.text, &choices)? {
            Ok("waiting")
        } else {
            Ok("done")
        }
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        self.0.poll(ctx, world)
    }
}
impl NodeModule for DialogueModule {
    fn id(&self) -> &'static str {
        "dialogue"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &[
            "done", "choice:0", "choice:1", "choice:2", "choice:3", "choice:4", "choice:5",
        ]
    }
    fn is_valid_output(&self, _parameters: &BTreeMap<String, Value>, key: &str) -> bool {
        key == "done" || DIALOGUE_CHOICE_PORTS.contains(&key)
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(compiled_dialogue(p)?))
    }
}
fn compiled_dialogue(p: &BTreeMap<String, Value>) -> Result<Dialogue, String> {
    let text = p
        .get("text")
        .and_then(|value| value.get("Text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let choices = dialog_choices(p.get("choices"));
    if text.is_empty() && choices.is_empty() {
        return Err("Dialogue needs text or choices".into());
    }
    if choices.len() > DIALOGUE_CHOICE_SLOTS {
        return Err(format!(
            "Dialogue supports at most {DIALOGUE_CHOICE_SLOTS} choices"
        ));
    }
    Ok(Dialogue { text, choices })
}
struct Dialogue {
    text: String,
    choices: Vec<(String, Option<String>)>,
}
impl Operation for Dialogue {
    fn retains_activity(&self, output: &str) -> bool {
        output == "waiting"
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let target = acting_target(ctx).ok_or("Dialogue has nobody to talk to")?;
        if !world.present_dialog(ctx.actor, target, &self.text, &self.choices)? {
            return Ok("done");
        }
        // Keeping "waiting" live parks the flow on this node until the player
        // answers; the chosen port then carries the flow down its branch.
        Ok("waiting")
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        match world.take_dialog_choice(ctx.actor) {
            Some(DialogChoiceMade::Index(index)) => Some(
                DIALOGUE_CHOICE_PORTS
                    .get(index as usize)
                    .copied()
                    .ok_or_else(|| format!("Dialogue choice {index} is out of range")),
            ),
            Some(DialogChoiceMade::Dismissed) => Some(Ok("done")),
            None => None,
        }
    }
}
/// Offers what this character carries to whoever triggered the event, which is
/// how a quartermaster opens her stock.
struct OfferInventoryModule;
impl NodeModule for OfferInventoryModule {
    fn id(&self) -> &'static str {
        "offer_inventory"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        // An empty filter offers everything the character carries.
        Ok(Box::new(OfferInventory {
            filter: text(p, "filter").unwrap_or_default(),
        }))
    }
}
struct OfferInventory {
    filter: String,
}
impl Operation for OfferInventory {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let target = acting_target(ctx).ok_or("Offer Inventory has nobody to offer to")?;
        world.offer_inventory(ctx.actor, target, &self.filter)?;
        Ok("out")
    }
}
/// Routes on whether whoever triggered this event carries an item.
struct InventoryHasModule;
impl NodeModule for InventoryHasModule {
    fn id(&self) -> &'static str {
        "inventory_has"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["has", "missing"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(InventoryHas {
            item: text(p, "item")?,
        }))
    }
}
struct InventoryHas {
    item: String,
}
impl Operation for InventoryHas {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let target = acting_target(ctx).ok_or("Inventory Has has nobody to check")?;
        if world.inventory_has(target, &self.item)? {
            Ok("has")
        } else {
            Ok("missing")
        }
    }
}
/// Raises an event on this actor later, which is how a door closes itself.
struct NotifyInModule;
impl NodeModule for NotifyInModule {
    fn id(&self) -> &'static str {
        "notify_in"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["out"]
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(NotifyIn {
            minutes: number(p, "minutes", 0.0, 10_000.0)?,
            event: text(p, "event")?,
        }))
    }
}
struct NotifyIn {
    minutes: f32,
    event: String,
}
impl Operation for NotifyIn {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        world.notify_in(ctx.actor, self.minutes, &self.event)?;
        Ok("out")
    }
}
/// Routes on whether anything else stands close by, which is how a door knows
/// it is safe to close.
struct EntitiesInRadiusModule;
impl NodeModule for EntitiesInRadiusModule {
    fn id(&self) -> &'static str {
        "entities_in_radius"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["empty", "occupied"]
    }
    fn compile(&self, _: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        Ok(Box::new(EntitiesInRadius))
    }
}
struct EntitiesInRadius;
impl Operation for EntitiesInRadius {
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        if world.entities_in_radius(ctx.actor)? {
            Ok("occupied")
        } else {
            Ok("empty")
        }
    }
}

/// Every output a Talk node exposes: the end of the conversation, then the
/// consequence ports a choice may leave through.
const TALK_OUTPUTS: [&str; CONSEQUENCE_SLOTS + 1] =
    ["done", "out:0", "out:1", "out:2", "out:3", "out:4", "out:5"];

/// Runs a whole conversation from one node. The steps and the choices live in the
/// node; a choice either moves inside the tree, ends it, or leaves through a
/// consequence port where ordinary nodes do the work.
struct TalkModule;
impl NodeModule for TalkModule {
    fn id(&self) -> &'static str {
        "talk"
    }
    fn inputs(&self) -> &'static [&'static str] {
        &["in"]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &TALK_OUTPUTS
    }
    fn is_valid_output(&self, _parameters: &BTreeMap<String, Value>, key: &str) -> bool {
        key == "done" || CONSEQUENCE_PORTS.contains(&key)
    }
    fn compile(&self, p: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
        let conversation = p
            .get("conversation")
            .and_then(Conversation::from_control)
            .ok_or("Talk needs a conversation")?;
        conversation.validate()?;
        Ok(Box::new(Talk { conversation }))
    }
}
struct Talk {
    conversation: Conversation,
}
impl Operation for Talk {
    fn retains_activity(&self, output: &str) -> bool {
        output == "waiting"
    }
    fn execute(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String> {
        let target = acting_target(ctx).ok_or("Talk has nobody to talk to")?;
        // A consequence chain that handed the conversation back continues where
        // the answer pointed; a plain entry starts at the entry step.
        let resume = world.take_talk_resume(ctx.actor);
        let step = resume
            .as_deref()
            .and_then(|name| self.conversation.step(name.trim()))
            .or_else(|| self.conversation.entry_step())
            .cloned()
            .ok_or("Talk conversation has no steps")?;
        if !present_step(world, ctx, target, &step)? {
            world.set_talk_step(ctx.actor, None);
            return Ok("done");
        }
        world.set_talk_step(ctx.actor, Some(step.name.trim()));
        // Parking on "waiting" keeps the flow on this node between steps.
        Ok("waiting")
    }
    fn poll(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        let choice = world.take_dialog_choice(ctx.actor)?;
        match self.resume(ctx, world, choice) {
            // A step was shown, so the node stays parked on this node until the
            // player answers again.
            Ok(TalkNext::Waiting) => None,
            Ok(TalkNext::Done(port)) => Some(Ok(port)),
            Err(error) => Some(Err(error)),
        }
    }
}
/// What a Talk node does after a choice: show another step, or leave.
enum TalkNext {
    Waiting,
    Done(&'static str),
}
impl Talk {
    /// Move the conversation on by what the player picked.
    fn resume(
        &self,
        ctx: &EventContext<'_>,
        world: &mut dyn WorldServices,
        choice: DialogChoiceMade,
    ) -> Result<TalkNext, String> {
        let target = acting_target(ctx).ok_or("Talk has nobody to talk to")?;
        let DialogChoiceMade::Index(index) = choice else {
            world.set_talk_step(ctx.actor, None);
            return Ok(TalkNext::Done("done"));
        };
        let current = world.talk_step(ctx.actor).unwrap_or_default();
        let step = self
            .conversation
            .step(current.trim())
            .ok_or_else(|| format!("Talk lost its step '{current}'"))?;
        let picked = step
            .choices
            .get(index as usize)
            .ok_or_else(|| format!("Talk choice {index} is out of range"))?
            .clone();

        match picked.then {
            // A jump stays inside the node: the next line is shown here.
            Then::Go { step } => {
                let step = self
                    .conversation
                    .step(step.trim())
                    .cloned()
                    .ok_or_else(|| format!("Talk has no step '{step}'"))?;
                if !present_step(world, ctx, target, &step)? {
                    world.set_talk_step(ctx.actor, None);
                    return Ok(TalkNext::Done("done"));
                }
                world.set_talk_step(ctx.actor, Some(step.name.trim()));
                Ok(TalkNext::Waiting)
            }
            // The consequence is graph work: leave through the port and let the
            // nodes wired to it run. If the chain hands the flow back to this
            // node, it continues at `resume`.
            Then::Out { slot, resume } => {
                world.set_talk_step(ctx.actor, None);
                world.set_talk_resume(ctx.actor, Some(resume.as_str()));
                CONSEQUENCE_PORTS
                    .get(slot)
                    .copied()
                    .map(TalkNext::Done)
                    .ok_or_else(|| format!("Talk has no output 'out:{slot}'"))
            }
            // A choice that does nothing else simply ends the conversation.
            Then::End => {
                world.set_talk_step(ctx.actor, None);
                Ok(TalkNext::Done("done"))
            }
        }
    }
}

/// Show one step and wait. The engine filters the choices by their conditions,
/// so an unavailable line never reaches the player.
fn present_step(
    world: &mut dyn WorldServices,
    ctx: &EventContext<'_>,
    target: u32,
    step: &Step,
) -> Result<bool, String> {
    let choices: Vec<(String, Option<String>)> = step
        .choices
        .iter()
        .map(|choice| {
            let condition = choice.condition.trim();
            (
                choice.label.clone(),
                (!condition.is_empty()).then(|| condition.to_string()),
            )
        })
        .collect();
    world.present_dialog(ctx.actor, target, step.text.trim(), &choices)
}
