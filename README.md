# freminal

[![codecov](https://codecov.io/gh/fredclausen/freminal/graph/badge.svg?token=H03IXCMD1Y)](https://codecov.io/gh/fredclausen/freminal)
[![GitHub license](https://img.shields.io/github/license/Naereen/StrapDown.js.svg)](https://github.com/fredclausen/freminal/LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/Naereen/StrapDown.js.svg)](https://github.com/fredclausen/freminal/issues/)
[![GitHub pull-requests](https://img.shields.io/github/issues-pr/Naereen/StrapDown.js.svg)](https://GitHub.com/fredclausen/freminal/pull/)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)

````

```bash



```shell

Alternatively, if you have pre-commit installed, you can run the following command:

And ensure that all tests pass. If you're adding a new feature, please add tests for it. If you're fixing a bug, please add a test that would have caught the bug. And lastly, please ensure your commits are signed.

As of now, I would consider this mostly stable, but hardly performant. Please see the [TODO](TODO.md) for a list of features that are planned. Please see [Supported Control Codes](SUPPORTED_CONTROL_CODES.md) for a list of control codes that are currently supported.

Before you submit a PR, please run the following commands:
cargo install cargo-docs-rs typos-cli cargo-deny cargo-xtask
cargo xtask ci

## SSH or why does the terminal act weird?

For remote sessions, like SSH, you may see odd things like duplicated input characters and such. This is because there are no termcaps installed on the remote host for freminal. Maybe, eventually, the termcaps can be included in distributions, but in the mean time the following command (run inside of freminal) will install the termcaps on the remote host:

## Description

Freminal (or "Fred's Terminal") is a terminal emulator, written in Rust, that the world didn't ask for or need. It's simply a personal passion project because I thought it would be cool to have a tool written by myself that I use every day.

Freminal is meant to emulate the old VT220s; however it supports (or will support) modern xterm control codes, unicode, and other modern features.

If you don't have the following tools installed, you can install them with the following commands:
infocmp -x | ssh YOUR-SERVER -- tic -x -

OR if you have installed the git hooks for the repo clippy, fmt and the ci xtask are run and everything will be good.
pre-commit run --all-files

## Contributing

PRs are welcome. To make sure that your PR is accepted, there are a few things I ask.

## Credit

This was originally forked from [sphaerophoria/termie](https://github.com/sphaerophoria/termie). Sphaerophoria's project was my introduction to his youtube and twitch channels, and this project really intrigued me.
