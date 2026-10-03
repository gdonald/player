use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::paths;
use player_types::{
    FieldErrors, MessageResponse, PlaylistListItem, PlaylistMp3, PlaylistMp3sResponse,
    PlaylistParams, PlaylistResponse, PlaylistSummary, PlaylistsResponse, wrap,
};
use serde_json::json;

use crate::api::{self, errors_for};
use crate::state::{Page, ctx};

#[component]
pub fn Playlists() -> impl IntoView {
    let ctx = ctx();
    let playlists = RwSignal::new(Vec::<PlaylistListItem>::new());

    let load = move || {
        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::get::<PlaylistsResponse>("/api/playlists").await {
                Ok(body) => playlists.set(body.playlists),
                Err(error) => ctx.fail(&error),
            }
        });
    };

    load();

    let delete = move |id: i64| {
        if !window()
            .confirm_with_message("Are you sure?")
            .unwrap_or(false)
        {
            return;
        }

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::delete::<MessageResponse>(&format!("/api/playlists/{id}")).await {
                Ok(body) => {
                    ctx.message.set(body.message);
                    load();
                }
                Err(error) => ctx.fail(&error),
            }
        });
    };

    let row = move |playlist: PlaylistListItem| {
        let id = playlist.id;

        view! {
            <tr class="align-middle" id=format!("playlist-{id}")>
                <td>{playlist.name}</td>
                <td class="text-center">{playlist.mp3s_count}</td>
                <td class="tight">
                    <div class="btn-group" role="group" aria-label="Playlist actions">
                        <Show when=move || { playlist.mp3s_count > 0 }>
                            <button
                                class="btn btn-sm btn-primary enqueue-playlist"
                                on:click=move |event: MouseEvent| {
                                    event.prevent_default();
                                    ctx.enqueue_playlist(id);
                                }
                            >
                                "Enqueue"
                            </button>
                        </Show>
                        <button
                            class="btn btn-sm btn-primary edit-playlist"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                ctx.page.set(Page::Playlist(id));
                            }
                        >
                            "Edit"
                        </button>
                        <button
                            class="btn btn-sm btn-primary delete-playlist"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                delete(id);
                            }
                        >
                            "Delete"
                        </button>
                    </div>
                </td>
            </tr>
        }
    };

    view! {
        <div class="container-fluid">
            <nav aria-label="breadcrumb">
                <ol class="breadcrumb">
                    <li class="breadcrumb-item">
                        <b>
                            <i class="bi-card-list"></i>
                            " Playlists"
                        </b>
                    </li>
                </ol>
            </nav>
            <Show
                when=move || !playlists.with(Vec::is_empty)
                fallback=|| view! { <p class="text-center pt-5">"No playlists found."</p> }
            >
                <table class="table table-striped table-hover" id="playlists">
                    <thead>
                        <tr>
                            <th>"Name"</th>
                            <th class="text-center">"MP3s"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>{move || playlists.get().into_iter().map(row).collect_view()}</tbody>
                </table>
            </Show>
        </div>
    }
}

#[component]
pub fn PlaylistEdit(id: i64) -> impl IntoView {
    let ctx = ctx();
    let playlist = RwSignal::new(None::<PlaylistSummary>);
    let name = RwSignal::new(String::new());
    let errors = RwSignal::new(None::<FieldErrors>);
    let entries = RwSignal::new(Vec::<PlaylistMp3>::new());

    spawn_local(async move {
        let _waiting = ctx.wait();
        match api::get::<PlaylistResponse>(&format!("/api/playlists/{id}")).await {
            Ok(body) => {
                name.set(body.playlist.name.clone());
                playlist.set(Some(body.playlist));
            }
            Err(error) => ctx.fail(&error),
        }
    });

    let save = move |event: MouseEvent| {
        event.prevent_default();

        let body = wrap(&PlaylistParams {
            name: Some(name.get_untracked()),
            playlist_mp3s_attributes: None,
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::put::<PlaylistResponse>(&format!("/api/playlists/{id}"), &body).await {
                Ok(response) => {
                    errors.set(None);
                    ctx.message
                        .set(response.message.clone().unwrap_or_default());
                    playlist.set(Some(response.playlist));
                }
                Err(error) => errors.set(ctx.invalid(&error)),
            }
        });
    };

    let back = move |event: MouseEvent| {
        event.prevent_default();
        ctx.show(Page::Playlists);
    };

    view! {
        <Show when=move || playlist.with(Option::is_some)>
            <div class="container-fluid">
                <nav aria-label="breadcrumb">
                    <ol class="breadcrumb">
                        <li class="breadcrumb-item">
                            <a href="#" on:click=back class="text-decoration-none">
                                <b>
                                    <i class="bi-card-list"></i>
                                    " Playlists"
                                </b>
                            </a>
                        </li>
                        <li class="breadcrumb-item" aria-current="page">
                            <b>{move || playlist.with(|playlist| playlist.as_ref().map(|playlist| playlist.name.clone()))}</b>
                        </li>
                    </ol>
                </nav>
                <form class="form">
                    <div class="mb-3">
                        <label for="name" class="form-label">
                            <b>"Name"</b>
                        </label>
                        <input
                            id="name"
                            type="text"
                            class="form-control"
                            prop:value=move || name.get()
                            on:input=move |event| name.set(event_target_value(&event))
                        />
                        <div class="error">{move || errors.with(|errors| errors_for(errors.as_ref(), "name"))}</div>
                    </div>
                    <button type="submit" class="btn btn-primary" id="save" on:click=save>
                        "Save"
                    </button>
                    " "
                    <Show when=move || !entries.with(Vec::is_empty)>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            id="enqueue"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                ctx.enqueue_playlist(id);
                            }
                        >
                            "Enqueue"
                        </button>
                    </Show>
                </form>
                <PlaylistMp3s id=id entries=entries />
            </div>
        </Show>
    }
}

