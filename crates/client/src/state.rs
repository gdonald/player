use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::playback;
use player_core::queue;
use player_types::{CountsResponse, FlexId, QueuedMp3, QueuedMp3Params, QueuedMp3sResponse, wrap};
use serde_json::json;

use crate::api::{self, ApiError};
use crate::audio_graph::{Engine, Plan, Transport};
use crate::storage;

const AUDIO_UNAVAILABLE: &str = "This browser cannot play audio.";

const QUERY_KEY: &str = "player.query";
const MODE_KEY: &str = "player.mode";
const RESUME_KEY: &str = "player.resume";
const RESUME_SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Mp3s,
    Mp3(i64),
    Playlists,
    Playlist(i64),
    Sources,
    Source(i64),
}

#[derive(Clone, Copy)]
pub struct Ctx {
    pub authenticated: RwSignal<Option<bool>>,
    pub page: RwSignal<Page>,
    pub query: RwSignal<Option<String>>,
    pub message: RwSignal<String>,
    pub waiting: RwSignal<u32>,
    pub queue: RwSignal<Vec<QueuedMp3>>,
    pub current: RwSignal<Option<QueuedMp3>>,
    /// Plays the songs, shared by the player, the equalizer, and the analyzer.
    pub engine: Engine,
    /// Play through, loop one song, or loop the playlist.
    pub mode: RwSignal<queue::Mode>,
    /// The library counts on the menu buttons.
    pub counts: RwSignal<Option<CountsResponse>>,
    /// Whether the first queue load restored the saved playback.
    restored: StoredValue<bool>,
}

pub fn ctx() -> Ctx {
    expect_context::<Ctx>()
}

/// Counts one request in flight for the spinner until dropped.
pub struct Waiting(RwSignal<u32>);

impl Drop for Waiting {
    fn drop(&mut self) {
        self.0.update(|count| *count = count.saturating_sub(1));
    }
}

impl Ctx {
    pub fn new() -> Self {
        Ctx {
            authenticated: RwSignal::new(None),
            page: RwSignal::new(Page::Mp3s),
            query: RwSignal::new(storage::get(QUERY_KEY).filter(|query| !query.is_empty())),
            message: RwSignal::new(String::new()),
            waiting: RwSignal::new(0),
            queue: RwSignal::new(Vec::new()),
            current: RwSignal::new(None),
            engine: Engine::new(),
            mode: RwSignal::new(queue::Mode::parse(storage::get(MODE_KEY).as_deref())),
            counts: RwSignal::new(None),
            restored: StoredValue::new(false),
        }
    }

    /// Keeps the MP3s search and the play mode across reloads.
    pub fn remember_query(&self) {
        let query = self.query;
        Effect::new(move |_| {
            storage::set(QUERY_KEY, query.get().as_deref().unwrap_or_default());
        });

        let mode = self.mode;
        Effect::new(move |_| storage::set(MODE_KEY, mode.get().as_str()));
    }

    fn save_playback(&self) {
        let saved = playback::serialize_resume(self.current_id(), self.engine.position());
        storage::set(RESUME_KEY, &saved);
    }

    /// On the first queue load, cues the saved entry paused at the saved
    /// position when it is still in the queue, since browsers do not start
    /// audio before a click. From then on the current entry and position are
    /// saved each second.
    fn restore(&self) {
        if self.restored.get_value() {
            return;
        }
        self.restored.set_value(true);

        let saved = playback::parse_resume(storage::get(RESUME_KEY).as_deref());
        let entry = saved.and_then(|saved| {
            self.entry(saved.entry_id)
                .map(|entry| (entry, saved.position))
        });

        if let Some((entry, position)) = entry
            && self.engine.cue(entry.mp3.id, position)
        {
            self.current.set(Some(entry));
        }

        let ctx = *self;
        set_interval(move || ctx.save_playback(), RESUME_SAVE_INTERVAL);
    }

    pub fn wait(&self) -> Waiting {
        self.waiting.update(|count| *count += 1);
        Waiting(self.waiting)
    }

    /// Switches views and clears the alert.
    pub fn show(&self, page: Page) {
        self.message.set(String::new());
        self.page.set(page);
    }

