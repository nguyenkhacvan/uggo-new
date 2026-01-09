#![deny(clippy::pedantic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::struct_excessive_bools)]

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use ratatui::crossterm::{
    ExecutableCommand,
    event::{self, Event},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;

#[cfg(debug_assertions)]
use std::time::Instant;

mod components;
mod context;
mod events;
mod transpose;
mod ui;
mod util;

use context::AppContext;

const HIDE_TARGETS: [&str; 14] = [ // Updated count
    "mio::poll",
    "rustls::client::client_conn",
    "rustls::client::hs",
    "rustls::client::tls13",
    "rustls::conn",
    "rustls::webpki::server_verifier",
    "ureq::pool", // Keeping specifically if ureq is still deeply nested somewhere
    "reqwest::connect", // New hide targets for reqwest
    "hyper::proto",
    "ureq::tls::native_tls",
    "ureq::tls::rustls",
    "ureq::unversioned::resolver",
    "ureq::unversioned::transport::tcp",
    "ureq_proto::client::flow",
];

// Chuyển sang Tokio Main Macro
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tui_logger::init_logger(log::LevelFilter::Trace)?;
    tui_logger::set_default_level(log::LevelFilter::Trace);
    for target in HIDE_TARGETS {
        tui_logger::set_level_for_target(target, log::LevelFilter::Error);
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // AppContext::new() có thể tốn thời gian, nhưng hiện tại giữ nguyên sync
    // Trong tương lai hãy refactor AppContext::new thành async fn new()
    let mut app_context = AppContext::new()?;
    let mut should_quit = false;

    // Tick rate để UI mượt mà (~60fps)
    let tick_rate = Duration::from_millis(16);
    let mut last_tick = std::time::Instant::now();

    while !should_quit {
        #[cfg(debug_assertions)]
        let start_render = Instant::now();

        terminal.draw(|frame| ui::render(frame, &app_context))?;

        #[cfg(debug_assertions)]
        app_context.set_render_duration(start_render.elapsed());

        // Event loop xử lý input
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
             // Đây là điểm chờ async events trong tương lai (Channels)
             should_quit = events::handle_events(&mut app_context)?;
        }
        
        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}