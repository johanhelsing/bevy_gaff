use args::*;
use bevy::core::{Pod, Zeroable};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::log::LogPlugin;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_ggrs::ggrs::{Config, DesyncDetection, PlayerHandle, PlayerType, SessionBuilder};
use bevy_ggrs::{
    AddRollbackCommandExtension, GgrsAppExtension, GgrsPlugin, GgrsSchedule, PlayerInputs, Session,
};
use bevy_matchbox::prelude::*;
use bevy_xpbd_2d::{math::*, prelude::*};
use grabber_2d::GrabberPlugin;

mod args;
mod grabber_2d;

/// You need to define a config struct to bundle all the generics of GGRS. You can safely ignore
/// `State` and leave it as u8 for all GGRS functionality.
/// TODO: Find a way to hide the state type.
#[derive(Debug)]
pub struct GgrsConfig;
impl Config for GgrsConfig {
    type Input = BoxInput;
    type State = u8;
    type Address = PeerId;
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct BoxInput {
    pub inp: u8,
}

#[derive(Component)]
struct Marble;

#[derive(Component, Default, Reflect)]
#[reflect(Component, Hash)]
struct PreviousPosition(Vec2);

impl std::hash::Hash for PreviousPosition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.x.to_bits().hash(state);
        self.0.y.to_bits().hash(state);
    }
}

#[derive(Resource, Debug, Default, Reflect, Hash, Deref, DerefMut)]
#[reflect(Resource, Hash)]
pub struct FrameCount {
    pub frame: usize,
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

    let half_width = 3;
    let half_height = 3;
    // Spawn stacks of marbles
    for x in -half_width..half_width {
        for y in -half_height..half_height {
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
        let input = input.0.inp;
        for mut linear_velocity in &mut marbles {
            if input & INPUT_UP != 0 {
                linear_velocity.y += 50.0;
            }
            if input & INPUT_DOWN != 0 {
                linear_velocity.y -= 10.0;
            }
            if input & INPUT_LEFT != 0 {
                linear_velocity.x -= 10.0;
            }
            if input & INPUT_RIGHT != 0 {
                linear_velocity.x += 10.0;
            }
        }
    }
}

// fn pause_button(
//     current_state: ResMut<State<AppState>>,
//     mut next_state: ResMut<NextState<AppState>>,
//     keys: Res<Input<KeyCode>>,
// ) {
//     if keys.just_pressed(KeyCode::P) {
//         let new_state = match current_state.get() {
//             AppState::Paused => AppState::InGame,
//             AppState::InGame => AppState::Paused,
//             _ => current_state.clone(),
//         };
//         next_state.0 = Some(new_state);
//     }
// }

// fn step_button(mut physics_loop: ResMut<PhysicsLoop>, keys: Res<Input<KeyCode>>) {
//     if keys.just_pressed(KeyCode::Return) {
//         physics_loop.step();
//     }
// }

const FPS: usize = 60;

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Startup,
    Lobby,
    InGame,
    Paused,
}

const SKY_COLOR: Color = Color::rgb(0.69, 0.69, 0.69);

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
                        "info,wgpu_core=warn,wgpu_hal=warn,matchbox_socket=debug,bevy_ggrs=debug"
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
            GrabberPlugin,
        ))
        .add_ggrs_plugin(
            GgrsPlugin::<GgrsConfig>::new()
                // define frequency of rollback game logic update
                .with_update_frequency(FPS)
                // define system that returns inputs given a player handle, so GGRS can send the inputs
                // around
                .with_input_system(input) // register types of components AND resources you want to be rolled back
                .register_rollback_component::<Transform>()
                .register_rollback_component::<Position>()
                .register_rollback_component::<LinearVelocity>()
                .register_rollback_component::<AngularVelocity>()
                .register_rollback_component::<PreviousPosition>()
                .register_rollback_resource::<FrameCount>(),
        )
        .insert_resource(ClearColor(SKY_COLOR))
        .insert_resource(ClearColor(Color::rgb(0.05, 0.05, 0.1)))
        .insert_resource(SubstepCount(6))
        .insert_resource(Gravity(Vector::NEG_Y * 1000.0))
        .insert_resource(PhysicsTimestep::FixedOnce(1. / 60.))
        .init_resource::<FrameCount>()
        // Some of our systems need the query parameters
        .insert_resource(args)
        .add_state::<AppState>()
        .add_systems(Startup, (setup, setup_scene, spawn_marbles).chain())
        .add_systems(
            OnEnter(AppState::Lobby),
            (lobby_startup, start_matchbox_socket),
        )
        .add_systems(Update, lobby_system.run_if(in_state(AppState::Lobby)))
        .add_systems(OnExit(AppState::Lobby), lobby_cleanup)
        // .add_systems(
        //     OnEnter(AppState::InGame),
        //     (
        //         setup_scene,
        //         // spawn_marbles.after(setup_scene),
        //     ),
        // )
        .add_systems(Update, log_ggrs_events.run_if(in_state(AppState::InGame)))
        // these systems will be executed as part of the advance frame update
        .add_systems(
            GgrsSchedule,
            (
                // setup_scene,
                // spawn_marbles,
                step_physics,
                movement,
                update_previous_position,
                increase_frame_system,
            )
                .chain(),
        )
        // .add_systems(OnEnter(AppState::Paused), bevy_xpbd_2d::pause)
        // .add_systems(OnExit(AppState::Paused), bevy_xpbd_2d::resume)
        // .add_systems(Update, step_button.run_if(in_state(AppState::Paused)))
        .run();
}

