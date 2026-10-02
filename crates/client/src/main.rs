#![allow(
    clippy::too_many_lines,
    reason = "Leptos view! markup counts toward function length"
)]

mod api;
mod layout;
mod mp3s;
mod playlists;
mod sources;
mod state;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(layout::App);
}
