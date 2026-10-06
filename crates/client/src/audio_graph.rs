use js_sys::Uint8Array;
use leptos::prelude::*;
use leptos::task::spawn_local;
use player_core::equalizer::{self, BANDS, FilterKind, PEAKING_Q, Settings};
use player_core::{paths, spectrum};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AnalyserNode, AudioBuffer, AudioBufferSourceNode, AudioContext, AudioNode,
    AudioScheduledSourceNode, BiquadFilterNode, BiquadFilterType, GainNode,
};

use crate::api::{self, ApiError};
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

/// What the player is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Playing,
    Paused,
    Stopped,
}

/// The queue entry that plays when the current song ends, and its song.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub entry_id: i64,
    pub mp3_id: i64,
}

/// A song came to its end. `started` is the planned entry that began on its
/// last sample, or none when nothing was ready and playback stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ended {
    pub started: Option<Plan>,
    count: u64,
}

/// Decoded songs play into the volume, the volume feeds the preamp, the
/// preamp feeds the filters in band order, the last filter feeds the
/// analyser, and the analyser feeds the speakers.
struct Graph {
    context: AudioContext,
    volume: GainNode,
    preamp: GainNode,
    filters: Vec<BiquadFilterNode>,
    analyser: AnalyserNode,
    bins: Vec<u8>,
}

impl Graph {
    fn build() -> Result<Graph, JsValue> {
        let context = AudioContext::new()?;
        let volume = context.create_gain()?;
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

        volume.connect_with_audio_node(&preamp)?;
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
            volume,
            preamp,
            filters,
            analyser,
            bins,
        })
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

/// A decoded song.
#[derive(Clone)]
struct Loaded {
    mp3_id: i64,
    buffer: AudioBuffer,
}

/// A source playing, or scheduled to play, a decoded song.
struct Sounding {
    source: AudioBufferSourceNode,
    /// The context time the song's first sample plays, or would have played
    /// when the source started partway in.
    started_at: f64,
    /// The context time the song's last sample ends.
    ends_at: f64,
}

impl Sounding {
    fn silence(self) {
        let node: &AudioScheduledSourceNode = self.source.as_ref();
        node.set_onended(None);
        node.stop().ok();
        self.source.disconnect().ok();
    }
}

/// The song that follows the current one, decoded, and its source once it is
/// scheduled to start on the current song's last sample.
struct Upcoming {
    plan: Plan,
    loaded: Loaded,
    scheduled: Option<Sounding>,
}

struct Inner {
    graph: Graph,
    current: Option<Loaded>,
    sounding: Option<Sounding>,
    /// Where the current song resumes from while it is not sounding.
    offset: f64,
    upcoming: Option<Upcoming>,
    plan: Option<Plan>,
    /// Counts song loads, so a slow load that was replaced is dropped.
    generation: u64,
    /// Counts next-song loads the same way.
    next_generation: u64,
}

impl Inner {
    fn now(&self) -> f64 {
        self.graph.context.current_time()
    }

    fn duration(&self) -> f64 {
        self.current
            .as_ref()
            .map_or(0.0, |loaded| loaded.buffer.duration())
    }

    fn position(&self) -> f64 {
        let position = self
            .sounding
            .as_ref()
            .map_or(self.offset, |sounding| self.now() - sounding.started_at);

        position.clamp(0.0, self.duration())
    }

    /// A source for `buffer` that starts at context time `when`, `offset`
    /// seconds into the song, and reports its end to the engine.
    fn sound(&self, engine: Engine, buffer: &AudioBuffer, when: f64, offset: f64) -> Sounding {
        let source = self
            .graph
            .context
            .create_buffer_source()
            .expect("an open audio context makes buffer sources");
        source.set_buffer(Some(buffer));
        source
            .connect_with_audio_node(&self.graph.volume)
            .expect("a buffer source connects to the volume");

        let ended = Closure::once_into_js(move || engine.ended());
        let node: &AudioScheduledSourceNode = source.as_ref();
        node.set_onended(Some(ended.unchecked_ref()));
        source
            .start_with_when_and_grain_offset(when, offset)
            .expect("a new source starts");

        Sounding {
            source,
            started_at: when - offset,
            ends_at: when - offset + buffer.duration(),
        }
    }

