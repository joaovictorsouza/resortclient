use bevy::{
    animation::{AnimatedBy, AnimationTargetId},
    prelude::*,
};

use crate::action::AnimalAction;
use crate::animal::{Animal, AnimalConfig, AnimalRoute, AnimalScene, Libraries, playback_speed};

const TRANSITION: std::time::Duration = std::time::Duration::from_millis(300);
/// Name of the skeleton root in the animation file: the paths of the animation targets start with it.
const RIG_ROOT_NAME: &str = "Armature";

/// The skeleton of a scene already has its animation targets.
#[derive(Component)]
pub(crate) struct RigBound;

/// Lives next to the [`AnimationPlayer`] on the armature node.
#[derive(Component)]
pub(crate) struct AnimalRig {
    animal: Entity,
    current: Option<AnimalAction>,
}

/// Depth-first `(entity, name path from the rig root)` of the armature's subtree.
fn rig_paths(
    rig: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Vec<(Entity, Vec<Name>)> {
    let mut out = Vec::new();
    let mut stack = vec![(rig, vec![Name::new(RIG_ROOT_NAME)])];
    while let Some((entity, path)) = stack.pop() {
        if let Ok(kids) = children.get(entity) {
            for child in kids.iter() {
                if let Ok(name) = names.get(child) {
                    let mut child_path = path.clone();
                    child_path.push(name.clone());
                    stack.push((child, child_path));
                }
            }
        }
        out.push((entity, path));
    }
    out
}

/// Gives a freshly spawned scene its animation player and targets.
#[allow(clippy::type_complexity)]
pub(crate) fn bind_rig(
    mut commands: Commands,
    libraries: Res<Libraries>,
    animals: Query<&Animal>,
    scenes: Query<(Entity, &ChildOf), (With<AnimalScene>, Without<RigBound>)>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for (scene, model) in &scenes {
        let Some(animal) = parents.get(model.parent()).ok().map(ChildOf::parent) else {
            continue;
        };
        let Some(graph) = animals
            .get(animal)
            .ok()
            .and_then(|a| libraries.0.get(&a.species))
            .and_then(|l| l.graph.clone())
        else {
            continue;
        };
        let Some(rig) = children
            .iter_descendants(scene)
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "Armature"))
        else {
            continue;
        };
        for (entity, path) in rig_paths(rig, &children, &names) {
            commands
                .entity(entity)
                .try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(rig)));
        }
        commands.entity(rig).try_insert((
            AnimationPlayer::default(),
            AnimationGraphHandle(graph),
            AnimationTransitions::new(),
            AnimalRig {
                animal,
                current: None,
            },
        ));
        commands.entity(scene).try_insert(RigBound);
    }
}

/// Plays the clip of what the animal is doing.
pub(crate) fn drive_animation(
    config: Res<AnimalConfig>,
    libraries: Res<Libraries>,
    animals: Query<(&Animal, &AnimalAction, Option<&AnimalRoute>)>,
    mut rigs: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut AnimalRig,
    )>,
) {
    for (mut player, mut transitions, mut rig) in &mut rigs {
        let Ok((animal, action, route)) = animals.get(rig.animal) else {
            continue;
        };
        if rig.current == Some(*action) {
            continue;
        }
        let (Some(data), Some(library)) = (
            config.species.get(&animal.species),
            libraries.0.get(&animal.species),
        ) else {
            continue;
        };
        let clip = action.clip();
        let Some(&node) = library.nodes.get(clip) else {
            continue;
        };
        let meta = data.clips.get(clip);
        let speed = match (action.is_gait(), route) {
            (true, Some(route)) => playback_speed(
                route.tiles_per_sec,
                meta.and_then(|m| m.tiles_per_sec),
                animal.height / data.model_height_m.max(0.01),
            ),
            _ => 1.0,
        };
        let active = transitions.play(&mut player, node, TRANSITION);
        if meta.is_none_or(|m| m.looping) {
            active.repeat();
        }
        active.set_speed(speed);
        rig.current = Some(*action);
    }
}
