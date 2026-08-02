use core_ids::EntityId;
use entity_state::{EntityComponent, EntityState};
use gameplay_mechanics::TracksComponent;

use crate::{
    vitality_track_id, ActorSideCandidate, FloorFeatureKind, FloorSceneContent, FloorSceneFacing,
    GeneratedFloor, RoguelikeRuleset,
};

use super::navigation::{relative, FloorSpatial};
use super::{
    EnemyParticipation, EnemyWorldComponent, MinimapActorView, MinimapCellView, MinimapFeatureKind,
    MinimapTerrainKind, MinimapView, PartyExplorationComponent, RelativeSceneFacing,
    VisibleActorView, VisibleSceneContent, VisibleScenePlacementView, WorldStateError, WorldView,
    WorldViewCell, WorldViewCellKind, MAX_PROJECTED_WORLD_FACTS, MAX_VISIBLE_ACTORS,
    MAX_VISIBLE_SCENE_PLACEMENTS, WORLD_VIEW_SCHEMA_VERSION,
};

pub(super) fn project_world(
    floor: &GeneratedFloor,
    rules: &RoguelikeRuleset,
    entities: &EntityState,
    spatial: &FloorSpatial,
    party_entity: EntityId,
) -> Result<WorldView, WorldStateError> {
    let party = component::<PartyExplorationComponent>(entities, party_entity)?;
    let visible_terrain = spatial.visible_terrain(party.position(), party.facing());
    let scene_terrain = spatial.scene_terrain(party.position(), party.facing());
    let visible_floor = &visible_terrain.floor;
    let visible_set = visible_floor
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();

    let mut cells = scene_terrain
        .floor
        .iter()
        .copied()
        .map(|cell| {
            view_cell(
                party.position(),
                party.facing(),
                cell,
                WorldViewCellKind::Floor,
            )
        })
        .chain(scene_terrain.walls.iter().copied().map(|cell| {
            view_cell(
                party.position(),
                party.facing(),
                cell,
                WorldViewCellKind::Wall,
            )
        }))
        .chain(
            scene_terrain
                .locked_doors_forward
                .iter()
                .copied()
                .map(|cell| {
                    view_cell(
                        party.position(),
                        party.facing(),
                        cell,
                        WorldViewCellKind::LockedDoorForward,
                    )
                }),
        )
        .chain(scene_terrain.locked_doors_side.iter().copied().map(|cell| {
            view_cell(
                party.position(),
                party.facing(),
                cell,
                WorldViewCellKind::LockedDoorSide,
            )
        }))
        .collect::<Result<Vec<_>, _>>()?;
    if cells.len() > MAX_PROJECTED_WORLD_FACTS {
        return Err(error(
            "world_projection_overflow",
            "visible world facts exceed the bounded protocol",
        ));
    }
    cells.sort_by_key(|cell| (cell.depth, cell.lateral, kind_order(cell.kind)));

    let scene_floor = scene_terrain
        .floor
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut scene_placements = floor
        .scene_placements
        .iter()
        .filter(|placement| scene_floor.contains(&super::WorldCell::from(&placement.cell)))
        .map(|placement| {
            let position = super::WorldCell::from(&placement.cell);
            let (lateral, depth) = relative(party.position(), party.facing(), position);
            Ok(VisibleScenePlacementView {
                id: placement.id.clone(),
                lateral: narrow_i16(lateral)?,
                depth: narrow_u8(depth)?,
                facing: relative_scene_facing(party.facing(), placement.facing),
                content: match &placement.content {
                    FloorSceneContent::Prop { content_id } => VisibleSceneContent::Prop {
                        content_id: content_id.clone(),
                    },
                    FloorSceneContent::PointLight {
                        color_rgb,
                        intensity_milli,
                        range_cells,
                    } => VisibleSceneContent::PointLight {
                        color_rgb: color_rgb.clone(),
                        intensity_milli: *intensity_milli,
                        range_cells: *range_cells,
                    },
                },
            })
        })
        .collect::<Result<Vec<_>, WorldStateError>>()?;
    if scene_placements.len() > MAX_VISIBLE_SCENE_PLACEMENTS {
        return Err(error(
            "world_projection_overflow",
            "visible scene placements exceed the bounded protocol",
        ));
    }
    scene_placements.sort_by(|left, right| {
        (left.depth, left.lateral, left.id.as_str()).cmp(&(
            right.depth,
            right.lateral,
            right.id.as_str(),
        ))
    });

    let mut minimap_actors = Vec::new();
    let mut visible_actors = rules
        .actors()
        .values()
        .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        .filter_map(|actor| {
            let entity = EntityId::new(actor.entity_id);
            let world = match component::<EnemyWorldComponent>(entities, entity) {
                Ok(world) => world,
                Err(error) => return Some(Err(error)),
            };
            let tracks = match component::<TracksComponent>(entities, entity) {
                Ok(tracks) => tracks,
                Err(error) => return Some(Err(error)),
            };
            if tracks
                .current(&vitality_track_id())
                .is_none_or(|value| value.get() <= 0)
            {
                return None;
            }
            if !visible_set.contains(&world.position()) {
                return None;
            }
            let (lateral, depth) = relative(party.position(), party.facing(), world.position());
            Some((|| {
                minimap_actors.push(MinimapActorView {
                    actor_id: actor.id.clone(),
                    entity_id: actor.entity_id,
                    name: actor.name.clone(),
                    x: world.position().x,
                    y: world.position().y,
                    participating: world.participation() == EnemyParticipation::Participating,
                });
                Ok(VisibleActorView {
                    actor_id: actor.id.clone(),
                    entity_id: actor.entity_id,
                    name: actor.name.clone(),
                    lateral: narrow_i16(lateral)?,
                    depth: narrow_u8(depth)?,
                    participating: world.participation() == EnemyParticipation::Participating,
                })
            })())
        })
        .collect::<Result<Vec<_>, WorldStateError>>()?;
    if visible_actors.len() > MAX_VISIBLE_ACTORS {
        return Err(error(
            "world_projection_overflow",
            "visible actors exceed the bounded protocol",
        ));
    }
    visible_actors.sort_by_key(|actor| (actor.depth, actor.lateral, actor.entity_id));
    minimap_actors.sort_by_key(|actor| (actor.y, actor.x, actor.entity_id));

    let currently_visible = visible_terrain
        .floor
        .iter()
        .chain(visible_terrain.walls.iter())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut minimap_cells = party
        .discovered()
        .iter()
        .copied()
        .map(|cell| MinimapCellView {
            x: cell.x,
            y: cell.y,
            terrain: MinimapTerrainKind::Floor,
            feature: minimap_feature(floor, cell),
            visible: currently_visible.contains(&cell),
        })
        .chain(
            party
                .discovered_walls()
                .iter()
                .copied()
                .map(|cell| MinimapCellView {
                    x: cell.x,
                    y: cell.y,
                    terrain: MinimapTerrainKind::Wall,
                    feature: None,
                    visible: currently_visible.contains(&cell),
                }),
        )
        .collect::<Vec<_>>();
    minimap_cells.sort_by_key(|cell| (cell.y, cell.x));

    let discovered_cell_count = u16::try_from(party.discovered().len()).map_err(|_| {
        error(
            "world_projection_overflow",
            "discovered-cell count exceeds the bounded world protocol",
        )
    })?;
    Ok(WorldView {
        schema_version: WORLD_VIEW_SCHEMA_VERSION,
        revision: entities.revision(),
        floor_id: floor.floor_id.clone(),
        facing: party.facing(),
        discovered_cell_count,
        cells,
        scene_placements,
        visible_actors,
        minimap: MinimapView {
            party: party.position(),
            facing: party.facing(),
            cells: minimap_cells,
            visible_actors: minimap_actors,
        },
    })
}

