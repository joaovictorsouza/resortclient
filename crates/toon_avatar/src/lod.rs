//! Device tiers and level of detail per avatar.
//!
//! * [`DeviceTier`] says how much the device can draw. It comes from the device itself
//!   ([`DeviceInfo`], classified by [`classify`]), can be overridden by the player
//!   ([`TierPreference`]) and is lowered at run time when frames take too long
//!   ([`FrameGovernor`]). It never goes up by itself within a session.
//! * [`AvatarLod`] is chosen for each avatar every frame: the local player ([`LodFocus`]) and the
//!   avatars that are big on the screen get the best level the device allows, the small and
//!   distant ones the lowest, and the lowest level also animates at a lower rate. The size on the
//!   screen (not the distance in the world) is what counts, so camera zoom is handled for free.
//!
//! The decision itself is the pure function [`desired_level`], tested without an engine.

use bevy::{prelude::*, render::renderer::RenderAdapterInfo};

use crate::{avatar::Avatar, quality::Quality};

/// How much the device can draw.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeviceTier {
    Low,
    Mid,
    High,
}

impl DeviceTier {
    pub fn label(self) -> &'static str {
        match self {
            DeviceTier::Low => "Low",
            DeviceTier::Mid => "Mid",
            DeviceTier::High => "High",
        }
    }

    /// The scene-level quality of the tier (shadow map, MSAA).
    pub fn quality(self) -> Quality {
        match self {
            DeviceTier::Low => Quality::Low,
            DeviceTier::Mid => Quality::Mid,
            DeviceTier::High => Quality::High,
        }
    }

    pub fn lower(self) -> Option<Self> {
        match self {
            DeviceTier::High => Some(DeviceTier::Mid),
            DeviceTier::Mid => Some(DeviceTier::Low),
            DeviceTier::Low => None,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "low" => Some(DeviceTier::Low),
            "mid" => Some(DeviceTier::Mid),
            "high" => Some(DeviceTier::High),
            _ => None,
        }
    }

    /// Frame time (ms) the tier is expected to hold.
    fn budget_ms(self) -> f32 {
        match self {
            DeviceTier::High => 1000.0 / 60.0,
            DeviceTier::Mid => 1000.0 / 45.0,
            DeviceTier::Low => 1000.0 / 30.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            DeviceTier::Low => DeviceTier::Mid,
            DeviceTier::Mid => DeviceTier::High,
            DeviceTier::High => DeviceTier::Low,
        }
    }
}

/// What the player chose for the quality of the characters.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TierPreference {
    /// Detect the device and adapt while playing.
    #[default]
    Auto,
    /// Always this tier.
    Fixed(DeviceTier),
}

/// What is known about the device. A game fills in the model; the GPU name comes from the
/// renderer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceInfo {
    /// Hardware identifier of a phone or tablet, for example `iPhone16,2` or `SM-S906E`.
    pub model: Option<String>,
    /// GPU name as reported by the driver, for example `Adreno (TM) 730` or `Apple A18 GPU`.
    pub gpu: String,
    /// Total memory in GiB, when known.
    pub ram_gib: Option<f32>,
    pub mobile: bool,
}

