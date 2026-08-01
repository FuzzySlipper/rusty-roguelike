use std::collections::BTreeMap;

use gameplay_mechanics::{
    CapacityMetricDefinition, CapacityMetricId, CatalogError, CatalogVersion, DamageKindDefinition,
    DamageKindId, EquipmentSlotDefinition, EquipmentSlotId, ItemCapacityCost, ItemClassificationId,
    ItemDefinition as MechanicsItemDefinition, ItemDefinitionId, ItemEquipmentPolicy, ItemKind,
    MechanicsCatalog, MechanicsCatalogDefinition, MechanicsScalar, SourceDefinition,
    SourceDefinitionId, StackingGroupId, StackingPolicy, StatContribution,
    StatContributionDefinition, StatDefinition, StatId, TrackDefinition, TrackId, TrackMaximum,
};

use super::{
    DefenseDefinition, EquipmentSlotCandidate, FeatDefinition, ItemDefinition, RoguelikeId,
};

pub fn defense_stat_id(id: &RoguelikeId) -> StatId {
    StatId::parse(format!("defense.{id}")).expect("admitted Roguelike ids form mechanics ids")
}

pub fn vitality_maximum_stat_id() -> StatId {
    StatId::parse("vitality.maximum").expect("fixed mechanics id is valid")
}

pub fn vitality_track_id() -> TrackId {
    TrackId::parse("vitality").expect("fixed mechanics id is valid")
}

pub fn feat_source_id(id: &RoguelikeId) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("feat.{id}"))
        .expect("admitted Roguelike ids form mechanics ids")
}

pub fn item_source_id(id: &RoguelikeId) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("item.{id}"))
        .expect("admitted Roguelike ids form mechanics ids")
}

pub fn item_definition_id(id: &RoguelikeId) -> ItemDefinitionId {
    ItemDefinitionId::parse(format!("item.{id}"))
        .expect("admitted Roguelike ids form mechanics ids")
}

pub fn equipment_slot_id(slot: EquipmentSlotCandidate) -> EquipmentSlotId {
    EquipmentSlotId::parse(match slot {
        EquipmentSlotCandidate::Body => "body",
        EquipmentSlotCandidate::Weapon => "weapon",
        EquipmentSlotCandidate::Focus => "focus",
    })
    .expect("fixed mechanics id is valid")
}

fn classification_id(slot: EquipmentSlotCandidate) -> ItemClassificationId {
    ItemClassificationId::parse(format!("equipment.{}", equipment_slot_id(slot).as_str()))
        .expect("fixed mechanics id is valid")
}

fn inventory_capacity_id() -> CapacityMetricId {
    CapacityMetricId::parse("inventory.slots").expect("fixed mechanics id is valid")
}

fn scalar(value: i64) -> MechanicsScalar {
    MechanicsScalar::new(value).expect("bounded compiler values fit Engine scalar")
}

fn modifier_source(
    id: SourceDefinitionId,
    namespace: &str,
    modifiers: &[super::StatModifierDefinition],
) -> SourceDefinition {
    SourceDefinition {
        id,
        priority: 0,
        stat_contributions: modifiers
            .iter()
            .map(|modifier| StatContributionDefinition {
                stat: defense_stat_id(&modifier.defense),
                contribution: StatContribution::Add {
                    amount: scalar(i64::from(modifier.amount)),
                },
                stacking_group: StackingGroupId::parse(format!("{namespace}.{}", modifier.defense))
                    .expect("admitted Roguelike ids form mechanics ids"),
                stacking: StackingPolicy::UniqueBySource,
            })
            .collect(),
        damage_responses: vec![],
    }
}

pub(super) fn build_mechanics_catalog(
    defenses: &BTreeMap<RoguelikeId, DefenseDefinition>,
    damage_types: &BTreeMap<RoguelikeId, super::DefinitionOrigin>,
    feats: &BTreeMap<RoguelikeId, FeatDefinition>,
    items: &BTreeMap<RoguelikeId, ItemDefinition>,
) -> Result<MechanicsCatalog, CatalogError> {
    let feat_sources = feats.values().map(|feat| {
        modifier_source(
            feat_source_id(&feat.id),
            &format!("feat.{}", feat.id),
            &feat.modifiers,
        )
    });
    let item_sources = items
        .values()
        .filter(|item| !item.modifiers.is_empty())
        .map(|item| {
            modifier_source(
                item_source_id(&item.id),
                &format!("item.{}", item.id),
                &item.modifiers,
            )
        });

    MechanicsCatalog::admit(MechanicsCatalogDefinition {
        version: CatalogVersion::parse("rusty-roguelike.v1")
            .expect("fixed catalog version is valid"),
        stats: defenses
            .values()
            .map(|defense| StatDefinition {
                id: defense_stat_id(&defense.id),
                minimum: scalar(-100),
                maximum: scalar(100),
            })
            .chain(std::iter::once(StatDefinition {
                id: vitality_maximum_stat_id(),
                minimum: scalar(1),
                maximum: scalar(10_000),
            }))
            .collect(),
        tracks: vec![TrackDefinition {
            id: vitality_track_id(),
            minimum: scalar(0),
            maximum: TrackMaximum::Stat {
                stat: vitality_maximum_stat_id(),
            },
        }],
        sources: feat_sources.chain(item_sources).collect(),
        damage_kinds: damage_types
            .keys()
            .map(|id| DamageKindDefinition {
                id: DamageKindId::parse(format!("damage.{id}"))
                    .expect("admitted Roguelike ids form mechanics ids"),
            })
            .collect(),
        effects: vec![],
        capacity_metrics: vec![CapacityMetricDefinition {
            id: inventory_capacity_id(),
        }],
        items: items
            .values()
            .map(|item| MechanicsItemDefinition {
                id: item_definition_id(&item.id),
                kind: ItemKind::Unique,
                maximum_quantity: 1,
                classifications: item.slot.map(classification_id).into_iter().collect(),
                capacity_costs: vec![ItemCapacityCost {
                    metric: inventory_capacity_id(),
                    units: 1,
                }],
                equipment: item.slot.map(|_| ItemEquipmentPolicy {
                    required_slots: 1,
                    exclusive_group: None,
                }),
                sources: (!item.modifiers.is_empty())
                    .then(|| item_source_id(&item.id))
                    .into_iter()
                    .collect(),
            })
            .collect(),
        equipment_slots: [
            EquipmentSlotCandidate::Body,
            EquipmentSlotCandidate::Weapon,
            EquipmentSlotCandidate::Focus,
        ]
        .into_iter()
        .map(|slot| EquipmentSlotDefinition {
            id: equipment_slot_id(slot),
            allowed_classifications: vec![classification_id(slot)],
        })
        .collect(),
    })
}
