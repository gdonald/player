use leptos::prelude::*;
use player_core::equalizer::{self, BANDS, FilterKind, PEAKING_Q, Settings};
use player_core::spectrum;
use wasm_bindgen::JsValue;
use web_sys::{
    AnalyserNode, AudioContext, AudioNode, BiquadFilterNode, BiquadFilterType, GainNode,
    HtmlMediaElement, MediaElementAudioSourceNode,
};

use crate::storage;

const SETTINGS_KEY: &str = "player.equalizer";

/// 4096 samples gives 2048 bins about 11 Hz wide, fine enough to keep the
/// lowest analyzer bars apart.
const FFT_SIZE: u32 = 4096;
const SMOOTHING: f64 = 0.6;

fn stored_settings() -> Settings {
    equalizer::parse(storage::get(SETTINGS_KEY).as_deref())
}

fn store_settings(settings: &Settings) {
    storage::set(SETTINGS_KEY, &equalizer::serialize(settings));
}

/// The audio element feeds the preamp, the preamp feeds the filters in band
/// order, the last filter feeds the analyser, and the analyser feeds the
/// speakers. A media element can be given a source node once, so each new
/// `<audio>` element gets its own.
struct Graph {
    context: AudioContext,
    preamp: GainNode,
    filters: Vec<BiquadFilterNode>,
    analyser: AnalyserNode,
    attached: Option<(HtmlMediaElement, MediaElementAudioSourceNode)>,
    bins: Vec<u8>,
}

impl Graph {
    fn build() -> Result<Graph, JsValue> {
        let context = AudioContext::new()?;
        let preamp = context.create_gain()?;
        let analyser = context.create_analyser()?;
        analyser.set_fft_size(FFT_SIZE);
        analyser.set_smoothing_time_constant(SMOOTHING);
        analyser.set_min_decibels(spectrum::MIN_DECIBELS);
        analyser.set_max_decibels(spectrum::MAX_DECIBELS);

        let mut filters = Vec::with_capacity(BANDS.len());
        for (index, frequency) in BANDS.iter().enumerate() {
            let filter = context.create_biquad_filter()?;

            filter.set_type(match equalizer::filter_kind(index) {
                FilterKind::LowShelf => BiquadFilterType::Lowshelf,
                FilterKind::Peaking => BiquadFilterType::Peaking,
                FilterKind::HighShelf => BiquadFilterType::Highshelf,
            });
            filter.frequency().set_value(*frequency);
            filter.q().set_value(PEAKING_Q);
            filter.gain().set_value(0.0);

            filters.push(filter);
        }

        let last_filter =
            filters
                .iter()
                .try_fold(AudioNode::from(preamp.clone()), |previous, filter| {
                    previous.connect_with_audio_node(filter)?;
                    Ok::<_, JsValue>(AudioNode::from(filter.clone()))
                });
        last_filter?.connect_with_audio_node(&analyser)?;
        analyser.connect_with_audio_node(&context.destination())?;

        let bins = vec![0; usize::try_from(analyser.frequency_bin_count()).unwrap_or(0)];

        Ok(Graph {
            context,
            preamp,
            filters,
            analyser,
            attached: None,
            bins,
        })
    }

    fn attach(&mut self, element: &HtmlMediaElement) -> Result<(), JsValue> {
        if self
            .attached
            .as_ref()
            .is_some_and(|(attached, _)| attached == element)
        {
            return Ok(());
        }

        if let Some((_, previous)) = self.attached.take() {
            previous.disconnect().ok();
        }

        let source = self.context.create_media_element_source(element)?;
        source.connect_with_audio_node(&self.preamp)?;
        self.attached = Some((element.clone(), source));

        Ok(())
    }

    fn apply(&self, settings: &Settings) {
        self.preamp
            .gain()
            .set_value(equalizer::decibels_to_amplitude(settings.applied_preamp()));

        for (filter, gain) in self.filters.iter().zip(settings.applied()) {
            filter.gain().set_value(gain);
        }
    }

    /// Browsers start an audio context suspended until the page has had a
    /// click or key press. Resuming a running context changes nothing.
    fn resume(&self) {
        self.context.resume().ok();
    }
}

/// The audio chain behind the player, shared by the equalizer and the
/// analyzer through context.
#[derive(Clone, Copy)]
pub struct AudioGraph {
    graph: StoredValue<Option<Graph>, LocalStorage>,
    pub settings: RwSignal<Settings>,
}

pub fn audio_graph() -> AudioGraph {
    expect_context::<AudioGraph>()
}

impl AudioGraph {
    /// Builds the chain when the first `<audio>` element appears, attaches each
    /// new element, applies the equalizer settings, and resumes the context
    /// on clicks and key presses.
    pub fn provide() {
        let audio = crate::state::ctx().audio;
        let handle = AudioGraph {
            graph: StoredValue::new_local(None),
            settings: RwSignal::new(stored_settings()),
        };
        provide_context(handle);

        Effect::new(move |_| {
            let current = handle.settings.get();

            if let Some(element) = audio.get() {
                if handle.graph.with_value(Option::is_none) {
                    match Graph::build() {
                        Ok(built) => handle.graph.set_value(Some(built)),
                        Err(error) => leptos::logging::error!("audio chain unavailable: {error:?}"),
                    }
                }

                handle.graph.update_value(|graph| {
                    if let Some(graph) = graph {
                        if let Err(error) = graph.attach(&element) {
                            leptos::logging::error!("audio chain could not attach: {error:?}");
                        }
                        graph.apply(&current);
                        graph.resume();
                    }
                });
            }

            store_settings(&current);
        });

        let resume = move || {
            handle
                .graph
                .with_value(|graph| graph.as_ref().map(Graph::resume))
        };
        let on_press = window_event_listener(leptos::ev::pointerdown, move |_| {
            resume();
        });
        let on_key = window_event_listener(leptos::ev::keydown, move |_| {
            resume();
        });
        on_cleanup(move || {
            on_press.remove();
            on_key.remove();
        });
    }

    /// Runs `read` with the current frequency bins and sample rate, or does
    /// nothing before the chain exists.
    pub fn with_frequency_data(&self, read: impl FnOnce(&[u8], f32)) {
        self.graph.update_value(|graph| {
            if let Some(graph) = graph {
                graph.analyser.get_byte_frequency_data(&mut graph.bins);
                read(&graph.bins, graph.context.sample_rate());
            }
        });
    }
}
