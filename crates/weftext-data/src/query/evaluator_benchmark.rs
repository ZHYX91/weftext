//! Measure the compiled expression evaluator on synthetic, already-decoded rows.
//! Run with cargo test -p weftext-data --release --lib evaluator_benchmark -- --ignored --nocapture.
use super::*;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn text(value: impl Into<String>) -> Cell {
    Cell::value(NativeValue::Text(value.into()))
}

fn rows(count: usize) -> Vec<BTreeMap<String, Cell>> {
    (0..count)
        .map(|i| {
            BTreeMap::from([
                ("title".into(), text(format!("任务 Task {i}"))),
                ("body".into(), text("会议 notes / minutes. ".repeat(12))),
                (
                    "status".into(),
                    text(if i % 2 == 0 { "open" } else { "done" }),
                ),
                (
                    "priorities".into(),
                    Cell::value(NativeValue::Array(vec![
                        NativeValue::Int((i % 5).to_string()),
                        NativeValue::Int("1".into()),
                    ])),
                ),
            ])
        })
        .collect()
}

#[test]
#[ignore = "synthetic expression throughput benchmark"]
fn evaluator_benchmark() {
    let sizes =
        std::env::var("WEFTEXT_BENCH_ROWS").unwrap_or_else(|_| "1000,10000,50000,100000".into());
    let samples: usize = std::env::var("WEFTEXT_BENCH_SAMPLES")
        .map_or(Ok(20), |s| s.parse())
        .unwrap();
    assert!(samples > 0);
    let schema = Schema::bundled().unwrap();
    let mut vars = symbols(&schema).unwrap();
    vars.insert("priorities".into(), QType::List(Box::new(scalar("int"))));
    println!("workload,rows,samples,cold_us,p50_us,p95_us");
    for size in sizes.split(',').map(|s| s.parse::<usize>().unwrap()) {
        let input = rows(size);
        for (workload, source, expected) in [
            ("scalar", "status == 'open'", size.div_ceil(2)),
            (
                "record",
                "{'status': status, 'name': title}.status == 'open'",
                size.div_ceil(2),
            ),
            (
                "collection",
                "priorities.exists(p, p >= 3)",
                (size / 5) * 2 + (size % 5).saturating_sub(3),
            ),
        ] {
            let start = Instant::now();
            let compiled = compile(source, &schema, &vars).unwrap();
            let env = query_env_traced(&schema, None).unwrap();
            let run = || {
                let mut budget = u64::MAX;
                input
                    .iter()
                    .filter(|cells| {
                        let result =
                            evaluate(&compiled, cells, &schema, &mut budget, &env).unwrap();
                        matches!(
                            black_box(result),
                            Cell::Value {
                                value: NativeValue::Bool(true)
                            }
                        )
                    })
                    .count()
            };
            assert_eq!(run(), expected);
            let cold = start.elapsed();
            let mut timings = Vec::with_capacity(samples);
            for _ in 0..samples {
                let start = Instant::now();
                assert_eq!(run(), expected);
                timings.push(start.elapsed());
            }
            timings.sort_unstable();
            let percentile = |p: usize| -> Duration { timings[(samples * p).div_ceil(100) - 1] };
            println!(
                "{workload},{size},{samples},{},{},{}",
                cold.as_micros(),
                percentile(50).as_micros(),
                percentile(95).as_micros()
            );
        }
    }
}
