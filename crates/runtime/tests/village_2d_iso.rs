use eldiron_runtime::{GameSession, SessionOptions};
use rusterix::{Entity, EntityAction, Item, Map, server::message::Choice};
use theframework_core::prelude::{TheTime, Vec2};

fn village_project() -> shared::project::Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/Village2DIso.eldiron");
    let bytes = std::fs::read(path).unwrap();
    shared::project_io::decode_project(&bytes).unwrap()
}

fn find_stock(villager: &Entity, id: &str) -> Option<u32> {
    villager.iter_inventory().find_map(|(_, item): (_, &Item)| {
        (item.attributes.get_str("ruleset_id") == Some(id)).then_some(item.id)
    })
}

fn actor_named<'a>(map: &'a Map, name: &str) -> &'a Entity {
    map.entities
        .iter()
        .find(|entity| entity.attributes.get_str("name") == Some(name))
        .unwrap_or_else(|| panic!("Missing villager or animal {name}"))
}

fn assert_deer_in_wood(map: &Map) {
    let wood = map.sectors.iter().find(|s| s.name == "Deer Wood").unwrap();
    for name in ["Meadow Deer", "Hillside Deer"] {
        let deer = actor_named(map, name);
        assert!(
            wood.is_inside(map, deer.get_pos_xz()),
            "{name} left Deer Wood at {:?}",
            deer.position
        );
    }
}

#[test]
fn village_can_make_and_sell_gear_from_finite_stock() {
    let mut project = village_project();
    project.time = TheTime::new_time(9, 30).unwrap();
    assert_eq!(project.name, "Village2DIso");
    let map_id = project.regions[0].map.id;
    let cask_id = project.regions[0]
        .items
        .values()
        .find(|item| item.name == "Tannery Water Cask")
        .expect("the tannery needs a placed water source")
        .id;
    assert!(
        project.regions[0]
            .map
            .sectors
            .iter()
            .any(|area| area.name == "Mara’s Tannery")
    );
    let mut game = GameSession::new(project);
    game.start(SessionOptions::default()).unwrap();
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    let seller_id = {
        let region = context.lock().unwrap();
        let seller = actor_named(&region.map, "Mara the Leatherworker");
        assert!(find_stock(seller, "leather_boots").is_none());
        assert!(find_stock(seller, "leather_hood").is_none());
        assert!(find_stock(seller, "leather").is_none());
        assert!(find_stock(seller, "raw_hide").is_some());
        let bowl = find_stock(seller, "wooden_bowl").expect("Mara starts with a bowl");
        assert_eq!(
            seller
                .get_item(bowl)
                .unwrap()
                .attributes
                .get_int("liquid_quantity"),
            Some(0)
        );
        seller.id
    };
    let mut tanned_hide = false;
    for _ in 0..180 {
        game.tick_once();
        let region = context.lock().unwrap();
        let seller = actor_named(&region.map, "Mara the Leatherworker");
        tanned_hide |=
            find_stock(seller, "raw_hide").is_none() && find_stock(seller, "leather").is_some();
        if find_stock(seller, "leather_hood").is_some() {
            break;
        }
    }
    assert!(tanned_hide, "the tanning frame must turn hide into leather");
    let (boot_id, buyer_id, price, seller_balance, buyer_balance) = {
        let mut region = context.lock().unwrap();
        let seller = region
            .map
            .entities
            .iter()
            .find(|actor| actor.id == seller_id)
            .unwrap();
        let boot_id = find_stock(seller, "leather_boots").expect("workbench must produce stock");
        assert!(find_stock(seller, "leather_hood").is_some());
        let cask = rusterix::server::region::resources::views(&region)
            .unwrap()
            .into_iter()
            .find(|view| view.access == cask_id)
            .expect("cask must register as a world resource");
        assert_eq!(cask.supplies["water"], Some(4));
        for consumed in [
            "raw_hide",
            "tree_bark",
            "plant_fiber",
            "leather",
            "twisted_cord",
        ] {
            assert!(
                find_stock(seller, consumed).is_none(),
                "{consumed} was not consumed"
            );
        }
        let price = seller.get_item(boot_id).unwrap().shop_price();
        assert_eq!(price, 14);
        let seller_balance = seller.wallet.get_balance(&region.currencies);
        let next_to_seller = seller.position + theframework_core::prelude::Vec3::new(0., 0., 0.75);
        let currencies = region.currencies.clone();
        let buyer = region
            .map
            .entities
            .iter_mut()
            .find(|actor| actor.is_player())
            .unwrap();
        buyer.position = next_to_seller;
        (
            boot_id,
            buyer.id,
            price,
            seller_balance,
            buyer.wallet.get_balance(&currencies),
        )
    };
    game.action(EntityAction::EntityClicked(
        seller_id,
        0.75,
        Some("talk".into()),
    ));
    let mut chosen = None;
    for _ in 0..12 {
        game.tick_once();
        for choice in game
            .take_events(&map_id)
            .choices
            .into_iter()
            .flat_map(|offer| offer.choices)
        {
            if matches!(choice, Choice::ItemToSell(item, seller, buyer, _, _) if item == boot_id && seller == seller_id && buyer == buyer_id)
            {
                chosen = Some(choice);
                break;
            }
        }
        if chosen.is_some() {
            break;
        }
    }
    let choice = chosen.expect("Talk must offer the crafted boot for sale");
    game.action(EntityAction::Choice(choice.clone()));
    for _ in 0..4 {
        game.tick_once();
    }
    game.action(EntityAction::Choice(choice));
    for _ in 0..4 {
        game.tick_once();
    }
    let region = context.lock().unwrap();
    let seller = region
        .map
        .entities
        .iter()
        .find(|actor| actor.id == seller_id)
        .unwrap();
    let buyer = region
        .map
        .entities
        .iter()
        .find(|actor| actor.id == buyer_id)
        .unwrap();
    assert!(seller.get_item(boot_id).is_none());
    assert_eq!(find_stock(buyer, "leather_boots"), Some(boot_id));
    assert_eq!(
        seller.wallet.get_balance(&region.currencies),
        seller_balance + price
    );
    assert_eq!(
        buyer.wallet.get_balance(&region.currencies),
        buyer_balance - price
    );
}

