use leptos::ev::{Event, KeyboardEvent, MouseEvent};
use leptos::html;
use leptos::prelude::*;
use player_core::playback;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{
    MediaMetadata, MediaMetadataInit, MediaSession, MediaSessionAction, MediaSessionPlaybackState,
};

use crate::audio_graph::Transport;
use crate::state::ctx;
use crate::storage;

const VOLUME_KEY: &str = "player.volume";
/// The marquee box's left and right padding together, in pixels.
const MARQUEE_PADDING: i32 = 12;
/// How fast the title slides, in pixels per second.
const MARQUEE_SPEED: f64 = 20.0;
/// Share of each one-way trip spent moving. The rest is the pause at the end.
const MARQUEE_MOVING_SHARE: f64 = 0.7;
const POSITION_FRAME: std::time::Duration = std::time::Duration::from_millis(50);
const NOTHING_PLAYING: &str = "Nothing playing";

fn stored_volume() -> Option<String> {
    storage::get(VOLUME_KEY)
}

fn store_volume(volume: f64) {
    storage::set(VOLUME_KEY, &volume.to_string());
}

/// `navigator.mediaSession`, when the browser has it.
fn media_session() -> Option<MediaSession> {
    let navigator = window().navigator();
    let present = js_sys::Reflect::has(&navigator, &"mediaSession".into()).unwrap_or(false);

    present.then(|| navigator.media_session())
}

fn range_value(event: &Event) -> f64 {
    event_target_value(event).parse().unwrap_or(0.0)
}

/// The slide distance and one-way trip time for a title `overflow` pixels too wide.
fn marquee_style(overflow: i32) -> String {
    let seconds = f64::from(overflow) / MARQUEE_SPEED / MARQUEE_MOVING_SHARE;

    format!("--marquee-shift: -{overflow}px; --marquee-duration: {seconds:.2}s")
}

fn progress_style(percent: f64) -> String {
    format!("--progress: {percent}%")
}

fn transport_icon(transport: Transport) -> &'static str {
    match transport {
        Transport::Playing => "bi-play-fill",
        Transport::Paused => "bi-pause-fill",
        Transport::Stopped => "bi-stop-fill",
    }
}

