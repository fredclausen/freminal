# freminal

[![codecov](https://codecov.io/gh/fredclausen/freminal/graph/badge.svg?token=H03IXCMD1Y)](https://codecov.io/gh/fredclausen/freminal)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)
[![GitHub license](https://img.shields.io/github/license/Naereen/StrapDown.js.svg)](https://github.com/fredclausen/freminal/LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/Naereen/StrapDown.js.svg)](https://github.com/fredclausen/freminal/issues/)
[![GitHub pull-requests](https://img.shields.io/github/issues-pr/Naereen/StrapDown.js.svg)](https://GitHub.com/fredclausen/freminal/pull/)

## Description

Freminal is a terminal emulator, written in Rust, that the world didn't ask for or need. It's simply a personal passion project.

Freminal is meant to emulate the old VT220s; however it supports (or will support) modern xterm control codes, unicode, and other modern features.

## Contributing

PRs are welcome. To make sure that your PR is accepted, there are a few things I ask.

If you don't have the following tools installed, you can install them with the following commands:

```bash
cargo install cargo-docs-rs typos-cli cargo-deny
```

Before you submit a PR, please run the following commands:

```bash
cargo xtask ci
```

Alternatively, if you have pre-commit installed, you can run the following command:

```bash
pre-commit run --all-files
```

OR if you have installed the git hooks for the repo clippy, fmt and the ci xtask are run and everything will be good.

And ensure that all tests pass. If you're adding a new feature, please add tests for it. If you're fixing a bug, please add a test that would have caught the bug. And lastly, please ensure your commits are signed.

## Credit

This was originally forked from [sphaerophoria/termie](https://github.com/sphaerophoria/termie). Sphaerophoria's project was my introduction to his youtube and twitch channels, and this project really intrigued me.
