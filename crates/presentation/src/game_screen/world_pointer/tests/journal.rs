use super::hud::dispatch;
use super::*;
use crate::game_screen::{HudPanelId, HudPart, hud::model::PanelFrame};
use rusterix::client::{command::ClientCommandBinding, screen_model::CatalogPanelKind};

fn fixture() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, mut screen, fonts, view) = setup();
    screen.resize((960, 540));
    session.assets.rules = eldiron_ruleset_source();
    let templates =
        shared::rulesets::ruleset_item_templates_from_source(&session.assets.rules).unwrap();
    let mut items = Vec::new();
    for (id, name) in [
        (100, "bone_cuirass"),
        (101, "bear_inlay"),
        (102, "tree_resin"),
        (103, "wooden_bowl"),
        (104, "stone_knife"),
        (105, "bone_workbench"),
    ] {
        let mut item = Item::new();
        item.id = id;
        rusterix::server::data::apply_item_data(
            &mut item,
            &templates
                .iter()
                .find(|template| template.id == name)
                .unwrap()
                .data,
        );
        if name == "wooden_bowl" {
            item.set_attribute("liquid_item", Value::Str("fresh_water".into()));
            item.set_attribute("liquid_quantity", Value::Int(1));
        }
        items.push(item);
    }
    let map = &mut session.project.regions[0].map;
    let actor = &mut map.entities[0];
    actor.set_attribute("player", Value::Bool(true));
    actor.set_attribute("skill_boneworking", Value::Int(10));
    actor.set_attribute("STR", Value::Int(12));
    actor.inventory = vec![None; 10];
    let mut station = items.pop().unwrap();
    station.position = actor.position;
    map.items.push(station);
    for item in items {
        actor.add_item(item).unwrap();
    }
    let rules = session.assets.value_rules().unwrap();
    rusterix::learning::acquire(
        actor,
        &rules,
        &["bear_inscription".into()],
        rusterix::learning::Origin::Character,
    )
    .unwrap();
    let mut target = Widget::new();
    target.id = 40;
    target.name = "Action slot".into();
    target.rect = Rect::new(8., 502., 40., 30.);
    target.command_slot = Some("main.0".into());
    screen.elements.button_widgets.insert(40, target);
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("journal".into()))
    ));
    (session, screen, fonts, view)
}
fn eldiron_ruleset_source() -> String {
    shared::rulesets::latest_official_ruleset().into()
}
fn frame(session: &GameSession, screen: &GameScreenController) -> PanelFrame {
    screen
        .hud
        .frames(&session.view(), &screen.elements)
        .into_iter()
        .find(|frame| {
            matches!(
                frame.id,
                HudPanelId::Catalog {
                    kind: CatalogPanelKind::Journal,
                    ..
                }
            )
        })
        .unwrap()
}
fn point(session: &GameSession, screen: &GameScreenController, part: HudPart) -> [f32; 2] {
    let bounds = frame(session, screen)
        .controls()
        .into_iter()
        .find(|(control, _)| control.part == part)
        .unwrap()
        .1;
    [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.]
}
fn ready(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    view: Uuid,
) {
    stage(session, screen, fonts, view);
    ack(session, screen);
}
fn click_part(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    part: HudPart,
) -> Vec<ScreenEffect> {
    let point = point(session, screen, part);
    dispatch(session, screen, fonts, button(point, true));
    dispatch(session, screen, fonts, button(point, false)).effects
}
fn search(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    view: Uuid,
) {
    ready(session, screen, fonts, view);
    assert!(click_part(session, screen, fonts, HudPart::JournalSearch).is_empty());
    assert!(
        dispatch(session, screen, fonts, InputEvent::Text("Inscribe".into()))
            .effects
            .is_empty()
    );
    ready(session, screen, fonts, view);
    assert_eq!(frame(session, screen).cards.len(), 1);
}
#[test]
fn journal_native_selects_without_crafting_and_submits_one_exact_owned_target_after_revalidation() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    let original = serde_json::to_value(&session.project.regions[0].map.entities[0]).unwrap();
    search(&session, &mut screen, &mut fonts, view);
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::JournalClear).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    assert!(
        frame(&session, &screen).cards.len() > 1,
        "clearing search restores the current recipe page"
    );
    search(&session, &mut screen, &mut fonts, view);
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0)).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    assert!(
        click_part(&session, &mut screen, &mut fonts, HudPart::JournalCraft).is_empty(),
        "a target is required"
    );
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::JournalTarget).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    assert!(
        click_part(
            &session,
            &mut screen,
            &mut fonts,
            HudPart::JournalTargetOption(0)
        )
        .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    assert!(
        frame(&session, &screen)
            .detail
            .unwrap()
            .description
            .lines
            .iter()
            .any(|line| line.contains("Bone Cuirass of the Bear"))
    );
    let p = point(&session, &screen, HudPart::JournalCraft);
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    let ingredient = session.project.regions[0].map.entities[0]
        .get_item_slot(101)
        .unwrap();
    let removed = session.project.regions[0].map.entities[0].inventory[ingredient].take();
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(p, false))
            .effects
            .is_empty(),
        "changed ingredients invalidate the captured craft"
    );
    session.project.regions[0].map.entities[0].inventory[ingredient] = removed;
    ready(&session, &mut screen, &mut fonts, view);
    assert_eq!(
        click_part(&session, &mut screen, &mut fonts, HudPart::JournalCraft),
        vec![ScreenEffect::Action(EntityAction::ItemClicked(
            100,
            0.,
            Some("action:inscribe_bear_cuirass".into()),
            Some(1)
        ))]
    );
    assert_eq!(
        serde_json::to_value(&session.project.regions[0].map.entities[0]).unwrap(),
        original,
        "inspection and input only emit requests"
    );
    session.project.regions[0].map.entities[0]
        .get_item_mut(101)
        .unwrap()
        .set_stack_quantity(2);
    ready(&session, &mut screen, &mut fonts, view);
    let p = point(&session, &screen, HudPart::JournalCraft);
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    let mut root = session.assets.rules.parse::<toml::Table>().unwrap();
    root["recipes"]["inscribe_bear_cuirass"]["consumes"][0]["quantity"] = toml::Value::Integer(2);
    session.assets.rules = toml::to_string(&root).unwrap();
    assert!(
        matches!(
            frame(&session, &screen).action(HudPart::JournalCraft),
            Some(crate::game_screen::hud::model::HudAction::JournalCraft { .. })
        ),
        "the new costs are still affordable"
    );
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(p, false))
            .effects
            .is_empty(),
        "changed but still affordable costs must invalidate the old click"
    );
}
#[test]
fn journal_native_pins_via_existing_command_slots_and_renders_knowledge_without_executing_it() {
    let (session, mut screen, mut fonts, view) = fixture();
    search(&session, &mut screen, &mut fonts, view);
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::Assign).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0)).is_empty());
    let p = [28., 516.];
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, button(p, false)).effects,
        vec![ScreenEffect::Action(EntityAction::SetCommandSlot {
            slot: "main.0".into(),
            command: Some("rules.inscribe_bear_cuirass".into())
        })]
    );
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::JournalTab(1)).is_empty());
    // A search remains explicit across tabs; clearing it does not alter knowledge.
    ready(&session, &mut screen, &mut fonts, view);
    click_part(&session, &mut screen, &mut fonts, HudPart::JournalSearch);
    for _ in 0..8 {
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            InputEvent::Key {
                physical: PhysicalKey::Backspace,
                logical: theframework::platform::LogicalKey::Other,
                pressed: true,
                repeat: false,
            },
        );
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            InputEvent::Key {
                physical: PhysicalKey::Backspace,
                logical: theframework::platform::LogicalKey::Other,
                pressed: false,
                repeat: false,
            },
        );
    }
    ready(&session, &mut screen, &mut fonts, view);
    let knowledge = frame(&session, &screen);
    assert_eq!(knowledge.cards.len(), 1);
    assert!(
        knowledge
            .detail
            .unwrap()
            .description
            .lines
            .iter()
            .any(|line| line.contains("Permanently learned"))
    );
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0)).is_empty());
    assert!(
        !frame(&session, &screen)
            .controls()
            .iter()
            .any(|(id, _)| id.part == HudPart::JournalCraft)
    );
}
