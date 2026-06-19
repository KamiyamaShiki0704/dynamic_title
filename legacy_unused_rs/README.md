# Legacy Rust Modules

These files were moved out of `src` because they are not part of the currently verified ER title BK2 background path.

The working path is:

1. `src\lib.rs` reads config and installs the active pieces.
2. `src\bink_probe.rs` starts ER playback through `movie_imp_trigger` / `CSMovieImp`.
3. `src\dx12_title_texture.rs` bridges the Bink RGB SRV into the visible `MENU_DummyMovie` descriptor.

## Archived files

- `dx12_draw_probe.rs`
  - Global D3D12 draw-call probe.
  - It helped identify draw state, but runtime testing showed it can cause black screen or no UI. Do not use it for the current title background path.

- `engine_flag_probe.rs`
  - Early engine/loading flag observation probe.
  - Useful only for historical diagnostics; current gating no longer depends on it.

- `native_movie.rs`
  - Earlier direct native movie trigger experiment.
  - Superseded by the working `movie_imp_trigger` implementation in `bink_probe.rs`.

- `systex_movie.rs`
  - Earlier SYSTEX/MovieStart factory and trigger experiment.
  - It could open/play BK2 audio but did not bind video into the main menu GFX layer reliably.

- `video.rs`
  - MediaFoundation MP4 decode path for the original ImGui/dynamic atlas overlay approach.
  - Superseded by native BK2 playback plus SRV descriptor bridging. The current project no longer builds the ImGui overlay path.

Keep these files as reference material unless the project explicitly decides to delete old experiments.
