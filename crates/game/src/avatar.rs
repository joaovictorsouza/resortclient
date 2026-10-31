use std::collections::HashSet;

use avian3d::prelude::*;
use bevy::prelude::*;
use protocol::{AvatarAppearance, AvatarAppearanceUpdated, Posture};

use crate::avatar_anim::{AvatarPose, AvatarPosture};
use crate::net::{
    NakamaResource, NetAvatarActionEvent, NetAvatarAppearanceUpdatedEvent,
    NetAvatarExpressionChangedEvent, NetAvatarHoldingChangedEvent, NetAvatarJoinedEvent,
    NetAvatarLeftEvent, NetAvatarPathEvent, NetAvatarPostureChangedEvent, NetRoomSnapshotEvent,
};
use crate::room::tile_to_world;
use crate::toon::look_from_appearance;

#[derive(Component, Debug)]
pub struct Avatar {
    pub user_id: String,
    pub display_name: String,
    pub is_local: bool,
    pub appearance: AvatarAppearance,
}

#[derive(Component, Debug, Default)]
pub struct PathFollower {
    pub remaining: Vec<Vec3>,
    pub speed: f32,
}

#[derive(Component)]
pub struct AvatarVisualRoot;

/// A sit/lay posture from the server waiting for the avatar to finish walking locally:
/// the server ends the path a little before the client's interpolation does.
#[derive(Component, Debug, Clone, Copy)]
pub struct PendingPosture {
    pub posture: AvatarPosture,
    pub facing: Quat,
    /// Where the avatar's origin must end up (e.g. the middle of a bed).
    pub position: Vec3,
    /// The posture was applied; the avatar is still sliding to `position`.
    pub applied: bool,
}

/// Distance from the avatar origin to the middle of the body in the `lay` pose, along the facing
/// direction. Printed by `crates/toon_avatar/tools/animations/build_cc_animations.py` ("body middle
/// along Z"): the body's bounding box is centred 0.33 world units towards the feet from the pelvis
/// (the head points to -Z in the model), so the origin must sit that far towards the head.
pub const LAY_CENTER_OFFSET: f32 = -0.33;
/// Speed (tiles/s) at which an avatar slides from its tile center to its resting spot.
const REST_SLIDE_SPEED: f32 = 1.5;

/// Where an avatar from a snapshot stands: its tile, or its resting spot on furniture.
pub fn snapshot_position(
    posture: Posture,
    x: i32,
    z: i32,
    rest_x: f64,
    rest_z: f64,
    rotation: i32,
) -> Vec3 {
    if posture == Posture::Stand {
        tile_to_world(x, z)
    } else {
        resting_position(posture, rest_x, rest_z, rotation)
    }
}

/// World position of the avatar origin for a server resting state.
pub fn resting_position(posture: Posture, rest_x: f64, rest_z: f64, rotation: i32) -> Vec3 {
    let mut pos = Vec3::new(rest_x as f32, tile_to_world(0, 0).y, rest_z as f32);
    if posture == Posture::Lay {
        // The origin sits towards the feet so the middle of the body lands on the rest point.
        let facing = facing_rotation(rotation) * Vec3::NEG_Z;
        pos += facing * LAY_CENTER_OFFSET;
    }
    pos
}

/// Root rotation for a server facing (0..7, 45° steps, 0 = +Z, 2 = +X), matching the
/// `look_at` convention used while walking (the root's -Z points where the avatar looks).
pub fn facing_rotation(rotation: i32) -> Quat {
    let theta = rotation.rem_euclid(8) as f32 * std::f32::consts::FRAC_PI_4;
    Quat::from_rotation_y(theta + std::f32::consts::PI)
}

/// Height of the character model in metres (`model_height_m` of `clips.json`, measured by
/// `build_cc_animations.py`).
pub const AVATAR_MODEL_HEIGHT: f32 = 1.596;
/// Target avatar height in world units (matches `Collider::capsule(0.3, 1.2)`: 1.2 + 2 * 0.3).
pub const AVATAR_TARGET_HEIGHT: f32 = 1.8;
/// Uniform scale applied to the glTF avatar scenes so they match the room's tile scale.
pub const AVATAR_MODEL_SCALE: f32 = AVATAR_TARGET_HEIGHT / AVATAR_MODEL_HEIGHT;

