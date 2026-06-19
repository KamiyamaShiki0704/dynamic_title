# PLANS

## Active Plan: Nightreign StaffRollScreen Movie Binding

1. Read the latest NR `_mod\dynamic-title-bg.log`. Completed: hooks installed, no call records.
2. Check whether `probe_staffroll_screen` hooks installed cleanly. Completed.
3. Check whether `NR slot17/movie init`, `NR movie setup`, and lambda hooks are called during title/main menu. Completed: no calls observed.
4. Interpret logged fields if calls appear:
   - `StaffRollScreen+0x188/+0x198`
   - `+0x5A0`
   - callback/list counts at `+0xCF0` and `+0xD50`
   - likely movie/status fields around `+0xE00/+0xEBC/+0xECC`
5. Continue static reverse engineering around:
   - `MovieWait`
   - `Main/Movie`
   - `MENU_DummyMovie`
   - all lambda vtables adjacent to the StaffRollScreen block
6. Identify whether the current probes were installed too late or targeted the wrong subset.
7. If adding broader probes is needed, write a specific implementation plan here before editing code.

## Candidate Next Probe Plan

Static analysis now supports a narrow version of this plan.

1. Reduce the `probe_staffroll_screen` install delay from 2 seconds to a short delay, because title initialization may occur early.
2. Add a specific NR `StaffRollScreen` slot2 hook at `0x975A70`, because static analysis shows this function directly references `MovieWait`.
3. Log the same StaffRollScreen fields as existing slot hook, especially `+0xE00`, `+0xEBC`, and `+0xECC`.
4. Keep the previous NR slot17/setup/lambda hooks for one more test, but do not add all StaffRollScreen slots yet.
5. Keep logging capped and read-only.
6. Keep Bink/Movie/D3D probes off during this test.

## Deferred Broader Probe

Executed after slot2 did not hit:

1. Added optional `probe_staffroll_broad`.
2. When enabled on NR, it hooks every NR StaffRollScreen vtable slot `0x7783B0`, `0x9757B0`, `0x975A70`, `0x778430`, `0x78C980`, `0x78D250`, `0x78DB50`, `0x78EBD0`, `0x78EBA0`, `0x78EAB0`, `0x778420`, `0x7783F0`, `0x78CAC0`, `0x78E1A0`, `0x778410`, `0x78EA60`, `0x78DEA0`, `0x78FEE0`.
3. Kept this behind a separate explicit config key.
4. Broad mode still leaves Bink/Movie/D3D probes off.
5. Result: unsafe in NR. All hooks installed, no call logs, startup crashed before main menu. Do not use this runtime approach again.

## Next Static Plan

1. Stop probing all StaffRollScreen vtable slots at runtime.
2. Reverse earlier constructor/registration sites around:
   - `0x9757B0`
   - `0x975A70`
   - StaffRollScreen vtable `0x2C549E8`
   - strings `MovieWait`, `Main/Movie`, `MENU_DummyMovie`
   - adjacent lambda vtables around `0x2C54B28..0x2C54CB8`
3. Identify a single stable function to probe, preferably one that registers status/lambda data and has a normal prolog.
4. If a new runtime probe is needed, do not hook tiny thunk/ret/jmp functions; add a narrow single-function probe only after updating this plan.

## Guardrails

- Do not re-enable global D3D12 draw hooks.
- Keep probes default-off unless explicitly testing.
- Prefer high-level StaffRollScreen/Scene/GFX binding probes over render-chain probes.
- Keep ER deployment in safe all-off state unless intentionally running a narrow test.

## Active Plan: CSMovieImp Global Owner Probe

The constructor probe installed cleanly but did not hit in the latest NR run, while the stable CSMovieIns open helper still captured `movie:/00001010.bk2`. Move the next read-only observation to the MovieImp singleton relation.

1. Keep broad StaffRollScreen and D3D draw probes disabled.
2. Do not add new hooks for this stage; extend only the existing `probe_movie_ins` logging.
3. When `probe_movie_ins` logs a CSMovieIns object, read the module-specific global `CSMovieImp@CS` singleton pointer:
   - Nightreign: `main.exe+0x442E0A8`.
   - Elden Ring: `main.exe+0x45878A8`.
