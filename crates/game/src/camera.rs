use crate::avatar::Avatar;
use crate::room::ActiveRoom;
use crate::state::AppState;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

#[derive(Component)]
pub struct MainRoomCamera;

/// Direction from the followed point towards the camera: fixed 45° above the floor and 45°
/// around it (no rotation), looking from the room's front corner so the two back walls
/// (x = 0 and z = 0) stay behind the avatar.
const CAMERA_OFFSET: Vec3 = Vec3::new(1.0, std::f32::consts::SQRT_2, 1.0);
/// Fraction of the screen the room may fill when zoomed all the way out.
const ROOM_SCREEN_FILL: f32 = 0.72;
/// Room walls/avatars extend this high above the floor (taken into account when framing).
const FRAME_HEIGHT: f32 = 2.4;
const MIN_FIT_DISTANCE: f32 = 5.0;

/// Closest the camera may get to the followed avatar.
const MIN_ZOOM_DISTANCE: f32 = 4.0;
/// Distance the camera starts at in every room.
const DEFAULT_ZOOM_DISTANCE: f32 = 9.0;
/// Zooming out is at least this far, even in tiny rooms.
const MIN_MAX_ZOOM_DISTANCE: f32 = 12.0;
/// Fraction of the remaining camera lag closed per second (higher is snappier).
const FOLLOW_SHARPNESS: f32 = 6.0;
/// Distance factor per mouse wheel notch / per pixel of trackpad scroll.
const WHEEL_LINE_ZOOM: f32 = 0.1;
const WHEEL_PIXEL_ZOOM: f32 = 0.002;

/// Camera distance that shows a whole `width` x `height` tiles room within
/// [`ROOM_SCREEN_FILL`] of the viewport, looking along the fixed [`CAMERA_OFFSET`] direction.
pub fn fit_distance(width: f32, height: f32, aspect: f32, vertical_fov: f32) -> f32 {
    let to_camera = CAMERA_OFFSET.normalize();
    let forward = -to_camera;
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward).normalize();

    let tan_v = (vertical_fov / 2.0).tan() * ROOM_SCREEN_FILL;
    let tan_h = tan_v * aspect;

    let mut distance = MIN_FIT_DISTANCE;
    for dx in [-0.5, 0.5] {
        for dz in [-0.5, 0.5] {
            for y in [0.0, FRAME_HEIGHT] {
                let corner = Vec3::new(dx * width, y, dz * height);
                // Depth along the view direction: nearer corners need less distance.
                let depth = corner.dot(forward);
                let needed_h = corner.dot(right).abs() / tan_h - depth;
                let needed_v = corner.dot(up).abs() / tan_v - depth;
                distance = distance.max(needed_h).max(needed_v);
            }
        }
    }
    distance
}

/// Camera transform that looks at `target` from `distance` along [`CAMERA_OFFSET`].
pub fn camera_transform(target: Vec3, distance: f32) -> Transform {
    Transform::from_translation(target + CAMERA_OFFSET.normalize() * distance)
        .looking_at(target, Vec3::Y)
}

/// Camera transform that centers a `width` x `height` tiles room and makes it fill about
/// [`ROOM_SCREEN_FILL`] of the viewport.
pub fn frame_room(width: f32, height: f32, aspect: f32, vertical_fov: f32) -> Transform {
    let center = Vec3::new(width / 2.0, 0.0, height / 2.0);
    camera_transform(center, fit_distance(width, height, aspect, vertical_fov))
}

/// `(min, max)` camera distance for a room: close to the avatar up to seeing the whole room.
pub fn zoom_limits(width: f32, height: f32, aspect: f32, vertical_fov: f32) -> (f32, f32) {
    let max = fit_distance(width, height, aspect, vertical_fov).max(MIN_MAX_ZOOM_DISTANCE);
    (MIN_ZOOM_DISTANCE, max)
}

