//! Host pipelines are made by workers, not by the title thread that first
//! records a draw with them. Translating a shader and the host driver's
//! compile take from milliseconds to seconds each (structurisation of the
//! biggest compute programs ~0.4 s, some driver compiles 3.4 s), and
//! the title's first world frame wants over 1,200 new pipelines: made
//! on its recording threads, that frame took more than 30 s and the
//! title's GPU watchdog ended the run (decision 0007, "Pipelines made off
//! the title's threads").
//!
//! A first use queues the pipeline and waits for it within a budget per
//! frame, so the cheap ones (menus, one-off passes such as a colour-grading
//! bake) are there at once as before; past the budget a draw or dispatch
//! whose pipeline is not ready is dropped, as an untranslated one is.
//!
//! The default budget is half a second per title frame. A pass the title
//! runs only once and whose pipeline takes longer is lost. Passes over the
//! screen (no vertex input, no depth) have workers and a longer budget of
//! their own ([`screen_budget`]): the title fills its lighting tables with
//! such draws once, at the world's first frame, and behind a world's worth of
//! pipelines they were dropped; that run had no lamps and no diffuse light to
//! its end (run textures-d2, the first after a new translation; Belobug
//! 18e57db8 found the same).

use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::Shader;
use super::host::{GraphicsState, HostPipeline, TargetFormats};

/// What a host pipeline is made from, and the host pipeline once made.
pub(crate) struct Pending {
    shaders: Vec<Arc<Shader>>,
    pub(crate) graphics: Option<Box<GraphicsState>>,
    queue_state: AtomicU8,
    /// Taken by a worker from [`Queue::slow`].
    was_slow: AtomicBool,
    /// When it was first queued, and the list a worker took it from.
    queued: OnceLock<Instant>,
    taken_from: AtomicU8,
    made: OnceLock<Option<HostPipeline>>,
    /// What identifies it across runs (shader containers and state), and
    /// whether a draw or dispatch has requested it yet.
    key: String,
    used: AtomicBool,
    /// A pass over the screen ([`GraphicsState::screen_pass`]).
    screen: bool,
    /// Whether draws already waited [`GIVE_UP`] for it in vain: later
    /// draws do not wait for it (see [`wait`]).
    given_up: AtomicBool,
    /// What draws waited for it in vain so far, in microseconds.
    waited_us: AtomicU64,
    /// The title frame in which draws first waited for it (`u64::MAX`: none
    /// yet). Draws of later frames do not wait for it ([`claim_wait`]).
    waited_frame: AtomicU64,
    /// The formats its state declares for what a draw draws into (zeros
    /// without a graphics state).
    pub(crate) formats: TargetFormats,
    /// The same shaders and state for other formats, bound at a draw
    /// ([`for_formats`]): a chain from the title's pipeline, one link per
    /// set of formats met, never shortened.
    variant: OnceLock<Arc<Pending>>,
}

// SAFETY: D3D12 objects are free-threaded; the graphics state (whose input
// elements point into its own names) is only read after creation.
unsafe impl Send for Pending {}
// SAFETY: as above.
unsafe impl Sync for Pending {}

/// The light in the water's volume, in tiles (`water_shading:CS_TransmissionVolumetrics_Tiled`
/// and `_Tiled_LocalLights`, the title's 1.0.2.1): programs of one 64-lane wave a group, which
/// a host of 32-lane waves gets only in the wave-wide form (4 and 7 branches taken wave-wide).
/// In that form they leave whole tiles of open water dark - around boats, along shores, under
/// bridges - and cost about 15 ms of the card a frame.
///
/// Taken from VIctory-NVIDIA (`crates/victory-gpu/src/pipeline/make.rs`, GPL-3.0-or-later),
/// where it was measured on an RTX 5070: 25 fps with them, 30 without, the blocks gone, the
/// water darker and bluer. Theirs leaves them out; ours keeps them unless told otherwise,
/// because a switch that changes the picture is not a default here - a run with correct water
/// and no dark blocks is a different picture, not a strictly better one.
///
/// `VICTORY_WATER_LIGHT=0` leaves the two out (the 15 ms and the blocks go with them);
/// unset or `1` keeps them, exactly as upstream.
const WATER_LIGHT: [&str; 2] = ["5dc3b25986622949", "6f225b641c5b0708"];

/// Whether the water's light is kept (`VICTORY_WATER_LIGHT`, unset: kept).
fn water_light_kept() -> bool {
    static KEPT: OnceLock<bool> = OnceLock::new();
    *KEPT.get_or_init(|| std::env::var("VICTORY_WATER_LIGHT").map_or(true, |v| v.trim() != "0"))
}

fn water_light_left_out(shaders: &[Arc<Shader>]) -> bool {
    !water_light_kept()
        && victory_shader::emit::host_settings()[0] == 32
        && shaders.iter().any(|s| WATER_LIGHT.contains(&s.hash.as_str()))
}

/// Why a pipeline is never made, if it is left out on purpose.
fn left_out(shaders: &[Arc<Shader>]) -> Option<&'static str> {
    if water_light_left_out(shaders) {
        return Some("the water's tiled light on a host of 32-lane waves (dark blocks, ~15 ms of the card a frame; VICTORY_WATER_LIGHT=1 keeps it)");
    }
    None
}

impl Pending {
    pub(crate) fn new(shaders: Vec<Arc<Shader>>, graphics: Option<Box<GraphicsState>>) -> Pending {
        let mut key = blake3::Hasher::new();
        for s in &shaders {
            key.update(s.stage.as_bytes());
            key.update(s.hash.as_bytes());
        }
        key.update(graphics.as_ref().map(|g| g.describe()).unwrap_or_default().as_bytes());
        let key = key.finalize().to_hex()[..32].to_owned();
        // Left out: never queued, never made, and draws or dispatches of it are
        // dropped the way an untranslated shader's are - one `made` of `None`,
        // set here so a worker cannot take it either.
        let made = OnceLock::new();
        if let Some(why) = left_out(&shaders) {
            let _ = made.set(None);
            tracing::info!(%key, why, "a pipeline is left out");
        }
        Pending {
            screen: graphics.as_ref().is_some_and(|g| g.screen_pass()),
            formats: graphics.as_ref().map_or([0; 9], |g| g.formats()),
            variant: OnceLock::new(),
            shaders,
            graphics,
            queue_state: AtomicU8::new(NOT_QUEUED),
            was_slow: AtomicBool::new(false),
            queued: OnceLock::new(),
            taken_from: AtomicU8::new(TAKEN_BY_TITLE),
            made,
            key,
            used: AtomicBool::new(false),
            given_up: AtomicBool::new(false),
            waited_us: AtomicU64::new(0),
            waited_frame: AtomicU64::new(u64::MAX),
        }
    }

    pub(super) fn key(&self) -> &str {
        &self.key
    }

    fn make(&self) {
        self.make_with(|| HostPipeline::new(&self.shaders, self.graphics.as_deref()));
    }

