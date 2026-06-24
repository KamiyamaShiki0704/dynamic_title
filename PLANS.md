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

### Follow-up Adjustment: Minimal Runtime Hooks For Direct New-game Crash

The latest dynamic-title log shows the title `MovieIns` native finish did complete:
ER ran the state7 cleanup path, closed the old BinkTexture, cleared `MovieIns+0xB8`,
and dropped active state before the new-game CG started. The log then reaches the
next native CG path `movie:/10010010.bk2`, so the remaining direct-start crash is
more likely caused by our diagnostic hooks staying active on the global movie/Bink
path than by incomplete title cleanup.

1. Do not auto-install `CSMovieIns` init probe just because `movie_imp_trigger`
   is enabled; install it only when `probe_movie_ins=true`.
2. Keep the functional title path intact:
   - title-target delayed `movie_imp_trigger`;
   - native-finish request on confirm input;
   - Bink plane/SRV bridge.
3. Disable the deployed diagnostic probes for the next test:
   - `probe_movie_ins=false`;
   - `probe_movie_step=false`;
   - `probe_movie_tick=false`;
   - `probe_bink_texture_open=false`.
4. Keep logging enabled only for trigger/bridge/finish monitor messages.
5. Build/deploy and test direct start new game again. If the crash disappears,
   the release path should keep movie/Bink diagnostic probes opt-in only.

### Follow-up Adjustment: Restore Dummy Descriptor On Title Stop

The minimal-hook log still shows title native finish completing cleanly. If
direct new-game still crashes, the next suspect is the frozen bridge itself:
`MENU_DummyMovie` remains pointed at the title Bink RGB resource, and the DLL
keeps a COM clone of that resource so the static video frame can remain visible.
That may keep old title movie GPU resources alive while ER immediately opens the
new-game CG movie.

1. Store the original `MENU_DummyMovie` target SRV resource/desc when the title
   descriptor is first identified.
2. Store the original `CreateShaderResourceView` trampoline pointer from the SRV
   hook so the descriptor can be restored outside the hook.
3. On title stop/native finish request, restore the title descriptor back to its
   original dummy SRV and drop the stored Bink RGB resource clone.
4. Keep Bink source capture disabled after title stop.
5. This test intentionally sacrifices the static-video-frame fallback after
   confirm/return; if it fixes direct-new-game crash, replace it later with an
   owned copy texture instead of retaining the Bink-owned resource.

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

## Active Plan: Automatic Bink RGB Source Selection

The manual tests proved different BK2 resolutions can work, but fixed `bink_plane_source_width/height/index` makes users retune the ini for each file. Add an auto mode so multiple resolutions can be tested with the same config.

1. Add a default-on config key for bridge mode, tentatively `bink_plane_auto_source=true`.
2. Keep the existing manual keys as fallback/debug controls:
   - `bink_plane_source_width`
   - `bink_plane_source_height`
   - `bink_plane_source_format`
   - `bink_plane_source_index`
3. In auto mode, only consider source candidates after the visible title target descriptor has been stored. This skips early non-title/non-movie resources that caused 4K `source_index=1` to be wrong.
4. In auto mode, prefer `DXGI_FORMAT(28)` Texture2D resources with:
   - 16:9-ish dimensions;
   - practical video size, at least 640x360;
   - one mip level;
   - not the tiny 64x36 title target.
5. Store/replace the current Bink source when a better candidate appears:
   - prefer larger area;
   - if same area, allow later resources to replace earlier ones.
6. Apply the latest selected source to the stored title descriptor each time it changes.
7. Add concise logs that distinguish manual and auto source selection.
8. Update example/deploy ini to use auto mode and remove per-resolution source width/height/index from normal config.
9. Build with `cargo build --release --offline`, deploy the DLL to `F:\GoldenAge\dll\dynamic_title`, and clear the log for user tests.

### Follow-up Adjustment: Freeze First Auto Source

