use crate::{
    hud::overlay::OverlayEvent,
    level::{CurrentLevel, GameLevel},
    states::GameState,
};
use bevy::prelude::*;

fn on_enter_warp(mut overlay_writer: EventWriter<OverlayEvent>, next_level: Res<CurrentLevel>) {
    overlay_writer.send(OverlayEvent::Close(match next_level.next_level {
        GameLevel::Level(_) => GameState::InGame,
        GameLevel::MultiPlayArena => GameState::NameInput,
    }));
}

pub struct WarpPagePlugin;

impl Plugin for WarpPagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Warp), on_enter_warp);
    }
}