    fn make_with(&self, build: impl FnOnce() -> Option<HostPipeline>) {
        self.made.get_or_init(|| {
            let run = super::spent::begin();
            let began = Instant::now();
            // A failed translation must complete this queue entry and wake its waiters.
            let made = match catch_unwind(AssertUnwindSafe(build)) {
                Ok(pipeline) => pipeline,
                Err(_) => {
                    tracing::warn!(shaders = ?self.shaders, "host pipeline build panicked");
                    None
                }
            };
            super::library::note_time(&self.key, began.elapsed().as_millis() as u64);
            let from = ["title", "screen", "requested", "early", "slow"][self.taken_from.load(Ordering::Relaxed) as usize];
            let queued = self.queued.get().map(|at| began.saturating_duration_since(*at));
            super::spent::end(run, &self.key, queued, from, made.is_some(), queues);
            made
        });
        let (lock, done) = &DONE;
        let _held = lock.lock().unwrap_or_else(|e| e.into_inner());
        done.notify_all();
    }
}

/// Why a pipeline has no host pipeline (diagnostics): its shaders that do
/// not translate, else "refused" or "being made"; `None` if it has one.
#[cfg(feature = "diagnostics")]
pub(crate) fn missing(p: &Pending) -> Option<String> {
    match p.made.get() {
        Some(Some(_)) => None,
        None => Some("being made".into()),
        Some(None) => {
            let failed: Vec<String> = p
                .shaders
                .iter()
                .filter(|s| matches!(s.translated.get(), Some(None)))
                .map(|s| format!("{} {}", s.stage, s.hash))
                .collect();
            Some(if failed.is_empty() {
                format!("refused {:?}", p.shaders)
            } else {
                failed.join(" + ")
            })
        }
    }
}

/// The hashes of the shaders `p` is made of.
pub(crate) fn hashes(p: &Pending) -> String {
    let hashes: Vec<&str> = p.shaders.iter().map(|s| s.hash.as_str()).collect();
    hashes.join("+")
}

/// Signalled whenever a worker has made a pipeline.
static DONE: (Mutex<()>, Condvar) = (Mutex::new(()), Condvar::new());

/// Waiting allowed per title frame, over all threads.
fn wait_budget() -> Duration {
    static BUDGET: OnceLock<Duration> = OnceLock::new();
    *BUDGET.get_or_init(|| {
        let millis = std::env::var("VICTORY_PIPELINE_WAIT_MS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(500);
        tracing::info!(millis, "host pipeline wait budget per frame");
        Duration::from_millis(millis)
    })
}

/// As [`wait_budget`], for passes over the screen (`VICTORY_SCREEN_WAIT_MS`,
/// 10 s; 0: like the others). Below the title's 30 s watchdog.
fn screen_budget() -> Duration {
    static BUDGET: OnceLock<Duration> = OnceLock::new();
    *BUDGET.get_or_init(|| {
        let millis = std::env::var("VICTORY_SCREEN_WAIT_MS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(10_000);
        Duration::from_millis(millis)
    })
}

/// Whether passes over the screen go before the others.
fn screens_first() -> bool {
    !screen_budget().is_zero()
}

/// Pipelines queued and not made yet.
static WAITING: AtomicU64 = AtomicU64::new(0);

/// Pipelines queued and not made yet (for the frame line).
pub(crate) fn waiting() -> u64 {
    WAITING.load(Ordering::Relaxed)
}

/// The host pipeline if it is made (or is made within the frame's budget);
/// queues it on first use.
pub(crate) fn ready(p: &Arc<Pending>) -> Option<&HostPipeline> {
    // A still-pending pipeline was requested by the title too. Prewarm it
    // on the next launch, even if this run never got to use it.
    if !p.used.load(Ordering::Relaxed) && !p.used.swap(true, Ordering::Relaxed) {
        super::library::note_used(&p.key);
    }
    if p.made.get().is_none() {
        start(p, REQUESTED);
        let frame = crate::frame::current();
        if p.graphics.is_none() && compute_waits() && compute_wait_left(frame) {
            // Made here, or waited for while a worker makes it.
            let began = Instant::now();
            p.make();
            compute_waited(frame, began.elapsed());
        } else {
            wait(p, frame);
            if p.made.get().is_none() {
                dropped(p, frame);
            }
        }
    }
    p.made.get()?.as_ref()
}

/// Whether a dispatch waits for its compute pipeline until it is made,
/// whatever the frame's budget ([`wait_budget`]); a draw still waits within
/// the budget and is dropped while its pipeline is not there.
///
/// A dropped draw is drawn again in the next frame. A dropped dispatch may
/// not be: the title runs some compute passes once, when a scene changes
/// (a buffer reset or cleared), and what they leave is read for as long as
/// the scene lasts. In the prison corridor a compute program met for the
/// first time took 4.4 s to make on a 16 GB machine; its one dispatch was
/// dropped after the half second of budget, and the windows of the room
/// before stayed blended over the whole picture for minutes (run 195).
/// A wait of seconds once (the pipeline library has it on the next run)
/// against a picture wrong until the scene ends.
/// Off by default until a cold and a warm run show what it costs;
/// `VICTORY_PIPELINE_COMPUTE_WAIT=1`: on.
fn compute_waits() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_COMPUTE_WAIT").is_some_and(|v| v == "1"))
}

/// What a frame may spend making compute pipelines for its dispatches
/// (`VICTORY_PIPELINE_COMPUTE_WAIT_MS`, 5 s; 0: no cap). A cold library can
/// meet dozens of new compute programs in one frame, and the title's
/// watchdog fires at 30 s. Once a frame has spent it, its dispatches wait
/// within the frame's budget as draws do; the one pipeline being made when
/// the cap runs out still finishes.
fn compute_wait_cap() -> Duration {
    static CAP: OnceLock<Duration> = OnceLock::new();
    *CAP.get_or_init(|| {
        let millis = std::env::var("VICTORY_PIPELINE_COMPUTE_WAIT_MS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(5_000);
        Duration::from_millis(millis)
    })
}

/// Frame and time spent in it making compute pipelines for dispatches.
static COMPUTE_SPENT: Mutex<(u64, Duration)> = Mutex::new((u64::MAX, Duration::ZERO));

fn compute_wait_left(frame: u64) -> bool {
    let cap = compute_wait_cap();
    if cap.is_zero() {
        return true;
    }
    let mut spent = COMPUTE_SPENT.lock().unwrap_or_else(|e| e.into_inner());
    if spent.0 != frame {
        *spent = (frame, Duration::ZERO);
    }
    spent.1 < cap
}

fn compute_waited(frame: u64, took: Duration) {
    let cap = compute_wait_cap();
    let mut spent = COMPUTE_SPENT.lock().unwrap_or_else(|e| e.into_inner());
    if spent.0 != frame {
        *spent = (frame, Duration::ZERO);
    }
    let before = spent.1;
    spent.1 += took;
    if !cap.is_zero() && before < cap && spent.1 >= cap {
        tracing::info!(
            frame,
            ms = spent.1.as_millis() as u64,
            "a frame spent its compute wait: its other dispatches wait within the frame's budget (VICTORY_PIPELINE_COMPUTE_WAIT_MS)"
        );
    }
}

/// The pipelines a frame's draws were dropped for, logged when the next
/// frame drops one (at most 4 lines a second, `VICTORY_PIPELINE_DROP_LOG=0`:
/// none). play-g's binoculars dropped 550-930 draws a frame for 30 frames
/// and the zoom came and went; with a warm cache these say which ones.
#[derive(Default)]
struct Dropped {
    frame: u64,
    draws: u64,
    pipelines: std::collections::HashSet<usize>,
    keys: Vec<String>,
    given_up: usize,
    logged_at: Option<Instant>,
    lines: u32,
    skipped: u64,
}

fn dropped(p: &Arc<Pending>, frame: u64) {
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_DROP_LOG").is_none_or(|v| v != "0")) {
        return;
    }
    static DROPPED: Mutex<Option<Dropped>> = Mutex::new(None);
    let mut d = DROPPED.lock().unwrap_or_else(|e| e.into_inner());
    let d = d.get_or_insert_with(Dropped::default);
    if d.frame != frame {
        if d.draws > 0 {
            let now = Instant::now();
            if d.logged_at.is_none_or(|at| now - at >= Duration::from_secs(1)) {
                d.logged_at = Some(now);
                d.lines = 0;
            }
            if d.lines < 4 {
                d.lines += 1;
                tracing::info!(
                    frame = d.frame,
                    draws = d.draws,
                    pipelines = d.pipelines.len(),
                    given_up = d.given_up,
                    frames_not_logged = d.skipped,
                    keys = %d.keys.join(" "),
                    "draws dropped for pipelines not made yet"
                );
                d.skipped = 0;
            } else {
                d.skipped += 1;
            }
        }
        d.frame = frame;
        d.draws = 0;
        d.pipelines.clear();
        d.keys.clear();
        d.given_up = 0;
    }
    d.draws += 1;
    if d.pipelines.insert(Arc::as_ptr(p) as usize) {
        if p.given_up.load(Ordering::Relaxed) {
            d.given_up += 1;
        }
        if d.keys.len() < 16 {
            d.keys.push(p.key.clone());
        }
    }
}

/// Pipelines made again for the formats bound at a draw, since the start.
static VARIANTS: AtomicU64 = AtomicU64::new(0);

/// Whether a draw into views of other formats than its pipeline declares
/// gets a host pipeline for those formats ([`for_formats`]). On by
/// default; `VICTORY_TARGET_VARIANTS=0` sends such a draw to the host with
/// the title's pipeline, as before.
fn variants_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_TARGET_VARIANTS").is_none_or(|v| v != "0"))
}

