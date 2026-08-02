use std::collections::BTreeSet;

use core_ids::EntityId;
use entity_state::{
    EntityAuthoringService, EntityComponent, EntityDefinition, EntityState, EntityStateSnapshot,
};
use gameplay_mechanics::{
    validate_state_against_catalog, ActiveEffectsComponent, EquipmentComponent,
    IntrinsicSourceBinding, IntrinsicSourcesComponent, InventoryCapacityLimit, InventoryComponent,
    ItemComponent, MechanicsScalar, SourceInstanceId, StatValue, StatsComponent, TrackValue,
    TracksComponent,
};
use serde::{Deserialize, Serialize};

use crate::{
    defense_stat_id, feat_source_id, inventory_capacity_id, item_definition_id,
    vitality_maximum_stat_id, vitality_track_id, AbilityScore, AbilityScoresComponent,
    ActorBuildComponent, ActorDefinition, ActorSideCandidate, CollapsedPartyComponent,
    FloorFeatureKind, GeneratedFloor, RoguelikeId, RoguelikeRuleset,
};

use super::navigation::FloorSpatial;
use super::projection::project_world;
use super::{
    EnemyParticipation, EnemyWorldComponent, Facing, PartyExplorationComponent, RelativeStep,
    WorldCell, WorldStateError, WorldView,
};