    /// Silences the current song and its scheduled follower, keeping the
    /// position for a resume.
    fn halt(&mut self) {
        self.offset = self.position();

        if let Some(sounding) = self.sounding.take() {
            sounding.silence();
        }
        if let Some(scheduled) = self
            .upcoming
            .as_mut()
            .and_then(|upcoming| upcoming.scheduled.take())
        {
            scheduled.silence();
        }
    }

    /// Starts the current song from the kept position, then schedules the
    /// next one. While the current song is still loading this does nothing,
    /// and the load starts it.
    fn begin(&mut self, engine: Engine) {
        if let Some(loaded) = self.current.clone() {
            let now = self.now();
            self.sounding = Some(self.sound(engine, &loaded.buffer, now, self.offset));
            self.schedule(engine);
        }
    }

    /// Schedules the decoded next song to start on the current song's last
    /// sample, when the current song is sounding.
    fn schedule(&mut self, engine: Engine) {
        if let (Some(sounding), Some(upcoming)) = (&self.sounding, &self.upcoming) {
            let scheduled = self.sound(engine, &upcoming.loaded.buffer, sounding.ends_at, 0.0);
            self.upcoming
                .as_mut()
                .expect("the upcoming song was just read")
                .scheduled = Some(scheduled);
        }
    }

    /// Starts `mp3_id` from the beginning. A song already decoded as the
    /// current or the next one plays at once. Otherwise this returns the
    /// generation of the load to start.
    fn play(&mut self, engine: Engine, mp3_id: i64) -> Option<u64> {
        self.halt();
        self.offset = 0.0;
        self.generation += 1;
        self.next_generation += 1;
        self.plan = None;

        let upcoming = self.upcoming.take().map(|upcoming| upcoming.loaded);
        let decoded = [self.current.take(), upcoming]
            .into_iter()
            .flatten()
            .find(|loaded| loaded.mp3_id == mp3_id);

        match decoded {
            Some(loaded) => {
                self.current = Some(loaded);
                self.begin(engine);
                None
            }
            None => Some(self.generation),
        }
    }

    /// A load finished. It becomes the current song unless another song was
    /// played since, and starts when the player is playing.
    fn loaded(&mut self, engine: Engine, generation: u64, loaded: Loaded, playing: bool) {
        if generation == self.generation {
            self.current = Some(loaded);
            if playing {
                self.begin(engine);
            }
            self.fetch_next(engine);
        }
    }

    /// The song to follow the current one changed.
    fn prepare(&mut self, engine: Engine, plan: Option<Plan>) {
        if self.plan == plan {
            return;
        }

        self.plan = plan;
        self.next_generation += 1;
        if let Some(scheduled) = self.upcoming.take().and_then(|upcoming| upcoming.scheduled) {
            scheduled.silence();
        }

        self.fetch_next(engine);
    }

    /// Gets the next song ready once the current one is decoded, so the
    /// current song's download has the connection to itself. The current song
    /// is reused when it is also the next one.
    fn fetch_next(&mut self, engine: Engine) {
        if let (Some(plan), Some(current)) = (self.plan, self.current.clone()) {
            if current.mp3_id == plan.mp3_id {
                self.upcoming = Some(Upcoming {
                    plan,
                    loaded: current,
                    scheduled: None,
                });
                self.schedule(engine);
            } else {
                let context = self.graph.context.clone();
                let generation = self.next_generation;
                spawn_local(async move {
                    // A next song that fails to load is loaded again, and its
                    // failure shown, when the queue reaches it.
                    let loaded = load(context, plan.mp3_id).await.ok();
                    engine.with_inner(|inner| inner.prepared(engine, generation, plan, loaded));
                });
            }
        }
    }

