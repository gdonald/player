#![allow(
    clippy::too_many_lines,
    reason = "Leptos view! markup counts toward function length"
)]

mod api;
mod audio_graph;
mod equalizer;
mod layout;
mod mp3s;
mod player;
mod playlists;
mod sources;
mod state;
mod storage;
mod visualizer;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(layout::App);
}
