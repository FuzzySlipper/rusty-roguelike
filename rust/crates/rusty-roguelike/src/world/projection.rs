use core_ids::EntityId;
use entity_state::{EntityComponent, EntityState};
use gameplay_mechanics::TracksComponent;

use crate::{vitality_track_id, ActorSideCandidate, GeneratedFloor, RoguelikeRuleset};

use super::navigation::{relative, FloorSpatial};
use super::{
    EnemyParticipation, EnemyWorldComponent, PartyExplorationComponent, VisibleActorView,
    WorldStateError, WorldView, WorldViewCell, WorldViewCellKind, MAX_PROJECTED_WORLD_FACTS,
    MAX_VISIBLE_ACTORS, WORLD_VIEW_SCHEMA_VERSION,
};

pub(super) fn project_world(
    floor: &GeneratedFloor,
    rules: &RoguelikeRuleset,
    entities: &EntityState,
    spatial: &FloorSpatial,
    party_entity: EntityId,
) -> Result<WorldView, WorldStateError> {
    let party = component::<PartyExplorationComponent>(entities, party_entity)?;
    let visible_floor = spatial.visible_floor_cells(party.position(), party.facing());
    let visible_set = visible_floor
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();

    let mut cells = visible_floor
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
        .chain(
            spatial
                .first_visible_walls(party.position(), party.facing(), &visible_floor)
                .into_iter()
                .map(|cell| {
                    view_cell(
                        party.position(),
                        party.facing(),
                        cell,
                        WorldViewCellKind::Wall,
                    )
                }),
        )
        .collect::<Result<Vec<_>, _>>()?;
    if cells.len() > MAX_PROJECTED_WORLD_FACTS {
        return Err(error(
            "world_projection_overflow",
            "visible world facts exceed the bounded protocol",
        ));
    }
    cells.sort_by_key(|cell| (cell.depth, cell.lateral, kind_order(cell.kind)));

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
        visible_actors,
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
        WorldViewCellKind::Wall => 1,
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