/// The host pipeline for a draw with `p` into views of the formats
/// `bound`, which `p` does not declare (`host::formats_differ`): that of
/// the same shaders and state for those formats. It is a pipeline like the
/// title's own: queued at its first draw, made by a worker, waited for
/// within the frame's budget, and the draw is dropped while it is not
/// there ([`ready`]).
#[cold]
pub(crate) fn for_formats<'a>(p: &'a Arc<Pending>, bound: &TargetFormats) -> Option<&'a HostPipeline> {
    match p.graphics.as_deref() {
        Some(g) if variants_on() => ready(variant(p, g, &super::host::formats_for(&p.formats, bound))),
        _ => ready(p),
    }
}

/// The pipeline of `p`'s shaders and its state `g` for the formats
/// `bound`: found in the chain that starts at `p`, or added to it (logged
/// then: once per pipeline and set of formats).
fn variant<'a>(p: &'a Arc<Pending>, g: &GraphicsState, bound: &TargetFormats) -> &'a Arc<Pending> {
    let mut at = p;
    // (`bound` is what `host::formats_for` gives, and a state made for it
    // declares exactly it, so the walk ends at the link made for it; the
    // count is a bound all the same: past it the title's own pipeline.)
    for _ in 0..64 {
        let next = at.variant.get_or_init(|| {
            let so_far = VARIANTS.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::info!(
                pipeline = ?p.shaders,
                declared = ?p.formats,
                bound = ?bound,
                so_far,
                "a pipeline is made again for the render target formats bound at its draw (eight render targets, then depth; VICTORY_TARGET_VARIANTS=0: the draw goes out with the title's pipeline)"
            );
            Arc::new(Pending::new(p.shaders.clone(), Some(Box::new(g.with_formats(bound)))))
        });
        // (A state made for `bound` declares exactly `bound`.)
        if next.formats == *bound {
            return next;
        }
        at = next;
    }
    p
}

/// Makes `p` on this thread if it is not made yet (waiting for a worker
/// that is making it); whether it has a host pipeline.
pub(crate) fn now(p: &Arc<Pending>) -> bool {
    p.make();
    p.made.get().is_some_and(|made| made.is_some())
}

/// Starts making `p` now, without waiting for it.
pub(crate) fn begin(p: &Arc<Pending>) {
    if !p.shaders.is_empty() {
        start(p, REQUESTED);
    }
}

/// Whether `p`'s host pipeline is made (`Some(true)`), refused or not
/// translated (`Some(false)`), or not yet (`None`); never waits.
pub(crate) fn state(p: &Pending) -> Option<bool> {
    p.made.get().map(Option::is_some)
}

/// At the title's creation of a pipeline: starts it at once if an earlier
/// run drew with it (during loading, so the world's first frames find it).
pub(crate) fn early(p: &Arc<Pending>) {
    if !p.shaders.is_empty() && super::library::wanted_early(&p.key) {
        super::library::EARLY.fetch_add(1, Ordering::Relaxed);
        start(p, EARLY);
    }
}

/// Whether the frame's budget is one for all threads together: a thread
/// that waits takes what is left of it and gives back what it did not
/// use. Counted only when a wait had ended, the budget was one per thread:
/// each of the title's five recording threads began its first wait of a
/// frame with nothing counted yet and waited the whole half second, 2.5 s
/// of waiting in every frame in which pipelines were being made (runs
/// 34-49: "2504 ms waiting for pipelines" in each such frame).
/// On by default; `VICTORY_PIPELINE_WAIT_SHARED=0` counts as before.
fn shared_budget() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_WAIT_SHARED").is_none_or(|v| v != "0"))
}

/// How long draws may wait for a pipeline in vain, all frames together,
/// before later draws stop waiting for it. A pipeline the driver takes seconds for otherwise
/// costs every frame the whole budget: in play-g's drive into the world
/// the frames waited 470-500 ms each for three minutes while pipelines
/// took 6-16 s each. Given up, it is still made in the background and its
/// draws are dropped until then, as they are once the budget is spent.
/// Counted over frames: in play-g's minute 27 the binoculars' pipelines
/// took the 500 ms of 30 frames in a row, most frames sharing it among
/// several, so no single wait reached 100 ms. Passes over the screen keep
/// waiting. `VICTORY_PIPELINE_GIVE_UP=0` waits as before.
const GIVE_UP: Duration = Duration::from_millis(100);

fn give_up() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_GIVE_UP").is_none_or(|v| v != "0"))
}