The first multi-resolution test with a `1920x1080` BK2 showed auto mode selected the likely correct `1920x1080 DXGI_FORMAT(28)` source, then replaced it with a later unrelated `3840x2160 DXGI_FORMAT(28)` resource, producing a black background.

1. Change auto mode to store only the first valid post-title `DXGI_FORMAT(28)` 16:9 source.
2. Do not replace an existing auto source with a larger later resource.
3. Keep manual mode unchanged for targeted debugging.
4. Build, deploy, clear the log, and retest the same `1920x1080 8fps` BK2.

## Active Plan: BK2 to MP4 Convenience Tool

Build a small helper tool for quickly previewing/exporting BK2 files as MP4.

1. Add a `_Tools\bk2-to-mp4` folder.
2. Implement a PowerShell script with a simple CLI:
   - accept one or more `.bk2` files;
   - default output path is beside each source file with `.mp4` extension;
   - optional `-OutputDir` writes all outputs to a chosen folder;
   - optional `-Crf` and `-Preset` tune H.264 quality/speed.
3. Add a `.bat` launcher so the tool can be used by dragging `.bk2` files onto it.
4. Auto-detect:
   - `G:\ffmpeg\ffmpeg-8.0-essentials_build\bin\ffmpeg.exe`
   - `ffmpeg.exe` from `PATH` as fallback.
5. Add a README with practical usage.
6. Update `TASK_STATUS.md` after implementation.

## Active Plan: Prevent MovieImp From Throttling Game FPS

User found a serious regression: after the DLL starts BK2 playback, the normally
60 fps game becomes 30 fps and remains that way after entering gameplay. The
likely cause is ER's native `CSMovieImp/CSMovieIns` render/update state being
driven from the main thread at the BK2 timing cadence.

1. Keep the existing working `MENU_DummyMovie` SRV bridge intact.
2. Do not re-enable global D3D12 draw hooks.
3. Track the ER `CSMovieIns` pointer when `movie_imp_trigger` starts the title
   BK2, even when logging/probes are disabled.
4. Reuse the existing narrow `main.exe+0xE215C0` MovieIns render hook, but add a
   functional throttle mode separate from verbose render probing.
5. In throttle mode, only call the original MovieIns render/update for the
   tracked title movie after a configurable minimum interval. Skip intermediate
   calls so the game/main menu can continue at 60 fps while the BK2 updates at
   its own cadence.
6. Add config keys:
   - `movie_imp_render_throttle`, default `true`;
   - `movie_imp_render_interval_ms`, default `33`.
7. Keep logging disabled by default; emit throttle diagnostics only if
   `log_enabled=true`.
8. Build, deploy to `F:\GoldenAge\dll\dynamic_title`, and update
   `TASK_STATUS.md`.

### Follow-up Adjustment: Stop On Gameplay Signals, Not Title Task Arm

The first stop-monitor test showed `title_flow=false` and `title_step=false`
for the whole current bridge path, so the old SYSTEX title gate never armed and
never closed the movie.

1. Keep full-rate main-menu playback; do not add render throttling.
2. Treat successful `movie_imp_trigger` as the arm point.
3. Add a short configurable grace period after MovieImp setup so initial
   title-loading state cannot close the movie immediately.
4. After the grace period, close the MovieIns when any gameplay/transition signal
   appears:
   - loading screen active;
   - HUD state is default;
   - in-game flow task group active.
5. Keep common/menu flow out of the stop condition for now, so title-side option
   menus do not unnecessarily kill the background video.
6. Build, deploy, and retest with `log_enabled=true` until a
   `movie imp stop monitor: closed` line appears during game entry.

### Follow-up Adjustment: Test MovieImp Sync Option

The next log showed the monitor closed after the 2s grace while the title was
still loading, which froze the video but restored 60 fps. The setup log also
showed ER fields `present[+F4]=1 option[+F8]=1`; this option may be the native
movie pacing/sync behavior that caps the main loop to the BK2 cadence.

