use leptos::prelude::*;
use player_core::theme::{self, THEMES, Theme};

use player_core::settings::THEME;

use crate::state::ctx;
use crate::{settings, storage};

/// The theme is also cached in the browser, where index.html reads it before
/// the app loads, so a reload does not flash the default theme.
pub const BROWSER_KEY: &str = "player.theme";

/// Points the theme stylesheet link in index.html at the theme's file.
fn apply(chosen: Theme) {
    document()
        .get_element_by_id("theme")
        .expect("index.html has the theme link")
        .set_attribute("href", &chosen.stylesheet())
        .ok();
}

/// Puts the cached theme in place until the settings load.
pub fn apply_cached() {
    apply(theme::find(storage::get(BROWSER_KEY).as_deref()));
}

/// Puts the saved theme in place and caches it for the next page load.
pub fn apply_saved(name: &str) {
    let chosen = theme::find(Some(name));
    storage::set(BROWSER_KEY, chosen.name);
    apply(chosen);
}

#[component]
pub fn ThemePicker() -> impl IntoView {
    let ctx = ctx();
    let current = RwSignal::new(theme::find(Some(&settings::get(ctx, THEME))));

    let choose = move |event| {
        let chosen = theme::find(Some(&event_target_value(&event)));
        settings::set(ctx, THEME, chosen.name.to_string());
        apply_saved(chosen.name);
        current.set(chosen);
    };

    view! {
        <select class="theme-select" id="theme-select" aria-label="Theme" title="Theme" on:change=choose>
            {THEMES
                .into_iter()
                .map(|option| {
                    view! {
                        <option value=option.name selected=move || current.get() == option>
                            {option.label}
                        </option>
                    }
                })
                .collect_view()}
        </select>
    }
}