4. Log `CSMovieImp+0x38`, `+0x40`, `+0x48`, `+0x50`, and `+0x54`, and compare whether `CSMovieImp+0x38` equals the current `CSMovieIns`.
5. Build and deploy the read-only probe to NR and ER, keeping ER ini all-off.
6. Use the next NR run to confirm whether the title BK2 `CSMovieIns` is owned by the global MovieImp singleton or by a separate title-specific object.

## Active Plan: ER CSMovieImp Direct Setup Experiment

The latest NR log confirmed `CSMovieImp+0x38` matches the `CSMovieIns` that opens `movie:/00001010.bk2`. Static reverse then connected NR title flow:

1. `StaffRollScreen` constructor binds `Main/Movie` to `this+0xE48` through `0x792FA0`.
2. It registers `MENU_DummyMovie` into the common title container through `0x78DF20`.
3. The `OneShot` lambda at `0x9764A0` gets the global `CSMovieImp`, reads movie id from `StaffRollScreen+0xEA8`, calls `0xF67AA0`.
4. `0xF67AA0` formats `movie:/%08d.bk2` and calls `CSMovieIns` setup (`0xF694D0` thunk, real code around `0x71DCC0`), which later reaches `0xF6A0E0`.

Next code experiment:

1. Add a default-off config key `movie_imp_trigger`.
2. ER only: after a configurable delay, read `global[main.exe+0x45878A8]`, then `CSMovieImp+0x38`.
3. Call ER `CSMovieIns` setup function `main.exe+0xE20F90` directly with:
   - path `movie:/00001010.bk2`
   - volume from config, default `0.7`
   - movie/present option values matching the observed NR title setup as closely as ER's layout allows.
4. Log before/after `CSMovieImp` and `CSMovieIns` fields using the existing movie-ins layout logger.
5. Do not call SYSTEX/MovieStart, do not enable D3D draw hooks, and do not touch title SRV replacement in this experiment.
6. Enable `probe_movie_ins=true` alongside the trigger only for ER testing, so the normal ER open helper `0xE212E0` can confirm whether BinkTexture gets created.

## Follow-up Plan: ER MovieImp Stepper Signal

The first ER trigger proved `main.exe+0xE20F90` writes path/options successfully, and the step/tick probe proved one tick runs and changes MovieIns state from `0/0` to `1/1`, but the open helper `0xE212E0` still does not run. Static comparison of the ER wrapper around `0xE1F400` shows the missing post-setup actions:

1. After `E20F90` returns success, write `CSMovieImp+0x40 = CSMovieImp+0x38`.
2. Call the CSMovieImp step object at `CSMovieImp+0x08` through vtable slot `+0x20` with event/state id `0x12`.
3. Keep this inside the existing default-off `movie_imp_trigger`; do not add new render/D3D hooks.
4. Keep ER `probe_movie_ins`, `probe_movie_step`, and `probe_movie_tick` enabled for the next test so we can verify whether `0xE212E0` finally runs.

## Active Plan: Narrow CSMovieIns Probe

Static analysis after the broad StaffRollScreen crash points back to the actual movie instance open path.

1. Keep `probe_staffroll_broad=false`; do not add more StaffRollScreen vtable hooks.
2. Reuse the existing `probe_movie_ins` switch, but make it module-aware:
   - Nightreign: hook `main.exe+0xF6A0E0`.
   - Elden Ring: keep existing hook `main.exe+0xE212E0`.
3. Add NR-specific CSMovieIns field logging:
   - BinkTexture at `+0xC0`
   - path/string at `+0xC8`
   - options at `+0xF8/+0xFC/+0x100`
4. Keep ER field logging on the existing offsets:
   - BinkTexture at `+0xB8`
   - path/string at `+0xC0`
   - options at `+0xF0/+0xF4/+0xF8`
5. Enable only `probe_movie_ins=true` for the next NR test; leave Bink replacement, SYSTEX trigger, draw hooks, and StaffRoll broad off.
6. Expected output: capture the exact CSMovieIns object that opens `movie:/00001010.bk2`, so the next step can inspect whether it has any link back to `Main/Movie` or `MENU_DummyMovie`.