1. Keep title video alive until a stable title state has been observed:
   `loading=false`, `hud_default=false`, and `ingame_flow=false`.
2. Only after that stable title state, stop on later `loading`, `hud_default`,
   or `ingame_flow`.
3. Add configurable ER MovieIns setup flags:
   - `movie_imp_setup_flag`, default `1`;
   - `movie_imp_present`, default `1`;
   - `movie_imp_unknown`, default `0`;
   - `movie_imp_option`, default `0` for the next test.
4. Deploy with `movie_imp_option=0` and logging enabled.
5. Interpret:
   - moving video + 60 fps: keep option 0;
   - moving video + 30 fps: MovieIns render itself is pacing the frame loop;
   - frozen/blank video: option 1 is required and the next path is a narrower
     BinkTexture update hook or direct update call without MovieIns draw/submit.

### Follow-up Adjustment: Accept Native Title 30fps, Stop Before Gameplay

User confirmed both Elden Ring and Nightreign lock to 30 fps while playing BK2
through the native movie path. Stop treating title-menu 30 fps as a bug for the
current release.

1. Restore `movie_imp_option=1`, matching the moving native movie path.
2. Keep the stable-title arming logic:
   - wait for `loading=false`, `hud_default=false`, `ingame_flow=false`;
   - only then close on later loading/gameplay/HUD signals.
3. Validate the release behavior:
   - title/main menu video moves, even if the menu is 30 fps;
   - starting/loading into gameplay logs `movie imp stop monitor: closed`;
   - gameplay frame rate returns to normal after the movie path is closed.

### Follow-up Adjustment: Use WorldChrMan Player Signal

The latest ER test showed the stop monitor armed after the title became stable,
but no later `loading`, `hud_default`, or `ingame_flow` transition was observed,
so MovieIns never closed and gameplay stayed at 30 fps.

1. Keep the native moving movie path (`movie_imp_option=1`).
2. Extend the stop snapshot with a direct in-world signal:
   `WorldChrMan::instance().main_player.is_some()`.
3. Keep this signal only as a stop condition after the stable title state has
   been observed, so the movie is not closed during initial title loading.
4. Build and deploy, then validate that entering gameplay produces
   `world_player=true` followed by `movie imp stop monitor: closed`.

### Follow-up Adjustment: Let World Player Bypass Title Arming

The next ER log showed `world_player=true`, but it happened before the monitor
had armed on stable title state. The monitor therefore missed the gameplay
transition and MovieIns stayed alive.

1. Keep stable-title arming for ambiguous signals such as `loading` and
   `hud_default`.
2. Treat `world_player=true` as a strong leave-title signal after the initial
   grace period, even if stable title state has not yet been observed.
3. Close MovieIns immediately on this strong signal.
4. Keep logging the close reason distinctly so the next validation can tell
   whether the bypass path fired.

## Active Plan: Re-arm Title BK2 After Returning From Gameplay

The stop-on-gameplay fix now restores gameplay FPS by closing the native MovieIns. The remaining uncommon case is returning from gameplay to the title menu: the previous one-shot trigger/callback state leaves the title background static.

1. Keep the current gameplay stop behavior: close MovieIns when `world_player=true` or other leave-title signals appear.
2. After a successful close, reset the MovieImp trigger and stop-monitor guards so the title movie can be started again later in the same process.
3. Let the title descriptor callback be re-fired when the title target is observed again after the trigger guard has been reset.
4. Avoid starting MovieImp while gameplay/world state is active; rely on the same post-title-target delay and stop monitor for each replay cycle.
5. Keep logging default behavior unchanged and add only concise debug lines when logging is enabled.
6. Build, deploy to `F:\GoldenAge\dll\dynamic_title`, clear the log, and update `TASK_STATUS.md`.

## Active Plan: Retry MovieImp Re-arm and Preserve Title Descriptor

