use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::{queue, title};
use player_types::CountsResponse;

use crate::api;
use crate::mp3s::{Mp3Edit, Mp3s};
use crate::playlists::{PlaylistEdit, Playlists};
use crate::sources::{SourceEdit, Sources};
use crate::state::{Ctx, Page, ctx};

#[component]
pub fn App() -> impl IntoView {
    let ctx = Ctx::new();
    provide_context(ctx);

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
            <div class="row mt-5">
                <div class="col-5"></div>
                <div class="col-2">
                    <form on:submit=submit>
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
                <div class="col-5"></div>
            </div>
        </div>
    }
}

#[component]
fn Layout() -> impl IntoView {
    let ctx = ctx();

    view! {
        <div class="container-fluid vh-100">
            <div class="row">
                <div class="col-2">
                    <div class="row border-bottom border-dark">
                        <div class="menu-items">
                            <Wait />
                            <Menu />
                        </div>
                    </div>
                    <div class="row">
                        <div class="queue-wrapper">
                            <div class="queue-content">
                                <Queue />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="col-10 min-vh-100 border-start border-dark">
                    <div class="main-wrapper">
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
                    <div class="row audio-content border-top border-dark">
                        <div class="text-center pt-3">
                            <Audio />
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
    let counts = RwSignal::new(None::<CountsResponse>);

    spawn_local(async move {
        match api::get::<CountsResponse>("/api/counts").await {
            Ok(body) => counts.set(Some(body)),
            Err(error) => ctx.fail(&error),
        }
    });

    let item = move |section: Section| {
        let active = move || section.contains(ctx.page.get());

        view! {
            <li
                class=move || {
                    if active() {
                        "list-group-item list-group-item-action active"
                    } else {
                        "list-group-item list-group-item-action"
                    }
                }
                on:click=move |event: MouseEvent| {
                    event.prevent_default();
                    ctx.show(section.page());
                }
            >
                <i class=move || {
                    let icon = section.icon();
                    if active() { format!("fs-5 bi-{icon} text-white") } else { format!("fs-5 bi-{icon}") }
                }></i>
                {format!(" \u{a0}{}", section.label())}
                <span class="badge bg-dark text-light float-end mt-1">
                    {move || counts.get().map_or(0, |counts| section.count(&counts))}
                </span>
            </li>
        }
    };

    view! {
        <ul class="list-group rounded-0">
            {item(Section::Mp3s)}
            {item(Section::Playlists)}
            {item(Section::Sources)}
        </ul>
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

    view! {
        <div>
            <Show when=shows_play>
                <div class="text-end pe-1 queue-play">
                    <a
                        href="#"
                        id="queue-play"
                        on:click=move |event: MouseEvent| {
                            event.prevent_default();
                            ctx.start_if_idle();
                        }
                    >
                        <i class="bi-play-btn text-primary"></i>
                    </a>
                </div>
            </Show>
            <div>
                <table class="table table-striped table-hover mb-0" id="queue">
                    <tbody>
                        {move || {
                            ctx.queue
                                .get()
                                .into_iter()
                                .map(|entry| {
                                    let id = entry.id;
                                    let is_current = move || current_id() == Some(id);
                                    view! {
                                        <tr class=move || {
                                            if is_current() { "align-middle table-primary" } else { "align-middle" }
                                        }>
                                            <td class="tight queue-delete-icon">
                                                <a
                                                    href="#"
                                                    class="queue-delete"
                                                    on:click=move |event: MouseEvent| {
                                                        event.prevent_default();
                                                        ctx.remove(id);
                                                    }
                                                >
                                                    <i class="bi-trash text-primary"></i>
                                                </a>
                                            </td>
                                            <td class="queue-title">{entry.mp3.title}</td>
                                            <td class="tight queue-current-icon">
                                                <Show when=is_current>
                                                    <i class="bi-arrow-left-circle-fill text-primary"></i>
                                                </Show>
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
fn Audio() -> impl IntoView {
    let ctx = ctx();

    view! {
        <div>
            <Show when=move || ctx.src.get().is_some()>
                <audio
                    controls=true
                    autoplay=true
                    preload="auto"
                    src=move || ctx.src.get().unwrap_or_default()
                    on:ended=move |_| ctx.advance()
                >
                    "Your browser does not support the "
                    <code>"audio"</code>
                    " element."
                </audio>
            </Show>
        </div>
    }
}
