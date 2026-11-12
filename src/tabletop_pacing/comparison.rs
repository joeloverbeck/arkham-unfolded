//! Decision-only comparison for #188, removed by its adopt or reject outcome.
//!
//! The windowed table of this build refuses to start unless `ARKHAM_DECORATIVE_RECORD`
//! names a directory in which startup creates one new record file, and unless
//! `nvidia-smi` gives a readable reading within 10 s. F5, F6 and F7 set the decorative
//! rate to display rate, 60/s and 30/s; pressing a key again keeps it. The record holds a
//! start row; every press of F5, F6, F7 and F9; the startup rate, unless a key sets one
//! at once; the rate each press of F5, F6 or F7 sets, even an unchanged one; each change
//! of focus and readings; each second's update count and pace; one `nvidia-smi` reading
//! per second; and a stop row when the app exits normally. F9 ends the readings for the
//! rest of the launch while the rest keeps recording, so one stretch shows what the
//! readings disturb.
//! `arkham-tabletop --decorative-report FILE` summarises a record.

use super::*;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt::Write as _,
    fs::File,
    io::{BufRead, BufReader, LineWriter, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const VARIABLE: &str = "ARKHAM_DECORATIVE_RECORD";
const TITLE: &str = " · #188 decorative rate: ";
const QUERY: &str = "--query-gpu=utilization.gpu,power.draw,temperature.gpu,fan.speed";
/// Readings settle after a change; the report leaves this much of each stretch out.
const SETTLE_MS: u64 = 5_000;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ComparedRate {
    /// The candidate's own rate, so the production tests run unchanged.
    #[default]
    Cap30,
    /// Before #188: decorations hold the display rate like any other request.
    DisplayRate,
    Cap60,
}

impl ComparedRate {
    const KEYS: [(KeyCode, &str, Self); 3] = [
        (KeyCode::F5, "F5", Self::DisplayRate),
        (KeyCode::F6, "F6", Self::Cap60),
        (KeyCode::F7, "F7", Self::Cap30),
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Cap30 => "30/s",
            Self::DisplayRate => "display rate",
            Self::Cap60 => "60/s",
        }
    }

    /// `None` draws decorations at display rate.
    pub(super) fn frame(self) -> Option<Duration> {
        match self {
            Self::Cap30 => Some(DECORATIVE_FRAME),
            Self::DisplayRate => None,
            Self::Cap60 => Some(SETTLED_FRAME),
        }
    }
}

struct Recorder {
    origin: Instant,
    path: PathBuf,
    file: Mutex<LineWriter<File>>,
    failed: AtomicBool,
    sampler: Mutex<Option<Child>>,
}

/// The launch's one recorder, shared by the systems and the reading thread.
#[derive(Resource, Clone, Copy)]
pub(super) struct Recording(&'static Recorder);

fn directory(value: Option<OsString>) -> Result<PathBuf, String> {
    value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!("this #188 comparison build records its launch: set {VARIABLE} to a directory")
        })
}

