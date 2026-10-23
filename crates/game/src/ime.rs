//! On-screen keyboard for text fields on touch devices.
//!
//! Neither Bevy nor `bevy_hui_widgets` ever set `Window::ime_enabled`, and winit only brings up the
//! iOS keyboard (it makes its view first responder) when the IME is allowed, so tapping a
//! `TextInput` focused the field (`UiActive`) but never showed a keyboard. The key presses the
//! keyboard produces then reach the widget as ordinary `KeyboardInput` events.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_hui_widgets::prelude::TextInput;

/// Touch devices have no hardware keyboard; on desktop the IME is left alone.
pub const MOBILE: bool = cfg!(any(target_os = "ios", target_os = "android"));

/// The keyboard is wanted while at least one text field is focused.
pub fn wants_keyboard(focused_inputs: usize) -> bool {
    focused_inputs > 0
}

pub struct ImePlugin;

impl Plugin for ImePlugin {
    fn build(&self, app: &mut App) {
        if MOBILE {
            app.add_systems(Update, (sync_ime, return_key_bridge));
        }
    }
}

/// The iOS keyboard types its Return key as the character `\n`, not as `Key::Enter`.
pub fn is_return(key: &Key) -> bool {
    matches!(key, Key::Character(c) if c == "\n" || c == "\r")
}

/// Treats the Return of the on-screen keyboard like a hardware Enter: it unfocuses the field
/// (which hides the keyboard) and registers `KeyCode::Enter` for the screens that submit on Enter.
/// Screens that read that must run `.after(return_key_bridge)`.
pub(crate) fn return_key_bridge(
    mut events: MessageReader<KeyboardInput>,
    focused: Query<Entity, (With<TextInput>, With<bevy_hui::prelude::UiActive>)>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for event in events.read() {
        if !event.state.is_pressed() || !is_return(&event.logical_key) || focused.is_empty() {
            continue;
        }
        for entity in &focused {
            commands
                .entity(entity)
                .remove::<bevy_hui::prelude::UiActive>();
        }
        keys.press(KeyCode::Enter);
        keys.release(KeyCode::Enter);
    }
}

fn sync_ime(
    focused: Query<(), (With<TextInput>, With<bevy_hui::prelude::UiActive>)>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let want = wants_keyboard(focused.iter().count());
    for mut window in &mut windows {
        if window.ime_enabled != want {
            window.ime_enabled = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_hui::prelude::UiActive;

    #[test]
    fn keyboard_is_wanted_only_with_a_focused_field() {
        assert!(!wants_keyboard(0));
        assert!(wants_keyboard(1));
        assert!(wants_keyboard(3));
    }

    #[test]
    fn ime_follows_the_focus_of_text_inputs() {
        let mut app = App::new();
        app.add_systems(Update, sync_ime);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let field = app.world_mut().spawn(TextInput::default()).id();

        app.update();
        assert!(!app.world().get::<Window>(window).unwrap().ime_enabled);

        app.world_mut().entity_mut(field).insert(UiActive);
        app.update();
        assert!(app.world().get::<Window>(window).unwrap().ime_enabled);

        app.world_mut().entity_mut(field).remove::<UiActive>();
        app.update();
        assert!(!app.world().get::<Window>(window).unwrap().ime_enabled);
    }

    #[test]
    fn return_character_is_recognised() {
        assert!(is_return(&Key::Character("\n".into())));
        assert!(is_return(&Key::Character("\r".into())));
        assert!(!is_return(&Key::Character("a".into())));
        assert!(!is_return(&Key::Enter));
    }

    #[test]
    fn return_unfocuses_the_field_and_registers_enter() {
        use bevy::input::ButtonState;
        let mut app = App::new();
        app.add_message::<KeyboardInput>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, return_key_bridge);
        let field = app.world_mut().spawn((TextInput::default(), UiActive)).id();
        let window = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
            logical_key: Key::Character("\n".into()),
            state: ButtonState::Pressed,
            text: Some("\n".into()),
            repeat: false,
            window,
        });
        app.update();
        assert!(app.world().get::<UiActive>(field).is_none());
        let keys = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(keys.just_pressed(KeyCode::Enter));
    }

    #[test]
    fn closing_the_screen_hides_the_keyboard() {
        let mut app = App::new();
        app.add_systems(Update, sync_ime);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let field = app.world_mut().spawn((TextInput::default(), UiActive)).id();
        app.update();
        assert!(app.world().get::<Window>(window).unwrap().ime_enabled);

        app.world_mut().despawn(field);
        app.update();
        assert!(!app.world().get::<Window>(window).unwrap().ime_enabled);
    }
}
