# Contributing

Thanks for helping. Bug reports, slang suggestions and pull requests are all welcome.

## Layout

- `addon/Translate`: the WoW addon (Lua 5.1). It must load on both WoW: Forever (`## Interface: 16001`) and retail, and must handle Midnight secret values: check `issecretvalue` before reading chat text.
- `companion`: the Rust companion app (eframe/egui, xcap, ureq).
- `assets`: the icon source.

## Before you open a pull request

In `companion/`:

```
cargo test
cargo clippy --all-targets
```

On Linux, install the PipeWire, X11 and Wayland development packages listed in `.github/workflows/build.yml` first. For addon changes, test in game with `/tr test` and `/tr probe`, and say which client and interface number you used.

Keep one change per pull request. The Build workflow must pass on Windows, macOS and Linux.

## Slang and glossary terms

New WoW terms go in the `SLANG` string in `companion/src/translate.rs` as `中文=English;`. Keep entries short and use the meaning players actually use (任务 = quest, not task). Single characters match too often, so use two or more. Real-money trading terms such as GDKP are not accepted.

## Pixel strip

If you change the strip format, change both `addon/Translate/Strip.lua` and `companion/src/strip.rs`, and keep the `finds_and_decodes` test passing.
