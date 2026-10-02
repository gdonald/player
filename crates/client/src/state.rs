use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::{paths, queue};
use player_types::{FlexId, QueuedMp3, QueuedMp3Params, QueuedMp3sResponse, wrap};
use serde_json::json;

use crate::api::{self, ApiError};

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
            query: RwSignal::new(None),
            message: RwSignal::new(String::new()),
            waiting: RwSignal::new(0),
            queue: RwSignal::new(Vec::new()),
            current: RwSignal::new(None),
            src: RwSignal::new(None),
        }
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

    fn play(&self, entry: Option<QueuedMp3>) {
        self.src
            .set(entry.as_ref().map(|entry| paths::play(entry.mp3.id)));
        self.current.set(entry);
    }

    pub fn start_if_idle(&self) {
        if let Some(id) = queue::start(&self.queue_ids(), self.current_id()) {
            self.play(self.entry(id));
        }
    }

    fn apply_queue(&self, result: Result<QueuedMp3sResponse, ApiError>, start: bool) {
        match result {
            Ok(body) => {
                self.queue.set(body.queued_mp3s);
                if start {
                    self.start_if_idle();
                }
            }
            Err(error) => self.fail(&error),
        }
    }

    pub fn load_queue(&self) {
        let ctx = *self;
        spawn_local(async move {
            ctx.apply_queue(api::get("/api/queued_mp3s").await, false);
        });
    }

    pub fn enqueue_mp3(&self, mp3_id: i64) {
        let ctx = *self;
        let body = wrap(&QueuedMp3Params {
            mp3_id: FlexId(Some(mp3_id)),
        });

        spawn_local(async move {
            let _waiting = ctx.wait();
            ctx.apply_queue(api::post("/api/queued_mp3s", &body).await, true);
        });
    }

    pub fn enqueue_playlist(&self, playlist_id: i64) {
        let ctx = *self;

        spawn_local(async move {
            let _waiting = ctx.wait();
            let path = format!("/api/playlists/{playlist_id}/enqueue");
            ctx.apply_queue(api::post(&path, &json!({})).await, true);
        });
    }

    fn delete_entry(&self, id: i64) {
        let ctx = *self;
        spawn_local(async move {
            let path = format!("/api/queued_mp3s/{id}");
            ctx.apply_queue(api::delete(&path).await, false);
        });
    }

    /// The current track ended: drop it from the queue and play the next one.
    pub fn advance(&self) {
        let step = queue::advance(&self.queue_ids(), self.current_id());

        self.play(step.play.and_then(|id| self.entry(id)));

        if let Some(id) = step.remove {
            self.delete_entry(id);
        }
    }

    /// Deleting the current entry moves on to the next one.
    pub fn remove(&self, id: i64) {
        if self.current_id() == Some(id) {
            self.advance();
        } else {
            self.delete_entry(id);
        }
    }
}