/// First number found in `text` after `prefix`, e.g. `("iPhone16,2", "iPhone") -> 16`.
fn number_after(text: &str, prefix: &str) -> Option<u32> {
    let rest = text.split(prefix).nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Sorts a device into a tier. The table is a starting point, to be calibrated with
/// measurements; an unknown device is `Mid`.
pub fn classify(info: &DeviceInfo) -> DeviceTier {
    if !info.mobile {
        return DeviceTier::High;
    }
    if info.ram_gib.is_some_and(|ram| ram < 4.0) {
        return DeviceTier::Low;
    }
    let model = info.model.as_deref().unwrap_or("");
    let gpu = info.gpu.as_str();
    let by_hardware = if model.starts_with("iPhone") {
        // iPhone15,2 is the 14 Pro (A16); iPhone16,1 is the 15 Pro (A17 Pro).
        number_after(model, "iPhone").map(|major| match major {
            16.. => DeviceTier::High,
            14..=15 => DeviceTier::Mid,
            _ => DeviceTier::Low,
        })
    } else if model.starts_with("iPad") {
        number_after(model, "iPad").map(|major| match major {
            13.. => DeviceTier::High,
            _ => DeviceTier::Mid,
        })
    } else if let Some(n) =
        number_after(gpu, "Adreno (TM) ").or_else(|| number_after(gpu, "Adreno "))
    {
        Some(match n {
            740.. => DeviceTier::High,
            640..=739 => DeviceTier::Mid,
            _ => DeviceTier::Low,
        })
    } else if let Some(n) = number_after(gpu, "Mali-G") {
        Some(match n {
            715.. => DeviceTier::High,
            710 | 78 | 77 | 76 => DeviceTier::Mid,
            _ => DeviceTier::Low,
        })
    } else if gpu.contains("Xclipse") {
        Some(DeviceTier::Mid)
    } else if gpu.starts_with("Apple") {
        Some(DeviceTier::High)
    } else {
        None
    };
    let tier = by_hardware.unwrap_or(DeviceTier::Mid);
    // Little memory caps the tier whatever the chip is.
    match info.ram_gib {
        Some(ram) if ram < 6.0 => tier.min_with(DeviceTier::Low),
        _ => tier,
    }
}

impl DeviceTier {
    fn min_with(self, other: Self) -> Self {
        let rank = |t: Self| match t {
            DeviceTier::Low => 0,
            DeviceTier::Mid => 1,
            DeviceTier::High => 2,
        };
        if rank(other) < rank(self) {
            other
        } else {
            self
        }
    }
}

/// How big an avatar is on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SizeClass {
    Near,
    #[default]
    Medium,
    Far,
}

/// Thresholds and limits of the level-of-detail policy.
#[derive(Resource, Clone, Copy, Debug)]
pub struct LodConfig {
    /// On-screen height (logical pixels) from which an avatar counts as near.
    pub near_px: f32,
    /// On-screen height below which an avatar counts as far.
    pub far_px: f32,
    /// Fraction a threshold is moved against the current class, to avoid flicker (0.15 = 15 %).
    pub hysteresis: f32,
    /// Shortest time between two level changes of one avatar.
    pub min_switch_secs: f32,
    /// Most avatars that get the highest level on a `High` device.
    pub max_full: usize,
    /// World height of an avatar, used to measure its size on the screen.
    pub avatar_height: f32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            near_px: 220.0,
            far_px: 90.0,
            hysteresis: 0.15,
            min_switch_secs: 0.5,
            max_full: 8,
            avatar_height: 1.8,
        }
    }
}

/// The level of detail of an avatar and how often it should animate. Written by this crate;
/// read it to throttle animation.
#[derive(Component, Clone, Copy, Debug)]
pub struct AvatarLod {
    pub level: Quality,
    pub class: SizeClass,
    /// Seconds between animation updates; 0 means every frame.
    pub anim_interval_secs: f32,
    /// False when the avatar is outside the view: it needs no animation at all.
    pub on_screen: bool,
    last_switch_secs: f32,
}

impl Default for AvatarLod {
    fn default() -> Self {
        Self {
            level: Quality::Mid,
            class: SizeClass::Medium,
            anim_interval_secs: 0.0,
            on_screen: true,
            last_switch_secs: f32::NEG_INFINITY,
        }
    }
}

/// Marks the avatar of the local player: it always gets the best level the device allows.
#[derive(Component)]
pub struct LodFocus;