The latest return-to-title test showed the re-arm monitor fired, but too early: `CSMovieImp` global was still null, so the second setup failed. It also suggests the `MENU_DummyMovie` descriptor is not recreated on return, so clearing the stored title descriptor prevents a later Bink source from being bridged.

1. Keep the `fs-title-skip-master` finding as reference only: it hooks a generic engine-flag getter and is not a direct title-ready detector for this bridge.
2. Change MovieImp trigger execution so setup returns success/failure instead of silently returning.
3. If MovieImp global or MovieIns is not ready after returning to title, retry for a bounded period instead of consuming the one-shot trigger state permanently.
4. Preserve the stored title descriptor across gameplay close; clear only the Bink source and callback-fired state so the next movie RGB source can be bridged into the existing descriptor.
5. Keep the world-player guard so retries never start movie playback while gameplay is active.
6. Build, deploy to `F:\GoldenAge\dll\dynamic_title`, clear the log, and update `TASK_STATUS.md`.

## Active Plan: Restore Static Dummy When Return Re-arm Cannot Play

The latest test showed return-to-title remains unable to create MovieImp: `CSMovieImp` global stays null. Preserving the old title descriptor caused a worse failure because auto-source captured a non-movie `DXGI_FORMAT(28)` resource and bridged it into the title slot, producing black.

1. Keep retrying MovieImp on return, but do not allow Bink RGB source capture until MovieImp setup succeeds.
2. When storing the title descriptor, also store the original `MENU_DummyMovie` resource/SRV desc/device so the static dummy can be restored later.
3. On MovieIns close, disable Bink source capture, clear the old source, and restore the original dummy descriptor if available.
4. Only after successful MovieImp setup re-enable Bink source capture, so post-return random `DXGI_FORMAT(28)` resources cannot black out the title.
5. Build, deploy, clear log, and update `TASK_STATUS.md`.

## Active Plan: Engine Flag Driven Return Re-arm

Use the `fs-title-skip-master` engine-flag getter as a read-only signal for title-flow activity. Do not change the flag return value.

1. Add a configurable `movie_imp_rearm_on_engine_flag` switch, enabled in the deployed ER config for this test.
2. Find the engine flag getter in `eldenring.exe` using the same AOB shape as `fs-title-skip-master`:
   `48 0F BE 01 48 8D 0D ?? ?? ?? ?? 48 FF 24 C1`.
3. Hook the getter with `ilhook`; call the original normally and never force false.
4. When the queried flag byte is `1..6`, log it sparingly and request MovieImp re-arm through the existing retry path if no trigger is already running and `world_player` is false.
5. Keep Bink source capture gated behind successful MovieImp setup so a false/early flag cannot black out the title.
6. Build, deploy, clear log, and update `TASK_STATUS.md`.

## Active Plan: Freeze Last Video Frame On Return

The desired stable behavior is not original dummy fallback. It should keep the last bridged BK2/video frame as a static image after MovieIns is stopped for gameplay FPS restoration.

1. Keep the current gameplay stop behavior and keep engine-flag re-arm disabled in deployment.
2. On MovieIns close, disable further Bink source capture so random post-return `DXGI_FORMAT(28)` resources cannot overwrite the title descriptor.
3. Do not clear the stored Bink RGB source and do not restore the original `MENU_DummyMovie` dummy descriptor.
4. Leave the title descriptor bound to the last successful Bink RGB source so the title shows a static video frame after returning from gameplay.
5. Build, deploy to `F:\GoldenAge\dll\dynamic_title`, clear log, and update `TASK_STATUS.md`.

## Active Plan: Non-destructive MovieImp Detach

The current gameplay FPS restore path is too destructive for shared MovieIns state: it calls the inner BinkTexture close method and clears fields inside `CSMovieIns`. Other in-game BK2 playback likely reuses the same `CSMovieIns`, so skipping or finishing later movies can crash when the engine sees the mutated state.

