use std::collections::BTreeSet;

use core_space::{
    ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelCoord, VoxelGridSpec, WorldPos,
};
use core_voxel::VoxelValue;
use svc_collision::{CollisionProjection, Ray};
use svc_pathfinding::{
    build_nav_projection, find_path, NavPathOutcome, NavPathQuery, NavProjection,
    NavProjectionConfig,
};
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

use crate::GeneratedFloor;

use super::{Facing, WorldCell, WorldStateError, MAX_DISCOVERED_CELLS, MAX_VIEW_DEPTH};

pub(super) struct FloorSpatial {
    min_x: i32,
    min_y: i32,
    walkable: BTreeSet<WorldCell>,
    navigation: NavProjection,
    collision: CollisionProjection,
}

impl FloorSpatial {
    pub(super) fn build(floor: &GeneratedFloor) -> Result<Self, WorldStateError> {
        let dimensions = ChunkDims::new(floor.bounds.width, 3, floor.bounds.height)
            .ok_or_else(|| error("world_grid_invalid", "floor cannot form an Engine grid"))?;
        let grid = VoxelGridSpec::new(GridId::new(0x524C), 1.0, dimensions)
            .ok_or_else(|| error("world_grid_invalid", "floor cannot form an Engine grid"))?;
        let walkable = floor
            .walkable_cells
            .iter()
            .map(WorldCell::from)
            .collect::<BTreeSet<_>>();
        if walkable.len() != floor.walkable_cells.len() || walkable.len() > MAX_DISCOVERED_CELLS {
            return Err(error(
                "world_floor_not_canonical",
                "walkable cells must be unique and bounded",
            ));
        }
        let maximum_x = i64::from(floor.bounds.min_x) + i64::from(floor.bounds.width);
        let maximum_y = i64::from(floor.bounds.min_y) + i64::from(floor.bounds.height);
        if walkable.iter().any(|cell| {
            i64::from(cell.x) < i64::from(floor.bounds.min_x)
                || i64::from(cell.x) >= maximum_x
                || i64::from(cell.y) < i64::from(floor.bounds.min_y)
                || i64::from(cell.y) >= maximum_y
        }) {
            return Err(error(
                "world_position_outside_bounds",
                "walkable cells must remain inside the admitted floor bounds",
            ));
        }
        let mut chunk = VoxelChunk::from_spec(&grid);
        for local_y in 0..floor.bounds.height {
            for local_x in 0..floor.bounds.width {
                chunk
                    .set(
                        LocalVoxelCoord::new(local_x, 0, local_y),
                        VoxelValue::solid_raw(1),
                    )
                    .map_err(|detail| error("world_grid_invalid", detail.to_string()))?;
                let x = i32::try_from(local_x)
                    .ok()
                    .and_then(|local| floor.bounds.min_x.checked_add(local))
                    .ok_or_else(|| error("world_grid_invalid", "floor x coordinate overflows"))?;
                let y = i32::try_from(local_y)
                    .ok()
                    .and_then(|local| floor.bounds.min_y.checked_add(local))
                    .ok_or_else(|| error("world_grid_invalid", "floor y coordinate overflows"))?;
                let absolute = WorldCell { x, y };
                if !walkable.contains(&absolute) {
                    for height in 1..=2 {
                        chunk
                            .set(
                                LocalVoxelCoord::new(local_x, height, local_y),
                                VoxelValue::solid_raw(2),
                            )
                            .map_err(|detail| error("world_grid_invalid", detail.to_string()))?;
                    }
                }
            }
        }
        let mut world = VoxelWorld::new(grid);
        world.insert(ChunkCoord::ORIGIN, chunk);
        let navigation = build_nav_projection(
            &world,
            NavProjectionConfig {
                agent_height_voxels: 2,
                require_solid_floor: true,
            },
        )
        .map_err(|detail| error("world_navigation_invalid", format!("{detail:?}")))?;
        let collision = CollisionProjection::build(&world);
        Ok(Self {
            min_x: floor.bounds.min_x,
            min_y: floor.bounds.min_y,
            walkable,
            navigation,
            collision,
        })
    }

    pub(super) fn is_walkable(&self, cell: WorldCell) -> bool {
        self.walkable.contains(&cell)
    }

    pub(super) fn require_reachable(
        &self,
        start: WorldCell,
        goal: WorldCell,
    ) -> Result<(), WorldStateError> {
        if !self.is_walkable(start) || !self.is_walkable(goal) {
            return Err(error(
                "world_position_not_walkable",
                format!("position {},{} is not walkable", goal.x, goal.y),
            ));
        }
        let readout = find_path(
            &self.navigation,
            NavPathQuery {
                start: self.voxel(start),
                goal: self.voxel(goal),
                max_visited: self.walkable.len(),
            },
        )
        .map_err(|detail| error("world_navigation_failed", format!("{detail:?}")))?;
        if readout.outcome != NavPathOutcome::Reached {
            return Err(error(
                "world_position_disconnected",
                format!("position {},{} is disconnected", goal.x, goal.y),
            ));
        }
        Ok(())
    }

