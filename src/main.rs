mod auth;
mod auth_handler;
mod global_handler;
mod settings;
mod twitch_api;
mod actions;

use openaction::{OpenActionResult, register_action, run};
use openaction::global_events::set_global_event_handler;
use global_handler::TwitchGlobalHandler;
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode, ColorChoice};

use crate::actions::{
    chat_message::ChatMessageAction,
    setup::SetupAction,
    clear_chat::ClearChatAction,
    create_clip::CreateClipAction,
    create_marker::CreateMarkerAction,
    emote_chat::EmoteChatAction,
    follower_chat::FollowerChatAction,
    play_ad::PlayAdAction,
    shield_mode::ShieldModeAction,
    slow_chat::SlowChatAction,
    sub_chat::SubChatAction,
    viewers::ViewersAction,
};

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    // Initialize logger (ignore errors if terminal not available)
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );

    // Set global event handler (requires 'static lifetime)
    set_global_event_handler(Box::leak(Box::new(TwitchGlobalHandler)));

    // Register all actions
    register_action(SetupAction).await;
    register_action(ChatMessageAction).await;
    register_action(ClearChatAction).await;
    register_action(CreateClipAction).await;
    register_action(CreateMarkerAction).await;
    register_action(EmoteChatAction).await;
    register_action(FollowerChatAction).await;
    register_action(PlayAdAction).await;
    register_action(ShieldModeAction).await;
    register_action(SlowChatAction).await;
    register_action(SubChatAction).await;
    register_action(ViewersAction).await;

    // Run the plugin event loop
    run(std::env::args().collect()).await
}
