//! Summarises one frame-stage record as Markdown for #157's evidence record.

use super::{
    CUE, FOCUSED, GpuReading, IDLE, MainFrame, REDUCED_MOTION, RenderFrame, Row, SAMPLING,
};
use std::{collections::HashMap, fmt::Write as _};

/// The camera counts as moving this close to an update that moved it. A mouse drag
/// leaves updates without movement between pointer events.
const MOTION_WINDOW: u64 = 150_000_000;
/// "Still" starts this long after the last movement, clear of any easing.
const STILL_AFTER: u64 = 500_000_000;
const CAMERA_MOVED: f32 = 1e-4;
/// A handoff wait longer than this means the render thread finished after the update.
const WAITED: u64 = 200_000;
const SECOND: u64 = 1_000_000_000;
/// One refresh of the owner's display, 143.99 Hz (#151).
const DISPLAY_REFRESH: u64 = 6_944_927;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Condition {
    Settled,
    StillWithCue,
    Panning,
    Other,
}

impl Condition {
    const ALL: [Self; 4] = [
        Self::Settled,
        Self::StillWithCue,
        Self::Panning,
        Self::Other,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Settled => "Settled",
            Self::StillWithCue => "Still with cue",
            Self::Panning => "Panning",
            Self::Other => "Other",
        }
    }
}

#[derive(Default)]
pub(crate) struct Record {
    unix_ms: Option<u64>,
    main: Vec<MainFrame>,
    render: HashMap<u64, RenderFrame>,
    gpu: Vec<(u64, GpuReading)>,
    sampling: Vec<(u64, bool)>,
    windows: Vec<(u64, u32, u32, f32)>,
}

impl Record {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut record = Self::default();
        let mut lines = text.lines().peekable();
        while let Some(line) = lines.next() {
            let row = match line.parse::<Row>() {
                Ok(row) => row,
                // The process can end while the last line is being written.
                Err(_) if lines.peek().is_none() => break,
                Err(error) => return Err(error),
            };
            match row {
                Row::Start { unix_ms } => record.unix_ms = Some(unix_ms),
                Row::Main(frame) => record.main.push(frame),
                Row::Render(frame) => {
                    record.render.insert(frame.frame, frame);
                }
                Row::Gpu { at, reading } => record.gpu.push((at, reading)),
                Row::Sampling { at, on } => record.sampling.push((at, on)),
                Row::Window {
                    at,
                    width,
                    height,
                    scale,
                } => record.windows.push((at, width, height, scale)),
            }
        }
        Ok(record)
    }
}

/// One update with what followed it.
#[derive(Clone, Copy)]
struct Frame {
    main: MainFrame,
    condition: Condition,
    /// From this update's start to the next one's.
    period: Option<u64>,
    /// From the end of extraction to the next update's start.
    gap: Option<u64>,
}

pub(crate) struct Analysis {
    record: Record,
    frames: Vec<Frame>,
    refresh: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    pub count: usize,
    pub median: u64,
    pub p95: u64,
    pub max: u64,
    /// How many values exceeded one refresh.
    pub over: usize,
    /// How many reached 1.5 refreshes.
    pub late: usize,
}

