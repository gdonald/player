use leptos::ev::MouseEvent;
use leptos::prelude::*;
use player_core::equalizer::{self, BANDS, CUSTOM, FLAT, Gains, PRESETS};

use crate::audio_graph::audio_graph;
use crate::storage;

const OPEN_KEY: &str = "player.equalizer.open";
const CURVE_WIDTH: f32 = 90.0;
const CURVE_HEIGHT: f32 = 20.0;

fn stored_open() -> bool {
    storage::get(OPEN_KEY).is_some_and(|stored| stored == "true")
}

fn store_open(open: bool) {
    storage::set(OPEN_KEY, &open.to_string());
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
    let settings = audio_graph().settings;
    let open = RwSignal::new(stored_open());

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
    let preset_name = move || settings.with(|settings| equalizer::preset_name(&settings.gains));
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
                            prop:value=preset_name
                            on:change=move |event| {
                                if let Some(preset) = equalizer::preset(&event_target_value(&event)) {
                                    set_gains(preset.gains);
                                }
                            }
                        >
                            {PRESETS
                                .iter()
                                .map(|preset| view! { <option value=preset.name>{preset.name}</option> })
                                .collect_view()}
                            <option value=CUSTOM disabled=true hidden=move || preset_name() != CUSTOM>
                                {CUSTOM}
                            </option>
                        </select>
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
        </div>
    }
}
