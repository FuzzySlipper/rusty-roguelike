use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rusty_engine::core_space::{
    ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelCoord, VoxelGridSpec, WorldPos,
};
use rusty_engine::core_voxel::VoxelValue;
use rusty_engine::svc_collision::{CollisionProjection, Ray};
use rusty_engine::svc_pathfinding::{
    build_nav_projection, find_path, NavPathOutcome, NavPathQuery, NavProjection,
    NavProjectionConfig,
};
use rusty_engine::svc_spatial::VoxelWorld;
use rusty_engine::svc_volume::VoxelChunk;

use crate::GeneratedFloor;

use super::{Facing, WorldCell, WorldStateError, MAX_DISCOVERED_CELLS, MAX_VIEW_DEPTH};

pub(super) struct FloorSpatial {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    walkable: BTreeSet<WorldCell>,
    opaque_walkable: BTreeSet<WorldCell>,
    locked_door_north_south: BTreeMap<WorldCell, bool>,
    navigation: NavProjection,
    collision: CollisionProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VisibleTerrain {
    pub(super) floor: Vec<WorldCell>,
    pub(super) locked_doors_forward: Vec<WorldCell>,
    pub(super) locked_doors_side: Vec<WorldCell>,
    pub(super) walls: Vec<WorldCell>,
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
        let max_x = i32::try_from(maximum_x - 1)
            .map_err(|_| error("world_grid_invalid", "floor x bound overflows"))?;
        let max_y = i32::try_from(maximum_y - 1)
            .map_err(|_| error("world_grid_invalid", "floor y bound overflows"))?;
        let opaque_walkable = floor
            .portals
            .iter()
            .filter(|portal| portal.traversal == "locked")
            .flat_map(|portal| portal.cells.iter().map(WorldCell::from))
            .collect::<BTreeSet<_>>();
        let locked_door_north_south = floor
            .portals
            .iter()
            .filter(|portal| portal.traversal == "locked")
            .flat_map(|portal| {
                let north_south = matches!(portal.orientation.as_str(), "north" | "south");
                portal
                    .cells
                    .iter()
                    .map(move |cell| (WorldCell::from(cell), north_south))
            })
            .collect::<BTreeMap<_, _>>();
        if opaque_walkable.iter().any(|cell| !walkable.contains(cell)) {
            return Err(error(
                "world_floor_not_canonical",
                "locked portal opacity must bind admitted walkable cells",
            ));
        }
        Ok(Self {
            min_x: floor.bounds.min_x,
            min_y: floor.bounds.min_y,
            max_x,
            max_y,
            walkable,
            opaque_walkable,
            locked_door_north_south,
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

    pub(super) fn path_distance(
        &self,
        start: WorldCell,
        goal: WorldCell,
    ) -> Result<usize, WorldStateError> {
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
        readout.path.len().checked_sub(1).ok_or_else(|| {
            error(
                "world_navigation_failed",
                "Engine reached path did not contain its origin",
            )
        })
    }

    pub(super) fn require_wall(&self, cell: WorldCell) -> Result<(), WorldStateError> {
        if !self.inside_bounds(cell) || self.walkable.contains(&cell) {
            return Err(error(
                "world_discovered_wall_invalid",
                "discovered wall must be a bounded nonwalkable floor cell",
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

    pub(super) fn next_step_toward(
        &self,
        start: WorldCell,
        goal: WorldCell,
        occupied: &BTreeSet<WorldCell>,
    ) -> Result<Option<WorldCell>, WorldStateError> {
        self.require_reachable(start, goal)?;
        let mut queue = VecDeque::from([start]);
        let mut previous = BTreeMap::from([(start, None)]);
        while let Some(cell) = queue.pop_front() {
            if previous.len() > self.walkable.len() {
                return Err(error(
                    "world_navigation_failed",
                    "occupied navigation exceeded the Engine projection bound",
                ));
            }
            if cell.x.abs_diff(goal.x) + cell.y.abs_diff(goal.y) == 1 {
                let mut cursor = cell;
                let mut prior = previous[&cursor];
                while let Some(parent) = prior {
                    if parent == start {
                        return Ok(Some(cursor));
                    }
                    cursor = parent;
                    prior = previous[&cursor];
                }
                return Ok(None);
            }
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let next = WorldCell {
                    x: cell.x + dx,
                    y: cell.y + dy,
                };
                if previous.contains_key(&next)
                    || occupied.contains(&next)
                    || !self.navigation.is_walkable(self.voxel(next))
                {
                    continue;
                }
                previous.insert(next, Some(cell));
                queue.push_back(next);
            }
        }
        Ok(None)
    }

    pub(super) fn clear_distance(&self, origin: WorldCell, target: WorldCell) -> Option<u32> {
        self.line_is_clear(origin, target).then(|| {
            origin
                .x
                .abs_diff(target.x)
                .saturating_add(origin.y.abs_diff(target.y))
        })
    }

    pub(super) fn visible_terrain(&self, origin: WorldCell, facing: Facing) -> VisibleTerrain {
        let mut shadowcast = BTreeSet::from([origin]);
        for [xx, xy, yx, yy] in [
            [1, 0, 0, 1],
            [0, 1, 1, 0],
            [0, -1, 1, 0],
            [-1, 0, 0, 1],
            [-1, 0, 0, -1],
            [0, -1, -1, 0],
            [0, 1, -1, 0],
            [1, 0, 0, -1],
        ] {
            self.cast_light(
                origin,
                1,
                1.0,
                0.0,
                MAX_VIEW_DEPTH,
                [xx, xy, yx, yy],
                &mut shadowcast,
            );
        }
        let visible = shadowcast
            .into_iter()
            .filter(|cell| {
                let (lateral, depth) = relative(origin, facing, *cell);
                (0..=MAX_VIEW_DEPTH).contains(&depth) && lateral.abs() <= depth.max(1)
            })
            .collect::<BTreeSet<_>>();
        VisibleTerrain {
            floor: visible
                .iter()
                .copied()
                .filter(|cell| {
                    self.walkable.contains(cell)
                        && self.line_is_clear(origin, *cell)
                        && !self.opaque_walkable_precedes(origin, *cell)
                })
                .collect(),
            locked_doors_forward: Vec::new(),
            locked_doors_side: Vec::new(),
            walls: visible
                .into_iter()
                .filter(|cell| {
                    !self.walkable.contains(cell)
                        && self.wall_is_first_hit(origin, *cell)
                        && !self.opaque_walkable_precedes(origin, *cell)
                })
                .collect(),
        }
    }

    pub(super) fn scene_terrain(&self, origin: WorldCell, facing: Facing) -> VisibleTerrain {
        let mut floor = Vec::new();
        let mut locked_doors_forward = Vec::new();
        let mut locked_doors_side = Vec::new();
        let mut walls = Vec::new();
        let (forward_x, forward_y) = facing.forward();
        let (right_x, right_y) = facing.right_axis();
        for depth in 0..=MAX_VIEW_DEPTH {
            for lateral in -depth.max(1)..=depth.max(1) {
                let x = i64::from(origin.x) + i64::from(right_x * lateral + forward_x * depth);
                let y = i64::from(origin.y) + i64::from(right_y * lateral + forward_y * depth);
                let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
                    continue;
                };
                let cell = WorldCell { x, y };
                if self.opaque_walkable.contains(&cell) {
                    let party_north_south = matches!(facing, Facing::North | Facing::South);
                    if self.locked_door_north_south.get(&cell).copied() == Some(party_north_south) {
                        locked_doors_forward.push(cell);
                    } else {
                        locked_doors_side.push(cell);
                    }
                } else if self.walkable.contains(&cell) {
                    floor.push(cell);
                } else {
                    walls.push(cell);
                }
            }
        }
        VisibleTerrain {
            floor,
            locked_doors_forward,
            locked_doors_side,
            walls,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn cast_light(
        &self,
        origin: WorldCell,
        row: i32,
        mut start_slope: f64,
        end_slope: f64,
        radius: i32,
        [xx, xy, yx, yy]: [i32; 4],
        visible: &mut BTreeSet<WorldCell>,
    ) {
        if start_slope < end_slope {
            return;
        }
        for distance in row..=radius {
            let mut blocked = false;
            let mut next_start = start_slope;
            let mut dx = -distance;
            let dy = -distance;
            while dx <= 0 {
                let left_slope = (f64::from(dx) - 0.5) / (f64::from(dy) + 0.5);
                let right_slope = (f64::from(dx) + 0.5) / (f64::from(dy) - 0.5);
                if start_slope < right_slope {
                    dx += 1;
                    continue;
                }
                if end_slope > left_slope {
                    break;
                }
                let cell = WorldCell {
                    x: origin.x + dx * xx + dy * xy,
                    y: origin.y + dx * yx + dy * yy,
                };
                if dx * dx + dy * dy <= radius * radius && self.inside_bounds(cell) {
                    visible.insert(cell);
                }
                let opaque = self.is_opaque(cell);
                if blocked {
                    if opaque {
                        next_start = right_slope;
                    } else {
                        blocked = false;
                        start_slope = next_start;
                    }
                } else if opaque && distance < radius {
                    blocked = true;
                    self.cast_light(
                        origin,
                        distance + 1,
                        start_slope,
                        left_slope,
                        radius,
                        [xx, xy, yx, yy],
                        visible,
                    );
                    next_start = right_slope;
                }
                dx += 1;
            }
            if blocked {
                break;
            }
        }
    }

    fn inside_bounds(&self, cell: WorldCell) -> bool {
        (self.min_x..=self.max_x).contains(&cell.x) && (self.min_y..=self.max_y).contains(&cell.y)
    }

    fn is_opaque(&self, cell: WorldCell) -> bool {
        !self.inside_bounds(cell)
            || !self.walkable.contains(&cell)
            || self.opaque_walkable.contains(&cell)
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

    fn opaque_walkable_precedes(&self, origin: WorldCell, target: WorldCell) -> bool {
        let target_dx = target.x - origin.x;
        let target_dy = target.y - origin.y;
        let target_distance_squared = target_dx * target_dx + target_dy * target_dy;
        self.opaque_walkable.iter().any(|opaque| {
            let opaque_dx = opaque.x - origin.x;
            let opaque_dy = opaque.y - origin.y;
            opaque_dx * target_dy == opaque_dy * target_dx
                && opaque_dx * target_dx + opaque_dy * target_dy > 0
                && opaque_dx * opaque_dx + opaque_dy * opaque_dy < target_distance_squared
        })
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
