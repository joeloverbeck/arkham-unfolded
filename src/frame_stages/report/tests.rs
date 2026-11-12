use super::*;

const MS: u64 = 1_000_000;
const CUE_SHOWN: u8 = FOCUSED | CUE;

fn frame(number: u64, start_ms: u64, flags: u8, camera_step: f32) -> MainFrame {
    MainFrame {
        frame: number,
        start: start_ms * MS,
        end: start_ms * MS + 2 * MS,
        flags,
        camera_step,
        ..Default::default()
    }
}

/// Updates every 7 ms from `from_ms`, numbered from `first`.
fn run(first: u64, from_ms: u64, count: u64, flags: u8, camera_step: f32) -> Vec<MainFrame> {
    (0..count)
        .map(|index| frame(first + index, from_ms + index * 7, flags, camera_step))
        .collect()
}

#[test]
fn rows_survive_a_round_trip() {
    let rows = [
        Row::Start {
            unix_ms: 1_794_000_000_123,
        },
        Row::Main(MainFrame {
            frame: 7,
            start: 100,
            step: 6_900_000,
            first: 1,
            pre: 2,
            update: 3,
            post: 4,
            last: 5,
            wait: 6,
            extract: 7,
            end: 200,
            flags: CUE_SHOWN | SAMPLING,
            camera_step: 0.0625,
        }),
        Row::Render(RenderFrame {
            frame: 7,
            start: 150,
            acquire: 1,
            graph: 2,
            present: 3,
            total: 9,
        }),
        Row::Gpu {
            at: 300,
            reading: GpuReading([Some(37.0), Some(120.55), Some(61.0), None]),
        },
        Row::Sampling { at: 400, on: false },
        Row::Window {
            at: 500,
            width: 3200,
            height: 2000,
            scale: 2.0,
        },
    ];
    for row in rows {
        assert_eq!(row.to_string().parse::<Row>(), Ok(row), "{row}");
    }
    assert_eq!(
        GpuReading::parse("37, 120.55, 61, [N/A]"),
        Some(GpuReading([Some(37.0), Some(120.55), Some(61.0), None]))
    );
    assert_eq!(GpuReading::parse("37, 120.55, 61"), None);
    // The default `csv` format's header and units are not readings.
    assert_eq!(
        GpuReading::parse("utilization.gpu [%], power.draw [W], temperature.gpu, fan.speed [%]"),
        None
    );
    assert_eq!(GpuReading::parse("37 %, 120.55 W, 61, 35 %"), None);
}

#[test]
fn a_record_drops_only_a_torn_last_line() {
    let record = Record::parse("H,5\nS,10,1\nG,20,1,2,3,4\nM,1,2").unwrap();
    assert_eq!(record.unix_ms, Some(5));
    assert_eq!(record.sampling, [(10, true)]);
    assert_eq!(record.gpu.len(), 1);
    assert!(record.main.is_empty());
    assert!(Record::parse("H,5\nM,1,2\nS,10,1").is_err());
}

#[test]
fn conditions_follow_camera_motion_the_cue_and_the_settled_table() {
    // The table is settled until the pan starts 100 ms later.
    let mut frames = vec![frame(1, 0, CUE_SHOWN | IDLE, 0.0)];
    frames.extend(run(2, 100, 10, CUE_SHOWN, 0.05));
    // A drag leaves updates without movement between pointer events.
    frames[5].camera_step = 0.0;
    frames.extend(run(12, 170, 100, CUE_SHOWN, 0.0));
    frames.extend(run(112, 870, 3, CUE_SHOWN | REDUCED_MOTION, 0.0));
    frames.push(frame(115, 1_100, CUE_SHOWN | REDUCED_MOTION | IDLE, 0.0));
    // An open menu hides the choices.
    frames.push(frame(116, 1_350, FOCUSED | IDLE, 0.0));
    let conditions = classify(&frames);

    assert_eq!(conditions[0], Condition::Settled, "just before the pan");
    assert!(conditions[1..11].iter().all(|c| *c == Condition::Panning));
    // The last movement is at 163 ms; the camera counts as moving for 150 ms more
    // and as still from 500 ms after it.
    assert_eq!(conditions[11 + 20], Condition::Panning, "at 310 ms");
    assert_eq!(conditions[11 + 21], Condition::Other, "at 317 ms");
    assert_eq!(conditions[11 + 70], Condition::Other, "at 660 ms");
    assert_eq!(conditions[11 + 71], Condition::StillWithCue, "at 667 ms");
    assert_eq!(
        conditions[111],
        Condition::Other,
        "steady cue, not yet settled"
    );
    assert_eq!(conditions[114], Condition::Settled);
    assert_eq!(
        conditions[115],
        Condition::Other,
        "settled with no choice shown"
    );
}

