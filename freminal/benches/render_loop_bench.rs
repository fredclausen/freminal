// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use egui::{Context, FontDefinitions, FontFamily, Id, Layout, Ui, UiBuilder};
use freminal::gui::terminal::render_terminal_text;
use std::time::Duration;

fn make_bench_ui() -> (Context, Ui) {
    let ctx = Context::default();

    // --- Load a monospace font for deterministic glyph widths ---
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
    ctx.begin_pass(egui::RawInput::default()); // ensures font atlas is initialized

    // --- Create a dummy Ui for layout/render benchmarking ---
    let id = Id::new("bench-ui");
    let builder = UiBuilder::new().layout(Layout::top_down_justified(egui::Align::LEFT));
    let ui = Ui::new(ctx.clone(), id, builder);

    (ctx, ui)
}

fn bench_render(c: &mut Criterion) {
    // --- Prepare test data ---
    let text = "abcdefghij\n".repeat(10_000); // ~100k chars
    let font_size = 14.0;
    //let mut row_cache = Vec::new();
    let job = egui::text::LayoutJob::default();

    // --- Create context + ui once ---
    let (_ctx, mut ui) = make_bench_ui();

    // --- Configure benchmark group ---
    let mut group = c.benchmark_group("render_terminal_text");
    group
        .sampling_mode(SamplingMode::Flat) // fixed iteration cost
        .sample_size(75) // 75 samples = good balance for ms-level work
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .noise_threshold(0.02); // ignore ≤2% noise

    group.bench_with_input(
        BenchmarkId::from_parameter("10k_lines"),
        &text,
        |b, data| {
            b.iter(|| {
                render_terminal_text(&mut ui, data, &job, font_size);
            });
        },
    );

    group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .confidence_level(0.95)
        .significance_level(0.05)
        .configure_from_args()
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = bench_render
}
criterion_main!(benches);
