use avian3d::prelude::Collider;
use bevy::prelude::*;

use crate::net::NetRoomSnapshotEvent;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub x: i32,
    pub z: i32,
}

#[derive(Resource, Default)]
pub struct ActiveRoom {
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
}

/// Marks the back wall pieces so they are rebuilt with the room.
#[derive(Component)]
pub struct RoomWall;

/// Wall height in tiles (one height everywhere).
pub const WALL_HEIGHT: f32 = 2.0;
/// Wall thickness in tiles; walls are centered on the floor edge.
pub const WALL_THICKNESS: f32 = 0.16;
/// Opacity of a wall that stands between the camera and the local avatar.
const GHOST_WALL_ALPHA: f32 = 0.22;
/// Height of the point on the avatar that must stay visible.
const AVATAR_FOCUS_HEIGHT: f32 = 1.0;

/// Which floor edge a wall piece stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallSide {
    /// Along the x axis at the tile's `z` edge (faces the camera's right side).
    North,
    /// Along the z axis at the tile's `x` edge (faces the camera's left side).
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallPiece {
    pub x: i32,
    pub z: i32,
    pub side: WallSide,
}

fn is_floor(heightmap: &[String], x: i32, z: i32) -> bool {
    if x < 0 || z < 0 {
        return false;
    }
    heightmap
        .get(z as usize)
        .and_then(|row| row.chars().nth(x as usize))
        .is_some_and(|ch| ch != 'x' && ch != 'X')
}

/// Wall pieces for the edges facing away from the camera only (low x and low z), one per
/// floor tile whose neighbour towards the back is not floor. Edges facing the camera stay open.
pub fn wall_pieces(heightmap: &[String]) -> Vec<WallPiece> {
    let mut pieces = Vec::new();
    for (z, row) in heightmap.iter().enumerate() {
        for x in 0..row.chars().count() {
            let (x, z) = (x as i32, z as i32);
            if !is_floor(heightmap, x, z) {
                continue;
            }
            if !is_floor(heightmap, x, z - 1) {
                pieces.push(WallPiece {
                    x,
                    z,
                    side: WallSide::North,
                });
            }
            if !is_floor(heightmap, x - 1, z) {
                pieces.push(WallPiece {
                    x,
                    z,
                    side: WallSide::West,
                });
            }
        }
    }
    pieces
}

/// Center and size of a wall piece's box (centered on the floor edge).
pub fn wall_box(piece: &WallPiece) -> (Vec3, Vec3) {
    let (x, z) = (piece.x as f32, piece.z as f32);
    let y = WALL_HEIGHT / 2.0;
    match piece.side {
        WallSide::North => (
            Vec3::new(x + 0.5, y, z),
            Vec3::new(1.0, WALL_HEIGHT, WALL_THICKNESS),
        ),
        WallSide::West => (
            Vec3::new(x, y, z + 0.5),
            Vec3::new(WALL_THICKNESS, WALL_HEIGHT, 1.0),
        ),
    }
}

/// Grid vertices where at least two wall pieces meet (or a piece ends): a post fills them so
/// corners are closed and staircase walls have no gaps.
pub fn wall_posts(pieces: &[WallPiece]) -> Vec<(i32, i32)> {
    use std::collections::HashMap;
    let mut counts: HashMap<(i32, i32), u32> = HashMap::new();
    for p in pieces {
        let ends = match p.side {
            WallSide::North => [(p.x, p.z), (p.x + 1, p.z)],
            WallSide::West => [(p.x, p.z), (p.x, p.z + 1)],
        };
        for end in ends {
            *counts.entry(end).or_default() += 1;
        }
    }
    let mut posts: Vec<_> = counts.into_keys().collect();
    posts.sort();
    posts
}

/// Whether the segment `from`-`to` crosses the box (`center`, `half` extents).
pub fn segment_hits_box(from: Vec3, to: Vec3, center: Vec3, half: Vec3) -> bool {
    let dir = to - from;
    let (mut t_min, mut t_max) = (0.0_f32, 1.0_f32);
    for axis in 0..3 {
        let (o, d) = (from[axis] - center[axis], dir[axis]);
        if d.abs() < 1e-9 {
            if o.abs() > half[axis] {
                return false;
            }
            continue;
        }
        let (mut t0, mut t1) = ((-half[axis] - o) / d, (half[axis] - o) / d);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        t_min = t_min.max(t0);
        t_max = t_max.min(t1);
        if t_min > t_max {
            return false;
        }
    }
    true
}

/// Shared wall materials: solid, and see-through for walls hiding the local avatar.
#[derive(Resource)]
pub struct WallMaterials {
    solid: Handle<StandardMaterial>,
    ghost: Handle<StandardMaterial>,
}