#[component]
fn PlaylistMp3s(id: i64, entries: RwSignal<Vec<PlaylistMp3>>) -> impl IntoView {
    let ctx = ctx();

    let apply = move |result: Result<PlaylistMp3sResponse, api::ApiError>| match result {
        Ok(body) => entries.set(body.playlist_mp3s),
        Err(error) => ctx.fail(&error),
    };

    spawn_local(async move {
        let _waiting = ctx.wait();
        apply(api::get(&format!("/api/playlists/{id}/playlist_mp3s")).await);
    });

    let act = move |entry_id: i64, action: &'static str| {
        spawn_local(async move {
            let _waiting = ctx.wait();
            let path = format!("/api/playlists/{id}/playlist_mp3s/{entry_id}");
            let result = if action == "delete" {
                api::delete(&path).await
            } else {
                api::post(&format!("{path}/{action}"), &json!({})).await
            };
            apply(result);
        });
    };

    let filter_by = move |field: &'static str, value: String| {
        ctx.query.set(Some(paths::filter_query(field, &value)));
        ctx.page.set(Page::Mp3s);
    };

    let row = move |entry: PlaylistMp3| {
        let entry_id = entry.id;
        let mp3_id = entry.mp3.id;
        let album = entry.mp3.album_name.clone();
        let artist = entry.mp3.artist_name.clone();

        view! {
            <tr class="align-middle" id=format!("entry-{entry_id}")>
                <td>
                    <a
                        href="#"
                        class="play-mp3"
                        on:click=move |event: MouseEvent| {
                            event.prevent_default();
                            ctx.enqueue_mp3(mp3_id);
                        }
                    >
                        {entry.mp3.title}
                    </a>
                </td>
                <td class="col-album">
                    <a
                        href="#"
                        class="search-album"
                        on:click=move |event: MouseEvent| {
                            event.prevent_default();
                            filter_by("album", album.clone());
                        }
                    >
                        {entry.mp3.album_name}
                    </a>
                </td>
                <td class="text-center col-track">{entry.mp3.track}</td>
                <td>
                    <a
                        href="#"
                        class="search-artist"
                        on:click=move |event: MouseEvent| {
                            event.prevent_default();
                            filter_by("artist", artist.clone());
                        }
                    >
                        {entry.mp3.artist_name}
                    </a>
                </td>
                <td class="tight">
                    <div class="btn-group" role="group" aria-label="Playlist MP3 actions">
                        <div class="btn-group" role="group">
                            <button
                                type="button"
                                class=if entry.first { "btn btn-sm btn-primary move-up disabled" } else { "btn btn-sm btn-primary move-up" }
                                on:click=move |event: MouseEvent| {
                                    event.prevent_default();
                                    act(entry_id, "move_higher");
                                }
                            >
                                "Up"
                            </button>
                            <button
                                type="button"
                                class=if entry.last { "btn btn-sm btn-primary move-down disabled" } else { "btn btn-sm btn-primary move-down" }
                                on:click=move |event: MouseEvent| {
                                    event.prevent_default();
                                    act(entry_id, "move_lower");
                                }
                            >
                                "Down"
                            </button>
                            <button
                                type="button"
                                class="btn btn-sm btn-primary delete-entry"
                                on:click=move |event: MouseEvent| {
                                    event.prevent_default();
                                    act(entry_id, "delete");
                                }
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>
                </td>
            </tr>
        }
    };

    view! {
        <div class="container-fluid pt-4 px-0">
            <table class="table table-striped table-hover" id="playlist-mp3s">
                <thead>
                    <tr>
                        <th>"Title"</th>
                        <th class="col-album">"Album"</th>
                        <th class="text-center col-track">"Track"</th>
                        <th>"Artist"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>{move || entries.get().into_iter().map(row).collect_view()}</tbody>
            </table>
        </div>
    }
}
