use backroll_transport_udp::*;
use bevy::tasks::IoTaskPool;
use bevy::{core::FixedTimestep, prelude::*};
use bevy_backroll::{backroll::*, *};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;

pub type P2PSession = bevy_backroll::backroll::P2PSession<BackrollConfig>;

pub struct BackrollConfig;

#[macro_use]
extern crate bitflags;

#[derive(Clone)]
pub struct Player {
    //position: Vec2,
    //velocity: Vec2,
    //size: Vec2,
    handle: PlayerHandle, // the network id
}

bitflags! {
    #[derive(Default, Pod, Zeroable)]
    #[repr(C)]
    pub struct PlayerInputFrame: u32 {
        // bit shift the stuff in the input struct
        const UP = 1<<0;
        const DOWN = 1<<1;
        const LEFT = 1<<2;
        const RIGHT = 1<<3;
    }
}

impl Config for BackrollConfig {
    type Input = PlayerInputFrame;
    type State = GameState;
}

#[derive(Clone, PartialEq, Hash)]
pub struct GameState {}

struct Materials {
    player_material: Handle<ColorMaterial>,
}

const MATCH_UPDATE_LABEL: &str = "MATCH_UPDATE";

const DELTA_TIME: f32 = 1.0 / 60.0; // in ms

pub struct OurBackrollPlugin;

impl Plugin for OurBackrollPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_plugin(BackrollPlugin::<BackrollConfig>::default())
            .with_rollback_run_criteria::<BackrollConfig, _>(
                FixedTimestep::step(DELTA_TIME.into()).with_label(MATCH_UPDATE_LABEL),
            )
            .with_input_sampler_system::<BackrollConfig, _>(sample_input.system())
            .with_world_save_system::<BackrollConfig, _>(save_world.system())
            .with_world_load_system::<BackrollConfig, _>(load_world.system());
    }
}

#[derive(Debug)]
struct StartupNetworkConfig {
    client: usize,
    bind: SocketAddr,
    remote: SocketAddr,
}

fn sample_input(handle: In<PlayerHandle>, keyboard_input: Res<Input<KeyCode>>) -> PlayerInputFrame {
    let mut local_input = PlayerInputFrame::empty();

    // local input handling
    {
        if keyboard_input.pressed(KeyCode::Left) {
            local_input.insert(PlayerInputFrame::LEFT);
            println!("Left");
        } else if keyboard_input.pressed(KeyCode::Right) {
            local_input.insert(PlayerInputFrame::RIGHT);
            println!("Right");
        }

        if keyboard_input.pressed(KeyCode::Up) {
            local_input.insert(PlayerInputFrame::UP);
            println!("Up");
        } else if keyboard_input.pressed(KeyCode::Down) {
            local_input.insert(PlayerInputFrame::DOWN);
            println!("Down");
        }
    }

    local_input
}

fn save_world() -> GameState {
    println!("Save da world");
    GameState {}
}

fn load_world(state: In<GameState>) {
    println!("Load da world");
}

fn setup_game(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.insert_resource(Materials {
        player_material: materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
    });
}

fn spawn_players(
    mut commands: Commands,
    config: Res<StartupNetworkConfig>,
    pool: Res<IoTaskPool>,
    materials: Res<Materials>,
) {
    let socket = UdpManager::bind(pool.deref().deref().clone(), config.bind).unwrap();
    let peer = socket.connect(UdpConnectionConfig::unbounded(config.remote));
    let remote = backroll::Player::Remote(peer);

    commands.insert_resource(socket);

    let mut builder = backroll::P2PSession::<BackrollConfig>::build();

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.player_material.clone(),
            sprite: Sprite::new(Vec2::new(10.0, 10.0)),
            ..Default::default()
        })
        // make sure to clone the player handles for reference stuff
        .insert(if config.client == 0 {
            // set up local player
            Player {
                handle: builder.add_player(backroll::Player::Local),
            }
        } else {
            // set up remote player
            Player {
                // make sure to clone the remote peer for reference stuff
                handle: builder.add_player(remote.clone()),
            }
        });

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.player_material.clone(),
            sprite: Sprite::new(Vec2::new(10.0, 10.0)),
            ..Default::default()
        })
        .insert(if config.client == 1 {
            // set up local player
            Player {
                handle: builder.add_player(backroll::Player::Local),
            }
        } else {
            // set up remote player
            Player {
                handle: builder.add_player(remote),
            }
        });

    commands.start_backroll_session(builder.start(pool.deref().deref().clone()).unwrap());
}

fn player_movement(
    keyboard_input: Res<GameInput<PlayerInputFrame>>,
    mut player_positions: Query<(&mut Transform, &Player)>,
) {
    for (mut transform, player) in player_positions.iter_mut() {
        let input = keyboard_input.get(player.handle).unwrap();
        if input.contains(PlayerInputFrame::LEFT) {
            transform.translation.x -= 2.;
        }
        if input.contains(PlayerInputFrame::RIGHT) {
            transform.translation.x += 2.;
        }
        if input.contains(PlayerInputFrame::DOWN) {
            transform.translation.y -= 2.;
        }
        if input.contains(PlayerInputFrame::UP) {
            transform.translation.y += 2.;
        }
    }
}

fn start_app(player_num: usize) {
    let bind_addr = if player_num == 0 {
        "127.0.0.1:4001".parse().unwrap()
    } else {
        "127.0.0.1:4002".parse().unwrap()
    };

    let remote_addr = if player_num == 0 {
        "127.0.0.1:4002".parse().unwrap()
    } else {
        "127.0.0.1:4001".parse().unwrap()
    };

    App::build()
        .add_startup_system(setup_game.system())
        .add_startup_stage("game_setup", SystemStage::single(spawn_players.system()))
        .add_plugins(DefaultPlugins)
        .add_plugin(OurBackrollPlugin)
        .insert_resource(StartupNetworkConfig {
            client: player_num,
            bind: bind_addr,
            remote: remote_addr,
        })
        .with_rollback_system::<BackrollConfig, _>(player_movement.system())
        .run();
}
fn main() {
    let mut args = std::env::args();
    let base = args.next().unwrap();
    if let Some(player_num) = args.next() {
        start_app(player_num.parse().unwrap());
    } else {
        let mut child_1 = std::process::Command::new(base.clone())
            .args(&["0"])
            .spawn()
            .unwrap();
        let mut child_2 = std::process::Command::new(base)
            .args(&["1"])
            .spawn()
            .unwrap();
        child_1.wait().unwrap();
        child_2.wait().unwrap();
    }
}