## Follow-up Plan: Decode CSMovieIns Path Object

The first NR `probe_movie_ins` run hit `main.exe+0xF6A0E0`, but `CSMovieIns+0xC8` is a wide string object rather than inline text.

1. Keep the narrow `probe_movie_ins` runtime target.
2. Add read-only decoding for the FD4/Dantelion wide string layout:
   - string object base at `path_offset`
   - data pointer at `path_offset+0x08` when capacity is external
   - length at `path_offset+0x18`
   - capacity at `path_offset+0x20`
3. Print decoded `fd4_wstr` text alongside the existing raw ascii/utf16/hex previews.
4. Use the decoded string for title movie marker detection.

## Active Plan: StaffRollScreen Constructor Probe

Static comparison narrowed the NR/ER difference to `StaffRollScreen` construction and binding setup.

1. Keep all broad StaffRollScreen vtable hooks disabled.
2. Add a new default-off `probe_staffroll_ctor` switch.
3. When enabled, hook only normal constructor entry points:
   - Nightreign: `main.exe+0x974E50`
   - Elden Ring: `main.exe+0x8BDD60`
4. Log constructor args and key fields before/after return:
   - common object/vtable and `+0x5A8`
   - NR movie/status fields `+0xDE8`, `+0xE00`, `+0xE48`, `+0xEA8`, `+0xEBC`, `+0xECC`
   - ER known title fields around `+0xA38`, `+0xA50`, `+0xA60`
5. Keep the probe read-only. Do not call movie open, SYSTEX, Bink replacement, or D3D draw hooks from this stage.
6. Use the resulting object/lifetime data to decide whether a later sidecar binding can safely call ER equivalents of NR binding helpers (`0x792FA0 -> ER 0x74A2F0`, `0x78DF20 -> ER 0x744490`).

## Follow-up Plan: Runtime MovieIns State Table Slots

The latest ER log confirms the CSMovieImp stepper signal runs, but MovieIns only advances state `0/0 -> 1/1` and still does not call `main.exe+0xE212E0`. The next change is narrow logging only:

1. Keep the existing ER `movie_imp_trigger`, `probe_movie_step`, `probe_movie_tick`, and `probe_movie_ins` switches.
2. Do not add render/D3D hooks and do not force-call `E212E0` yet.
3. Extend `log_movie_step` to print the runtime `MovieIns+0x08` state function table pointer and the first several slot targets.
4. Decode each slot target through `caller_rva` so state 0/state 1 can be mapped to ER functions.
5. Rebuild/deploy and run ER once more. If state 1 points to or leads toward `E212E0`, test whether a second scheduler wake is needed; if no state slot points toward open, return to static analysis of the CSMovieIns state registration.

## Follow-up Plan: MovieIns State0/State1 Runtime Probe

The state table log shows ER MovieIns state 2 is the open helper `main.exe+0xE212E0`, while the current trigger stops at state `1/1`. Static logic suggests state 0 should set repeat and immediately run state 1, but the outer step log does not prove whether state 1 actually ran.

1. Keep the experiment read-only; do not force state 2 and do not call `E212E0` directly yet.
2. Under the existing `probe_movie_step` test mode, add narrow hooks for normal-prolog state functions only:
   - state 0: `main.exe+0xE212B0`
   - state 1: `main.exe+0xE21750`
3. Log before/after key fields on the current MovieIns:
   - state `[+40/+44]`
   - repeat `[+48]`
   - path/options/open flags `[+130..+134]`
   - BinkTexture `[+B8]`
4. Keep broad vtable hooks disabled and do not hook tiny `ret`/`jmp` helpers such as `E21930`.
5. Rebuild/deploy and run ER once. If state 1 is not called, the issue is scheduling/repeat. If state 1 is called but does not advance to state 2, inspect the resource-ready check around `0x141edc770/0x141edc930` or consider forcing state 2 as a separate default-off experiment.

## Follow-up Plan: MovieIns State1 Resource Ready Probe