/// Nearest-rank percentile of a sorted, non-empty slice.
fn percentile(sorted: &[u64], fraction: f64) -> u64 {
    let rank = (fraction * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

pub(crate) fn summarise(values: impl Iterator<Item = u64>, refresh: u64) -> Option<Summary> {
    let mut values: Vec<u64> = values.collect();
    values.sort_unstable();
    let max = *values.last()?;
    Some(Summary {
        count: values.len(),
        median: percentile(&values, 0.5),
        p95: percentile(&values, 0.95),
        max,
        over: values.iter().filter(|value| **value > refresh).count(),
        late: values
            .iter()
            .filter(|value| is_late(**value, refresh))
            .count(),
    })
}

/// Whether a time reached 1.5 refreshes: clear of the jitter around one refresh that
/// display-paced times always show.
fn is_late(time: u64, refresh: u64) -> bool {
    time * 2 >= refresh * 3
}

pub(crate) fn classify(frames: &[MainFrame]) -> Vec<Condition> {
    // Distance in time from each update to the nearest one that moved the camera.
    let mut nearest = vec![u64::MAX; frames.len()];
    let mut last = None;
    for (index, frame) in frames.iter().enumerate() {
        if frame.camera_step > CAMERA_MOVED {
            last = Some(frame.start);
        }
        if let Some(last) = last {
            nearest[index] = frame.start - last;
        }
    }
    let mut next = None;
    for (index, frame) in frames.iter().enumerate().rev() {
        if frame.camera_step > CAMERA_MOVED {
            next = Some(frame.start);
        }
        if let Some(next) = next {
            nearest[index] = nearest[index].min(next - frame.start);
        }
    }
    frames
        .iter()
        .zip(nearest)
        .map(|(frame, nearest)| {
            let choice_shown = frame.has(CUE) && frame.has(FOCUSED);
            // A settled update did not move the camera, even just before a pan, and
            // the long wait after it must not count as a late panning frame.
            if frame.has(IDLE) {
                if choice_shown {
                    Condition::Settled
                } else {
                    Condition::Other
                }
            } else if nearest <= MOTION_WINDOW {
                Condition::Panning
            } else if nearest > STILL_AFTER && choice_shown && !frame.has(REDUCED_MOTION) {
                Condition::StillWithCue
            } else {
                Condition::Other
            }
        })
        .collect()
}

fn ms(nanoseconds: u64) -> f64 {
    nanoseconds as f64 / 1e6
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0usize), |(sum, count), value| {
        (sum + value, count + 1)
    });
    (count > 0).then(|| sum / count as f64)
}

impl Analysis {
    pub(crate) fn new(record: Record, refresh: u64) -> Self {
        let conditions = classify(&record.main);
        let frames = record
            .main
            .iter()
            .zip(conditions)
            .enumerate()
            .map(|(index, (main, condition))| {
                // A new app in the same process restarts nothing here, but a gap in
                // the numbering means an update was not recorded.
                let next = record
                    .main
                    .get(index + 1)
                    .filter(|next| next.frame == main.frame + 1);
                Frame {
                    main: *main,
                    condition,
                    period: next.map(|next| next.start - main.start),
                    gap: next.map(|next| next.start.saturating_sub(main.end)),
                }
            })
            .collect();
        Self {
            record,
            frames,
            refresh,
        }
    }

    fn late(&self, frame: &Frame) -> bool {
        frame
            .period
            .is_some_and(|period| is_late(period, self.refresh))
    }

    fn of(&self, condition: Condition) -> impl Iterator<Item = &Frame> {
        self.frames
            .iter()
            .filter(move |frame| frame.condition == condition)
    }

    /// The render thread's work that an update's handoff waited for: the frame before.
    fn awaited(&self, frame: &Frame) -> Option<&RenderFrame> {
        self.record.render.get(&frame.main.frame.checked_sub(1)?)
    }

    /// The condition that held for at least nine tenths of the second before `at`.
    pub(crate) fn condition_before(&self, at: u64) -> Option<Condition> {
        let from = at.saturating_sub(SECOND);
        let mut held: HashMap<Condition, u64> = HashMap::new();
        let first = self.frames.partition_point(|frame| frame.main.start < from);
        for frame in &self.frames[first.saturating_sub(1)..] {
            if frame.main.start >= at {
                break;
            }
            let Some(period) = frame.period else { continue };
            let start = frame.main.start.max(from);
            let end = (frame.main.start + period).min(at);
            *held.entry(frame.condition).or_default() += end.saturating_sub(start);
        }
        held.into_iter()
            .find(|(_, held)| *held * 10 >= SECOND * 9)
            .map(|(condition, _)| condition)
    }

    fn stage_row(&self, out: &mut String, name: &str, values: impl Iterator<Item = u64>) {
        if let Some(summary) = summarise(values, self.refresh) {
            let _ = writeln!(
                out,
                "| {name} | {} | {:.2} | {:.2} | {:.2} | {} | {} |",
                summary.count,
                ms(summary.median),
                ms(summary.p95),
                ms(summary.max),
                summary.over,
                summary.late
            );
        }
    }

