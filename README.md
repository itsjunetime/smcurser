# SMCurser

A tui client for [SMServer](https://github.com/ianwelker/smserver), written in Rust using the [tui-rs framework](https://github.com/fdehau/tui-rs). This is a rewrite of [SMServer_receiver](https://github.com/ianwelker/smserver_receiver), but using Rust, fixing a lot of issues, and adding a lot of features.

## New Features
- Typing indicators
- Ability to send tapbacks per-text
- Way faster, with much lower CPU & memory usage
- Optional configuration file for persistent customization
- Support for resizing the terminal without everything getting messed up
- New Colorschemes
- Vim-like scrolling of multiple items at a time
- Tab completion when sending files
- No more weird flashes upon refreshing
