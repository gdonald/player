#![allow(
    clippy::too_many_lines,
    reason = "Leptos view! markup counts toward function length"
)]

mod api;
mod audio_graph;
mod confirm;
mod equalizer;
mod layout;
mod mp3s;
mod player;
mod playlists;
mod settings;
mod sources;
mod state;
mod storage;
mod theme;
mod visualizer;

/// The coverage counters as a `.profraw` file, which the browser tests save
/// before each navigation and at the end of each test.
#[cfg(feature = "coverage")]
#[allow(
    unsafe_code,
    reason = "minicov reads the counters; the page runs on one thread"
)]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn player_coverage() -> Vec<u8> {
    let mut coverage = Vec::new();
    unsafe { minicov::capture_coverage(&mut coverage) }.ok();

    coverage
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(layout::App);
}
