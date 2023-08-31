use crate::{input::*, lobby::LobbyPlugin};
use args::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::log::LogPlugin;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_ggrs::ggrs::{Config, GGRSEvent, PlayerType, SessionBuilder};
use bevy_ggrs::{
    AddRollbackCommandExtension, GgrsAppExtension, GgrsPlugin, GgrsSchedule, PlayerInputs, Session,
};
use bevy_matchbox::prelude::*;
use bevy_xpbd_2d::{math::*, prelude::*};
use grabber_2d::GrabberPlugin;

mod args;
mod grabber_2d;
mod input;
mod lobby;

const FPS: usize = 60;

#[derive(Debug)]
pub struct GgrsConfig;
impl Config for GgrsConfig {
    type Input = GaffInput;
    type State = u8;
    type Address = PeerId;
}

#[derive(Component)]
struct Marble;

/// just used for desync detection for now
#[derive(Component, Default, Reflect)]
#[reflect(Component, Hash)]
struct PreviousPosition(Vec2);

#[derive(Component)]
pub struct MainCamera;

impl std::hash::Hash for PreviousPosition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.x.to_bits().hash(state);
        self.0.y.to_bits().hash(state);
    }
}

#[derive(Resource, Debug, Default, Reflect, Hash, Deref, DerefMut)]
#[reflect(Resource, Hash)]
struct FrameCount {
    frame: usize,
}

fn setup_scene(mut commands: Commands, frame: Res<FrameCount>) {
    if **frame != 0 {
        return;
    }

    info!("Setting up scene");
    let square_sprite = Sprite {
        color: Color::rgb(0.7, 0.7, 0.8),
        custom_size: Some(Vec2::splat(50.0)),
        ..default()
    };

    // Ceiling
    commands
        .spawn((
            SpriteBundle {
                sprite: square_sprite.clone(),
                transform: Transform::from_scale(Vec3::new(20.0, 1.0, 1.0)),
                ..default()
            },
            RigidBody::Static,
            Position(Vector::Y * 50.0 * 6.0),
            Collider::cuboid(50.0 * 20.0, 50.0),
        ))
        .add_rollback();

    // Floor
    commands
        .spawn((
            SpriteBundle {
                sprite: square_sprite.clone(),
                transform: Transform::from_scale(Vec3::new(20.0, 1.0, 1.0)),
                ..default()
            },
            RigidBody::Static,
            Position(Vector::NEG_Y * 50.0 * 6.0),
            Collider::cuboid(50.0 * 20.0, 50.0),
        ))
        .add_rollback();

    // Left wall
    commands
        .spawn((
            SpriteBundle {
                sprite: square_sprite.clone(),
                transform: Transform::from_scale(Vec3::new(1.0, 11.0, 1.0)),
                ..default()
            },
            RigidBody::Static,
            Position(Vector::NEG_X * 50.0 * 9.5),
            Collider::cuboid(50.0, 50.0 * 11.0),
        ))
        .add_rollback();

    // Right wall
    commands
        .spawn((
            SpriteBundle {
                sprite: square_sprite,
                transform: Transform::from_scale(Vec3::new(1.0, 11.0, 1.0)),
                ..default()
            },
            RigidBody::Static,
            Position(Vector::X * 50.0 * 9.5),
            Collider::cuboid(50.0, 50.0 * 11.0),
        ))
        .add_rollback();
}

fn spawn_marbles(
    mut commands: Commands,
    frame_count: Res<FrameCount>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if **frame_count != 0 {
        info!("not spawning marbles on frame {frame_count:?}");
        return;
    }
    info!("Spawning marbles");

    let marble_radius = 10.0;
    let marble_mesh = MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Circle::new(marble_radius as f32).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::rgb(0.2, 0.7, 0.9))),
        ..default()
    };

    let half_width = 9;
    let half_height = 9;

    // Spawn stacks of marbles
    for x in -half_width..=half_width {
        for y in -half_height..=half_height {
            let position = Vector::new(
                x as Scalar * (2.5 * marble_radius),
                y as Scalar * (2.5 * marble_radius),
            );
            commands
                .spawn((
                    marble_mesh.clone(),
                    RigidBody::Dynamic,
                    Position(position),
                    Rotation::default(),
                    Collider::ball(marble_radius),
                    Friction::new(0.0),
                    PreviousPosition(position),
                    Marble,
                ))
                .add_rollback();
        }
    }
}