    pub fn fail(&self, error: &ApiError) {
        match error.message() {
            Some(text) => self.message.set(text),
            None => self.authenticated.set(Some(false)),
        }
    }

    /// Shows the 422 message and returns its field errors.
    pub fn invalid(&self, error: &ApiError) -> Option<player_types::FieldErrors> {
        self.fail(error);
        error.field_errors().map(|body| body.errors)
    }

    fn queue_ids(&self) -> Vec<i64> {
        self.queue
            .with_untracked(|entries| entries.iter().map(|entry| entry.id).collect())
    }

    fn current_id(&self) -> Option<i64> {
        self.current
            .with_untracked(|current| current.as_ref().map(|entry| entry.id))
    }

    fn entry(&self, id: i64) -> Option<QueuedMp3> {
        self.queue
            .with_untracked(|entries| entries.iter().find(|entry| entry.id == id).cloned())
    }

    fn restart(&self) {
        self.engine.replay();
    }

    fn play(&self, entry: Option<QueuedMp3>) {
        match &entry {
            Some(_) if !self.engine.available() => {
                self.message.set(AUDIO_UNAVAILABLE.to_string());
                return;
            }
            Some(chosen) => {
                self.engine.play(chosen.mp3.id);
                self.record_played(chosen.mp3.id);
            }
            None => self.engine.clear(),
        }

        self.current.set(entry);
    }

    /// Keeps the engine told which entry follows the current one, and moves
    /// the queue on when a song ends.
    pub fn follow_engine(&self) {
        let ctx = *self;

        Effect::new(move |_| {
            let ids: Vec<i64> = ctx
                .queue
                .with(|entries| entries.iter().map(|entry| entry.id).collect());
            let current = ctx.current.get();
            let step =
                queue::finished(&ids, current.as_ref().map(|entry| entry.id), ctx.mode.get());

            let next = if step.restart {
                current
            } else {
                step.play.and_then(|id| ctx.entry(id))
            };
            ctx.engine.prepare(next.map(|entry| Plan {
                entry_id: entry.id,
                mp3_id: entry.mp3.id,
            }));
        });

        Effect::new(move |_| {
            if let Some(ended) = ctx.engine.ended.get() {
                ctx.finished(ended.started);
            }
        });

        Effect::new(move |_| {
            if let Some(error) = ctx.engine.failure.get() {
                ctx.fail(&error);
            }
        });
    }

    /// A song ended. When the engine had the next entry decoded, that entry
    /// is already playing, so the queue follows it. Otherwise the play mode
    /// decides what comes next, as it would for a song that ended on its own.
    fn finished(&self, started: Option<Plan>) {
        let Some(plan) = started else {
            self.advance();
            return;
        };

        let step = queue::finished(
            &self.queue_ids(),
            self.current_id(),
            self.mode.get_untracked(),
        );
        if self.current_id() != Some(plan.entry_id) {
            self.record_played(plan.mp3_id);
        }
        self.current.set(self.entry(plan.entry_id));

        if let Some(id) = step.remove {
            self.delete_entry(id);
        }
    }

    fn record_played(&self, mp3_id: i64) {
        let ctx = *self;
        spawn_local(async move {
            let path = format!("/api/mp3s/{mp3_id}/played");
            // Recording is bookkeeping, so only an ended session is shown.
            if let Err(ApiError::Unauthorized) =
                api::post::<serde_json::Value>(&path, &json!({})).await
            {
                ctx.authenticated.set(Some(false));
            }
        });
    }

    /// Plays the chosen queue entry. Choosing the entry that is already
    /// playing starts it over.
    pub fn play_entry(&self, entry: QueuedMp3) {
        if self.current_id() == Some(entry.id) {
            self.restart();
            return;
        }

        self.play(Some(entry));
    }

    pub fn start_if_idle(&self) {
        if let Some(id) = queue::start(&self.queue_ids(), self.current_id()) {
            self.play(self.entry(id));
        }
    }

    fn apply_queue(&self, result: Result<QueuedMp3sResponse, ApiError>) {
        match result {
            Ok(body) => {
                self.queue.set(body.queued_mp3s);
                self.restore();
            }
            Err(error) => self.fail(&error),
        }
    }