1. Keep the title `MENU_DummyMovie` SRV bridge and freeze-last-video-frame behavior.
2. Replace the destructive close path with a non-destructive detach path:
   - do not call the inner BinkTexture close vtable;
   - do not clear `MovieIns+0xB8`, `+0x130`, `+0x40`, or `+0x44`;
   - only clear `CSMovieImp+0x40` if it still points to the tracked title `MovieIns`.
3. Disable the return re-arm monitor for the stable baseline, since true replay after returning to title is unresolved and repeated MovieImp setup can interfere with later movie playback.
4. Keep `reset_bink_bridge_cycle()` so title descriptors retain the last Bink/video frame and source capture is disabled after leaving title.
5. Build and deploy to `F:\GoldenAge\dll\dynamic_title`, then test:
   - first title still plays;
   - gameplay FPS returns;
   - in-game BK2 can be skipped or can end without freeze/crash.

### Follow-up Adjustment: Stop Title Movie Without Re-arming

The non-destructive detach test was too light: the native title movie kept ticking, so gameplay stayed locked to 30 fps, returning to title replayed the BK2, and later in-game BK2 playback reused/overlapped the title movie.

1. Keep avoiding the dangerous state-machine writes to `CSMovieIns+0x40/+0x44`.
2. On gameplay entry, stop only the title movie resources:
   - call the inner BinkTexture close vtable;
   - clear the title `CSMovieIns` BinkTexture pointer;
   - clear the title active/open flag at `+0x130`;
   - clear `CSMovieImp+0x40` if it still points to the title MovieIns.
3. Do not reset `MOVIE_IMP_TRIGGER_STARTED` after the stop, so the title target callback cannot restart native BK2 when returning to the title menu in the same process.
4. Add a bridge reset variant that freezes the last video frame and keeps the title callback marked fired.
5. Build, deploy, and retest gameplay FPS plus in-game BK2 skip/end behavior.

### Follow-up Adjustment: Soft-stop Title Movie To Avoid New-game BK2 Race

The stop-with-close build can still crash probabilistically when starting a new game directly from the first title menu while the title BK2 is still playing. If the user first enters gameplay once, returns to title, and the title BK2 has already stopped, starting a new game is stable. This points to a race between the title BinkTexture close vtable call and ER's own new-game BK2/movie initialization.

1. Keep the no-rearm behavior so returning to title cannot restart native title BK2 in the same process.
2. Keep preserving `CSMovieIns+0x40/+0x44` state-machine fields.
3. Remove the direct title BinkTexture close vtable call from the gameplay stop path.
4. Soft-stop the title movie by clearing only:
   - title `CSMovieIns` BinkTexture pointer;
   - title active/open flag at `+0x130`;
   - `CSMovieImp+0x40` when it points to the title MovieIns.
5. Keep freezing the last bridged title video frame.
6. Build/deploy and test direct first-menu new-game start several times.

### Follow-up Adjustment: Auto Soft-stop On Stable Title

Soft-stopping at the gameplay/loading transition is still too late: direct first-menu new-game start can crash while the title movie is still active. Move the stop earlier, into the stable title-menu window.

1. Keep the soft-stop behavior: no BinkTexture close vtable call, no `+0x40/+0x44` state-machine writes.
2. After MovieImp setup and the configured grace period, if the title gate is stable/ready, soft-stop the title movie immediately.
3. If the title gate becomes stable later, soft-stop then instead of merely arming for a later loading/gameplay transition.
4. Keep `world_player=true` as a fallback strong stop signal.
5. Preserve no-rearm/static-last-video-frame behavior.
6. Deploy with the existing `movie_imp_stop_grace_ms` first, then tune the grace lower only if direct-start can beat the auto-stop.

### Follow-up Adjustment: Handoff At Native Movie Init