/// Transform of the node holding the avatar's glTF scenes: scaled to world units and
/// turned around, because the pack models face +Z while `look_at` points -Z forward.
pub fn avatar_visual_transform() -> Transform {
    Transform::from_scale(Vec3::splat(AVATAR_MODEL_SCALE))
        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
}

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // Chained so the snapshot's spawn commands are applied before joins run.
                (
                    handle_spawn_avatars_snapshot.run_if(resource_exists::<NakamaResource>),
                    handle_avatar_joined.run_if(resource_exists::<NakamaResource>),
                )
                    .chain(),
                handle_avatar_appearance_updated,
                handle_avatar_emotes,
                handle_avatar_left,
                handle_avatar_path,
                handle_avatar_posture_changed,
                move_avatars_along_path,
                apply_pending_postures.after(move_avatars_along_path),
            ),
        );
    }
}

struct AvatarSpawnData<'a> {
    user_id: &'a str,
    display_name: &'a str,
    x: i32,
    z: i32,
    is_local: bool,
    appearance: AvatarAppearance,
    expression: &'a str,
    holding: &'a str,
    posture: Posture,
    rotation: i32,
    y: f64,
    rest_x: f64,
    rest_z: f64,
}

fn spawn_avatar_entity(commands: &mut Commands, data: AvatarSpawnData<'_>) {
    let world_pos = snapshot_position(
        data.posture,
        data.x,
        data.z,
        data.rest_x,
        data.rest_z,
        data.rotation,
    );

    commands
        .spawn((
            Avatar {
                user_id: data.user_id.to_string(),
                display_name: data.display_name.to_string(),
                is_local: data.is_local,
                appearance: data.appearance.clone(),
            },
            PathFollower::default(),
            AvatarPose::default(),
            AvatarPosture::from_server(data.posture, data.y),
            RigidBody::Kinematic,
            Collider::capsule(0.3, 1.2),
            Transform::from_translation(world_pos).with_rotation(facing_rotation(data.rotation)),
            Visibility::default(),
        ))
        .with_children(|root| {
            // The character (body, garments, face, held prop): built and animated by the
            // `toon_avatar` crate, which keeps it hidden until it is complete. Scaled from the
            // model's metres to world units.
            let mut visual = root.spawn((
                AvatarVisualRoot,
                avatar_visual_transform(),
                toon_avatar::Avatar::new(look_from_appearance(&data.appearance)),
                toon_avatar::AvatarWalkSpeed(0.0),
                toon_avatar::AvatarFace {
                    expression: toon_avatar::Expression::from_name(data.expression)
                        .unwrap_or_default(),
                    blink: true,
                },
                toon_avatar::AvatarHolding(crate::toon::held_prop(data.holding)),
            ));
            if data.is_local {
                // The local player's avatar always gets the best level of detail the device allows.
                visual.insert(toon_avatar::LodFocus);
            }

            // Nametag above avatar
            root.spawn((
                Text2d::new(data.display_name),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(if data.is_local {
                    Color::srgb(0.2, 0.8, 1.0)
                } else {
                    Color::srgb(1.0, 0.9, 0.6)
                }),
                Transform::from_xyz(0.0, 1.8, 0.0),
            ));
        });
}