/// Whether a pipeline is waited for in one frame only: one the worker has
/// not made within that frame's budget (a slow or stuck driver compile) has
/// its draws dropped in later frames until it is made, instead of taking
/// what is left of the budget again in every frame that draws with it
/// (fxceless/victory-fork #1234, for its #1181: 500 ms waited in every world
/// frame, felt as a freeze while the player moves). [`GIVE_UP`] stops the
/// waits once they add up to 100 ms; this stops them after the first frame,
/// however short its wait. Passes over the screen keep waiting.
/// `VICTORY_PIPELINE_WAIT_EVERY_FRAME=1`: as before.
fn wait_every_frame() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_WAIT_EVERY_FRAME").is_some_and(|v| v == "1"))
}

/// Claims `frame` as the one in which draws wait for `p`: whether it is (or
/// already was) that frame.
fn claim_wait(p: &Pending, frame: u64) -> bool {
    if wait_every_frame() {
        return true;
    }
    match p.waited_frame.compare_exchange(u64::MAX, frame, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => true,
        Err(first) => first == frame,
    }
}

/// Waits for `p` while frame `frame` has budget left, unless an earlier
/// wait for it gave up or an earlier frame waited for it.
fn wait(p: &Pending, frame: u64) {
    let screen = p.screen && screens_first();
    if !screen && p.given_up.load(Ordering::Relaxed) {
        return;
    }
    let budget = if screen { screen_budget() } else { wait_budget() };
    if budget.is_zero() {
        return;
    }
    // The frame the budget belongs to, and what was waited of it (or is
    // taken by a thread waiting now); of the passes over the screen too.
    static SPENT: [Mutex<(u64, Duration)>; 2] = [const { Mutex::new((u64::MAX, Duration::ZERO)) }; 2];
    let spent_of = &SPENT[usize::from(screen)];
    let shared = shared_budget();
    let left = {
        let mut spent = spent_of.lock().unwrap_or_else(|e| e.into_inner());
        if spent.0 != frame {
            *spent = (frame, Duration::ZERO);
        }
        let left = budget.saturating_sub(spent.1);
        // Claimed only with time left, so a pipeline first drawn with once
        // the frame's budget is spent keeps its one wait for a later frame.
        if left.is_zero() || (!screen && !claim_wait(p, frame)) {
            return;
        }
        if shared {
            spent.1 += left;
        }
        left
    };
    let started = Instant::now();
    let (lock, done) = &DONE;
    let mut held = lock.lock().unwrap_or_else(|e| e.into_inner());
    while p.made.get().is_none() {
        let rest = left.saturating_sub(started.elapsed());
        if rest.is_zero() {
            break;
        }
        held = done.wait_timeout(held, rest).unwrap_or_else(|e| e.into_inner()).0;
    }
    drop(held);
    let waited = started.elapsed().min(left);
    if !screen && p.made.get().is_none() && give_up() && waited_in_vain(p, waited) {
        tracing::info!(
            ms = p.waited_us.load(Ordering::Relaxed) / 1000,
            key = %p.key,
            "a pipeline waited for in vain: later draws do not wait for it (VICTORY_PIPELINE_GIVE_UP=0: they do)"
        );
    }
    if screen && waited > Duration::from_millis(500) {
        tracing::info!(
            ms = waited.as_millis() as u64,
            made = p.made.get().is_some(),
            shaders = ?p.shaders,
            "a pass over the screen was waited for (VICTORY_SCREEN_WAIT_MS=0: dropped past half a second a frame)"
        );
    }
    {
        let mut spent = spent_of.lock().unwrap_or_else(|e| e.into_inner());
        // A frame that began meanwhile has its own budget.
        if spent.0 == frame {
            if shared {
                spent.1 = spent.1.saturating_sub(left - waited);
            } else {
                spent.1 += waited;
            }
        }
    }
    #[cfg(feature = "diagnostics")]
    crate::copystat::add(
        crate::copystat::C::PipelineWaitUs,
        started.elapsed().as_micros() as u64,
    );
}

/// Lists for [`Pending::taken_from`].
const TAKEN_BY_TITLE: u8 = 0;
/// Adds `waited` to what draws waited for `p` in vain: whether that has now
/// reached [`GIVE_UP`] (true once).
fn waited_in_vain(p: &Pending, waited: Duration) -> bool {
    let us = waited.as_micros() as u64;
    let total = p.waited_us.fetch_add(us, Ordering::Relaxed) + us;
    total >= GIVE_UP.as_micros() as u64 && !p.given_up.swap(true, Ordering::Relaxed)
}

const TAKEN_SCREEN: u8 = 1;
const TAKEN_REQUESTED: u8 = 2;
const TAKEN_EARLY: u8 = 3;
const TAKEN_SLOW: u8 = 4;

const NOT_QUEUED: u8 = 0;
const EARLY: u8 = 1;
const REQUESTED: u8 = 2;
const MAKING: u8 = 3;

/// How many workers may make pipelines at once that are known to take long
/// and that no draw has asked for: a quarter of them. On a fresh cache (a
/// change of the translator or the driver) the early list begins with
/// programs that take tens of seconds each: all 16 workers were in them
/// through the whole of the title's loading, 97 pipelines made in 20 s, and
/// the first minute and a half of the world ran at 0.5 to 1.6 s a frame,
/// dropping up to 2,900 draws a frame (run shaders-1).
/// `VICTORY_PIPELINE_SLOW=0`: in the order the title creates them, as before.
fn slow_limit() -> usize {
    static LIMIT: OnceLock<usize> = OnceLock::new();
    *LIMIT.get_or_init(|| {
        if std::env::var_os("VICTORY_PIPELINE_SLOW").is_some_and(|v| v == "0") {
            usize::MAX
        } else {
            (workers_wanted() / 4).max(1)
        }
    })
}

/// How long nothing but known slow pipelines must have been queued before
/// [`quiet_slow_limit`] holds.
const QUIET: Duration = Duration::from_secs(2);

/// [`slow_limit`] while the title queues nothing else: every worker. In
/// play-g the early list ran out at 06:28 and for four minutes 3 to 5 of the
/// 16 workers were busy, all on the slow list (854 waiting), while the rest
/// idled; the drive at 06:32 then asked for 519 of them at once, and draws
/// waited 500 ms a frame. On the first run after a translator VERSION
/// (play-cold, 24 workers) the title loaded with nothing but the slow list
/// queued, 868 of them, and with three quarters 15 to 18 workers were busy
/// for 140 s while the other 6 idled. Cold starts with every worker against
/// three quarters (sc2 and sc8 against sc1 and sc7): 51k and 65k draws
/// dropped in the first 340 s against 89k and 87k, and in sc8 the slow list
/// was made by 245 s (create_ms summed 8871 s against 24291 s, 22 made in
/// vain against 260) while sc7 still had 230 being made at 565 s.
/// `VICTORY_PIPELINE_SLOW_QUIET=3/4`: three quarters, as before. `=0`: a
/// quarter at all times.
fn quiet_slow_limit() -> usize {
    static LIMIT: OnceLock<usize> = OnceLock::new();
    *LIMIT.get_or_init(|| {
        quiet_limit(std::env::var("VICTORY_PIPELINE_SLOW_QUIET").ok().as_deref(), workers_wanted(), slow_limit())
    })
}

