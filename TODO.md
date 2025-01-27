# TODO

## All Versions

- [ ] freminal-common to 100% testing Code Coverage\*
- [ ] freminal-terminal-emulator to 100% testing Code Coverage\*
- [ ] Remove custom portable-pty/filedescriptor from the freminal repo and go back to using the one from crates.io\*\*

\* Please see the [Code Coverage](https://codecov.io/gh/fredclausen/freminal) for the current status.

\*\* This is a temporary solution to a problem that I had with the portable-pty crate not being updated on crates.io but it had been updated in the wezterm repo. The published one was using a very old version of a crate used to interact with the file descriptor. This is a temporary solution until the portable-pty crate is updated on crates.io.

## Version 0.2.0

Will be focused on performance improvements. Very likely, this will mean a rewrite/improvement to the way that the terminal emulator is handling the internal buffer

## Version 0.3.0

Will be focused on moving from the current text box we're drawing to using raw OpenGL and shaders. This will allow for more advanced features like ligatures, supporting OSC 1337 for image display, and other features that are not possible with the current text box.

## Version 0.4.0

Supporting more advanced features like OSC 1337 for image display, as well as improving coverage of supported escape codes.
