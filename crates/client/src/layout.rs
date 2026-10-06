use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::{playback, queue, title};
use player_types::CountsResponse;

use crate::api;
use crate::audio_graph::AudioGraph;
use crate::equalizer::Equalizer;
use crate::mp3s::{Mp3Edit, Mp3s};
use crate::player::Player;
use crate::playlists::{PlaylistEdit, Playlists};
use crate::sources::{SourceEdit, Sources};
use crate::state::{Ctx, Page, ctx};
use crate::storage;
use crate::theme::{self, ThemePicker};

const PLAYLIST_OPEN_KEY: &str = "player.playlist.open";
use crate::visualizer::Visualizer;

#[component]
pub fn App() -> impl IntoView {
    let ctx = Ctx::new();
    provide_context(ctx);
    ctx.remember_query();
    theme::apply_stored();

    spawn_local(async move {
        ctx.authenticated.set(Some(api::active().await));
    });

    Effect::new(move |_| {
        if ctx.authenticated.get() == Some(true) {
            ctx.load_queue();
        }
    });

    Effect::new(move |_| {
        let page_title = ctx.current.with(|current| {
            title::page_title(
                current
                    .as_ref()
                    .map(|entry| (entry.mp3.artist_name.as_str(), entry.mp3.title.as_str())),
            )
        });
        document().set_title(&page_title);
    });

    view! {
        <div id="root" class="vh-100">
            {move || match ctx.authenticated.get() {
                None => ().into_any(),
                Some(false) => view! { <LoginForm /> }.into_any(),
                Some(true) => view! { <Layout /> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn LoginForm() -> impl IntoView {
    let ctx = ctx();
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    let submit = move |event: SubmitEvent| {
        event.prevent_default();

        let (name, secret) = (username.get_untracked(), password.get_untracked());
        if name.is_empty() || secret.is_empty() {
            return;
        }

        spawn_local(async move {
            ctx.authenticated.set(Some(api::login(name, secret).await));
        });
    };

    view! {
        <div class="container-fluid">
            <div class="row mt-5 justify-content-center">
                <div class="col-11 col-sm-8 col-md-4 col-xl-2">
                    <div class="winamp-window login" id="login">
                        <div class="winamp-titlebar">
                            <ThemePicker />
                            <span>"LOGIN"</span>
                        </div>
                        <form class="login-body" on:submit=submit>
                            <div class="mb-3">
                                <label for="username" class="form-label">"Username"</label>
                                <input
                                    type="text"
                                    placeholder="Username"
                                    id="username"
                                    class="form-control"
                                    on:input=move |event| username.set(event_target_value(&event))
                                />
                            </div>
                            <div class="mb-3">
                                <label for="password" class="form-label">"Password"</label>
                                <input
                                    type="password"
                                    placeholder="Password"
                                    id="password"
                                    class="form-control"
                                    on:input=move |event| password.set(event_target_value(&event))
                                />
                            </div>
                            <button type="submit" class="btn btn-primary">"Login"</button>
                        </form>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn Layout() -> impl IntoView {
    let ctx = ctx();
    AudioGraph::provide();

    view! {
        <div class="container-fluid app-frame">
            <div class="row h-100">
                <div class="col-12 col-md-3 app-column app-side">
                    <div class="row border-bottom border-dark area-library">
                        <div class="menu-items">
                            <Menu />
                        </div>
                    </div>
                    <div class="row app-fill area-windows">
                        <div class="queue-wrapper">
                            <Queue />
                            <Equalizer />
                            <Visualizer />
                        </div>
                    </div>
                </div>
                <div class="col-12 col-md-9 app-column app-main border-start border-dark">
                    <div class="row audio-content border-bottom border-dark area-player">
                        <div class="h-100 p-0">
                            <Player />
                        </div>
                    </div>
                    <div class="main-wrapper app-fill area-content">
                        <div class="row main-content pt-2">
                            <div class="p-0">
                                <Alert />
                                {move || match ctx.page.get() {
                                    Page::Mp3s => view! { <Mp3s /> }.into_any(),
                                    Page::Mp3(id) => view! { <Mp3Edit id=id /> }.into_any(),
                                    Page::Playlists => view! { <Playlists /> }.into_any(),
                                    Page::Playlist(id) => view! { <PlaylistEdit id=id /> }.into_any(),
                                    Page::Sources => view! { <Sources /> }.into_any(),
                                    Page::Source(id) => view! { <SourceEdit id=id /> }.into_any(),
                                }}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[derive(Debug, Clone, Copy)]
enum Section {
    Mp3s,
    Playlists,
    Sources,
}

impl Section {
    fn contains(self, page: Page) -> bool {
        matches!(
            (self, page),
            (Section::Mp3s, Page::Mp3s | Page::Mp3(_))
                | (Section::Playlists, Page::Playlists | Page::Playlist(_))
                | (Section::Sources, Page::Sources | Page::Source(_))
        )
    }

    fn page(self) -> Page {
        match self {
            Section::Mp3s => Page::Mp3s,
            Section::Playlists => Page::Playlists,
            Section::Sources => Page::Sources,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Section::Mp3s => "MP3s",
            Section::Playlists => "Playlists",
            Section::Sources => "Sources",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::Mp3s => "music-note-beamed",
            Section::Playlists => "card-list",
            Section::Sources => "folder",
        }
    }

    fn count(self, counts: &CountsResponse) -> i64 {
        match self {
            Section::Mp3s => counts.mp3s_count,
            Section::Playlists => counts.playlists_count,
            Section::Sources => counts.sources_count,
        }
    }
}

#[component]
fn Menu() -> impl IntoView {
    let ctx = ctx();
    let counts = ctx.counts;
    ctx.load_counts();

    let item = move |section: Section| {
        let active = move || section.contains(ctx.page.get());

        view! {
            <button
                type="button"
                class=move || if active() { "library-button winamp-lit" } else { "library-button" }
                aria-pressed=move || active().to_string()
                on:click=move |event: MouseEvent| {
                    event.prevent_default();
                    ctx.show(section.page());
                }
            >
                <i class=format!("bi-{}", section.icon())></i>
                <span class="library-label">{section.label()}</span>
                <span class="library-count">
                    {move || counts.get().map_or(0, |counts| section.count(&counts))}
                </span>
            </button>
        }
    };

    view! {
        <div class="winamp-window library">
            <div class="winamp-titlebar">
                <ThemePicker />
                <span>"LIBRARY"</span>
                <Wait />
            </div>
            <div class="library-nav">
                {item(Section::Mp3s)}
                {item(Section::Playlists)}
                {item(Section::Sources)}
            </div>
        </div>
    }
}

#[component]
fn Alert() -> impl IntoView {
    let ctx = ctx();

    view! {
        <Show when=move || !ctx.message.get().is_empty()>
            <div class="px-2">
                <div class="alert alert-primary alert-dismissible fade show" role="alert">
                    {move || ctx.message.get()}
                    <button
                        type="button"
                        class="btn-close"
                        aria-label="Close"
                        on:click=move |_| ctx.message.set(String::new())
                    ></button>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn Wait() -> impl IntoView {
    let ctx = ctx();

    view! {
        <div class="wait">
            <Show when=move || { ctx.waiting.get() > 0 }>
                <div class="waiting">
                    <div></div>
                    <div></div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn Queue() -> impl IntoView {
    let ctx = ctx();
    let open =
        RwSignal::new(storage::get(PLAYLIST_OPEN_KEY).is_none_or(|stored| stored != "false"));
    Effect::new(move |_| storage::set(PLAYLIST_OPEN_KEY, &open.get().to_string()));

    let current_id = move || {
        ctx.current
            .with(|current| current.as_ref().map(|entry| entry.id))
    };
    let shows_play = move || {
        let ids: Vec<i64> = ctx
            .queue
            .with(|entries| entries.iter().map(|entry| entry.id).collect());
        queue::shows_play_button(&ids, current_id())
    };
    let mode_button = move |mode: queue::Mode, icon: &'static str, label: &'static str| {
        let selected = move || ctx.mode.get() == mode;

        view! {
            <button
                type="button"
                class=move || if selected() { "winamp-button playlist-mode winamp-lit" } else { "winamp-button playlist-mode" }
                id=format!("mode-{}", mode.as_str())
                aria-label=label
                title=label
                aria-pressed=move || selected().to_string()
                on:click=move |_: MouseEvent| ctx.mode.set(mode)
            >
                <i class=icon></i>
            </button>
        }
    };

    let total_time = move || {
        let lengths: Vec<Option<i32>> = ctx
            .queue
            .with(|entries| entries.iter().map(|entry| entry.mp3.length).collect());
        playback::format_time(playback::total_seconds(&lengths))
    };

    view! {
        <div class=move || if open.get() { "winamp-window playlist" } else { "winamp-window playlist playlist-closed" }>
            <div class="window-bar">
                <button
                    type="button"
                    class=move || if open.get() { "winamp-button winamp-toggle winamp-lit" } else { "winamp-button winamp-toggle" }
                    id="playlist-toggle"
                    aria-label="Playlist"
                    title="Show or hide the playlist"
                    aria-expanded=move || open.get().to_string()
                    on:click=move |_: MouseEvent| open.update(|open| *open = !*open)
                >
                    <span class="winamp-light"></span>
                    "PL"
                </button>
                <div class="winamp-titlebar window-bar-title">
                    <span>"PLAYLIST"</span>
                </div>
            </div>
            <Show when=move || open.get()>
            <div class="playlist-list">
                <table class="playlist-table" id="queue">
                    <tbody>
                        {move || {
                            ctx.queue
                                .get()
                                .into_iter()
                                .enumerate()
                                .map(|(index, entry)| {
                                    let id = entry.id;
                                    let chosen = entry.clone();
                                    let is_current = move || current_id() == Some(id);
                                    let length = entry
                                        .mp3
                                        .length
                                        .map(|seconds| playback::format_time(f64::from(seconds)))
                                        .unwrap_or_default();
                                    view! {
                                        <tr
                                            class=move || if is_current() { "playlist-row table-primary" } else { "playlist-row" }
                                            title="Double-click to play"
                                            on:dblclick=move |_| ctx.play_entry(chosen.clone())
                                        >
                                            <td class="playlist-name">
                                                {format!("{}. {} - ", index + 1, entry.mp3.artist_name)}
                                                <span class="queue-title">{entry.mp3.title}</span>
                                            </td>
                                            <td class="playlist-length">{length}</td>
                                            <td class="playlist-remove">
                                                <a
                                                    href="#"
                                                    class="queue-delete"
                                                    title="Remove"
                                                    aria-label="Remove"
                                                    on:click=move |event: MouseEvent| {
                                                        event.prevent_default();
                                                        event.stop_propagation();
                                                        ctx.remove(id);
                                                    }
                                                    on:dblclick=|event: MouseEvent| event.stop_propagation()
                                                >
                                                    <i class="bi-x-lg"></i>
                                                </a>
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>
            </div>
            <div class="playlist-footer">
                <div class="playlist-modes" role="group" aria-label="Play mode">
                    {mode_button(queue::Mode::Play, "bi-arrow-right", "Play through, removing each song as it finishes")}
                    {mode_button(queue::Mode::LoopOne, "bi-repeat-1", "Loop one song")}
                    {mode_button(queue::Mode::LoopAll, "bi-repeat", "Loop the playlist")}
                </div>
                <button
                    type="button"
                    class="winamp-button"
                    id="queue-clear"
                    title="Clear the playlist"
                    disabled=move || ctx.queue.with(Vec::is_empty)
                    on:click=move |event: MouseEvent| {
                        event.prevent_default();
                        ctx.clear_queue();
                    }
                >
                    "Clear"
                </button>
                <Show when=shows_play>
                    <a
                        href="#"
                        class="winamp-button"
                        id="queue-play"
                        title="Play the queue"
                        on:click=move |event: MouseEvent| {
                            event.prevent_default();
                            ctx.start_if_idle();
                        }
                    >
                        <i class="bi-play-fill"></i>
                    </a>
                </Show>
                <span class="playlist-total" id="queue-total">{total_time}</span>
            </div>
            </Show>
        </div>
    }
}
