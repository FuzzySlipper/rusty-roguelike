use std::collections::{BTreeMap, BTreeSet};

use gameplay_mechanics::CatalogError;
use gameplay_rules::{
    resolve_rule_packages, AdmittedRulePackage, RulePackageError, RulePackageSetError,
    RuleSubjectId, MAX_SAFE_JSON_INTEGER,
};

use super::mechanics::build_mechanics_catalog;
use super::*;

#[derive(Debug)]
pub enum RoguelikeCompileError {
    PackageSet(RulePackageSetError),
    Package(RulePackageError),
    InvalidPayload(String),
    Semantic { path: String, reason: String },
    MechanicsCatalog(CatalogError),
}

impl std::fmt::Display for RoguelikeCompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Roguelike rules compilation failed: {self:?}")
    }
}

impl std::error::Error for RoguelikeCompileError {}

impl RoguelikeRuleset {
    pub fn compile(packages: Vec<AdmittedRulePackage>) -> Result<Self, RoguelikeCompileError> {
        let resolved =
            resolve_rule_packages(packages).map_err(RoguelikeCompileError::PackageSet)?;
        if resolved.packages().len() != 1 {
            return semantic(
                "$/packages",
                "the v1 Roguelike catalog must be supplied by exactly one admitted package",
            );
        }
        let package = &resolved.packages()[0];
        let candidate: RoguelikeRulesCandidate = serde_json::from_value(package.payload().clone())
            .map_err(|error| RoguelikeCompileError::InvalidPayload(error.to_string()))?;
        compile_candidate(package, candidate)
    }
}