fn quiet_limit(asked: Option<&str>, workers: usize, slow: usize) -> usize {
    match asked.map(str::trim) {
        Some("0") => slow,
        Some("3/4") => (workers * 3 / 4).max(slow),
        _ => workers.max(slow),
    }
}

/// `VICTORY_PIPELINE_SLOW_FIRST=1`: the slow list's longest first (by the
/// longest each took before), and those of [`LONG_MS`] or more made at
/// normal priority. In play-cold one compute program took 130 s to
/// translate: taken 11 s into the run behind shorter ones, made at 141 s,
/// the world started at 150 s. It had a processor 94% of the time, but
/// went from processor 7 to 30.
fn longest_first() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_SLOW_FIRST").is_some_and(|v| v == "1"))
}

const LONG_MS: u64 = 10_000;

/// The unclaimed entry of `list` that took longest by `ms` (the first among
/// equals); claimed ones are dropped from it.
fn longest(list: &mut VecDeque<Arc<Pending>>, ms: impl Fn(&str) -> u64) -> Option<Arc<Pending>> {
    list.retain(|p| p.queue_state.load(Ordering::Relaxed) == EARLY);
    let at = (0..list.len()).max_by_key(|&i| (ms(&list[i].key), std::cmp::Reverse(i)))?;
    list.remove(at)
}

/// Graphics pipelines are made before compute ones, and compute ones never
/// hold every worker (upstream #3025). In the world some 180 compute
/// pipelines take the driver 3.7 s each on average (up to 24 s); taken in
/// order of first use they held all workers for minutes while the scene's
/// graphics pipelines, made in milliseconds, waited behind them and their
/// draws were dropped (a 360 s run: ~870 queued, 500 to 1,000 draws a frame
/// dropped; with graphics first, 7 queued after three minutes and ~370
/// dropped). `VICTORY_PIPELINE_GRAPHICS_FIRST=0`: one list for both, as
/// before.
fn graphics_first() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_GRAPHICS_FIRST").is_none_or(|v| v != "0"))
}

/// Of the compute pipelines draws have asked for, the one the slow list says
/// is made soonest (one not on it first; in the order asked among equals).
/// In the 980073cbe run draws asked for 323: 107 of them took the driver
/// 20 s or more each (2,989 s in all) and the other 216 616 s in all, but in
/// the order asked they waited behind the long ones, half of them 160 s or
/// more, their dispatches dropped all the while.
/// `VICTORY_PIPELINE_QUICK_FIRST=0`: in the order asked, as before.
fn quickest(list: &mut VecDeque<Arc<Pending>>) -> Option<Arc<Pending>> {
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("VICTORY_PIPELINE_QUICK_FIRST").is_none_or(|v| v != "0")) {
        return list.pop_front();
    }
    let at = (0..list.len()).min_by_key(|&i| super::library::slow_ms(&list[i].key))?;
    list.remove(at)
}

struct Queue {
    /// Requested passes over the screen: before all others, and the only ones
    /// [`SCREEN_WORKERS`] take.
    screen: VecDeque<Arc<Pending>>,
    early: VecDeque<Arc<Pending>>,
    /// Those of the early ones known to take long, and how many of them are
    /// being made ([`slow_limit`]).
    slow: VecDeque<Arc<Pending>>,
    slow_now: usize,
    requested: VecDeque<Arc<Pending>>,
    /// Compute ones, after the graphics ones of the same list
    /// ([`graphics_first`]).
    requested_compute: VecDeque<Arc<Pending>>,
    early_compute: VecDeque<Arc<Pending>>,
    /// Compute pipelines being made, and how many may be: all workers but
    /// one, so one is always free for graphics.
    compute_now: usize,
    compute_cap: usize,
    /// When a pipeline last went into a list other than the slow one.
    others_at: Option<Instant>,
}

impl Queue {
    const fn new() -> Self {
        Self {
            screen: VecDeque::new(),
            early: VecDeque::new(),
            slow: VecDeque::new(),
            slow_now: 0,
            requested: VecDeque::new(),
            requested_compute: VecDeque::new(),
            early_compute: VecDeque::new(),
            compute_now: 0,
            compute_cap: usize::MAX,
            others_at: None,
        }
    }

    /// How many slow pipelines may be made at once now.
    fn slow_cap(&self) -> usize {
        if self.others_at.is_none_or(|at| at.elapsed() >= QUIET) { quiet_slow_limit() } else { slow_limit() }
    }

    /// Queues an unrequested pipeline or promotes one queued at creation.
    /// Returns whether it is new to the queue, or `None` if nothing changed.
    fn push(&mut self, p: &Arc<Pending>, priority: u8) -> Option<bool> {
        self.push_as(p, priority, slow_limit() != usize::MAX && super::library::slow(&p.key))
    }

    fn push_as(&mut self, p: &Arc<Pending>, priority: u8, slow: bool) -> Option<bool> {
        let prev = p.queue_state.load(Ordering::Relaxed);
        if prev >= priority {
            return None;
        }
        p.queue_state.store(priority, Ordering::Relaxed);
        p.queued.get_or_init(Instant::now);
        if priority == REQUESTED || !slow {
            self.others_at = Some(Instant::now());
        }
        let compute = p.graphics.is_none() && graphics_first();
        if priority == REQUESTED && p.screen && screens_first() {
            self.screen.push_back(p.clone());
        } else if priority == REQUESTED && compute {
            self.requested_compute.push_back(p.clone());
        } else if priority == REQUESTED {
            self.requested.push_back(p.clone());
        } else if slow {
            self.slow.push_back(p.clone());
        } else if compute {
            self.early_compute.push_back(p.clone());
        } else {
            self.early.push_back(p.clone());
        }
        Some(prev == NOT_QUEUED)
    }

    fn pop(&mut self, screens_only: bool, slow_limit: usize) -> Option<Arc<Pending>> {
        // An early entry can have been promoted and claimed already.
        let unclaimed = |list: &mut VecDeque<Arc<Pending>>| {
            std::iter::from_fn(|| list.pop_front()).find(|p| p.queue_state.load(Ordering::Relaxed) == EARLY)
        };
        let compute_free = self.compute_now < self.compute_cap;
        let mut p = self.screen.pop_front();
        let mut from = TAKEN_SCREEN;
        if p.is_none() && !screens_only {
            p = self.requested.pop_front();
            from = TAKEN_REQUESTED;
            if p.is_none() && compute_free {
                p = quickest(&mut self.requested_compute);
            }
            if p.is_none() {
                p = unclaimed(&mut self.early);
                from = TAKEN_EARLY;
            }
            if p.is_none() && compute_free {
                p = unclaimed(&mut self.early_compute);
            }
            if p.is_none() && self.slow_now < slow_limit {
                p = if longest_first() {
                    longest(&mut self.slow, super::library::slow_ms)
                } else {
                    unclaimed(&mut self.slow)
                };
                // A compute one waits its turn while compute holds all it may.
                if let Some(q) = p.take_if(|q| q.graphics.is_none() && !compute_free) {
                    self.slow.push_front(q);
                }
                from = TAKEN_SLOW;
                if let Some(p) = &p {
                    p.was_slow.store(true, Ordering::Relaxed);
                    self.slow_now += 1;
                }
            }
        }
        let p = p?;
        if p.graphics.is_none() {
            self.compute_now += 1;
        }
        p.taken_from.store(from, Ordering::Relaxed);
        p.queue_state.store(MAKING, Ordering::Relaxed);
        Some(p)
    }
}