/// Distance after a pinch whose finger span went from `previous_span` to `span` (fingers
/// moving apart zoom in, i.e. shrink the distance).
pub fn pinch_zoom(distance: f32, previous_span: f32, span: f32, limits: (f32, f32)) -> f32 {
    if previous_span <= 1.0 || span <= 1.0 {
        return distance.clamp(limits.0, limits.1);
    }
    (distance * previous_span / span).clamp(limits.0, limits.1)
}

/// Distance after a scroll of `y` (positive scrolls away from the user: zoom in).
pub fn wheel_zoom(distance: f32, unit: MouseScrollUnit, y: f32, limits: (f32, f32)) -> f32 {
    let per_unit = match unit {
        MouseScrollUnit::Line => WHEEL_LINE_ZOOM,
        MouseScrollUnit::Pixel => WHEEL_PIXEL_ZOOM,
    };
    (distance * (-y * per_unit).exp()).clamp(limits.0, limits.1)
}

/// Keeps the followed point inside the room so the camera never looks at the void.
pub fn clamp_to_room(target: Vec3, width: f32, height: f32) -> Vec3 {
    Vec3::new(target.x.clamp(0.0, width), 0.0, target.z.clamp(0.0, height))
}

/// One frame of smooth follow: moves `current` towards `goal`, frame-rate independent.
pub fn follow_step(current: Vec3, goal: Vec3, dt: f32) -> Vec3 {
    current.lerp(goal, 1.0 - (-FOLLOW_SHARPNESS * dt).exp())
}

/// What the camera is looking at and how far away it is.
#[derive(Resource, Debug)]
pub struct CameraRig {
    pub target: Vec3,
    pub distance: f32,
    /// Room the rig was last snapped to; a new room starts at its default view.
    room_id: String,
    /// Finger span of the previous frame while two fingers are down.
    pinch_span: Option<f32>,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: DEFAULT_ZOOM_DISTANCE,
            room_id: String::new(),
            pinch_span: None,
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraRig>()
            .add_systems(Startup, setup_camera_and_lighting)
            .add_systems(
                Update,
                (
                    update_main_camera_active,
                    (zoom_camera, follow_local_avatar).chain(),
                ),
            );
    }
}

fn setup_camera_and_lighting(mut commands: Commands) {
    commands.spawn((
        MainRoomCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            ..default()
        },
        AmbientLight {
            color: Color::srgb(0.9, 0.92, 0.98),
            brightness: 400.0,
            ..default()
        },
        frame_room(12.0, 10.0, 16.0 / 9.0, std::f32::consts::FRAC_PI_4),
    ));

    // Directional light representing warm ambient room lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn camera_aspect_and_fov(camera: &Camera, projection: &Projection) -> Option<(f32, f32)> {
    let size = camera.logical_viewport_size()?;
    let Projection::Perspective(persp) = projection else {
        return None;
    };
    (size.y > 0.0).then(|| (size.x / size.y, persp.fov))
}

/// Pinch (two fingers) and mouse wheel change the camera distance. The wheel is ignored while
/// a finger is down (`touch_scroll` turns one-finger drags into wheel messages for the
/// modals) and while the pointer is over the UI (modals scroll with it).
#[allow(clippy::too_many_arguments)]
fn zoom_camera(
    mut rig: ResMut<CameraRig>,
    room: Res<ActiveRoom>,
    touches: Res<Touches>,
    mut wheel: MessageReader<MouseWheel>,
    ui: Query<&Interaction, With<Node>>,
    cameras: Query<(&Camera, &Projection), With<MainRoomCamera>>,
) {
    let wheel_events: Vec<_> = wheel.read().map(|e| (e.unit, e.y)).collect();
    if room.width <= 0 || room.height <= 0 {
        return;
    }
    let Ok((camera, projection)) = cameras.single() else {
        return;
    };
    let Some((aspect, fov)) = camera_aspect_and_fov(camera, projection) else {
        return;
    };
    let limits = zoom_limits(room.width as f32, room.height as f32, aspect, fov);
    let over_ui = ui
        .iter()
        .any(|i| matches!(i, Interaction::Hovered | Interaction::Pressed));

    let fingers: Vec<Vec2> = touches.iter().map(|t| t.position()).collect();
    if let [a, b] = fingers[..] {
        let span = a.distance(b);
        if let Some(previous) = rig.pinch_span
            && !over_ui
        {
            rig.distance = pinch_zoom(rig.distance, previous, span, limits);
        }
        rig.pinch_span = Some(span);
    } else {
        rig.pinch_span = None;
    }

    if fingers.is_empty() && !over_ui {
        for (unit, y) in wheel_events {
            rig.distance = wheel_zoom(rig.distance, unit, y, limits);
        }
    }
    rig.distance = rig.distance.clamp(limits.0, limits.1);
}

