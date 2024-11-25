use criterion::criterion_group;
use criterion::criterion_main;
use criterion::BenchmarkId;
use criterion::Criterion;

use freminal_terminal_emulator::state::internal::Buffer;
use std::io::Read;

fn load_random_file() -> Vec<u8> {
    println!("Loading random_crap.txt from ../speed_tests/random_crap.txt");
    // load random_crap.txt from ../speed_tests/random_crap.txt
    let path = std::path::Path::new("../speed_tests/random_crap.txt");
    let file = std::fs::File::open(path).unwrap();

    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    buffer
}

fn bench_display_vec_tchar_as_string(bench: &mut Criterion) {
    let data = load_random_file();

    // create a Buffer
    let mut group = bench.benchmark_group("display_vec_tchar_as_string");
    group.bench_with_input(BenchmarkId::from_parameter("test"), &data, |b, data| {
        b.iter(|| {
            let mut buf = Buffer::new(100, 80);

            let response = buf
                .terminal_buffer
                .insert_data(&buf.cursor_state.pos, data)
                .unwrap(); // insert data into the buffer

            buf.format_tracker
                .push_range_adjustment(response.insertion_range);
            buf.format_tracker
                .push_range(&buf.cursor_state, response.written_range);
            buf.cursor_state.pos = response.new_cursor_pos;
        });
    });

    group.finish();
}

criterion_group!(benches, bench_display_vec_tchar_as_string);
criterion_main!(benches);
