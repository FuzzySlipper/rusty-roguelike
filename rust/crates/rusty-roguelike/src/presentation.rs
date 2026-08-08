use rusty_engine::render_host_contracts::{
    RendererCameraPose, RendererCameraProjection, RendererCompositionCamera,
    RendererCompositionTarget, RendererCompositionView, RendererTargetColor, RendererTargetDepth,
    RendererTargetSampling, RendererViewComposition, RendererViewTarget, RendererViewport,
    RENDERER_VIEW_COMPOSITION_SCHEMA_VERSION,
};
use rusty_engine::render_model::{
    Geometry, LightDescriptor, LightShadowIntent, Material, RenderDiff, RenderFrameDiff,
    RenderFrameError, RenderHandle, RenderLayer, RenderMetadata, RenderNode, Transform,
};

use crate::{RoguelikeId, SessionView, TurnReceipt, VisibleSceneContent, WorldViewCellKind};

const CELL_SIZE: f32 = 2.4;
const DUNGEON_VIEW_CAMERA_ID: &str = "camera.dungeon-local-overview";
const DUNGEON_VIEW_TARGET_ID: &str = "target.dungeon-local-overview";

#[derive(Debug, Clone, PartialEq)]
pub struct DungeonFrame {
    pub frame: RenderFrameDiff,
    pub handles: Vec<RenderHandle>,
}