    /// A next-song load finished. It is kept unless the plan changed while it
    /// loaded.
    fn prepared(&mut self, engine: Engine, generation: u64, plan: Plan, loaded: Option<Loaded>) {
        if let (true, Some(loaded)) = (generation == self.next_generation, loaded) {
            self.upcoming = Some(Upcoming {
                plan,
                loaded,
                scheduled: None,
            });
            self.schedule(engine);
        }
    }

    /// The current song played to its end. When the next song was scheduled
    /// it is already sounding and becomes the current one.
    fn ended(&mut self) -> Option<Plan> {
        self.sounding = None;
        self.offset = 0.0;

        match self.upcoming.take() {
            Some(Upcoming {
                plan,
                loaded,
                scheduled: Some(scheduled),
            }) => {
                self.current = Some(loaded);
                self.sounding = Some(scheduled);
                self.plan = None;
                Some(plan)
            }
            unscheduled => {
                self.upcoming = unscheduled;
                None
            }
        }
    }

    /// Forgets every song, for a stop with nothing loaded.
    fn clear(&mut self) {
        self.halt();
        self.generation += 1;
        self.current = None;
        self.upcoming = None;
        self.plan = None;
        self.offset = 0.0;
    }
}

/// Fetches and decodes a song. The browser trims the encoder delay and
/// padding that LAME records, so decoded songs join without silence.
async fn load(context: AudioContext, mp3_id: i64) -> Result<Loaded, ApiError> {
    let undecodable = |_: JsValue| ApiError::Failed("The song could not be played".to_string());

    let bytes = api::bytes(&paths::play(mp3_id)).await?;
    let data = Uint8Array::from(bytes.as_slice()).buffer();
    let promise = context.decode_audio_data(&data).map_err(undecodable)?;
    let decoded = JsFuture::from(promise).await.map_err(undecodable)?;

    Ok(Loaded {
        mp3_id,
        buffer: decoded.unchecked_into(),
    })
}

/// The player's audio: decodes songs, plays them through the equalizer and
/// the analyser, and starts each next song on the last sample of the one
/// before it.
#[derive(Clone, Copy)]
pub struct Engine {
    inner: StoredValue<Option<Inner>, LocalStorage>,
    pub transport: RwSignal<Transport>,
    pub duration: RwSignal<f64>,
    pub settings: RwSignal<Settings>,
    /// Set when a song ends, for the queue to move on.
    pub ended: RwSignal<Option<Ended>>,
    /// Set when a song cannot be loaded.
    pub failure: RwSignal<Option<ApiError>>,
}

impl Engine {
    pub fn new() -> Engine {
        let inner = Graph::build()
            .map_err(|error| leptos::logging::error!("audio is unavailable: {error:?}"))
            .ok()
            .map(|graph| Inner {
                graph,
                current: None,
                sounding: None,
                offset: 0.0,
                upcoming: None,
                plan: None,
                generation: 0,
                next_generation: 0,
            });

        Engine {
            inner: StoredValue::new_local(inner),
            transport: RwSignal::new(Transport::Stopped),
            duration: RwSignal::new(0.0),
            settings: RwSignal::new(stored_settings()),
            ended: RwSignal::new(None),
            failure: RwSignal::new(None),
        }
    }

    fn with_inner<R>(self, act: impl FnOnce(&mut Inner) -> R) -> Option<R> {
        self.inner
            .try_update_value(|inner| inner.as_mut().map(act))
            .flatten()
    }

    /// Whether the browser can play audio.
    pub fn available(self) -> bool {
        self.inner.with_value(Option::is_some)
    }

    /// Applies and keeps the equalizer settings, and resumes the audio
    /// context on clicks and key presses. The engine lasts as long as the
    /// page, so the listeners stay.
    pub fn run(self) {
        Effect::new(move |_| {
            let current = self.settings.get();
            self.with_inner(|inner| inner.graph.apply(&current));
            store_settings(&current);
        });

        let resume = move || {
            self.with_inner(|inner| inner.graph.resume());
        };
        let _ = window_event_listener(leptos::ev::pointerdown, move |_| resume());
        let _ = window_event_listener(leptos::ev::keydown, move |_| resume());
    }