fn handle_spawn_avatars_snapshot(
    mut commands: Commands,
    mut snapshot_events: MessageReader<NetRoomSnapshotEvent>,
    mut existing_avatars: Query<(
        Entity,
        &Avatar,
        &mut Transform,
        &mut PathFollower,
        &mut AvatarPosture,
    )>,
    net: Option<Res<NakamaResource>>,
) {
    let Some(net) = net else { return };

    // Spawns are deferred commands, so a second snapshot in the same frame (e.g. the
    // public room followed quickly by the player's own room) would not see the avatar the
    // first one just spawned and would spawn a duplicate at the other room's door. A
    // snapshot is the full truth, so only the latest one of the frame matters.
    if let Some(event) = snapshot_events.read().last() {
        let snapshot_user_ids: HashSet<&str> = event
            .0
            .avatars
            .iter()
            .map(|av| av.user_id.as_str())
            .collect();

        // Remove avatars that left (or belonged to a room we just navigated away from).
        for (entity, avatar, ..) in existing_avatars.iter() {
            if !snapshot_user_ids.contains(avatar.user_id.as_str()) {
                commands.entity(entity).try_despawn();
            }
        }

        // Spawn only avatars we don't already have an entity for: a duplicate snapshot,
        // or one racing an `AvatarJoined` event for the same user, must not respawn (and
        // flash) an avatar that is already there — doing so also used to race the
        // in-flight asset loads of the entity it just replaced.
        for av in &event.0.avatars {
            if let Some((entity, _, mut transform, mut follower, mut posture)) = existing_avatars
                .iter_mut()
                .find(|(_, avatar, ..)| avatar.user_id == av.user_id)
            {
                // Same user in a (possibly different) room: the snapshot is the truth, so
                // the avatar must not stay where it was in the previous room.
                follower.remaining.clear();
                *posture = AvatarPosture::from_server(av.posture, av.y);
                transform.translation =
                    snapshot_position(av.posture, av.x, av.z, av.rest_x, av.rest_z, av.rotation);
                transform.rotation = facing_rotation(av.rotation);
                commands.entity(entity).try_remove::<PendingPosture>();
                continue;
            }

            let is_local = av.user_id == net.local_user_id;
            let appearance = av.appearance.clone().unwrap_or_default();
            spawn_avatar_entity(
                &mut commands,
                AvatarSpawnData {
                    user_id: &av.user_id,
                    display_name: &av.display_name,
                    x: av.x,
                    z: av.z,
                    is_local,
                    appearance,
                    expression: &av.expression,
                    holding: &av.holding,
                    posture: av.posture,
                    rotation: av.rotation,
                    y: av.y,
                    rest_x: av.rest_x,
                    rest_z: av.rest_z,
                },
            );
        }
    }
}

fn handle_avatar_joined(
    mut commands: Commands,
    mut joined_events: MessageReader<NetAvatarJoinedEvent>,
    existing: Query<&Avatar>,
    net: Option<Res<NakamaResource>>,
) {
    let Some(net) = net else { return };

    // Same deferred-spawn caveat as the snapshot handler: remember who we spawned this run.
    let mut spawned_now: HashSet<String> = HashSet::new();
    for event in joined_events.read() {
        let joined = &event.0;
        let exists = existing.iter().any(|a| a.user_id == joined.user_id);
        if exists || !spawned_now.insert(joined.user_id.clone()) {
            continue;
        }

        let is_local = joined.user_id == net.local_user_id;
        let appearance = joined.appearance.clone().unwrap_or_default();
        spawn_avatar_entity(
            &mut commands,
            AvatarSpawnData {
                user_id: &joined.user_id,
                display_name: &joined.display_name,
                x: joined.x,
                z: joined.z,
                is_local,
                appearance,
                expression: &joined.expression,
                holding: &joined.holding,
                posture: joined.posture,
                rotation: joined.rotation,
                y: joined.y,
                rest_x: joined.rest_x,
                rest_z: joined.rest_z,
            },
        );
    }
}

/// The `toon_avatar` character of a game avatar (the child with the visual root).
fn visual_of(
    children: Option<&Children>,
    visuals: &Query<(), With<AvatarVisualRoot>>,
) -> Option<Entity> {
    children?.iter().find(|c| visuals.contains(*c))
}

/// Changes what an avatar wears. The crate keeps the avatar and swaps only the parts that changed.
fn handle_avatar_appearance_updated(
    mut update_events: MessageReader<NetAvatarAppearanceUpdatedEvent>,
    mut avatar_query: Query<(&mut Avatar, Option<&Children>)>,
    visuals: Query<(), With<AvatarVisualRoot>>,
    mut looks: Query<&mut toon_avatar::Avatar>,
) {
    for event in update_events.read() {
        let upd: &AvatarAppearanceUpdated = &event.0;
        for (mut avatar, children) in avatar_query.iter_mut() {
            if avatar.user_id != upd.user_id {
                continue;
            }
            avatar.appearance = upd.appearance.clone();
            if let Some(visual) = visual_of(children, &visuals)
                && let Ok(mut character) = looks.get_mut(visual)
            {
                character.look = look_from_appearance(&avatar.appearance);
            }
        }
    }
}