#[test]
fn a_summary_ranks_values_and_counts_those_over_one_refresh() {
    // 0.1 ms to 12 ms: one refresh is 6.94 ms and 1.5 refreshes are 10.42 ms.
    let values = (1..=120).map(|value| value * 100_000);
    assert_eq!(
        summarise(values, DISPLAY_REFRESH),
        Some(Summary {
            count: 120,
            median: 6_000_000,
            p95: 11_400_000,
            max: 12_000_000,
            over: 51,
            late: 16,
        })
    );
    assert_eq!(summarise(std::iter::empty(), DISPLAY_REFRESH), None);
    assert_eq!(percentile(&[4], 0.05), 4);
}

#[test]
fn a_late_update_is_measured_to_the_next_one_and_names_the_render_frame_it_waited_for() {
    let mut text = String::from("H,1\n");
    let mut frames = run(1, 0, 4, CUE_SHOWN, 0.05);
    // The third update is followed 14 ms later, and waited 6 ms for frame 2's render.
    frames[2].wait = 6 * MS;
    frames[3].start = 28 * MS;
    // Update 6 is missing, so update 5 has no measurable period.
    frames.push(frame(5, 35, CUE_SHOWN, 0.05));
    frames.push(frame(7, 60, CUE_SHOWN, 0.05));
    for frame in &frames {
        text.push_str(&format!("{}\n", Row::Main(*frame)));
    }
    let render = RenderFrame {
        frame: 2,
        acquire: 5 * MS,
        total: 8 * MS,
        ..Default::default()
    };
    text.push_str(&format!("{}\n", Row::Render(render)));
    let analysis = Analysis::new(Record::parse(&text).unwrap(), DISPLAY_REFRESH);

    let periods: Vec<_> = analysis.frames.iter().map(|frame| frame.period).collect();
    assert_eq!(
        periods,
        [
            Some(7 * MS),
            Some(7 * MS),
            Some(14 * MS),
            Some(7 * MS),
            None,
            None
        ]
    );
    assert_eq!(analysis.frames[2].gap, Some(12 * MS));
    let late: Vec<_> = analysis
        .frames
        .iter()
        .filter(|frame| analysis.late(frame))
        .map(|frame| frame.main.frame)
        .collect();
    assert_eq!(late, [3]);
    assert_eq!(analysis.awaited(&analysis.frames[2]), Some(&render));
    assert_eq!(analysis.awaited(&analysis.frames[0]), None);

    let markdown = analysis.markdown(&[]);
    assert!(
        markdown.contains(
            "1 of 4 updates took at least 1.5 refreshes to the next one. \
             In 1 of them the update waited for the render thread; in 0 the update finished last."
        ),
        "{markdown}"
    );
    // Mean on late updates, how many of those exceeded one refresh, mean on the rest.
    for row in [
        "| Update to update | 14.00 | 1 | 7.00 |",
        "| Handoff wait | 6.00 | 0 | 0.00 |",
        "| Awaited render frame | 8.00 | 1 | – |",
        "| · Surface image wait | 5.00 | 0 | – |",
    ] {
        assert!(markdown.contains(row), "{row} in\n{markdown}");
    }
}

#[test]
fn a_gpu_reading_belongs_to_the_condition_that_held_for_the_second_before_it() {
    // 1.4 s of panning, then a still camera with the cue.
    let mut frames = run(1, 0, 200, CUE_SHOWN, 0.05);
    frames.extend(run(201, 1_400, 400, CUE_SHOWN, 0.0));
    let text: String = frames
        .iter()
        .map(|frame| format!("{}\n", Row::Main(*frame)))
        .collect();
    let analysis = Analysis::new(Record::parse(&text).unwrap(), DISPLAY_REFRESH);

    assert_eq!(
        analysis.condition_before(1_200 * MS),
        Some(Condition::Panning)
    );
    assert_eq!(
        analysis.condition_before(2_000 * MS),
        None,
        "the camera stopped"
    );
    assert_eq!(
        analysis.condition_before(3_200 * MS),
        Some(Condition::StillWithCue)
    );
    assert_eq!(
        analysis.condition_before(9_000 * MS),
        None,
        "nothing recorded"
    );
}

#[test]
fn msaa_rows_survive_a_round_trip() {
    for row in [
        Row::MsaaSupport {
            at: 10,
            counts: 2 | 4 | 8,
        },
        Row::Msaa {
            at: 20,
            samples: 2,
            allowed: false,
        },
        Row::Msaa {
            at: 30,
            samples: 1,
            allowed: true,
        },
    ] {
        assert_eq!(row.to_string().parse::<Row>(), Ok(row), "{row}");
    }
}