    fn refresh_duration(self) {
        self.duration
            .set(self.with_inner(|inner| inner.duration()).unwrap_or(0.0));
    }

    /// Plays a song from the beginning, loading it first unless it is
    /// already decoded.
    pub fn play(self, mp3_id: i64) {
        self.transport.set(Transport::Playing);

        let load_needed = self
            .with_inner(|inner| {
                inner.graph.resume();
                inner
                    .play(self, mp3_id)
                    .map(|generation| (inner.graph.context.clone(), generation))
            })
            .flatten();
        self.refresh_duration();

        if let Some((context, generation)) = load_needed {
            spawn_local(async move {
                match load(context, mp3_id).await {
                    Ok(loaded) => {
                        let playing = self.transport.get_untracked() == Transport::Playing;
                        self.with_inner(|inner| inner.loaded(self, generation, loaded, playing));
                        self.refresh_duration();
                    }
                    Err(error) => {
                        self.transport.set(Transport::Stopped);
                        self.failure.set(Some(error));
                    }
                }
            });
        }
    }

    /// Stops and forgets every song.
    pub fn clear(self) {
        self.with_inner(Inner::clear);
        self.transport.set(Transport::Stopped);
        self.refresh_duration();
    }

    pub fn pause(self) {
        self.with_inner(Inner::halt);
        self.transport.set(Transport::Paused);
    }

    pub fn resume(self) {
        self.transport.set(Transport::Playing);
        self.with_inner(|inner| inner.begin(self));
    }

    /// Pauses a playing song and resumes a paused or stopped one.
    pub fn toggle_pause(self) {
        match self.transport.get_untracked() {
            Transport::Playing => self.pause(),
            Transport::Paused | Transport::Stopped => self.resume(),
        }
    }

    /// Stops and returns to the start of the song.
    pub fn stop(self) {
        self.with_inner(|inner| {
            inner.halt();
            inner.offset = 0.0;
        });
        self.transport.set(Transport::Stopped);
    }

    /// Moves to `seconds` into the song, and keeps playing when it was.
    pub fn seek(self, seconds: f64) {
        let playing = self.transport.get_untracked() == Transport::Playing;

        self.with_inner(|inner| {
            inner.halt();
            inner.offset = seconds.clamp(0.0, inner.duration());
            if playing {
                inner.begin(self);
            }
        });
    }

    /// Starts the song over and plays it.
    pub fn replay(self) {
        self.seek(0.0);
        self.resume();
    }

    pub fn position(self) -> f64 {
        self.with_inner(|inner| inner.position()).unwrap_or(0.0)
    }

    pub fn set_volume(self, level: f32) {
        self.with_inner(|inner| inner.graph.volume.gain().set_value(level));
    }

    /// Tells the engine which entry follows the current song, so it can
    /// decode it and schedule it to start on the current song's last sample.
    pub fn prepare(self, plan: Option<Plan>) {
        self.with_inner(|inner| inner.prepare(self, plan));
    }

    fn ended(self) {
        let started = self.with_inner(Inner::ended).flatten();
        self.refresh_duration();

        if started.is_none() {
            self.transport.set(Transport::Stopped);
        }

        let count = self
            .ended
            .with_untracked(|ended| ended.map_or(0, |ended| ended.count + 1));
        self.ended.set(Some(Ended { started, count }));
    }

    /// Runs `read` with the current frequency bins and sample rate, or does
    /// nothing when the browser cannot play audio.
    pub fn with_frequency_data(self, read: impl FnOnce(&[u8], f32)) {
        self.with_inner(|inner| {
            inner
                .graph
                .analyser
                .get_byte_frequency_data(&mut inner.graph.bins);
            read(&inner.graph.bins, inner.graph.context.sample_rate());
        });
    }
}