The latest ER runtime log proves state0 schedules state1, and state1 rejects the request by clearing `MovieIns+0x130` before state2/open can run. Static disassembly shows the decisive branch is the boolean result from `main.exe+0x1EDC930`, called at state1 return site `main.exe+0xE217D7`.

1. Keep the experiment read-only and gated behind the existing `probe_movie_step` mode.
2. Add a narrow ER hook for `main.exe+0x1EDC930`, the resource-ready helper called by state1.
3. Log only calls whose return address is the state1 callsite around `main.exe+0xE217D7`, plus the first few calls for sanity.
4. For matching calls, log:
   - `rcx` resource object pointer.
   - fields `[+00]`, `[+08]`, `[+10]`, `[+18]`.
   - short hex previews of `rcx`, `[+08]`, and `[+10]` if readable.
   - low-byte return value.
5. Do not force state2, do not call `E212E0`, and do not enable render/D3D hooks in this stage.
6. Rebuild/deploy and run ER once. If the helper returns false with empty/null resource fields, inspect the constructor helper `0x1EDC770` or test a known ER movie path. If it returns true but state1 still clears the flag, re-check the branch/return interpretation.

## Follow-up Plan: ER Root Movie Namespace Test for 00001010

The known ER movie control test proves the CSMovieImp/MovieIns playback chain works when the BK2 exists in ER's `movie:/` namespace. The next step is a file-placement control for the target title background movie.

1. Keep render/D3D hooks disabled.
2. Verify no ER/NR process is running before touching game-root files.
3. If `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\movie\00001010.bk2` does not already exist, copy the candidate from `F:\GoldenAge\movie\00001010.bk2` into that directory.
4. Back up the ER ini, then set `movie_imp_path=movie:/00001010.bk2`.
5. Clear the ER log.
6. Run ER once and check that:
   - state1 resource-ready returns `low=0x01`.
   - `main.exe+0xE212E0` creates a nonzero BinkTexture at `CSMovieIns+0xB8`.
   - audio plays.
7. Treat visual blue-background persistence as a separate binding problem; do not re-enable global D3D draw probes for this test.

## Follow-up Plan: Narrow MovieIns Render Observation

The target `movie:/00001010.bk2` now opens and plays audio in ER, with a nonzero BinkTexture at `CSMovieIns+0xB8`, but the title remains the blue GFX background. The next observation should test whether ER ever asks this active MovieIns to render.

1. Do not enable global D3D draw probes.
2. Keep the direct `movie_imp_trigger` and `probe_movie_ins` enabled so the active title MovieIns can be selected by path marker.
3. Enable only the narrow MovieIns render hook `probe_movie_render=true` (`main.exe+0xE215C0`).
4. Disable `probe_movie_step` and `probe_movie_tick` for this run to reduce log spam; the native state machine should still run without those hooks.
5. Keep `probe_movie_draw_submit=false` for the first render observation.
6. Clear the ER log and run ER once.
7. Interpret results:
   - If `movie render probe:` logs appear for the active MovieIns, inspect render result, draw arg, and inner BinkTexture relation next.
   - If no `movie render probe:` logs appear while audio plays, the active MovieIns is not attached to any visible render path, so the next step should focus on GFX/Scaleform binding to `Movie` / `MENU_DummyMovie` rather than movie playback.

## Follow-up Plan: Narrow Movie Draw Submit Observation

The active `movie:/00001010.bk2` MovieIns renders every frame and returns a stable render result, but the title remains the blue GFX background. The next observation is whether the MovieIns render output reaches the engine draw-submit path.

1. Do not enable `probe_draw_calls`; the global D3D12 draw hook remains off.
2. Keep `movie_imp_trigger=true`, `probe_movie_ins=true`, and `probe_movie_render=true` so the active MovieIns is selected and render output is tracked.
3. Enable only the existing narrow `probe_movie_draw_submit=true` hook for `main.exe+0x1AEA9A0`.
4. Keep `probe_movie_step=false` and `probe_movie_tick=false` to reduce log noise.
5. Clear the ER log and run ER once.
6. Interpret results:
   - If `movie draw submit probe:` logs appear with tracked `parent`, `render_result`, `draw_arg`, or `inner`, inspect the matched argument and decide whether the movie is being drawn under the opaque blue title layer or into an offscreen/non-visible target.
   - If no submit logs appear while `movie render probe:` continues, inspect MovieIns render internals or the return object's consumers rather than D3D draw calls.

