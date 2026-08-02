use std::collections::{BTreeMap, BTreeSet, VecDeque};

use core_ids::EntityId;

use crate::{
    generate_authored_floor, starter_ruleset, CollapsedPartyComponent, FloorBounds, FloorCell,
    FloorFeature, FloorFeatureKind, FloorPortal,
};

use super::*;

const SEED: u64 = 5_201;

#[test]
fn admitted_floor_seeds_one_collapsed_party_and_four_facing_engine_views() {
    let floor = generate_authored_floor(SEED).expect("admitted floor");
    let rules = starter_ruleset().expect("starter rules");
    let mut world = WorldState::new(floor.clone(), rules).expect("world state");
    let initial = world.view().expect("world view");
    let party = world
        .entities()
        .component::<CollapsedPartyComponent>(world.party_entity())
        .expect("party component read")
        .expect("collapsed party component");

    assert_eq!(party.member_entity_ids().len(), 3);
    assert_eq!(initial.schema_version, WORLD_VIEW_SCHEMA_VERSION);
    assert_eq!(initial.floor_id, floor.floor_id);
    assert_eq!(initial.facing, Facing::North);
    assert!(!initial.cells.is_empty());
    assert!(initial
        .cells
        .iter()
        .all(|cell| i32::from(cell.depth) <= MAX_VIEW_DEPTH));

    assert_eq!(world.turn_right().expect("east view").facing, Facing::East);
    assert_eq!(
        world.turn_right().expect("south view").facing,
        Facing::South
    );
    assert_eq!(world.turn_right().expect("west view").facing, Facing::West);
    let north_again = world.turn_right().expect("north view");
    assert_eq!(north_again.facing, Facing::North);
    assert_eq!(north_again.cells, initial.cells);
    assert_eq!(north_again.visible_actors, initial.visible_actors);
}

#[test]
fn collision_projection_stops_at_first_wall_and_hidden_actors_remain_dormant() {
    let floor = occlusion_floor();
    let mut world =
        WorldState::new(floor, starter_ruleset().expect("starter rules")).expect("occlusion world");
    let view = world.view().expect("world view");

    assert!(view.cells.contains(&WorldViewCell {
        lateral: 0,
        depth: 1,
        kind: WorldViewCellKind::Floor,
    }));
    assert!(view.cells.contains(&WorldViewCell {
        lateral: 0,
        depth: 2,
        kind: WorldViewCellKind::Wall,
    }));
    assert!(!view.cells.iter().any(|cell| {
        cell.lateral == 0 && cell.depth >= 3 && cell.kind == WorldViewCellKind::Floor
    }));
    assert!(view.visible_actors.is_empty());
    assert!(view.minimap.cells.contains(&MinimapCellView {
        x: 2,
        y: 2,
        terrain: MinimapTerrainKind::Wall,
        feature: None,
        visible: true,
    }));
    assert!(!view
        .minimap
        .cells
        .iter()
        .any(|cell| cell.x == 2 && cell.y <= 1));
    let turned = world.turn_right().expect("turn away from remembered wall");
    assert!(turned.minimap.cells.contains(&MinimapCellView {
        x: 2,
        y: 2,
        terrain: MinimapTerrainKind::Wall,
        feature: None,
        visible: false,
    }));
    assert!(world
        .durable_state()
        .expect("durable world")
        .enemies
        .iter()
        .all(|enemy| enemy.world.participation() == EnemyParticipation::Dormant));
}

#[test]
fn locked_door_is_a_visible_minimap_fixture_and_occludes_the_forward_cone() {
    let mut floor = occlusion_floor();
    floor.floor_id = "floor.locked-door-occlusion".to_owned();
    floor.portals = vec![FloorPortal {
        id: "door.locked".to_owned(),
        source_edge_id: "edge.locked".to_owned(),
        cells: vec![FloorCell { x: 2, y: 3 }],
        orientation: "horizontal".to_owned(),
        traversal: "locked".to_owned(),
        required_item: Some("key.test".to_owned()),
    }];
    let world =
        WorldState::new(floor, starter_ruleset().expect("starter rules")).expect("door world");
    let view = world.view().expect("door view");
    assert!(view.minimap.cells.contains(&MinimapCellView {
        x: 2,
        y: 3,
        terrain: MinimapTerrainKind::Floor,
        feature: Some(MinimapFeatureKind::LockedDoor),
        visible: true,
    }));
    assert!(!view
        .cells
        .iter()
        .any(|cell| cell.lateral == 0 && cell.depth >= 2));
    assert!(view.visible_actors.is_empty());
}

