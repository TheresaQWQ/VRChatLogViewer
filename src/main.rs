#![windows_subsystem = "windows"]

pub mod config;
pub mod log_entry;
pub mod message;
pub mod state;
pub mod stream;
pub mod view;
pub mod vrc;

use iced::application;
use state::VRCLog;

fn main() -> iced::Result {
    application(VRCLog::default, VRCLog::update, VRCLog::view)
        .subscription(VRCLog::subscription)
        .title("VRC Log Viewer")
        .run()
}
