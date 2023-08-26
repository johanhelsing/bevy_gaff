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
pub struct GrabberJoint;

/// The point that the grabbed entity should follow, positioned at the cursor position.
#[derive(Component)]
pub struct GrabPoint;

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn grab(
    mut commands: Commands,
    mut grabbers: Query<(Entity, &mut Position), (With<GrabPoint>, Without<Collider>)>,
    joints: Query<(Entity, &DistanceJoint), With<GrabberJoint>>,
    bodies: Query<(&RigidBody, &Position, &Rotation), Without<GrabPoint>>,
    spatial_query: SpatialQuery,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for input in inputs.iter() {
        let buttons = input.0.buttons;
        // If grab button is pressed, spawn or update grab point and grabber joint if they don't exist
        if buttons & INPUT_MOUSE_LEFT != 0 {
            let cursor_world_pos = input.0.mouse_pos;
            info!("mouse left held, updating grab {cursor_world_pos}");
            let grabber_entity: Entity;

            // If grabber exists, update its position, otherwise spawn it
            if let Ok((entity, mut position)) = grabbers.get_single_mut() {
                position.0 = cursor_world_pos;
                grabber_entity = entity;
            } else {
                grabber_entity = commands
                    .spawn((RigidBody::Kinematic, Position(cursor_world_pos), GrabPoint))
                    .add_rollback()
                    .id();
            }

            // If grabber joint doesn't exist, spawn it
            if joints.is_empty() {
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
                                    GrabberJoint,
                                ))
                                .add_rollback();
                        }
                    }
                }
            }
        } else {
            // // If grab button is released, despawn any grabbers and grabber joints
            // for (entity, _) in &grabbers {
            //     commands.entity(entity).despawn_recursive();
            // }
            // for (entity, _) in &joints {
            //     commands.entity(entity).despawn_recursive();
            // }
        }
    }
}
