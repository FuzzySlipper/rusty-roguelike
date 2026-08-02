use std::collections::{BTreeMap, BTreeSet};

use core_ids::EntityId;
use gameplay_mechanics::{
    EquipmentComponent, EquipmentEquipRequest, EquipmentService, EquipmentSlotId,
    EquipmentUnequipRequest, InventoryService, ItemComponent, ItemTransferRequest, MechanicsError,
    OperationId, SourceInstanceId, SourceInstanceIdentity, StatService,
};

use crate::{
    defense_stat_id, equipment_slot_id, inventory_capacity_id, item_definition_id,
    AbilityScoresComponent, ActorBuildComponent, EquipmentSlotCandidate, ItemDefinition,
    RoguelikeId,
};

use super::runtime::{error, GameSession};
use super::{
    AbilityReadoutView, CharacterActionView, DefenseReadoutView, EquipmentSlotView,
    FeatReadoutView, LoadoutCapacityView, LoadoutItemView, LoadoutView, PartyMemberStatusView,
    PreparationView, SessionError, SessionPhase, TurnReceipt,
};

impl GameSession {
    pub(super) fn initialize_canonical_loadout(&mut self) -> Result<(), SessionError> {
        if self.loadout_ready()? {
            return Ok(());
        }
        let stash = self.world.stash_entity();
        let inventory =
            InventoryService::view(self.world.entities(), self.world.rules().mechanics(), stash)
                .map_err(mechanics_error)?;
        let mut available = BTreeMap::new();
        for item in inventory.unique_items() {
            let component = self
                .world
                .entities()
                .component::<ItemComponent>(item.entity)
                .map_err(|detail| error("session_item_read", detail.to_string()))?
                .ok_or_else(|| {
                    error(
                        "session_initial_loadout_item_missing",
                        format!("stash item {} has no item component", item.entity),
                    )
                })?;
            available
                .entry(component.definition().clone())
                .or_insert_with(Vec::new)
                .push(item.entity);
        }
        let mut assignments = Vec::new();

        for actor_id in &self.world.rules().party().members {
            let actor = &self.world.rules().actors()[actor_id];
            let owner = EntityId::new(actor.entity_id);
            for definition_id in &actor.items {
                let expected = item_definition_id(definition_id);
                let item = available
                    .get_mut(&expected)
                    .and_then(|items| (!items.is_empty()).then(|| items.remove(0)))
                    .ok_or_else(|| {
                        error(
                            "session_initial_loadout_item_missing",
                            format!("party item {definition_id} is absent from the shared stash"),
                        )
                    })?;
                let slot = self.world.rules().items()[definition_id]
                    .slot
                    .map(equipment_slot_id)
                    .ok_or_else(|| {
                        error(
                            "session_initial_loadout_slot_missing",
                            format!("party item {definition_id} has no equipment slot"),
                        )
                    })?;
                assignments.push((item, owner, slot));
            }
        }

        let catalog = self.world.rules().mechanics().clone();
        for (index, (item, owner, slot)) in assignments.into_iter().enumerate() {
            let transfer_operation = operation(&format!("initial-loadout-{index}-transfer"))?;
            let transfer_source = request_source(&transfer_operation, "initial-loadout-transfer")?;
            let expected_relationship_revision = self.world.entities().revision();
            EquipmentService::transfer_unique_item(
                self.world.entities_mut(),
                &catalog,
                ItemTransferRequest {
                    operation: transfer_operation,
                    source: transfer_source,
                    item,
                    from_owner: stash,
                    to_owner: owner,
                    expected_relationship_revision,
                    expected_from_inventory_revision: None,
                    expected_to_inventory_revision: None,
                },
            )
            .map_err(mechanics_error)?;

            let equip_operation = operation(&format!("initial-loadout-{index}-equip"))?;
            let equip_source = request_source(&equip_operation, "initial-loadout-equip")?;
            let expected_state_revision = self.world.entities().revision();
            EquipmentService::equip(
                self.world.entities_mut(),
                &catalog,
                EquipmentEquipRequest {
                    operation: equip_operation,
                    source: equip_source,
                    owner,
                    item,
                    slots: vec![slot],
                    expected_equipment_revision: None,
                    expected_state_revision,
                },
            )
            .map_err(mechanics_error)?;
        }

        if !self.loadout_ready()? {
            return Err(error(
                "session_initial_loadout_incomplete",
                "canonical party equipment did not produce a ready preparation state",
            ));
        }
        Ok(())
    }