fn relative_scene_facing(party: super::Facing, scene: FloorSceneFacing) -> RelativeSceneFacing {
    let absolute = match scene {
        FloorSceneFacing::North => super::Facing::North,
        FloorSceneFacing::East => super::Facing::East,
        FloorSceneFacing::South => super::Facing::South,
        FloorSceneFacing::West => super::Facing::West,
    };
    if absolute == party {
        RelativeSceneFacing::Forward
    } else if absolute == party.right() {
        RelativeSceneFacing::Right
    } else if absolute == party.right().right() {
        RelativeSceneFacing::Backward
    } else {
        RelativeSceneFacing::Left
    }
}

fn minimap_feature(floor: &GeneratedFloor, cell: super::WorldCell) -> Option<MinimapFeatureKind> {
    if let Some(portal) = floor.portals.iter().find(|portal| {
        portal
            .cells
            .iter()
            .any(|candidate| super::WorldCell::from(candidate) == cell)
    }) {
        return Some(if portal.traversal == "locked" {
            MinimapFeatureKind::LockedDoor
        } else {
            MinimapFeatureKind::OpenDoor
        });
    }
    floor
        .features
        .iter()
        .find(|feature| super::WorldCell::from(&feature.cell) == cell)
        .map(|feature| match feature.kind {
            FloorFeatureKind::Entry => MinimapFeatureKind::Entry,
            FloorFeatureKind::Goal => MinimapFeatureKind::Goal,
            FloorFeatureKind::Key => MinimapFeatureKind::Key,
            FloorFeatureKind::Gate => MinimapFeatureKind::Gate,
        })
}

fn view_cell(
    origin: super::WorldCell,
    facing: super::Facing,
    cell: super::WorldCell,
    kind: WorldViewCellKind,
) -> Result<WorldViewCell, WorldStateError> {
    let (lateral, depth) = relative(origin, facing, cell);
    Ok(WorldViewCell {
        lateral: narrow_i16(lateral)?,
        depth: narrow_u8(depth)?,
        kind,
    })
}

fn narrow_i16(value: i32) -> Result<i16, WorldStateError> {
    i16::try_from(value).map_err(|_| {
        error(
            "world_projection_overflow",
            "relative world coordinate exceeds the bounded protocol",
        )
    })
}

fn narrow_u8(value: i32) -> Result<u8, WorldStateError> {
    u8::try_from(value).map_err(|_| {
        error(
            "world_projection_overflow",
            "relative world depth exceeds the bounded protocol",
        )
    })
}

const fn kind_order(kind: WorldViewCellKind) -> u8 {
    match kind {
        WorldViewCellKind::Floor => 0,
        WorldViewCellKind::LockedDoorForward => 1,
        WorldViewCellKind::LockedDoorSide => 2,
        WorldViewCellKind::Wall => 3,
    }
}

fn component<T: EntityComponent>(
    state: &EntityState,
    entity: EntityId,
) -> Result<&T, WorldStateError> {
    state
        .component::<T>(entity)
        .map_err(|detail| error("world_component_read", detail.to_string()))?
        .ok_or_else(|| error("world_component_missing", std::any::type_name::<T>()))
}

fn error(code: &'static str, detail: impl Into<String>) -> WorldStateError {
    WorldStateError::new(code, detail)
}