/// The cells of the table row that starts with `prefix`, after the prefix.
fn cells(markdown: &str, prefix: &str) -> Vec<String> {
    let line = markdown
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no row `{prefix}` in\n{markdown}"));
    line[prefix.len()..]
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

#[test]
fn msaa_stretches_leave_out_startup_and_settling_and_keep_each_setting_apart() {
    const S: u64 = 1_000 * MS;
    // As recorded: readings start just before the start setting's row.
    let mut text = String::from("H,1\nS,0,1\nA,1000,4,1\nX,1000,14\n");
    // Off at 5 s; a refused 2× and a repeated off change nothing; 4× at 15 s; readings
    // stop at 20 s.
    text.push_str(&format!(
        "A,{},1,1\nA,{},2,0\nA,{},1,1\nA,{},4,1\nS,{},0\n",
        5 * S,
        10 * S + S / 2,
        12 * S,
        15 * S,
        20 * S
    ));
    // Panning every 7 ms for 25 s, with one late update just before 1, 6, 10, 19 and 22 s.
    let late_before = [1_000, 6_000, 10_000, 19_000, 22_000];
    let (mut start, mut number) = (0, 1);
    while start < 25_000 {
        text.push_str(&format!("{}\n", Row::Main(frame(number, start, 0, 0.05))));
        let late = late_before.iter().any(|at| start < *at && start + 7 >= *at);
        start += if late { 14 } else { 7 };
        number += 1;
    }
    // At 2 s the startup; at 15.5 s straddling a change; at 9.5 and 19.5 s settled.
    for (at_ms, power) in [(2_000, 102), (9_500, 109), (15_500, 999), (19_500, 119)] {
        text.push_str(&format!("G,{},50,{power},70,60\n", at_ms * MS));
    }
    let analysis = Analysis::new(Record::parse(&text).unwrap(), DISPLAY_REFRESH);

    assert_eq!(
        analysis.stretches,
        [
            Stretch {
                from: 1000,
                samples: 4,
                sampling: true,
                settled: u64::MAX,
            },
            Stretch {
                from: 5 * S,
                samples: 1,
                sampling: true,
                settled: 8 * S,
            },
            Stretch {
                from: 15 * S,
                samples: 4,
                sampling: true,
                settled: 18 * S,
            },
            Stretch {
                from: 20 * S,
                samples: 4,
                sampling: false,
                settled: 18 * S,
            },
        ]
    );
    let reading = |power: f32| GpuReading([Some(50.0), Some(power), Some(70.0), Some(60.0)]);
    assert_eq!(
        analysis.settled_readings(),
        [
            (1, Condition::Panning, reading(109.0)),
            (2, Condition::Panning, reading(119.0))
        ]
    );

    let markdown = analysis.markdown(&[]);
    assert!(
        markdown.contains("The adapter allows MSAA off, 2×, 4×, 8× for the table's formats."),
        "{markdown}"
    );
    assert!(markdown.contains("- 10.5 s: 2× refused."), "{markdown}");
    // The late update before 1 s fell in the startup, the one before 6 s while off settled.
    let left_out = markdown
        .lines()
        .find(|line| line.contains("prepares new pipelines"))
        .unwrap();
    assert!(
        left_out.contains(", 1 late), and the first 3 s after each change")
            && left_out.contains(" updates, 1 late). A GPU reading"),
        "{left_out}"
    );
    assert!(
        !markdown.contains("| 1 | 0.0 |"),
        "the startup stretch has no row"
    );

    // Updates, seconds, late, late %, two render medians, readings, then four GPU cells.
    let off = cells(&markdown, "| 2 | 5.0 | off | yes | Panning |");
    assert_eq!((off[1].as_str(), off[2].as_str()), ("7.0", "1"));
    assert_eq!((off[4].as_str(), off[6].as_str()), ("–", "1"));
    assert_eq!(off[8], "109 (109–109)", "power");
    let four = cells(&markdown, "| 3 | 15.0 | 4× | yes | Panning |");
    assert_eq!(
        (four[1].as_str(), four[2].as_str(), four[8].as_str()),
        ("2.0", "1", "119 (119–119)")
    );
    let unread = cells(&markdown, "| 4 | 20.0 | 4× | no | Panning |");
    assert_eq!(
        (unread[1].as_str(), unread[2].as_str(), unread[6].as_str()),
        ("5.0", "1", "0")
    );

    let by_setting = cells(&markdown, "| 4× | yes | Panning |");
    assert_eq!(
        (by_setting[1].as_str(), by_setting[2].as_str()),
        ("2.0", "1")
    );
    assert!(
        markdown.contains("| off | yes | Panning |") && markdown.contains("| 4× | no | Panning |"),
        "{markdown}"
    );
}