impl Recorder {
    /// A launch that cannot record must not look like one that did.
    fn start(directory: &Path) -> Result<&'static Self, String> {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis());
        let path = directory.join(format!("decorative-{unix_ms}.csv"));
        let file = File::create_new(&path)
            .map_err(|error| format!("{VARIABLE}: cannot create {}: {error}", path.display()))?;
        let recorder: &'static Self = Box::leak(Box::new(Self {
            origin: Instant::now(),
            path,
            file: Mutex::new(LineWriter::new(file)),
            failed: AtomicBool::new(false),
            sampler: Mutex::new(None),
        }));
        recorder.row(format_args!("start,0,{unix_ms}"));
        if recorder.failed() {
            return Err(format!(
                "{VARIABLE}: cannot write {}",
                recorder.path.display()
            ));
        }
        Ok(recorder)
    }

    fn row(&self, row: std::fmt::Arguments) {
        if let Err(error) = writeln!(self.file.lock().unwrap(), "{row}")
            && !self.failed.swap(true, Ordering::Relaxed)
        {
            error!("Decorative-rate record failed: {error}");
        }
    }

    fn mark(&self, kind: &str, value: impl std::fmt::Display) {
        let at = self.origin.elapsed().as_millis();
        self.row(format_args!("{kind},{at},{value}"));
    }

    fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    fn sampling(&self) -> bool {
        self.sampler.lock().unwrap().is_some()
    }

    /// The launch needs the readings, so it starts only once one has been read; it
    /// waits at most 10 s for that, past any unreadable lines.
    /// `nvidia-smi` writes its own errors to the app's standard error.
    fn start_sampling(&'static self) -> Result<(), String> {
        if self.sampling() {
            return Ok(());
        }
        let mut child = Command::new("nvidia-smi")
            .args([QUERY, "--format=csv,noheader,nounits", "-l", "1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("{VARIABLE}: nvidia-smi did not start: {error}"))?;
        let output = child.stdout.take().expect("piped nvidia-smi output");
        *self.sampler.lock().unwrap() = Some(child);
        self.mark("readings", "on");
        let (first, readable) = mpsc::channel();
        std::thread::Builder::new()
            .name("decorative-gpu-readings".into())
            .spawn(move || {
                for line in BufReader::new(output).lines().map_while(Result::ok) {
                    let line = line.replace(' ', "");
                    let _ = first.send(Reading::parse(&line).is_some());
                    self.mark("gpu", line);
                }
                // Still held here means nobody stopped it: it ended by itself.
                if let Some(mut child) = self.sampler.lock().unwrap().take() {
                    let _ = child.wait();
                    self.mark("readings", "off");
                    warn!("nvidia-smi ended; no further GPU readings");
                }
            })
            .map_err(|error| format!("{VARIABLE}: no reading thread: {error}"))?;
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut lines = std::iter::from_fn(|| {
            readable
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .ok()
        });
        if lines.any(|readable| readable) {
            Ok(())
        } else {
            self.stop_sampling();
            Err(format!(
                "{VARIABLE}: nvidia-smi gave no readable reading within 10 s"
            ))
        }
    }

    fn stop_sampling(&self) {
        if let Some(mut child) = self.sampler.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
            self.mark("readings", "off");
        }
    }
}

/// Called once a second by the production counter; does nothing unless recording.
pub(super) fn record_updates(
    recording: Option<&Recording>,
    count: u32,
    elapsed: Duration,
    longest_gap: Duration,
    pace: Pace,
) {
    if let Some(Recording(recorder)) = recording {
        recorder.mark(
            "updates",
            format_args!(
                "{count},{},{:.1},{}",
                elapsed.as_millis(),
                longest_gap.as_secs_f64() * 1000.0,
                pace.label()
            ),
        );
    }
}

/// Registered only by the windowed Gathering table, after `TabletopPacingPlugin`.
pub(crate) struct DecorativeComparisonPlugin;

impl Plugin for DecorativeComparisonPlugin {
    fn build(&self, app: &mut App) {
        let recorder = directory(std::env::var_os(VARIABLE))
            .and_then(|directory| Recorder::start(&directory))
            .and_then(|recorder| recorder.start_sampling().map(|()| recorder))
            .unwrap_or_else(|error| panic!("{error}"));
        info!(
            "Recording the #188 decorative-rate comparison to {}",
            recorder.path.display()
        );
        install(app, recorder);
    }
}

fn install(app: &mut App, recorder: &'static Recorder) {
    app.insert_resource(Recording(recorder))
        .init_resource::<ComparedRate>()
        .add_systems(
            Last,
            (compare, record_focus, record_exit)
                .chain()
                .before(SelectCadence),
        );
}