    fn stages(&self, out: &mut String, condition: Condition) {
        let frames: Vec<&Frame> = self.of(condition).collect();
        if frames.is_empty() {
            return;
        }
        let _ = writeln!(out, "\n### {}: stage times (ms)\n", condition.name());
        let _ = writeln!(
            out,
            "| Stage | Frames | Median | 95th | Max | Over one refresh | At least 1.5 refreshes |\n\
             | --- | --- | --- | --- | --- | --- | --- |"
        );
        let main =
            |stage: fn(&MainFrame) -> u64| frames.iter().map(move |frame| stage(&frame.main));
        self.stage_row(
            out,
            "Update to update",
            frames.iter().filter_map(|frame| frame.period),
        );
        self.stage_row(out, "Time step", main(|frame| frame.step));
        self.stage_row(out, "Main-app update", main(MainFrame::main_update));
        self.stage_row(out, "· First", main(|frame| frame.first));
        self.stage_row(out, "· PreUpdate", main(|frame| frame.pre));
        self.stage_row(out, "· Update", main(|frame| frame.update));
        self.stage_row(out, "· PostUpdate", main(|frame| frame.post));
        self.stage_row(out, "· Last", main(|frame| frame.last));
        self.stage_row(out, "Handoff wait", main(|frame| frame.wait));
        self.stage_row(out, "Extraction", main(|frame| frame.extract));
        self.stage_row(
            out,
            "Between updates",
            frames.iter().filter_map(|frame| frame.gap),
        );
        let render: Vec<&RenderFrame> = frames
            .iter()
            .filter_map(|frame| self.record.render.get(&frame.main.frame))
            .collect();
        let rendered =
            |stage: fn(&RenderFrame) -> u64| render.iter().map(move |frame| stage(frame));
        self.stage_row(
            out,
            "Render thread, whole frame",
            rendered(|frame| frame.total),
        );
        self.stage_row(out, "· Surface image wait", rendered(|frame| frame.acquire));
        self.stage_row(out, "· Render graph", rendered(|frame| frame.graph));
        self.stage_row(out, "· Present", rendered(|frame| frame.present));
        self.stage_row(out, "· Everything else", rendered(render_rest));
    }

    fn late_frames(&self, out: &mut String, condition: Condition) {
        let (late, on_time): (Vec<&Frame>, Vec<&Frame>) = self
            .of(condition)
            .filter(|frame| frame.period.is_some())
            .partition(|frame| self.late(frame));
        if late.is_empty() {
            return;
        }
        let waited = late.iter().filter(|frame| frame.main.wait > WAITED).count();
        let _ = writeln!(
            out,
            "\n### {}: late frames\n\n{} of {} updates took at least 1.5 refreshes to the next one. \
             In {} of them the update waited for the render thread; in {} the update finished last.\n",
            condition.name(),
            late.len(),
            late.len() + on_time.len(),
            waited,
            late.len() - waited
        );
        let _ = writeln!(
            out,
            "| Stage | Late: mean ms | Late: over one refresh | On time: mean ms |\n\
             | --- | --- | --- | --- |"
        );
        let mut row = |name: &str, stage: &dyn Fn(&Frame) -> Option<u64>| {
            let mean_of = |frames: &[&Frame]| {
                mean(frames.iter().filter_map(|frame| stage(frame)).map(ms))
                    .map_or("–".into(), |mean| format!("{mean:.2}"))
            };
            let over = late
                .iter()
                .filter_map(|frame| stage(frame))
                .filter(|time| *time > self.refresh)
                .count();
            let _ = writeln!(
                out,
                "| {name} | {} | {over} | {} |",
                mean_of(&late),
                mean_of(&on_time)
            );
        };
        row("Update to update", &|frame| frame.period);
        row("Main-app update", &|frame| Some(frame.main.main_update()));
        row("Handoff wait", &|frame| Some(frame.main.wait));
        row("Extraction", &|frame| Some(frame.main.extract));
        row("Between updates", &|frame| frame.gap);
        row("Awaited render frame", &|frame| {
            self.awaited(frame).map(|render| render.total)
        });
        row("· Surface image wait", &|frame| {
            self.awaited(frame).map(|render| render.acquire)
        });
        row("· Render graph", &|frame| {
            self.awaited(frame).map(|render| render.graph)
        });
        row("· Present", &|frame| {
            self.awaited(frame).map(|render| render.present)
        });
        row("· Everything else", &|frame| {
            self.awaited(frame).map(render_rest)
        });
    }