    pub(super) fn preparation_view(&self) -> Result<Option<PreparationView>, SessionError> {
        if self.phase != SessionPhase::Preparation {
            return Ok(None);
        }
        Ok(Some(PreparationView {
            stash: self.project_loadout(self.world.stash_entity(), false)?,
            ready: self.loadout_ready()?,
        }))
    }

    pub(super) fn party_status(&self) -> Result<Vec<PartyMemberStatusView>, SessionError> {
        self.world
            .rules()
            .party()
            .members
            .iter()
            .map(|actor_id| self.project_party_member(actor_id))
            .collect()
    }

    fn project_party_member(
        &self,
        actor_id: &RoguelikeId,
    ) -> Result<PartyMemberStatusView, SessionError> {
        let actor = &self.world.rules().actors()[actor_id];
        let entity = EntityId::new(actor.entity_id);
        let current = self
            .world
            .entities()
            .component::<gameplay_mechanics::TracksComponent>(entity)
            .map_err(|detail| error("session_tracks_read", detail.to_string()))?
            .and_then(|tracks| tracks.current(&crate::vitality_track_id()))
            .ok_or_else(|| error("session_vitality_missing", format!("entity {entity}")))?
            .get();
        let operation = operation(&format!(
            "party-view-{}-revision-{}",
            actor.entity_id, self.revision
        ))?;
        let maximum = StatService::evaluate(
            self.world.entities(),
            self.world.rules().mechanics(),
            entity,
            &crate::vitality_maximum_stat_id(),
            &operation,
            &[],
        )
        .map_err(mechanics_error)?
        .value
        .get();
        let scores = self
            .world
            .entities()
            .component::<AbilityScoresComponent>(entity)
            .map_err(|detail| error("session_abilities_read", detail.to_string()))?
            .ok_or_else(|| error("session_abilities_missing", format!("entity {entity}")))?;
        let build = self
            .world
            .entities()
            .component::<ActorBuildComponent>(entity)
            .map_err(|detail| error("session_build_read", detail.to_string()))?
            .ok_or_else(|| error("session_build_missing", format!("entity {entity}")))?;
        let class = &self.world.rules().classes()[build.class()];
        let defenses = self
            .world
            .rules()
            .defenses()
            .values()
            .map(|definition| {
                let evaluated = StatService::evaluate(
                    self.world.entities(),
                    self.world.rules().mechanics(),
                    entity,
                    &defense_stat_id(&definition.id),
                    &operation,
                    &[],
                )
                .map_err(mechanics_error)?;
                Ok(DefenseReadoutView {
                    defense_id: definition.id.clone(),
                    value: i16::try_from(evaluated.value.get()).map_err(|_| {
                        error(
                            "session_defense_out_of_range",
                            "evaluated defense does not fit i16",
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        Ok(PartyMemberStatusView {
            entity_id: actor.entity_id,
            actor_id: actor.id.clone(),
            name: actor.name.clone(),
            title: actor.title.clone(),
            level: actor.level,
            experience: actor.experience,
            class_id: class.id.clone(),
            class_name: class.name.clone(),
            class_level: build.class_level(),
            current_vitality: u16::try_from(current.max(0)).map_err(|_| {
                error(
                    "session_vitality_out_of_range",
                    "current vitality exceeds u16",
                )
            })?,
            maximum_vitality: u16::try_from(maximum.max(0)).map_err(|_| {
                error(
                    "session_vitality_out_of_range",
                    "maximum vitality exceeds u16",
                )
            })?,
            conscious: current > 0,
            abilities: scores
                .scores()
                .iter()
                .map(|score| AbilityReadoutView {
                    ability_id: score.id().clone(),
                    score: score.score(),
                    modifier: ability_modifier(score.score()),
                })
                .collect(),
            defenses,
            feats: build
                .feats()
                .iter()
                .map(|feat_id| {
                    let feat = &self.world.rules().feats()[feat_id];
                    FeatReadoutView {
                        feat_id: feat.id.clone(),
                        name: feat.name.clone(),
                        description: feat.description.clone(),
                    }
                })
                .collect(),
            actions: build
                .actions()
                .iter()
                .map(|action_id| {
                    let action = &self.world.rules().actions()[action_id];
                    CharacterActionView {
                        action_id: action.id.clone(),
                        name: action.name.clone(),
                    }
                })
                .collect(),
            loadout: self.project_loadout(entity, true)?,
        })
    }

    fn project_loadout(
        &self,
        owner: EntityId,
        include_equipment: bool,
    ) -> Result<LoadoutView, SessionError> {
        let inventory =
            InventoryService::view(self.world.entities(), self.world.rules().mechanics(), owner)
                .map_err(mechanics_error)?;
        let assignments = if include_equipment {
            self.world
                .entities()
                .component::<EquipmentComponent>(owner)
                .map_err(|detail| error("session_equipment_read", detail.to_string()))?
                .ok_or_else(|| error("session_equipment_missing", format!("entity {owner}")))?
                .assignments()
                .to_vec()
        } else {
            vec![]
        };
        let equipped = assignments
            .iter()
            .map(|assignment| (assignment.item, assignment.slot.to_string()))
            .collect::<BTreeMap<_, _>>();
        let mut items = inventory
            .unique_items()
            .iter()
            .map(|item| self.project_item(item.entity, equipped.get(&item.entity).cloned()))
            .collect::<Result<Vec<_>, _>>()?;
        items.sort_by_key(|item| item.entity_id);
        let capacity = inventory
            .capacity()
            .iter()
            .find(|usage| usage.metric == inventory_capacity_id())
            .ok_or_else(|| error("session_capacity_missing", format!("entity {owner}")))?;
        let maximum = capacity.maximum.ok_or_else(|| {
            error(
                "session_capacity_unbounded",
                "Roguelike inventories require a bounded slot maximum",
            )
        })?;
        let slot_count = usize::try_from(maximum).map_err(|_| {
            error(
                "session_capacity_out_of_range",
                "inventory maximum does not fit memory",
            )
        })?;
        if items.len() > slot_count {
            return Err(error(
                "session_capacity_projection_invalid",
                "inventory items exceed projected capacity",
            ));
        }
        let mut inventory_slots = items
            .iter()
            .cloned()
            .map(Some)
            .collect::<Vec<Option<LoadoutItemView>>>();
        inventory_slots.resize(slot_count, None);
        let mut by_slot = assignments
            .iter()
            .map(|assignment| {
                Ok((
                    assignment.slot.to_string(),
                    self.project_item(assignment.item, Some(assignment.slot.to_string()))?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, SessionError>>()?;
        let equipment_slots = if include_equipment {
            [
                EquipmentSlotCandidate::Body,
                EquipmentSlotCandidate::Weapon,
                EquipmentSlotCandidate::Focus,
            ]
            .into_iter()
            .map(|slot| {
                let id = equipment_slot_id(slot).to_string();
                EquipmentSlotView {
                    label: humanize(&id),
                    equipped: by_slot.remove(&id),
                    slot_id: id,
                }
            })
            .collect()
        } else {
            Vec::new()
        };
        Ok(LoadoutView {
            owner_entity_id: owner.raw(),
            inventory_slots,
            equipment_slots,
            capacity: LoadoutCapacityView {
                used: capacity.used,
                maximum,
            },
        })
    }

    fn project_item(
        &self,
        entity: EntityId,
        equipped_slot_id: Option<String>,
    ) -> Result<LoadoutItemView, SessionError> {
        let definition = self.item_definition(entity)?;
        Ok(LoadoutItemView {
            entity_id: entity.raw(),
            item_id: definition.id.clone(),
            name: definition.name.clone(),
            equipment_slot_id: definition
                .slot
                .map(equipment_slot_id)
                .map(|id| id.to_string()),
            equipped_slot_id,
        })
    }

    fn item_definition(&self, entity: EntityId) -> Result<&ItemDefinition, SessionError> {
        let component = self
            .world
            .entities()
            .component::<ItemComponent>(entity)
            .map_err(|detail| error("session_item_read", detail.to_string()))?
            .ok_or_else(|| error("session_item_missing", format!("entity {entity}")))?;
        self.world
            .rules()
            .items()
            .values()
            .find(|item| item_definition_id(&item.id) == *component.definition())
            .ok_or_else(|| {
                error(
                    "session_item_definition_unknown",
                    component.definition().to_string(),
                )
            })
    }

    fn loadout_ready(&self) -> Result<bool, SessionError> {
        let stash = InventoryService::view(
            self.world.entities(),
            self.world.rules().mechanics(),
            self.world.stash_entity(),
        )
        .map_err(mechanics_error)?;
        if !stash.unique_items().is_empty() {
            return Ok(false);
        }
        let party = self.party_entities();
        for owner in party {
            let inventory = InventoryService::view(
                self.world.entities(),
                self.world.rules().mechanics(),
                owner,
            )
            .map_err(mechanics_error)?;
            let equipment = self
                .world
                .entities()
                .component::<EquipmentComponent>(owner)
                .map_err(|detail| error("session_equipment_read", detail.to_string()))?
                .ok_or_else(|| error("session_equipment_missing", format!("entity {owner}")))?;
            let assigned = equipment
                .assignments()
                .iter()
                .map(|assignment| assignment.item)
                .collect::<BTreeSet<_>>();
            if inventory
                .unique_items()
                .iter()
                .any(|item| !assigned.contains(&item.entity))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn move_loadout_item(
        &mut self,
        item_entity_id: u64,
        from_owner_entity_id: u64,
        to_owner_entity_id: u64,
        destination_slot_id: Option<String>,
    ) -> Result<(), SessionError> {
        if self.phase != SessionPhase::Preparation {
            return Err(error(
                "session_loadout_phase_forbidden",
                "loadout changes are available only during preparation",
            ));
        }
        let item = EntityId::new(item_entity_id);
        let from = EntityId::new(from_owner_entity_id);
        let to = EntityId::new(to_owner_entity_id);
        let mut owners = self.party_entities();
        owners.insert(self.world.stash_entity());
        if !owners.contains(&from) || !owners.contains(&to) {
            return Err(error(
                "session_loadout_owner_forbidden",
                "loadout moves are limited to the party and shared stash",
            ));
        }
        let actual_owner = self.world.entities().contained_in(item);
        if actual_owner != Some(from) {
            return Err(error(
                "session_loadout_owner_stale",
                format!("item {item} is not contained by requested owner {from}"),
            ));
        }
        let definition = self.item_definition(item)?.clone();
        let catalog = self.world.rules().mechanics().clone();
        let destination_slot = destination_slot_id.as_deref().map(parse_slot).transpose()?;
        if to == self.world.stash_entity() && destination_slot.is_some() {
            return Err(error(
                "session_loadout_slot_forbidden",
                "the shared stash has no equipment slots",
            ));
        }
        if let Some(slot) = &destination_slot {
            let required = definition.slot.map(equipment_slot_id).ok_or_else(|| {
                error(
                    "session_loadout_item_not_equippable",
                    "the selected item has no equipment slot",
                )
            })?;
            if slot != &required {
                return Err(error(
                    "session_loadout_slot_invalid",
                    format!("item requires {required}, not {slot}"),
                ));
            }
        }
        let equipped = self
            .world
            .entities()
            .component::<EquipmentComponent>(from)
            .map_err(|detail| error("session_equipment_read", detail.to_string()))?
            .and_then(|equipment| {
                equipment
                    .assignments()
                    .iter()
                    .find(|assignment| assignment.item == item)
                    .cloned()
            });
        if from == to && equipped.is_none() && destination_slot.is_none() {
            return Err(error(
                "session_loadout_unchanged",
                "the item is already in that pack",
            ));
        }
        if from == to
            && equipped
                .as_ref()
                .zip(destination_slot.as_ref())
                .is_some_and(|(current, destination)| &current.slot == destination)
        {
            return Err(error(
                "session_loadout_unchanged",
                "the item already occupies that slot",
            ));
        }
        if equipped.is_some() {
            let operation = operation(&format!("loadout-{}-unequip", self.revision))?;
            let source = request_source(&operation, "loadout-unequip")?;
            let expected_state_revision = self.world.entities().revision();
            EquipmentService::unequip(
                self.world.entities_mut(),
                &catalog,
                EquipmentUnequipRequest {
                    operation,
                    source,
                    owner: from,
                    item,
                    expected_equipment_revision: None,
                    expected_state_revision,
                },
            )
            .map_err(mechanics_error)?;
        }
        if from != to {
            let operation = operation(&format!("loadout-{}-transfer", self.revision))?;
            let source = request_source(&operation, "loadout-transfer")?;
            let expected_relationship_revision = self.world.entities().revision();
            EquipmentService::transfer_unique_item(
                self.world.entities_mut(),
                &catalog,
                ItemTransferRequest {
                    operation,
                    source,
                    item,
                    from_owner: from,
                    to_owner: to,
                    expected_relationship_revision,
                    expected_from_inventory_revision: None,
                    expected_to_inventory_revision: None,
                },
            )
            .map_err(mechanics_error)?;
        }
        if let Some(slot) = destination_slot {
            let operation = operation(&format!("loadout-{}-equip", self.revision))?;
            let source = request_source(&operation, "loadout-equip")?;
            let expected_state_revision = self.world.entities().revision();
            EquipmentService::equip(
                self.world.entities_mut(),
                &catalog,
                EquipmentEquipRequest {
                    operation,
                    source,
                    owner: to,
                    item,
                    slots: vec![slot],
                    expected_equipment_revision: None,
                    expected_state_revision,
                },
            )
            .map_err(mechanics_error)?;
        }
        self.latest_receipts.push(TurnReceipt::LoadoutMoved {
            item_entity_id,
            from_owner_entity_id,
            to_owner_entity_id,
            destination_slot_id,
        });
        Ok(())
    }

    pub(super) fn begin_expedition(&mut self) -> Result<(), SessionError> {
        if self.phase != SessionPhase::Preparation {
            return Err(error(
                "session_expedition_already_started",
                "the expedition has already started",
            ));
        }
        if !self.loadout_ready()? {
            return Err(error(
                "session_preparation_incomplete",
                "equip every shared-stash item before beginning the expedition",
            ));
        }
        self.phase = SessionPhase::Expedition;
        self.rebuild_order()?;
        self.refresh_outcome()?;
        self.settle_automatic()?;
        self.latest_receipts.push(TurnReceipt::ExpeditionBegan);
        Ok(())
    }

    pub(super) fn actor_action_available(
        &self,
        actor_entity_id: u64,
        action_id: &RoguelikeId,
    ) -> Result<bool, SessionError> {
        let actor = self.actor(actor_entity_id)?;
        let class = &self.world.rules().classes()[&actor.class];
        if class
            .levels
            .iter()
            .take(usize::from(actor.class_level))
            .any(|level| level.actions.contains(action_id))
        {
            return Ok(true);
        }
        let required = self
            .world
            .rules()
            .items()
            .values()
            .filter(|item| item.grants_action.as_ref() == Some(action_id))
            .map(|item| item_definition_id(&item.id))
            .collect::<BTreeSet<_>>();
        if required.is_empty() {
            return Ok(false);
        }
        let entity = EntityId::new(actor.entity_id);
        let equipment = self
            .world
            .entities()
            .component::<EquipmentComponent>(entity)
            .map_err(|detail| error("session_equipment_read", detail.to_string()))?
            .ok_or_else(|| error("session_equipment_missing", format!("entity {entity}")))?;
        for assignment in equipment.assignments() {
            let component = self
                .world
                .entities()
                .component::<ItemComponent>(assignment.item)
                .map_err(|detail| error("session_item_read", detail.to_string()))?
                .ok_or_else(|| {
                    error(
                        "session_item_missing",
                        format!("entity {}", assignment.item),
                    )
                })?;
            if required.contains(component.definition()) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn party_entities(&self) -> BTreeSet<EntityId> {
        self.world
            .rules()
            .party()
            .members
            .iter()
            .map(|id| EntityId::new(self.world.rules().actors()[id].entity_id))
            .collect()
    }
}

fn ability_modifier(score: i16) -> i16 {
    (score - 10).div_euclid(2)
}

fn parse_slot(value: &str) -> Result<EquipmentSlotId, SessionError> {
    [
        EquipmentSlotCandidate::Body,
        EquipmentSlotCandidate::Weapon,
        EquipmentSlotCandidate::Focus,
    ]
    .into_iter()
    .map(equipment_slot_id)
    .find(|slot| slot.as_str() == value)
    .ok_or_else(|| error("session_loadout_slot_unknown", value.to_owned()))
}

fn operation(value: &str) -> Result<OperationId, SessionError> {
    OperationId::parse(value)
        .map_err(|detail| error("session_operation_invalid", detail.to_string()))
}

fn request_source(
    operation: &OperationId,
    label: &str,
) -> Result<SourceInstanceIdentity, SessionError> {
    Ok(SourceInstanceIdentity::Request {
        operation: operation.clone(),
        instance: SourceInstanceId::parse(label)
            .map_err(|detail| error("session_source_invalid", detail.to_string()))?,
    })
}

fn mechanics_error(detail: MechanicsError) -> SessionError {
    let code = match detail {
        MechanicsError::InventoryCapacityExceeded { .. }
        | MechanicsError::CapacityArithmeticOverflow { .. } => "session_loadout_capacity",
        MechanicsError::EquipmentSlotOccupied { .. }
        | MechanicsError::UnknownEquipmentSlot { .. }
        | MechanicsError::EquipmentSlotClassificationMismatch { .. } => {
            "session_loadout_slot_rejected"
        }
        _ => "session_loadout_rejected",
    };
    error(code, detail.to_string())
}

fn humanize(value: &str) -> String {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    format!(
        "{}{}",
        first.to_uppercase(),
        characters.as_str().replace('-', " ")
    )
}