Auto-stopping during the stable title menu would freeze the dynamic main menu before the user enters gameplay, which conflicts with the goal. Instead, keep the title movie moving until ER actually starts another movie.

1. Revert stable-title auto-stop behavior.
2. Stop using loading/hud-default transitions as title stop signals because they are too close to new-game BK2 initialization and can race.
3. Always install the narrow `CSMovieIns` init hook when title MovieImp playback is enabled.
4. In `CSMovieIns` init, detect when the tracked title MovieIns is about to initialize a non-title movie path.
5. At that exact native movie-init boundary, soft-stop the title state without calling BinkTexture close:
   - clear title Bink pointer;
   - clear active/open flag;
   - disable Bink source capture and keep the last title frame;
   - do not clear `CSMovieImp+0x40` during handoff because ER is about to use the same MovieIns for the new movie.
6. Keep `world_player=true` as a fallback stop for paths that enter gameplay without a new BK2.
7. Build/deploy and test direct first-menu new-game start while the title BK2 is still moving.

### Follow-up Adjustment: Stop On User Confirm Input

The crash log showed no native movie init handoff before the direct-start crash, and the only recorded stop was the late `world_player=true` fallback. Add an earlier user-confirm signal.

1. Keep the dynamic title movie running while the user only watches/navigates the main menu.
2. Add a small title stop monitor that polls keyboard/gamepad confirm inputs after title movie setup succeeds.
3. On confirm input, soft-stop the title movie immediately:
   - no BinkTexture close;
   - no `+0x40/+0x44` writes;
   - clear title Bink pointer/active flag and matching `CSMovieImp+0x40`.
4. Leave the native movie init handoff and world-player fallback in place.
5. Add the required Windows keyboard input feature to `Cargo.toml`.
6. Build/deploy and test direct first-menu new-game start while the title BK2 is still moving.

### Follow-up Adjustment: Reset Movie State On Confirm Stop

The confirm-input build stops the visible title movie on "press any button", but direct new-game still crashes. This suggests soft-stop leaves `CSMovieIns` in an inconsistent half-stopped state: Bink pointer and active flag are cleared while state fields remain in a movie-running state.

1. Keep broad confirm input as a diagnostic trigger for now.
2. For confirm-triggered stop only, clear `CSMovieIns+0x40/+0x44` along with Bink pointer and active flag.
3. Keep world-player fallback as the lighter preserve-state stop, because clearing state later in gameplay previously caused in-game BK2 skip/end instability.
4. Do not call BinkTexture close.
5. Build/deploy and test direct new-game start again.

### Follow-up Adjustment: Early Confirm Full Close

The confirm reset-state build allows the new-game CG to play, but skipping the CG often crashes. This suggests the old title BinkTexture may still need a real close, but only at an earlier safe point before new-game movie initialization begins.

1. Keep confirm-input as the early diagnostic trigger.
2. On confirm-input stop only, call the title BinkTexture close vtable before clearing the pointer.
3. Also reset `CSMovieIns+0x40/+0x44` on this early confirm path.
4. Keep the world-player fallback as the lighter no-close/no-state-reset stop.
5. Build/deploy and test:
   - direct new-game CG starts;
   - skipping CG does not crash;
   - later in-game BK2 skip/end remains stable.

### Follow-up Diagnostic: Log MovieIns State Machine Fields

Early confirm full-close still crashes when skipping the new-game CG. The CG opens successfully, so the problem is likely the MovieIns state machine after the title handoff.

1. Add `CSMovieIns+0x40/+0x44/+0x48` to `log_movie_ins()`.
2. Keep current behavior unchanged for one diagnostic build.
3. Use the next log to compare:
   - title movie after init;
   - confirm stop after reset;
   - new-game CG before/after init;
   - final state before skip crash, if available.
4. If `+0x40/+0x44` remain at an invalid value after new CG init, test restoring the state to the value ER expects instead of blindly clearing to `0/0`.

