use leptos::ev::{KeyboardEvent, MouseEvent, SubmitEvent};
use leptos::html;
use leptos::prelude::*;
use player_core::equalizer::{self, BANDS, CUSTOM, FLAT, Gains, NameProblem, PRESETS, SavedPreset};

use crate::state::ctx;
use crate::storage;

const OPEN_KEY: &str = "player.equalizer.open";
const PRESETS_KEY: &str = "player.equalizer.presets";
const CURVE_WIDTH: f32 = 90.0;
const CURVE_HEIGHT: f32 = 20.0;

fn stored_open() -> bool {
    storage::get(OPEN_KEY).is_some_and(|stored| stored == "true")
}

fn store_open(open: bool) {
    storage::set(OPEN_KEY, &open.to_string());
}

fn stored_presets() -> Vec<SavedPreset> {
    equalizer::parse_saved(storage::get(PRESETS_KEY).as_deref())
}

fn store_presets(presets: &[SavedPreset]) {
    storage::set(PRESETS_KEY, &equalizer::serialize_saved(presets));
}

#[component]
fn SavePresetDialog(saved: RwSignal<Vec<SavedPreset>>, open: RwSignal<bool>) -> impl IntoView {
    let settings = ctx().engine.settings;
    let name = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<NameProblem>);
    let name_input: NodeRef<html::Input> = NodeRef::new();

    name_input.on_load(|input| {
        input.focus().ok();
    });

    let submit = move |event: SubmitEvent| {
        event.prevent_default();

        let current = settings.get_untracked();
        match equalizer::save_preset(
            &saved.get_untracked(),
            &name.get_untracked(),
            current.preamp,
            current.gains,
        ) {
            Ok(presets) => {
                store_presets(&presets);
                saved.set(presets);
                open.set(false);
            }
            Err(error) => problem.set(Some(error)),
        }
    };

    view! {
        <div class="dialog-backdrop" id="preset-dialog">
            <form
                class="winamp-window dialog"
                on:submit=submit
                on:keydown=move |event: KeyboardEvent| {
                    if event.key() == "Escape" {
                        open.set(false);
                    }
                }
            >
                <div class="winamp-titlebar">
                    <span>"SAVE PRESET"</span>
                </div>
                <div class="dialog-body">
                    <input
                        type="text"
                        class="form-control"
                        id="preset-name"
                        aria-label="Preset name"
                        placeholder="Name"
                        node_ref=name_input
                        prop:value=move || name.get()
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                    <div class="error" id="preset-error">{move || problem.get().map(NameProblem::message)}</div>
                    <div class="dialog-buttons">
                        <button type="submit" class="winamp-button" id="preset-save">"SAVE"</button>
                        <button
                            type="button"
                            class="winamp-button"
                            id="preset-cancel"
                            on:click=move |_: MouseEvent| open.set(false)
                        >
                            "CANCEL"
                        </button>
                    </div>
                </div>
            </form>
        </div>
    }
}

fn slider_style(gain: f32) -> String {
    let percent = equalizer::gain_percent(gain);
    let (from, to) = if percent < 50.0 {
        (percent, 50.0)
    } else {
        (50.0, percent)
    };

    format!(
        "--from: {from}%; --to: {to}%; --hue: {}",
        equalizer::slider_hue(gain)
    )
}

