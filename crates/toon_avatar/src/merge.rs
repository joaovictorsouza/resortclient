//! Draw-call reduction: skinned meshes that share a material and a skeleton are merged
//! into a single mesh.
//!
//! The body comes as 11 separate parts (so garments can hide the ones they cover), the
//! head has several pieces that use the skin material, and so on. Each separate mesh is one
//! draw call (two with shadows), which is the main cost on low-end phones. Within a body type
//! every GLB has the same joint order and bind poses, so merging is a plain concatenation.
//!
//! The merge is rebuilt whenever the set of visible meshes changes (equipping or removing an
//! item, swapping the body). Meshes with morph targets (the face) are left alone, and so are
//! meshes of other skeletons (hair and hats have their own).

use std::collections::HashMap;

use bevy::{mesh::skinning::SkinnedMesh, prelude::*};

use crate::{
    avatar::{Body, BodyPart, Equipped, OwnedBy, owner_of},
    material::ToonMaterial,
};

/// The merged mesh that replaces a group of source meshes.
#[derive(Component)]
struct MergedMesh {
    // Keeps the merged mesh asset alive for as long as the entity exists.
    _mesh: Handle<Mesh>,
}

/// A source mesh hidden because its geometry is part of a merged mesh.
#[derive(Component)]
struct MergedSource;

/// Whether meshes are merged (on by default; turn it off to compare draw calls).
#[derive(Resource)]
pub struct MergeEnabled(pub bool);

pub struct MergePlugin;

impl Plugin for MergePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MergeEnabled(true))
            .add_systems(Update, merge_meshes);
    }
}

/// Avatar, material and skeleton of a group of meshes that can be merged.
type GroupKey = (Entity, AssetId<ToonMaterial>, Vec<Entity>);

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn merge_meshes(
    mut commands: Commands,
    enabled: Res<MergeEnabled>,
    mut meshes: ResMut<Assets<Mesh>>,
    equipped: Query<&Equipped>,
    sources: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<ToonMaterial>,
            &SkinnedMesh,
            Option<&ChildOf>,
            Option<&BodyPart>,
        ),
        Without<MergedMesh>,
    >,
    parts: Query<&BodyPart>,
    parents: Query<&ChildOf>,
    owners: Query<&OwnedBy>,
    merged_sources: Query<Entity, With<MergedSource>>,
    merged: Query<Entity, With<MergedMesh>>,
    bodies: Query<(Entity, &Body)>,
    mut last: Local<Vec<Vec<Entity>>>,
) {
    // Group the eligible, visible meshes.
    let mut groups: HashMap<GroupKey, Vec<Entity>> = HashMap::new();
    if enabled.0 {
        for (entity, mesh, material, skin, parent, own_part) in &sources {
            let Some(avatar) = owner_of(entity, &parents, &owners) else {
                continue;
            };
            let part = own_part.or_else(|| parent.and_then(|p| parts.get(p.parent()).ok()));
            let covered = part.is_some_and(|p| {
                equipped
                    .iter()
                    .any(|e| e.avatar == avatar && e.hides.contains(&p.0))
            });
            if covered {
                continue;
            }
            let Some(asset) = meshes.get(&mesh.0) else {
                continue;
            };
            if asset.has_morph_targets() {
                continue;
            }
            groups
                .entry((avatar, material.0.id(), skin.joints.clone()))
                .or_default()
                .push(entity);
        }
    }
    groups.retain(|_, members| members.len() > 1);

    // Rebuild only when the grouping changed.
    let mut signature: Vec<Vec<Entity>> = groups
        .values()
        .map(|members| {
            let mut m = members.clone();
            m.sort();
            m
        })
        .collect();
    signature.sort();
    if signature == *last {
        return;
    }
    *last = signature;

    for entity in &merged {
        commands.entity(entity).try_despawn();
    }
    for entity in &merged_sources {
        commands
            .entity(entity)
            .try_remove::<MergedSource>()
            .try_insert(Visibility::Inherited);
    }

    let mut draw_calls_saved = 0;
    for ((avatar, _, joints), members) in &groups {
        let Some((body, _)) = bodies.iter().find(|(_, b)| b.avatar == *avatar) else {
            continue;
        };
        let Ok((_, first_mesh, material, skin, _, _)) = sources.get(members[0]) else {
            continue;
        };
        let Some(mut merged_mesh) = meshes.get(&first_mesh.0).cloned() else {
            continue;
        };
        let mut ok = true;
        for member in &members[1..] {
            let Ok((_, mesh, ..)) = sources.get(*member) else {
                ok = false;
                break;
            };
            let Some(other) = meshes.get(&mesh.0) else {
                ok = false;
                break;
            };
            if let Err(err) = merged_mesh.merge(other) {
                warn!("mesh merge failed: {err}");
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }
        let handle = meshes.add(merged_mesh);
        commands.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(material.0.clone()),
            SkinnedMesh {
                inverse_bindposes: skin.inverse_bindposes.clone(),
                joints: joints.clone(),
            },
            Transform::IDENTITY,
            MergedMesh { _mesh: handle },
            ChildOf(body),
        ));
        for member in members {
            commands
                .entity(*member)
                .try_insert((MergedSource, Visibility::Hidden));
        }
        draw_calls_saved += members.len() - 1;
    }
    info!(
        "mesh merge: {} groups, {draw_calls_saved} draw calls saved",
        groups.len()
    );
}