    fn time_steps(&self, out: &mut String) {
        // A step is the time since the previous update started, so a step below 1.5
        // refreshes belongs to an update that did not follow a late one.
        let steps: Vec<u64> = self
            .of(Condition::Panning)
            .map(|frame| frame.main.step)
            .filter(|step| *step > 0 && !is_late(*step, self.refresh))
            .collect();
        let Some(summary) = summarise(steps.iter().copied(), self.refresh) else {
            return;
        };
        let mut sorted = steps.clone();
        sorted.sort_unstable();
        let off = |limit: u64| {
            let count = steps
                .iter()
                .filter(|step| step.abs_diff(self.refresh) > limit)
                .count();
            100.0 * count as f64 / steps.len() as f64
        };
        let _ = writeln!(
            out,
            "\n### Panning: time steps that did not follow a late update\n\n\
             {} steps: median {:.2} ms, 5th percentile {:.2} ms, 95th {:.2} ms. \
             {:.1} % differ from one refresh by more than 0.5 ms and {:.1} % by more than 1 ms.",
            summary.count,
            ms(summary.median),
            ms(percentile(&sorted, 0.05)),
            ms(summary.p95),
            off(500_000),
            off(1_000_000)
        );
    }

    fn gpu(&self, out: &mut String, baseline: &[GpuReading]) {
        let _ = writeln!(
            out,
            "\n### GPU readings (one per second)\n\n\
             | Condition | Readings | Utilisation % | Power W | Temperature °C | Fan % |\n\
             | --- | --- | --- | --- | --- | --- |"
        );
        let mut row = |name: &str, readings: &[GpuReading]| {
            if readings.is_empty() {
                return;
            }
            let mut line = format!("| {name} | {} |", readings.len());
            for column in 0..4 {
                let values: Vec<f32> = readings
                    .iter()
                    .filter_map(|reading| reading.0[column])
                    .collect();
                match mean(values.iter().map(|value| f64::from(*value))) {
                    Some(mean) => {
                        let low = values.iter().copied().fold(f32::INFINITY, f32::min);
                        let high = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                        let _ = write!(line, " {mean:.0} ({low:.0}–{high:.0}) |");
                    }
                    None => line.push_str(" – |"),
                }
            }
            let _ = writeln!(out, "{line}");
        };
        row("Baseline, app closed", baseline);
        let mut by_condition: HashMap<Option<Condition>, Vec<GpuReading>> = HashMap::new();
        for (at, reading) in &self.record.gpu {
            by_condition
                .entry(self.condition_before(*at))
                .or_default()
                .push(*reading);
        }
        for condition in Condition::ALL {
            row(
                condition.name(),
                by_condition
                    .get(&Some(condition))
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            );
        }
        row(
            "Mixed second",
            by_condition
                .get(&None)
                .map(Vec::as_slice)
                .unwrap_or_default(),
        );
        let _ = writeln!(
            out,
            "\nMean (lowest–highest). A reading belongs to a condition that held for at least \
             nine tenths of the second before it."
        );
    }