/// Another avatar's expression, the prop in its hand, and the gesture it performs with it.
#[allow(clippy::too_many_arguments)]
fn handle_avatar_emotes(
    mut expressions: MessageReader<NetAvatarExpressionChangedEvent>,
    mut holdings: MessageReader<NetAvatarHoldingChangedEvent>,
    mut actions: MessageReader<NetAvatarActionEvent>,
    avatars: Query<(&Avatar, Option<&Children>)>,
    visuals: Query<(), With<AvatarVisualRoot>>,
    mut faces: Query<&mut toon_avatar::AvatarFace>,
    mut hands: Query<&mut toon_avatar::AvatarHolding>,
    mut poses: Query<&mut toon_avatar::AvatarPose>,
) {
    let visual_for = |user_id: &str| {
        avatars
            .iter()
            .find(|(a, _)| a.user_id == user_id)
            .and_then(|(_, children)| visual_of(children, &visuals))
    };
    for event in expressions.read() {
        if let Some(visual) = visual_for(&event.0.user_id)
            && let Ok(mut face) = faces.get_mut(visual)
        {
            face.expression =
                toon_avatar::Expression::from_name(&event.0.expression).unwrap_or_default();
        }
    }
    for event in holdings.read() {
        if let Some(visual) = visual_for(&event.0.user_id)
            && let Ok(mut hand) = hands.get_mut(visual)
        {
            hand.0 = crate::toon::held_prop(&event.0.prop_id);
        }
    }
    for event in actions.read() {
        if event.0.action != protocol::ACTION_DRINK {
            continue;
        }
        if let Some(visual) = visual_for(&event.0.user_id)
            && let Ok(mut pose) = poses.get_mut(visual)
            && let Some(drinking) = pose.drinking()
        {
            *pose = drinking;
        }
    }
}

fn handle_avatar_left(
    mut commands: Commands,
    mut left_events: MessageReader<NetAvatarLeftEvent>,
    query: Query<(Entity, &Avatar)>,
) {
    for event in left_events.read() {
        for (entity, av) in query.iter() {
            if av.user_id == event.0.user_id {
                commands.entity(entity).try_despawn();
            }
        }
    }
}

fn handle_avatar_path(
    mut path_events: MessageReader<NetAvatarPathEvent>,
    mut query: Query<(&Avatar, &mut PathFollower)>,
) {
    for event in path_events.read() {
        let path_data = &event.0;
        for (av, mut follower) in query.iter_mut() {
            if av.user_id == path_data.user_id {
                follower.speed = path_data.tiles_per_sec as f32;
                follower.remaining = path_data
                    .path
                    .iter()
                    .map(|pt| tile_to_world(pt[0], pt[1]))
                    .collect();
            }
        }
    }
}

fn handle_avatar_posture_changed(
    mut commands: Commands,
    mut posture_events: MessageReader<NetAvatarPostureChangedEvent>,
    mut query: Query<(
        Entity,
        &Avatar,
        &mut AvatarPosture,
        &PathFollower,
        &mut Transform,
    )>,
) {
    for event in posture_events.read() {
        let change = &event.0;
        for (entity, av, mut posture, follower, mut transform) in query.iter_mut() {
            if av.user_id != change.user_id {
                continue;
            }
            if change.posture == Posture::Stand {
                // Standing up happens right away; while walking the path drives the facing.
                *posture = AvatarPosture::default();
                commands.entity(entity).try_remove::<PendingPosture>();
                if follower.remaining.is_empty() {
                    // Standing up in place (e.g. the furniture was removed): back on the tile.
                    transform.translation = resting_position(
                        Posture::Stand,
                        change.rest_x,
                        change.rest_z,
                        change.rotation,
                    );
                }
            } else {
                commands.entity(entity).try_insert(PendingPosture {
                    posture: AvatarPosture::from_server(change.posture, change.y),
                    facing: facing_rotation(change.rotation),
                    position: resting_position(
                        change.posture,
                        change.rest_x,
                        change.rest_z,
                        change.rotation,
                    ),
                    applied: false,
                });
            }
        }
    }
}

/// Applies a pending sit/lay once the avatar has reached its final tile locally.
fn apply_pending_postures(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &PathFollower,
        &mut PendingPosture,
        &mut AvatarPosture,
        &mut Transform,
    )>,
) {
    for (entity, follower, mut pending, mut posture, mut transform) in query.iter_mut() {
        if !follower.remaining.is_empty() {
            continue;
        }
        if !pending.applied {
            *posture = pending.posture;
            transform.rotation = pending.facing;
            pending.applied = true;
        }
        let diff = pending.position - transform.translation;
        let step = REST_SLIDE_SPEED * time.delta_secs();
        if diff.length() <= step {
            transform.translation = pending.position;
            commands.entity(entity).try_remove::<PendingPosture>();
        } else {
            transform.translation += diff.normalize() * step;
        }
    }
}