#[component]
pub fn Equalizer() -> impl IntoView {
    let settings = ctx().engine.settings;
    let open = RwSignal::new(stored_open());
    let saved = RwSignal::new(stored_presets());
    let saving = RwSignal::new(false);

    Effect::new(move |_| store_open(open.get()));

    let set_gains = move |gains: Gains| {
        settings.update(|settings| {
            settings.gains = gains;
            settings.enabled = true;
        });
    };

    let slider = move |id: String, label: String, gain: Signal<f32>, set: Callback<f32>| {
        view! {
            <input
                type="range"
                class="equalizer-slider"
                id=id
                aria-label=label
                min=format!("-{}", equalizer::MAX_GAIN)
                max=equalizer::MAX_GAIN.to_string()
                step="0.5"
                prop:value=move || gain.get().to_string()
                style=move || slider_style(gain.get())
                on:input=move |event| {
                    set.run(equalizer::clamp_gain(event_target_value(&event).parse().unwrap_or(0.0)));
                }
            />
        }
    };

    let band = move |index: usize| {
        let gain = Signal::derive(move || settings.with(|settings| settings.gains[index]));
        let set = Callback::new(move |value: f32| {
            let mut gains = settings.with_untracked(|settings| settings.gains);
            gains[index] = value;
            set_gains(gains);
        });

        view! {
            <div class="equalizer-band">
                <span class="equalizer-gain">{move || equalizer::gain_label(gain.get())}</span>
                {slider(
                    format!("equalizer-band-{index}"),
                    format!("{} Hz", equalizer::band_label(BANDS[index])),
                    gain,
                    set,
                )}
                <span class="equalizer-frequency">{equalizer::band_label(BANDS[index])}</span>
            </div>
        }
    };

    let preamp = Signal::derive(move || settings.with(|settings| settings.preamp));
    let set_preamp = Callback::new(move |value: f32| {
        settings.update(|settings| {
            settings.preamp = value;
            settings.enabled = true;
        });
    });

    let enabled = move || settings.with(|settings| settings.enabled);
    let selected =
        move || saved.with(|saved| settings.with(|settings| equalizer::selected(settings, saved)));
    let curve = move || {
        settings
            .with(|settings| equalizer::curve_points(&settings.gains, CURVE_WIDTH, CURVE_HEIGHT))
    };

    view! {
        <div class="winamp-window equalizer">
            <div class="window-bar">
            <button
                type="button"
                class=move || if open.get() { "winamp-button winamp-toggle winamp-lit" } else { "winamp-button winamp-toggle" }
                id="equalizer-toggle"
                aria-label="Equalizer"
                title="Equalizer"
                aria-expanded=move || open.get().to_string()
                on:click=move |_: MouseEvent| open.update(|open| *open = !*open)
            >
                <span class="winamp-light"></span>
                "EQ"
            </button>
            <div class="winamp-titlebar window-bar-title">
                <span>"EQUALIZER"</span>
            </div>
            </div>
            <Show when=move || open.get()>
                <div class="equalizer-panel" id="equalizer-panel">
                    <div class="equalizer-header">
                        <button
                            type="button"
                            class=move || if enabled() { "winamp-button winamp-toggle winamp-lit" } else { "winamp-button winamp-toggle" }
                            id="equalizer-on"
                            aria-pressed=move || enabled().to_string()
                            on:click=move |_: MouseEvent| settings.update(|settings| settings.enabled = !settings.enabled)
                        >
                            <span class="winamp-light"></span>
                            "ON"
                        </button>
                        <svg
                            class="equalizer-curve"
                            id="equalizer-curve"
                            viewBox=format!("-2 -2 {} {}", CURVE_WIDTH + 4.0, CURVE_HEIGHT + 4.0)
                            preserveAspectRatio="none"
                        >
                            <line
                                x1="0"
                                x2=CURVE_WIDTH.to_string()
                                y1=(CURVE_HEIGHT / 2.0).to_string()
                                y2=(CURVE_HEIGHT / 2.0).to_string()
                                class="equalizer-curve-zero"
                            />
                            <polyline points=curve class="equalizer-curve-line" />
                        </svg>
                        <select
                            class="winamp-select"
                            id="equalizer-preset"
                            aria-label="Presets"
                            on:change=move |event| {
                                // The Custom option is disabled, so a change always picks a preset.
                                let (gains, preamp) = equalizer::choose(&event_target_value(&event), &saved.get_untracked())
                                    .expect("the menu offers only presets");
                                settings.update(|settings| {
                                    settings.gains = gains;
                                    settings.preamp = preamp.unwrap_or(settings.preamp);
                                    settings.enabled = true;
                                });
                            }
                        >
                            {PRESETS
                                .iter()
                                .map(|preset| view! {
                                    <option value=preset.name prop:selected=move || selected() == preset.name>{preset.name}</option>
                                })
                                .collect_view()}
                            {move || {
                                saved
                                    .get()
                                    .into_iter()
                                    .map(|preset| {
                                        let (value, label) = (preset.name.clone(), preset.name.clone());
                                        view! {
                                            <option value=value prop:selected=move || selected() == preset.name>{label}</option>
                                        }
                                    })
                                    .collect_view()
                            }}
                            <option
                                value=CUSTOM
                                disabled=true
                                hidden=move || selected() != CUSTOM
                                prop:selected=move || selected() == CUSTOM
                            >
                                {CUSTOM}
                            </option>
                        </select>
                        <button
                            type="button"
                            class="winamp-button"
                            id="equalizer-save"
                            on:click=move |_: MouseEvent| saving.set(true)
                        >
                            "SAVE"
                        </button>
                        <button
                            type="button"
                            class="winamp-button"
                            id="equalizer-reset"
                            on:click=move |_: MouseEvent| {
                                settings.update(|settings| {
                                    settings.gains = FLAT;
                                    settings.preamp = 0.0;
                                });
                            }
                        >
                            "RESET"
                        </button>
                    </div>
                    <div class=move || if enabled() { "equalizer-bands" } else { "equalizer-bands equalizer-bands-off" }>
                        <div class="equalizer-band equalizer-preamp">
                            <span class="equalizer-gain">{move || equalizer::gain_label(preamp.get())}</span>
                            {slider("equalizer-preamp".to_string(), "Preamp".to_string(), preamp, set_preamp)}
                            <span class="equalizer-frequency">"PREAMP"</span>
                        </div>
                        <div class="equalizer-scale">
                            <span>"+12 db"</span>
                            <span>"+0 db"</span>
                            <span>"-12 db"</span>
                        </div>
                        {(0..BANDS.len()).map(band).collect_view()}
                    </div>
                </div>
            </Show>
            <Show when=move || saving.get()>
                <SavePresetDialog saved=saved open=saving />
            </Show>
        </div>
    }
}
