# dynamic-title-bg Project Copy

This folder is the isolated working copy migrated from:

`F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg`

The original example was left in place. Continue feature work here unless a task explicitly targets the upstream example folder.

## Current verified ER bridge

- Playback trigger: `movie_imp_trigger=true`
- Movie path: `movie:/00001010.bk2`
- Visible GFX target: `MENU_DummyMovie`, `64x36`, `DXGI_FORMAT(98)`, title index `1`
- Movie RGB source: `1920x1080`, `DXGI_FORMAT(28)`, source index `1`
- Global D3D12 draw hook remains disabled.

Build from this folder with:

```powershell
cargo build --release
```
