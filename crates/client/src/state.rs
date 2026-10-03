use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::{paths, queue};
use player_types::{FlexId, QueuedMp3, QueuedMp3Params, QueuedMp3sResponse, wrap};
use serde_json::json;

use crate::api::{self, ApiError};
use crate::storage;

const QUERY_KEY: &str = "player.query";
const MODE_KEY: &str = "player.mode";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Mp3s,
    Mp3(i64),
    Playlists,
    Playlist(i64),
    Sources,
    Source(i64),
}

#[derive(Debug, Clone, Copy)]
pub struct Ctx {
    pub authenticated: RwSignal<Option<bool>>,
    pub page: RwSignal<Page>,
    pub query: RwSignal<Option<String>>,
    pub message: RwSignal<String>,
    pub waiting: RwSignal<u32>,
    pub queue: RwSignal<Vec<QueuedMp3>>,
    pub current: RwSignal<Option<QueuedMp3>>,
    pub src: RwSignal<Option<String>>,
    /// The `<audio>` element, shared by the player and the equalizer.
    pub audio: NodeRef<leptos::html::Audio>,
    /// Play through, loop one song, or loop the playlist.
    pub mode: RwSignal<queue::Mode>,
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
            src: RwSignal::new(None),
            audio: NodeRef::new(),
            mode: RwSignal::new(queue::Mode::parse(storage::get(MODE_KEY).as_deref())),
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

    pub fn wait(&self) -> Waiting {
        self.waiting.update(|count| *count += 1);
        Waiting(self.waiting)
    }

    /// Switches views and clears the alert, as the menu and Go Back did.
    pub fn show(&self, page: Page) {
        self.message.set(String::new());
        self.page.set(page);
    }

    pub fn fail(&self, error: &ApiError) {
        if matches!(error, ApiError::Unauthorized) {
            self.authenticated.set(Some(false));
        } else {
            self.message.set(error.message());
        }
    }

    /// Shows the 422 message, as the React views did, and returns its field errors.
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
        if let Some(element) = self.audio.get_untracked() {
            element.set_current_time(0.0);
            element.play().ok();
        }
    }

    fn play(&self, entry: Option<QueuedMp3>) {
        let src = entry.as_ref().map(|entry| paths::play(entry.mp3.id));

        // The same song queued twice has the same source, which the element
        // would not reload.
        if src.is_some() && self.src.with_untracked(|current| *current == src) {
            self.restart();
        }
        self.src.set(src);

        if let Some(mp3_id) = entry.as_ref().map(|entry| entry.mp3.id) {
            self.record_played(mp3_id);
        }

        self.current.set(entry);
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
    pub fn play_entry(&self, id: i64) {
        if self.current_id() == Some(id) {
            self.restart();
            return;
        }

        if let Some(entry) = self.entry(id) {
            self.play(Some(entry));
        }
    }

    pub fn start_if_idle(&self) {
        if let Some(id) = queue::start(&self.queue_ids(), self.current_id()) {
            self.play(self.entry(id));
        }
    }

    fn apply_queue(&self, result: Result<QueuedMp3sResponse, ApiError>) {
        match result {
            Ok(body) => self.queue.set(body.queued_mp3s),
            Err(error) => self.fail(&error),
        }
    }

    /// Nothing loaded, stopped, or finished. A song paused partway through
    /// is not stopped.
    fn is_stopped(&self) -> bool {
        self.audio.get_untracked().is_none_or(|element| {
            element.paused() && (element.current_time() <= 0.0 || element.ended())
        })
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

    /// Removes the entry from the queue and nothing more. Removing the
    /// playing entry stops playback.
    pub fn remove(&self, id: i64) {
        if self.current_id() == Some(id) {
            self.play(None);
        }

        self.delete_entry(id);
    }
}