    /// Nothing loaded, stopped, or finished. A song paused partway through
    /// is not stopped.
    fn is_stopped(&self) -> bool {
        self.engine.transport.get_untracked() == Transport::Stopped
    }

    /// After an enqueue: when nothing is playing, start the first song it
    /// added rather than whatever was already at the top of the queue.
    fn apply_enqueued(&self, before: &[i64], result: Result<QueuedMp3sResponse, ApiError>) {
        self.apply_queue(result);

        if self.is_stopped()
            && let Some(id) = queue::first_added(before, &self.queue_ids())
        {
            self.play(self.entry(id));
        }
    }

    pub fn load_queue(&self) {
        let ctx = *self;
        spawn_local(async move {
            ctx.apply_queue(api::get("/api/queued_mp3s").await);
        });
    }

    /// Loads the queue after entries were removed on the server. Playback
    /// stops when the playing entry is gone.
    pub fn reload_queue(&self) {
        let ctx = *self;
        spawn_local(async move {
            ctx.apply_queue(api::get("/api/queued_mp3s").await);

            if ctx.current_id().is_some_and(|id| ctx.entry(id).is_none()) {
                ctx.play(None);
            }
        });
    }

    pub fn enqueue_mp3(&self, mp3_id: i64) {
        let ctx = *self;
        let body = wrap(&QueuedMp3Params {
            mp3_id: FlexId(Some(mp3_id)),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            let before = ctx.queue_ids();
            ctx.apply_enqueued(&before, api::post("/api/queued_mp3s", &body).await);
        });
    }

    pub fn enqueue_playlist(&self, playlist_id: i64) {
        let ctx = *self;

        spawn_local(async move {
            let _waiting = ctx.wait();
            let path = format!("/api/playlists/{playlist_id}/enqueue");
            let before = ctx.queue_ids();
            ctx.apply_enqueued(&before, api::post(&path, &json!({})).await);
        });
    }

    fn delete_entry(&self, id: i64) {
        let ctx = *self;
        spawn_local(async move {
            let path = format!("/api/queued_mp3s/{id}");
            ctx.apply_queue(api::delete(&path).await);
        });
    }

    fn apply(&self, step: queue::Step) {
        if step.restart {
            self.restart();
        } else {
            self.play(step.play.and_then(|id| self.entry(id)));
        }

        if let Some(id) = step.remove {
            self.delete_entry(id);
        }
    }

    /// The current track ended: the play mode decides what comes next.
    pub fn advance(&self) {
        let mode = self.mode.get_untracked();
        self.apply(queue::finished(&self.queue_ids(), self.current_id(), mode));
    }

    /// The next button.
    pub fn next(&self) {
        let mode = self.mode.get_untracked();
        self.apply(queue::skip(&self.queue_ids(), self.current_id(), mode));
    }

    /// Loads the library counts for the menu buttons, after a change that
    /// adds or removes a playlist or a source.
    pub fn load_counts(&self) {
        let ctx = *self;
        spawn_local(async move {
            match api::get::<CountsResponse>("/api/counts").await {
                Ok(body) => ctx.counts.set(Some(body)),
                Err(error) => ctx.fail(&error),
            }
        });
    }

    /// Ends the session, stops playback, and shows the login form.
    pub fn log_out(&self) {
        let ctx = *self;
        spawn_local(async move {
            let _waiting = ctx.wait();
            match api::logout().await {
                Ok(()) => {
                    ctx.play(None);
                    ctx.authenticated.set(Some(false));
                }
                Err(error) => ctx.fail(&error),
            }
        });
    }

    /// Empties the queue and stops playback.
    pub fn clear_queue(&self) {
        self.play(None);

        let ctx = *self;
        spawn_local(async move {
            let _waiting = ctx.wait();
            ctx.apply_queue(api::delete("/api/queued_mp3s").await);
        });
    }

    /// Removes the entry from the queue and nothing more. Removing the
    /// playing entry stops playback.
    pub fn remove(&self, id: i64) {
        if self.current_id() == Some(id) {
            self.play(None);
        }

        self.delete_entry(id);
    }
}