fn setup(mut commands: Commands, mut app_state: ResMut<NextState<AppState>>, args: Res<Args>) {
    commands.spawn(Camera2dBundle::default());
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

fn start_matchbox_socket(mut commands: Commands, args: Res<Args>) {
    let room_id = match &args.room {
        Some(id) => id.clone(),
        None => format!("bevy_ggrs?next={}", &args.players),
    };

    let room_url = format!("{}/{}", &args.matchbox, room_id);
    info!("connecting to matchbox server: {room_url:?}");

    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

// Marker components for UI
#[derive(Component)]
struct LobbyText;
#[derive(Component)]
struct LobbyUI;

fn lobby_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // All this is just for spawning centered text.
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            background_color: Color::rgb(0.43, 0.41, 0.38).into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: Style {
                        align_self: AlignSelf::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    text: Text::from_section(
                        "Entering lobby...",
                        TextStyle {
                            font: asset_server.load("fonts/quicksand-light.ttf"),
                            font_size: 96.,
                            color: Color::BLACK,
                        },
                    ),
                    ..default()
                })
                .insert(LobbyText);
        })
        .insert(LobbyUI);
}

fn lobby_cleanup(query: Query<Entity, With<LobbyUI>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn lobby_system(
    mut app_state: ResMut<NextState<AppState>>,
    args: Res<Args>,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut commands: Commands,
    mut query: Query<&mut Text, With<LobbyText>>,
) {
    // regularly call update_peers to update the list of connected peers
    for (peer, new_state) in socket.update_peers() {
        // you can also handle the specific dis(connections) as they occur:
        match new_state {
            PeerState::Connected => info!("peer {peer} connected"),
            PeerState::Disconnected => info!("peer {peer} disconnected"),
        }
    }

    let connected_peers = socket.connected_peers().count();
    let remaining = args.players - (connected_peers + 1);
    query.single_mut().sections[0].value = format!("Waiting for {remaining} more player(s)",);
    if remaining > 0 {
        return;
    }

    info!("All peers have joined, going in-game");

    // extract final player list
    let players = socket.players();

    let mut session_builder = configure_session(args.players);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    let channel = socket.take_channel(0).unwrap();

    // start the GGRS session
    let session = session_builder
        .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(Session::P2P(session));

    // transition to in-game state
    app_state.set(AppState::InGame);
}

fn configure_session(players: usize) -> SessionBuilder<GgrsConfig> {
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
            }
        }
        Session::SyncTest(_) => {}
        _ => panic!("This example focuses on p2p and synctest"),
    }
}

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

fn input(_handle: In<PlayerHandle>, keyboard_input: Res<Input<KeyCode>>) -> BoxInput {
    let mut input: u8 = 0;

    if keyboard_input.pressed(KeyCode::W) {
        input |= INPUT_UP;
    }
    if keyboard_input.pressed(KeyCode::A) {
        input |= INPUT_LEFT;
    }
    if keyboard_input.pressed(KeyCode::S) {
        input |= INPUT_DOWN;
    }
    if keyboard_input.pressed(KeyCode::D) {
        input |= INPUT_RIGHT;
    }

    BoxInput { inp: input }
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
