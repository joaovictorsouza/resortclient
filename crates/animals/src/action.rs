/// What an animal is doing; each action plays the clip of the same name.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnimalAction {
    #[default]
    Idle,
    Sit,
    Lay,
    Walk,
    Run,
    Bark,
    Pant,
}

impl AnimalAction {
    pub fn clip(self) -> &'static str {
        match self {
            AnimalAction::Idle => "idle",
            AnimalAction::Sit => "sit",
            AnimalAction::Lay => "lay",
            AnimalAction::Walk => "walk",
            AnimalAction::Run => "run",
            AnimalAction::Bark => "bark",
            AnimalAction::Pant => "pant",
        }
    }

    /// The action called `name` (the clip name, as the server sends it).
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "idle" => AnimalAction::Idle,
            "sit" => AnimalAction::Sit,
            "lay" => AnimalAction::Lay,
            "walk" => AnimalAction::Walk,
            "run" => AnimalAction::Run,
            "bark" => AnimalAction::Bark,
            "pant" => AnimalAction::Pant,
            _ => return None,
        })
    }

    /// Whether the clip carries the animal along a route (a gait).
    pub fn is_gait(self) -> bool {
        matches!(self, AnimalAction::Walk | AnimalAction::Run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_round_trips_through_its_clip_name() {
        for action in [
            AnimalAction::Idle,
            AnimalAction::Sit,
            AnimalAction::Lay,
            AnimalAction::Walk,
            AnimalAction::Run,
            AnimalAction::Bark,
            AnimalAction::Pant,
        ] {
            assert_eq!(AnimalAction::from_name(action.clip()), Some(action));
        }
        assert_eq!(AnimalAction::from_name("fly"), None);
    }

    #[test]
    fn only_walk_and_run_are_gaits() {
        assert!(AnimalAction::Walk.is_gait() && AnimalAction::Run.is_gait());
        assert!(!AnimalAction::Idle.is_gait() && !AnimalAction::Bark.is_gait());
    }
}