## Follow-up Plan: Hide MENU_DummyMovie GFX Occlusion Test

The latest ER log proves `movie:/00001010.bk2` opens, renders every frame, and reaches the narrow movie draw-submit path, but the visible title remains the blue dummy layer. Static GFX comparison shows the current ER title GFX places a `Movie` sprite at root depth 2 and the title/logo sprite at depth 3, while that `Movie` sprite contains `MENU_DummyMovie`.

1. Do not re-enable global D3D draw probes.
2. Keep the native movie trigger/render/submit test config intact for ER.
3. Back up the current `F:\GoldenAge\GA\menu\05_001_title_logo.gfx`.
4. Generate a new GFX test from the current NR-order XML:
   - keep the root display object named `Movie`;
   - keep `MENU_DummyMovie` and SymbolClass entries;
   - set only the inner `PlaceObject3Tag` for `MENU_DummyMovie` to `placeFlagHasVisible=true` and `visible=0`.
5. Convert the XML back to GFX with FFDec `xml2swf`.
6. Replace only `F:\GoldenAge\GA\menu\05_001_title_logo.gfx` with this test file.
7. Run ER once:
   - if BK2 video appears behind logo/menu, the blocker was GFX dummy occlusion;
   - if the dummy disappears but video still does not appear, the native MovieIns draw path is not composited into the visible title layer and we need a real GFX texture binding/sidecar;
   - if title/logo disappears or UI breaks, restore the backup and use a more conservative GFX edit.

## Follow-up Plan: MENU_DummyMovie SRV Locator Probe

The hidden-dummy test produced a black background while the native `movie:/00001010.bk2` render/submit path remained active. This proves the visible blue was the static dummy image, but the BK2 is not bound to the GFX `Movie/MENU_DummyMovie` layer.

1. Do not modify code for this stage.
2. Keep the hidden-dummy GFX installed so the dummy image no longer masks the result.
3. Enable only the existing `probe_title_srv` CreateShaderResourceView hook in probe-only mode:
   - `probe_title_srv=true`
   - `enable_title_hijack=false`
   - `enable_dynamic_title=false`
   - `bink_plane_hijack=false`
4. Set the target probe size to the GFX-declared `MENU_DummyMovie` size:
   - `hijack_resource_width=1920`
   - `hijack_resource_height=1080`
   - `hijack_require_bc7=false`
5. Keep the current native movie trigger/render/submit probes enabled for correlation.
6. Clear the ER log and run once.
7. Interpret:
   - If a 1920x1080 SRV appears near title GFX load, it is likely the dummy/external-image texture descriptor and can be the next hijack target.
   - If no 1920x1080 SRV appears, `MENU_DummyMovie` is likely atlas-backed or resolved through another Scaleform resource path; next step should probe GFX/Scaleform image registration by string/object rather than D3D resource size.

## Follow-up Plan: Visible Dummy Descriptor Debug Fill Test

The SRV locator found five 1920x1080 descriptors. `#1/#2` are `DXGI_FORMAT(61)` and likely Bink/movie-plane resources. `#4/#5` are later `DXGI_FORMAT(98)` resources and likely GFX/static dummy resources. Before changing code, identify the visible target descriptor.

1. Restore the visible dummy GFX from `F:\GoldenAge\GA\menu\05_001_title_logo.gfx.before_hide_dummy_visible_20260619_171845`.
2. Back up the current hidden-dummy GFX and ER ini before changing them.
3. Configure ER for an existing descriptor debug-fill hijack:
   - `enable_title_hijack=true`
   - `probe_title_srv=true`
   - `hijack_title_index=4`
   - `atlas_debug_fill=255,0,0,255`
   - `hijack_resource_width=1920`
   - `hijack_resource_height=1080`
   - `hijack_require_bc7=false`
4. Disable noisy native movie probes for this visual target test unless needed:
   - `probe_movie_render=false`
   - `probe_movie_draw_submit=false`