fn compile_candidate(
    package: &AdmittedRulePackage,
    candidate: RoguelikeRulesCandidate,
) -> Result<RoguelikeRuleset, RoguelikeCompileError> {
    if candidate.schema_version != ROGUELIKE_CANDIDATE_SCHEMA_VERSION {
        return semantic(
            "$/payload/schemaVersion",
            format!(
                "expected schema version {ROGUELIKE_CANDIDATE_SCHEMA_VERSION}, found {}",
                candidate.schema_version
            ),
        );
    }
    for (name, actual) in [
        ("abilities", candidate.abilities.len()),
        ("defenses", candidate.defenses.len()),
        ("damageTypes", candidate.damage_types.len()),
        ("actions", candidate.actions.len()),
        ("feats", candidate.feats.len()),
        ("classes", candidate.classes.len()),
        ("items", candidate.items.len()),
        ("actors", candidate.actors.len()),
    ] {
        if actual == 0 || actual > MAX_ROGUELIKE_DEFINITIONS_PER_KIND {
            return semantic(
                format!("$/payload/{name}"),
                format!(
                    "requires 1..={MAX_ROGUELIKE_DEFINITIONS_PER_KIND} definitions, found {actual}"
                ),
            );
        }
    }
    validate_roll_policy(&candidate.roll_policy)?;

    let mut abilities = BTreeMap::new();
    for value in candidate.abilities {
        if value.minimum < 1 || value.maximum > 30 || value.minimum > value.maximum {
            return semantic(
                format!("$/payload/abilities/{}", value.id),
                "ability bounds must be ordered inside 1..=30",
            );
        }
        let origin = definition_origin(package, "ability", &value.id)?;
        insert_unique(
            &mut abilities,
            value.id.clone(),
            AbilityDefinition {
                id: value.id,
                minimum: value.minimum,
                maximum: value.maximum,
                origin,
            },
            "abilities",
        )?;
    }

    let mut defenses = BTreeMap::new();
    for value in candidate.defenses {
        if !(-20..=40).contains(&value.base)
            || value.abilities.is_empty()
            || value.abilities.len() > 2
            || !is_unique(&value.abilities)
            || value.abilities.iter().any(|id| !abilities.contains_key(id))
        {
            return semantic(
                format!("$/payload/defenses/{}", value.id),
                "defense requires a bounded base and one or two distinct known abilities",
            );
        }
        let origin = definition_origin(package, "defense", &value.id)?;
        insert_unique(
            &mut defenses,
            value.id.clone(),
            DefenseDefinition {
                id: value.id,
                base: value.base,
                abilities: value.abilities,
                origin,
            },
            "defenses",
        )?;
    }

    let mut damage_types = BTreeMap::new();
    for value in candidate.damage_types {
        let origin = definition_origin(package, "damage-type", &value.id)?;
        insert_unique(&mut damage_types, value.id, origin, "damageTypes")?;
    }

    let mut actions = BTreeMap::new();
    for value in candidate.actions {
        validate_text(&value.name, "action name")?;
        if value.tags.len() > MAX_ROGUELIKE_ACTION_TAGS || !is_unique(&value.tags) {
            return semantic(
                format!("$/payload/actions/{}/tags", value.id),
                "action tags must be distinct and within the authored quota",
            );
        }
        let effect = match (value.movement, value.attack) {
            (Some(movement), None) => {
                if movement.steps != 1 || value.target != ActionTargetCandidate::SelfOnly {
                    return semantic(
                        format!("$/payload/actions/{}", value.id),
                        "movement must move exactly one grid step and target self",
                    );
                }
                ActionEffectDefinition::Movement { steps: 1 }
            }
            (None, Some(attack)) => {
                if !abilities.contains_key(&attack.ability)
                    || !defenses.contains_key(&attack.defense)
                    || !damage_types.contains_key(&attack.damage.kind)
                    || attack.damage.dice == 0
                    || attack.damage.dice > MAX_ROGUELIKE_DAMAGE_DICE
                    || attack.damage.sides < 2
                    || attack.damage.sides > MAX_ROGUELIKE_DAMAGE_DIE_SIDES
                    || !(-100..=100).contains(&attack.damage.bonus)
                    || attack.range == 0
                    || attack.range > MAX_ROGUELIKE_RANGE
                    || value.target != ActionTargetCandidate::HostileCell
                {
                    return semantic(
                        format!("$/payload/actions/{}", value.id),
                        "attack references or bounds are invalid for one hostile-cell activation",
                    );
                }
                ActionEffectDefinition::Attack {
                    ability: attack.ability,
                    defense: attack.defense,
                    damage: DamageDefinition {
                        kind: attack.damage.kind,
                        dice: attack.damage.dice,
                        sides: attack.damage.sides,
                        bonus: attack.damage.bonus,
                    },
                    range: attack.range,
                }
            }
            _ => {
                return semantic(
                    format!("$/payload/actions/{}", value.id),
                    "an action must define exactly one movement or attack effect",
                )
            }
        };
        let origin = definition_origin(package, "action", &value.id)?;
        insert_unique(
            &mut actions,
            value.id.clone(),
            ActionDefinition {
                id: value.id,
                name: value.name,
                tags: value.tags,
                target: value.target,
                effect,
                activation_cost: 1,
                origin,
            },
            "actions",
        )?;
    }

    let mut feats = BTreeMap::new();
    for value in candidate.feats {
        validate_text(&value.name, "feat name")?;
        validate_text(&value.description, "feat description")?;
        let modifiers = compile_modifiers(value.modifiers, &defenses, "feat", &value.id)?;
        let origin = definition_origin(package, "feat", &value.id)?;
        insert_unique(
            &mut feats,
            value.id.clone(),
            FeatDefinition {
                id: value.id,
                name: value.name,
                description: value.description,
                modifiers,
                origin,
            },
            "feats",
        )?;
    }

    let mut classes = BTreeMap::new();
    for value in candidate.classes {
        validate_text(&value.name, "class name")?;
        if value.levels.is_empty() || value.levels.len() > 20 {
            return semantic(
                format!("$/payload/classes/{}/levels", value.id),
                "class requires 1..=20 contiguous levels",
            );
        }
        let mut levels = Vec::with_capacity(value.levels.len());
        for (index, level) in value.levels.into_iter().enumerate() {
            if usize::from(level.level) != index + 1
                || !is_unique(&level.actions)
                || !is_unique(&level.feats)
                || level.actions.iter().any(|id| !actions.contains_key(id))
                || level.feats.iter().any(|id| !feats.contains_key(id))
                || level.action_slot_increase > 8
                || level.feat_slot_increase > 8
            {
                return semantic(
                    format!("$/payload/classes/{}/levels/{}", value.id, index),
                    "class levels must be contiguous with distinct known grants and bounded slots",
                );
            }
            levels.push(ClassLevelDefinition {
                level: level.level,
                actions: level.actions,
                feats: level.feats,
                action_slot_increase: level.action_slot_increase,
                feat_slot_increase: level.feat_slot_increase,
            });
        }
        let origin = definition_origin(package, "class", &value.id)?;
        insert_unique(
            &mut classes,
            value.id.clone(),
            ClassDefinition {
                id: value.id,
                name: value.name,
                levels,
                origin,
            },
            "classes",
        )?;
    }

    let mut items = BTreeMap::new();
    for value in candidate.items {
        validate_text(&value.name, "item name")?;
        if value
            .grants_action
            .as_ref()
            .is_some_and(|id| !actions.contains_key(id))
        {
            return semantic(
                format!("$/payload/items/{}/grantsAction", value.id),
                "item references an unknown action",
            );
        }
        let modifiers = compile_modifiers(value.modifiers, &defenses, "item", &value.id)?;
        let origin = definition_origin(package, "item", &value.id)?;
        insert_unique(
            &mut items,
            value.id.clone(),
            ItemDefinition {
                id: value.id,
                name: value.name,
                slot: value.slot,
                grants_action: value.grants_action,
                modifiers,
                origin,
            },
            "items",
        )?;
    }

    let ability_ids = abilities.keys().cloned().collect::<BTreeSet<_>>();
    let mut actors = BTreeMap::new();
    let mut entity_ids = BTreeSet::new();
    for value in candidate.actors {
        if value.entity_id == 0
            || value.entity_id > MAX_SAFE_JSON_INTEGER
            || !entity_ids.insert(value.entity_id)
            || value.level == 0
            || value.level > 20
            || value.class_level != value.level
            || value.experience > MAX_ROGUELIKE_EXPERIENCE
            || value.vitality == 0
            || value.inventory_capacity == 0
            || value.inventory_capacity > 64
            || value.items.len() > usize::from(value.inventory_capacity)
        {
            return semantic(
                format!("$/payload/actors/{}", value.id),
                "actor identity, progression, vitality, or inventory bounds are invalid",
            );
        }
        validate_text(&value.name, "actor name")?;
        validate_text(&value.title, "actor title")?;
        let Some(class) = classes.get(&value.class) else {
            return semantic(
                format!("$/payload/actors/{}/class", value.id),
                "actor references an unknown class",
            );
        };
        if usize::from(value.class_level) > class.levels.len()
            || !is_unique_by(&value.abilities, |score| &score.ability)
            || value
                .abilities
                .iter()
                .map(|score| score.ability.clone())
                .collect::<BTreeSet<_>>()
                != ability_ids
            || value.abilities.iter().any(|score| {
                let ability = &abilities[&score.ability];
                !(ability.minimum..=ability.maximum).contains(&score.score)
            })
            || !is_unique(&value.actions)
            || !is_unique(&value.feats)
            || !is_unique(&value.items)
            || value.items.iter().any(|id| !items.contains_key(id))
        {
            return semantic(
                format!("$/payload/actors/{}", value.id),
                "actor abilities, selected grants, or item references are invalid",
            );
        }
        let granted_actions = class
            .levels
            .iter()
            .take(usize::from(value.class_level))
            .flat_map(|level| level.actions.iter().cloned())
            .chain(
                value
                    .items
                    .iter()
                    .filter_map(|id| items[id].grants_action.clone()),
            )
            .collect::<BTreeSet<_>>();
        let granted_feats = class
            .levels
            .iter()
            .take(usize::from(value.class_level))
            .flat_map(|level| level.feats.iter().cloned())
            .collect::<BTreeSet<_>>();
        let action_slots: usize = class
            .levels
            .iter()
            .take(usize::from(value.class_level))
            .map(|level| usize::from(level.action_slot_increase))
            .sum();
        let feat_slots: usize = class
            .levels
            .iter()
            .take(usize::from(value.class_level))
            .map(|level| usize::from(level.feat_slot_increase))
            .sum();
        let occupied_slots = value
            .items
            .iter()
            .filter_map(|id| items[id].slot)
            .collect::<Vec<_>>();
        if value.actions.len() > action_slots
            || value.feats.len() > feat_slots
            || value.actions.iter().any(|id| !granted_actions.contains(id))
            || value.feats.iter().any(|id| !granted_feats.contains(id))
            || !is_unique(&occupied_slots)
        {
            return semantic(
                format!("$/payload/actors/{}", value.id),
                "actor exceeds class slots, selects ungranted entries, or double-equips a slot",
            );
        }
        let origin = definition_origin(package, "actor", &value.id)?;
        insert_unique(
            &mut actors,
            value.id.clone(),
            ActorDefinition {
                id: value.id,
                entity_id: value.entity_id,
                name: value.name,
                title: value.title,
                side: value.side,
                level: value.level,
                experience: value.experience,
                vitality: value.vitality,
                inventory_capacity: value.inventory_capacity,
                class: value.class,
                class_level: value.class_level,
                abilities: value
                    .abilities
                    .into_iter()
                    .map(|score| AbilityScoreDefinition {
                        ability: score.ability,
                        score: score.score,
                    })
                    .collect(),
                actions: value.actions,
                feats: value.feats,
                items: value.items,
                origin,
            },
            "actors",
        )?;
    }

    if candidate.party.entity_id == 0
        || candidate.party.entity_id > MAX_SAFE_JSON_INTEGER
        || !entity_ids.insert(candidate.party.entity_id)
        || !(2..=4).contains(&candidate.party.members.len())
        || !is_unique(&candidate.party.members)
        || candidate.party.members.iter().any(|id| {
            !actors
                .get(id)
                .is_some_and(|actor| actor.side == ActorSideCandidate::Party)
        })
        || actors
            .values()
            .filter(|actor| actor.side == ActorSideCandidate::Party)
            .map(|actor| &actor.id)
            .collect::<BTreeSet<_>>()
            != candidate.party.members.iter().collect::<BTreeSet<_>>()
    {
        return semantic(
            "$/payload/party",
            "party must name every and only 2..=4 distinct party-side actors with a unique entity id",
        );
    }
    let party_origin = definition_origin(package, "party", &candidate.party.id)?;
    let party = PartyDefinition {
        id: candidate.party.id,
        entity_id: candidate.party.entity_id,
        members: candidate.party.members,
        origin: party_origin,
    };
    let mechanics = build_mechanics_catalog(&defenses, &damage_types, &feats, &items)
        .map_err(RoguelikeCompileError::MechanicsCatalog)?;

    Ok(RoguelikeRuleset {
        fingerprint: format!("{}={}", package.identity(), package.fingerprint()),
        roll_policy: candidate.roll_policy,
        abilities,
        defenses,
        damage_types,
        actions,
        feats,
        classes,
        items,
        actors,
        party,
        mechanics,
    })
}