/// Moves the camera smoothly after the local avatar (or the room center before it exists).
fn follow_local_avatar(
    time: Res<Time>,
    room: Res<ActiveRoom>,
    mut rig: ResMut<CameraRig>,
    avatars: Query<(&Avatar, &GlobalTransform)>,
    mut cameras: Query<&mut Transform, With<MainRoomCamera>>,
) {
    if room.width <= 0 || room.height <= 0 {
        return;
    }
    let (w, h) = (room.width as f32, room.height as f32);
    let goal = avatars
        .iter()
        .find(|(avatar, _)| avatar.is_local)
        .map(|(_, transform)| clamp_to_room(transform.translation(), w, h))
        .unwrap_or(Vec3::new(w / 2.0, 0.0, h / 2.0));

    if rig.room_id != room.id {
        rig.room_id = room.id.clone();
        rig.target = goal;
        rig.distance = DEFAULT_ZOOM_DISTANCE;
    } else {
        rig.target = follow_step(rig.target, goal, time.delta_secs());
    }
    for mut transform in cameras.iter_mut() {
        *transform = camera_transform(rig.target, rig.distance);
    }
}

fn update_main_camera_active(
    app_state: Res<State<AppState>>,
    mut query: Query<&mut Camera, With<MainRoomCamera>>,
) {
    let should_be_active = *app_state.get() == AppState::InRoom;
    for mut cam in query.iter_mut() {
        if cam.is_active != should_be_active {
            cam.is_active = should_be_active;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOV: f32 = std::f32::consts::FRAC_PI_4;

    const LIMITS: (f32, f32) = (4.0, 20.0);

    #[test]
    fn camera_looks_down_at_forty_five_degrees_from_the_corner() {
        let elevation = CAMERA_OFFSET
            .y
            .atan2(Vec2::new(CAMERA_OFFSET.x, CAMERA_OFFSET.z).length());
        assert!(
            (elevation.to_degrees() - 45.0).abs() < 0.1,
            "{}",
            elevation.to_degrees()
        );
        assert!((CAMERA_OFFSET.x - CAMERA_OFFSET.z).abs() < 1e-6);
    }

    #[test]
    fn pinching_apart_zooms_in_and_together_zooms_out() {
        assert!(pinch_zoom(10.0, 100.0, 200.0, LIMITS) < 10.0);
        assert!(pinch_zoom(10.0, 200.0, 100.0, LIMITS) > 10.0);
        assert_eq!(pinch_zoom(10.0, 100.0, 100.0, LIMITS), 10.0);
    }

    #[test]
    fn zoom_is_clamped_to_the_limits() {
        assert_eq!(pinch_zoom(10.0, 10.0, 1000.0, LIMITS), 4.0);
        assert_eq!(pinch_zoom(10.0, 1000.0, 10.0, LIMITS), 20.0);
        assert_eq!(wheel_zoom(5.0, MouseScrollUnit::Line, 50.0, LIMITS), 4.0);
        assert_eq!(wheel_zoom(15.0, MouseScrollUnit::Line, -50.0, LIMITS), 20.0);
    }

    #[test]
    fn degenerate_pinch_spans_do_not_move_the_camera() {
        assert_eq!(pinch_zoom(10.0, 0.0, 50.0, LIMITS), 10.0);
        assert_eq!(pinch_zoom(10.0, 50.0, 0.0, LIMITS), 10.0);
    }

    #[test]
    fn wheel_up_zooms_in_and_down_zooms_out() {
        assert!(wheel_zoom(10.0, MouseScrollUnit::Line, 1.0, LIMITS) < 10.0);
        assert!(wheel_zoom(10.0, MouseScrollUnit::Pixel, -20.0, LIMITS) > 10.0);
    }

    #[test]
    fn zoom_limits_allow_seeing_the_whole_room() {
        let (min, max) = zoom_limits(12.0, 10.0, 16.0 / 9.0, FOV);
        assert_eq!(min, MIN_ZOOM_DISTANCE);
        assert!(max >= fit_distance(12.0, 10.0, 16.0 / 9.0, FOV));
        assert!(zoom_limits(3.0, 3.0, 16.0 / 9.0, FOV).1 >= MIN_MAX_ZOOM_DISTANCE);
    }

    #[test]
    fn follow_target_stays_inside_the_room() {
        let t = clamp_to_room(Vec3::new(-3.0, 1.0, 50.0), 8.0, 6.0);
        assert_eq!(t, Vec3::new(0.0, 0.0, 6.0));
    }

    #[test]
    fn follow_converges_without_overshooting() {
        let goal = Vec3::new(5.0, 0.0, 5.0);
        let mut pos = Vec3::ZERO;
        for _ in 0..600 {
            let next = follow_step(pos, goal, 1.0 / 60.0);
            assert!(next.distance(goal) <= pos.distance(goal));
            pos = next;
        }
        assert!(pos.distance(goal) < 1e-3);
    }

    #[test]
    fn camera_keeps_looking_at_the_target() {
        let target = Vec3::new(2.0, 0.0, 7.0);
        let t = camera_transform(target, 9.0);
        assert!((t.translation.distance(target) - 9.0).abs() < 1e-4);
        let to_target = (target - t.translation).normalize();
        assert!(t.forward().as_vec3().abs_diff_eq(to_target, 1e-4));
    }

    #[test]
    fn camera_looks_at_the_room_center() {
        let t = frame_room(6.0, 6.0, 16.0 / 9.0, FOV);
        let center = Vec3::new(3.0, 0.0, 3.0);
        let to_center = (center - t.translation).normalize();
        assert!(t.forward().as_vec3().abs_diff_eq(to_center, 1e-4));
    }

    #[test]
    fn bigger_rooms_get_a_farther_camera() {
        let small = frame_room(6.0, 6.0, 16.0 / 9.0, FOV);
        let big = frame_room(12.0, 10.0, 16.0 / 9.0, FOV);
        let small_d = small.translation.distance(Vec3::new(3.0, 0.0, 3.0));
        let big_d = big.translation.distance(Vec3::new(6.0, 0.0, 5.0));
        assert!(big_d > small_d * 1.4, "{small_d} vs {big_d}");
    }

    #[test]
    fn every_room_corner_is_inside_the_screen_margin() {
        for (w, h, aspect) in [
            (6.0, 6.0, 16.0 / 9.0),
            (12.0, 10.0, 16.0 / 9.0),
            (10.0, 10.0, 1.0),
        ] {
            let t = frame_room(w, h, aspect, FOV);
            let inverse = t.compute_affine().inverse();
            for x in [0.0, w] {
                for z in [0.0, h] {
                    let view = inverse.transform_point3(Vec3::new(x, 0.0, z));
                    let ndc_y = (view.y / -view.z) / (FOV / 2.0).tan();
                    let ndc_x = (view.x / -view.z) / ((FOV / 2.0).tan() * aspect);
                    assert!(
                        ndc_x.abs() <= 1.0 && ndc_y.abs() <= 1.0,
                        "{w}x{h}: ({ndc_x}, {ndc_y})"
                    );
                }
            }
        }
    }
}
