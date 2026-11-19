use super::*;
use crate::server::{message::NodeChoice, regionctx::ChoiceSession};

fn fixture() -> (RegionInstance, Arc<Mutex<RegionCtx>>) {
    let mut ctx = RegionCtx::default();
    ctx.currencies = Currencies::official_default();
    for id in 1..=3 {
        let mut entity = Entity::default();
        entity.id = id;
        entity.position = Vec3::new(id as f32 * 0.3, 0., 0.);
        entity.set_attribute("player", Value::Bool(id != 3));
        entity.inventory = vec![None; 4];
        entity
            .wallet
            .balances
            .insert(ctx.currencies.base_currency.clone(), 20);
        ctx.entity_classes.insert(id, "Actor".into());
        ctx.map.entities.push(entity);
    }
    let ctx = Arc::new(Mutex::new(ctx));
    let region = RegionInstance::with_context(1, ctx.clone());
    ctx.lock()
        .unwrap()
        .from_sender
        .set(region.from_sender.clone())
        .unwrap();
    (region, ctx)
}
fn item(id: u32) -> Item {
    let mut item = Item::default();
    item.id = id;
    item
}

#[test]
fn terrain_walk_uses_host_time_and_camera_switches_require_project_permission() {
    let (mut region, ctx) = fixture();
    let request = GameRequest::Terrain {
        position: [0.3, 3.],
        intent: String::new(),
    };
    assert_eq!(
        region.prepare_game_request(1, &request, None),
        Err(RequestError::Forbidden)
    );
    {
        let mut ctx = ctx.lock().unwrap();
        ctx.config =
            "[game]\nauto_walk_2d=true\n[multiplayer.gameplay]\nallowed_cameras=['firstp_mouse']"
                .parse()
                .unwrap();
        ctx.map.entities.retain(|e| e.id == 1);
        ctx.map.entities[0].set_attribute("player_camera", Value::PlayerCamera(PlayerCamera::D2));
    }
    let start = ctx.lock().unwrap().map.entities[0].position;
    execute(&mut region, 1, request, None).unwrap();
    assert_eq!(ctx.lock().unwrap().map.entities[0].position, start);
    assert!(matches!(
        ctx.lock().unwrap().map.entities[0].action,
        EntityAction::Goto(..)
    ));
    region.movement_tick();
    assert_ne!(ctx.lock().unwrap().map.entities[0].position, start);
    assert_eq!(
        region.prepare_game_request(1, &GameRequest::Camera(PlayerCamera::D2Grid), None),
        Err(RequestError::Forbidden)
    );
    execute(
        &mut region,
        1,
        GameRequest::Camera(PlayerCamera::D3FirstPMouse),
        None,
    )
    .unwrap();
    assert_eq!(
        ctx.lock().unwrap().map.entities[0]
            .attributes
            .get("player_camera"),
        Some(&Value::PlayerCamera(PlayerCamera::D3FirstPMouse))
    );
}
#[test]
fn damage_tags_both_combatants_using_shared_simulation_time() {
    let (_, ctx) = fixture();
    let mut ctx = ctx.lock().unwrap();
    ctx.health_attr = "HP".into();
    for entity in &mut ctx.map.entities {
        entity.set_attribute("HP", Value::Int(20));
    }
    ctx.world.advance_resource_time(Duration::from_secs(2));
    assert!(apply_damage_direct(&mut ctx, 1, 2, 3, "physical", None));
    assert_eq!(ctx.map.entities[0].attributes.get_int("HP"), Some(17));
    for id in [1, 2] {
        assert_eq!(
            ctx.find_entity(id)
                .unwrap()
                .attributes
                .get("__last_combat_ms"),
            Some(&Value::Int64(2000))
        );
    }
    assert!(
        ctx.find_entity(3)
            .unwrap()
            .attributes
            .get("__last_combat_ms")
            .is_none()
    );
}
fn execute(
    region: &mut RegionInstance,
    actor: u32,
    request: GameRequest,
    offer: Option<&Choice>,
) -> Result<(), RequestError> {
    for action in region.prepare_game_request(actor, &request, offer)? {
        region
            .to_sender
            .send(RegionMessage::UserAction(actor, action))
            .unwrap();
    }
    region.redraw_tick();
    Ok(())
}
fn take(id: u32) -> GameRequest {
    GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: id,
            owner: ItemOwner::World,
        }),
        destination: ItemDestination::Inventory(None),
    }
}
#[test]
fn damage_marks_both_combatants_on_shared_simulation_time_and_crafting_is_declared() {
    let (region, ctx) = fixture();
    let mut ctx = ctx.lock().unwrap();
    ctx.health_attr = "HP".into();
    for entity in &mut ctx.map.entities {
        entity.set_attribute("HP", Value::Int(20));
    }
    ctx.world.advance_resource_time(Duration::from_secs(5));
    assert!(apply_damage_direct(&mut ctx, 1, 2, 3, "physical", None));
    for id in [1, 2] {
        assert_eq!(
            ctx.find_entity(id)
                .unwrap()
                .attributes
                .get("__last_combat_ms"),
            Some(&Value::Int64(5000))
        );
    }
    assert_eq!(
        ctx.find_entity(1).unwrap().attributes.get_int("HP"),
        Some(17)
    );
    ctx.rules = "[recipes.bowl]\nname='Bowl'".parse().unwrap();
    drop(ctx);
    assert_eq!(
        region
            .prepare_game_request(
                1,
                &GameRequest::Craft {
                    recipe: "bowl".into()
                },
                None
            )
            .unwrap(),
        vec![EntityAction::TextCommand("craft bowl".into())]
    );
    assert_eq!(
        region.prepare_game_request(
            1,
            &GameRequest::Craft {
                recipe: "missing".into()
            },
            None
        ),
        Err(RequestError::Unsupported)
    );
    assert!(
        !GameRequest::Craft {
            recipe: "bowl\nadmin".into()
        }
        .bounded()
    );
}
#[test]
fn inventory_is_owned_and_contested_pickup_happens_once() {
    let (mut region, ctx) = fixture();
    ctx.lock().unwrap().map.items.push(item(10));
    execute(&mut region, 1, take(10), None).unwrap();
    assert_eq!(
        execute(&mut region, 2, take(10), None),
        Err(RequestError::NotFound)
    );
    let steal = GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: 10,
            owner: ItemOwner::Corpse(1),
        }),
        destination: ItemDestination::Inventory(None),
    };
    assert_eq!(
        execute(&mut region, 2, steal, None),
        Err(RequestError::Forbidden)
    );
    let ctx = ctx.lock().unwrap();
    assert!(ctx.map.entities[0].get_item(10).is_some());
    assert!(ctx.map.entities[1].get_item(10).is_none());
    assert!(ctx.map.items.is_empty());
}
#[test]
fn companion_transfers_require_live_npc_membership_and_recheck_on_consumption() {
    let (mut region, ctx) = fixture();
    {
        let mut ctx = ctx.lock().unwrap();
        ctx.map.entities[0].set_attribute("__frame_motion", Value::Bool(true));
        ctx.map.entities[0].inventory[0] = Some(item(10));
        ctx.map.entities[2].set_attribute("party_member", Value::Bool(true));
        ctx.map.entities[2].set_attribute("party_leader_id", Value::UInt(1));
    }
    let give = GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: 10,
            owner: ItemOwner::Owned,
        }),
        destination: ItemDestination::FollowerInventory {
            entity: 3,
            index: Some(0),
        },
    };
    execute(&mut region, 1, give, None).unwrap();
    assert!(ctx.lock().unwrap().map.entities[0].get_item(10).is_none());
    assert!(ctx.lock().unwrap().map.entities[2].get_item(10).is_some());
    let take = GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: 10,
            owner: ItemOwner::Follower(3),
        }),
        destination: ItemDestination::Inventory(Some(1)),
    };
    assert_eq!(
        region.prepare_game_request(2, &take, None),
        Err(RequestError::Forbidden)
    );
    let actions = region.prepare_game_request(1, &take, None).unwrap();
    ctx.lock().unwrap().map.entities[2].set_attribute("party_leader_id", Value::UInt(2));
    for action in actions {
        region
            .to_sender
            .send(RegionMessage::UserAction(1, action))
            .unwrap();
    }
    region.redraw_tick();
    assert!(
        ctx.lock().unwrap().map.entities[2].get_item(10).is_some(),
        "stale membership cannot take a companion item"
    );
    ctx.lock().unwrap().map.entities[2].set_attribute("party_leader_id", Value::UInt(1));
    execute(&mut region, 1, take, None).unwrap();
    assert!(ctx.lock().unwrap().map.entities[0].get_item(10).is_some());
    let mut ctx = ctx.lock().unwrap();
    ctx.map.entities[1].set_attribute("party_member", Value::Bool(true));
    ctx.map.entities[1].set_attribute("party_leader_id", Value::UInt(1));
    drop(ctx);
    let foreign = GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: 10,
            owner: ItemOwner::Owned,
        }),
        destination: ItemDestination::FollowerInventory {
            entity: 2,
            index: None,
        },
    };
    assert_eq!(
        region.prepare_game_request(1, &foreign, None),
        Err(RequestError::Forbidden),
        "a player's party membership never grants inventory writes"
    );
}
#[test]
fn containers_require_live_access_and_range_is_rechecked() {
    let (mut region, ctx) = fixture();
    let mut bag = item(10);
    bag.container = Some(vec![item(11)]);
    ctx.lock().unwrap().map.items.push(bag);
    let address = ItemAddress {
        item: 10,
        owner: ItemOwner::World,
    };
    let request = GameRequest::Transfer {
        source: ItemSource::Container {
            container: address,
            item: 11,
        },
        destination: ItemDestination::Inventory(None),
    };
    assert_eq!(
        execute(&mut region, 1, request.clone(), None),
        Err(RequestError::Forbidden)
    );
    execute(&mut region, 1, GameRequest::OpenContainer(address), None).unwrap();
    ctx.lock().unwrap().map.entities[0].position.x = 100.;
    assert_eq!(
        execute(&mut region, 1, request.clone(), None),
        Err(RequestError::Forbidden)
    );
    ctx.lock().unwrap().map.entities[0].position.x = 0.3;
    execute(&mut region, 1, GameRequest::OpenContainer(address), None).unwrap();
    execute(&mut region, 1, request, None).unwrap();
    let ctx = ctx.lock().unwrap();
    assert!(ctx.map.items[0].container.as_ref().unwrap().is_empty());
    assert!(ctx.map.entities[0].get_item(11).is_some());
}
#[test]
fn combat_checks_server_range_known_actions_costs_and_cooldown() {
    let (mut region, ctx) = fixture();
    {
        let mut ctx = ctx.lock().unwrap();
        ctx.rules = toml::from_str::<toml::Table>(
            r#"
            [classes.Warrior]
            [actions.strike]
            kind="attack"
            target="any_entity"
            range=2
            cooldown=4
            cost={MP=3}
            requires={ability="strike"}
            result={damage="abilities.strike.damage"}
            [abilities.strike.damage]
            roll="1d1"
            bonus=2
        "#,
        )
        .unwrap();
        ctx.health_attr = "HP".into();
        ctx.max_health_attr = "MAX_HP".into();
        ctx.entity_programs
            .insert("Actor".into(), Arc::new(crate::vm::Program::new()));
        ctx.map.entities[1].set_attribute("HP", Value::Int(20));
        ctx.map.entities[1].set_attribute("MAX_HP", Value::Int(20));
        ctx.map.entities[0].set_attribute("MP", Value::Int(10));
        ctx.map.entities[0].set_attribute("class", Value::Str("Warrior".into()));
        ctx.map.entities[1].position.x = 100.;
    }
    let attack = GameRequest::Entity {
        target: 2,
        intent: Some("action:strike".into()),
    };
    assert_eq!(
        execute(&mut region, 1, attack.clone(), None),
        Err(RequestError::OutOfRange)
    );
    ctx.lock().unwrap().map.entities[1].position.x = 0.6;
    execute(&mut region, 1, attack.clone(), None).unwrap();
    assert_eq!(
        ctx.lock().unwrap().map.entities[0].attributes.get_int("MP"),
        Some(10),
        "unknown ability cannot spend/cast"
    );
    ctx.lock().unwrap().map.entities[0]
        .set_attribute("abilities", Value::StrArray(vec!["strike".into()]));
    execute(&mut region, 1, attack.clone(), None).unwrap();
    assert_eq!(
        ctx.lock().unwrap().map.entities[0].attributes.get_int("MP"),
        Some(7)
    );
    assert!(
        ctx.lock().unwrap().map.entities[0]
            .attributes
            .get_float_default("cooldown_left_rules_strike", 0.)
            > 0.
    );
    assert_eq!(
        ctx.lock().unwrap().map.entities[1].attributes.get_int("HP"),
        Some(17)
    );
    execute(&mut region, 1, attack, None).unwrap();
    assert_eq!(
        ctx.lock().unwrap().map.entities[1].attributes.get_int("HP"),
        Some(17)
    );
    assert_eq!(
        ctx.lock().unwrap().map.entities[0].attributes.get_int("MP"),
        Some(7),
        "cooldown survives another request without host time"
    );
    for intent in ["admin.grant", "spell:unlearned", "action:missing"] {
        assert_eq!(
            region.prepare_game_request(1, &GameRequest::Intent(intent.into()), None),
            Err(RequestError::Unsupported)
        );
    }
}
#[test]
fn dialogue_choices_are_recipient_bound_expire_and_consume_once() {
    let (mut region, ctx) = fixture();
    let choice = Choice::NodeChoice(NodeChoice {
        label: "Accept".into(),
        index: 2,
        from: 3,
        to: 1,
        expires_at_tick: 100,
        max_distance: 3.,
    });
    let request = GameRequest::Choose {
        offer: Uuid::new_v4(),
    };
    ctx.lock()
        .unwrap()
        .active_choice_sessions
        .push(ChoiceSession {
            from: 3,
            to: 1,
            expires_at_tick: 100,
            max_distance: 3.,
        });
    assert_eq!(
        execute(&mut region, 1, request.clone(), None),
        Err(RequestError::ExpiredOffer)
    );
    assert_eq!(
        execute(&mut region, 2, request.clone(), Some(&choice)),
        Err(RequestError::ExpiredOffer)
    );
    ctx.lock().unwrap().map.entities[0].position.x = 100.;
    assert_eq!(
        execute(&mut region, 1, request.clone(), Some(&choice)),
        Err(RequestError::ExpiredOffer)
    );
    ctx.lock().unwrap().map.entities[0].position.x = 0.3;
    execute(&mut region, 1, request.clone(), Some(&choice)).unwrap();
    assert!(matches!(
        ctx.lock().unwrap().dialog_choices.get(&3),
        Some(crate::server::nodes::DialogChoiceMade::Index(2))
    ));
    assert_eq!(
        execute(&mut region, 1, request.clone(), Some(&choice)),
        Err(RequestError::ExpiredOffer)
    );
    ctx.lock()
        .unwrap()
        .active_choice_sessions
        .push(ChoiceSession {
            from: 3,
            to: 1,
            expires_at_tick: 100,
            max_distance: 3.,
        });
    ctx.lock().unwrap().ticks = 101;
    assert_eq!(
        execute(&mut region, 1, request, Some(&choice)),
        Err(RequestError::ExpiredOffer)
    );
}
#[test]
fn purchases_transfer_both_wallets_once_and_full_or_negative_price_rolls_back() {
    for (price, full, success) in [(5, false, true), (5, true, false), (-1, false, false)] {
        let (mut region, ctx) = fixture();
        {
            let mut ctx = ctx.lock().unwrap();
            let mut sale = item(10);
            sale.attributes.set("worth", Value::Int(price));
            ctx.map.entities[2].inventory[0] = Some(sale);
            if full {
                ctx.map.entities[0].inventory = vec![Some(item(20))];
            }
            ctx.active_choice_sessions.push(ChoiceSession {
                from: 3,
                to: 1,
                expires_at_tick: 100,
                max_distance: 3.,
            });
        }
        let choice = Choice::ItemToSell(10, 3, 1, 100, 3.);
        let request = GameRequest::Choose {
            offer: Uuid::new_v4(),
        };
        execute(&mut region, 1, request.clone(), Some(&choice)).unwrap();
        assert_eq!(
            execute(&mut region, 1, request, Some(&choice)),
            Err(RequestError::ExpiredOffer)
        );
        let ctx = ctx.lock().unwrap();
        assert_eq!(ctx.map.entities[0].get_item(10).is_some(), success);
        assert_eq!(ctx.map.entities[2].get_item(10).is_some(), !success);
        assert_eq!(
            ctx.map.entities[0].wallet.get_balance(&ctx.currencies),
            if success { 15 } else { 20 }
        );
        assert_eq!(
            ctx.map.entities[2].wallet.get_balance(&ctx.currencies),
            if success { 25 } else { 20 }
        );
    }
}