#[component]
pub fn Player() -> impl IntoView {
    let ctx = ctx();
    let engine = ctx.engine;
    let transport = engine.transport;
    let duration = engine.duration;
    let position = RwSignal::new(0.0_f64);
    let volume = RwSignal::new(playback::parse_volume(stored_volume().as_deref()));
    let muted = RwSignal::new(false);

    let has_track = move || ctx.current.with(Option::is_some);
    let has_queue = move || ctx.queue.with(|queue| !queue.is_empty());

    let seek_to = move |seconds: f64| {
        engine.seek(seconds);
        position.set(engine.position());
    };

    // Play starts a stopped queue, resumes a paused track, and restarts a
    // playing one, as Winamp's play button did.
    let play = move || match (has_track(), transport.get_untracked()) {
        (false, _) => ctx.start_if_idle(),
        (true, Transport::Playing) => seek_to(0.0),
        (true, Transport::Paused | Transport::Stopped) => engine.resume(),
    };

    let pause = move || {
        if transport.get_untracked() == Transport::Playing {
            engine.pause();
        }
    };

    // Pause toggles between paused and playing.
    let toggle_pause = move || engine.toggle_pause();

    let stop = move || {
        engine.stop();
        position.set(0.0);
    };

    let restart = move || seek_to(0.0);

    Effect::new(move |_| {
        let level = if muted.get() { 0.0 } else { volume.get() };
        #[allow(clippy::cast_possible_truncation, reason = "a volume between 0 and 1")]
        engine.set_volume(level as f32);
    });

    // The engine knows the position at any moment, so the seek bar and the
    // clock read it at the frame rate.
    let follow = set_interval_with_handle(move || position.set(engine.position()), POSITION_FRAME)
        .expect("the browser runs intervals");
    on_cleanup(move || follow.clear());

    let space_toggles = window_event_listener(leptos::ev::keydown, move |event: KeyboardEvent| {
        let toggles = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|element| playback::space_toggles_playback(&element.tag_name()));

        if toggles && event.key() == " " && has_track() {
            event.prevent_default();
            toggle_pause();
        }
    });
    on_cleanup(move || space_toggles.remove());

    if let Some(session) = media_session() {
        let handlers: Vec<(MediaSessionAction, Closure<dyn Fn()>)> = vec![
            (MediaSessionAction::Play, Closure::new(play)),
            (MediaSessionAction::Pause, Closure::new(pause)),
            (MediaSessionAction::Stop, Closure::new(stop)),
            (
                MediaSessionAction::Nexttrack,
                Closure::new(move || ctx.next()),
            ),
            (MediaSessionAction::Previoustrack, Closure::new(restart)),
        ];

        for (action, handler) in &handlers {
            session.set_action_handler(*action, Some(handler.as_ref().unchecked_ref()));
        }

        let actions: Vec<MediaSessionAction> = handlers.iter().map(|(action, _)| *action).collect();
        let kept = StoredValue::new_local(handlers);

        let cleared_session = session.clone();
        on_cleanup(move || {
            for action in actions {
                cleared_session.set_action_handler(action, None);
            }
            kept.dispose();
        });

        let metadata_session = session.clone();
        Effect::new(move |_| {
            let metadata = ctx.current.with(|current| {
                current.as_ref().and_then(|entry| {
                    let init = MediaMetadataInit::new();
                    init.set_title(&entry.mp3.title);
                    init.set_artist(&entry.mp3.artist_name);
                    init.set_album(&entry.mp3.album_name);
                    MediaMetadata::new_with_init(&init).ok()
                })
            });
            metadata_session.set_metadata(metadata.as_ref());
        });

        Effect::new(move |_| {
            session.set_playback_state(match (has_track(), transport.get()) {
                (false, _) => MediaSessionPlaybackState::None,
                (true, Transport::Playing) => MediaSessionPlaybackState::Playing,
                (true, _) => MediaSessionPlaybackState::Paused,
            });
        });
    }

    let now_playing = move || {
        let number = ctx.current.with(|current| {
            let id = current.as_ref().map(|entry| entry.id);
            ctx.queue
                .with(|queue| queue.iter().position(|entry| Some(entry.id) == id))
                .map_or(1, |index| index + 1)
        });

        ctx.current.with(|current| {
            current.as_ref().map_or_else(
                || NOTHING_PLAYING.to_string(),
                |entry| {
                    playback::now_playing_line(
                        number,
                        &entry.mp3.artist_name,
                        &entry.mp3.title,
                        entry.mp3.length,
                    )
                },
            )
        })
    };
    // The title only moves when it is wider than its box, and then only as
    // far as its overflow: it slides left until its end shows, pauses, and
    // slides back.
    let marquee: NodeRef<html::Div> = NodeRef::new();
    let marquee_text: NodeRef<html::Span> = NodeRef::new();
    let overflow = RwSignal::new(0_i32);

    // Called from an effect and a resize listener, which only run while the
    // player is mounted, so both elements exist.
    let measure = move || {
        let frame = marquee.get_untracked().expect("the marquee is mounted");
        let text = marquee_text
            .get_untracked()
            .expect("the marquee text is mounted");

        request_animation_frame(move || {
            let room = frame.client_width() - MARQUEE_PADDING;
            overflow.set((text.offset_width() - room).max(0));
        });
    };

    Effect::new(move |_| {
        now_playing();
        measure();
    });

    let remeasure = window_event_listener(leptos::ev::resize, move |_| measure());
    on_cleanup(move || remeasure.remove());

    let album = move || {
        ctx.current.with(|current| {
            current
                .as_ref()
                .map(|entry| entry.mp3.album_name.clone())
                .unwrap_or_default()
        })
    };

    view! {
        <div class="winamp-window player" id="player">
            <div class="winamp-titlebar">
                <span>"PLAYER"</span>
                <button
                    type="button"
                    class="winamp-button logout"
                    id="logout"
                    title="Log out"
                    aria-label="Log out"
                    on:click=move |_: MouseEvent| ctx.log_out()
                >
                    <i class="bi-box-arrow-right"></i>
                </button>
            </div>
            <Show when=move || engine.loading.get()>
                <div class="buffering" id="buffering" role="status">
                    <div class="buffering-notes" aria-hidden="true">
                        <i class="bi-music-note"></i>
                        <i class="bi-music-note-beamed"></i>
                        <i class="bi-music-note"></i>
                    </div>
                    <span>"Buffering..."</span>
                </div>
            </Show>
            <div class="player-body">
                <div class="player-lcd" id="player-lcd">
                    <i class=move || format!("player-state {}", transport_icon(transport.get())) id="player-state"></i>
                    <span class="player-clock" id="player-position">
                        {move || playback::format_time(position.get())}
                    </span>
                </div>
                <div class="player-info">
                    <div class="player-marquee" node_ref=marquee>
                        <span
                            class=move || {
                                if has_track() && overflow.get() > 0 {
                                    "player-marquee-text player-marquee-scrolling"
                                } else {
                                    "player-marquee-text"
                                }
                            }
                            style=move || marquee_style(overflow.get())
                            id="player-title"
                            node_ref=marquee_text
                        >
                            {now_playing}
                        </span>
                    </div>
                    <div class="player-meta">
                        <span class="player-album" id="player-byline">{album}</span>
                        <span class="player-length" id="player-duration">
                            {move || playback::format_time(playback::remaining(position.get(), duration.get()))}
                        </span>
                    </div>
                </div>
                <div class="player-volume">
                    <button
                        type="button"
                        class="winamp-button player-mute"
                        id="player-mute"
                        aria-label=move || if muted.get() { "Unmute" } else { "Mute" }
                        title=move || if muted.get() { "Unmute" } else { "Mute" }
                        on:click=move |_: MouseEvent| muted.update(|muted| *muted = !*muted)
                    >
                        <i class=move || format!("bi-{}", playback::volume_icon(volume.get(), muted.get()))></i>
                    </button>
                    <input
                        type="range"
                        class="winamp-volume"
                        id="player-volume"
                        aria-label="Volume"
                        min="0"
                        max="1"
                        step="0.01"
                        prop:value=move || volume.get().to_string()
                        style=move || progress_style(volume.get() * 100.0)
                        on:input=move |event| {
                            let level = range_value(&event).clamp(0.0, 1.0);
                            volume.set(level);
                            muted.set(false);
                            store_volume(level);
                        }
                    />
                </div>
                <input
                    type="range"
                    class="winamp-seek"
                    id="player-seek"
                    aria-label="Seek"
                    min="0"
                    step="any"
                    max=move || duration.get().to_string()
                    prop:value=move || position.get().to_string()
                    style=move || progress_style(playback::percent(position.get(), duration.get()))
                    disabled=move || !has_track()
                    on:input=move |event| seek_to(range_value(&event))
                />
                <div class="player-transport">
                    <button
                        type="button"
                        class="winamp-button"
                        id="player-restart"
                        aria-label="Previous"
                        title="Previous (restart track)"
                        disabled=move || !has_track()
                        on:click=move |_: MouseEvent| restart()
                    >
                        <i class="bi-skip-start-fill"></i>
                    </button>
                    <button
                        type="button"
                        class="winamp-button"
                        id="player-play"
                        aria-label="Play"
                        title="Play"
                        disabled=move || !has_queue()
                        on:click=move |_: MouseEvent| play()
                    >
                        <i class="bi-play-fill"></i>
                    </button>
                    <button
                        type="button"
                        class="winamp-button"
                        id="player-pause"
                        aria-label="Pause"
                        title="Pause"
                        disabled=move || !has_track()
                        on:click=move |_: MouseEvent| toggle_pause()
                    >
                        <i class="bi-pause-fill"></i>
                    </button>
                    <button
                        type="button"
                        class="winamp-button"
                        id="player-stop"
                        aria-label="Stop"
                        title="Stop"
                        disabled=move || !has_track()
                        on:click=move |_: MouseEvent| stop()
                    >
                        <i class="bi-stop-fill"></i>
                    </button>
                    <button
                        type="button"
                        class="winamp-button"
                        id="player-next"
                        aria-label="Next"
                        title="Next"
                        disabled=move || !has_track()
                        on:click=move |_: MouseEvent| ctx.next()
                    >
                        <i class="bi-skip-end-fill"></i>
                    </button>
                </div>
            </div>
        </div>
    }
}
