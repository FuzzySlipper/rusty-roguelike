use std::collections::BTreeSet;

use core_ids::EntityId;
use entity_state::{EntityAuthoringService, EntityComponent, EntityDefinition, EntityState};
use serde::{Deserialize, Serialize};

use crate::{
    AbilityScore, AbilityScoresComponent, ActorBuildComponent, ActorSideCandidate,
    CollapsedPartyComponent, FloorFeatureKind, GeneratedFloor, RoguelikeRuleset,
};

use super::navigation::FloorSpatial;
use super::projection::project_world;
use super::{
    EnemyParticipation, EnemyWorldComponent, Facing, PartyExplorationComponent, RelativeStep,
    WorldCell, WorldStateError, WorldView,
};

pub const WORLD_DURABLE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DurableEnemyState {
    #[serde(rename = "entityId")]
    pub entity_id: u64,
    pub world: EnemyWorldComponent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorldDurableState {
    pub schema_version: u32,
    pub floor_id: String,
    pub party: PartyExplorationComponent,
    pub enemies: Vec<DurableEnemyState>,
}

pub struct WorldState {
    floor: GeneratedFloor,
    rules: RoguelikeRuleset,
    entities: EntityState,
    spatial: FloorSpatial,
    party_entity: EntityId,
}

impl WorldState {
    pub fn new(floor: GeneratedFloor, rules: RoguelikeRuleset) -> Result<Self, WorldStateError> {
        let spatial = FloorSpatial::build(&floor)?;
        let entry = entry_cell(&floor)?;
        spatial.require_reachable(entry, entry)?;
        for cell in floor.walkable_cells.iter().map(WorldCell::from) {
            spatial.require_reachable(entry, cell)?;
        }
        let party_entity = EntityId::new(rules.party().entity_id);
        let definitions = rules
            .actors()
            .values()
            .map(|actor| EntityDefinition::new(EntityId::new(actor.entity_id), actor.name.clone()))
            .chain(std::iter::once(EntityDefinition::new(
                party_entity,
                rules.party().id.to_string(),
            )))
            .collect::<Vec<_>>();
        let registry = super::roguelike_world_component_registry()
            .map_err(|detail| error("world_component_registry", detail.to_string()))?;
        let mut entities = EntityState::from_definitions_with_registry(registry, definitions)
            .map_err(|detail| error("world_entity_seed", detail.to_string()))?;

        for actor in rules.actors().values() {
            let entity = EntityId::new(actor.entity_id);
            attach(
                &mut entities,
                entity,
                AbilityScoresComponent::new(
                    actor
                        .abilities
                        .iter()
                        .map(|score| AbilityScore::new(score.ability.clone(), score.score))
                        .collect(),
                )
                .map_err(|detail| error("world_actor_seed", detail.to_string()))?,
            )?;
            attach(
                &mut entities,
                entity,
                ActorBuildComponent::new(
                    actor.class.clone(),
                    actor.class_level,
                    actor.actions.clone(),
                    actor.feats.clone(),
                    actor.items.clone(),
                )
                .map_err(|detail| error("world_actor_seed", detail.to_string()))?,
            )?;
        }

        let member_entity_ids = rules
            .party()
            .members
            .iter()
            .map(|id| rules.actors()[id].entity_id)
            .collect();
        attach(
            &mut entities,
            party_entity,
            CollapsedPartyComponent::new(member_entity_ids)
                .map_err(|detail| error("world_party_seed", detail.to_string()))?,
        )?;
        let visible = spatial.visible_floor_cells(entry, Facing::North);
        attach(
            &mut entities,
            party_entity,
            PartyExplorationComponent::new(floor.floor_id.clone(), entry, Facing::North, visible)
                .map_err(|detail| error("world_party_seed", detail))?,
        )?;

        let opposition = rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
            .collect::<Vec<_>>();
        let placements = initial_enemy_positions(&floor, entry, opposition.len())?;
        for (actor, position) in opposition.into_iter().zip(placements) {
            attach(
                &mut entities,
                EntityId::new(actor.entity_id),
                EnemyWorldComponent::new(
                    floor.floor_id.clone(),
                    position,
                    EnemyParticipation::Dormant,
                )
                .map_err(|detail| error("world_enemy_seed", detail))?,
            )?;
        }

        let mut state = Self {
            floor,
            rules,
            entities,
            spatial,
            party_entity,
        };
        state.refresh_visibility()?;
        Ok(state)
    }

    pub fn restore(
        floor: GeneratedFloor,
        rules: RoguelikeRuleset,
        durable: WorldDurableState,
    ) -> Result<Self, WorldStateError> {
        if durable.schema_version != WORLD_DURABLE_SCHEMA_VERSION {
            return Err(error(
                "world_schema_unsupported",
                format!("unsupported world schema {}", durable.schema_version),
            ));
        }
        if durable.floor_id != floor.floor_id || durable.party.floor_id() != floor.floor_id {
            return Err(error(
                "world_floor_mismatch",
                "durable state does not identify the admitted floor",
            ));
        }
        let mut state = Self::new(floor, rules)?;
        let entry = entry_cell(&state.floor)?;
        state
            .spatial
            .require_reachable(entry, durable.party.position())?;
        let party = PartyExplorationComponent::new(
            durable.party.floor_id().to_owned(),
            durable.party.position(),
            durable.party.facing(),
            durable.party.discovered().to_vec(),
        )
        .map_err(|detail| error("world_party_restore_invalid", detail))?;
        if party != durable.party {
            return Err(error(
                "world_discovery_not_canonical",
                "durable discovery cells must already be sorted and unique",
            ));
        }
        for cell in party.discovered() {
            state.spatial.require_reachable(entry, *cell)?;
        }
        let visible = state
            .spatial
            .visible_floor_cells(party.position(), party.facing());
        if visible
            .iter()
            .any(|cell| party.discovered().binary_search(cell).is_err())
        {
            return Err(error(
                "world_discovery_incomplete",
                "durable discovery omits currently visible floor cells",
            ));
        }

        replace(&mut state.entities, state.party_entity, party.clone())?;
        let expected = state
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
            .map(|actor| actor.entity_id)
            .collect::<BTreeSet<_>>();
        let observed = durable
            .enemies
            .iter()
            .map(|enemy| enemy.entity_id)
            .collect::<BTreeSet<_>>();
        if durable.enemies.len() != observed.len()
            || durable
                .enemies
                .windows(2)
                .any(|pair| pair[0].entity_id >= pair[1].entity_id)
            || observed != expected
        {
            return Err(error(
                "world_enemy_roster_mismatch",
                "durable enemies do not exactly match compiled opposition",
            ));
        }
        let mut occupied = BTreeSet::new();
        for enemy in durable.enemies {
            if enemy.world.floor_id() != state.floor.floor_id
                || enemy.world.position() == party.position()
                || !occupied.insert(enemy.world.position())
            {
                return Err(error(
                    "world_enemy_position_invalid",
                    "enemy placement has a floor mismatch, overlap, or duplicate",
                ));
            }
            state
                .spatial
                .require_reachable(entry, enemy.world.position())?;
            if enemy.world.participation() == EnemyParticipation::Dormant
                && party
                    .discovered()
                    .binary_search(&enemy.world.position())
                    .is_ok()
            {
                return Err(error(
                    "world_dormancy_forged",
                    "an enemy on a discovered cell cannot remain dormant",
                ));
            }
            replace(
                &mut state.entities,
                EntityId::new(enemy.entity_id),
                enemy.world,
            )?;
        }
        state.validate_current_visibility()?;
        Ok(state)
    }

    pub fn durable_state(&self) -> Result<WorldDurableState, WorldStateError> {
        let party = self.party()?.clone();
        let mut enemies = self
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
            .map(|actor| {
                Ok(DurableEnemyState {
                    entity_id: actor.entity_id,
                    world: self.enemy(EntityId::new(actor.entity_id))?.clone(),
                })
            })
            .collect::<Result<Vec<_>, WorldStateError>>()?;
        enemies.sort_by_key(|enemy| enemy.entity_id);
        Ok(WorldDurableState {
            schema_version: WORLD_DURABLE_SCHEMA_VERSION,
            floor_id: self.floor.floor_id.clone(),
            party,
            enemies,
        })
    }

    pub fn turn_left(&mut self) -> Result<WorldView, WorldStateError> {
        self.rotate(self.party()?.facing().left())
    }

    pub fn turn_right(&mut self) -> Result<WorldView, WorldStateError> {
        self.rotate(self.party()?.facing().right())
    }

    pub fn step(&mut self, step: RelativeStep) -> Result<WorldView, WorldStateError> {
        let party = self.party()?.clone();
        let (forward_x, forward_y) = party.facing().forward();
        let (right_x, right_y) = party.facing().right_axis();
        let (dx, dy) = match step {
            RelativeStep::Forward => (forward_x, forward_y),
            RelativeStep::Backward => (-forward_x, -forward_y),
            RelativeStep::Left => (-right_x, -right_y),
            RelativeStep::Right => (right_x, right_y),
        };
        let destination = WorldCell {
            x: party.position().x + dx,
            y: party.position().y + dy,
        };
        for actor in self
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        {
            if self.enemy(EntityId::new(actor.entity_id))?.position() == destination {
                return Err(error(
                    "world_step_occupied",
                    "party movement cannot enter an occupied actor cell",
                ));
            }
        }
        self.spatial
            .require_single_step(party.position(), destination)?;
        let mut staged = self.entities.clone();
        replace(
            &mut staged,
            self.party_entity,
            party.with_pose(destination, party.facing()),
        )?;
        self.refresh_visibility_on(&mut staged)?;
        self.entities = staged;
        self.view()
    }

    pub fn view(&self) -> Result<WorldView, WorldStateError> {
        project_world(
            &self.floor,
            &self.rules,
            &self.entities,
            &self.spatial,
            self.party_entity,
        )
    }

    pub fn entities(&self) -> &EntityState {
        &self.entities
    }

    pub const fn party_entity(&self) -> EntityId {
        self.party_entity
    }

    fn rotate(&mut self, facing: Facing) -> Result<WorldView, WorldStateError> {
        let party = self.party()?.clone();
        let mut staged = self.entities.clone();
        replace(
            &mut staged,
            self.party_entity,
            party.with_pose(party.position(), facing),
        )?;
        self.refresh_visibility_on(&mut staged)?;
        self.entities = staged;
        self.view()
    }

    fn refresh_visibility(&mut self) -> Result<(), WorldStateError> {
        let mut staged = self.entities.clone();
        self.refresh_visibility_on(&mut staged)?;
        self.entities = staged;
        Ok(())
    }

    fn refresh_visibility_on(&self, entities: &mut EntityState) -> Result<(), WorldStateError> {
        let party = component::<PartyExplorationComponent>(entities, self.party_entity)?.clone();
        let visible = self
            .spatial
            .visible_floor_cells(party.position(), party.facing());
        let mut discovered = party.discovered().iter().copied().collect::<BTreeSet<_>>();
        discovered.extend(visible.iter().copied());
        replace(
            entities,
            self.party_entity,
            party.with_discovered(discovered.into_iter().collect()),
        )?;
        let visible = visible.into_iter().collect::<BTreeSet<_>>();
        for actor in self
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        {
            let entity = EntityId::new(actor.entity_id);
            let enemy = component::<EnemyWorldComponent>(entities, entity)?.clone();
            if enemy.participation() == EnemyParticipation::Dormant
                && visible.contains(&enemy.position())
            {
                replace(entities, entity, enemy.participating())?;
            }
        }
        Ok(())
    }

    fn validate_current_visibility(&self) -> Result<(), WorldStateError> {
        let party = self.party()?;
        let visible = self
            .spatial
            .visible_floor_cells(party.position(), party.facing())
            .into_iter()
            .collect::<BTreeSet<_>>();
        for actor in self
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        {
            let enemy = self.enemy(EntityId::new(actor.entity_id))?;
            if visible.contains(&enemy.position())
                && enemy.participation() == EnemyParticipation::Dormant
            {
                return Err(error(
                    "world_visible_enemy_dormant",
                    "a currently visible enemy cannot restore as dormant",
                ));
            }
        }
        Ok(())
    }

    fn party(&self) -> Result<&PartyExplorationComponent, WorldStateError> {
        component(&self.entities, self.party_entity)
    }

    fn enemy(&self, entity: EntityId) -> Result<&EnemyWorldComponent, WorldStateError> {
        component(&self.entities, entity)
    }
}

fn entry_cell(floor: &GeneratedFloor) -> Result<WorldCell, WorldStateError> {
    floor
        .features
        .iter()
        .find(|feature| feature.kind == FloorFeatureKind::Entry)
        .map(|feature| WorldCell::from(&feature.cell))
        .ok_or_else(|| error("world_entry_missing", "admitted floor has no entry"))
}

fn initial_enemy_positions(
    floor: &GeneratedFloor,
    entry: WorldCell,
    count: usize,
) -> Result<Vec<WorldCell>, WorldStateError> {
    let mut candidates = floor
        .walkable_cells
        .iter()
        .map(WorldCell::from)
        .filter(|cell| *cell != entry)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        distance(entry, *right)
            .cmp(&distance(entry, *left))
            .then_with(|| left.cmp(right))
    });
    if candidates.len() < count {
        return Err(error(
            "world_enemy_placement_exhausted",
            "floor has too few distinct enemy placement cells",
        ));
    }
    Ok(candidates.into_iter().take(count).collect())
}

fn distance(left: WorldCell, right: WorldCell) -> i64 {
    i64::from(left.x.abs_diff(right.x)) + i64::from(left.y.abs_diff(right.y))
}

fn attach<T: EntityComponent>(
    state: &mut EntityState,
    entity: EntityId,
    component: T,
) -> Result<(), WorldStateError> {
    let revision = state
        .component_revision::<T>(entity)
        .map_err(|detail| error("world_component_revision", detail.to_string()))?;
    EntityAuthoringService
        .attach_component(state, revision, entity, component)
        .map_err(|detail| error("world_component_attach", detail.to_string()))?;
    Ok(())
}

fn replace<T: EntityComponent + PartialEq>(
    state: &mut EntityState,
    entity: EntityId,
    component: T,
) -> Result<(), WorldStateError> {
    let revision = state
        .component_revision::<T>(entity)
        .map_err(|detail| error("world_component_revision", detail.to_string()))?;
    EntityAuthoringService
        .replace_component(state, revision, entity, component)
        .map_err(|detail| error("world_component_replace", detail.to_string()))?;
    Ok(())
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