#[test]
fn equipped_items_can_be_unequipped_without_exposing_foreign_equipment() {
    let (mut region, ctx) = fixture();
    ctx.lock().unwrap().map.entities[0]
        .equipped
        .insert("main_hand".into(), item(10));
    let request = GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: 10,
            owner: ItemOwner::Owned,
        }),
        destination: ItemDestination::Inventory(Some(0)),
    };
    assert_eq!(
        execute(&mut region, 2, request.clone(), None),
        Err(RequestError::NotFound)
    );
    ctx.lock().unwrap().map.entities[0].inventory[0] = Some(item(11));
    execute(&mut region, 1, request.clone(), None).unwrap();
    assert!(
        ctx.lock().unwrap().map.entities[0]
            .equipped
            .contains_key("main_hand"),
        "a swap cannot silently equip an item without a valid slot/policy"
    );
    ctx.lock().unwrap().map.entities[0].inventory[0] = None;
    execute(&mut region, 1, request, None).unwrap();
    let ctx = ctx.lock().unwrap();
    assert!(ctx.map.entities[0].equipped.is_empty());
    assert!(ctx.map.entities[0].get_item(10).is_some());
}

#[test]
fn automatic_inventory_destination_unequips_once_and_rejects_a_full_inventory() {
    let (mut region, ctx) = fixture();
    {
        let mut ctx = ctx.lock().unwrap();
        ctx.map.entities[0]
            .equipped
            .insert("main_hand".into(), item(10));
        ctx.map.entities[0].inventory = vec![Some(item(11))];
    }
    let request = GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: 10,
            owner: ItemOwner::Owned,
        }),
        destination: ItemDestination::Inventory(None),
    };
    assert_eq!(
        execute(&mut region, 1, request.clone(), None),
        Err(RequestError::Invalid)
    );
    assert!(
        ctx.lock().unwrap().map.entities[0]
            .equipped
            .contains_key("main_hand")
    );
    ctx.lock().unwrap().map.entities[0].inventory[0] = None;
    execute(&mut region, 1, request, None).unwrap();
    let ctx = ctx.lock().unwrap();
    assert!(ctx.map.entities[0].equipped.is_empty());
    assert_eq!(ctx.map.entities[0].inventory[0].as_ref().unwrap().id, 10);
}