/// Size class of an avatar of `px` pixels, moving each threshold against the previous class.
pub fn size_class(px: f32, previous: SizeClass, cfg: &LodConfig) -> SizeClass {
    let (grow, shrink) = (1.0 + cfg.hysteresis, 1.0 - cfg.hysteresis);
    let near = cfg.near_px
        * if previous == SizeClass::Near {
            shrink
        } else {
            grow
        };
    let far = cfg.far_px
        * if previous == SizeClass::Far {
            grow
        } else {
            shrink
        };
    if px >= near {
        SizeClass::Near
    } else if px < far {
        SizeClass::Far
    } else {
        SizeClass::Medium
    }
}

/// The level of detail an avatar should have. `full_slot` says whether it is among the few that
/// may get the highest level on a `High` device.
pub fn desired_level(tier: DeviceTier, focus: bool, class: SizeClass, full_slot: bool) -> Quality {
    if focus {
        return match tier {
            DeviceTier::High => Quality::High,
            DeviceTier::Mid | DeviceTier::Low => Quality::Mid,
        };
    }
    match (tier, class) {
        (_, SizeClass::Far) => Quality::Low,
        (DeviceTier::Low, _) => Quality::Low,
        (DeviceTier::High, SizeClass::Near) if full_slot => Quality::High,
        _ => Quality::Mid,
    }
}

/// Seconds between animation updates for an avatar at `level`.
pub fn animation_interval(level: Quality, class: SizeClass, px: f32, cfg: &LodConfig) -> f32 {
    match level {
        Quality::Low if class == SizeClass::Far && px < cfg.far_px * 0.5 => 0.2,
        Quality::Low => 0.1,
        _ => 0.0,
    }
}

/// Whether the frame times in `samples_ms` are too slow for `tier`: the 95th percentile is more
/// than 1.3 times the tier's budget.
pub fn too_slow(samples_ms: &[f32], tier: DeviceTier) -> bool {
    if samples_ms.len() < 30 {
        return false;
    }
    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(f32::total_cmp);
    let p95 = sorted[((sorted.len() as f32 * 0.95) as usize).min(sorted.len() - 1)];
    p95 > tier.budget_ms() * 1.3
}

/// Lowers the tier when frames stay too slow.
#[derive(Resource, Default)]
pub struct FrameGovernor {
    /// The tier never goes below this one: a flagship that stutters while assets download is still a
    /// flagship (set from the detected tier).
    floor: Option<DeviceTier>,
    samples_ms: Vec<f32>,
    window_secs: f32,
    slow_windows: u8,
}

const GOVERNOR_WINDOW_SECS: f32 = 5.0;
const GOVERNOR_SLOW_WINDOWS: u8 = 2;
/// Seconds after start-up that the governor ignores: loading and downloading stall frames.
const GOVERNOR_WARMUP_SECS: f32 = 30.0;

impl FrameGovernor {
    fn allows(&self, tier: DeviceTier) -> bool {
        self.floor.is_none_or(|floor| tier >= floor)
    }
}

pub struct LodPlugin;

impl Plugin for LodPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LodConfig>()
            .init_resource::<TierPreference>()
            .init_resource::<FrameGovernor>()
            .add_systems(Startup, detect_device_tier)
            .add_systems(
                Update,
                (apply_preference, govern_tier, update_avatar_lod).chain(),
            );
    }
}

/// Picks the first tier: the `TOON_AVATAR_TIER` environment variable, else the device.
fn detect_device_tier(
    mut commands: Commands,
    adapter: Option<Res<RenderAdapterInfo>>,
    info: Option<Res<DeviceInfoOverride>>,
    mut preference: ResMut<TierPreference>,
    mut governor: ResMut<FrameGovernor>,
) {
    if let Some(tier) = std::env::var("TOON_AVATAR_TIER")
        .ok()
        .and_then(|v| DeviceTier::parse(&v))
    {
        *preference = TierPreference::Fixed(tier);
    }
    let mut device = info.map(|i| i.0.clone()).unwrap_or_default();
    if device.gpu.is_empty() {
        if let Some(adapter) = adapter {
            device.gpu = adapter.name.clone();
        }
        device.mobile |= cfg!(any(target_os = "ios", target_os = "android"));
    }
    let tier = classify(&device);
    info!(
        "device tier: {} (gpu {:?}, model {:?})",
        tier.label(),
        device.gpu,
        device.model
    );
    governor.floor = Some(match tier {
        DeviceTier::High => DeviceTier::Mid,
        other => other,
    });
    commands.insert_resource(tier);
}

