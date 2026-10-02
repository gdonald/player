use gloo_timers::future::TimeoutFuture;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::{names, paths, selection, sort};
use player_types::{
    FieldErrors, FlexId, Mp3, Mp3Params, Mp3Response, Mp3sResponse, PlaylistListItem,
    PlaylistMp3Attributes, PlaylistParams, PlaylistResponse, PlaylistsResponse, wrap,
};
use serde_json::Value;

use crate::api::{self, errors_for};
use crate::state::{Ctx, Page, ctx};

const SEARCH_DEBOUNCE_MILLISECONDS: u32 = 500;

fn attributes(mp3_ids: &[i64]) -> Vec<PlaylistMp3Attributes> {
    mp3_ids
        .iter()
        .map(|id| PlaylistMp3Attributes {
            mp3_id: FlexId(Some(*id)),
        })
        .collect()
}

fn load_playlists(ctx: Ctx, playlists: RwSignal<Vec<PlaylistListItem>>) {
    spawn_local(async move {
        match api::get::<PlaylistsResponse>("/api/playlists").await {
            Ok(body) => playlists.set(body.playlists),
            Err(error) => ctx.fail(&error),
        }
    });
}

#[component]
pub fn Mp3s() -> impl IntoView {
    let ctx = ctx();
    let mp3s = RwSignal::new(Vec::<Mp3>::new());
    let playlists = RwSignal::new(Vec::<PlaylistListItem>::new());
    let sort_by = RwSignal::new(None::<String>);
    let selected = RwSignal::new(Vec::<i64>::new());
    let open_menu = RwSignal::new(None::<i64>);
    let search_text = RwSignal::new(ctx.query.get_untracked().unwrap_or_default());
    let typing = StoredValue::new(0_u64);

    let load = move || {
        let url = paths::mp3s(
            ctx.query.get_untracked().as_deref(),
            sort_by.get_untracked().as_deref(),
        );

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::get::<Mp3sResponse>(&url).await {
                Ok(body) => {
                    selected.set(Vec::new());
                    mp3s.set(body.mp3s);
                }
                Err(error) => ctx.fail(&error),
            }
        });
    };

    let search = move |query: String| {
        search_text.set(query.clone());
        ctx.query.set(Some(query));
        load();
    };

    load();
    load_playlists(ctx, playlists);

    let close_menus = window_event_listener(leptos::ev::click, move |_| open_menu.set(None));
    on_cleanup(move || close_menus.remove());

    let on_type = move |event| {
        let text = event_target_value(&event);
        search_text.set(text.clone());

        let attempt = typing.get_value() + 1;
        typing.set_value(attempt);

        spawn_local(async move {
            TimeoutFuture::new(SEARCH_DEBOUNCE_MILLISECONDS).await;
            if typing.try_get_value() == Some(attempt) {
                ctx.query.set(Some(text));
                load();
            }
        });
    };

    let clear = move |event: MouseEvent| {
        event.prevent_default();
        search_text.set(String::new());
        ctx.query.set(None);
        load();
    };

    let sort_header = move |column: &'static str, label: &'static str| {
        view! {
            <a
                href="#"
                class=format!("sort-{column}")
                on:mousedown=move |event: MouseEvent| {
                    event.prevent_default();
                    sort_by.set(Some(sort::toggle(sort_by.get_untracked().as_deref(), column)));
                    load();
                }
                on:click=|event: MouseEvent| event.prevent_default()
            >
                {label}
            </a>
        }
    };

    let visible_ids = move || mp3s.with(|list| list.iter().map(|mp3| mp3.id).collect::<Vec<_>>());
    let search = Callback::new(search);

    view! {
        <div class="container-fluid">
            <nav aria-label="breadcrumb">
                <ol class="breadcrumb">
                    <li class="breadcrumb-item" aria-current="page">
                        <b>
                            <i class="bi-music-note-beamed"></i>
                            " MP3s"
                        </b>
                    </li>
                </ol>
            </nav>
            <div class="search">
                <form on:submit=|event| event.prevent_default() class="px-0">
                    <div class="input-group mb-0">
                        <input
                            class="form-control"
                            id="search"
                            placeholder="Search"
                            prop:value=move || search_text.get()
                            on:input=on_type
                        />
                        <button
                            class="btn btn-primary"
                            type="button"
                            id="button-addon-search"
                            on:click=clear
                        >
                            "Clear Search"
                        </button>
                    </div>
                </form>
            </div>
            <Show when=move || !selected.with(Vec::is_empty)>
                <PlaylistsForm selected=selected playlists=playlists />
            </Show>
            <div class="list mt-2">
                <Show
                    when=move || !mp3s.with(Vec::is_empty)
                    fallback=|| view! { <p class="text-center pt-5">"No mp3s found."</p> }
                >
                    <table class="table table-striped table-hover" id="mp3s">
                        <thead>
                            <tr>
                                <th class="tight">
                                    <input
                                        type="checkbox"
                                        id="select-all"
                                        prop:checked=move || {
                                            selected.with(|ids| selection::all_selected(ids, &visible_ids()))
                                        }
                                        on:change=move |event| {
                                            selected.set(selection::select_all(event_target_checked(&event), &visible_ids()));
                                        }
                                    />
                                </th>
                                <th>{sort_header("title", "Title")}</th>
                                <th>{sort_header("album", "Album")}</th>
                                <th class="text-center">{sort_header("track", "Track")}</th>
                                <th>{sort_header("artist", "Artist")}</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                mp3s.get()
                                    .into_iter()
                                    .map(|mp3| {
                                        view! {
                                            <Mp3Row
                                                mp3=mp3
                                                selected=selected
                                                playlists=playlists
                                                open_menu=open_menu
                                                search=search
                                            />
                                        }
                                    })
                                    .collect_view()
                            }}
                        </tbody>
                    </table>
                </Show>
            </div>
        </div>
    }
}