#[test]
fn discovering_an_enemy_starts_participation_which_survives_lost_sight() {
    let floor = occlusion_floor();
    let rules = starter_ruleset().expect("starter rules");
    let mut world = WorldState::new(floor.clone(), rules).expect("occlusion world");
    let target = world
        .durable_state()
        .expect("durable world")
        .enemies
        .into_iter()
        .next()
        .expect("starter opposition");
    assert_eq!(target.world.participation(), EnemyParticipation::Dormant);

    let entry = party_position(&world);
    let route = route(&floor, entry, target.world.position()).expect("route to enemy");
    let mut revealed = false;
    for destination in route.into_iter().skip(1) {
        let origin = party_position(&world);
        face_delta(
            &mut world,
            destination.x - origin.x,
            destination.y - origin.y,
        );
        if world
            .view()
            .expect("facing view")
            .visible_actors
            .iter()
            .any(|actor| actor.entity_id == target.entity_id)
        {
            revealed = true;
            break;
        }
        match world.step(RelativeStep::Forward) {
            Ok(view) => {
                if view
                    .visible_actors
                    .iter()
                    .any(|actor| actor.entity_id == target.entity_id)
                {
                    revealed = true;
                    break;
                }
            }
            Err(error) if error.code() == "world_step_occupied" => break,
            Err(error) => panic!("route step failed: {error}"),
        }
    }
    assert!(revealed, "route must reveal the selected hidden actor");
    assert_eq!(
        participation(&world, target.entity_id),
        EnemyParticipation::Participating
    );

    world.turn_right().expect("turn away once");
    let away = world.turn_right().expect("turn away twice");
    assert!(!away
        .visible_actors
        .iter()
        .any(|actor| actor.entity_id == target.entity_id));
    assert_eq!(
        participation(&world, target.entity_id),
        EnemyParticipation::Participating
    );
}

#[test]
fn rejected_movement_and_forged_or_disconnected_restore_publish_nothing() {
    let floor = occlusion_floor();
    let rules = starter_ruleset().expect("starter rules");
    let mut world = WorldState::new(floor.clone(), rules).expect("occlusion world");
    world.step(RelativeStep::Forward).expect("open first step");
    let before_rejection = world.durable_state().expect("durable before rejection");
    let error = world
        .step(RelativeStep::Forward)
        .expect_err("front wall must reject");
    assert_eq!(error.code(), "world_position_not_walkable");
    assert_eq!(
        world.durable_state().expect("durable after rejection"),
        before_rejection
    );

    let mut forged = serde_json::to_value(&before_rejection).expect("encode durable world");
    forged["party"]["position"] = serde_json::json!({ "x": 2, "y": 2 });
    let forged: WorldDurableState = serde_json::from_value(forged).expect("strict forged shape");
    let error = WorldState::restore(
        floor.clone(),
        starter_ruleset().expect("starter rules"),
        forged,
    )
    .err()
    .expect("nonwalkable restore must reject");
    assert_eq!(error.code(), "world_position_not_walkable");

    let mut disconnected = floor;
    disconnected.bounds.width = 7;
    disconnected.walkable_cells.push(FloorCell { x: 6, y: 4 });
    let error = WorldState::new(disconnected, starter_ruleset().expect("starter rules"))
        .err()
        .expect("disconnected floor must reject");
    assert_eq!(error.code(), "world_position_disconnected");
}

