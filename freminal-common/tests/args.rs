// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::args::Args;

#[test]
fn test_args_parse() {
    let args = Args::parse(vec!["freminal".to_string()].into_iter()).unwrap();
    assert_eq!(args.recording, None);
    assert_eq!(args.shell, None);

    let args = Args::parse(
        vec![
            "freminal".to_string(),
            "--recording-path".to_string(),
            "test".to_string(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(args.recording, Some("test".to_string()));
    assert_eq!(args.shell, None);

    let args = Args::parse(
        vec![
            "freminal".to_string(),
            "--shell".to_string(),
            "test".to_string(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(args.recording, None);
    assert_eq!(args.shell, Some("test".to_string()));

    let args = Args::parse(
        vec![
            "freminal".to_string(),
            "--recording-path".to_string(),
            "test".to_string(),
            "--shell".to_string(),
            "test".to_string(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(args.recording, Some("test".to_string()));
    assert_eq!(args.shell, Some("test".to_string()));
}

#[test]
fn test_invalid_arg() {
    let args = Args::parse(
        vec![
            "freminal".to_string(),
            "--recording-path".to_string(),
            "test".to_string(),
            "--invalid".to_string(),
        ]
        .into_iter(),
    );
    assert!(args.is_err());
}