### Follow-up Adjustment: Preserve MovieImp Current On Confirm Stop

The latest diagnostic log shows the new-game CG (`movie:/10010010.bk2`) initializes normally after the confirm stop, including a sane state transition and `CSMovieImp+0x40` pointing back to the shared `CSMovieIns`. The remaining crash happens after skipping the CG, so the next low-risk variable is the earlier temporary detach of `CSMovieImp+0x40`.

1. Keep the confirm-input stop timing and current reset/inner-close behavior for one test.
2. Add an explicit `detach_imp_current` flag to the title stop helper.
3. For confirm-input stop, preserve `CSMovieImp+0x40` instead of clearing it.
4. For `world_player` fallback stop, keep clearing `CSMovieImp+0x40` when it still points at the tracked title MovieIns.
5. Log whether detaching was requested and whether it actually happened.
6. Build, deploy, clear the ER log, then test direct new game and CG skip again.

### Follow-up Diagnostic: Native CG Skip/End Lifecycle Control

The preserve-current test still crashes after skipping the new-game CG. The next diagnostic should compare against ER's native movie lifecycle without the title MovieImp trigger active.

1. Disable `movie_imp_trigger` in the deployed ER config so the DLL does not start the title BK2.
2. Keep only read-only movie probes enabled:
   - `probe_movie_ins=true`;
   - `probe_movie_step=true`;
   - `probe_movie_tick=true`;
   - `probe_bink_texture_open=true` if needed for Bink object details.
3. Add a read-only dynamic BinkTexture close hook:
   - after `CSMovieIns` init returns a nonzero BinkTexture pointer, read its vtable slot `+0x10`;
   - install one hook on that close function;
   - log caller, object, vtable, Bink handle fields, and whether it matches the last observed movie BinkTexture.
4. Do not mutate MovieIns, MovieImp, BinkTexture, title descriptors, or D3D resources in this diagnostic mode.
5. Deploy the diagnostic DLL and config, clear log, then ask for one normal ER run:
   - start new game;
   - let the native CG begin;
   - skip it;
   - send the log.
6. Compare native close/skip order against the title-trigger crash path to identify the official stop/end boundary.

### Follow-up Adjustment: Request Native MovieIns Finish On Confirm

The native CG lifecycle control log shows ER does not only call the BinkTexture
close vtable when a movie is skipped or ends. The normal cleanup path also calls
the BinkTexture pre-close slot, releases the BinkTexture object, clears
`MovieIns+0xB8`, updates callback fields, marks `+0x131`, advances state, and
then lets state1 clear `+0x130`. The previous title-confirm stop skipped much of
that lifecycle and can poison the next new-game CG cleanup.

1. Keep the broad confirm-input trigger as the current diagnostic early signal.
2. Replace the confirm-input manual stop path with a native-finish request:
   - do not call the BinkTexture close vtable from the DLL;
   - do not clear `MovieIns+0xB8`;
   - do not clear `MovieIns+0x130`;
   - do not reset `MovieIns+0x40/+0x44`;
   - write `MovieIns+0x133 = 1` so ER's current movie state can enter its own
     full cleanup on the next native tick.
3. Freeze the bridged title frame immediately so the title descriptor keeps the
   last video frame while the native cleanup completes.
4. Add a short monitor that waits for native cleanup to clear `+0xB8` or
   `+0x130`, then releases the tracked title parent and stop-monitor guard.
5. Keep the world-player fallback path unchanged for now.
6. Deploy with dynamic-title playback enabled and verbose movie lifecycle logs
   still on for one validation run.

### Follow-up Adjustment: Force MovieIns Cleanup State On Native Finish Request

The first native-finish request build did capture the confirm input and wrote
`MovieIns+0x133 = 1`, but the MovieIns stayed in `state[40/44]=6/6` and the
video kept playing until the monitor timed out. The expanded state table showed
that ER's full cleanup function `main.exe+0xE21090` is state slot 7, while state
6 is the render/playback state `main.exe+0xE215C0`.