    pub(super) fn require_single_step(
        &self,
        start: WorldCell,
        goal: WorldCell,
    ) -> Result<(), WorldStateError> {
        self.require_reachable(start, goal)?;
        let readout = find_path(
            &self.navigation,
            NavPathQuery {
                start: self.voxel(start),
                goal: self.voxel(goal),
                max_visited: self.walkable.len(),
            },
        )
        .map_err(|detail| error("world_navigation_failed", format!("{detail:?}")))?;
        if readout.path.len() != 2 {
            return Err(error(
                "world_step_not_adjacent",
                "party movement must be exactly one Engine-routed grid step",
            ));
        }
        Ok(())
    }

    pub(super) fn path(
        &self,
        start: WorldCell,
        goal: WorldCell,
    ) -> Result<Vec<WorldCell>, WorldStateError> {
        self.require_reachable(start, goal)?;
        let readout = find_path(
            &self.navigation,
            NavPathQuery {
                start: self.voxel(start),
                goal: self.voxel(goal),
                max_visited: self.walkable.len(),
            },
        )
        .map_err(|detail| error("world_navigation_failed", format!("{detail:?}")))?;
        if readout.outcome != NavPathOutcome::Reached {
            return Err(error(
                "world_position_disconnected",
                format!("position {},{} is disconnected", goal.x, goal.y),
            ));
        }
        readout
            .path
            .into_iter()
            .map(|cell| {
                let x = i32::try_from(cell.x)
                    .ok()
                    .and_then(|local| self.min_x.checked_add(local))
                    .ok_or_else(|| error("world_grid_invalid", "path x coordinate overflows"))?;
                let y = i32::try_from(cell.z)
                    .ok()
                    .and_then(|local| self.min_y.checked_add(local))
                    .ok_or_else(|| error("world_grid_invalid", "path y coordinate overflows"))?;
                Ok(WorldCell { x, y })
            })
            .collect()
    }

    pub(super) fn clear_distance(&self, origin: WorldCell, target: WorldCell) -> Option<u32> {
        self.line_is_clear(origin, target).then(|| {
            origin
                .x
                .abs_diff(target.x)
                .saturating_add(origin.y.abs_diff(target.y))
        })
    }

    pub(super) fn visible_floor_cells(&self, origin: WorldCell, facing: Facing) -> Vec<WorldCell> {
        let mut visible = self
            .walkable
            .iter()
            .copied()
            .filter(|cell| {
                let (lateral, depth) = relative(origin, facing, *cell);
                (0..=MAX_VIEW_DEPTH).contains(&depth)
                    && lateral.abs() <= depth.max(1)
                    && self.line_is_clear(origin, *cell)
            })
            .collect::<Vec<_>>();
        visible.sort();
        visible
    }

    pub(super) fn first_visible_walls(
        &self,
        origin: WorldCell,
        facing: Facing,
        visible_floor: &[WorldCell],
    ) -> Vec<WorldCell> {
        let mut walls = BTreeSet::new();
        for floor in visible_floor {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let wall = WorldCell {
                    x: floor.x + dx,
                    y: floor.y + dy,
                };
                let (lateral, depth) = relative(origin, facing, wall);
                if !self.walkable.contains(&wall)
                    && (0..=MAX_VIEW_DEPTH).contains(&depth)
                    && lateral.abs() <= depth.max(1)
                    && self.wall_is_first_hit(origin, wall)
                {
                    walls.insert(wall);
                }
            }
        }
        walls.into_iter().collect()
    }

    fn line_is_clear(&self, origin: WorldCell, target: WorldCell) -> bool {
        if origin == target {
            return true;
        }
        let origin = self.world_position(origin);
        let target = self.world_position(target);
        let direction = target - origin;
        let distance = direction.length();
        self.collision
            .raycast(Ray::new(origin, direction), (distance - 0.01).max(0.01))
            .is_none()
    }

    fn wall_is_first_hit(&self, origin: WorldCell, wall: WorldCell) -> bool {
        let origin_position = self.world_position(origin);
        let wall_position = self.world_position(wall);
        let direction = wall_position - origin_position;
        let distance = direction.length();
        self.collision
            .raycast(Ray::new(origin_position, direction), distance + 0.01)
            .is_some_and(|hit| hit.voxel == self.voxel(wall))
    }

    fn voxel(&self, cell: WorldCell) -> VoxelCoord {
        VoxelCoord::new(
            i64::from(cell.x) - i64::from(self.min_x),
            1,
            i64::from(cell.y) - i64::from(self.min_y),
        )
    }

    fn world_position(&self, cell: WorldCell) -> WorldPos {
        WorldPos::new(
            f64::from(cell.x) - f64::from(self.min_x) + 0.5,
            1.5,
            f64::from(cell.y) - f64::from(self.min_y) + 0.5,
        )
    }
}

pub(super) fn relative(origin: WorldCell, facing: Facing, cell: WorldCell) -> (i32, i32) {
    let delta_x = cell.x - origin.x;
    let delta_y = cell.y - origin.y;
    let (forward_x, forward_y) = facing.forward();
    let (right_x, right_y) = facing.right_axis();
    (
        delta_x * right_x + delta_y * right_y,
        delta_x * forward_x + delta_y * forward_y,
    )
}

fn error(code: &'static str, detail: impl Into<String>) -> WorldStateError {
    WorldStateError::new(code, detail)
}