pub const WORLD_DURABLE_SCHEMA_VERSION: u32 = 2;

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
    stash_entity: EntityId,
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
        let stash_entity = next_entity_id(&rules, 1)?;
        let item_instances = initial_item_instances(&rules, stash_entity)?;
        let definitions = rules
            .actors()
            .values()
            .map(|actor| EntityDefinition::new(EntityId::new(actor.entity_id), actor.name.clone()))
            .chain(std::iter::once(EntityDefinition::new(
                party_entity,
                rules.party().id.to_string(),
            )))
            .chain(std::iter::once(EntityDefinition::new(
                stash_entity,
                "Expedition Stash",
            )))
            .chain(item_instances.iter().map(|item| {
                EntityDefinition::new(item.entity, rules.items()[&item.definition].name.clone())
                    .with_containment(item.owner)
            }))
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
            attach_mechanics(&mut entities, &rules, actor)?;
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
            attach_inventory(
                &mut entities,
                &rules,
                entity,
                u64::from(actor.inventory_capacity),
            )?;
        }
        attach_inventory(&mut entities, &rules, stash_entity, 32)?;
        for item in &item_instances {
            attach(
                &mut entities,
                item.entity,
                ItemComponent::new(
                    rules.mechanics().version().clone(),
                    item_definition_id(&item.definition),
                ),
            )?;
        }
        validate_state_against_catalog(&entities, rules.mechanics())
            .map_err(|detail| error("world_mechanics_seed", detail.to_string()))?;

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
        let visible = spatial.visible_terrain(entry, Facing::North);
        attach(
            &mut entities,
            party_entity,
            PartyExplorationComponent::new(
                floor.floor_id.clone(),
                entry,
                Facing::North,
                visible.floor,
                visible.walls,
            )
            .map_err(|detail| error("world_party_seed", detail))?,
        )?;

        let opposition = rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
            .collect::<Vec<_>>();
        let placements = initial_enemy_positions(&floor, &spatial, entry, opposition.len())?;
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
            stash_entity,
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
            durable.party.discovered_walls().to_vec(),
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
        for cell in party.discovered_walls() {
            state.spatial.require_wall(*cell)?;
        }
        let visible = state
            .spatial
            .visible_terrain(party.position(), party.facing());
        if visible
            .floor
            .iter()
            .any(|cell| party.discovered().binary_search(cell).is_err())
            || visible
                .walls
                .iter()
                .any(|cell| party.discovered_walls().binary_search(cell).is_err())
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

    pub fn restore_snapshot(
        floor: GeneratedFloor,
        rules: RoguelikeRuleset,
        snapshot: EntityStateSnapshot,
    ) -> Result<Self, WorldStateError> {
        let canonical = Self::new(floor.clone(), rules.clone())?;
        validate_snapshot_core(&snapshot, &canonical)?;
        let registry = super::roguelike_world_component_registry()
            .map_err(|detail| error("world_component_registry", detail.to_string()))?;
        let entities = EntityState::from_snapshot_with_registry(snapshot, registry)
            .map_err(|detail| error("world_snapshot_invalid", detail.to_string()))?;
        validate_state_against_catalog(&entities, rules.mechanics())
            .map_err(|detail| error("world_mechanics_restore", detail.to_string()))?;
        validate_immutable_components(&entities, canonical.entities())?;

        let spatial = FloorSpatial::build(&floor)?;
        let state = Self {
            floor,
            rules,
            entities,
            spatial,
            party_entity: canonical.party_entity,
            stash_entity: canonical.stash_entity,
        };
        state.validate_restored_world_state()?;
        state.validate_loadout_state()?;
        state.validate_vitality_state()?;
        state.validate_current_visibility()?;
        Ok(state)
    }

    pub fn entity_snapshot(&self) -> EntityStateSnapshot {
        self.entities.durable_snapshot()
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
            let entity = EntityId::new(actor.entity_id);
            if self.actor_alive(entity)? && self.enemy(entity)?.position() == destination {
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

    pub(crate) fn floor(&self) -> &GeneratedFloor {
        &self.floor
    }

    pub(crate) fn fork(&self) -> Result<Self, WorldStateError> {
        Ok(Self {
            floor: self.floor.clone(),
            rules: self.rules.clone(),
            entities: self.entities.clone(),
            spatial: FloorSpatial::build(&self.floor)?,
            party_entity: self.party_entity,
            stash_entity: self.stash_entity,
        })
    }

    pub(crate) fn rules(&self) -> &RoguelikeRuleset {
        &self.rules
    }

    pub(crate) fn entities_mut(&mut self) -> &mut EntityState {
        &mut self.entities
    }

    pub(crate) fn party_position(&self) -> Result<WorldCell, WorldStateError> {
        Ok(self.party()?.position())
    }

    pub(crate) fn enemy_position(&self, entity: EntityId) -> Result<WorldCell, WorldStateError> {
        Ok(self.enemy(entity)?.position())
    }

    pub(crate) fn participating_enemies(&self) -> Result<Vec<EntityId>, WorldStateError> {
        self.rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
            .filter_map(|actor| {
                let entity = EntityId::new(actor.entity_id);
                match self.enemy(entity) {
                    Ok(world) if world.participation() == EnemyParticipation::Participating => {
                        Some(Ok(entity))
                    }
                    Ok(_) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect()
    }

    pub(crate) fn clear_distance(&self, origin: WorldCell, target: WorldCell) -> Option<u32> {
        self.spatial.clear_distance(origin, target)
    }

    pub(crate) fn move_enemy_toward_party(
        &mut self,
        entity: EntityId,
    ) -> Result<bool, WorldStateError> {
        let enemy = self.enemy(entity)?.clone();
        let party = self.party_position()?;
        let mut occupied = BTreeSet::from([party]);
        for actor in self
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        {
            let other = EntityId::new(actor.entity_id);
            if other != entity && self.actor_alive(other)? {
                occupied.insert(self.enemy_position(other)?);
            }
        }
        let Some(destination) =
            self.spatial
                .next_step_toward(enemy.position(), party, &occupied)?
        else {
            return Ok(false);
        };
        if occupied.contains(&destination) {
            return Err(error(
                "world_enemy_step_overlap",
                "opposition navigation selected an occupied actor cell",
            ));
        }
        let mut staged = self.entities.clone();
        replace(&mut staged, entity, enemy.with_position(destination))?;
        self.entities = staged;
        Ok(true)
    }

    pub const fn party_entity(&self) -> EntityId {
        self.party_entity
    }

    pub(crate) const fn stash_entity(&self) -> EntityId {
        self.stash_entity
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
            .visible_terrain(party.position(), party.facing());
        let mut discovered = party.discovered().iter().copied().collect::<BTreeSet<_>>();
        discovered.extend(visible.floor.iter().copied());
        let mut discovered_walls = party
            .discovered_walls()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        discovered_walls.extend(visible.walls.iter().copied());
        replace(
            entities,
            self.party_entity,
            party.with_discovered(
                discovered.into_iter().collect(),
                discovered_walls.into_iter().collect(),
            ),
        )?;
        let visible = visible.floor.into_iter().collect::<BTreeSet<_>>();
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
            .visible_terrain(party.position(), party.facing())
            .floor
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

    fn validate_loadout_state(&self) -> Result<(), WorldStateError> {
        let party = self
            .rules
            .party()
            .members
            .iter()
            .map(|id| EntityId::new(self.rules.actors()[id].entity_id))
            .collect::<BTreeSet<_>>();
        let mut owners = party.clone();
        owners.insert(self.stash_entity);
        for (item, _) in self
            .entities
            .components::<ItemComponent>()
            .map_err(|detail| error("world_item_restore", detail.to_string()))?
        {
            let owner = self.entities.contained_in(item).ok_or_else(|| {
                error(
                    "world_item_owner_missing",
                    format!("item {item} has no durable owner"),
                )
            })?;
            let actor = self
                .rules
                .actors()
                .values()
                .find(|actor| EntityId::new(actor.entity_id) == owner);
            if !owners.contains(&owner)
                && !actor.is_some_and(|actor| actor.side == ActorSideCandidate::Opposition)
            {
                return Err(error(
                    "world_item_owner_invalid",
                    format!("item {item} has invalid owner {owner}"),
                ));
            }
        }
        for owner in owners {
            let inventory = gameplay_mechanics::InventoryService::view(
                &self.entities,
                self.rules.mechanics(),
                owner,
            )
            .map_err(|detail| error("world_inventory_restore", detail.to_string()))?;
            if owner != self.stash_entity {
                let equipment = component::<EquipmentComponent>(&self.entities, owner)?;
                for assignment in equipment.assignments() {
                    if self.entities.contained_in(assignment.item) != Some(owner)
                        || !inventory
                            .unique_items()
                            .iter()
                            .any(|item| item.entity == assignment.item)
                    {
                        return Err(error(
                            "world_equipment_restore_invalid",
                            format!("owner {owner} equips an item outside its inventory"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_restored_world_state(&self) -> Result<(), WorldStateError> {
        let entry = entry_cell(&self.floor)?;
        let party = self.party()?;
        if party.floor_id() != self.floor.floor_id {
            return Err(error(
                "world_floor_mismatch",
                "durable party does not identify the admitted floor",
            ));
        }
        self.spatial.require_reachable(entry, party.position())?;
        let canonical_party = PartyExplorationComponent::new(
            party.floor_id().to_owned(),
            party.position(),
            party.facing(),
            party.discovered().to_vec(),
            party.discovered_walls().to_vec(),
        )
        .map_err(|detail| error("world_party_restore_invalid", detail))?;
        if &canonical_party != party {
            return Err(error(
                "world_discovery_not_canonical",
                "durable discovery cells must already be sorted and unique",
            ));
        }
        for cell in party.discovered() {
            self.spatial.require_reachable(entry, *cell)?;
        }
        for cell in party.discovered_walls() {
            self.spatial.require_wall(*cell)?;
        }
        let visible = self
            .spatial
            .visible_terrain(party.position(), party.facing());
        if visible
            .floor
            .iter()
            .any(|cell| party.discovered().binary_search(cell).is_err())
            || visible
                .walls
                .iter()
                .any(|cell| party.discovered_walls().binary_search(cell).is_err())
        {
            return Err(error(
                "world_discovery_incomplete",
                "durable discovery omits currently visible terrain cells",
            ));
        }

        let mut occupied = BTreeSet::new();
        for actor in self
            .rules
            .actors()
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        {
            let entity = EntityId::new(actor.entity_id);
            let enemy = self.enemy(entity)?;
            if enemy.floor_id() != self.floor.floor_id {
                return Err(error(
                    "world_enemy_position_invalid",
                    "enemy placement has a floor mismatch",
                ));
            }
            self.spatial.require_reachable(entry, enemy.position())?;
            let alive = self.actor_alive(entity)?;
            if alive && (enemy.position() == party.position() || !occupied.insert(enemy.position()))
            {
                return Err(error(
                    "world_enemy_position_invalid",
                    "living enemy placement overlaps the party or another living enemy",
                ));
            }
            if enemy.participation() == EnemyParticipation::Dormant
                && (!alive || party.discovered().binary_search(&enemy.position()).is_ok())
            {
                return Err(error(
                    "world_dormancy_forged",
                    "a dead or discovered enemy cannot remain dormant",
                ));
            }
        }
        Ok(())
    }

    fn validate_vitality_state(&self) -> Result<(), WorldStateError> {
        for actor in self.rules.actors().values() {
            let entity = EntityId::new(actor.entity_id);
            let tracks = component::<TracksComponent>(&self.entities, entity)?;
            if tracks.values().len() != 1
                || tracks.values()[0].track() != &vitality_track_id()
                || tracks.values()[0].current().get() < 0
                || tracks.values()[0].current().get() > i64::from(actor.vitality)
            {
                return Err(error(
                    "world_vitality_restore_invalid",
                    format!("entity {entity} has impossible vitality"),
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

    fn actor_alive(&self, entity: EntityId) -> Result<bool, WorldStateError> {
        let tracks = component::<TracksComponent>(&self.entities, entity)?;
        Ok(tracks
            .current(&vitality_track_id())
            .is_some_and(|value| value.get() > 0))
    }
}

fn validate_snapshot_core(
    snapshot: &EntityStateSnapshot,
    canonical: &WorldState,
) -> Result<(), WorldStateError> {
    let expected = canonical.entities.durable_snapshot();
    if snapshot.schema_version != expected.schema_version
        || snapshot.entities.len() != expected.entities.len()
    {
        return Err(error(
            "world_entity_roster_mismatch",
            "durable entity roster does not match compiled content",
        ));
    }
    let item_ids = canonical
        .entities
        .components::<ItemComponent>()
        .map_err(|detail| error("world_item_restore", detail.to_string()))?
        .map(|(entity, _)| entity.raw())
        .collect::<BTreeSet<_>>();
    for (actual, authored) in snapshot.entities.iter().zip(expected.entities.iter()) {
        let mut authored = authored.clone();
        if item_ids.contains(&actual.id) {
            authored.contained_in = actual.contained_in;
        }
        if actual != &authored {
            return Err(error(
                "world_entity_identity_mismatch",
                format!(
                    "durable entity {} disagrees with compiled content",
                    actual.id
                ),
            ));
        }
    }
    Ok(())
}

fn validate_immutable_components(
    actual: &EntityState,
    canonical: &EntityState,
) -> Result<(), WorldStateError> {
    macro_rules! require_equal {
        ($component:ty) => {{
            let actual = actual
                .components::<$component>()
                .map_err(|detail| error("world_component_restore", detail.to_string()))?
                .map(|(entity, value)| (entity, value.clone()))
                .collect::<Vec<_>>();
            let expected = canonical
                .components::<$component>()
                .map_err(|detail| error("world_component_restore", detail.to_string()))?
                .map(|(entity, value)| (entity, value.clone()))
                .collect::<Vec<_>>();
            if actual != expected {
                return Err(error(
                    "world_component_identity_mismatch",
                    <$component>::LABEL,
                ));
            }
        }};
    }
    require_equal!(AbilityScoresComponent);
    require_equal!(ActorBuildComponent);
    require_equal!(CollapsedPartyComponent);
    require_equal!(StatsComponent);
    require_equal!(IntrinsicSourcesComponent);
    require_equal!(ActiveEffectsComponent);
    require_equal!(ItemComponent);
    Ok(())
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
    spatial: &FloorSpatial,
    entry: WorldCell,
    count: usize,
) -> Result<Vec<WorldCell>, WorldStateError> {
    let prohibited = floor
        .features
        .iter()
        .map(|feature| WorldCell::from(&feature.cell))
        .chain(
            floor
                .portals
                .iter()
                .flat_map(|portal| portal.cells.iter().map(WorldCell::from)),
        )
        .chain(std::iter::once(entry))
        .collect::<BTreeSet<_>>();
    let mut candidates = floor
        .walkable_cells
        .iter()
        .map(WorldCell::from)
        .filter(|cell| !prohibited.contains(cell))
        .map(|cell| Ok((spatial.path_distance(entry, cell)?, cell)))
        .collect::<Result<Vec<_>, WorldStateError>>()?;
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    if candidates.len() < count {
        return Err(error(
            "world_enemy_placement_exhausted",
            "floor has too few distinct enemy placement cells",
        ));
    }
    let available = candidates.len();
    Ok((0..count)
        .map(|index| {
            let rank = if count == 1 {
                0
            } else {
                index * (available - 1) / (count - 1)
            };
            candidates[rank].1
        })
        .collect())
}

#[derive(Debug, Clone)]
struct InitialItemInstance {
    entity: EntityId,
    definition: RoguelikeId,
    owner: EntityId,
}

fn next_entity_id(rules: &RoguelikeRuleset, offset: u64) -> Result<EntityId, WorldStateError> {
    let maximum = rules
        .actors()
        .values()
        .map(|actor| actor.entity_id)
        .chain(std::iter::once(rules.party().entity_id))
        .max()
        .unwrap_or_default();
    maximum
        .checked_add(offset)
        .map(EntityId::new)
        .ok_or_else(|| {
            error(
                "world_entity_identity_exhausted",
                "entity identity overflowed",
            )
        })
}

fn initial_item_instances(
    rules: &RoguelikeRuleset,
    stash: EntityId,
) -> Result<Vec<InitialItemInstance>, WorldStateError> {
    let party = rules.party().members.iter().map(|id| &rules.actors()[id]);
    let opposition = rules
        .actors()
        .values()
        .filter(|actor| actor.side == ActorSideCandidate::Opposition);
    party
        .chain(opposition)
        .flat_map(|actor| {
            actor
                .items
                .iter()
                .cloned()
                .map(move |definition| (definition, actor.side, EntityId::new(actor.entity_id)))
        })
        .enumerate()
        .map(|(index, (definition, side, actor))| {
            let offset = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(2))
                .ok_or_else(|| {
                    error(
                        "world_item_identity_exhausted",
                        "item instance identity overflowed",
                    )
                })?;
            Ok(InitialItemInstance {
                entity: next_entity_id(rules, offset)?,
                definition,
                owner: if side == ActorSideCandidate::Party {
                    stash
                } else {
                    actor
                },
            })
        })
        .collect()
}

fn attach_mechanics(
    entities: &mut EntityState,
    rules: &RoguelikeRuleset,
    actor: &ActorDefinition,
) -> Result<(), WorldStateError> {
    let entity = EntityId::new(actor.entity_id);
    let catalog_version = rules.mechanics().version().clone();
    let ability = |id: &crate::RoguelikeId| {
        actor
            .abilities
            .iter()
            .find(|score| &score.ability == id)
            .map(|score| ability_modifier(score.score))
            .unwrap_or_default()
    };
    let mut stats = rules
        .defenses()
        .values()
        .map(|defense| {
            let modifier = defense
                .abilities
                .iter()
                .map(ability)
                .max()
                .unwrap_or_default();
            StatValue::new(
                defense_stat_id(&defense.id),
                MechanicsScalar::new(i64::from(defense.base + modifier))
                    .expect("compiled defense values fit Engine scalar"),
            )
        })
        .collect::<Vec<_>>();
    stats.push(StatValue::new(
        vitality_maximum_stat_id(),
        MechanicsScalar::new(i64::from(actor.vitality))
            .expect("compiled vitality fits Engine scalar"),
    ));
    attach(
        entities,
        entity,
        StatsComponent::new(catalog_version.clone(), stats)
            .map_err(|detail| error("world_mechanics_seed", detail.to_string()))?,
    )?;
    attach(
        entities,
        entity,
        TracksComponent::new(
            catalog_version.clone(),
            vec![TrackValue::new(
                vitality_track_id(),
                MechanicsScalar::new(i64::from(actor.vitality))
                    .expect("compiled vitality fits Engine scalar"),
            )],
        )
        .map_err(|detail| error("world_mechanics_seed", detail.to_string()))?,
    )?;
    attach(
        entities,
        entity,
        IntrinsicSourcesComponent::new(
            catalog_version.clone(),
            actor
                .feats
                .iter()
                .map(|feat| {
                    IntrinsicSourceBinding::new(
                        SourceInstanceId::parse(format!("feat.{feat}"))
                            .expect("compiled feat ids form mechanics ids"),
                        feat_source_id(feat),
                    )
                })
                .collect(),
        )
        .map_err(|detail| error("world_mechanics_seed", detail.to_string()))?,
    )?;
    attach(
        entities,
        entity,
        ActiveEffectsComponent::new(catalog_version.clone(), vec![])
            .map_err(|detail| error("world_mechanics_seed", detail.to_string()))?,
    )?;
    attach(
        entities,
        entity,
        EquipmentComponent::new(catalog_version, vec![])
            .map_err(|detail| error("world_mechanics_seed", detail.to_string()))?,
    )?;
    Ok(())
}

fn attach_inventory(
    entities: &mut EntityState,
    rules: &RoguelikeRuleset,
    owner: EntityId,
    maximum: u64,
) -> Result<(), WorldStateError> {
    attach(
        entities,
        owner,
        InventoryComponent::with_capacity_limits(
            rules.mechanics().version().clone(),
            vec![],
            vec![InventoryCapacityLimit::new(
                inventory_capacity_id(),
                maximum,
            )],
        )
        .map_err(|detail| error("world_inventory_seed", detail.to_string()))?,
    )
}

fn ability_modifier(score: i16) -> i16 {
    (score - 10).div_euclid(2)
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

pub(crate) fn component<T: EntityComponent>(
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
