use std::collections::BTreeMap;
use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::equalizer;
use player_core::queue::Mode;
use player_core::settings::{
    self, EQUALIZER, EQUALIZER_OPEN, EQUALIZER_PRESETS, LIST_MODE, LOOP_MODE, PLAYLIST_OPEN,
    RESUME, SEARCH, THEME, VISUALIZER_OPEN, VOLUME,
};
use player_types::{SettingParams, SettingsResponse, wrap};

use crate::api;
use crate::state::Ctx;
use crate::{storage, theme};

/// Where this browser kept each setting before settings moved to the server.
/// A value found there is used and uploaded when the server has none.
const BROWSER_KEYS: [(&str, &str); 11] = [
    (EQUALIZER, "player.equalizer"),
    (EQUALIZER_PRESETS, "player.equalizer.presets"),
    (EQUALIZER_OPEN, "player.equalizer.open"),
    (VISUALIZER_OPEN, "player.visualizer.open"),
    (PLAYLIST_OPEN, "player.playlist.open"),
    (THEME, theme::BROWSER_KEY),
    (VOLUME, "player.volume"),
    (LOOP_MODE, "player.mode"),
    (RESUME, "player.resume"),
    (SEARCH, "player.query"),
    (LIST_MODE, "player.mp3s.mode"),
];

/// A slider drag changes its setting many times a second, so its save waits
/// for the changes to stop.
const SLIDER_SAVE_DELAY: Duration = Duration::from_millis(300);

fn save_delay(name: &str) -> Duration {
    match name {
        EQUALIZER | VOLUME => SLIDER_SAVE_DELAY,
        _ => Duration::ZERO,
    }
}

/// The values the page uses, the values the server holds (`None` until they
/// load, or when loading failed), and the saves waiting to go out. The page
/// renders once `loaded`.
#[derive(Clone, Copy)]
pub struct Synced {
    pub loaded: RwSignal<bool>,
    state: StoredValue<State>,
}

#[derive(Default)]
struct State {
    values: BTreeMap<String, String>,
    server: Option<BTreeMap<String, String>>,
    timers: BTreeMap<&'static str, Option<TimeoutHandle>>,
}

impl Synced {
    pub fn new() -> Self {
        Synced {
            loaded: RwSignal::new(false),
            state: StoredValue::new(State::default()),
        }
    }
}

/// The setting's current value. Empty before the settings load, which every
/// setting reads as its default.
pub fn get(ctx: Ctx, name: &str) -> String {
    ctx.synced
        .state
        .with_value(|state| state.values.get(name).cloned())
        .unwrap_or_default()
}

/// Changes the setting and saves it.
pub fn set(ctx: Ctx, name: &'static str, value: String) {
    ctx.synced.state.update_value(|state| {
        state.values.insert(name.to_string(), value.clone());
    });

    save_later(ctx, name, value);
}

/// Loads the signed-in user's settings, puts them in place, then loads the
/// queue, whose first load resumes the saved song.
pub fn load(ctx: Ctx) {
    ctx.synced.loaded.set(false);

    spawn_local(async move {
        match api::get::<SettingsResponse>("/api/settings").await {
            Ok(body) => start(ctx, body.settings),
            Err(error) => ctx.fail(&error),
        }

        apply(ctx);
        ctx.synced.loaded.set(true);
        ctx.load_queue();
    });
}

/// Picks each setting's starting value: the server's, else this browser's
/// old one, which is then uploaded, else the default, which counts as saved.
fn start(ctx: Ctx, mut server: BTreeMap<String, String>) {
    let mut values = BTreeMap::new();
    let mut uploads = Vec::new();

    for (name, browser_key) in BROWSER_KEYS {
        let value = if let Some(saved) = server.get(name) {
            saved.clone()
        } else if let Some(kept) = storage::get(browser_key) {
            let kept = settings::normalize(name, &kept).unwrap_or_default();
            uploads.push((name, kept.clone()));
            kept
        } else {
            let default = settings::default(name).unwrap_or_default();
            server.insert(name.to_string(), default.clone());
            default
        };

        values.insert(name.to_string(), value);
    }

    ctx.synced.state.update_value(|state| {
        state.values = values;
        state.server = Some(server);
    });

    for (name, value) in uploads {
        save_later(ctx, name, value);
    }
}

/// Puts the loaded settings into the signals that the rest of the page reads.
fn apply(ctx: Ctx) {
    ctx.engine
        .settings
        .set(equalizer::parse(Some(&get(ctx, EQUALIZER))));
    ctx.presets
        .set(equalizer::parse_saved(Some(&get(ctx, EQUALIZER_PRESETS))));
    ctx.mode.set(Mode::parse(Some(&get(ctx, LOOP_MODE))));
    ctx.query
        .set(Some(get(ctx, SEARCH)).filter(|query| !query.is_empty()));
    theme::apply_saved(&get(ctx, THEME));
}

/// Saves the settings held in signals whenever they change.
pub fn save_on_change(ctx: Ctx) {
    Effect::new(move |_| {
        let value = equalizer::serialize(&ctx.engine.settings.get());
        set(ctx, EQUALIZER, value);
    });

    Effect::new(move |_| {
        let value = ctx
            .presets
            .with(|presets| equalizer::serialize_saved(presets));
        set(ctx, EQUALIZER_PRESETS, value);
    });

    Effect::new(move |_| set(ctx, LOOP_MODE, ctx.mode.get().as_str().to_string()));

    Effect::new(move |_| {
        let query = ctx.query.get().unwrap_or_default();
        set(ctx, SEARCH, query);
    });
}

fn save_later(ctx: Ctx, name: &'static str, value: String) {
    let unchanged = ctx.synced.state.with_value(|state| {
        state
            .server
            .as_ref()
            .is_none_or(|server| server.get(name) == Some(&value))
    });

    if unchanged {
        return;
    }

    let waiting = ctx
        .synced
        .state
        .try_update_value(|state| state.timers.remove(name));
    waiting
        .flatten()
        .flatten()
        .iter()
        .for_each(TimeoutHandle::clear);

    let timer = set_timeout_with_handle(move || save(ctx, name, value), save_delay(name));
    ctx.synced.state.update_value(|state| {
        state.timers.insert(name, timer.ok());
    });
}

fn save(ctx: Ctx, name: &'static str, value: String) {
    let body = wrap(&SettingParams { value: Some(value) });

    spawn_local(async move {
        match api::put::<SettingsResponse>(&format!("/api/settings/{name}"), &body).await {
            Ok(body) => ctx
                .synced
                .state
                .update_value(|state| state.server = Some(body.settings)),
            Err(error) => ctx.fail(&error),
        }
    });
}