fn move_avatars_along_path(time: Res<Time>, mut query: Query<(&mut Transform, &mut PathFollower)>) {
    let delta = time.delta_secs();

    for (mut transform, mut follower) in query.iter_mut() {
        if follower.remaining.is_empty() {
            continue;
        }

        let target = follower.remaining[0];
        let diff = target - transform.translation;
        let distance = diff.length();
        let step = follower.speed * delta;

        if distance <= step {
            transform.translation = target;
            follower.remaining.remove(0);
        } else {
            transform.translation += diff.normalize() * step;
            // Face the movement direction smoothly
            let flat_diff = Vec3::new(diff.x, 0.0, diff.z);
            if flat_diff.length_squared() > 0.001 {
                let look_target = transform.translation + flat_diff;
                transform.look_at(look_target, Vec3::Y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_scale_maps_rig_height_to_target_height() {
        let scaled = AVATAR_MODEL_HEIGHT * AVATAR_MODEL_SCALE;
        assert!((scaled - AVATAR_TARGET_HEIGHT).abs() < 1e-4);
        // The pack is authored in metres, so the scale stays close to 1.
        const { assert!(AVATAR_MODEL_SCALE < 1.2 && AVATAR_MODEL_SCALE > 0.8) };
    }

    #[test]
    fn facing_rotation_matches_server_directions() {
        let cases = [
            (0, Vec3::Z),
            (2, Vec3::X),
            (4, Vec3::NEG_Z),
            (6, Vec3::NEG_X),
            (1, Vec3::new(1.0, 0.0, 1.0).normalize()),
            (8, Vec3::Z),
            (-2, Vec3::NEG_X),
        ];
        for (rotation, expected) in cases {
            // Same convention as `look_at`: the root's forward (-Z) is where it looks.
            let forward = facing_rotation(rotation) * Vec3::NEG_Z;
            assert!(
                forward.abs_diff_eq(expected, 1e-5),
                "rotation {rotation}: {forward:?} != {expected:?}"
            );
        }
    }

    fn posture_app() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_message::<NetAvatarPostureChangedEvent>()
            .add_systems(
                Update,
                (handle_avatar_posture_changed, apply_pending_postures).chain(),
            );
        let avatar = app
            .world_mut()
            .spawn((
                Avatar {
                    user_id: "u1".to_string(),
                    display_name: "Alice".to_string(),
                    is_local: false,
                    appearance: AvatarAppearance::default(),
                },
                PathFollower {
                    remaining: vec![Vec3::ONE],
                    speed: 2.0,
                },
                AvatarPosture::default(),
                Transform::default(),
            ))
            .id();
        (app, avatar)
    }

    fn send_posture(app: &mut App, posture: Posture, rotation: i32, y: f64) {
        app.world_mut().write_message(NetAvatarPostureChangedEvent(
            protocol::AvatarPostureChanged {
                user_id: "u1".to_string(),
                posture,
                rotation,
                y,
                rest_x: 3.5,
                rest_z: 3.5,
            },
        ));
        app.update();
    }

    #[test]
    fn sit_waits_for_local_walk_to_finish() {
        let (mut app, avatar) = posture_app();

        send_posture(&mut app, Posture::Sit, 4, 0.8);
        let world = app.world();
        assert_eq!(
            world.get::<AvatarPosture>(avatar).unwrap().kind,
            crate::avatar_anim::PostureKind::Stand
        );
        assert!(world.get::<PendingPosture>(avatar).is_some());

        app.world_mut()
            .get_mut::<PathFollower>(avatar)
            .unwrap()
            .remaining
            .clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(10));
        app.update();
        let world = app.world();
        assert_eq!(world.get::<Transform>(avatar).unwrap().translation.x, 3.5);
        let posture = world.get::<AvatarPosture>(avatar).unwrap();
        assert_eq!(posture.kind, crate::avatar_anim::PostureKind::Sit);
        assert_eq!(posture.support_height, Some(0.8));
        assert!(
            world
                .get::<Transform>(avatar)
                .unwrap()
                .rotation
                .abs_diff_eq(facing_rotation(4), 1e-5)
        );
        assert!(world.get::<PendingPosture>(avatar).is_none());

        send_posture(&mut app, Posture::Stand, 4, 0.0);
        assert_eq!(
            *app.world().get::<AvatarPosture>(avatar).unwrap(),
            AvatarPosture::default()
        );
    }

    #[test]
    fn stand_cancels_pending_sit() {
        let (mut app, avatar) = posture_app();
        send_posture(&mut app, Posture::Lay, 0, 0.8);
        send_posture(&mut app, Posture::Stand, 0, 0.0);
        app.world_mut()
            .get_mut::<PathFollower>(avatar)
            .unwrap()
            .remaining
            .clear();
        app.update();
        assert_eq!(
            *app.world().get::<AvatarPosture>(avatar).unwrap(),
            AvatarPosture::default()
        );
    }

    #[test]
    fn snapshot_position_uses_tile_or_rest_point() {
        assert_eq!(
            snapshot_position(Posture::Stand, 2, 3, 9.0, 9.0, 0),
            tile_to_world(2, 3)
        );
        let sit = snapshot_position(Posture::Sit, 2, 3, 2.5, 3.5, 4);
        assert!((sit.x - 2.5).abs() < 1e-5 && (sit.z - 3.5).abs() < 1e-5);
    }

    #[test]
    fn laying_is_centered_on_the_rest_point() {
        // Facing +Z (rotation 0): the origin is shifted towards the feet (+Z) so the body
        // middle (towards the head, -Z) lands on the bed center.
        let p = resting_position(Posture::Lay, 4.5, 3.0, 0);
        assert!((p.x - 4.5).abs() < 1e-5 && (p.z - (3.0 + LAY_CENTER_OFFSET)).abs() < 1e-5);
        // Facing -X (rotation 6): shifted towards -X.
        let p = resting_position(Posture::Lay, 2.0, 4.5, 6);
        assert!((p.x - (2.0 - LAY_CENTER_OFFSET)).abs() < 1e-5 && (p.z - 4.5).abs() < 1e-5);
        // Sitting and standing stay on the rest point.
        let p = resting_position(Posture::Sit, 3.5, 3.5, 2);
        assert!((p.x - 3.5).abs() < 1e-5 && (p.z - 3.5).abs() < 1e-5);
        assert!((p.y - tile_to_world(0, 0).y).abs() < 1e-5);
    }

    #[test]
    fn facing_rotation_agrees_with_look_at() {
        let mut walked = Transform::default();
        walked.look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
        assert!(walked.rotation.abs_diff_eq(facing_rotation(2), 1e-5));
    }

    fn snapshot_with(user_id: &str, x: i32) -> NetRoomSnapshotEvent {
        use protocol::{AvatarSnapshot, LayoutSnapshot, RoomSnapshot};
        NetRoomSnapshotEvent(RoomSnapshot {
            room_id: "r".into(),
            room_name: "r".into(),
            owner_id: String::new(),
            layout: LayoutSnapshot {
                id: "l".into(),
                width: 8,
                height: 8,
                door: [0, 0],
                heightmap: vec![],
            },
            avatars: vec![AvatarSnapshot {
                user_id: user_id.into(),
                display_name: "me".into(),
                x,
                z: 0,
                appearance: None,
                expression: String::new(),
                holding: String::new(),
                posture: Posture::Stand,
                rotation: 0,
                y: 0.0,
                rest_x: 0.0,
                rest_z: 0.0,
            }],
            items: vec![],
        })
    }

    #[test]
    fn two_snapshots_in_one_frame_spawn_a_single_avatar() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<WorldAsset>()
            .insert_resource(NakamaResource {
                handle: nakama_client::NakamaClientHandle::spawn(),
                local_user_id: "me".into(),
            })
            .add_message::<NetRoomSnapshotEvent>()
            .add_message::<NetAvatarJoinedEvent>()
            .add_systems(
                Update,
                (handle_spawn_avatars_snapshot, handle_avatar_joined).chain(),
            );
        app.world_mut().write_message(snapshot_with("me", 1));
        app.world_mut().write_message(snapshot_with("me", 6));
        app.update();

        let world = app.world_mut();
        let avatars: Vec<_> = world.query::<(&Avatar, &Transform)>().iter(world).collect();
        assert_eq!(avatars.len(), 1);
        // The latest snapshot wins.
        assert_eq!(avatars[0].1.translation, tile_to_world(6, 0));
    }
}
