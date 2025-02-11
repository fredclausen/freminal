# TODO

## All Versions

- [ ] freminal-common to 100% testing Code Coverage\*
- [ ] freminal-terminal-emulator to 100% testing Code Coverage\*
- [x] Remove custom portable-pty/filedescriptor from the freminal repo and go back to using the one from crates.io\*\*

\* Please see the [Code Coverage](https://codecov.io/gh/fredclausen/freminal) for the current status.

\*\* This is a temporary solution to a problem that I had with the portable-pty crate not being updated on crates.io but it had been updated in the wezterm repo. The published one was using a very old version of a crate used to interact with the file descriptor. This is a temporary solution until the portable-pty crate is updated on crates.io.

## Version 0.1.0

Initial release version. It will be functional, but certainly not feature complete. And perhaps more importantly, it will not be as performant as I'd like.

The below list is not a complete list of all completed tasks. It's a reflection of all the tasks that I can think of that are not completed when I created this document.

- [ ] Test suite 100% coverage on `freminal-terminal-emulator/src/ansi_components`
- [ ] Pass validation of [vttest](http://invisible-island.net/vttest/)
- [ ] Pass validation of [wraptest](https://github.com/mattiase/wraptest)
- [ ] Complete [Supported Control Codes](SUPPORTED_CONTROL_CODES.md)
- [ ] Adjust mouse mode to include all active mouse modes
- [ ] GitHub CI action to build/publish executables
  - [ ] MacOS
    - [ ] Code Sign macOS
  - [ ] Linux
    - [x] AMD64
    - [x] ARM64
  - [ ] Windows
    - [ ] Code Sign Windows
  - [ ] Tag on manual build
  - [ ] Auto publish pre-release builds

## Version 0.2.0

Will be focused on performance improvements as well as a replay system to help step through control codes that caused a problem. Very likely, this will mean a rewrite/improvement to the way that the terminal emulator is handling the internal buffer

## Version 0.3.0

Will be focused on moving from the current text box we're drawing to using raw OpenGL and shaders. This will allow for more advanced features like ligatures, supporting OSC 1337 for image display, and other features that are not possible with the current text box.

## Version 0.4.0

Supporting more advanced features like OSC 1337 for image display, as well as improving coverage of supported escape codes.