fn validate_roll_policy(policy: &RollPolicyCandidate) -> Result<(), RoguelikeCompileError> {
    match policy.kind {
        RollPolicyKindCandidate::Seeded if policy.seed.is_some() && policy.rolls.is_empty() => Ok(()),
        RollPolicyKindCandidate::Static
            if policy.seed.is_none()
                && !policy.rolls.is_empty()
                && policy.rolls.len() <= MAX_ROGUELIKE_STATIC_ROLLS
                && policy.rolls.iter().all(|roll| {
                    (1..=20).contains(&roll.d20)
                        && !roll.damage.is_empty()
                        && roll.damage.len() <= MAX_ROGUELIKE_DAMAGE_DICE as usize
                        && roll
                            .damage
                            .iter()
                            .all(|value| (1..=MAX_ROGUELIKE_DAMAGE_DIE_SIDES).contains(value))
                }) =>
        {
            Ok(())
        }
        _ => semantic(
            "$/payload/rollPolicy",
            "seeded policy requires only a seed; static policy requires bounded positive rolls only",
        ),
    }
}

fn compile_modifiers(
    values: Vec<StatModifierCandidate>,
    defenses: &BTreeMap<RoguelikeId, DefenseDefinition>,
    kind: &str,
    id: &RoguelikeId,
) -> Result<Vec<StatModifierDefinition>, RoguelikeCompileError> {
    if values.len() > 8
        || !is_unique_by(&values, |modifier| &modifier.defense)
        || values.iter().any(|modifier| {
            !defenses.contains_key(&modifier.defense) || !(-20..=20).contains(&modifier.amount)
        })
    {
        return semantic(
            format!("$/payload/{kind}s/{id}/modifiers"),
            "modifiers must reference distinct known defenses with amounts inside -20..=20",
        );
    }
    Ok(values
        .into_iter()
        .map(|modifier| StatModifierDefinition {
            defense: modifier.defense,
            amount: modifier.amount,
        })
        .collect())
}