static QUEUE: Mutex<Queue> = Mutex::new(Queue::new());
static AVAILABLE: Condvar = Condvar::new();

/// Start or promote work for a draw without waiting behind the entire early
/// list. Queue changes and worker claims happen under the same lock.
fn start(p: &Arc<Pending>, priority: u8) {
    if p.queue_state.load(Ordering::Relaxed) >= priority {
        return;
    }
    if !workers() {
        if p.queue_state.swap(MAKING, Ordering::Relaxed) == NOT_QUEUED {
            p.make();
        }
        return;
    }
    let mut queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(new) = queue.push(p, priority) {
        if new {
            PEAK.fetch_max(WAITING.fetch_add(1, Ordering::Relaxed) + 1, Ordering::Relaxed);
        }
        // (One woken could be a worker that takes passes over the screen
        // alone: it would leave the pipeline to nobody.)
        AVAILABLE.notify_all();
    }
}

/// Workers that make passes over the screen alone, besides the others.
const SCREEN_WORKERS: usize = 2;

fn next(screens_only: bool) -> Arc<Pending> {
    let mut queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        let cap = queue.slow_cap();
        if let Some(p) = queue.pop(screens_only, cap) {
            return p;
        }
        // (Woken again after QUIET: the slow list's limit can have risen.)
        queue = AVAILABLE.wait_timeout(queue, QUIET).unwrap_or_else(|e| e.into_inner()).0;
    }
}

/// How many workers make pipelines: `asked` (`VICTORY_PIPELINE_WORKERS`, 1 to
/// 64), else three quarters of the logical processors, 2 to 24. They run below
/// the title's own threads, and a cold cache queues hundreds at once. The
/// driver compiles each pipeline on one thread: the first run after a change
/// of the translator (980073cbe) had 7.5 ks of compute pipelines to make, at
/// least 470 s on 16 workers; with a warm library they wait idle (two thirds,
/// at most 16, before).
fn worker_count(asked: Option<usize>, processors: usize) -> usize {
    match asked {
        Some(n) => n.clamp(1, 64),
        None => (processors * 3 / 4).clamp(2, 24),
    }
}

/// [`worker_count`] here: the slow list's limits follow it too, so that
/// `VICTORY_PIPELINE_WORKERS` changes them as well (they took 16 whatever it
/// asked).
fn workers_wanted() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| {
        worker_count(
            std::env::var("VICTORY_PIPELINE_WORKERS").ok().and_then(|v| v.trim().parse().ok()),
            std::thread::available_parallelism().map_or(2, |n| n.get()),
        )
    })
}

/// Stack of each pipeline worker: room for the driver's shader compiler,
/// which runs on it (reserved address space; committed only as used).
const PIPELINE_STACK: usize = 64 << 20;

/// Starts the workers ([`worker_count`]) on first use.
fn workers() -> bool {
    static RUNNING: OnceLock<bool> = OnceLock::new();
    *RUNNING.get_or_init(|| {
        let mut running = false;
        let workers = workers_wanted();
        tracing::info!(workers, "pipeline workers");
        if graphics_first() {
            // With one worker only, compute has to take it, else none is made.
            QUEUE.lock().unwrap_or_else(|e| e.into_inner()).compute_cap = workers.max(2) - 1;
        }
        let screens = if screens_first() { SCREEN_WORKERS } else { 0 };
        for i in 0..workers + screens {
            let screens_only = i >= workers;
            let spawned = std::thread::Builder::new()
                .name(format!("victory-pipelines-{i}"))
                // The driver compiles pipelines on this thread, recursively:
                // NVIDIA's compiler (nvgpucomp64) overflowed the default 2 MB
                // on a world compute program and refused it as out of memory.
                .stack_size(PIPELINE_STACK)
                .spawn(move || {
                    // Below the title's own threads (loading, recording),
                    // and the streaming one, which is below normal itself
                    // (`VICTORY_PIPELINE_LOWEST=0`: level with it, as before).
                    use windows::Win32::System::Threading::*;
                    let lowest = !screens_only && std::env::var_os("VICTORY_PIPELINE_LOWEST").is_none_or(|v| v != "0");
                    let priority = if lowest { THREAD_PRIORITY_LOWEST } else { THREAD_PRIORITY_BELOW_NORMAL };
                    // SAFETY: a plain call on our own thread.
                    let set = |priority| unsafe {
                        let _ = SetThreadPriority(GetCurrentThread(), priority);
                    };
                    set(priority);
                    loop {
                        let p = next(screens_only);
                        let raise = longest_first();
                        let hold = crate::cores::hold_long_on_fast_cores();
                        let long = (raise || hold)
                            && p.was_slow.load(Ordering::Relaxed)
                            && super::library::slow_ms(&p.key) >= LONG_MS;
                        if long && raise {
                            set(THREAD_PRIORITY_NORMAL);
                        }
                        if long && hold {
                            crate::cores::hold_on_fast_cores(true);
                        }
                        BUSY.fetch_add(1, Ordering::Relaxed);
                        {
                            let _slot = ComputeSlot(p.graphics.is_none());
                            p.make();
                        }
                        BUSY.fetch_sub(1, Ordering::Relaxed);
                        if long && raise {
                            set(priority);
                        }
                        if long && hold {
                            crate::cores::hold_on_fast_cores(false);
                        }
                        if p.was_slow.load(Ordering::Relaxed) {
                            let mut queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
                            queue.slow_now -= 1;
                            AVAILABLE.notify_all();
                        }
                        if WAITING.fetch_sub(1, Ordering::Relaxed) == 1 {
                            drained();
                        } else {
                            super::library::save(false);
                        }
                    }
                });
            if let Err(e) = spawned {
                tracing::warn!(error = %e, "no pipeline worker");
            } else {
                running = true;
            }
        }
        // The workers save after a pipeline (at most every 20 s), so the
        // last of a burst waited for the next pipeline, and with none made
        // until the window was closed they were lost: the binoculars' in
        // play-g. Saves nothing when nothing was added.
        let saver = std::thread::Builder::new().name("victory-pipelines-save".into()).spawn(|| loop {
            std::thread::sleep(Duration::from_secs(20));
            super::library::save(false);
        });
        if let Err(e) = saver {
            tracing::warn!(error = %e, "no pipeline library saver");
        }
        running
    })
}

