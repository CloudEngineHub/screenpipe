mod auto_destruct;
pub mod chunking;
pub mod cli;
pub mod core;
pub mod db;
pub mod db_types;
pub mod filtering;
mod add;
pub mod pipe_manager;
mod plugin;
mod resource_monitor;
mod server;
mod video;
pub mod video_cache;
mod video_db;
pub mod video_utils;
pub mod text_embeds;

pub use auto_destruct::watch_pid;
pub use cli::Cli;
pub use core::start_continuous_recording;
pub use db::DatabaseManager;
pub use add::handle_index_command;
pub use pipe_manager::PipeManager;
pub use resource_monitor::{ResourceMonitor, RestartSignal};
pub use screenpipe_core::Language;
pub use server::create_router;
pub use server::health_check;
pub use server::AppState;
pub use server::ContentItem;
pub use server::HealthCheckResponse;
pub use server::PaginatedResponse;
pub use server::Server;
pub use video::VideoCapture;
