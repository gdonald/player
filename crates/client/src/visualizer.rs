use std::time::Duration;

use leptos::html;
use leptos::prelude::*;
use player_core::spectrum::{self, BAR_COUNT};
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use crate::audio_graph::audio_graph;
use crate::storage;

const OPEN_KEY: &str = "player.visualizer.open";
const FRAME: Duration = Duration::from_millis(33);
const GAP: f64 = 2.0;
const PEAK_HEIGHT: f64 = 2.0;

/// Shown unless it was hidden before.
fn stored_open() -> bool {
    storage::get(OPEN_KEY).is_none_or(|stored| stored != "false")
}

fn store_open(open: bool) {
    storage::set(OPEN_KEY, &open.to_string());
}

/// The analyzer colors, read from the theme's `--vis-*` custom properties.
struct Palette {
    background: String,
    bar_low: String,
    bar_mid: String,
    bar_high: String,
    peak: String,
    scanline: String,
    scanline_spacing: f64,
}

impl Palette {
    fn read(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let style = window().get_computed_style(canvas).ok().flatten();
        let color = |property: &str, fallback: &str| {
            style
                .as_ref()
                .and_then(|style| style.get_property_value(property).ok())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        };

        Palette {
            background: color("--vis-background", "#000"),
            bar_low: color("--vis-bar-low", "#00b000"),
            bar_mid: color("--vis-bar-mid", "#e0e000"),
            bar_high: color("--vis-bar-high", "#e83000"),
            peak: color("--vis-peak", "#c8c8d8"),
            scanline: color("--vis-scanline", "rgba(0, 0, 0, 0.35)"),
            scanline_spacing: spectrum::scanline_spacing(&color("--vis-scanline-spacing", "")),
        }
    }
}

fn draw(canvas: &web_sys::HtmlCanvasElement, levels: &[f32], peaks: &[f32]) {
    let ratio = window().device_pixel_ratio();
    let width = f64::from(canvas.client_width()) * ratio;
    let height = f64::from(canvas.client_height()) * ratio;

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "canvas sizes are small positive pixel counts"
    )]
    let (pixel_width, pixel_height) = (width as u32, height as u32);
    if canvas.width() != pixel_width || canvas.height() != pixel_height {
        canvas.set_width(pixel_width);
        canvas.set_height(pixel_height);
    }

    let Some(context) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|context| context.dyn_into::<CanvasRenderingContext2d>().ok())
    else {
        return;
    };

    let palette = Palette::read(canvas);

    context.set_fill_style_str(&palette.background);
    context.fill_rect(0.0, 0.0, width, height);

    let gradient = context.create_linear_gradient(0.0, height, 0.0, 0.0);
    gradient.add_color_stop(0.0, &palette.bar_low).ok();
    gradient.add_color_stop(0.55, &palette.bar_mid).ok();
    gradient.add_color_stop(1.0, &palette.bar_high).ok();

    #[allow(clippy::cast_precision_loss, reason = "bar counts are small")]
    let count = levels.len().max(1) as f64;
    let gap = GAP * ratio;
    let bar_width = (width - gap * (count - 1.0)) / count;

    for (index, (level, peak)) in levels.iter().zip(peaks).enumerate() {
        #[allow(clippy::cast_precision_loss, reason = "bar indexes are small")]
        let x = index as f64 * (bar_width + gap);
        let bar_height = f64::from(*level) * height;

        context.set_fill_style_canvas_gradient(&gradient);
        context.fill_rect(x, height - bar_height, bar_width, bar_height);

        let peak_y = height - f64::from(*peak) * height;
        context.set_fill_style_str(&palette.peak);
        context.fill_rect(
            x,
            (peak_y - PEAK_HEIGHT * ratio).max(0.0),
            bar_width,
            PEAK_HEIGHT * ratio,
        );
    }

    context.set_fill_style_str(&palette.scanline);
    let mut y = 0.0;
    while y < height {
        context.fill_rect(0.0, y, width, ratio);
        y += palette.scanline_spacing * ratio;
    }
}

/// Winamp's spectrum analyzer: bars for the loudness of each pitch range,
/// with peak caps that fall back slowly.
#[component]
pub fn Visualizer() -> impl IntoView {
    let graph = audio_graph();
    let canvas: NodeRef<html::Canvas> = NodeRef::new();
    let peaks = StoredValue::new(vec![0.0_f32; BAR_COUNT]);
    let open = RwSignal::new(stored_open());

    Effect::new(move |_| store_open(open.get()));

    let frame = set_interval_with_handle(
        move || {
            let Some(element) = canvas.get_untracked() else {
                return;
            };

            let mut levels = vec![0.0; BAR_COUNT];
            graph.with_frequency_data(|bins, sample_rate| {
                levels = spectrum::tilt(&spectrum::bar_levels(bins, sample_rate, BAR_COUNT));
            });

            peaks.update_value(|peaks| spectrum::fall_peaks(peaks, &levels));
            peaks.with_value(|peaks| draw(&element, &levels, peaks));
        },
        FRAME,
    );

    on_cleanup(move || {
        if let Ok(handle) = frame {
            handle.clear();
        }
    });

    view! {
        <div class="winamp-window visualizer">
            <div class="window-bar">
                <button
                    type="button"
                    class=move || if open.get() { "winamp-button winamp-toggle winamp-lit" } else { "winamp-button winamp-toggle" }
                    id="visualizer-toggle"
                    aria-label="Visualization"
                    title="Show or hide the visualization"
                    aria-expanded=move || open.get().to_string()
                    on:click=move |_| open.update(|open| *open = !*open)
                >
                    <span class="winamp-light"></span>
                    "VIS"
                </button>
                <div class="winamp-titlebar window-bar-title">
                    <span>"VISUALIZATION"</span>
                </div>
            </div>
            <Show when=move || open.get()>
                <canvas class="visualizer-canvas" id="visualizer" node_ref=canvas></canvas>
            </Show>
        </div>
    }
}