1. Keep the native-finish request path, but make it set the state machine to the
   cleanup state:
   - write `MovieIns+0x40 = 7`;
   - write `MovieIns+0x44 = 7`;
   - write `MovieIns+0x133 = 1`.
2. Do not call the cleanup function directly from the DLL thread; let the next
   native MovieIns tick call state7 in the normal engine context.
3. Keep freezing the bridged title frame immediately.
4. Add a leave-title fallback in the stop monitor: if HUD default or world
   player appears while the title movie is still tracked, request native finish
   instead of the old manual soft-stop path.
5. Build/deploy and test direct new-game start plus CG skip again.

### Follow-up Adjustment: Handoff Finish On New Movie Init Instead Of Confirm Input

The confirm-input native finish path is technically using ER's cleanup state, but
the user experience is wrong because it freezes the title BK2 as soon as the
player presses a button on the title/menu. The desired behavior is to keep the
title BK2 moving until ER actually starts a new movie path such as new-game CG.

1. Disable the broad confirm-input monitor for normal dynamic-title operation.
2. Re-enable a narrow `CSMovieIns` init hook as a functional handoff hook when
   `movie_imp_trigger=true`, not as a verbose diagnostic probe.
3. In that hook, when the tracked title `MovieIns` is about to initialize a
   non-title path, request the same state7 native finish used by the confirm test.
4. Replace the old handoff soft-stop behavior with native-finish request:
   - do not clear `+B8` manually;
   - do not clear `+130` manually;
   - write state `+40/+44 = 7` and `+133 = 1`;
   - freeze the visible bridge frame immediately.
5. Let the original ER movie init continue after this request for the first test.
   If it still races, add a bounded wait/yield at the handoff boundary in a later
   build.
6. Keep `world_player`/HUD fallback as a late backup for paths that enter gameplay
   without starting another movie.

### Follow-up Adjustment: Earlier Setup Handoff

The no-confirm-pause test crashed at new-game start and did not log
`native movie init handoff`. The `CSMovieIns` init boundary is therefore too
late or not reached before the crash. The log also showed the stop monitor using
`hud_default=true` as an early leave-title signal, which stops the title movie
before actual game start and recreates the old pre-stop crash setup.

1. Remove `hud_default` as a stop condition; keep only `world_player=true` as a
   late gameplay fallback.
2. Install an ER-only hook at `CSMovieIns` setup `main.exe+0xE20F90` when
   `movie_imp_trigger=true`.
3. In the setup hook, decode the incoming UTF-16 path pointer from `r8`.
4. If the tracked title `MovieIns` is active and the incoming path is not the
   title path `movie:/00001010.bk2`, request the same state7 native finish before
   calling original setup.
5. Wait briefly for the native finish to clear `+B8` or `+130`; if the same
   thread cannot progress cleanup, the log should show a timeout and the next
   step will be a direct cleanup call or exact menu action hook.
6. Keep verbose movie probes disabled for this test.

### Follow-up Adjustment: Exact World-player Cleanup Timing Probe

The setup handoff build did not log a non-title setup call before the crash.
The log showed `world_player=true`, then state7 cleanup completed immediately,
then the process crashed/stopped. This means the direct-start path being tested
may enter world state without a new CG setup first, and the late world-player
cleanup itself may be unsafe during the first frames of world creation.

1. Temporarily remove `world_player=true` as an immediate cleanup condition.
2. Keep setup handoff hook installed for any path that does start a native movie.
3. Add a delayed world-player cleanup monitor:
   - require `world_player=true` continuously for a short grace period;
   - then request state7 finish after world creation has settled.
4. Keep title BK2 active during the short grace window, accepting a brief 30 fps
   lock after entering world for this test.
5. If delayed cleanup fixes direct-start crash and restores FPS shortly after,
   tune the grace period down.