/// A compute pipeline's place in [`Queue::compute_now`], given back once it
/// is made, also when the driver's compiler panics (it would be kept for
/// good otherwise).
struct ComputeSlot(bool);

impl Drop for ComputeSlot {
    fn drop(&mut self) {
        if self.0 {
            let mut queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
            queue.compute_now -= 1;
            AVAILABLE.notify_all();
        }
    }
}

/// Workers making a pipeline now.
static BUSY: AtomicU64 = AtomicU64::new(0);

/// The lists' lengths and the busy workers, for the workers' summary line.
fn queues() -> super::spent::Queues {
    let queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    super::spent::Queues {
        lists: [
            queue.screen.len(),
            queue.requested.len() + queue.requested_compute.len(),
            queue.early.len() + queue.early_compute.len(),
            queue.slow.len(),
        ],
        busy: BUSY.load(Ordering::Relaxed) as usize,
    }
}

/// The most pipelines waiting at once since the workers last ran out.
static PEAK: AtomicU64 = AtomicU64::new(0);

/// The workers ran out of work: logged after a batch of 10 or more, and
/// the cache saved (at most every 20 s).
fn drained() {
    use super::library::{EARLY, LOADED, MADE};
    if PEAK.swap(0, Ordering::Relaxed) >= 10 {
        tracing::info!(
            made = MADE.load(Ordering::Relaxed),
            loaded = LOADED.load(Ordering::Relaxed),
            started_early = EARLY.load(Ordering::Relaxed),
            "host pipelines all made"
        );
    }
    super::library::save(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panicking_pipeline_is_refused_and_the_worker_continues() {
        let bad = Arc::new(Pending::new(Vec::new(), None));
        let good = Arc::new(Pending::new(Vec::new(), None));
        let failed = bad.clone();
        let next = good.clone();
        let worker = std::thread::spawn(move || {
            failed.make_with(|| panic!("synthetic pipeline build panic"));
            next.make_with(|| None);
        });

        worker.join().expect("the pipeline worker should survive");
        assert!(matches!(bad.made.get(), Some(None)));
        assert!(matches!(good.made.get(), Some(None)));

        bad.make_with(|| panic!("the refused pipeline should stay complete"));
    }

    #[test]
    fn worker_count_follows_the_processors_and_the_switch() {
        assert_eq!(worker_count(None, 1), 2);
        assert_eq!(worker_count(None, 24), 18);
        assert_eq!(worker_count(None, 32), 24);
        assert_eq!(worker_count(None, 64), 24);
        assert_eq!(worker_count(Some(0), 24), 1);
        assert_eq!(worker_count(Some(20), 24), 20);
    }

    /// A pipeline one wait gave up on is not waited for again: nothing makes
    /// it here, so the first draw waits the frame's budget for it, and a
    /// draw of a later frame not at all.
    #[test]
    fn a_pipeline_waited_for_in_vain_is_not_waited_for_again() {
        let p = Pending::new(Vec::new(), None);
        // Frames of its own, so the first has its whole budget.
        let frame = u64::MAX - 7;
        let started = Instant::now();
        wait(&p, frame);
        assert!(started.elapsed() >= GIVE_UP, "{:?}", started.elapsed());
        assert!(p.given_up.load(Ordering::Relaxed));
        let again = Instant::now();
        wait(&p, frame - 1);
        assert!(again.elapsed() < GIVE_UP / 10, "{:?}", again.elapsed());
    }

    /// A pipeline an earlier frame waited for is not waited for again, even
    /// if that wait was short (fxceless/victory-fork #1234).
    #[test]
    fn a_pipeline_is_waited_for_in_one_frame_only() {
        let p = Pending::new(Vec::new(), None);
        let frame = u64::MAX - 17;
        p.waited_frame.store(frame - 1, Ordering::Relaxed);
        let started = Instant::now();
        wait(&p, frame);
        assert!(started.elapsed() < GIVE_UP / 10, "{:?}", started.elapsed());
        assert!(!p.given_up.load(Ordering::Relaxed));
    }

    /// The first frame keeps the claim, and its other draws may wait too.
    #[test]
    fn the_first_frame_claims_the_wait() {
        let p = Pending::new(Vec::new(), None);
        assert!(claim_wait(&p, 7));
        assert!(claim_wait(&p, 7));
        assert!(!claim_wait(&p, 8));
        assert_eq!(p.waited_frame.load(Ordering::Relaxed), 7);
    }

    /// Short waits in vain add up over frames until draws stop waiting.
    #[test]
    fn short_waits_in_vain_add_up() {
        let p = Pending::new(Vec::new(), None);
        let short = GIVE_UP / 3;
        assert!(!waited_in_vain(&p, short));
        assert!(!waited_in_vain(&p, short));
        assert!(!p.given_up.load(Ordering::Relaxed));
        assert!(waited_in_vain(&p, short * 2));
        assert!(p.given_up.load(Ordering::Relaxed));
        assert!(!waited_in_vain(&p, short));
    }

    /// A first use queues the pipeline and, within the budget, gets the
    /// worker's answer (here: no host pipeline for no shaders).
    #[test]
    fn first_use_is_made_by_a_worker() {
        let p = Arc::new(Pending::new(Vec::new(), None));
        assert!(ready(&p).is_none());
        assert_ne!(p.queue_state.load(Ordering::Relaxed), NOT_QUEUED);
        let started = Instant::now();
        while p.made.get().is_none() && started.elapsed() < Duration::from_secs(10) {
            std::thread::yield_now();
        }
        assert!(matches!(p.made.get(), Some(None)));
    }

    /// A draw into other formats than its pipeline declares finds the
    /// pipeline for them, which exists once per set of formats and is
    /// queued by its first draw like any other.
    #[test]
    fn a_pipeline_for_bound_formats_is_made_once() {
        // The title's pipeline declares one render target, format 24.
        let p = Arc::new(Pending::new(Vec::new(), Some(Box::new(GraphicsState::declaring(24)))));
        assert_eq!(p.formats, [24, 0, 0, 0, 0, 0, 0, 0, 0]);
        let g = p.graphics.as_deref().unwrap();
        let (a, b) = ([26, 0, 0, 0, 0, 0, 0, 0, 0], [28, 0, 0, 0, 0, 0, 0, 0, 0]);
        let first = variant(&p, g, &a);
        assert_eq!(first.formats, a);
        assert_ne!(first.key, p.key);
        assert!(Arc::ptr_eq(first, variant(&p, g, &a)));
        let second = variant(&p, g, &b);
        assert_eq!(second.formats, b);
        assert!(!Arc::ptr_eq(first, second));
        assert_ne!(second.key, first.key);
        assert!(Arc::ptr_eq(first, variant(&p, g, &a)));
        assert!(Arc::ptr_eq(second, variant(&p, g, &b)));
        assert_eq!(first.queue_state.load(Ordering::Relaxed), NOT_QUEUED);
        assert_eq!(p.queue_state.load(Ordering::Relaxed), NOT_QUEUED);
    }

    #[test]
    fn pending_pipeline_is_recorded_for_the_next_launch() {
        let mut p = Pending::new(Vec::new(), None);
        p.key = "pending-pipeline-test".to_owned();
        // Simulate a worker that has taken the job but cannot finish it yet.
        p.queue_state.store(MAKING, Ordering::Relaxed);
        let p = Arc::new(p);

        assert!(!super::super::library::wanted_early(&p.key));
        assert!(ready(&p).is_none());
        assert!(p.made.get().is_none());
        assert!(super::super::library::wanted_early(&p.key));
    }

    #[test]
    fn draws_promote_early_pipelines_without_claiming_them_twice() {
        let earlier = Arc::new(Pending::new(Vec::new(), None));
        let promoted = Arc::new(Pending::new(Vec::new(), None));
        let requested = Arc::new(Pending::new(Vec::new(), None));
        let mut queue = Queue::new();

        assert_eq!(queue.push(&earlier, EARLY), Some(true));
        assert_eq!(queue.push(&promoted, EARLY), Some(true));
        assert_eq!(queue.push(&requested, REQUESTED), Some(true));
        assert_eq!(queue.push(&promoted, REQUESTED), Some(false));
        assert_eq!(queue.push(&promoted, REQUESTED), None);

        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &requested));
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &promoted));
        assert_eq!(queue.push(&promoted, REQUESTED), None);
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &earlier));
        assert!(queue.pop(false, 1).is_none());
    }

    #[test]
    fn pipelines_known_to_take_long_leave_workers_for_the_others() {
        let new = || Arc::new(Pending::new(Vec::new(), None));
        let (long, longer, asked, quick) = (new(), new(), new(), new());
        let mut queue = Queue::new();
        for p in [&long, &longer, &asked] {
            assert_eq!(queue.push_as(p, EARLY, true), Some(true));
        }
        assert_eq!(queue.push_as(&quick, EARLY, false), Some(true));
        // The quick one first, then one long one at a time; one a draw asks
        // for waits for nobody.
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &quick));
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &long));
        assert!(queue.pop(false, 1).is_none());
        assert_eq!(queue.push_as(&asked, REQUESTED, true), Some(false));
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &asked));
        assert!(!asked.was_slow.load(Ordering::Relaxed));
        assert!(queue.pop(false, 1).is_none());
        queue.slow_now -= 1;
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &longer));
        assert!(queue.pop(false, 1).is_none());
    }

    #[test]
    fn slow_pipelines_get_more_workers_while_nothing_else_is_queued() {
        let mut queue = Queue::new();
        assert_eq!(queue.slow_cap(), quiet_slow_limit());
        let long = Arc::new(Pending::new(Vec::new(), None));
        assert_eq!(queue.push_as(&long, EARLY, true), Some(true));
        assert_eq!(queue.slow_cap(), quiet_slow_limit());
        let quick = Arc::new(Pending::new(Vec::new(), None));
        assert_eq!(queue.push_as(&quick, EARLY, false), Some(true));
        assert_eq!(queue.slow_cap(), slow_limit());
        queue.others_at = Instant::now().checked_sub(QUIET);
        assert_eq!(queue.slow_cap(), quiet_slow_limit());
        assert!(quiet_slow_limit() >= slow_limit());
    }

    #[test]
    fn the_quiet_slow_limit_follows_its_switch() {
        assert_eq!(quiet_limit(None, 24, 6), 24);
        assert_eq!(quiet_limit(Some("3/4"), 24, 6), 18);
        assert_eq!(quiet_limit(Some("3/4"), 4, 2), 3);
        assert_eq!(quiet_limit(Some("0"), 24, 6), 6);
        assert_eq!(quiet_limit(Some("all"), 24, 6), 24);
        assert_eq!(quiet_limit(Some("all"), 1, 1), 1);
    }

    #[test]
    fn a_frame_stops_making_compute_pipelines_at_its_cap() {
        let cap = compute_wait_cap();
        if cap.is_zero() {
            return;
        }
        let frame = u64::MAX - 7;
        assert!(compute_wait_left(frame));
        compute_waited(frame, cap / 2);
        assert!(compute_wait_left(frame));
        compute_waited(frame, cap);
        assert!(!compute_wait_left(frame));
        assert!(compute_wait_left(frame + 1));
    }

    #[test]
    fn the_longest_unclaimed_is_taken_first() {
        let one = |key: &str| {
            let mut p = Pending::new(Vec::new(), None);
            p.key = key.to_owned();
            p.queue_state.store(EARLY, Ordering::Relaxed);
            Arc::new(p)
        };
        let ms = |key: &str| match key {
            "a" => 5_000,
            "b" | "c" | "d" => 130_000,
            _ => 0,
        };
        let mut list: VecDeque<_> = ["a", "b", "c", "d", "e"].map(one).into();
        list[1].queue_state.store(MAKING, Ordering::Relaxed);
        let taken: Vec<String> = std::iter::from_fn(|| longest(&mut list, ms)).map(|p| p.key.clone()).collect();
        assert_eq!(taken, ["c", "d", "a", "e"]);
    }

    /// Graphics pipelines go before compute ones of the same list, and
    /// compute never takes the last worker (upstream #3025).
    #[test]
    fn graphics_go_first_and_compute_leaves_a_worker() {
        let compute = || Arc::new(Pending::new(Vec::new(), None));
        let graphics = || Arc::new(Pending::new(Vec::new(), Some(Box::default())));
        let (asked_c, asked_g, early_c, early_g, long_c) = (compute(), graphics(), compute(), graphics(), compute());
        let mut queue = Queue::new();
        queue.compute_cap = 1;
        assert_eq!(queue.push_as(&early_c, EARLY, false), Some(true));
        assert_eq!(queue.push_as(&long_c, EARLY, true), Some(true));
        assert_eq!(queue.push_as(&early_g, EARLY, false), Some(true));
        assert_eq!(queue.push_as(&asked_c, REQUESTED, false), Some(true));
        assert_eq!(queue.push_as(&asked_g, REQUESTED, false), Some(true));
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &asked_g));
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &asked_c));
        // One compute is being made: the others wait, graphics do not.
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &early_g));
        assert!(queue.pop(false, 1).is_none());
        assert_eq!(queue.slow.len(), 1, "the slow compute one is left for later");
        queue.compute_now -= 1;
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &early_c));
        queue.compute_now -= 1;
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &long_c));
    }

    /// Passes over the screen go before all others, and workers of their own
    /// take nothing else.
    #[test]
    fn passes_over_the_screen_go_first() {
        let of = |screen| {
            let mut p = Pending::new(Vec::new(), None);
            p.screen = screen;
            Arc::new(p)
        };
        let (world, screen) = (of(false), of(true));
        let mut queue = Queue::new();
        assert_eq!(queue.push(&world, REQUESTED), Some(true));
        assert_eq!(queue.push(&screen, REQUESTED), Some(true));
        assert!(Arc::ptr_eq(&queue.pop(true, 1).unwrap(), &screen));
        assert!(queue.pop(true, 1).is_none());
        assert!(Arc::ptr_eq(&queue.pop(false, 1).unwrap(), &world));
    }
}