5. Keep dangerous/global paths disabled:
   - `probe_draw_calls=false`
   - `bink_plane_hijack=false`
6. Clear the ER log and run once.
7. Interpret:
   - If the title background becomes red, descriptor `#4` is the visible dummy target.
   - If the title background remains blue/black, repeat with `hijack_title_index=5`.
8. After the visible target is identified, update `bink_plane_hijack` code to cache earlier Bink-plane source descriptors/resources and apply them when the later GFX target descriptor appears.

## Follow-up Plan: MENU_DummyMovie 64x36 Descriptor Debug Fill Test

The actual ER dummy texture was found at `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\menu\hi\05_dummy-tpf-dcx\MENU_DummyMovie.dds`. Header parsing shows it is `64x36`, one mip, `DXGI_FORMAT(98)`, not `1920x1080`.

1. Stop targeting the 1920x1080 title-sized descriptors; user confirmed #1/#2 are pre-main-menu loading resources.
2. Keep visible-dummy GFX installed so a descriptor replacement can be seen on the main title background.
3. Configure the existing SRV hijack for a narrow visual test:
   - `enable_title_hijack=true`
   - `probe_title_srv=true`
   - `hijack_resource_width=64`
   - `hijack_resource_height=36`
   - `hijack_require_bc7=false`
   - `hijack_title_index=1`
   - `atlas_debug_fill=255,0,0,255`
4. Keep movie playback and global draw probes disabled for this visual locator run.
5. Clear the ER log and run once.
6. Interpret:
   - If the main title background turns red, this is the visible `MENU_DummyMovie` descriptor.
   - If it stays blue, inspect the log for how many `64x36` candidates appeared and retarget by `hijack_title_index`.
   - If no `64x36` candidate appears, add a narrower texture-load/name probe or test file-level replacement of `MENU_DummyMovie.dds` with backup.

## Active Plan: Bridge Bink Plane Into MENU_DummyMovie Descriptor

The `64x36` debug-fill test confirmed `MENU_DummyMovie.dds` is the visible main-menu background descriptor at `hijack_title_index=1`. The existing `bink_plane_hijack` is not sufficient because it assumes source and target dimensions are the same.

1. Extend `dx12_title_texture.rs` so `bink_plane_hijack` tracks two independent resources:
   - target descriptor: the normal title match from `hijack_resource_width=64`, `hijack_resource_height=36`, `bink_plane_target_title_index=1`;
   - source plane: a Bink/movie plane with configurable source width/height/format, defaulting to `1920x1080`, `DXGI_FORMAT(61)`, `bink_plane_source_index=1`.
2. Cache the source plane COM resource and its SRV desc safely when observed. Use a cloned `ID3D12Resource` so the resource remains alive.
3. Cache the target descriptor when the `64x36` title descriptor is observed.
4. When either side appears, attempt to apply the cached source plane SRV to the cached target descriptor by calling the original `CreateShaderResourceView`.
5. Add concise log lines for:
   - target descriptor stored;
   - source plane stored;
   - bridge applied;
   - missing source/target state.
6. Add default-off/example ini keys for the source plane dimensions and format.
7. Build release, deploy the DLL to ER, and configure ER for one test:
   - visible GFX restored/kept;
   - `movie_imp_trigger=true`;
   - `probe_movie_ins=true` if useful;
   - `probe_title_srv=true`;
   - `enable_title_hijack=false`;
   - `bink_plane_hijack=true`;
   - target `64x36`, target index `1`;
   - source `1920x1080`, format `61`, source index `1`;
   - global draw hook remains disabled.

## Follow-up Plan: Correct Color Source Probe

The first bridge test displayed the BK2 in the main menu, but it was red-tinted because the source was `DXGI_FORMAT(61)`, which behaves like a single-channel Bink Y/luma plane when sampled by the GFX RGBA shader.

1. Do not change code for this stage.
2. Keep the confirmed target descriptor:
   - `hijack_resource_width=64`
   - `hijack_resource_height=36`
   - `bink_plane_target_title_index=1`