#[component]
fn Mp3Row(
    mp3: Mp3,
    selected: RwSignal<Vec<i64>>,
    playlists: RwSignal<Vec<PlaylistListItem>>,
    open_menu: RwSignal<Option<i64>>,
    search: Callback<String>,
) -> impl IntoView {
    let ctx = ctx();

    let add_to_playlist = move |playlist_id: i64, mp3_id: i64| {
        let body = wrap(&PlaylistParams {
            name: None,
            playlist_mp3s_attributes: Some(attributes(&[mp3_id])),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            let path = format!("/api/playlists/{playlist_id}");
            match api::put::<PlaylistResponse>(&path, &body).await {
                Ok(response) => ctx.message.set(response.message.unwrap_or_default()),
                Err(error) => ctx.fail(&error),
            }
        });
    };

    let id = mp3.id;
    let album = mp3.album_name.clone();
    let artist = mp3.artist_name.clone();

    view! {
        <tr class="align-middle" id=format!("mp3-{id}")>
            <td>
                <input
                    type="checkbox"
                    class="select-mp3"
                    prop:checked=move || selected.with(|ids| ids.contains(&id))
                    on:change=move |_| selected.update(|ids| *ids = selection::toggle(ids, id))
                />
            </td>
            <td>
                <a
                    href="#"
                    class="play-mp3"
                    on:click=move |event: MouseEvent| {
                        event.prevent_default();
                        ctx.enqueue_mp3(id);
                    }
                >
                    {mp3.title}
                </a>
            </td>
            <td>
                <a
                    href="#"
                    class="search-album"
                    on:click=move |event: MouseEvent| {
                        event.prevent_default();
                        search.run(paths::filter_query("album", &album));
                    }
                >
                    {mp3.album_name}
                </a>
            </td>
            <td class="text-center">{mp3.track}</td>
            <td>
                <a
                    href="#"
                    class="search-artist"
                    on:click=move |event: MouseEvent| {
                        event.prevent_default();
                        search.run(paths::filter_query("artist", &artist));
                    }
                >
                    {mp3.artist_name}
                </a>
            </td>
            <td class="tight">
                <div class="btn-group" role="group" aria-label="Mp3 actions">
                    <Show when=move || !playlists.with(Vec::is_empty)>
                        <div class="btn-group" role="group">
                            <button
                                type="button"
                                class=move || {
                                    if open_menu.get() == Some(id) {
                                        "btn btn-sm btn-primary dropdown-toggle show"
                                    } else {
                                        "btn btn-sm btn-primary dropdown-toggle"
                                    }
                                }
                                aria-expanded=move || (open_menu.get() == Some(id)).to_string()
                                on:click=move |event: MouseEvent| {
                                    event.stop_propagation();
                                    open_menu.update(|open| {
                                        *open = if *open == Some(id) { None } else { Some(id) };
                                    });
                                }
                            >
                                "Add to Playlist"
                            </button>
                            <ul
                                class=move || {
                                    if open_menu.get() == Some(id) { "dropdown-menu show" } else { "dropdown-menu" }
                                }
                                data-bs-popper="static"
                            >
                                {move || {
                                    playlists
                                        .get()
                                        .into_iter()
                                        .map(|playlist| {
                                            let playlist_id = playlist.id;
                                            view! {
                                                <li>
                                                    <a
                                                        class="dropdown-item"
                                                        href="#"
                                                        on:click=move |event: MouseEvent| {
                                                            event.prevent_default();
                                                            add_to_playlist(playlist_id, id);
                                                        }
                                                    >
                                                        {playlist.name}
                                                    </a>
                                                </li>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </ul>
                        </div>
                    </Show>
                    <button
                        type="button"
                        class="btn btn-sm btn-primary edit-mp3"
                        on:click=move |event: MouseEvent| {
                            event.prevent_default();
                            ctx.page.set(Page::Mp3(id));
                        }
                    >
                        "Edit"
                    </button>
                </div>
            </td>
        </tr>
    }
}

#[component]
fn PlaylistsForm(
    selected: RwSignal<Vec<i64>>,
    playlists: RwSignal<Vec<PlaylistListItem>>,
) -> impl IntoView {
    let ctx = ctx();
    let creating = RwSignal::new(false);
    let chosen = RwSignal::new("0".to_string());
    let name = RwSignal::new(String::new());
    let errors = RwSignal::new(None::<FieldErrors>);

    let finish = move |result: Result<PlaylistResponse, api::ApiError>| match result {
        Ok(response) => {
            errors.set(None);
            ctx.message.set(response.message.unwrap_or_default());
            ctx.page.set(Page::Playlist(response.playlist.id));
        }
        Err(error) => errors.set(ctx.invalid(&error)),
    };

    let add = move |event: MouseEvent| {
        event.prevent_default();

        let Ok(playlist_id) = chosen.get_untracked().parse::<i64>() else {
            return;
        };
        if playlist_id == 0 {
            return;
        }

        let body = wrap(&PlaylistParams {
            name: None,
            playlist_mp3s_attributes: Some(attributes(&selected.get_untracked())),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            finish(api::put(&format!("/api/playlists/{playlist_id}"), &body).await);
        });
    };

    let create = move |event: MouseEvent| {
        event.prevent_default();

        let body = wrap(&PlaylistParams {
            name: Some(name.get_untracked()),
            playlist_mp3s_attributes: Some(attributes(&selected.get_untracked())),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            finish(api::post("/api/playlists", &body).await);
        });
    };

    view! {
        <div class="playlists pt-1">
            <form>
                <Show
                    when=move || creating.get()
                    fallback=move || {
                        view! {
                            <div class="input-group mb-0">
                                <select
                                    class="form-select"
                                    id="playlist-select"
                                    prop:value=move || chosen.get()
                                    on:change=move |event| {
                                        let value = event_target_value(&event);
                                        if value == "new" {
                                            name.set(names::playlist_name_from_query(ctx.query.get_untracked().as_deref()));
                                            creating.set(true);
                                        }
                                        chosen.set(value);
                                    }
                                >
                                    <option value="0">"Select Playlist"</option>
                                    <option value="new">"** Create New Playlist **"</option>
                                    {move || {
                                        playlists
                                            .get()
                                            .into_iter()
                                            .map(|playlist| view! { <option value=playlist.id.to_string()>{playlist.name}</option> })
                                            .collect_view()
                                    }}
                                </select>
                                <button type="submit" class="btn btn-primary" id="add-selected" on:click=add>
                                    "Add to Playlist"
                                </button>
                            </div>
                        }
                    }
                >
                    <div class="input-group mb-0">
                        <input
                            type="text"
                            class="form-control"
                            id="new-playlist-name"
                            placeholder="Playlist Name"
                            prop:value=move || name.get()
                            on:input=move |event| name.set(event_target_value(&event))
                        />
                        <button
                            type="submit"
                            class="btn btn-primary"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                chosen.set("0".to_string());
                                creating.set(false);
                            }
                        >
                            "Cancel"
                        </button>
                        <button type="submit" class="btn btn-primary" id="create-playlist" on:click=create>
                            "Create New Playlist"
                        </button>
                        <div class="error">{move || errors.with(|errors| errors_for(errors.as_ref(), "name"))}</div>
                    </div>
                </Show>
            </form>
        </div>
    }
}

fn track_value(text: &str) -> Option<Value> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(Value::String(trimmed.to_string()))
    }
}

#[component]
pub fn Mp3Edit(id: i64) -> impl IntoView {
    let ctx = ctx();
    let loaded = RwSignal::new(None::<Mp3>);
    let title = RwSignal::new(String::new());
    let artist = RwSignal::new(String::new());
    let album = RwSignal::new(String::new());
    let track = RwSignal::new(String::new());
    let errors = RwSignal::new(None::<FieldErrors>);

    let fill = move |mp3: Mp3| {
        title.set(mp3.title.clone());
        artist.set(mp3.artist_name.clone());
        album.set(mp3.album_name.clone());
        track.set(
            mp3.track
                .map(|number| number.to_string())
                .unwrap_or_default(),
        );
        loaded.set(Some(mp3));
    };

    spawn_local(async move {
        let _waiting = ctx.wait();
        match api::get::<Mp3Response>(&format!("/api/mp3s/{id}")).await {
            Ok(body) => fill(body.mp3),
            Err(error) => ctx.fail(&error),
        }
    });

    let save = move |event: MouseEvent| {
        event.prevent_default();

        let body = wrap(&Mp3Params {
            title: Some(title.get_untracked()),
            artist: Some(artist.get_untracked()),
            album: Some(album.get_untracked()),
            track: track_value(&track.get_untracked()),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::put::<Mp3Response>(&format!("/api/mp3s/{id}"), &body).await {
                Ok(response) => {
                    errors.set(None);
                    ctx.message
                        .set(response.message.clone().unwrap_or_default());
                    fill(response.mp3);
                }
                Err(error) => errors.set(ctx.invalid(&error)),
            }
        });
    };

    let back = move |event: MouseEvent| {
        event.prevent_default();
        ctx.show(Page::Mp3s);
    };

    let field = move |field_id: &'static str,
                      label: &'static str,
                      value: RwSignal<String>,
                      error_key: &'static str| {
        view! {
            <div class="mb-3">
                <label for=field_id class="form-label">{label}</label>
                <input
                    id=field_id
                    type="text"
                    class="form-control"
                    prop:value=move || value.get()
                    on:input=move |event| value.set(event_target_value(&event))
                />
                <div class="error">{move || errors.with(|errors| errors_for(errors.as_ref(), error_key))}</div>
            </div>
        }
    };

    view! {
        <Show when=move || loaded.with(Option::is_some)>
            <div class="container-fluid">
                <nav aria-label="breadcrumb">
                    <ol class="breadcrumb">
                        <li class="breadcrumb-item">
                            <a href="#" on:click=back>"Mp3s"</a>
                        </li>
                        <li class="breadcrumb-item" aria-current="page">
                            {move || loaded.with(|mp3| mp3.as_ref().map(|mp3| mp3.title.clone()))}
                        </li>
                    </ol>
                </nav>
                <form class="form">
                    {field("title", "Title", title, "title")}
                    {field("artist_name", "Artist", artist, "artist_name")}
                    {field("album_name", "Album", album, "album_name")}
                    {field("track", "Track", track, "track")}
                    <div class="btn-group" role="group" aria-label="Save MP3">
                        <button type="submit" class="btn btn-primary" id="save" on:click=save>
                            "Save"
                        </button>
                        <button type="submit" class="btn btn-primary" id="go-back" on:click=back>
                            "Go Back"
                        </button>
                    </div>
                </form>
            </div>
        </Show>
    }
}
