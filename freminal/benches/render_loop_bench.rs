// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use egui::{Context, FontDefinitions, FontFamily, Id, Layout, Ui, UiBuilder};
use std::time::Duration;

// --- Freminal render function ---
use freminal::gui::terminal::render_terminal_text;

/// Create a headless egui Context + Ui for benchmarking.
fn make_bench_ui() -> (Context, Ui) {
    let ctx = Context::default();

    // Load deterministic monospace font
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "monospace".to_owned(),
        egui::FontData::from_static(include_bytes!("../../res/MesloLGSNerdFontMono-Regular.ttf"))
            .into(),
    );
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "monospace".to_owned());
    ctx.set_fonts(fonts);

    // Force font atlas + style system initialization
    ctx.begin_pass(egui::RawInput::default());

    let id = Id::new("bench-ui");
    let builder = UiBuilder::new().layout(Layout::top_down_justified(egui::Align::LEFT));
    let ui = Ui::new(ctx.clone(), id, builder);

    (ctx, ui)
}

/// Benchmark both logic-only and full-render variants.
fn bench_render(c: &mut Criterion) {
    let text = "abcdefghij\n".repeat(10_000);
    let font_size = 14.0;
    let mut row_cache = Vec::new();
    let job = egui::text::LayoutJob::default();

    let (ctx, mut ui) = make_bench_ui();

    let mut group = c.benchmark_group("render_terminal_text");
    group
        .sampling_mode(SamplingMode::Flat)
        .sample_size(75)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .noise_threshold(0.02);

    // ---------- logic-only (no flush) ----------
    group.bench_with_input(
        BenchmarkId::new("logic_only", "10k_lines"),
        &text,
        |b, data| {
            b.iter(|| {
                // Just build + layout, no frame end
                render_terminal_text(&mut ui, data, &job, font_size, &mut row_cache, None);
            });
        },
    );

    // ---------- full-render (layout + paint) ----------
    group.bench_with_input(
        BenchmarkId::new("full_render", "10k_lines"),
        &text,
        |b, data| {
            b.iter(|| {
                // render + flush a simulated frame
                render_terminal_text(&mut ui, data, &job, font_size, &mut row_cache, None);
                let _ = ui.ctx().end_pass(); // triggers paint batching
            });
        },
    );

    group.finish();

    // keep ctx alive until after benchmark ends
    drop(ctx);
}

/// Criterion configuration (same tuning as before)
fn criterion_config() -> Criterion {
    Criterion::default()
        .confidence_level(0.95)
        .significance_level(0.05)
        .configure_from_args()
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = bench_render, bench_render_dirty_5pct
}
criterion_main!(benches);

fn bench_render_dirty_5pct(c: &mut Criterion) {
    let (_ctx, mut ui) = make_bench_ui();
    let text = "abcdefghij\n".repeat(10_000);
    let job = egui::text::LayoutJob::default();
    let font_size = 14.0f32;
    let mut row_cache = vec![Default::default(); text.lines().count()];

    // Initially mark all rows dirty to build caches
    let all_rows: Vec<usize> = (0..row_cache.len()).collect();
    render_terminal_text(
        &mut ui,
        &text,
        &job,
        font_size,
        &mut row_cache,
        Some(&all_rows),
    );

    // Precompute a 5%% dirty set (every 20th row)
    let dirty: Vec<usize> = (0..row_cache.len()).step_by(20).collect();

    c.bench_with_input(
        BenchmarkId::new("render_terminal_text/dirty_5pct/10k_lines", ""),
        &"",
        |b, _| {
            b.iter(|| {
                // Only pass the dirty subset
                render_terminal_text(
                    &mut ui,
                    &text,
                    &job,
                    font_size,
                    &mut row_cache,
                    Some(&dirty),
                );
            });
        },
    );
}