3. Test direct RGB-like candidates before implementing YUV conversion:
   - first: `bink_plane_source_width=1920`, `bink_plane_source_height=1080`, `bink_plane_source_format=98`, `bink_plane_source_index=1`;
   - if not correct, repeat with `bink_plane_source_index=2`;
   - consider `1280x720` candidates only if the 1920x1080 candidates fail and log correlation suggests they are movie-related.
4. Keep `movie_imp_trigger=true`, `probe_title_srv=true`, `bink_plane_hijack=true`, and `probe_draw_calls=false`.
5. Interpret:
   - correct colors: use that source permanently;
   - blue/static/black: source is not the converted movie texture;
   - still red/monochrome: still sampling a plane, requiring explicit YUV->RGB composition or a native shader/texture binding.

## Active Plan: Bink Plane Inventory and R8 Swizzle Probe

The `DXGI_FORMAT(98)` direct-source tests both selected loading/static images rather than BK2 RGB output. The remaining low-risk path is to keep the confirmed `64x36` `MENU_DummyMovie` target and inspect the actual Bink plane resources.

1. Keep global D3D draw hooks disabled.
2. Extend the existing CreateShaderResourceView hook only:
   - add a default-off Bink plane inventory log for source-sized and half-sized movie-plane candidates;
   - include SRV format, resource format, dimensions, descriptor, caller, and component mapping;
   - avoid logging every BC7 UI texture.
3. Add a default-off `bink_plane_source_swizzle_rrr1` option.
4. When bridging an R8 source plane into `MENU_DummyMovie`, optionally rewrite only the copied SRV desc component mapping to `R,R,R,1`.
5. Interpret the next visual result:
   - grayscale BK2: GFX sampling accepts component mapping; architecture is stable, but full color still needs UV/chroma composition;
   - still red: Scaleform/title shader ignores or overrides SRV component mapping;
   - static/loading image: source config is wrong;
   - crash/black/no UI: revert the option and inspect the log.
6. Configure the next ER test for `DXGI_FORMAT(61)`, source index `1`, `bink_plane_source_swizzle_rrr1=true`, and plane inventory enabled.

## Follow-up Plan: DXGI_FORMAT(28) Movie RGB Candidate Probe

The R8 swizzle test produced a grayscale BK2, proving the title GFX layer accepts the bridged descriptor and respects component mapping. The same Bink-open cluster also logged a `DXGI_FORMAT(28) 1920x1080` inventory entry immediately after the two R8 planes.

1. Do not change code for this stage.
2. Keep the confirmed target:
   - `hijack_resource_width=64`
   - `hijack_resource_height=36`
   - `bink_plane_target_title_index=1`
3. Retarget only the source:
   - `bink_plane_source_format=28`
   - `bink_plane_source_width=1920`
   - `bink_plane_source_height=1080`
   - `bink_plane_source_index=1`
   - `bink_plane_source_swizzle_rrr1=false`
4. Keep `bink_plane_probe_all=true`, `movie_imp_trigger=true`, `probe_title_srv=true`, and `probe_draw_calls=false`.
5. Interpret:
   - correct color BK2: use format 28 as the direct RGB movie source;
   - black/blank/static: format 28 is an intermediate render target or not sampled as expected;
   - grayscale/red: source matching or descriptor reuse still selected a plane.

## Active Plan: Prevent Atlas Hijack From Overwriting Bink Bridge

The low-noise success config still wrote the Bink RGB source into `MENU_DummyMovie`, but the old atlas/debug-fill hijack path immediately overwrote the same descriptor because `hijack_title_index=1` remained configured.

1. Keep the confirmed source/target:
   - source `DXGI_FORMAT(28) 1920x1080`, index `1`;
   - target `MENU_DummyMovie 64x36`, title index `1`.
2. Make the install path distinguish descriptor observation/bridge mode from atlas hijack mode.
3. When `bink_plane_hijack=true` but both `enable_title_hijack=false` and `enable_dynamic_title=false`, treat the title descriptor as probe/bridge-only after storing it, so `hijack_descriptor` cannot replace it with the RGBA atlas/debug fill.
4. Build, deploy, keep the low-noise ini, and clear the ER log for another verification run.

## Completed Plan: Migrate dynamic-title-bg to _Project