/// What a game knows about the device and the renderer cannot tell (the model, the memory).
#[derive(Resource, Clone, Debug)]
pub struct DeviceInfoOverride(pub DeviceInfo);

/// A fixed preference replaces the detected tier.
fn apply_preference(preference: Res<TierPreference>, tier: Option<ResMut<DeviceTier>>) {
    let (TierPreference::Fixed(wanted), Some(mut tier)) = (*preference, tier) else {
        return;
    };
    if preference.is_changed() && *tier != wanted {
        *tier = wanted;
    }
}

fn govern_tier(
    time: Res<Time>,
    preference: Res<TierPreference>,
    tier: Option<ResMut<DeviceTier>>,
    mut governor: ResMut<FrameGovernor>,
) {
    let Some(mut tier) = tier else {
        return;
    };
    if *preference != TierPreference::Auto || time.elapsed_secs() < GOVERNOR_WARMUP_SECS {
        return;
    }
    governor.samples_ms.push(time.delta_secs() * 1000.0);
    governor.window_secs += time.delta_secs();
    if governor.window_secs < GOVERNOR_WINDOW_SECS {
        return;
    }
    if too_slow(&governor.samples_ms, *tier) {
        governor.slow_windows += 1;
    } else {
        governor.slow_windows = 0;
    }
    governor.samples_ms.clear();
    governor.window_secs = 0.0;
    if governor.slow_windows >= GOVERNOR_SLOW_WINDOWS
        && let Some(lower) = tier.lower().filter(|l| governor.allows(*l))
    {
        warn!(
            "frames are too slow: lowering the device tier {} -> {}",
            tier.label(),
            lower.label()
        );
        *tier = lower;
        governor.slow_windows = 0;
    }
}

