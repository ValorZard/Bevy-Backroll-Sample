use bevy::{core::FixedTimestep, prelude::*};
use bevy_backroll::*;
use bytemuck::{Zeroable, Pod};

pub type P2PSession = bevy_backroll::backroll::P2PSession<BackrollConfig>;

pub struct BackrollConfig;

#[macro_use]
extern crate bitflags;

#[derive(Clone)]
pub struct Player {
    position: Vec2,
    velocity: Vec2,
    size: Vec2,
    handle: backroll::PlayerHandle, // the network id 
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

impl backroll::Config for BackrollConfig {
    type Input = PlayerInputFrame;
    type State = GameState;
}

#[derive(Clone, PartialEq, Hash)]
pub struct GameState {}

const MATCH_UPDATE_LABEL: &str = "MATCH_UPDATE";

const DELTA_TIME: f32 = 1000.0/60.0; // in ms

pub struct FcBackrollPlugin;

impl Plugin for FcBackrollPlugin {
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

fn sample_input(
    handle: In<backroll::PlayerHandle>,
    keyboard_input: Res<Input<KeyCode>>,
) -> PlayerInputFrame {
    let mut local_input = PlayerInputFrame::empty();

    // local input handling
    {
        if keyboard_input.pressed(KeyCode::Left)  {
            local_input.insert(PlayerInputFrame::LEFT);
        } else if keyboard_input.pressed(KeyCode::Right) {
            local_input.insert(PlayerInputFrame::RIGHT);
        }

        if keyboard_input.pressed(KeyCode::Up)  {
            local_input.insert(PlayerInputFrame::UP);
        } else if keyboard_input.pressed(KeyCode::Up) {
            local_input.insert(PlayerInputFrame::DOWN);
        }
    }

    local_input
}

fn save_world() -> GameState {
    GameState {}
}

fn load_world(state: In<GameState>) {}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(BackrollPlugin::<BackrollConfig>::default())
        .run();
}