pub fn create_dungeon_frame(
    session: &SessionView,
    previous_handles: &[RenderHandle],
    selected_action_id: Option<&RoguelikeId>,
) -> Result<DungeonFrame, RenderFrameError> {
    let mut ops = previous_handles
        .iter()
        .copied()
        .map(|handle| RenderDiff::Destroy { handle })
        .collect::<Vec<_>>();
    let mut handles = Vec::new();

    for cell in &session.world.cells {
        let x = f32::from(cell.lateral) * CELL_SIZE;
        let z = -f32::from(cell.depth) * CELL_SIZE;
        let cell_index = i64::from(cell.depth) * 13 + i64::from(cell.lateral) + 6;
        debug_assert!(cell_index >= 0, "admitted local cell index is non-negative");
        let base = 100 + u64::try_from(cell_index).expect("admitted local cell index") * 4;
        match cell.kind {
            WorldViewCellKind::Floor
            | WorldViewCellKind::LockedDoorForward
            | WorldViewCellKind::LockedDoorSide => {
                create_cuboid(
                    &mut ops,
                    &mut handles,
                    base,
                    format!("floor-{}-{}", cell.lateral, cell.depth),
                    [x, -0.12, z],
                    [CELL_SIZE - 0.06, 0.24, CELL_SIZE - 0.06],
                    [0.28, 0.265, 0.225, 1.0],
                    None,
                    &["dungeon-floor", "rusty-roguelike"],
                );
                create_cuboid(
                    &mut ops,
                    &mut handles,
                    base + 1,
                    format!("ceiling-{}-{}", cell.lateral, cell.depth),
                    [x, 3.05, z],
                    [CELL_SIZE - 0.06, 0.18, CELL_SIZE - 0.06],
                    [0.21, 0.2, 0.17, 1.0],
                    None,
                    &["dungeon-ceiling", "rusty-roguelike"],
                );
                if matches!(
                    cell.kind,
                    WorldViewCellKind::LockedDoorForward | WorldViewCellKind::LockedDoorSide
                ) {
                    let forward = cell.kind == WorldViewCellKind::LockedDoorForward;
                    create_cuboid(
                        &mut ops,
                        &mut handles,
                        base + 2,
                        format!("locked-door-{}-{}", cell.lateral, cell.depth),
                        [x, 1.42, z],
                        if forward {
                            [CELL_SIZE - 0.26, 2.84, 0.24]
                        } else {
                            [0.24, 2.84, CELL_SIZE - 0.26]
                        },
                        [0.27, 0.13, 0.055, 1.0],
                        None,
                        &["dungeon-barrier", "locked-door", "rusty-roguelike"],
                    );
                }
            }
            WorldViewCellKind::Wall => create_cuboid(
                &mut ops,
                &mut handles,
                base + 2,
                format!("wall-{}-{}", cell.lateral, cell.depth),
                [x, 1.48, z],
                [CELL_SIZE, 3.2, CELL_SIZE],
                [0.4, 0.375, 0.315, 1.0],
                None,
                &["dungeon-wall", "rusty-roguelike"],
            ),
        }
    }

    for (index, placement) in session.world.scene_placements.iter().enumerate() {
        let x = f32::from(placement.lateral) * CELL_SIZE;
        let z = -f32::from(placement.depth) * CELL_SIZE;
        match &placement.content {
            VisibleSceneContent::Prop { .. } => create_cuboid(
                &mut ops,
                &mut handles,
                30_000 + index as u64,
                placement.id.clone(),
                [x, 1.05, z],
                [0.18, 1.4, 0.18],
                [0.34, 0.16, 0.055, 1.0],
                None,
                &["dungeon-prop", "rusty-roguelike", "torch"],
            ),
            VisibleSceneContent::PointLight {
                color_rgb,
                intensity_milli,
                range_cells,
            } => {
                let handle = RenderHandle::new(20_000 + index as u64);
                handles.push(handle);
                ops.push(RenderDiff::CreateLight {
                    handle,
                    parent: None,
                    light: LightDescriptor::Point {
                        color: parse_rgb(color_rgb),
                        intensity: *intensity_milli as f32 / 1_000.0,
                        enabled: true,
                        position: [x, 2.05, z],
                        range: Some(*range_cells as f32 * CELL_SIZE),
                        decay: 2.0,
                        shadow_intent: LightShadowIntent::Requested,
                    },
                });
            }
        }
    }

    let hit_target = session.latest_receipts.iter().rev().find_map(|receipt| {
        if let TurnReceipt::PartyAttacked {
            target_entity_id, ..
        } = receipt
        {
            Some(*target_entity_id)
        } else {
            None
        }
    });
    let legal_targets = session
        .decision
        .as_ref()
        .and_then(|decision| {
            decision.actions.iter().find(|action| {
                selected_action_id.is_some_and(|selected| selected == &action.action_id)
            })
        })
        .map(|action| action.legal_target_entity_ids.as_slice())
        .unwrap_or_default();

    for actor in &session.world.visible_actors {
        let x = f32::from(actor.lateral) * CELL_SIZE;
        let z = -f32::from(actor.depth) * CELL_SIZE;
        let base = 10_000 + actor.entity_id * 2;
        let targeted = legal_targets.contains(&actor.entity_id);
        let color = if hit_target == Some(actor.entity_id) {
            [1.0, 0.48, 0.12, 1.0]
        } else if targeted {
            [0.82, 0.58, 0.16, 1.0]
        } else {
            [0.55, 0.16, 0.105, 1.0]
        };
        let tags = if targeted {
            vec!["enemy", "legal-target", "rusty-roguelike"]
        } else {
            vec!["enemy", "rusty-roguelike"]
        };
        create_cuboid(
            &mut ops,
            &mut handles,
            base,
            format!("enemy-{}", actor.entity_id),
            [x, 0.92, z],
            [0.9, 1.75, 0.58],
            color,
            Some(actor.entity_id),
            &tags,
        );
        create_cuboid(
            &mut ops,
            &mut handles,
            base + 1,
            format!("enemy-head-{}", actor.entity_id),
            [x, 2.03, z],
            [0.62, 0.62, 0.62],
            color,
            Some(actor.entity_id),
            &tags,
        );
    }

    Ok(DungeonFrame {
        frame: RenderFrameDiff::try_from_ops(ops)?,
        handles,
    })
}