    fn sampling(&self, out: &mut String) {
        let panning: Vec<&Frame> = self
            .of(Condition::Panning)
            .filter(|frame| frame.period.is_some())
            .collect();
        if panning.is_empty() {
            return;
        }
        let _ = writeln!(
            out,
            "\n### Whether taking GPU readings disturbed frames (panning)\n\n\
             | GPU readings | Updates | Late | Late % |\n| --- | --- | --- | --- |"
        );
        for (name, on) in [("On", true), ("Off", false)] {
            let frames: Vec<&&Frame> = panning
                .iter()
                .filter(|frame| frame.main.has(SAMPLING) == on)
                .collect();
            if frames.is_empty() {
                continue;
            }
            let late = frames.iter().filter(|frame| self.late(frame)).count();
            let _ = writeln!(
                out,
                "| {name} | {} | {late} | {:.2} |",
                frames.len(),
                100.0 * late as f64 / frames.len() as f64
            );
        }
        let mut phases = [(0usize, 0usize); 10];
        let mut readings = self.record.gpu.iter().map(|(at, _)| *at).peekable();
        let mut previous = None;
        for frame in &panning {
            while let Some(at) = readings.next_if(|at| *at <= frame.main.start) {
                previous = Some(at);
            }
            let Some(since) = previous.map(|at| frame.main.start - at) else {
                continue;
            };
            if frame.main.has(SAMPLING) && since < SECOND {
                let phase = &mut phases[(since / (SECOND / 10)) as usize];
                phase.0 += 1;
                phase.1 += usize::from(self.late(frame));
            }
        }
        let _ = writeln!(
            out,
            "\nLate updates by time since the previous reading, in tenths of a second \
             (late/updates): {}.",
            phases
                .iter()
                .map(|(frames, late)| format!("{late}/{frames}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    pub(crate) fn markdown(&self, baseline: &[GpuReading]) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "## Frame stages\n\n{} updates and {} render frames recorded; started at Unix time {} ms. \
             One refresh is {:.3} ms. A late update is one followed by the next at least 1.5 refreshes later. \
             Update-to-update times and time steps are paced at one refresh, so about half exceed it by jitter alone; \
             for those rows the 1.5-refresh count is the meaningful one.",
            self.frames.len(),
            self.record.render.len(),
            self.record
                .unix_ms
                .map_or("unknown".into(), |unix_ms| unix_ms.to_string()),
            ms(self.refresh)
        );
        for (at, width, height, scale) in &self.record.windows {
            let _ = writeln!(
                out,
                "- Window {width}×{height} physical pixels at scale factor {scale}, from {:.1} s.",
                *at as f64 / 1e9
            );
        }
        for (at, on) in &self.record.sampling {
            let _ = writeln!(
                out,
                "- GPU readings {} at {:.1} s.",
                if *on { "on" } else { "off" },
                *at as f64 / 1e9
            );
        }
        let _ = writeln!(
            out,
            "\n| Condition | Updates | Seconds | Focused seconds | Updates per second | Late | Late % |\n\
             | --- | --- | --- | --- | --- | --- | --- |"
        );
        for condition in Condition::ALL {
            let frames: Vec<&Frame> = self
                .of(condition)
                .filter(|frame| frame.period.is_some())
                .collect();
            if frames.is_empty() {
                continue;
            }
            let total = seconds(frames.iter().copied());
            let focused = seconds(
                frames
                    .iter()
                    .copied()
                    .filter(|frame| frame.main.has(FOCUSED)),
            );
            let late = frames.iter().filter(|frame| self.late(frame)).count();
            let _ = writeln!(
                out,
                "| {} | {} | {total:.1} | {focused:.1} | {:.1} | {late} | {:.2} |",
                condition.name(),
                frames.len(),
                frames.len() as f64 / total,
                100.0 * late as f64 / frames.len() as f64
            );
        }
        for condition in Condition::ALL {
            self.stages(&mut out, condition);
            self.late_frames(&mut out, condition);
        }
        self.time_steps(&mut out);
        self.gpu(&mut out, baseline);
        self.sampling(&mut out);
        out
    }
}

fn seconds<'a>(frames: impl Iterator<Item = &'a Frame>) -> f64 {
    frames.filter_map(|frame| frame.period).sum::<u64>() as f64 / 1e9
}

fn render_rest(frame: &RenderFrame) -> u64 {
    frame
        .total
        .saturating_sub(frame.acquire + frame.graph + frame.present)
}

/// `FILE [--baseline FILE]`. The baseline holds the four columns of `nvidia-smi`'s
/// `csv,noheader,nounits` format with the app closed.
pub fn run(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let read =
        |path: &str| std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"));
    let mut file = None;
    let mut baseline = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--baseline" => {
                let path = arguments.next().ok_or("--baseline needs a file")?;
                baseline = read(&path)?.lines().filter_map(GpuReading::parse).collect();
                if baseline.is_empty() {
                    return Err(format!("{path}: no GPU readings"));
                }
            }
            _ if file.is_none() => file = Some(argument),
            _ => return Err(format!("unexpected argument `{argument}`")),
        }
    }
    let path = file.ok_or("usage: --frame-stage-report FILE [--baseline FILE]")?;
    let analysis = Analysis::new(Record::parse(&read(&path)?)?, DISPLAY_REFRESH);
    print!("{}", analysis.markdown(&baseline));
    Ok(())
}

#[cfg(test)]
mod tests;
