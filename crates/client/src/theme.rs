use leptos::prelude::*;
use player_core::theme::{self, THEMES, Theme};

use crate::storage;

const THEME_KEY: &str = "player.theme";

fn stored() -> Theme {
    theme::find(storage::get(THEME_KEY).as_deref())
}

/// Points the theme stylesheet link in index.html at the theme's file.
fn apply(chosen: Theme) {
    if let Some(link) = document().get_element_by_id("theme") {
        link.set_attribute("href", &chosen.stylesheet()).ok();
    }
}

/// Puts the stored theme in place, which also replaces a stored name that is
/// not a known theme with the default.
pub fn apply_stored() {
    apply(stored());
}

#[component]
pub fn ThemePicker() -> impl IntoView {
    let current = RwSignal::new(stored());

    let choose = move |event| {
        let chosen = theme::find(Some(&event_target_value(&event)));
        storage::set(THEME_KEY, chosen.name);
        apply(chosen);
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
