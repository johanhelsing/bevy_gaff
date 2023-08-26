//! 2D grabber plugin for bevy_xpbd_2d

use bevy::prelude::*;
use bevy_ggrs::{AddRollbackCommandExtension, PlayerInputs};
use bevy_xpbd_2d::{math::*, prelude::*};

use crate::{input::INPUT_MOUSE_LEFT, GgrsConfig};

#[derive(Default)]
pub struct GrabberPlugin;

impl Plugin for GrabberPlugin {
    fn build(&self, _app: &mut App) {
        info!("Adding grabber plugin");
        // todo: why is this borked?
        // app.add_systems(GgrsSchedule, grab.before(step_physics));
    }
}

// Hard coded for now, could probably use a resource though
const GRAB_MIN_DISTANCE: Scalar = 100.0;
const GRAB_COMPLIANCE: Scalar = 0.000_001;
const GRAB_LINEAR_DAMPING: Scalar = 5.0;
const GRAB_ANGULAR_DAMPING: Scalar = 1.0;

/// A marker component for joints used by grabbers.
#[derive(Component)]
pub struct GrabberJoint {
    player_handle: usize,
}

/// The point that the grabbed entity should follow, positioned at the cursor position.
#[derive(Component)]
pub struct Grabber {
    player_handle: usize,
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn grab(
    mut commands: Commands,
    mut grabbers: Query<(Entity, &Grabber, &mut Position), (With<Grabber>, Without<Collider>)>,
    joints: Query<(Entity, &GrabberJoint, &DistanceJoint)>,
    bodies: Query<(&RigidBody, &Position, &Rotation), Without<Grabber>>,
    spatial_query: SpatialQuery,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for (player_handle, input) in inputs.iter().enumerate() {
        let buttons = input.0.buttons;
        // If grab button is pressed, spawn or update grab point and grabber joint if they don't exist
        if buttons & INPUT_MOUSE_LEFT != 0 {
            let cursor_world_pos = input.0.mouse_pos;
            info!("mouse left held, updating grab {cursor_world_pos}");

            // If grabber exists, update its position, otherwise spawn it
            let grabber_entity = if let Some((entity, _grabber, mut position)) = grabbers
                .iter_mut()
                .find(|(_entity, grabber, _position)| grabber.player_handle == player_handle)
            {
                position.0 = cursor_world_pos;
                entity
            } else {
                commands
                    .spawn((
                        RigidBody::Kinematic,
                        Position(cursor_world_pos),
                        Grabber { player_handle },
                    ))
                    .add_rollback()
                    .id()
            };

            // if joints.is_empty() {
            if joints
                .iter()
                .find(|(_entity, grabber_joint, _joint)| {
                    grabber_joint.player_handle == player_handle
                })
                .is_none()
            {
                // Use point projection to find closest point on collider
                let filter = SpatialQueryFilter::default();
                let projection = spatial_query.project_point(cursor_world_pos, true, filter);

                if let Some(projection) = projection {
                    if projection.point.distance(cursor_world_pos) <= GRAB_MIN_DISTANCE {
                        // Spawn grabber joint
                        if let Ok((_, position, rotation)) = bodies.get(projection.entity) {
                            commands
                                .spawn((
                                    DistanceJoint::new(grabber_entity, projection.entity)
                                        .with_compliance(GRAB_COMPLIANCE)
                                        .with_local_anchor_2(
                                            rotation
                                                .inverse()
                                                .rotate(projection.point - position.0),
                                        )
                                        .with_linear_velocity_damping(GRAB_LINEAR_DAMPING)
                                        .with_angular_velocity_damping(GRAB_ANGULAR_DAMPING),
                                    GrabberJoint { player_handle },
                                ))
                                .add_rollback();
                        }
                    }
                }
            }
        } else {
            // If grab button is released, despawn any grabbers and grabber joints
            for (entity, grabber, _) in &grabbers {
                if (grabber.player_handle) == player_handle {
                    commands.entity(entity).despawn_recursive();
                }
            }
            for (entity, grabber_joint, _) in &joints {
                if (grabber_joint.player_handle) == player_handle {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
}