/// Collision box of a wall, used to test whether it is in the camera's line of sight.
#[derive(Component)]
pub struct WallBox {
    center: Vec3,
    half: Vec3,
}

/// Everything a room snapshot spawns and a new snapshot replaces.
type RoomGeometry = Or<(With<Tile>, With<RoomWall>)>;

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveRoom>().add_systems(
            Update,
            (handle_room_snapshot, ghost_walls_in_the_way).chain(),
        );
    }
}

/// Thickness of the floor tile slab; it spans `y = 0..FLOOR_TILE_TOP`.
pub const FLOOR_TILE_THICKNESS: f32 = 0.2;
/// Height of the top surface of the floor tiles: the ground level for avatars and furniture.
pub const FLOOR_TILE_TOP: f32 = FLOOR_TILE_THICKNESS;

/// World position of a tile's walking surface (center of the tile, on top of the slab).
pub fn tile_to_world(x: i32, z: i32) -> Vec3 {
    Vec3::new(x as f32 + 0.5, FLOOR_TILE_TOP, z as f32 + 0.5)
}

pub fn world_to_tile(pos: Vec3) -> (i32, i32) {
    (pos.x.floor() as i32, pos.z.floor() as i32)
}

/// Walls between the camera and the local avatar turn see-through so the avatar stays visible.
fn ghost_walls_in_the_way(
    wall_materials: Option<Res<WallMaterials>>,
    cameras: Query<&GlobalTransform, With<crate::camera::MainRoomCamera>>,
    avatars: Query<(&crate::avatar::Avatar, &GlobalTransform)>,
    mut walls: Query<(&WallBox, &mut MeshMaterial3d<StandardMaterial>), With<RoomWall>>,
) {
    let Some(wall_materials) = wall_materials else {
        return;
    };
    let Ok(camera) = cameras.single() else {
        return;
    };
    let Some((_, avatar)) = avatars.iter().find(|(a, _)| a.is_local) else {
        return;
    };
    let from = camera.translation();
    let to = avatar.translation() + Vec3::Y * AVATAR_FOCUS_HEIGHT;
    for (wall, mut material) in walls.iter_mut() {
        let wanted = if segment_hits_box(from, to, wall.center, wall.half) {
            &wall_materials.ghost
        } else {
            &wall_materials.solid
        };
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}

fn handle_room_snapshot(
    mut commands: Commands,
    mut snapshot_events: MessageReader<NetRoomSnapshotEvent>,
    mut active_room: ResMut<ActiveRoom>,
    existing_tiles: Query<Entity, RoomGeometry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in snapshot_events.read() {
        let snap = &event.0;
        info!(
            "Building 3D room tiles for: {} ({}x{})",
            snap.room_name, snap.layout.width, snap.layout.height
        );

        // Clear previous room tiles if any
        for entity in existing_tiles.iter() {
            commands.entity(entity).despawn();
        }

        active_room.id = snap.room_id.clone();
        active_room.name = snap.room_name.clone();
        active_room.width = snap.layout.width;
        active_room.height = snap.layout.height;

        let wall_color = Color::srgb(0.78, 0.74, 0.68);
        let wall_materials = WallMaterials {
            solid: materials.add(StandardMaterial {
                base_color: wall_color,
                perceptual_roughness: 0.95,
                ..default()
            }),
            ghost: materials.add(StandardMaterial {
                base_color: wall_color.with_alpha(GHOST_WALL_ALPHA),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.95,
                ..default()
            }),
        };
        let pieces = wall_pieces(&snap.layout.heightmap);
        let mut boxes: Vec<(Vec3, Vec3)> = pieces.iter().map(wall_box).collect();
        for (x, z) in wall_posts(&pieces) {
            boxes.push((
                Vec3::new(x as f32, WALL_HEIGHT / 2.0, z as f32),
                Vec3::new(WALL_THICKNESS, WALL_HEIGHT, WALL_THICKNESS),
            ));
        }
        for (center, size) in boxes {
            commands.spawn((
                RoomWall,
                WallBox {
                    center,
                    half: size / 2.0,
                },
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(wall_materials.solid.clone()),
                Transform::from_translation(center),
            ));
        }
        commands.insert_resource(wall_materials);

        let tile_mesh = meshes.add(Cuboid::new(0.96, FLOOR_TILE_THICKNESS, 0.96));

        // Curated checkerboard tile palette (subtle modern aesthetic)
        let mat_tile_a = materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.30, 0.38),
            perceptual_roughness: 0.8,
            ..default()
        });
        let mat_tile_b = materials.add(StandardMaterial {
            base_color: Color::srgb(0.21, 0.25, 0.32),
            perceptual_roughness: 0.8,
            ..default()
        });

        for (z, row) in snap.layout.heightmap.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == 'x' || ch == 'X' {
                    continue; // Empty tile, do not render floor
                }

                let x_coord = x as i32;
                let z_coord = z as i32;
                // The slab is centered half its thickness below the walking surface.
                let world_pos =
                    tile_to_world(x_coord, z_coord) - Vec3::Y * (FLOOR_TILE_THICKNESS / 2.0);

                let is_even = (x_coord + z_coord) % 2 == 0;
                let material = if is_even {
                    mat_tile_a.clone()
                } else {
                    mat_tile_b.clone()
                };

                commands.spawn((
                    Tile {
                        x: x_coord,
                        z: z_coord,
                    },
                    Mesh3d(tile_mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(world_pos),
                    // Avian 3D collider for spatial raycasting and physics bounds
                    Collider::cuboid(1.0, FLOOR_TILE_THICKNESS, 1.0),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(rows: &[&str]) -> Vec<String> {
        rows.iter().map(|r| r.to_string()).collect()
    }

    #[test]
    fn rectangular_room_gets_walls_only_on_the_two_back_sides() {
        let pieces = wall_pieces(&map(&["000", "000"]));
        let north = pieces.iter().filter(|p| p.side == WallSide::North).count();
        let west = pieces.iter().filter(|p| p.side == WallSide::West).count();
        assert_eq!(north, 3);
        assert_eq!(west, 2);
        assert_eq!(pieces.len(), 5);
    }

    #[test]
    fn back_corner_is_closed_by_a_post() {
        let pieces = wall_pieces(&map(&["00", "00"]));
        assert!(wall_posts(&pieces).contains(&(0, 0)));
    }

    #[test]
    fn staircase_walls_are_continuous() {
        // Diagonal back edge: every joint between a north and a west piece has a post.
        let pieces = wall_pieces(&map(&["xx0", "x00", "000"]));
        let posts = wall_posts(&pieces);
        for p in [(2, 1), (1, 2), (2, 0), (1, 1), (0, 2)] {
            assert!(posts.contains(&p), "{p:?} in {posts:?}");
        }
    }

    #[test]
    fn empty_tiles_get_no_walls_and_expose_edges_next_to_them() {
        // The hole at (1,0) exposes a west wall on (2,0) and a north wall on (1,1).
        let pieces = wall_pieces(&map(&["0x0", "000"]));
        assert!(
            pieces
                .iter()
                .any(|p| p.side == WallSide::West && p.x == 2 && p.z == 0)
        );
        assert!(
            pieces
                .iter()
                .any(|p| p.side == WallSide::North && p.x == 1 && p.z == 1)
        );
        assert!(!pieces.iter().any(|p| p.x == 1 && p.z == 0));
    }

    #[test]
    fn wall_boxes_are_centered_on_the_floor_edge() {
        let (center, size) = wall_box(&WallPiece {
            x: 3,
            z: 0,
            side: WallSide::North,
        });
        assert_eq!(center.z, 0.0);
        assert_eq!(size.y, WALL_HEIGHT);
    }

    #[test]
    fn wall_between_camera_and_avatar_is_hit_and_others_are_not() {
        let camera = Vec3::new(10.0, 10.0, 10.0);
        let avatar = Vec3::new(2.0, 1.0, 2.0);
        let half = Vec3::new(0.5, 1.0, 0.08);
        // In front of the avatar, on the line of sight.
        assert!(segment_hits_box(
            camera,
            avatar,
            Vec3::new(5.0, 3.5, 5.0),
            Vec3::splat(2.0)
        ));
        // Behind the avatar (a back wall).
        assert!(!segment_hits_box(
            camera,
            avatar,
            Vec3::new(2.0, 1.0, 0.0),
            half
        ));
        // Far to the side.
        assert!(!segment_hits_box(
            camera,
            avatar,
            Vec3::new(9.0, 1.0, 0.0),
            half
        ));
    }

    #[test]
    fn test_tile_to_world_and_back() {
        let (x, z) = (4, 7);
        let world = tile_to_world(x, z);
        assert_eq!(world.x, 4.5);
        assert_eq!(world.z, 7.5);
        // Avatars stand on top of the floor slab, not inside it.
        assert_eq!(world.y, FLOOR_TILE_TOP);

        let (recovered_x, recovered_z) = world_to_tile(world);
        assert_eq!(recovered_x, x);
        assert_eq!(recovered_z, z);
    }
}
