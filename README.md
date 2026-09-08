# Dynamic Title Background

Restores an Elden Ring title-screen BK2 background by combining two engine paths:

1. Start playback through ER's native `CSMovieImp` / `CSMovieIns` movie path.
2. Bridge the native Bink RGB SRV into the visible `MENU_DummyMovie` GFX descriptor.

The title movie is intentionally one-shot. After the first confirmed gameplay
cleanup, the DLL permanently stops interacting with later movies. The captured
title texture is retained as a frozen frame so it is not released beside the
next native CG. Returning to the title screen keeps the normal logo/menu UI
over that frozen frame; the title BK2 does not restart.

Normal title playback does not install the diagnostic `CSMovieIns` init probe
or the removed global MovieIns setup/init handoff hooks.

The verified title-menu target is:

- `MENU_DummyMovie.dds`
- `64x36`
- `DXGI_FORMAT(98)`
- title descriptor index `1`

## Game Compatibility

The native title-movie path is currently validated for these Worldwide PC
executables:

- Elden Ring `2.6.2.0`
- Elden Ring `2.7.0.0`
- Elden Ring `2.7.1.0` (game version `1.17.1`; static validation complete,
  in-game verification pending)

At startup, the DLL validates the native `CSMovieIns` setup and cleanup code,
the movie-state layout, and the `CSMovieImp` global reference before enabling
the title movie pipeline. An unknown executable is treated as unsupported: the
native title movie and Bink bridge are left disabled instead of hooking
unverified addresses.

The standalone compatibility checker can validate an executable without
launching the game:

```powershell
py -3 .\_Tools\compat\check_er_compat.py "C:\path\to\ELDEN RING\Game\eldenring.exe"
```

## Build

```powershell
cd C:\path\to\dynamic-title-bg
cargo build --release
```

`Cargo.toml` pins `eldenring` and `fromsoftware-shared` to the same exact
official `vswarte/eldenring-rs` Git revision. No sibling fsrs checkout is
required. After Cargo has fetched the pinned revision once, subsequent builds
can use `cargo build --release --offline`.

## Deploy

```powershell
Copy-Item -LiteralPath .\target\release\dynamic_title_bg.dll -Destination C:\path\to\your\mod-loader\dynamic_title_bg.dll -Force
```

## Config

Put `dynamic-title-bg.ini` next to the injected DLL or next to `eldenring.exe`.

Use `dynamic-title-bg.example.ini` as the current low-noise baseline. The BK2 must be available in ER's game-root movie namespace, for example:

```text
ELDEN RING\Game\movie\00001010.bk2
```

## Assets

The `_Asset` folder contains project reference assets used while developing the title-screen bridge. Runtime BK2 files, deployed DLLs, logs, and machine-local config files are intentionally not included.

## Source Layout

Active build files:

- `src\lib.rs`
- `src\bink_probe.rs`
- `src\dx12_title_texture.rs`

Archived experiment files are in `legacy_unused_rs`.

## License

This project is licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option. See `LICENSE-APACHE`, `LICENSE-MIT`, and `LICENSE`.