#[test]
fn restore_requires_exact_roster_discovery_and_dormancy_facts() {
    let floor = occlusion_floor();
    let world = WorldState::new(floor.clone(), starter_ruleset().expect("starter rules"))
        .expect("occlusion world");
    let durable = world.durable_state().expect("durable world");
    let reopened = WorldState::restore(
        floor.clone(),
        starter_ruleset().expect("starter rules"),
        durable.clone(),
    )
    .expect("canonical reopen");
    assert_eq!(reopened.durable_state().expect("reopened durable"), durable);

    let mut forged = durable.clone();
    let mut discovered = forged.party.discovered().to_vec();
    discovered.push(forged.enemies[0].world.position());
    forged.party = PartyExplorationComponent::new(
        forged.party.floor_id().to_owned(),
        forged.party.position(),
        forged.party.facing(),
        discovered,
        forged.party.discovered_walls().to_vec(),
    )
    .expect("canonical forged discovery");
    let error = WorldState::restore(
        floor.clone(),
        starter_ruleset().expect("starter rules"),
        forged,
    )
    .err()
    .expect("dormant actor on discovered cell must reject");
    assert_eq!(error.code(), "world_dormancy_forged");

    let mut noncanonical = serde_json::to_value(&durable).expect("encode durable world");
    noncanonical["party"]["discovered"]
        .as_array_mut()
        .expect("discovered array")
        .reverse();
    let noncanonical: WorldDurableState =
        serde_json::from_value(noncanonical).expect("noncanonical durable shape");
    let error = WorldState::restore(
        floor.clone(),
        starter_ruleset().expect("starter rules"),
        noncanonical,
    )
    .err()
    .expect("noncanonical discovery must reject");
    assert_eq!(error.code(), "world_discovery_not_canonical");

    let mut forged_wall = durable.clone();
    let mut walls = forged_wall.party.discovered_walls().to_vec();
    walls.push(forged_wall.enemies[0].world.position());
    forged_wall.party = PartyExplorationComponent::new(
        forged_wall.party.floor_id().to_owned(),
        forged_wall.party.position(),
        forged_wall.party.facing(),
        forged_wall.party.discovered().to_vec(),
        walls,
    )
    .expect("canonical forged wall shape");
    let error = WorldState::restore(
        floor.clone(),
        starter_ruleset().expect("starter rules"),
        forged_wall,
    )
    .err()
    .expect("walkable discovered wall must reject");
    assert_eq!(error.code(), "world_discovered_wall_invalid");

    let mut missing = durable;
    missing.enemies.pop();
    let error = WorldState::restore(floor, starter_ruleset().expect("starter rules"), missing)
        .err()
        .expect("missing enemy must reject");
    assert_eq!(error.code(), "world_enemy_roster_mismatch");
}

#[test]
fn snapshot_restore_rejects_omitted_currently_visible_wall_discovery() {
    let floor = occlusion_floor();
    let world = WorldState::new(floor.clone(), starter_ruleset().expect("starter rules"))
        .expect("occlusion world");
    let mut snapshot = world.entity_snapshot();
    let party = snapshot
        .registered_components
        .iter_mut()
        .find(|component| component.type_id == PARTY_EXPLORATION_COMPONENT_TYPE_ID)
        .expect("party exploration snapshot");
    let party_value = party
        .values
        .iter_mut()
        .find(|value| value.entity == world.party_entity().raw())
        .expect("party exploration value");
    assert!(party_value.value["discoveredWalls"]
        .as_array()
        .is_some_and(|walls| !walls.is_empty()));
    party_value.value["discoveredWalls"] = serde_json::json!([]);

    let error = WorldState::restore_snapshot(
        floor,
        starter_ruleset().expect("starter rules"),
        snapshot,
    )
    .err()
    .expect("visible wall omission must reject");
    assert_eq!(error.code(), "world_discovery_incomplete");
}

#[test]
fn opposition_navigation_routes_around_walls_and_an_occupied_preferred_step() {
    let wall_floor = movement_floor(true);
    let mut world = positioned_movement_world(
        wall_floor,
        WorldCell { x: 2, y: 0 },
        WorldCell { x: 4, y: 4 },
    );
    assert_eq!(
        world.clear_distance(WorldCell { x: 2, y: 0 }, WorldCell { x: 2, y: 4 }),
        None
    );
    assert!(world.move_enemy_toward_party(EntityId::new(201)).unwrap());
    assert_eq!(
        world.enemy_position(EntityId::new(201)).unwrap(),
        WorldCell { x: 3, y: 0 }
    );
    assert!(world.move_enemy_toward_party(EntityId::new(201)).unwrap());
    assert_eq!(
        world.enemy_position(EntityId::new(201)).unwrap(),
        WorldCell { x: 3, y: 1 }
    );

    let open_floor = movement_floor(false);
    let mut world = positioned_movement_world(
        open_floor,
        WorldCell { x: 2, y: 0 },
        WorldCell { x: 2, y: 1 },
    );
    assert!(world.move_enemy_toward_party(EntityId::new(201)).unwrap());
    assert_eq!(
        world.enemy_position(EntityId::new(201)).unwrap(),
        WorldCell { x: 3, y: 0 }
    );
    assert!(world.move_enemy_toward_party(EntityId::new(201)).unwrap());
    assert!(world.move_enemy_toward_party(EntityId::new(201)).unwrap());
    assert_eq!(
        world.enemy_position(EntityId::new(201)).unwrap(),
        WorldCell { x: 3, y: 2 }
    );
    assert_eq!(
        world.enemy_position(EntityId::new(202)).unwrap(),
        WorldCell { x: 2, y: 1 }
    );
}