fn movement(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut marbles: Query<&mut LinearVelocity, With<Marble>>,
) {
    for input in inputs.iter() {
        let buttons = input.0.buttons;
        for mut linear_velocity in &mut marbles {
            if buttons & INPUT_UP != 0 {
                linear_velocity.y += 50.0;
            }
            if buttons & INPUT_DOWN != 0 {
                linear_velocity.y -= 10.0;
            }
            if buttons & INPUT_LEFT != 0 {
                linear_velocity.x -= 10.0;
            }
            if buttons & INPUT_RIGHT != 0 {
                linear_velocity.x += 10.0;
            }
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Startup,
    Lobby,
    InGame,
    Paused,
}

#[derive(ScheduleLabel, Clone, Debug, Hash, Eq, PartialEq)]
struct PhysicsSchedule;

fn main() {
    // read query string or command line arguments
    let args = Args::get();
    info!("{args:?}");

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(LogPlugin {
                    filter:
                        // "info,wgpu_core=warn,wgpu_hal=warn,matchbox_socket=debug,bevy_ggrs=debug"
                            "info,wgpu_core=warn,wgpu_hal=warn,matchbox_socket=debug"
                            .into(),
                    level: bevy::log::Level::DEBUG,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true, // behave on wasm
                        ..default()
                    }),
                    ..default()
                }),
            PhysicsPlugins::new(PhysicsSchedule),
            FrameTimeDiagnosticsPlugin,
            LobbyPlugin,
            GrabberPlugin,
        ))
        .add_ggrs_plugin(
            GgrsPlugin::<GgrsConfig>::new()
                .with_update_frequency(FPS)
                .with_input_system(input)
                .register_rollback_component::<Transform>()
                .register_rollback_component::<Position>()
                .register_rollback_component::<LinearVelocity>()
                .register_rollback_component::<AngularVelocity>()
                .register_rollback_component::<PreviousPosition>()
                .register_rollback_resource::<FrameCount>(),
        )
        .insert_resource(ClearColor(Color::rgb(0.05, 0.05, 0.1)))
        .insert_resource(SubstepCount(6))
        .insert_resource(Gravity(Vector::NEG_Y * 1000.0))
        .insert_resource(PhysicsTimestep::FixedOnce(1. / FPS as f32))
        .init_resource::<FrameCount>()
        // Some of our systems need the query parameters
        .insert_resource(args)
        .add_state::<AppState>()
        .add_systems(Startup, (setup, setup_scene, spawn_marbles).chain())
        .add_systems(Update, log_ggrs_events.run_if(in_state(AppState::InGame)))
        // these systems will be executed as part of the advance frame update
        .add_systems(
            GgrsSchedule,
            (
                // ideally these systems should be part of the rollback schedule, but seems it breaks
                // synctest sessions for some reason... should investigate...
                // setup_scene,
                // spawn_marbles,
                step_physics,
                movement,
                update_previous_position,
                increase_frame_system,
            )
                .chain(),
        )
        .add_systems(GgrsSchedule, grabber_2d::grab.before(step_physics))
        .run();
}

fn setup(mut commands: Commands, mut app_state: ResMut<NextState<AppState>>, args: Res<Args>) {
    commands.spawn((MainCamera, Camera2dBundle::default()));
    if args.players == 1 {
        info!("starting synctest session");
        let mut session_builder = configure_session(1);
        session_builder = session_builder
            .add_player(PlayerType::Local, 0)
            .expect("failed to add player");
        let session = session_builder
            .start_synctest_session()
            .expect("failed to start synctest session");
        commands.insert_resource(Session::SyncTest(session));
        app_state.set(AppState::InGame)
    } else {
        info!("joining multiplayer lobby");
        app_state.set(AppState::Lobby)
    }
}

pub fn configure_session(players: usize) -> SessionBuilder<GgrsConfig> {
    SessionBuilder::<GgrsConfig>::new()
        .with_num_players(players)
        .with_max_prediction_window(12)
        .with_input_delay(2)
        .with_fps(FPS)
        .expect("invalid fps")
}

fn log_ggrs_events(mut session: ResMut<Session<GgrsConfig>>) {
    match session.as_mut() {
        Session::P2P(s) => {
            for event in s.events() {
                info!("GGRS Event: {event:?}");
                if let GGRSEvent::DesyncDetected { .. } = event {
                    panic!("desynced!");
                }
            }
        }
        Session::SyncTest(_) => {}
        _ => panic!("This example focuses on p2p and synctest"),
    }
}

fn increase_frame_system(mut frame_count: ResMut<FrameCount>) {
    frame_count.frame += 1;
}

fn update_previous_position(mut positions: Query<(&mut PreviousPosition, &Position)>) {
    for (mut previous_position, position) in &mut positions {
        previous_position.0 = position.0;
    }
}

fn step_physics(world: &mut World) {
    world.run_schedule(PhysicsSchedule);
}
