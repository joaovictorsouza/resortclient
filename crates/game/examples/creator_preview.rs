//! Dev preview of the character creator (no server needed).
//!
//!   cargo run -p game --example creator_preview                       # interactive
//!   cargo run -p game --example creator_preview -- shot.png [categoria] [pose] # screenshot and exit
//!
//! The optional category is one of: face, skin, hair, top, bottom, shoes, accessories.
//! The optional pose is one of: idle, walk, sit, lay (plays that clip on the preview avatar).
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game::UiFont;
use game::avatar_anim::{AvatarAnimationPlugin, AvatarPose};
use game::character_creator::{
    AvatarPreviewRoot, CharacterCreatorPlugin, CharacterCreatorState, CreatorCategory,
};
use game::config::GameConfig;
use game::state::AppState;

fn main() {
    let shot = std::env::args().nth(1);
    let category = std::env::args()
        .nth(2)
        .and_then(|k| CreatorCategory::from_key(&k));

    let pose = std::env::args().nth(3).and_then(|p| match p.as_str() {
        "idle" => Some(AvatarPose::Idle),
        "walk" => Some(AvatarPose::Walk),
        "sit" => Some(AvatarPose::Sit),
        "lay" => Some(AvatarPose::Lay),
        _ => None,
    });

    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .init_resource::<UiFont>()
        .init_resource::<GameConfig>()
        .add_plugins((CharacterCreatorPlugin, AvatarAnimationPlugin))
        .add_systems(Startup, |mut next: ResMut<NextState<AppState>>| {
            next.set(AppState::CharacterCreator);
        });

    if let Some(category) = category {
        app.add_systems(
            Update,
            move |mut state: ResMut<CharacterCreatorState>, mut frame: Local<u32>| {
                *frame += 1;
                if *frame == 30 {
                    state.active_category = category;
                }
            },
        );
    }

    if let Some(pose) = pose {
        app.add_systems(
            Update,
            move |mut poses: Query<&mut AvatarPose, With<AvatarPreviewRoot>>| {
                for mut current in &mut poses {
                    if *current != pose {
                        *current = pose;
                    }
                }
            },
        );
    }

    if let Some(path) = shot {
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  mut frame: Local<u32>,
                  mut exit: MessageWriter<AppExit>| {
                *frame += 1;
                if *frame == 90 {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(path.clone()));
                }
                if *frame == 120 {
                    exit.write(AppExit::Success);
                }
            },
        );
    }
    app.run();
}