fn occlusion_floor() -> crate::GeneratedFloor {
    let mut floor = generate_authored_floor(SEED).expect("provenance donor floor");
    floor.floor_id = "floor.occlusion-regression".to_owned();
    floor.bounds = FloorBounds {
        min_x: 0,
        min_y: 0,
        width: 5,
        height: 5,
    };
    floor.walkable_cells = [
        (2, 4),
        (2, 3),
        (1, 4),
        (1, 3),
        (1, 2),
        (1, 1),
        (1, 0),
        (2, 0),
        (2, 1),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 4),
    ]
    .into_iter()
    .map(|(x, y)| FloorCell { x, y })
    .collect();
    floor.regions.clear();
    floor.portals.clear();
    floor.features = vec![FloorFeature {
        id: "entry".to_owned(),
        source_node_id: "node.entry".to_owned(),
        kind: FloorFeatureKind::Entry,
        cell: FloorCell { x: 2, y: 4 },
    }];
    floor
}

fn movement_floor(with_wall: bool) -> crate::GeneratedFloor {
    let mut floor = generate_authored_floor(SEED).expect("provenance donor floor");
    floor.floor_id = if with_wall {
        "floor.movement-around-wall"
    } else {
        "floor.movement-around-actor"
    }
    .to_owned();
    floor.bounds = FloorBounds {
        min_x: 0,
        min_y: 0,
        width: 5,
        height: 5,
    };
    floor.walkable_cells = (0..5)
        .flat_map(|y| (0..5).map(move |x| FloorCell { x, y }))
        .filter(|cell| !with_wall || *cell != FloorCell { x: 2, y: 2 })
        .collect();
    floor.regions.clear();
    floor.portals.clear();
    floor.features = vec![FloorFeature {
        id: "entry".to_owned(),
        source_node_id: "node.entry".to_owned(),
        kind: FloorFeatureKind::Entry,
        cell: FloorCell { x: 2, y: 4 },
    }];
    floor
}

fn positioned_movement_world(
    floor: crate::GeneratedFloor,
    mover: WorldCell,
    blocker: WorldCell,
) -> WorldState {
    let seeded = WorldState::new(floor.clone(), starter_ruleset().unwrap()).unwrap();
    let mut durable = seeded.durable_state().unwrap();
    durable.enemies[0].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        mover,
        EnemyParticipation::Participating,
    )
    .unwrap();
    durable.enemies[1].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        blocker,
        EnemyParticipation::Participating,
    )
    .unwrap();
    WorldState::restore(floor, starter_ruleset().unwrap(), durable).unwrap()
}

fn party_position(world: &WorldState) -> WorldCell {
    world
        .durable_state()
        .expect("durable world")
        .party
        .position()
}

fn participation(world: &WorldState, entity_id: u64) -> EnemyParticipation {
    world
        .durable_state()
        .expect("durable world")
        .enemies
        .into_iter()
        .find(|enemy| enemy.entity_id == entity_id)
        .expect("enemy state")
        .world
        .participation()
}

fn face_delta(world: &mut WorldState, dx: i32, dy: i32) {
    let desired = match (dx, dy) {
        (0, -1) => Facing::North,
        (1, 0) => Facing::East,
        (0, 1) => Facing::South,
        (-1, 0) => Facing::West,
        _ => panic!("route must use adjacent cardinal steps"),
    };
    for _ in 0..4 {
        if world.durable_state().expect("durable world").party.facing() == desired {
            return;
        }
        world.turn_right().expect("route turn");
    }
    panic!("could not face route destination");
}

fn route(
    floor: &crate::GeneratedFloor,
    start: WorldCell,
    goal: WorldCell,
) -> Option<Vec<WorldCell>> {
    let walkable = floor
        .walkable_cells
        .iter()
        .map(WorldCell::from)
        .collect::<BTreeSet<_>>();
    let mut queue = VecDeque::from([start]);
    let mut previous = BTreeMap::from([(start, None)]);
    while let Some(cell) = queue.pop_front() {
        if cell == goal {
            let mut path = Vec::new();
            let mut cursor = Some(cell);
            while let Some(current) = cursor {
                path.push(current);
                cursor = previous[&current];
            }
            path.reverse();
            return Some(path);
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let next = WorldCell {
                x: cell.x + dx,
                y: cell.y + dy,
            };
            if walkable.contains(&next) && !previous.contains_key(&next) {
                previous.insert(next, Some(cell));
                queue.push_back(next);
            }
        }
    }
    None
}