/// Chooses the level of detail and the animation rate of every avatar from its size on the
/// screen and the device tier.
#[allow(clippy::too_many_arguments)]
fn update_avatar_lod(
    time: Res<Time>,
    cfg: Res<LodConfig>,
    tier: Option<Res<DeviceTier>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut avatars: Query<(Entity, &GlobalTransform, &mut AvatarLod, Has<LodFocus>), With<Avatar>>,
) {
    let (Some(tier), Some((camera, camera_transform))) = (
        tier,
        cameras
            .iter()
            .find(|(c, _)| c.is_active && c.logical_viewport_size().is_some()),
    ) else {
        return;
    };
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };

    // On-screen height of every avatar, to rank them for the few full-detail slots.
    let mut measured: Vec<(Entity, f32, bool)> = Vec::new();
    for (entity, transform, _, _) in &avatars {
        let feet = transform.translation();
        let head = feet + Vec3::Y * cfg.avatar_height;
        let (Ok(a), Ok(b)) = (
            camera.world_to_viewport(camera_transform, feet),
            camera.world_to_viewport(camera_transform, head),
        ) else {
            measured.push((entity, 0.0, false));
            continue;
        };
        let margin = (a - b).length();
        let inside = |p: Vec2| {
            p.x > -margin && p.y > -margin && p.x < viewport.x + margin && p.y < viewport.y + margin
        };
        measured.push((entity, (a - b).length(), inside(a) || inside(b)));
    }
    let mut ranked: Vec<(Entity, f32)> = measured.iter().map(|(e, px, _)| (*e, *px)).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let full: Vec<Entity> = ranked.iter().take(cfg.max_full).map(|(e, _)| *e).collect();

    let now = time.elapsed_secs();
    for (entity, px, on_screen) in measured {
        let Ok((_, _, mut lod, focus)) = avatars.get_mut(entity) else {
            continue;
        };
        let mut next = *lod;
        next.on_screen = on_screen;
        if on_screen {
            next.class = size_class(px, lod.class, &cfg);
            let level = desired_level(*tier, focus, next.class, full.contains(&entity));
            if level != lod.level && now - lod.last_switch_secs >= cfg.min_switch_secs {
                next.level = level;
                next.last_switch_secs = now;
            }
            next.anim_interval_secs = animation_interval(next.level, next.class, px, &cfg);
        }
        // Avoid waking change detection (which rebuilds bodies) when nothing changed.
        if next.level != lod.level
            || next.class != lod.class
            || next.on_screen != lod.on_screen
            || next.anim_interval_secs != lod.anim_interval_secs
        {
            if next.level != lod.level {
                info!(
                    "avatar {entity:?}: level of detail {} -> {} ({px:.0} px)",
                    lod.level.label(),
                    next.level.label()
                );
            }
            *lod = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone(model: &str, gpu: &str, ram: Option<f32>) -> DeviceInfo {
        DeviceInfo {
            model: Some(model.into()),
            gpu: gpu.into(),
            ram_gib: ram,
            mobile: true,
        }
    }

    #[test]
    fn a_desktop_is_high() {
        assert_eq!(classify(&DeviceInfo::default()), DeviceTier::High);
    }

    #[test]
    fn iphones_are_sorted_by_generation() {
        let apple = "Apple A18 GPU";
        assert_eq!(
            classify(&phone("iPhone17,1", apple, Some(8.0))),
            DeviceTier::High
        );
        assert_eq!(
            classify(&phone("iPhone16,1", apple, Some(8.0))),
            DeviceTier::High
        );
        assert_eq!(
            classify(&phone("iPhone15,2", apple, Some(6.0))),
            DeviceTier::Mid
        );
        assert_eq!(
            classify(&phone("iPhone12,1", apple, Some(4.0))),
            DeviceTier::Low
        );
    }

    #[test]
    fn android_is_sorted_by_gpu_and_memory() {
        assert_eq!(
            classify(&phone("SM-S906E", "Adreno (TM) 730", Some(8.0))),
            DeviceTier::Mid
        );
        assert_eq!(
            classify(&phone("SM-S928B", "Adreno (TM) 750", Some(12.0))),
            DeviceTier::High
        );
        assert_eq!(
            classify(&phone("SM-A566E", "Samsung Xclipse 540", Some(8.0))),
            DeviceTier::Mid
        );
        assert_eq!(
            classify(&phone("X", "Mali-G715", Some(12.0))),
            DeviceTier::High
        );
        assert_eq!(
            classify(&phone("X", "Adreno (TM) 619", Some(6.0))),
            DeviceTier::Low
        );
        // A strong GPU with little memory is capped.
        assert_eq!(
            classify(&phone("X", "Adreno (TM) 750", Some(5.0))),
            DeviceTier::Low
        );
    }

    #[test]
    fn an_unknown_phone_is_mid() {
        assert_eq!(
            classify(&phone("Foo", "Mystery GPU", None)),
            DeviceTier::Mid
        );
    }

    #[test]
    fn thresholds_move_against_the_current_class() {
        let cfg = LodConfig::default();
        // 230 px is near for an avatar that was medium only after growing past 220 * 1.15.
        assert_eq!(
            size_class(230.0, SizeClass::Medium, &cfg),
            SizeClass::Medium
        );
        assert_eq!(size_class(260.0, SizeClass::Medium, &cfg), SizeClass::Near);
        // Once near it stays near until it shrinks below 220 * 0.85.
        assert_eq!(size_class(200.0, SizeClass::Near, &cfg), SizeClass::Near);
        assert_eq!(size_class(180.0, SizeClass::Near, &cfg), SizeClass::Medium);
        // Same on the far side.
        assert_eq!(size_class(85.0, SizeClass::Medium, &cfg), SizeClass::Medium);
        assert_eq!(size_class(75.0, SizeClass::Medium, &cfg), SizeClass::Far);
        assert_eq!(size_class(100.0, SizeClass::Far, &cfg), SizeClass::Far);
        assert_eq!(size_class(110.0, SizeClass::Far, &cfg), SizeClass::Medium);
    }

    #[test]
    fn the_local_player_gets_the_best_level_the_device_allows() {
        assert_eq!(
            desired_level(DeviceTier::High, true, SizeClass::Far, false),
            Quality::High
        );
        assert_eq!(
            desired_level(DeviceTier::Mid, true, SizeClass::Near, true),
            Quality::Mid
        );
        assert_eq!(
            desired_level(DeviceTier::Low, true, SizeClass::Near, true),
            Quality::Mid
        );
    }

    #[test]
    fn near_avatars_get_the_full_level_only_on_high_devices_and_only_in_the_few_slots() {
        assert_eq!(
            desired_level(DeviceTier::High, false, SizeClass::Near, true),
            Quality::High
        );
        assert_eq!(
            desired_level(DeviceTier::High, false, SizeClass::Near, false),
            Quality::Mid
        );
        assert_eq!(
            desired_level(DeviceTier::Mid, false, SizeClass::Near, true),
            Quality::Mid
        );
        assert_eq!(
            desired_level(DeviceTier::Low, false, SizeClass::Near, true),
            Quality::Low
        );
    }

    #[test]
    fn far_avatars_are_always_low() {
        for tier in [DeviceTier::High, DeviceTier::Mid, DeviceTier::Low] {
            assert_eq!(
                desired_level(tier, false, SizeClass::Far, true),
                Quality::Low
            );
        }
    }

    #[test]
    fn only_the_lowest_level_animates_at_a_lower_rate() {
        let cfg = LodConfig::default();
        assert_eq!(
            animation_interval(Quality::High, SizeClass::Near, 300.0, &cfg),
            0.0
        );
        assert_eq!(
            animation_interval(Quality::Mid, SizeClass::Medium, 150.0, &cfg),
            0.0
        );
        assert_eq!(
            animation_interval(Quality::Low, SizeClass::Far, 80.0, &cfg),
            0.1
        );
        assert_eq!(
            animation_interval(Quality::Low, SizeClass::Far, 30.0, &cfg),
            0.2
        );
    }

    #[test]
    fn the_governor_only_complains_about_a_slow_95th_percentile() {
        let fine = vec![16.0; 100];
        assert!(!too_slow(&fine, DeviceTier::High));
        let mut slow = vec![16.0; 90];
        slow.extend(vec![40.0; 10]);
        assert!(too_slow(&slow, DeviceTier::High));
        // A few spikes are tolerated.
        let mut spikes = vec![16.0; 97];
        spikes.extend(vec![80.0; 3]);
        assert!(!too_slow(&spikes, DeviceTier::High));
        // Too few samples say nothing.
        assert!(!too_slow(&[100.0; 10], DeviceTier::High));
    }

    #[test]
    fn a_high_device_is_never_governed_below_mid() {
        let governor = FrameGovernor {
            floor: Some(DeviceTier::Mid),
            ..default()
        };
        assert!(governor.allows(DeviceTier::Mid));
        assert!(!governor.allows(DeviceTier::Low));
        assert!(FrameGovernor::default().allows(DeviceTier::Low));
    }

    #[test]
    fn a_tier_only_goes_down() {
        assert_eq!(DeviceTier::High.lower(), Some(DeviceTier::Mid));
        assert_eq!(DeviceTier::Low.lower(), None);
    }
}