#[test]
fn villagers_follow_daily_routes_and_deer_remain_in_deer_wood() {
    let mut project = village_project();
    project.time = TheTime::new_time(9, 30).unwrap();
    let map_id = project.regions[0].map.id;
    let mut game = GameSession::new(project);
    game.start(SessionOptions::default()).unwrap();
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    {
        let region = context.lock().unwrap();
        let bram = actor_named(&region.map, "Bram the Hunter");
        assert_eq!(
            bram.get_equipped_item("back")
                .and_then(|item| item.attributes.get_str("ruleset_path")),
            Some("items.clothing.hunting_cloak")
        );
        assert!(bram.get_equipped_item("head").is_none());
        assert!(find_stock(bram, "hunter_hood").is_some());
        for (name, color, hair, face) in [
            ("Bram the Hunter", "#6f4937", "tousled", "weathered"),
            ("Rowan the Innkeeper", "#9e9a91", "receding", "bearded"),
            ("Mara the Leatherworker", "#a95330", "braid", "round"),
            ("Nessa the Trader", "#352524", "bob", "angular"),
            ("Elin the Gate Guard", "#bd9c61", "crop", "square"),
        ] {
            let actor = actor_named(&region.map, name);
            assert_eq!(actor.attributes.get_str("hair_color"), Some(color));
            assert_eq!(actor.attributes.get_str("hair_style"), Some(hair));
            assert_eq!(actor.attributes.get_str("face_style"), Some(face));
        }
        for (name, build, height, legs, feet) in [
            ("Bram the Hunter", "lean", 1.87, "forest_trousers", "trail_boots"),
            ("Rowan the Innkeeper", "stout", 1.73, "wool_trousers", "leather_shoes"),
            ("Mara the Leatherworker", "compact", 1.63, "canvas_trousers", "work_boots"),
            ("Nessa the Trader", "slender", 1.69, "slate_trousers", "town_shoes"),
            ("Elin the Gate Guard", "athletic", 1.94, "watch_trousers", "watch_boots"),
        ] {
            let actor = actor_named(&region.map, name);
            assert_eq!(actor.attributes.get_str("body_build"), Some(build));
            assert_eq!(actor.attributes.get_float("body_height"), Some(height));
            for (slot, id) in [("legs", legs), ("feet", feet)] {
                assert_eq!(
                    actor.get_equipped_item(slot).and_then(|item| item.attributes.get_str("ruleset_id")),
                    Some(id),
                    "wrong {slot} equipment for {name}"
                );
            }
        }
    }
    let daytime = [
        ("Bram the Hunter", "Bram’s Hunting Trail"),
        ("Mara the Leatherworker", "Mara’s Workbench"),
        ("Nessa the Trader", "Nessa’s Stall"),
        ("Rowan the Innkeeper", "Inn Counter"),
    ];
    let evening = [
        ("Bram the Hunter", "Bram’s Rest"),
        ("Mara the Leatherworker", "Mara’s Rest Spot"),
        ("Nessa the Trader", "Nessa’s Well Court Stop"),
        ("Rowan the Innkeeper", "Inn Hearthside"),
    ];
    let beds = ["Shepherd Cot", "Mara’s Cot", "Rowan Cot", "Inn Bed"];
    for (hour, minute, places, crosses_bridge, sleeping) in [
        (9, 30, daytime, true, false),
        (19, 0, evening, true, false),
        (23, 20, evening, false, true),
        (9, 30, daytime, true, false),
    ] {
        game.server
            .set_time(&map_id, TheTime::new_time(hour, minute).unwrap());
        let mut arrived = false;
        let mut crossed_bridge = false;
        for _ in 0..260 {
            game.tick_once();
            let region = context.lock().unwrap();
            assert_deer_in_wood(&region.map);
            let bram = actor_named(&region.map, "Bram the Hunter");
            let bridge = region
                .map
                .sectors
                .iter()
                .find(|s| s.name == "Woodland Bridge")
                .unwrap();
            crossed_bridge |=
                bridge.is_inside(&region.map, bram.get_pos_xz()) && bram.position.y > 0.35;
            arrived = places.iter().zip(beds).all(|((name, place), bed)| {
                let actor = actor_named(&region.map, name);
                if sleeping {
                    actor.attributes.get_str("posture") == Some("lying")
                        && region.map.items.iter().any(|item| {
                            item.attributes.get_str("name") == Some(bed)
                                && actor.attributes.get_uint("furniture_id") == Some(item.id)
                        })
                } else {
                    actor.attributes.get_str("posture") != Some("lying")
                        && actor
                            .get_pos_xz()
                            .distance(region.map.named_area_center(place).unwrap())
                            < 0.32
                }
            });
            if arrived {
                break;
            }
        }
        assert!(arrived, "{hour:02}:{minute:02}: {:?}", {
            let region = context.lock().unwrap();
            places.map(|(name, place)| {
                let actor = actor_named(&region.map, name);
                (
                    name,
                    actor.position,
                    actor.action.clone(),
                    place,
                    region.map.named_area_center(place),
                )
            })
        });
        if crosses_bridge {
            assert!(
                crossed_bridge,
                "Bram must cross the raised bridge deck at {hour:02}:{minute:02}"
            );
        }
    }

    // The target is inside the wood, but tile navigation snaps it past the edge.
    {
        let mut region = context.lock().unwrap();
        let start = Vec2::new(-5.5, 36.0);
        let target = Vec2::new(-5.5, 35.85);
        let wood = region
            .map
            .sectors
            .iter()
            .find(|s| s.name == "Deer Wood")
            .unwrap();
        assert!(wood.is_inside(&region.map, start));
        assert!(wood.is_inside(&region.map, target));
        let deer_id = actor_named(&region.map, "Meadow Deer").id;
        let deer = region
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == deer_id)
            .unwrap();
        deer.set_pos_xz(start);
        deer.action = EntityAction::RandomWalkInSector(2.6, 0.65, 2, 1, target);
    }
    for _ in 0..3 {
        game.tick_once();
        assert_deer_in_wood(&context.lock().unwrap().map);
    }
}
