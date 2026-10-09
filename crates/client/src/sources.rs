use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use player_types::{
    FieldErrors, MessageResponse, SourceListItem, SourceParams, SourceResponse, SourceSummary,
    SourcesResponse, wrap,
};

use crate::api::{self, errors_for};
use crate::confirm::ConfirmModal;
use crate::state::{Page, ctx};

#[component]
pub fn Sources() -> impl IntoView {
    let ctx = ctx();
    let sources = RwSignal::new(Vec::<SourceListItem>::new());

    let load = move || {
        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::get::<SourcesResponse>("/api/sources").await {
                Ok(body) => sources.set(body.sources),
                Err(error) => ctx.fail(&error),
            }
        });
    };

    load();

    let scan = move |id: i64| {
        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::touch(&format!("/api/sources/{id}/scan")).await {
                Ok(()) => ctx
                    .message
                    .set("Source scanning has been scheduled".to_string()),
                Err(api::ApiError::Unauthorized) => ctx.authenticated.set(Some(false)),
                Err(_) => ctx.message.set("Source scanning failed".to_string()),
            }
        });
    };

    let removing = RwSignal::new(None::<SourceListItem>);

    let remove = move |id: i64| {
        removing.set(None);

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::delete::<MessageResponse>(&format!("/api/sources/{id}")).await {
                Ok(body) => {
                    ctx.message.set(body.message);
                    ctx.load_counts();
                    ctx.reload_queue();
                    load();
                }
                Err(error) => ctx.fail(&error),
            }
        });
    };

    let row = move |source: SourceListItem| {
        let id = source.id;
        let chosen = source.clone();

        view! {
            <tr class="align-middle" id=format!("source-{id}")>
                <td>{source.path}</td>
                <td>{source.mp3s_count}</td>
                <td class="tight">
                    <div class="btn-group" role="group" aria-label="Source actions">
                        <button
                            class="btn btn-sm btn-primary edit-source"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                ctx.page.set(Page::Source(id));
                            }
                        >
                            "Edit"
                        </button>
                        <button
                            class="btn btn-sm btn-primary scan-source"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                scan(id);
                            }
                        >
                            "Scan"
                        </button>
                        <button
                            class="btn btn-sm btn-danger remove-source"
                            on:click=move |event: MouseEvent| {
                                event.prevent_default();
                                removing.set(Some(chosen.clone()));
                            }
                        >
                            "Remove"
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
                            <i class="bi-folder"></i>
                            " Sources"
                        </b>
                    </li>
                </ol>
            </nav>
            <NewSource on_created=Callback::new(move |()| load()) />
            <Show
                when=move || !sources.with(Vec::is_empty)
                fallback=|| view! { <p class="text-center pt-5">"No sources found."</p> }
            >
                <table class="table table-striped table-hover" id="sources">
                    <thead>
                        <tr>
                            <th>"Path"</th>
                            <th>"MP3s"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>{move || sources.get().into_iter().map(row).collect_view()}</tbody>
                </table>
            </Show>
            {move || {
                removing
                    .get()
                    .map(|source| {
                        let id = source.id;

                        view! {
                            <ConfirmModal
                                title="Remove Source"
                                confirm_label="Remove"
                                on_confirm=Callback::new(move |()| remove(id))
                                on_cancel=Callback::new(move |()| removing.set(None))
                            >
                                <p>"Remove " <b>{source.path}</b> " from the library?"</p>
                                <p class="mb-0">
                                    "Its " {source.mp3s_count}
                                    " MP3s will be removed from the library, playlists, and queue. The files are not deleted."
                                </p>
                            </ConfirmModal>
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn NewSource(on_created: Callback<()>) -> impl IntoView {
    let ctx = ctx();
    let path = RwSignal::new(String::new());
    let errors = RwSignal::new(None::<FieldErrors>);

    let add = move |event: SubmitEvent| {
        event.prevent_default();

        let body = wrap(&SourceParams {
            path: Some(path.get_untracked()),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::post::<SourceResponse>("/api/sources", &body).await {
                Ok(response) => {
                    errors.set(None);
                    path.set(String::new());
                    ctx.message.set(response.message.unwrap_or_default());
                    ctx.load_counts();
                    on_created.run(());
                }
                Err(error) => errors.set(ctx.invalid(&error)),
            }
        });
    };

    view! {
        <form class="pb-3" id="new-source" on:submit=add>
            <div class="input-group mb-0">
                <input
                    type="text"
                    class="form-control"
                    id="new-source-path"
                    placeholder="Path to a music directory"
                    prop:value=move || path.get()
                    on:input=move |event| path.set(event_target_value(&event))
                />
                <button type="submit" class="btn btn-primary" id="add-source">
                    "Add Source"
                </button>
            </div>
            <div class="error">{move || errors.with(|errors| errors_for(errors.as_ref(), "path"))}</div>
        </form>
    }
}

#[component]
pub fn SourceEdit(id: i64) -> impl IntoView {
    let ctx = ctx();
    let source = RwSignal::new(None::<SourceSummary>);
    let path = RwSignal::new(String::new());
    let errors = RwSignal::new(None::<FieldErrors>);

    spawn_local(async move {
        let _waiting = ctx.wait();
        match api::get::<SourceResponse>(&format!("/api/sources/{id}")).await {
            Ok(body) => {
                path.set(body.source.path.clone());
                source.set(Some(body.source));
            }
            Err(error) => ctx.fail(&error),
        }
    });

    let save = move |event: MouseEvent| {
        event.prevent_default();

        let body = wrap(&SourceParams {
            path: Some(path.get_untracked()),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::put::<SourceResponse>(&format!("/api/sources/{id}"), &body).await {
                Ok(response) => {
                    errors.set(None);
                    ctx.message
                        .set(response.message.clone().unwrap_or_default());
                    source.set(Some(response.source));
                }
                Err(error) => errors.set(ctx.invalid(&error)),
            }
        });
    };

    let back = move |event: MouseEvent| {
        event.prevent_default();
        ctx.show(Page::Sources);
    };

    view! {
        <Show when=move || source.with(Option::is_some)>
            <div class="container-fluid">
                <nav aria-label="breadcrumb">
                    <ol class="breadcrumb">
                        <li class="breadcrumb-item">
                            <a href="#" on:click=back class="text-decoration-none">
                                <b>
                                    <i class="bi-folder"></i>
                                    " Sources"
                                </b>
                            </a>
                        </li>
                        <li class="breadcrumb-item" aria-current="page">
                            <b>{move || source.with(|source| source.as_ref().map(|source| source.path.clone()))}</b>
                        </li>
                    </ol>
                </nav>
                <form class="form">
                    <div class="mb-3">
                        <label for="path" class="form-label">"Path"</label>
                        <input
                            id="path"
                            type="text"
                            class="form-control"
                            prop:value=move || path.get()
                            on:input=move |event| path.set(event_target_value(&event))
                        />
                        <div class="error">{move || errors.with(|errors| errors_for(errors.as_ref(), "path"))}</div>
                    </div>
                    <div class="btn-group" role="group" aria-label="Source actions">
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