pub fn create_dungeon_view_composition(
    target_revision: u64,
    compact: bool,
) -> RendererViewComposition {
    let target_size = if compact { 128 } else { 256 };
    RendererViewComposition {
        schema_version: RENDERER_VIEW_COMPOSITION_SCHEMA_VERSION,
        cameras: vec![RendererCompositionCamera {
            id: DUNGEON_VIEW_CAMERA_ID.to_owned(),
            pose: RendererCameraPose {
                position: [0.0, 15.0, 0.0],
                pitch_degrees: -90.0,
                yaw_degrees: 0.0,
            },
            projection: RendererCameraProjection::Orthographic {
                vertical_size: 22.0,
                near: 0.1,
                far: 32.0,
            },
        }],
        targets: vec![RendererCompositionTarget {
            id: DUNGEON_VIEW_TARGET_ID.to_owned(),
            revision: target_revision,
            width: target_size,
            height: target_size,
            color: RendererTargetColor::Rgba8Srgb,
            depth: RendererTargetDepth::Depth24,
            sampling: RendererTargetSampling::Nearest,
        }],
        views: vec![RendererCompositionView {
            id: "view.dungeon-local-overview".to_owned(),
            camera_id: DUNGEON_VIEW_CAMERA_ID.to_owned(),
            target: RendererViewTarget::Offscreen {
                target_id: DUNGEON_VIEW_TARGET_ID.to_owned(),
                target_revision,
            },
            viewport: RendererViewport {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            order: 10,
        }],
        presentations: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_cuboid(
    ops: &mut Vec<RenderDiff>,
    handles: &mut Vec<RenderHandle>,
    raw_handle: u64,
    label: String,
    translation: [f32; 3],
    scale: [f32; 3],
    color: [f32; 4],
    source_entity: Option<u64>,
    tags: &[&str],
) {
    let handle = RenderHandle::new(raw_handle);
    handles.push(handle);
    ops.push(RenderDiff::Create {
        handle,
        parent: None,
        node: RenderNode {
            geometry: Geometry::Cube,
            material: Material {
                color,
                wireframe: false,
            },
            transform: Transform {
                translation,
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale,
            },
            visible: true,
            layer: RenderLayer::Scene,
            metadata: RenderMetadata {
                source_entity,
                source_scene_node: None,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                label: Some(label),
            },
        },
    });
}

fn parse_rgb(value: &str) -> [f32; 3] {
    let bytes = value.strip_prefix('#').unwrap_or(value).as_bytes();
    if bytes.len() != 6 {
        return [1.0, 0.55, 0.2];
    }
    let component = |offset| {
        std::str::from_utf8(&bytes[offset..offset + 2])
            .ok()
            .and_then(|component| u8::from_str_radix(component, 16).ok())
            .map(|component| f32::from(component) / 255.0)
            .unwrap_or(1.0)
    };
    [component(0), component(2), component(4)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_authored_floor, starter_ruleset, GameSession, WorldState};

    #[test]
    fn rust_projection_builds_a_valid_renderer_frame_and_view() {
        let session = GameSession::new(
            WorldState::new(
                generate_authored_floor(5_201).unwrap(),
                starter_ruleset().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let view = session.view().unwrap();
        let projected = create_dungeon_frame(&view, &[], None).unwrap();
        assert!(!projected.handles.is_empty());
        projected.frame.validate().unwrap();

        let composition = create_dungeon_view_composition(1, false);
        composition.validate().unwrap();
        assert_eq!(composition.targets[0].width, 256);
    }

    #[test]
    fn replacement_frame_destroys_every_previous_handle_first() {
        let session = GameSession::new(
            WorldState::new(
                generate_authored_floor(5_201).unwrap(),
                starter_ruleset().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let view = session.view().unwrap();
        let initial = create_dungeon_frame(&view, &[], None).unwrap();
        let replacement = create_dungeon_frame(&view, &initial.handles, None).unwrap();
        assert!(replacement
            .frame
            .ops
            .iter()
            .take(initial.handles.len())
            .all(|operation| matches!(operation, RenderDiff::Destroy { .. })));
    }
}
