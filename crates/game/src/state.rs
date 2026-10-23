use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    InitAuth,
    Login,
    ChooseNickname,
    CharacterCreator,
    Connecting,
    InRoom,
    ConnectionError,
}