The current ER BK2 bridge is functionally verified inside `fromsoftware-rs-0.14.0/examples/dynamic-title-bg`. Move the working project into its own folder under `_Project` without deleting or rewriting the original example.

1. Create a new standalone project folder at `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg`.
2. Copy the current `src`, `README.md`, and `dynamic-title-bg.example.ini` from the verified example project.
3. Replace the copied `Cargo.toml` workspace-only metadata/dependencies with explicit standalone package metadata and path dependencies back to `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\crates\...`.
4. Add a short local migration note so future work knows this folder is the active isolated project copy.
5. Run `cargo build --release` from the new project folder to verify it builds outside the original workspace.
6. Update `TASK_STATUS.md` with completed migration, modified files, judgment, unresolved items, and next step.

## Completed Plan: Separate Legacy Rust Modules

Current verified ER title BK2 playback uses only the MovieImp trigger plus the `MENU_DummyMovie` SRV bridge. Separate modules that belong to rejected or superseded experiments while preserving them as reference material.

1. Keep active source in `src` limited to the modules needed by the working path:
   - `lib.rs`
   - `bink_probe.rs`
   - `dx12_title_texture.rs`
2. Move legacy modules out of `src` into `legacy_unused_rs`:
   - `dx12_draw_probe.rs`
   - `engine_flag_probe.rs`
   - `native_movie.rs`
   - `systex_movie.rs`
   - `video.rs`
3. Add `legacy_unused_rs\README.md` describing what each archived module did and why it is not part of the current build.
4. Update `lib.rs` to remove module declarations, imports, config fields, and install calls for the archived modules.
5. Keep the successful current config path intact:
   - `movie_imp_trigger`
   - `bink_plane_hijack`
   - Bink RGB source `DXGI_FORMAT(28) 1920x1080 #1`
   - target `MENU_DummyMovie 64x36 #1`
6. Build from the standalone project with `cargo build --release --offline`.
7. Update `TASK_STATUS.md` with the cleanup result and any remaining warnings.

## Active Plan: Default-Quiet Logging Switch

The user chose the second logging option: default to no log output, and only write `dynamic-title-bg.log` when explicitly enabled for debugging.

1. Add a `log_enabled` / `enable_log` config key, default `false`.
2. Stop writing early `DllMain` and config-load lines before the config is parsed.
3. Keep passing `None` as `log_path` into probe/bridge modules when logging is disabled, so existing module log guards remain effective.
4. Update the example ini and deployed ini with `log_enabled=false`.
5. Build the standalone project, deploy the rebuilt DLL to `F:\GoldenAge\dll\dynamic_title_bg`, and update `TASK_STATUS.md`.

## Active Plan: Trigger Movie Playback After Title Target Appears

The current `movie_imp_trigger` is delay-based from DLL attach, so the BK2 can start during the pre-title loading screen. Shift normal playback to the moment the visible `MENU_DummyMovie` descriptor is observed.

1. Add a config key `movie_imp_trigger_on_title_target`, defaulting to `false` for compatibility but enabled in the deployed/example ini.
2. Refactor the ER MovieImp trigger so the actual setup can be fired once either by a fixed-delay thread or by a callback.
3. Let `dx12_title_texture` accept an optional callback and invoke it once when the configured target descriptor (`64x36`, title index `1`) is stored.
4. When callback mode is enabled, treat `movie_imp_delay_ms` as a post-title-target delay, not a DLL-attach delay.
5. Build, deploy, and update `TASK_STATUS.md`.

## Active Plan: Prepare GitHub Upload

The project should be published as its own repository from `_Project\dynamic-title-bg`.

1. Exclude build outputs, logs, deployed DLLs, and machine-local config.
2. Include `_Asset` because the user explicitly wants these project assets uploaded.
3. Add a project `.gitignore` that preserves source, docs, Cargo files, and example ini.
4. Update README with a short note that game assets/BK2/GFX files are not included.
5. Initialize a local git repository, stage the intended files only, and create an initial commit.
6. If GitHub CLI or an origin URL is available, push to GitHub; otherwise stop with clear next steps for installing `gh` or providing a remote URL.