fn compare(
    keys: Res<ButtonInput<KeyCode>>,
    recording: Res<Recording>,
    mut selected: ResMut<ComparedRate>,
    mut shown: Local<Option<(ComparedRate, bool, bool)>>,
    mut windows: Query<&mut Window>,
) {
    let Recording(recorder) = *recording;
    let mut pressed = false;
    for (key, name, rate) in ComparedRate::KEYS {
        if keys.just_pressed(key) {
            recorder.mark("key", name);
            *selected = rate;
            // Every press opens a measured stretch, even when the rate stays the same.
            recorder.mark("rate", rate.label());
            pressed = true;
        }
    }
    if keys.just_pressed(KeyCode::F9) {
        recorder.mark("key", "F9");
        recorder.stop_sampling();
    }
    if shown.is_none() && !pressed {
        recorder.mark("rate", selected.label());
    }
    let state = (*selected, recorder.sampling(), recorder.failed());
    if *shown == Some(state) {
        return;
    }
    *shown = Some(state);
    let (_, readings, failed) = state;
    let suffix = format!(
        "{TITLE}{} · F5 display, F6 60/s, F7 30/s · readings {}{}",
        selected.label(),
        if readings { "on (F9 ends)" } else { "off" },
        if failed { " · RECORD FAILED" } else { "" }
    );
    info!("Decorative rate comparison (#188):{suffix}");
    for mut window in &mut windows {
        let base = window
            .title
            .split(TITLE)
            .next()
            .unwrap_or_default()
            .to_owned();
        window.title = format!("{base}{suffix}");
    }
}

/// The decorations stop without focus, so the record keeps the window's focus.
fn record_focus(recording: Res<Recording>, windows: Query<&Window>, mut last: Local<Option<bool>>) {
    let focused = windows.iter().any(|window| window.focused);
    if *last != Some(focused) {
        *last = Some(focused);
        recording.0.mark("focus", focused);
    }
}

fn record_exit(recording: Res<Recording>, mut exits: MessageReader<AppExit>) {
    if exits.read().count() > 0 {
        recording.0.mark("stop", "exit");
        recording.0.stop_sampling();
    }
}

/// `arkham-tabletop --decorative-report FILE`.
pub fn report(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let (Some(record), None) = (arguments.next(), arguments.next()) else {
        return Err("usage: arkham-tabletop --decorative-report FILE".into());
    };
    let text = std::fs::read_to_string(&record).map_err(|error| format!("{record}: {error}"))?;
    print!("{}", summarise(&text)?);
    Ok(())
}

/// `nvidia-smi`'s utilisation (%), power draw (W), temperature (°C) and fan speed (%).
#[derive(Clone, Copy)]
struct Reading([f64; 4]);

