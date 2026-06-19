# Dynamic Title Background

Restores an Elden Ring title-screen BK2 background by combining two engine paths:

1. Start playback through ER's native `CSMovieImp` / `CSMovieIns` movie path.
2. Bridge the native Bink RGB SRV into the visible `MENU_DummyMovie` GFX descriptor.

The verified title-menu target is:

- `MENU_DummyMovie.dds`
- `64x36`
- `DXGI_FORMAT(98)`
- title descriptor index `1`

The verified BK2 RGB source is:

- `1920x1080`
- `DXGI_FORMAT(28)`
- Bink source index `1`

## Build

```powershell
cd F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg
cargo build --release --offline
```

The DLL is written to:

```text
F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\target\release\dynamic_title_bg.dll
```

## Deploy

```powershell
Copy-Item -LiteralPath .\target\release\dynamic_title_bg.dll -Destination F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll -Force
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