fn definition_origin(
    package: &AdmittedRulePackage,
    kind: &str,
    id: &RoguelikeId,
) -> Result<DefinitionOrigin, RoguelikeCompileError> {
    let subject =
        RuleSubjectId::parse(format!("{kind}:{id}")).map_err(RoguelikeCompileError::Package)?;
    let Some((provenance, source)) = package.correlated_source(&subject) else {
        return semantic(
            format!("$/payload/{kind}/{id}"),
            "definition has no exact package provenance",
        );
    };
    Ok(DefinitionOrigin {
        package: package.identity().clone(),
        source_path: source.path().to_owned(),
        line: provenance.line(),
        column: provenance.column(),
    })
}

fn validate_text(value: &str, field: &str) -> Result<(), RoguelikeCompileError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_ROGUELIKE_AUTHORED_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return semantic(
            format!("$/payload/{field}"),
            "authored text must be nonempty, trimmed, printable, and bounded",
        );
    }
    Ok(())
}

fn insert_unique<K: Ord + std::fmt::Display, V>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    family: &str,
) -> Result<(), RoguelikeCompileError> {
    if map.insert(key, value).is_some() {
        return semantic(
            format!("$/payload/{family}"),
            "duplicate definition identity",
        );
    }
    Ok(())
}

fn is_unique<T: Ord>(values: &[T]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn is_unique_by<T, K: Ord>(values: &[T], key: impl Fn(&T) -> &K) -> bool {
    values.iter().map(key).collect::<BTreeSet<_>>().len() == values.len()
}

fn semantic<T>(
    path: impl Into<String>,
    reason: impl Into<String>,
) -> Result<T, RoguelikeCompileError> {
    Err(RoguelikeCompileError::Semantic {
        path: path.into(),
        reason: reason.into(),
    })
}