impl Reading {
    fn parse(fields: &str) -> Option<Self> {
        let mut values = fields.split(',').map(|field| field.parse::<f64>().ok());
        let reading = [
            values.next()??,
            values.next()??,
            values.next()??,
            values.next()??,
        ];
        values.next().is_none().then_some(Self(reading))
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn range(values: &[f64]) -> (f64, f64) {
    values
        .iter()
        .fold((f64::MAX, f64::MIN), |(min, max), value| {
            (min.min(*value), max.max(*value))
        })
}

#[derive(Default)]
struct Gpu {
    readings: Vec<Reading>,
    unreadable: u32,
}

impl Gpu {
    fn add(&mut self, fields: &str) {
        match Reading::parse(fields) {
            Some(reading) => self.readings.push(reading),
            None => self.unreadable += 1,
        }
    }

    fn extend(&mut self, other: &Self) {
        self.readings.extend(other.readings.iter().copied());
        self.unreadable += other.unreadable;
    }

    fn column(&self, index: usize) -> Vec<f64> {
        self.readings
            .iter()
            .map(|Reading(values)| values[index])
            .collect()
    }

    /// Samples, power, utilisation, temperature and fan. A contiguous stretch shows how
    /// temperature and fan moved; pooled stretches only their mean and maximum.
    fn columns(&self, contiguous: bool) -> String {
        if self.readings.is_empty() {
            return format!("0 ({} unreadable) | – | – | – | –", self.unreadable);
        }
        let [utilisation, power, temperature, fan] = [0, 1, 2, 3].map(|index| self.column(index));
        let (low, high) = range(&power);
        let trend = |values: &[f64]| {
            let (_, max) = range(values);
            if contiguous {
                format!(
                    "{:.0}→{:.0} (max {max:.0})",
                    values[0],
                    values[values.len() - 1]
                )
            } else {
                format!("mean {:.0} (max {max:.0})", mean(values))
            }
        };
        format!(
            "{} ({} unreadable) | {:.1} ({low:.1}–{high:.1}) | {:.0} | {} | {}",
            self.readings.len(),
            self.unreadable,
            mean(&power),
            mean(&utilisation),
            trend(&temperature),
            trend(&fan)
        )
    }
}

#[derive(Default)]
struct Updates {
    rates: Vec<f64>,
    longest_gap: f64,
    paces: BTreeMap<String, u32>,
}

impl Updates {
    fn add(&mut self, fields: &[&str]) {
        let number = |index: usize| {
            fields
                .get(index)
                .and_then(|field| field.parse::<f64>().ok())
        };
        if let (Some(count), Some(elapsed), Some(gap)) = (number(0), number(1), number(2)) {
            self.rates.push(count * 1000.0 / elapsed);
            self.longest_gap = self.longest_gap.max(gap);
            *self
                .paces
                .entry(fields.get(3).copied().unwrap_or_default().into())
                .or_default() += 1;
        }
    }

    fn columns(&self) -> String {
        if self.rates.is_empty() {
            return "0 | – | – | –".into();
        }
        let (low, high) = range(&self.rates);
        format!(
            "{} | {:.1} ({low:.1}–{high:.1}) | {:.1} | {:?}",
            self.rates.len(),
            mean(&self.rates),
            self.longest_gap,
            self.paces
        )
    }
}

/// The time from one rate row, readings change or focus change to the next. Each press
/// of F5, F6 or F7 writes a rate row, so it opens a stretch even when the rate stays.
#[derive(Default)]
struct Stretch {
    /// `None` before the first rate row.
    rate: Option<String>,
    /// Opened by pressing a rate key, not by startup.
    keyed: bool,
    readings: bool,
    focused: bool,
    start: u64,
    end: u64,
    gpu: Gpu,
    updates: Updates,
}

impl Stretch {
    fn next(&self, kind: &str, value: &str, at: u64, keyed: bool) -> Self {
        let mut next = Self {
            rate: self.rate.clone(),
            keyed: self.keyed,
            readings: self.readings,
            focused: self.focused,
            start: at,
            ..Self::default()
        };
        match kind {
            "rate" => (next.rate, next.keyed) = (Some(value.into()), keyed),
            "readings" => next.readings = value == "on",
            _ => next.focused = value == "true",
        }
        next
    }
}

fn summarise(record: &str) -> Result<String, String> {
    let mut stretches = vec![];
    let mut current = Stretch::default();
    let mut stopped = None;
    let mut rate_key = false;
    for (number, line) in record.lines().enumerate() {
        let (kind, rest) = line.split_once(',').unwrap_or((line, ""));
        let (at, value) = rest.split_once(',').unwrap_or((rest, ""));
        let at: u64 = at
            .parse()
            .map_err(|_| format!("line {}: no time: {line}", number + 1))?;
        current.end = at;
        match kind {
            "rate" | "readings" | "focus" => {
                let next = current.next(kind, value, at, rate_key);
                rate_key &= kind != "rate";
                stretches.push(std::mem::replace(&mut current, next));
            }
            "key" => rate_key = ["F5", "F6", "F7"].contains(&value),
            "gpu" if at >= current.start + SETTLE_MS => current.gpu.add(value),
            // Each counts the second before it.
            "updates" if at >= current.start + SETTLE_MS + 1_000 => {
                current.updates.add(&value.split(',').collect::<Vec<_>>())
            }
            "stop" => {
                stopped = Some(at);
                break;
            }
            _ => {}
        }
    }
    stretches.push(current);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "Record {}. Each stretch leaves out its first {} s; its update seconds start a second \
         later. Paces show whether interaction mixed into a stretch.\n",
        if stopped.is_some() {
            "ends with the app's exit"
        } else {
            "has NO stop row: the app did not exit normally, or the record was cut short"
        },
        SETTLE_MS / 1000
    );
    let _ = writeln!(
        out,
        "| From s | To s | Rate | Opened by | Readings | Focus | GPU samples | Power W, mean (min–max) | GPU % | Temp °C | Fan % | Update seconds | Updates/s, mean (min–max) | Longest gap ms | Paces |\n|{}",
        "---|".repeat(15)
    );
    let mut by_rate: BTreeMap<&str, (Gpu, Updates)> = BTreeMap::new();
    for stretch in &stretches {
        let Some(rate) = stretch.rate.as_deref() else {
            continue;
        };
        let _ = writeln!(
            out,
            "| {:.1} | {:.1} | {rate} | {} | {} | {} | {} | {} |",
            stretch.start as f64 / 1000.0,
            stretch.end as f64 / 1000.0,
            if stretch.keyed { "key" } else { "startup" },
            if stretch.readings { "on" } else { "off" },
            if stretch.focused {
                "focused"
            } else {
                "unfocused"
            },
            stretch.gpu.columns(true),
            stretch.updates.columns()
        );
        if stretch.keyed && stretch.focused && stretch.readings {
            let (gpu, updates) = by_rate.entry(rate).or_default();
            gpu.extend(&stretch.gpu);
            updates.rates.extend(&stretch.updates.rates);
        }
    }
    let _ = writeln!(
        out,
        "\nKey-opened, focused stretches with readings on, by rate:\n\n| Rate | GPU samples | Power W, mean (min–max) | GPU % | Temp °C | Fan % | Update seconds | Updates/s mean |\n|---|---|---|---|---|---|---|---|"
    );
    for (rate, (gpu, updates)) in &by_rate {
        let rates = if updates.rates.is_empty() {
            "–".to_string()
        } else {
            format!("{:.1}", mean(&updates.rates))
        };
        let _ = writeln!(
            out,
            "| {rate} | {} | {} | {rates} |",
            gpu.columns(false),
            updates.rates.len()
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{app, decorate, end_active_interval, pace};
    use super::*;

    fn press(app: &mut App, key: KeyCode) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        keys.press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
    }

    fn title(app: &mut App) -> String {
        app.world_mut()
            .query::<&Window>()
            .single(app.world())
            .unwrap()
            .title
            .clone()
    }

    fn settles_at(app: &mut App, pace_expected: Pace, mode: UpdateMode) {
        end_active_interval(app);
        decorate(app);
        assert_eq!(pace(app), pace_expected);
        assert_eq!(app.world().resource::<WinitSettings>().focused_mode, mode);
    }

    #[test]
    fn each_key_sets_one_rate_however_often_it_is_pressed_and_the_record_reads_back() {
        let root = tempfile::tempdir().unwrap();
        let recorder = Recorder::start(root.path()).unwrap();
        let mut app = app();
        install(&mut app, recorder);
        app.update();
        assert!(title(&mut app).contains(" · #188 decorative rate: 30/s · "));

        for _ in 0..2 {
            press(&mut app, KeyCode::F5);
            assert!(title(&mut app).contains(" · #188 decorative rate: display rate · "));
            assert_eq!(title(&mut app).matches(TITLE).count(), 1);
            // Today's behaviour: the decorations alone hold the display rate.
            settles_at(&mut app, Pace::Active, UpdateMode::Continuous);
        }
        for _ in 0..2 {
            press(&mut app, KeyCode::F6);
            settles_at(
                &mut app,
                Pace::Decorative,
                UpdateMode::reactive_low_power(SETTLED_FRAME),
            );
        }
        record_updates(
            Some(&Recording(recorder)),
            60,
            Duration::from_secs(1),
            Duration::from_millis(17),
            Pace::Decorative,
        );
        for _ in 0..2 {
            press(&mut app, KeyCode::F7);
            assert!(title(&mut app).contains(" · #188 decorative rate: 30/s · "));
            settles_at(
                &mut app,
                Pace::Decorative,
                UpdateMode::reactive_low_power(DECORATIVE_FRAME),
            );
        }
        press(&mut app, KeyCode::F9);
        app.world_mut().write_message(AppExit::Success);
        app.update();

        let record = std::fs::read_to_string(&recorder.path).unwrap();
        let rows: Vec<_> = record
            .lines()
            .map(|line| {
                let (kind, rest) = line.split_once(',').unwrap();
                let value = rest.split_once(',').map_or("", |(_, value)| value);
                format!("{kind} {value}")
            })
            .collect();
        assert_eq!(
            rows.iter()
                // The production counter may add its own rows if the test passes a second.
                .filter(|row| !row.starts_with("start ") && !row.starts_with("updates "))
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "rate 30/s",
                "focus true",
                "key F5",
                "rate display rate",
                "key F5",
                "rate display rate",
                "key F6",
                "rate 60/s",
                "key F6",
                "rate 60/s",
                "key F7",
                "rate 30/s",
                "key F7",
                "rate 30/s",
                "key F9",
                "stop exit",
            ],
            "{record}"
        );
        assert!(rows.contains(&"updates 60,1000,17.0,decorative".to_string()));
        let summary = summarise(&record).unwrap();
        assert!(summary.contains("ends with the app's exit"), "{summary}");
        for opened in [
            "| 30/s | startup | off | unfocused |",
            "| display rate | key | off | focused |",
            "| 60/s | key | off | focused |",
            "| 30/s | key | off | focused |",
        ] {
            assert!(summary.contains(opened), "{opened}\n{summary}");
        }
    }

    #[test]
    fn startup_refuses_without_a_record_directory_or_file() {
        assert!(directory(None).unwrap_err().contains(VARIABLE));
        assert!(directory(Some(OsString::new())).is_err());
        let missing = tempfile::tempdir().unwrap().path().join("missing");
        assert!(Recorder::start(&missing).is_err());
    }

    #[test]
    fn report_counts_only_settled_key_opened_focused_stretches_with_readings() {
        let mut record = String::from("start,0,1\nreadings,0,on\nrate,0,30/s\nfocus,0,true\n");
        // Startup at 30/s is shown but never pooled with a key-opened 30/s stretch.
        for second in 1..=12u64 {
            record.push_str(&format!("gpu,{},90,400.0,70,60\n", second * 1000));
        }
        record.push_str("key,12000,F5\nrate,12000,display rate\n");
        for second in 13..=24u64 {
            let at = second * 1000;
            // Readings in the first five seconds still settle and are left out.
            let power = if second < 17 { 300 } else { 220 };
            record.push_str(&format!("gpu,{at},90,{power}.0,70,60\n"));
            record.push_str(&format!("updates,{at},142,1000,9.0,active\n"));
        }
        record.push_str("key,24000,F7\nrate,24000,30/s\n");
        for second in 25..=36u64 {
            let at = second * 1000;
            let power = if second < 29 { 220 } else { 60 };
            record.push_str(&format!("gpu,{at},5,{power}.0,65,55\n"));
            record.push_str(&format!("updates,{at},30,1000,34.0,decorative\n"));
        }
        record.push_str("gpu,36500,[N/A],60.0,65,55\nfocus,37000,false\n");
        for second in 38..=48u64 {
            record.push_str(&format!("gpu,{},1,500.0,60,50\n", second * 1000));
        }
        record.push_str("focus,48000,true\nkey,48000,F9\nreadings,48000,off\n");
        for second in 49..=60u64 {
            record.push_str(&format!(
                "updates,{},31,1000,33.5,decorative\n",
                second * 1000
            ));
        }
        record.push_str("stop,60500,exit\nreadings,60600,off\n");
        let summary = summarise(&record).unwrap();
        assert!(summary.contains("ends with the app's exit"), "{summary}");
        assert!(
            summary.contains("| display rate | 8 (0 unreadable) | 220.0 (220.0–220.0) | 90 | mean 70 (max 70) | mean 60 (max 60) | 7 | 142.0 |"),
            "{summary}"
        );
        assert!(
            summary.contains("| 30/s | 8 (1 unreadable) | 60.0 (60.0–60.0) | 5 | mean 65 (max 65) | mean 55 (max 55) | 7 | 30.0 |"),
            "{summary}"
        );
        assert!(
            summary.contains(
                "| 0.0 | 12.0 | 30/s | startup | on | focused | 8 (0 unreadable) | 400.0"
            ),
            "{summary}"
        );
        assert!(
            summary
                .contains("| 37.0 | 48.0 | 30/s | key | on | unfocused | 7 (0 unreadable) | 500.0"),
            "{summary}"
        );
        assert!(
            summary.contains("| 48.0 | 60.5 | 30/s | key | off | focused | 0 (0 unreadable) | – | – | – | – | 7 | 31.0 (31.0–31.0) |"),
            "{summary}"
        );
        // The readings row after the stop opens no stretch.
        assert!(!summary.contains("| 60.6 |"), "{summary}");
        assert!(
            summarise("start,0,1\nrate,0,30/s\n")
                .unwrap()
                .contains("NO stop row")
        );
    }
}
