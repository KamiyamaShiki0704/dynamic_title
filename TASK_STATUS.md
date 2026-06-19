# TASK_STATUS

## Current Goal

还原 Nightreign 标题/主菜单 BK2 动态背景调用逻辑，并判断如何移植到 Elden Ring：底层播放 BK2，保留上层标题/logo/menu UI。

## Completed

- 已创建并采用本文件作为当前任务唯一状态源；已创建 `PLANS.md` 记录阶段计划。
- 确认 ER 当前 `05_001_title_logo.gfx` 的蓝底 `MENU_DummyMovie` 层能进入主菜单 UI 层，说明 GFX 图层路径基本正确。
- 确认全局 D3D12 draw hook 会导致黑屏/无 UI，不再作为定位路线。
- 确认 NR 主菜单自然打开 `movie:/00001010.bk2`，BinkTexture open wrapper 为 `nightreign.exe+0x21152A0`。
- 确认 NR BinkTexture open 的调用返回点在 `main.exe+0xF6A264`，属于 `CSMovieIns` 内部通过 vtable slot `+0x18` 调用 BinkTexture open。
- 对比 NR/ER `CSMovieIns`：
  - NR 标题路径使用 `+0xC0` BinkTexture、`+0xC8` path、`+0xF8/+0xFC/+0x100` options。
  - ER 通用 MovieIns 使用 `+0xB8` BinkTexture、`+0xC0` path、`+0xF0/+0xF4/+0xF8` options。
- 修正判断：ER 也有 `01_920_Movie` / `05_001_Title_Logo` 宽字符串；真正缺口是 NR 的 ASCII `MovieWait` / `Main/Movie` / `MENU_DummyMovie` 绑定块。
- 发现 NR `StaffRollScreen@CS` vtable 有 18 个 slot，ER 对应类只有 13 个 slot；NR 额外 `slot17 = main.exe+0x78FEE0`，会调用 `0x78FA80` 并遍历 `+0xD50` lambda 回调表。
- 新增默认关闭的高层 probe：`probe_staffroll_screen`。
  - NR 模式 hook：`0x78FEE0`、`0x78FA80`、`0x9764A0`、`0x9764E0`、`0x9765F0`。
  - ER 模式 hook：`0x746E80`。
  - 仅记录高层 StaffRollScreen/SceneObjProxy 字段，不 hook D3D draw，不强行播放 Bink。
- 已编译通过 `cargo build --release`。
- ER 部署 DLL 已更新，ER ini 保持所有危险 probe 关闭。
- NR `_mod` DLL 已更新，NR `_mod` ini 已调整为只开 `probe_staffroll_screen=true`。
- 已分析 NR 最新测试 log：
  - `probe_staffroll_screen` 配置被读取。
  - NR 五个 hook 均安装成功：
    - `0x78FEE0` StaffRollScreen slot17/movie init
    - `0x78FA80` movie setup
    - `0x9764A0` OneShot lambda
    - `0x9764E0` SceneObj lambda A
    - `0x9765F0` SceneObj lambda B
  - log 中没有任何上述 hook 的 call 记录。
- 静态反查 `MovieWait/Main/Movie/MENU_DummyMovie` 周边 RTTI/vtable：
  - `MovieWait` 的直接代码引用位于 NR `StaffRollScreen` vtable slot2：`main.exe+0x975A70`，不是此前 hook 的 slot17。
  - NR slot2 会检查 `StaffRollScreen+0xE00` 是否包含 `OneShot` 与 `MovieWait`，并访问/更新 `+0xEBC/+0xECC` 相关字段。
  - ER 对应 `StaffRollScreen` slot2 `main.exe+0x8BE060` 只引用/检查 `OneShot`，没有 `MovieWait`。
  - NR slot2 后续会构造多个 lambda/vtable 对象，其中包括靠近 `MENU_DummyMovie` 块的 lambda。
- 已执行下一轮窄 probe 改动：
  - `probe_staffroll_screen` 安装延迟从 2 秒缩短到 100ms。
  - 新增 NR `StaffRollScreen` slot2 hook：`main.exe+0x975A70`。
  - StaffRollScreen 字段日志新增 `+0xEA8`。
  - 保留原有 NR slot17/setup/lambda hook 作为辅助观察。
- 已重新编译通过 `cargo build --release`。
- 已部署新 DLL 到 NR `_mod`，并备份旧 DLL 为 `dynamic_title_bg.dll.before_slot2_probe_*`。
- 已确认 NR `_mod` ini 仍只开启 `probe_staffroll_screen=true`，Bink/Movie/D3D 相关 probe 均关闭。
- 重新读取 NR 最新 log（`LastWriteTime=2026-06-19 14:27:30`，长度 1150）：
  - `probe_staffroll_screen` 配置被读取。
  - `0x975A70`、`0x78FEE0`、`0x78FA80`、`0x9764A0`、`0x9764E0`、`0x9765F0` 均安装成功。
  - 当前最新文件没有任何 `status_slot call`、`slot call`、`setup call` 或 lambda call 记录。
- 已执行 broader StaffRollScreen probe 改动：
  - 新增配置 `probe_staffroll_broad`，默认 false。
  - `probe_staffroll_broad=true` 时 hook NR `StaffRollScreen@CS` vtable 全部 18 个 slot：
    - `0x7783B0`, `0x9757B0`, `0x975A70`, `0x778430`, `0x78C980`, `0x78D250`, `0x78DB50`, `0x78EBD0`, `0x78EBA0`, `0x78EAB0`, `0x778420`, `0x7783F0`, `0x78CAC0`, `0x78E1A0`, `0x778410`, `0x78EA60`, `0x78DEA0`, `0x78FEE0`。
  - broad 模式不重复安装 slot2/slot17 的窄 hook，但保留 `0x78FA80` setup 和三个 lambda 辅助 hook。
  - StaffRollScreen probe 现在尽可能立即安装，不再延迟 100ms。
  - 已重新编译通过 `cargo build --release`。
  - 已部署新 DLL 到 NR `_mod`，并备份旧 DLL 为 `dynamic_title_bg.dll.before_staffroll_broad_*`。
  - 已同步新 DLL 到 ER 部署目录；ER ini 仍全关。
  - NR `_mod` ini 当前只开启 `probe_staffroll_screen=true` 与 `probe_staffroll_broad=true`，Bink/Movie/D3D 相关 probe 均关闭。
- 已读取 broad 版 NR 崩溃 log：
  - broad 模式 18 个 StaffRollScreen slot hook 全部显示安装成功。
  - log 没有任何 `broad_slotXX call`、`setup call` 或 lambda call。
  - 游戏刚启动即崩溃，未能进入主界面。
  - 判断 broad hook 不安全，原因很可能是 18 个 slot 中包含大量极短 `ret`/`jmp`/thunk 函数，不适合用 `hook_closure_retn` 批量 patch。
- 已止血：NR `_mod` ini 中 `probe_staffroll_broad=false`，避免下次继续 broad 崩溃；窄 `probe_staffroll_screen=true` 暂保留。

## Changed Files

- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic-title-bg.ini`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\PLANS.md`

## Current Judgment

当前最可能的缺口不是 GFX 图层本身，也不是 Bink open wrapper 本身，而是 NR 在 `StaffRollScreen` 标题流程中额外注册了 `MovieWait/Main/Movie/MENU_DummyMovie` 相关 Scene/lambda 绑定。ER 保留了通用 Movie/CSMovieIns，但标题主菜单缺少这段高层绑定，因此简单替换 GFX 或直接调用 SYSTEX/MovieStart 会脱离主菜单 UI 层，导致黑屏或无 UI。

最新 NR log 说明当前选择的 StaffRollScreen slot/lambda hook 点“安装成功但未命中”。静态反查后，当前更偏向判断为“选错了 StaffRollScreen 子集”：真正需要先观察的是 NR `StaffRollScreen` slot2 `0x975A70`，因为它直接引用 `MovieWait`。

已按当前判断新增 slot2 probe 并提前安装。最新一次文件仍没有 call 记录，因此当前无法从运行时确认 `MovieWait` 状态检查。broader StaffRollScreen 全 slot probe 会导致 NR 启动崩溃，不能继续使用该运行时方案。

- 本次测试没有走到会触发 StaffRollScreen 更新/标题状态切换的位置。
- 100ms 安装仍可能晚于一次性初始化。
- 当前 hook 点虽相关，但实际 BK2 绑定可能发生在更早的 StaffRollScreen 构造/初始化 slot 或其它注册函数。

## Unresolved

- 最新 log 中新增 NR `StaffRollScreen` slot2 `0x975A70` hook 安装成功但未命中。
- broad StaffRollScreen vtable slot probe 已确认会导致 NR 启动崩溃，已关闭。
- 尚未确认 `0x78FEE0 -> 0x78FA80 -> lambda` 是否只是辅助扩展，还是某些条件下的后续路径。
- 尚未确认 `StaffRollScreen+0xD50` 回调表里哪一个 lambda 负责 `Main/Movie` 或 `MENU_DummyMovie`。
- 尚未确认 ER 侧是否能用现有 Scene API 重建一个窄绑定，还是需要直接接入 `CSMovieIns` / BinkTexture 到 GFX external image。

## Next Step

转向静态反编译早期构造/注册点：

1. 不再使用 broad StaffRollScreen vtable runtime hook。
2. 静态分析 NR `StaffRollScreen` 构造/析构/注册函数，重点围绕 `0x9757B0`、`0x975A70`、`MovieWait`、`Main/Movie`、`MENU_DummyMovie` 及相关 lambda vtable。
3. 如需新增运行时 probe，优先选择单个标准函数入口或数据注册函数，避免 hook 极短 thunk/jmp/ret 函数。

## 2026-06-19 14:57 CST - Narrow CSMovieIns Probe Stage

### Completed

- Re-read AGENTS.md, TASK_STATUS.md, PLANS.md after resume/compact.
- Confirmed NR ini stopgap: `probe_staffroll_broad=false`.
- Confirmed latest NR crash log was the previous broad StaffRollScreen hook run: all 18 hooks installed, no call logs, startup crash.
- Static analysis confirmed NR and ER both have `DLRuntimeClassImpl<CSStepLocal<CSMovieIns>>`, but ER lacks NR ASCII binding strings `MovieWait`, `Main/Movie`, and `MENU_DummyMovie`.
- GFX dump comparison confirmed NR `05_001_title_logo.gfx` contains `DefineExternalImage2 exportName=MENU_DummyMovie` and a display instance named `Movie`; ER current blue-layer test has the layer but not the NR exe-side binding logic.
- Static disassembly confirmed NR `CSMovieIns` movie-open helper at `main.exe+0xF6A0E0` uses `+0xC0` BinkTexture, `+0xC8` path, and `+0xF8/+0xFC/+0x100` options.
- Static disassembly confirmed ER corresponding helper remains `main.exe+0xE212E0` with `+0xB8` BinkTexture, `+0xC0` path, and `+0xF0/+0xF4/+0xF8` options.
- Wrote the narrow CSMovieIns probe plan to PLANS.md before code modification.
- Implemented module-aware `probe_movie_ins`: NR hooks `0xF6A0E0`, ER keeps `0xE212E0`.
- Added layout-aware CSMovieIns logging for NR/ER offsets and path ascii/utf16/hex previews.
- Built `dynamic-title-bg` release successfully; only existing warnings remained.
- Deployed new DLL to NR `_mod` and ER deploy directory; backed up previous DLLs and previous NR log.
- Changed NR `_mod` ini for next test: only `probe_movie_ins=true`; StaffRoll, Bink replace/open, SYSTEX/native trigger, and draw probes are off.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic-title-bg.ini`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`

### Current Judgment

Broad StaffRollScreen runtime probing is unsafe and should stay disabled. The next useful runtime signal is the stable CSMovieIns open helper, because it is the object that actually opens `movie:/00001010.bk2`. If NR reaches title/menu without crashing, the new log should capture the exact movie instance object and path/options before and after BinkTexture open.

### Unresolved

- Still unknown which NR high-level binding object connects `Main/Movie` to `MENU_DummyMovie`.
- Still unknown whether ER can recreate that binding through existing Scene/GFX APIs or needs a custom CSMovieIns/BinkTexture-to-GFX bridge.
- Need a fresh NR run with the new ini to see whether `main.exe+0xF6A0E0` is hit and whether the hook is stable.

### Next Step

Run NR once with current `_mod` config. Expected log lines should start with `movie ins probe: module="nightreign.exe" layout=NR hooking main.exe+0xF6A0E0`, followed by `movie ins probe: init call` and path output if the title movie opens. If it crashes before main menu, inspect the fresh log first before changing code.

## 2026-06-19 15:03 CST - CSMovieIns Hit and FD4 String Decode

### Completed

- Read fresh NR log from the narrow `probe_movie_ins` run.
- Confirmed the hook was stable and installed at `main.exe+0xF6A0E0` in `nightreign.exe`.
- Confirmed `CSMovieIns` init was called once from caller return `main.exe+0x81DAF3`.
- Confirmed return state created BinkTexture at `CSMovieIns+0xC0` (`0x7FF40695F530` in this run).
- Confirmed options at the NR offsets: volume `+0xF8 = 0.700`, present `+0xFC = 1`, option `+0x100 = 1`.
- Determined `CSMovieIns+0xC8` is not inline text; it is an FD4/Dantelion wide string object.
- Static disassembly of `main.exe+0x108710` showed the string layout used by NR:
  - object base is `CSMovieIns+0xC8`
  - data pointer at `+0x08` when capacity is external
  - length at `+0x18`
  - capacity at `+0x20`
- The fresh log showed length `0x13` and capacity `0x17`, matching the expected 19-character title movie path `movie:/00001010.bk2`.
- Added read-only FD4 wide-string decoding to `log_movie_ins`, printing `path[+.. fd4_wstr] data/len/cap/text/hex`.
- Built release successfully; only pre-existing warnings remain.
- Deployed new DLL to NR `_mod` and ER deploy directory; backed up the successful CSMovieIns-hit log and cleared current NR log for the next test.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`

### Current Judgment

The NR title BK2 open path is now confirmed through the normal `CSMovieIns` helper, and the earlier unreadable path data was a string-object decoding issue rather than a wrong offset. The next run should directly print `movie:/00001010.bk2` from `CSMovieIns+0xC8`. The caller `main.exe+0x81DAF3` suggests this is invoked through an owning work/step object at `[rbx+0x40]` vtable slot `+0x08`, so the next reverse-engineering target is the owner/caller object around `0x81DAE0` and its relation to title scene/GFX binding.

### Unresolved

- Need fresh NR run with FD4 string decode to confirm exact decoded text in log.
- Still unknown which owner object at caller `0x81DAF3` corresponds to the high-level title/movie binding.
- Still unknown where `Main/Movie` and `MENU_DummyMovie` are connected to the CSMovieIns/BinkTexture output.

### Next Step

Run NR once more with the current config. Expected new log should include `path[+C8 fd4_wstr] ... text="movie:/00001010.bk2"`. After that, statically follow the caller/owner path around `main.exe+0x81DAE0` and the object's `[+0x40]` virtual target to find the binding owner.

## 2026-06-19 15:31 CST - StaffRollScreen Constructor Diff and Narrow Probe

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, `PLANS.md`, and the latest NR `_mod` log after context resume/compact.
- Confirmed latest NR log decodes `CSMovieIns+0xC8` as FD4 wide string text `movie:/00001010.bk2`.
- Static reverse of NR confirmed `0x81DAF3` is the return site from `TmpWorkerThread` slot0 `0x81DAE0`; `[TmpWorkerThread+0x40]` is a `MemfunWork<CSMovieIns,H>` object that calls the member pointer `0xF6A0E0`.
- Static reverse of NR `0xF69780` confirmed the async movie setup path:
  - creates `MovieSetup` worker,
  - writes `MemfunWork<CSMovieIns,H>` at worker `+0x48`,
  - writes CSMovieIns pointer at `+0x08`,
  - writes member function pointer `0xF6A0E0` at `+0x10`.
- Confirmed NR exe does not contain hardcoded `movie:/00001010.bk2`; the BK2 path likely comes from resource/GFX/layout instance data.
- String/xref comparison:
  - NR has ASCII `MovieWait`, `Main/Movie`, `MENU_DummyMovie`.
  - ER current and ER 1.16 do not have ASCII `Main/Movie` or `MENU_DummyMovie`; they only retain generic `OneShot`, `Main`, and StaffRoll license strings.
- RTTI/vtable comparison:
  - NR `StaffRollScreen@CS` vtable at `0x2C549E8` has 18 visible slots and extra movie-related data/lambdas nearby.
  - ER `StaffRollScreen@CS` vtable at `0x2AE6808` has 13 visible slots before adjacent string data.
- Constructor/factory comparison:
  - NR factory `0x7B37F0` allocates `0xED0` bytes and calls `StaffRollScreen` constructor `0x974E50`.
  - ER factory `0x764BB0` allocates `0xA98` bytes and calls corresponding constructor `0x8BDD60`.
  - NR constructor binds `Main/Movie` to `this+0xE48` and later uses `MENU_DummyMovie` with the common `this+0x5A8` registration path.
  - ER object is smaller, so NR's movie/status fields beyond `0xA98` cannot be transplanted directly into ER's `StaffRollScreen`.
- Wrote the constructor-probe plan to `PLANS.md` before code modification.
- Added default-off `probe_staffroll_ctor`.
- Implemented module-aware constructor probe:
  - NR hooks `main.exe+0x974E50`.
  - ER hooks `main.exe+0x8BDD60`.
  - Logs constructor args and common/NR/ER field previews before and after return.
- Built release successfully; only pre-existing warnings remained.
- Deployed new DLL to NR `_mod` and ER deploy directory, backing up previous DLLs.
- Updated NR `_mod` ini for next test:
  - `probe_movie_ins=true`
  - `probe_staffroll_ctor=true`
  - `probe_staffroll_screen=false`
  - `probe_staffroll_broad=false`
  - dangerous Bink/SYSTEX/D3D replacement hooks remain off.
- Updated ER deploy ini with `probe_staffroll_ctor=false`, keeping ER in all-off stopgap state.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic-title-bg.ini`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`

### Current Judgment

The missing piece is now narrower than the full movie stack. NR's actual BK2 open uses generic `CSMovieIns`, but the title menu entry point adds `StaffRollScreen` movie binding data that ER does not have. ER cannot receive NR's fields by simple offset transplant because the ER object is `0xA98` bytes while NR's movie fields live around `0xDE8..0xECC`. A future ER restoration likely needs a sidecar binding owned by the DLL or by another existing ER object, while reusing ER's binding helper equivalents where possible:

- NR `0x792FA0` appears to correspond to ER `0x74A2F0` for binding scene/GFX paths.
- NR `0x78DF20` appears to correspond to ER `0x744490` for registering a string/path object into a common container such as `+0x5A8`.

The new constructor probe is read-only and intended to confirm exact construction timing/lifetime before any sidecar binding attempt.

### Unresolved

- Need fresh NR run with `probe_staffroll_ctor=true` to confirm the constructor hook is early enough and stable.
- Need determine whether `MENU_DummyMovie` registration through `+0x5A8` alone is sufficient, or whether `Main/Movie`/`MovieWait` state and `CSMovieIns` ownership must also be recreated.
- Need determine how to safely construct the string/path object expected by ER `0x744490` if used from DLL.
- Need determine whether a sidecar can own the missing NR `+0xDE8/+0xE48/+0xE00/+0xEA8/+0xEBC/+0xECC`-style data without changing ER object size.

### Next Step

Run NR once with current `_mod` config. Expected log should include:

- `staffroll ctor probe: module="...nightreign.exe" selected main.exe+0x974E50`
- `staffroll ctor probe: call #...`
- `staffroll ctor probe: after #...`
- the existing `movie ins probe ... text="movie:/00001010.bk2"`

If stable, compare constructor timing with the `CSMovieIns` open call. Then decide whether to add a next default-off ER-only sidecar binding experiment at `StaffRollScreen` constructor return.

## 2026-06-19 15:43 CST - CSMovieImp Global Owner Probe

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` after context compaction before continuing.
- Re-read latest NR `_mod` log:
  - `staffroll ctor probe` installed at `main.exe+0x974E50` and was stable.
  - No `staffroll ctor probe: call` appeared.
  - `movie ins probe` still hit `main.exe+0xF6A0E0`.
  - `CSMovieIns+0xC8` decoded to FD4 wide string text `movie:/00001010.bk2`.
- Interpreted latest log as: StaffRollScreen constructor probe was installed too late or this constructor was not the active observed path; CSMovieIns remains the stable runtime observation point.
- Wrote the `CSMovieImp Global Owner Probe` plan to `PLANS.md` before modifying code.
- Extended existing `probe_movie_ins` logging only; no new hook was added.
- Added module-aware read-only logging of the global `CSMovieImp@CS` singleton relation:
  - NR global pointer RVA: `main.exe+0x442E0A8`.
  - ER global pointer RVA: `main.exe+0x45878A8`.
  - Logs `CSMovieImp+0x38`, `+0x40`, `+0x48`, `+0x50`, `+0x54`.
  - Logs whether `CSMovieImp+0x38` matches the current `CSMovieIns` object that opened the BK2.
- Built `dynamic-title-bg` release successfully; only pre-existing warnings remained:
  - `log_draw_submit_arg_refs` unused.
  - `DecodedFrame.duration` unused.
- Deployed new DLL to NR `_mod` and ER deploy directory.
- Backed up previous NR/ER DLLs and the previous NR log; cleared the active NR log for the next run.
- Static disassembly of the NR global initialization block corrected the NR `CSMovieImp@CS` singleton RVA:
  - Previous hand-copied value `main.exe+0x443E0A8` was wrong.
  - Actual store target at `main.exe+0xF34BE9` is `main.exe+0x442E0A8`.
  - ER value remains `main.exe+0x45878A8`.
- Rebuilt and redeployed after the RVA correction.
- Confirmed NR ini remains narrow:
  - `probe_movie_ins=true`
  - `probe_staffroll_ctor=true`
  - `probe_staffroll_screen=false`
  - `probe_staffroll_broad=false`
  - Bink replacement, SYSTEX/native trigger, and D3D draw probes remain off.
- Confirmed ER ini remains safe all-off, including `probe_movie_ins=false` and `probe_staffroll_ctor=false`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`

### Current Judgment

The constructor hook not firing does not block progress because the title BK2 path is already observable at the stable `CSMovieIns` open helper. The next decisive question is ownership: if NR's title `CSMovieIns` equals `CSMovieImp+0x38`, ER may be able to reuse its existing global MovieImp/CSMovieIns stack and only needs the missing GFX/Scene binding restored. If it is a different object, NR likely has a title-specific owner path that must be reproduced or bridged.

The new probe is low risk because it only reads singleton fields when the already-working `probe_movie_ins` hook fires.

Static note: the correct NR `CSMovieImp@CS` singleton pointer is `main.exe+0x442E0A8`, not `0x443E0A8`.

### Unresolved

- Need fresh NR run with the new DLL to see `movie_imp` relation logs.
- Still unknown whether the title BK2 `CSMovieIns` is the global `CSMovieImp+0x38` object or a separate title-specific instance.
- Still unknown where the `Main/Movie` / `MENU_DummyMovie` binding becomes visible from ER's GFX/Scene side.
- Still unknown whether ER can restore the binding by calling existing helpers (`0x74A2F0`, `0x744490`) or needs a DLL-owned sidecar object.

### Next Step

Run NR once with current `_mod` config. Expected new log should include the existing `movie ins probe ... text="movie:/00001010.bk2"` lines plus:

- `movie ins probe: ... movie_imp layout=NR global[main.exe+0x442E0A8]=...`
- `relation=matches-current` or `relation=different`

After that, use the relation result to decide whether to analyze `CSMovieImp` methods/binding calls or search for a separate title-specific owner.

## 2026-06-19 16:08 CST - NR MovieImp Relation Confirmed

### Completed

- Re-read `TASK_STATUS.md`, `PLANS.md`, `AGENTS.md`, and the latest NR `_mod` log after context compaction/new log.
- Confirmed the new NR run is stable with only the narrow probes:
  - `probe_movie_ins` installed at `main.exe+0xF6A0E0`.
  - `probe_staffroll_ctor` installed at `main.exe+0x974E50` but still did not log a constructor call.
  - `movie:/00001010.bk2` was decoded again from `CSMovieIns+0xC8`.
- Confirmed the decisive MovieImp ownership result:
  - `global[main.exe+0x442E0A8]=0x7FF3CE20BF40`.
  - `CSMovieImp+0x38=0x7FF3CE7412C0`.
  - Current `CSMovieIns rcx=0x7FF3CE7412C0`.
  - `relation=matches-current`.
- Confirmed after open, NR writes the BinkTexture object to `CSMovieIns+0xC0`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

NR's title BK2 is not opened by a separate hidden title-only `CSMovieIns`; it is the global `CSMovieImp@CS` movie instance at `CSMovieImp+0x38`. This makes ER restoration more plausible through the existing ER `CSMovieImp/CSMovieIns` stack. The missing part is now likely the higher-level title/GFX binding and the code path that sets the global MovieIns path/options for `MENU_DummyMovie` / `Main/Movie`.

Because `CSMovieImp+0x38` matches, the next reverse target should be `CSMovieImp` methods and their callers, especially the method that writes the FD4 path/options and schedules the async open helper.

### Unresolved

- Still unknown which high-level title/scene/GFX function calls into `CSMovieImp` to set `movie:/00001010.bk2`.
- Still unknown whether ER already has an equivalent high-level method that can be called with a sidecar binding.
- Still unknown why the StaffRollScreen constructor hook installs but does not hit; it is now lower priority.

### Next Step

Statically trace NR `CSMovieImp` call chain:

1. Re-check callers of NR `0xF69780` / related `CSMovieImp` path setup methods.
2. Compare with ER equivalents around `0xE20F90` / `0xE212E0`.
3. Identify the first higher-level caller that knows about title/GFX binding strings or resource-derived movie path data.
4. Only after that, decide whether a new narrow runtime probe is needed.

## 2026-06-19 16:21 CST - ER MovieImp Direct Trigger Implemented

### Completed

- Statically traced the NR title BK2 path after the MovieImp relation result:
  - `StaffRollScreen` constructor binds `Main/Movie` to `this+0xE48` via `0x792FA0`.
  - The constructor registers `MENU_DummyMovie` into `this+0x5A8` via `0x78DF20`.
  - `OneShot`/movie lambda `0x9764A0` reads the global MovieImp and calls `0xF67AA0`.
  - `0xF67AA0` formats `movie:/%08d.bk2`, then calls the CSMovieIns setup path (`0xF694D0` thunk / real code around `0x71DCC0`) which later reaches `0xF6A0E0`.
  - ER equivalent CSMovieIns setup is `main.exe+0xE20F90`, and the normal ER open helper remains `main.exe+0xE212E0`.
- Wrote the `ER CSMovieImp Direct Setup Experiment` plan to `PLANS.md` before code changes.
- Added default-off config keys:
  - `movie_imp_trigger`
  - `movie_imp_path`
  - `movie_imp_delay_ms`
  - `movie_imp_volume`
- Implemented ER-only `movie_imp_trigger`:
  - waits for the configured delay,
  - reads ER `global[main.exe+0x45878A8]`,
  - reads `CSMovieImp+0x38`,
  - calls ER `main.exe+0xE20F90` with `movie:/00001010.bk2`,
  - logs `CSMovieIns` fields before/after using the existing ER layout logger.
- Built release successfully after `cargo fmt`; only the existing warnings remained:
  - `log_draw_submit_arg_refs` unused,
  - `DecodedFrame.duration` unused.
- Deployed the rebuilt DLL to:
  - `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
  - `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`
- Updated ER deploy ini for the next ER test:
  - `enable_dynamic_title=false`
  - `probe_movie_ins=true`
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `movie_imp_delay_ms=8000`
  - `movie_imp_volume=0.7`
  - SYSTEX/native/Bink replacement/D3D draw/title hijack remain off.
- Updated NR `_mod` ini only to explicitly keep `movie_imp_trigger=false`; existing NR narrow probes remain unchanged.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic-title-bg.ini`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`

### Current Judgment

The NR title movie path is now understood as: title StaffRollScreen private binding creates/registers the GFX movie target, while the actual BK2 playback uses the global CSMovieImp/CSMovieIns stack. ER lacks the title binding table (`Main/Movie`, `MENU_DummyMovie`, `MovieWait`) but it still has the global CSMovieImp/CSMovieIns stack and a compatible setup/open path.

The new ER experiment tests only the playback half through ER's own `CSMovieIns` setup path. If it creates BinkTexture without black/no UI, the remaining problem is specifically the GFX external image binding. If it still causes black/no UI or fails to open, then ER's global movie stack needs more state setup before it can be safely reused.

### Unresolved

- Need ER run output for the new `movie_imp_trigger`.
- Unknown whether calling ER `0xE20F90` directly is sufficient to schedule `0xE212E0`.
- Unknown whether the existing ER blue `MENU_DummyMovie` layer can consume the global CSMovieIns output without recreating NR's `Main/Movie` binding.
- The ER log file was absent/empty after deployment; the next ER run should create it.

### Next Step

Run ER once with the current `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`. Expected useful lines:

- `movie imp trigger: waiting 8s ...`
- `movie imp trigger: CSMovieImp=... CSMovieIns[+38]=...`
- `movie imp trigger: setup returned 0x01` if `0xE20F90` accepted the setup.
- `movie ins probe: init call ... main.exe+0xE212E0`
- after-open `bink_texture[+B8]` nonzero and `path[+C0 fd4_wstr] text="movie:/00001010.bk2"`

If ER shows blue UI plus no BK2, keep the playback logs and next focus on recreating `MENU_DummyMovie` / `Main/Movie` binding. If ER blackscreens or crashes, revert only `movie_imp_trigger=false` while keeping the code for further static analysis.

## 2026-06-19 16:30 CST - ER MovieImp Trigger Result

### Completed

- Re-read `TASK_STATUS.md` before analyzing the new ER log.
- Read ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed ER `movie_imp_trigger` ran without crashing during the logged period:
  - `probe_movie_ins` installed at ER `main.exe+0xE212E0`.
  - `movie_imp_trigger` found `CSMovieImp=0x7FF493C89600`.
  - `CSMovieImp+0x38=0x7FF49513D970`, and relation to current object is `matches-current`.
  - Before trigger, ER `CSMovieIns+0xC0` path was empty, volume `1.000`, present `0`, option `0`.
  - Calling ER `main.exe+0xE20F90` returned `0x01`.
  - After trigger, ER `CSMovieIns+0xC0` decoded as `movie:/00001010.bk2`, volume `0.700`, present `1`, option `1`, state `+0x130=1`.
- Confirmed no `movie ins probe: init call` appeared in this log, so ER did not reach `main.exe+0xE212E0` during the captured period.
- Updated ER deploy ini for the next run:
  - kept `probe_movie_ins=true`,
  - kept `movie_imp_trigger=true`,
  - enabled `probe_movie_step=true`,
  - enabled `probe_movie_tick=true`,
  - SYSTEX/native/D3D/title hijack remain off.
- Backed up and cleared the ER log so the next ER run will show only step/tick results.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` (backed up and cleared)

### Current Judgment

The ER global CSMovieImp/CSMovieIns setup function is usable: `0xE20F90` accepted `movie:/00001010.bk2` and wrote the expected path/options into the ER MovieIns object. The remaining failure in this run is not path construction; it is that the MovieIns state machine did not reach the open helper `0xE212E0`.

The most likely next split is:

- If `probe_movie_step` / `probe_movie_tick` logs calls after setup, then inspect why state `+0x130=1` does not transition to open.
- If there are no step/tick calls for the MovieIns object, then direct setup is missing the owner/stepper scheduling path that NR's `CSMovieImp+0x08` step object normally triggers.

### Unresolved

- Unknown whether the ER MovieIns stepper/tick runs after the direct trigger.
- Unknown whether the lack of `0xE212E0` is caused by missing scheduling, an install timing issue, or state/flag mismatch.
- Unknown whether successful BinkTexture creation will be visible through ER's existing `MENU_DummyMovie` GFX layer without restoring `Main/Movie` binding.

### Next Step

Run ER once with the current ER ini. Expected new useful lines:

- `movie step probe: hooking eldenring.exe+0xE20920`
- `movie tick probe: hooking eldenring.exe+0xE21B70`
- `movie imp trigger: setup returned 0x01`
- Either step/tick call logs involving the MovieIns/step object, or still no calls.
- Ideally later `movie ins probe: init call` at `main.exe+0xE212E0`.

Use that result to decide whether to call/trigger the ER stepper owner path directly or adjust the MovieIns setup flags.

## 2026-06-19 16:29 CST - ER Step/Tick Observed and Stepper Signal Added

### Completed

- Re-read `TASK_STATUS.md` and the latest ER log.
- Confirmed step/tick hooks installed:
  - `movie tick probe` at ER `main.exe+0xE21B70`
  - `movie step probe` at ER `main.exe+0xE20920`
- Confirmed `movie_imp_trigger` again wrote `movie:/00001010.bk2` into ER `CSMovieIns+0xC0` and returned `0x01`.
- Confirmed one ER MovieIns tick/step ran after trigger:
  - tick caller `main.exe+0x26D5565`
  - `rcx` matched the global `CSMovieImp+0x38` MovieIns object.
  - before step state was `state[40/44]=0/0`.
  - after step state was `state[40/44]=1/1`.
- Confirmed the open helper `main.exe+0xE212E0` still did not run in this log.
- Static comparison of ER wrapper around `main.exe+0xE1F400` identified missing post-setup work:
  - after `E20F90` returns success, ER writes `CSMovieImp+0x40 = CSMovieImp+0x38`;
  - then calls the step object at `CSMovieImp+0x08` through vtable slot `+0x20` with event id `0x12`.
- Wrote the follow-up plan to `PLANS.md`.
- Updated the ER `movie_imp_trigger` implementation to mimic that wrapper post-setup sequence:
  - write `imp+0x40 = movie_ins`,
  - read `(imp+0x08).vtable[+0x20]`,
  - call it with event `0x12`,
  - log the signal target and result.
- Ran `cargo fmt` and `cargo build --release`; build succeeded with only the existing warnings.
- Deployed rebuilt DLL to ER deploy directory and NR `_mod`, backing up previous DLLs.
- Backed up and cleared ER log for the next test.
- ER ini remains in narrow test state:
  - `probe_movie_ins=true`
  - `probe_movie_step=true`
  - `probe_movie_tick=true`
  - `movie_imp_trigger=true`
  - SYSTEX/native/D3D/title hijack remain off.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` (backed up and cleared)
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`

### Current Judgment

The ER MovieIns state machine is reachable, but direct `E20F90` setup alone is incomplete. The missing piece appears to be the CSMovieImp wrapper's post-setup activation/signal step, not path setup and not the first tick itself. The new build now mimics that post-setup signal.

### Unresolved

- Need next ER run to see whether `movie imp trigger: signaled CSMovieImp stepper ... event=0x12` appears and whether it causes more step/tick calls.
- Need confirm whether `main.exe+0xE212E0` finally runs and creates BinkTexture at `CSMovieIns+0xB8`.
- Still unknown whether successful BinkTexture playback will bind to the visible `MENU_DummyMovie` layer.

### Next Step

Run ER once with the current ER ini. Expected new useful lines:

- `movie imp trigger: setup returned 0x01`
- `movie imp trigger: signaled CSMovieImp stepper ... event=0x12`
- additional movie tick/step lines beyond the previous single transition
- ideally `movie ins probe: init call` at `main.exe+0xE212E0` and after-open `bink_texture[+B8]` nonzero.

## 2026-06-19 16:34 CST - Signal Protect Build Deployed

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` after context compaction before continuing.
- Read the current ER deploy log and confirmed it was from the previous deployed DLL:
  - `movie_imp_trigger` still wrote `movie:/00001010.bk2` successfully.
  - `CSMovieImp+0x40` was written to the current `CSMovieIns`.
  - the stepper signal was skipped with `unreadable vtable=0x...EC88`.
  - no `movie ins probe: init call` at `main.exe+0xE212E0` appeared.
- Confirmed the source tree already contains the widened readable-memory protection check:
  - `PAGE_EXECUTE`
  - `PAGE_WRITECOPY`
  - `PAGE_EXECUTE_WRITECOPY`
- Compared SHA256 hashes and found the release build DLL differed from the ER/NR deployed DLLs.
- Verified no `eldenring` or `nightreign` process was running.
- Backed up the previous ER and NR deployed DLLs with suffix `before_signal_protect_20260619_163359`.
- Backed up the old ER log as `dynamic-title-bg.log.before_signal_protect_20260619_163359`.
- Deployed the latest release DLL to both ER and NR deployment directories.
- Confirmed the release, ER deploy, and NR deploy DLL hashes now match.
- Cleared the active ER log for the next test.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`

### Current Judgment

The previous ER log did not test the widened memory-protection fix because the deploy DLL was older than the release build. The next ER run should now exercise the updated guard and should no longer skip the CSMovieImp stepper vtable solely because it is execute-protected.

### Unresolved

- Need a fresh ER run with the newly deployed DLL.
- Need confirm whether the log now contains `movie imp trigger: signaled CSMovieImp stepper ... event=0x12`.
- Need confirm whether that signal causes `main.exe+0xE212E0` to run and create `CSMovieIns+0xB8` BinkTexture.
- Still unknown whether a successful BinkTexture open will bind to the visible title `MENU_DummyMovie` layer.

### Next Step

Run ER once with the current ER ini. The active log should be fresh and should include either:

- `movie imp trigger: signaled CSMovieImp stepper object=... slot[+20]=... event=0x12 result=...`, or
- a new, more specific skip/crash point after the widened protection guard.

## 2026-06-19 16:37 CST - ER Stepper Signal Static Validation

### Completed

- Statically disassembled ER `main.exe+0xE1F400` wrapper and confirmed the post-setup sequence:
  - `call main.exe+0xE20F90`
  - on success, `CSMovieImp+0x40 = CSMovieImp+0x38`
  - `lea rcx, [CSMovieImp+0x08]`
  - `edx = 0x12`
  - `call [vtable+0x20]`
- Statically dumped ER stepper vtable at RVA `0x2BDEC88`:
  - slot `+0x20` points to `main.exe+0x3E81B0`.
- Disassembled `main.exe+0x3E81B0` and confirmed it begins by writing `dword ptr [rcx+0x18] = 1`, then jumps to `main.exe+0xEB1660`.
- Confirmed the DLL-side stepper signal call shape matches the native wrapper shape: object is `CSMovieImp+0x08`, event/state id is `0x12`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The direct stepper signal logic is structurally correct relative to ER's own wrapper. If the next ER run still fails to reach `main.exe+0xE212E0`, the failure is less likely to be a simple wrong object/signature for the vtable call and more likely to be missing surrounding scheduler state or a later state-machine gate.

### Unresolved

- Need fresh ER runtime log from the newly deployed DLL.
- Need observe whether `main.exe+0xEB1660` side effects after the vtable call cause additional MovieIns processing.

### Next Step

Run ER once and inspect the fresh log. The decisive branch is whether the log progresses from `signaled CSMovieImp stepper` to extra tick/step calls and then to `movie ins probe: init call`.

## 2026-06-19 16:40 CST - ER Stepper Signal Runtime Result and State Slot Logging

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing.
- Read the fresh ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed the newly deployed DLL exercised the widened memory-protection guard:
  - `movie imp trigger: setup returned 0x01`
  - `movie imp trigger: signaled CSMovieImp stepper object=... vtable=... slot[+20]=... event=0x12 result=...`
- Confirmed the stepper signal did not crash.
- Confirmed one tick/step still ran after the signal:
  - `rcx` matched global `CSMovieImp+0x38`.
  - `state[40/44]` advanced from `0/0` to `1/1`.
- Confirmed `main.exe+0xE212E0` still did not run in this log; no `movie ins probe: init call` appeared.
- Observed after the signal/tick sequence that `CSMovieImp+0x40` became `0` again, while `CSMovieIns+0x130` remained `1`.
- Statically checked `main.exe+0xE1F4A0` and `main.exe+0xE20F40`:
  - `E20F40` returns `MovieIns+0x130`.
  - `E1F4A0` only clears `CSMovieImp+0x40` if `E20F40` returns zero, so the observed clear likely happens through another native state/scheduler path after the signal.
- Wrote the `Runtime MovieIns State Table Slots` plan to `PLANS.md`.
- Added read-only runtime logging to `log_movie_step`:
  - prints `MovieIns+0x08` state table pointer;
  - prints first six state slot primary/secondary targets with `caller_rva` decoding.
- Ran `cargo fmt` and `cargo build --release`; build succeeded with only existing warnings:
  - `log_draw_submit_arg_refs` unused.
  - `DecodedFrame.duration` unused.
- Deployed the rebuilt DLL to ER and NR deployment directories.
- Backed up previous deployed DLLs with suffix `before_state_slots_20260619_164027`.
- Backed up the useful ER log as `dynamic-title-bg.log.before_state_slots_20260619_164027`.
- Cleared the active ER log for the next run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`

### Current Judgment

The CSMovieImp post-setup signal is now confirmed working, but it is not sufficient by itself to reach the Bink open helper. ER accepts the movie path/options and wakes the scheduler/state object, but the observed state progression stops after the first transition to state `1/1`. The next narrow observation is to map the runtime state table slots so we know what state 1 is supposed to call.

### Unresolved

- Need a fresh ER run with the state-slot logging DLL.
- Need determine whether state 1 maps to a function that should eventually call `main.exe+0xE212E0`.
- Need determine why `CSMovieImp+0x40` becomes zero again after the signal even though `MovieIns+0x130` remains set.
- Still unknown whether successful BinkTexture creation would bind to the visible title `MENU_DummyMovie` layer.

### Next Step

Run ER once with the current ER ini. The new useful lines are the existing `movie step probe` entries with an added `state_slots:` suffix. Use those runtime slot targets to decide whether the next experiment should be a second scheduler wake, a direct call to an existing wrapper, or a return to static state-machine analysis.

## 2026-06-19 16:48 CST - ER State Table Mapped and State0/State1 Probe Deployed

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, `PLANS.md`, and the latest ER log before continuing.
- Confirmed new `state_slots:` logging worked.
- Mapped ER runtime `MovieIns+0x08` state slots:
  - state 0: `main.exe+0xE212B0`
  - state 1: `main.exe+0xE21750`
  - state 2: `main.exe+0xE212E0` (the BinkTexture/open helper already hooked by `probe_movie_ins`)
  - state 3: `main.exe+0xE21810`
  - state 4: `main.exe+0xE21220`
  - state 5: `main.exe+0xE212D0`
- Confirmed the run still stopped after one outer tick/step:
  - before: `state[40/44]=0/0`
  - after: `state[40/44]=1/1`
  - no `movie ins probe: init call` at `main.exe+0xE212E0`
- Statically disassembled state functions:
  - state 0 `E212B0` calls `E21940` and vtable `+0x30`, which should increment next state and request repeat.
  - state 1 `E21750` checks flags `+0x131/+0x130`, performs a path/resource ready check using the path object around `+0xC8`, and only on success calls `E21940` plus vtable `+0x30` to advance toward state 2.
  - state 2 is confirmed to allocate/create BinkTexture at `+0xB8`.
- Wrote the `MovieIns State0/State1 Runtime Probe` plan to `PLANS.md`.
- Added read-only hooks under existing `probe_movie_step` mode:
  - state 0 `main.exe+0xE212B0`
  - state 1 `main.exe+0xE21750`
- Added compact before/after state logging for those state hooks:
  - state `[+40/+44]`
  - repeat `[+48]`
  - BinkTexture `[+B8]`
  - volume/present/option
  - flags `[+130..+134]`
- Ran `cargo fmt` and `cargo build --release`; build succeeded with only existing warnings:
  - `log_draw_submit_arg_refs` unused.
  - `DecodedFrame.duration` unused.
- Deployed rebuilt DLL to ER and NR deployment directories.
- Backed up previous deployed DLLs with suffix `before_state01_probe_20260619_164819`.
- Backed up the useful ER state-slot log as `dynamic-title-bg.log.before_state01_probe_20260619_164819`.
- Cleared the active ER log for the next run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`

### Current Judgment

ER's state machine is now mapped far enough to see that `E212E0` is not missing; it is state 2. The current trigger gets the MovieIns object to state 1 but does not reach state 2. The next decisive runtime question is whether state 1 is ever actually entered after state 0, and if it is entered, whether its resource-ready check fails.

### Unresolved

- Need fresh ER run with state0/state1 hooks.
- Unknown whether state 0 is setting repeat as expected.
- Unknown whether state 1 is not scheduled, or scheduled but fails its `0x141edc770/0x141edc930` path/resource check.
- Still unknown whether forcing state 2 would create a usable BinkTexture and whether it would bind to `MENU_DummyMovie`.

### Next Step

Run ER once with the current ER ini. Expected new lines:

- `movie state probe: state0 before/after ...`
- possibly `movie state probe: state1 before/after ...`
- if state 1 reaches state 2, the existing `movie ins probe: init call` at `main.exe+0xE212E0`.

If state 1 is not called, focus on scheduler/repeat. If state 1 is called and leaves state at `1/1`, focus on the path/resource ready check or a controlled force-state-2 experiment.

## 2026-06-19 17:00 CST - ER State1 Runtime Result

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing after the new ER log.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed the state0/state1 probes installed and both were called during the MovieIns tick.
- Confirmed state0 behaved as expected:
  - before `state[40/44]=0/0`, `repeat[48]=0`.
  - after `state[40/44]=0/1`, `repeat[48]=1`.
- Confirmed state1 was scheduled immediately after state0:
  - before `state[40/44]=1/1`, `flags[+130]=0001`.
  - return value was `0`.
  - after `state[40/44]=1/1`, `repeat[48]=0`, `flags[+130]=0000`.
- Confirmed state2/open helper `main.exe+0xE212E0` still did not run and `CSMovieIns+0xB8` stayed null.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The scheduler/repeat hypothesis is resolved. ER reaches state1 normally, but state1 rejects the current movie path/open request and clears the pending-open flag before state2 can run. The most likely failing gate is the state1 path/resource ready check around the calls previously identified near `0x141edc770` and `0x141edc930`, or a missing resource registration condition required for `movie:/00001010.bk2`.

### Unresolved

- Need identify the exact state1 branch that clears `+0x130`.
- Need know whether the failure is path existence, resource mount/registration, wrong setup arguments, or an ER-vs-NR path namespace difference.
- Need avoid forcing state2 blindly until the ready-check reason is known.

### Next Step

Write a narrow plan for probing state1's resource-ready helper result, then add read-only logging around the relevant helper calls or branch inputs. If the helper clearly reports path-not-ready for `movie:/00001010.bk2`, test a known ER movie path or adjust how the path is registered before attempting a controlled force-state2 experiment.

## 2026-06-19 17:09 CST - ER State1 Resource Ready Probe Deployed

### Completed

- Wrote the `MovieIns State1 Resource Ready Probe` plan to `PLANS.md` before editing code.
- Statically rechecked ER `main.exe+0xE21750` state1:
  - state1 passes the path payload starting at `CSMovieIns+0xC8` into `main.exe+0x1EDC770`.
  - it then calls `main.exe+0x1EDC930` at return site `main.exe+0xE217D7`.
  - if that helper returns false, state1 clears `CSMovieIns+0x130/+0x132` and does not advance to state2.
- Added a read-only resource-ready hook under the existing `probe_movie_step` test mode:
  - hooks ER `main.exe+0x1EDC930`.
  - logs calls from the state1 callsite `main.exe+0xE217D7` plus the first few calls for sanity.
  - logs the helper object fields `[+00]`, `[+08]`, `[+10]`, `[+18]`, pointer previews, and low-byte return value.
- Ran `cargo fmt`.
- Ran `cargo build --release`; build succeeded with only existing warnings:
  - `log_draw_submit_arg_refs` unused.
  - `DecodedFrame.duration` unused.
- Verified no `eldenring` or `nightreign` process was running.
- Backed up the pre-deploy ER/NR DLLs and ER log with suffix `before_resource_ready_probe_*`.
- Initially attempted deployment from the wrong release path; copy failed before replacing DLLs. Corrected the path to `target\x86_64-pc-windows-msvc\release\dynamic_title_bg.dll`.
- Deployed the rebuilt DLL to ER and NR deployment directories and confirmed release/ER/NR hashes match.
- Cleared the active ER log for the next run.
- Confirmed ER ini remains in the narrow MovieIns test state: `probe_movie_ins=true`, `probe_movie_step=true`, `probe_movie_tick=true`, `movie_imp_trigger=true`, D3D/render/replacement hooks disabled.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`
- `F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\_mod\dynamic_title_bg.dll`

### Current Judgment

The active hypothesis is now precise: state1 is rejecting the request because `main.exe+0x1EDC930` returns false for the resource object constructed from `movie:/00001010.bk2`. The next ER run should reveal whether that object contains null/empty ready handles, which would point to missing ER resource registration or an unavailable path namespace, rather than scheduler failure.

### Unresolved

- Need fresh ER log with `movie resource ready probe:` lines.
- Need determine whether `0x1EDC930` returns low byte `0x00` for the state1 callsite and what object fields are populated.
- Need decide whether to test a known ER movie path, probe constructor helper `0x1EDC770`, or do a controlled force-state2 experiment.

### Next Step

Run ER once with the current DLL/ini. Look for:

- `movie resource ready probe: hook installed`
- `movie resource ready probe: call ... caller_rva=main.exe+0xE217D7 ... low=0x00/0x01`
- the existing state1 before/after lines to correlate the helper result with `+0x130` being cleared.

## 2026-06-19 17:18 CST - ER Resource Ready Failure Confirmed

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing with the new ER log.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed the resource-ready hook installed successfully.
- Confirmed the decisive state1 resource-ready call came from `main.exe+0xE217D7`:
  - helper call `#586` returned `low=0x00`.
  - state1 then returned `0` and cleared `CSMovieIns+0x130` from `0001` to `0000`.
  - state2/open helper `main.exe+0xE212E0` still did not run and `CSMovieIns+0xB8` stayed null.
- Checked movie files on disk:
  - ER has `Game\movie\10010010.bk2`, `13000050.bk2`, `19000010.bk2`, etc.
  - ER does not have `Game\movie\00001010.bk2`.
  - NR has `Game\movie\00001010.bk2`.
  - A local candidate exists at `F:\GoldenAge\movie\00001010.bk2`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The failure is now likely a resource/path availability problem rather than scheduler, MovieIns setup, or CSMovieImp signaling. ER's state1 resource-ready gate rejects `movie:/00001010.bk2`; the simplest explanation is that this movie id is not mounted/registered in ER's `movie:/` namespace because ER has no native `Game\movie\00001010.bk2`.

### Unresolved

- Need confirm whether a known ER movie path, such as `movie:/10010010.bk2`, passes state1 and reaches `main.exe+0xE212E0`.
- If known ER path succeeds, need decide how to make the NR title BK2 visible to ER's movie resource namespace: copy into ER `Game\movie`, use the mod loader path, or hook/redirect resource resolution.
- Still unknown whether a successfully opened MovieIns BinkTexture will draw behind the visible title UI without additional GFX binding work.

### Next Step

Run a control test with a known ER movie id. Temporarily set `movie_imp_path=movie:/10010010.bk2`, clear the ER log, and run ER once. Expected decisive result: `movie resource ready probe` from `main.exe+0xE217D7` should return `low=0x01`, followed by `movie ins probe: init call` at `main.exe+0xE212E0`. If this succeeds, return the path to `00001010` after arranging for ER to resolve that file.

## 2026-06-19 17:20 CST - Known ER Movie Control Test Prepared

### Completed

- Backed up the ER ini before changing it with suffix `before_known_er_movie_*`.
- Temporarily changed `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`:
  - `movie_imp_path=movie:/10010010.bk2`
- Confirmed the ER ini remains a narrow test setup:
  - `enable_dynamic_title=false`
  - `probe_movie_ins=true`
  - `probe_movie_step=true`
  - `probe_movie_tick=true`
  - `movie_imp_trigger=true`
  - `probe_movie_render=false`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next ER run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

This is a control test. If ER's own `movie:/10010010.bk2` passes state1 and reaches `main.exe+0xE212E0`, then the previous failure for `movie:/00001010.bk2` is almost certainly because that id/file is not available through ER's current `movie:/` resource namespace.

### Unresolved

- Need fresh ER log for the known ER movie path.
- Need restore or revise `movie_imp_path` after the control result.

### Next Step

Run ER once. Expected useful lines:

- `movie imp trigger: ... path="movie:/10010010.bk2"`
- `movie resource ready probe: call ... caller_rva=main.exe+0xE217D7 ... low=0x01`
- ideally `movie ins probe: init call` at `main.exe+0xE212E0` and nonzero `bink_texture[+B8]`.

## 2026-06-19 17:32 CST - Known ER Movie Opens and Plays Audio

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing with the new ER log.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed the known ER movie control path `movie:/10010010.bk2` passed state1:
  - `movie resource ready probe` from `main.exe+0xE217D7` returned `low=0x01`.
  - state advanced from `1/1` to `1/2` with `repeat[48]=1`.
  - `movie ins probe: init call` at `main.exe+0xE212E0` fired.
  - after init, `CSMovieIns+0xB8` became nonzero (`BinkTexture` created).
  - subsequent tick/step calls stayed at state `6/6`.
- User confirmed runtime behavior: blue title background remains visible, but BK2 audio can be heard.
- Confirmed the user's path-namespace diagnosis: ER resolves `movie:/...` through the game root `Game\movie` namespace, not `F:\GoldenAge\movie`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The native ER MovieImp/MovieIns playback chain is now proven to work when the BK2 exists in ER's movie namespace. The previous `movie:/00001010.bk2` failure was not a state-machine or setup bug; it was a missing resource in `Game\movie`. However, successful playback currently produces audio only and does not replace the visible title blue background, so the remaining problem is binding/presenting the BinkTexture into the title GFX layer (`Movie` / `MENU_DummyMovie`) rather than merely opening the BK2.

### Unresolved

- Need test `movie:/00001010.bk2` after placing the BK2 where ER can resolve it.
- Need identify why a successfully opened CSMovieIns/BinkTexture is not drawn in the title UI layer.
- Need decide whether to bind existing BinkTexture to the GFX Movie/MENU_DummyMovie object, hook a narrow present/render call, or reproduce NR's title-side binding helper.

### Next Step

Prepare a root-movie test for `00001010`: copy a candidate `00001010.bk2` into ER `Game\movie` if no file already exists, set `movie_imp_path=movie:/00001010.bk2`, clear the ER log, and run ER once. Expected result: state1 should return `low=0x01`, `E212E0` should create BinkTexture, audio should play. Visual binding will likely still require a separate step.

## 2026-06-19 17:37 CST - Root Movie 00001010 Test Prepared

### Completed

- Wrote the `ER Root Movie Namespace Test for 00001010` plan to `PLANS.md` before touching game-root files.
- Verified no `eldenring` or `nightreign` process was running.
- Confirmed `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\movie\00001010.bk2` did not already exist.
- Copied `F:\GoldenAge\movie\00001010.bk2` to `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\movie\00001010.bk2`.
- Backed up the ER ini before changing it with suffix `before_root_movie_00001010_*`.
- Set `movie_imp_path=movie:/00001010.bk2` in `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`.
- Confirmed the ER ini remains in narrow MovieIns test mode:
  - `enable_dynamic_title=false`
  - `probe_movie_ins=true`
  - `probe_movie_step=true`
  - `probe_movie_tick=true`
  - `movie_imp_trigger=true`
  - `probe_movie_render=false`
  - `probe_movie_draw_submit=false`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`
- `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\movie\00001010.bk2`

### Current Judgment

The resource namespace issue is addressed for the next test by placing `00001010.bk2` directly in ER's `Game\movie` directory. If the user's diagnosis is correct, state1 should now return `low=0x01`, `main.exe+0xE212E0` should create a BinkTexture, and audio should play for the target BK2. The blue-background visual issue is expected to remain until the BinkTexture is bound/presented to the title GFX Movie/MENU_DummyMovie layer.

### Unresolved

- Need fresh ER log after root-movie placement.
- Need confirm whether target `00001010` now opens successfully.
- Need solve visual binding after playback is proven with target BK2.

### Next Step

Run ER once. Look for:

- `movie imp trigger: ... path="movie:/00001010.bk2"`
- `movie resource ready probe: call ... caller_rva=main.exe+0xE217D7 ... low=0x01`
- `movie ins probe: init call` at `main.exe+0xE212E0`
- nonzero `bink_texture[+B8]` after init.

## 2026-06-19 17:45 CST - Target 00001010 Opens and Plays Audio

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing after the target BK2 run.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed `movie:/00001010.bk2` now resolves from ER `Game\movie` and passes state1:
  - `movie resource ready probe` from `main.exe+0xE217D7` returned `low=0x01`.
  - `movie ins probe: init call` at `main.exe+0xE212E0` fired.
  - after init, `CSMovieIns+0xB8` became nonzero (`0x7FF3EA3A8730` in this run), confirming BinkTexture creation.
  - path remained `movie:/00001010.bk2` before/after init.
- User confirmed runtime behavior: BK2 audio can be heard, but the title remains the blue GFX background.
- Confirmed current ER ini still has render/D3D draw probes disabled:
  - `probe_movie_render=false`
  - `probe_movie_draw_submit=false`
  - `probe_draw_calls=false`

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The target NR-style BK2 playback chain is working inside ER when the file exists in ER's root `Game\movie` namespace. The remaining blocker is not movie loading, BinkTexture creation, or audio playback. The remaining blocker is presentation/binding: the active `CSMovieIns`/BinkTexture is not connected to the visible title GFX `Movie` / `MENU_DummyMovie` layer, so only audio is observed while the GFX blue background continues to draw.

### Unresolved

- Need determine whether ER calls any MovieIns render/present method for this active `CSMovieIns` during the title screen.
- Need identify the GFX-side object or binding slot corresponding to the visible blue `MENU_DummyMovie` layer.
- Need decide whether to reproduce NR's title GFX binding helper or implement a narrow ER-side sidecar binding from the active MovieIns/BinkTexture to that layer.

### Next Step

Do not continue with resource/state-machine work. Next phase should focus on visual binding. Preferred first observation: enable or add a narrow MovieIns render/present probe, not a global D3D draw hook, to see whether the active `CSMovieIns` is ever asked to render and what draw target/object it uses.

## 2026-06-19 17:51 CST - Narrow MovieIns Render Observation Prepared

### Completed

- Wrote the `Narrow MovieIns Render Observation` plan to `PLANS.md` before changing the test configuration.
- Reviewed existing render probe code and confirmed it hooks only ER `main.exe+0xE215C0` and logs when arguments match the selected active `CSMovieIns`.
- Verified no `eldenring` or `nightreign` process was running.
- Confirmed `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\movie\00001010.bk2` exists.
- Backed up the ER ini before changing it with suffix `before_render_observation_*`.
- Updated `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini` for the render observation:
  - `probe_movie_ins=true`
  - `probe_movie_step=false`
  - `probe_movie_tick=false`
  - `probe_movie_render=true`
  - `probe_movie_draw_submit=false`
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The next run should answer whether ER ever calls the active MovieIns render path for the title BK2. If render logs appear, the issue is likely later in render result/draw target/submission. If render logs do not appear while audio plays, the active MovieIns is playing but is not attached to the visible title UI/GFX render path.

### Unresolved

- Need fresh ER log with the render observation config.
- Need determine whether `movie render probe:` appears for the selected `movie:/00001010.bk2` MovieIns.

### Next Step

Run ER once. Look for:

- `movie ins probe: selected movie parent object=...`
- `movie render probe: call ...`
- `movie render probe: return ... tracked_draw_arg=... tracked_inner=...`

No global D3D draw probe is enabled for this run.

## 2026-06-19 18:00 CST - MovieIns Render Is Active But Visual Still Blue

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing with the render observation log.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed `movie:/00001010.bk2` still opens successfully:
  - `movie ins probe: init call` fired.
  - after init, `CSMovieIns+0xB8` was nonzero.
  - active movie parent object was selected from the path marker.
- Confirmed the target active `CSMovieIns` render path is called repeatedly:
  - first render call from `main.exe+0xE20A17`.
  - total `movie render probe: call` count in the log was 790.
  - render return was stable around `0x7FF6D873D100`.
  - tracked `draw_arg` was `0x7FF449842580` in this run.
  - tracked inner/BinkTexture was `0x7FF451259A60` in this run.
- Confirmed no draw-submit observation was active in this run:
  - `movie draw submit probe` count was 0 because `probe_movie_draw_submit=false`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The active MovieIns is not merely playing audio; ER is also calling its render function every frame. Since the screen remains the blue title GFX background, the likely remaining issue is after MovieIns render: either the render result/draw object is not submitted to the visible title layer, it is submitted behind/under an opaque blue GFX layer, or it is submitted into a non-visible target. The next narrow observation should track whether the render result, draw_arg, or BinkTexture appears in the movie draw-submit path.

### Unresolved

- Need determine whether MovieIns render output is submitted by the engine draw-submit function.
- Need determine whether the blue GFX layer is occluding a rendered movie plane or whether the movie plane is not actually submitted to the visible scene.
- Need avoid the previously unsafe global D3D12 draw probe.

### Next Step

Enable the existing narrow `probe_movie_draw_submit=true` while keeping `probe_movie_render=true` and `probe_draw_calls=false`. Run ER once and look for `movie draw submit probe:` lines that match the tracked parent/render_result/draw_arg/inner values.

## 2026-06-19 18:03 CST - Narrow Movie Draw Submit Observation Prepared

### Completed

- Wrote the `Narrow Movie Draw Submit Observation` plan to `PLANS.md` before changing test configuration.
- Verified no `eldenring` or `nightreign` process was running.
- Backed up the ER ini before changing it with suffix `before_draw_submit_observation_*`.
- Updated `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini` for the submit observation:
  - `probe_movie_ins=true`
  - `probe_movie_step=false`
  - `probe_movie_tick=false`
  - `probe_movie_render=true`
  - `probe_movie_draw_submit=true`
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

This next run should show whether MovieIns render output reaches the narrow engine draw-submit function. The unsafe global D3D draw probe remains disabled.

### Unresolved

- Need fresh ER log with `probe_movie_draw_submit=true`.
- Need determine whether any submit call receives the tracked `parent`, `render_result`, `draw_arg`, or `inner` values from MovieIns render.

### Next Step

Run ER once. Look for:

- `movie render probe: return ... tracked_parent=... tracked_draw_arg=... tracked_inner=...`
- `movie draw submit probe: call ... tracked parent=... render_result=... draw_arg=... inner=...`
- `movie draw submit probe: arg ... refs=...` if matching argument reference logging appears.

## 2026-06-19 18:11 CST - Movie Draw Submit Is Active But Still Occluded/Invisible

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing with the draw-submit observation log.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed active `movie:/00001010.bk2` MovieIns continues to render every frame.
- Confirmed MovieIns draw-submit path is active:
  - `movie draw submit probe: call` appears repeatedly.
  - submit caller is `main.exe+0xE21664`.
  - `r8_movie` matches the active BinkTexture pointer.
  - `r9_draw_arg` matches the tracked `CSMovieIns+0xA8` draw argument.
  - submit return is nonzero.
- This proves the native MovieIns path proceeds through render and draw-submit without using the unsafe global D3D draw hook.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The movie is no longer merely audio-only at the native layer: it opens, renders, and reaches the movie draw-submit path. Since the visible title remains the blue GFX background, the remaining problem is likely layer/composition/binding: the MovieIns draw is either behind an opaque GFX blue layer, submitted into a non-visible target, or not associated with the `05_001_title_logo.gfx` `Movie` / `MENU_DummyMovie` object that should display the BK2 inside the title UI.

### Unresolved

- Need identify the exact visible blue layer in `05_001_title_logo.gfx` and whether it occludes the movie plane.
- Need compare ER/NR title GFX objects and scripts around `Movie`, `MENU_DummyMovie`, `ExternalImage`, and SymbolClass.
- Need determine whether a GFX-side edit can expose the already-submitted movie plane, or whether ER needs an explicit binding between `CSMovieIns` and the GFX Movie object.

### Next Step

Return to GFX/Scaleform layer analysis. Compare ER and NR `05_001_title_logo.gfx` dumps and exported assets for `MENU_DummyMovie`, `Movie`, blue dummy texture, SymbolClass, ExternalImage, and frame/layer order. Avoid global D3D draw probes.

## 2026-06-19 17:18 CST - Hide MENU_DummyMovie GFX Occlusion Test Prepared

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, `PLANS.md`, and the latest ER log after context compaction/resume.
- Confirmed latest ER log still shows the target `movie:/00001010.bk2` active in the native MovieIns path:
  - render calls continue every frame;
  - `movie draw submit probe` is hit from `main.exe+0xE21664`;
  - `r8_movie` matches the active BinkTexture and `r9_draw_arg` matches the tracked draw argument.
- Compared NR and current ER title GFX structure:
  - NR original: `MENU_DummyMovie` is `characterID=2`, root `Movie` sprite is placed at depth 3, `MENU_TitleScene` overlay at depth 5.
  - Current ER test GFX: `MENU_DummyMovie` is `characterID=8`, root `Movie` sprite is placed at depth 2, title/logo sprite is depth 3.
  - Current ER `Movie` sprite contains a static `MENU_DummyMovie` external image, so blue screen can be the dummy GFX image rather than the BK2 output.
- Checked `SB_Title_01.dds`; the `MENU_Title_EldenRing_01` region has alpha and is mostly transparent, so it is less likely to be the full-screen blue blocker.
- Wrote the `Hide MENU_DummyMovie GFX Occlusion Test` plan to `PLANS.md` before modifying GFX files.
- Verified no `eldenring` or `nightreign` process was running.
- Backed up current GFX to `F:\GoldenAge\GA\menu\05_001_title_logo.gfx.before_hide_dummy_visible_20260619_171845`.
- Generated `F:\GoldenAge\GA\menu\xml_05_001_hide_dummy_visible_test.xml` from `xml_05_001_nr_order_test.xml`.
- Changed only the inner `PlaceObject3Tag` for `characterId=8` inside `spriteId=9`:
  - `placeFlagHasVisible=true`
  - `visible=0`
- Converted the XML back to GFX with FFDec and installed it as `F:\GoldenAge\GA\menu\05_001_title_logo.gfx`.
- Verified the installed GFX:
  - root `Movie` instance remains at depth 2;
  - `MENU_DummyMovie` SymbolClass remains;
  - dump shows the dummy placement now has the visible flag bytes (`06 30 ...`) instead of the previous non-visible-flag placement.
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next ER run.
- Confirmed ER ini remains in narrow native movie test mode:
  - `probe_movie_ins=true`
  - `probe_movie_render=true`
  - `probe_movie_draw_submit=true`
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `probe_draw_calls=false`

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\GA\menu\05_001_title_logo.gfx`
- `F:\GoldenAge\GA\menu\05_001_title_logo.gfx.hide_dummy_visible_test`
- `F:\GoldenAge\GA\menu\xml_05_001_hide_dummy_visible_test.xml`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The most likely immediate visible blocker is the static GFX `MENU_DummyMovie` image still drawing in the root `Movie` sprite. The new test keeps the `Movie` instance and symbol binding present, but hides the dummy image itself. If the native MovieIns plane is composited behind the GFX layer, the BK2 video should become visible after this test. If the blue dummy disappears but BK2 still does not appear, then the native draw-submit path is not actually feeding the visible title GFX layer and the next step must be a real GFX texture binding or sidecar bridge.

### Unresolved

- Need fresh ER run with the hidden-dummy GFX.
- Need observe whether the screen changes from blue to BK2 video, black/transparent, or broken title/logo.
- Still unknown whether ER can bind `CSMovieIns` output to `MENU_DummyMovie` without reproducing NR's title-side binding helper.

### Next Step

Run ER once with the current test GFX and current DLL/ini. Interpret:

- BK2 appears behind logo/menu: dummy occlusion was the immediate blocker.
- Blue disappears but no BK2: proceed to GFX texture binding/sidecar bridge.
- UI/title breaks: restore `05_001_title_logo.gfx.before_hide_dummy_visible_20260619_171845` and use a narrower edit.

## 2026-06-19 17:26 CST - Hidden Dummy Result: Black Background

### Completed

- User ran ER with the hidden-`MENU_DummyMovie` GFX test.
- Observed result: the blue dummy background disappeared and became a black background.
- Read the fresh ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed the target movie path is still active after the GFX change:
  - render path continues for `movie:/00001010.bk2`;
  - `movie draw submit probe` continues to hit from `main.exe+0xE21664`;
  - tracked BinkTexture/draw_arg values still match between render and submit.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The blue background was the static `MENU_DummyMovie` dummy image. Hiding that image removes the blue, so the GFX edit is behaving as intended. However, the screen becomes black instead of showing BK2, which proves the native `CSMovieIns` render/submit path is not naturally composited into the visible `05_001_title_logo.gfx` `Movie/MENU_DummyMovie` layer. The remaining blocker is now specifically the missing Scaleform/external-image binding between `MENU_DummyMovie` and the active BinkTexture/MovieIns output.

### Unresolved

- Need identify the ER GFX external image/resource creation path for `MENU_DummyMovie`.
- Need decide whether to reproduce NR's high-level binding helper or implement a narrow sidecar bridge from active MovieIns/BinkTexture to the GFX external image/texture slot.
- Need avoid returning to global D3D draw hooks.

### Next Step

Inspect existing `dynamic-title-bg` code around title SRV/GFX texture probes and Bink plane hijack. Prefer a narrow probe that targets GFX external image/resource creation for `MENU_DummyMovie`, not the global draw chain.

## 2026-06-19 17:33 CST - MENU_DummyMovie SRV Probe Prepared

### Completed

- Inspected existing `dx12_title_texture.rs` and confirmed the current SRV hook is `CreateShaderResourceView` based and probe-only when `probe_title_srv=true` while title hijack/dynamic title are disabled.
- Confirmed prior title SRV logic was size/format based, so it can miss the visible GFX object if the dummy image is atlas-backed or if the previous target size was wrong.
- Wrote the `MENU_DummyMovie SRV Locator Probe` plan to `PLANS.md`.
- Backed up ER ini to `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini.before_dummy_srv_probe_20260619_173321`.
- Updated `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini` for a read-only SRV locator test:
  - `probe_title_srv=true`
  - `enable_title_hijack=false`
  - `enable_dynamic_title=false`
  - `bink_plane_hijack=false`
  - `hijack_resource_width=1920`
  - `hijack_resource_height=1080`
  - `hijack_require_bc7=false`
  - `probe_draw_calls=false`
- Kept the native movie correlation probes enabled:
  - `probe_movie_ins=true`
  - `probe_movie_render=true`
  - `probe_movie_draw_submit=true`
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next ER run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The next useful read-only signal is whether `MENU_DummyMovie` creates an identifiable `1920x1080` D3D12 SRV. If it does, that descriptor can become a precise hijack/binding target. If it does not, the GFX resource is likely atlas-backed or resolved through a higher-level Scaleform image registry, and D3D size-based SRV probing is not enough.

### Unresolved

- Need fresh ER run with `probe_title_srv=true`.
- Need determine whether any `1920x1080` candidate or `probe-only title_index` appears.
- Need still avoid global D3D draw probes.

### Next Step

Run ER once and inspect `dynamic-title-bg.log` for:

- `dx12 title texture probe: SRV hook installed`
- `dx12 title texture probe: candidate ... 1920x1080`
- `dx12 title texture probe: probe-only title_index=...`
- continued `movie render probe` / `movie draw submit probe` for `movie:/00001010.bk2`.

## 2026-06-19 18:34 CST - MENU_DummyMovie SRV Locator Result

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` after context compaction/resume.
- Read the current ER log from `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.
- Confirmed the SRV hook installed successfully:
  - `dx12 title texture probe: CreateShaderResourceView=...`
  - `dx12 title texture probe: SRV hook installed`
- Confirmed five target-sized 1920x1080 SRV matches:
  - title_index `#1`: `DXGI_FORMAT(61)`, descriptor `0x18C9F152880`
  - title_index `#2`: `DXGI_FORMAT(61)`, descriptor `0x18C9F154080`
  - title_index `#3`: `DXGI_FORMAT(28)`, descriptor `0x18C9F155880`
  - title_index `#4`: `DXGI_FORMAT(98)`, descriptor `0x18C9F170080`
  - title_index `#5`: `DXGI_FORMAT(98)`, descriptor `0x18C9F170880`
- Confirmed native movie playback/render/submit is still active for `movie:/00001010.bk2` in the same run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The `DXGI_FORMAT(61)` 1920x1080 resources appear before the later `DXGI_FORMAT(98)` 1920x1080 resources. The format/order strongly suggests `#1/#2` are Bink/movie-plane resources and `#4/#5` are GFX/static external-image resources. This also explains why the current one-way `bink_plane_hijack` code is likely insufficient: it only copies a Bink plane after the target descriptor has already been stored, but in this run the Bink-like resources are created earlier than the likely GFX target descriptors.

The next safest test is not a code change yet. Restore a visible dummy GFX, then use the existing title descriptor hijack with a solid debug fill to identify whether `title_index #4` or `#5` is the visible `MENU_DummyMovie` descriptor.

### Unresolved

- Need identify whether `title_index #4` or `#5` is the visible dummy layer.
- Need avoid hidden-dummy GFX during descriptor hijack tests, because an invisible image can receive a replaced descriptor without showing anything.
- Existing `bink_plane_hijack` likely needs bidirectional source/target caching before it can copy a Bink plane created before the GFX target descriptor.

### Next Step

Prepare a visible-dummy descriptor debug-fill test:

1. Restore `F:\GoldenAge\GA\menu\05_001_title_logo.gfx.before_hide_dummy_visible_20260619_171845` to `05_001_title_logo.gfx`.
2. Set ER ini to use `enable_title_hijack=true`, `probe_title_srv=true`, `hijack_title_index=4`, `atlas_debug_fill=255,0,0,255`, `hijack_resource_width=1920`, `hijack_resource_height=1080`, `hijack_require_bc7=false`.
3. Keep `probe_draw_calls=false` and `bink_plane_hijack=false`.
4. Run ER once. If the dummy background becomes red, `#4` is the visible target; if not, repeat with `hijack_title_index=5`.

## 2026-06-19 18:39 CST - Visible Dummy Descriptor #4 Debug Fill Prepared

### Completed

- Wrote the visible-dummy descriptor debug-fill plan to `PLANS.md`.
- Verified no `eldenring` or `nightreign` process was running.
- Backed up the current hidden-dummy GFX to `F:\GoldenAge\GA\menu\05_001_title_logo.gfx.before_visible_debug_fill_20260619_172902`.
- Restored the visible-dummy GFX from `F:\GoldenAge\GA\menu\05_001_title_logo.gfx.before_hide_dummy_visible_20260619_171845` to `F:\GoldenAge\GA\menu\05_001_title_logo.gfx`.
- Backed up the ER ini to `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini.before_visible_debug_fill_20260619_172902`.
- Updated ER ini for the descriptor #4 debug-fill test:
  - `enable_title_hijack=true`
  - `probe_title_srv=true`
  - `hijack_title_index=4`
  - `hijack_resource_width=1920`
  - `hijack_resource_height=1080`
  - `hijack_require_bc7=false`
  - `atlas_debug_fill=255,0,0,255`
  - `bink_plane_hijack=false`
  - `probe_draw_calls=false`
- Disabled noisy native movie probes/triggers for this visual target test:
  - `probe_movie_ins=false`
  - `probe_movie_render=false`
  - `probe_movie_draw_submit=false`
  - `movie_imp_trigger=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next ER run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\GA\menu\05_001_title_logo.gfx`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The next run is a pure visual descriptor identification test. If the main-menu dummy background turns red, title_index `#4` is the visible `MENU_DummyMovie` target. If it remains blue or otherwise unchanged, the likely visible target is `#5`, and the ini should be changed to `hijack_title_index=5` for a second run.

### Unresolved

- Need user run ER once with the current #4 debug-fill config.
- Need know whether the visible dummy background becomes red.

### Next Step

Run ER once. Do not wait for BK2 playback in this test; `movie_imp_trigger=false`. Report whether the title background is red, blue, black, or broken, then inspect the new log.

## 2026-06-19 18:47 CST - Visible Debug Fill Index Renumbering

### Completed

- User tested `hijack_title_index=4` and `hijack_title_index=5`; both remained blue.
- Re-read `TASK_STATUS.md` and `PLANS.md`.
- Read the latest ER log and current ER ini.
- Found the reason #4/#5 did not affect the visible background in this reduced visual test:
  - `movie_imp_trigger=false` means the earlier `DXGI_FORMAT(61)` Bink-plane 1920x1080 resources no longer appear.
  - With Bink/movie resources absent, the only 1920x1080 title-sized matches are `DXGI_FORMAT(98)` and are numbered `title_index #1` and `#2`.
  - The log shows `skipped title-sized descriptor title_index=#1` and `#2`; there are no `#4/#5` title-sized entries in this run.
- Backed up the current ER ini before retargeting the test.
- Updated ER ini for the next visual debug-fill run:
  - `hijack_title_index=1`
  - `enable_title_hijack=true`
  - `probe_title_srv=true`
  - `atlas_debug_fill=255,0,0,255`
  - `movie_imp_trigger=false`
  - `bink_plane_hijack=false`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The descriptor numbering depends on whether the movie/Bink trigger is active. In the reduced visual debug-fill test, the likely visible dummy descriptors are `#1/#2`, not `#4/#5`. The #4/#5 test did not prove those descriptors are invisible; it only proved they were not present under this reduced config.

### Unresolved

- Need run ER once with `hijack_title_index=1`.
- If #1 remains blue, repeat with `hijack_title_index=2`.
- Need confirm whether the existing debug-fill hijack logs `hijacked title-sized descriptor`.

### Next Step

Run ER once with the current #1 debug-fill config. Check whether the main-menu background becomes red. If it stays blue, switch only `hijack_title_index=2`, clear the log, and run again.

## 2026-06-19 17:39 CST - Actual MENU_DummyMovie Texture Found and 64x36 Debug Fill Prepared

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` after context compaction/resume.
- User found the real ER dummy title background texture at `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\menu\hi\05_dummy-tpf-dcx\MENU_DummyMovie.dds`.
- Parsed the DDS header:
  - width `64`
  - height `36`
  - mip count `1`
  - FourCC `DX10`
  - DXGI format `98`
  - data size/path matches `_witchy-tpf.xml` entry `format=102`.
- Confirmed this explains why targeting `1920x1080` descriptors was wrong for the visible main-menu dummy; user also confirmed `hijack_title_index=1/2` at `1920x1080` affect the pre-main loading screen.
- Wrote the `MENU_DummyMovie 64x36 Descriptor Debug Fill Test` plan to `PLANS.md`.
- Verified no `eldenring` or `nightreign` process was running.
- Backed up the ER ini to `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini.before_dummy64_debug_fill_20260619_173902`.
- Updated ER ini for a narrow visual locator:
  - `enable_title_hijack=true`
  - `probe_title_srv=true`
  - `hijack_title_index=1`
  - `hijack_resource_width=64`
  - `hijack_resource_height=36`
  - `hijack_require_bc7=false`
  - `atlas_rect=0,0,64,36`
  - `atlas_debug_fill=255,0,0,255`
  - `movie_imp_trigger=false`
  - `probe_draw_calls=false`
  - `bink_plane_hijack=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log` for the next ER run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The visible blue main-menu background is very likely the small `64x36` BC7/DXGI 98 `MENU_DummyMovie.dds` texture scaled by GFX, not a `1920x1080` resource. The previous `1920x1080` descriptor tests were hitting loading-screen resources and cannot identify the main-menu dummy binding.

### Unresolved

- Need fresh ER run with the current `64x36` debug-fill config.
- Need know whether the main-menu blue background turns red.
- If it remains blue, need inspect how many `64x36` candidates appear and retarget `hijack_title_index` accordingly.

### Next Step

Run ER once with the current config. Expected successful locator result: the main-menu blue dummy background becomes solid red while title/logo/menu UI remains on top. If not, inspect `dynamic-title-bg.log` for `64x36` candidate ordering and choose the next index.

## 2026-06-19 17:45 CST - MENU_DummyMovie Descriptor Confirmed Visible

### Completed

- User ran ER with the `64x36` debug-fill config and observed the main-menu background turned red while title/logo/menu UI remained visible.
- Read the fresh ER log and confirmed the exact descriptor:
  - `candidate #98 srv#351`
  - resource `0x1d7ca24ae40`
  - `DXGI_FORMAT(98) 64x36 array=1 mips=1`
  - descriptor `0x1D4F1855880`
  - `title_index=#1`
  - debug-fill texture `64x36` was created and applied.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The visible `05_001_title_logo.gfx` `MENU_DummyMovie` layer is definitively the `64x36` DXGI 98 descriptor at `hijack_title_index=1`. This is the correct GFX-side binding target. The remaining task is no longer locating the blue layer; it is bridging/copying the native Bink/Movie texture into this descriptor.

### Unresolved

- Existing `bink_plane_hijack` cannot directly handle this case because it assumes the Bink plane has the same width/height as the title target. Here the target is `64x36`, while the Bink/movie plane is expected to be `1920x1080` `DXGI_FORMAT(61)`.
- Need update the hijack code so it caches a target descriptor by `64x36` title index and a Bink plane source independently, then writes the Bink SRV into the target descriptor once both are available.

### Next Step

Write a code-change plan to `PLANS.md`, then modify `dx12_title_texture.rs` and config/example ini so `bink_plane_hijack` supports separate target and source dimensions/formats/order.

## 2026-06-19 17:49 CST - Bink Plane to MENU_DummyMovie Bridge Prepared

### Completed

- Wrote `Active Plan: Bridge Bink Plane Into MENU_DummyMovie Descriptor` to `PLANS.md` before code modification.
- Modified `dx12_title_texture.rs`:
  - added independent Bink source matching config;
  - caches the visible target descriptor from the normal `64x36` title match;
  - caches the Bink source plane COM resource and SRV desc when the configured source plane appears;
  - attempts bridge application when either side becomes available;
  - logs waiting/source/target/applied states.
- Modified `lib.rs`:
  - `bink_plane_hijack=true` now installs the SRV hook even if title hijack/dynamic title are disabled;
  - added config keys `bink_plane_source_width`, `bink_plane_source_height`, and `bink_plane_source_format`.
- Modified `dynamic-title-bg.example.ini` to document the new Bink bridge keys.
- Built release successfully with `cargo build --release -p dynamic-title-bg`; only pre-existing warnings remained.
- Deployed the new DLL to `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`, backing up the prior DLL with suffix `before_bink_bridge_*`.
- Backed up the ER ini with suffix `before_bink_bridge_20260619_174911`.
- Updated ER ini for the next bridge test:
  - `enable_title_hijack=false`
  - `probe_title_srv=true`
  - `probe_movie_ins=true`
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `hijack_title_index=1`
  - `hijack_resource_width=64`
  - `hijack_resource_height=36`
  - `bink_plane_hijack=true`
  - `bink_plane_target_title_index=1`
  - `bink_plane_source_index=1`
  - `bink_plane_source_width=1920`
  - `bink_plane_source_height=1080`
  - `bink_plane_source_format=61`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\dx12_title_texture.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The next test should prove whether a Bink/movie plane SRV can be directly assigned to the GFX `MENU_DummyMovie` descriptor. If successful, the main menu should show the BK2 video behind title/logo/menu UI. If it remains blue or goes black, the log should distinguish whether the bridge failed because no source plane was captured, the target/source order was wrong, or the SRV write succeeded but Scaleform/rendering rejected the mismatched source.

### Unresolved

- Need fresh ER run with the current bridge config.
- Need check for log lines:
  - `stored title descriptor title_index=#1`
  - `bink plane candidate #1`
  - `stored bink plane source #1`
  - `bink bridge applied source 1920x1080 fmt=61 to title descriptor=...`
- Need observe visual result: BK2 video, blue, black, red, crash, or no UI.

### Next Step

Run ER once with the current config. The red debug-fill hijack is disabled for this run; if the bridge does not apply, the fallback visible background should be the original blue dummy rather than red.

## 2026-06-19 18:00 CST - BK2 Visible Through Bridge, Source Is Wrong Color Plane

### Completed

- User ran ER with the bridge config.
- Observed result: BK2 video appears behind title/logo/menu UI, proving the `CSMovieIns/Bink` output can be bridged into the visible GFX `MENU_DummyMovie` layer.
- Visual issue: the whole video is red-tinted.
- Read the fresh ER log and confirmed:
  - target descriptor was stored at `64x36`, `DXGI_FORMAT(98)`, `title_index=#1`;
  - bridge applied `1920x1080 fmt=61` to the title descriptor;
  - two `DXGI_FORMAT(61) 1920x1080` Bink plane candidates appeared.
- Current interpretation: `DXGI_FORMAT(61)` is a single-channel Bink Y/luma plane. Sampling it through the GFX RGBA shader maps the channel as red, so the bridge is working but using the wrong color representation.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The main architectural problem is solved: BK2 can enter the title GFX layer while logo/menu UI stays above it. Remaining work is color correctness. The best low-risk next test is to retarget the bridge to a direct RGB-like 1920x1080 candidate before implementing a custom YUV->RGB conversion path.

### Unresolved

- Need determine whether an engine-created RGB/conversion texture exists and can be assigned directly.
- `DXGI_FORMAT(61)` source is confirmed visually wrong for the GFX dummy shader.
- Need test `DXGI_FORMAT(98)` 1920x1080 source candidates, then possibly source index #2.

### Next Step

Run a config-only source probe: keep target `64x36/#1`, but set `bink_plane_source_format=98`, `bink_plane_source_width=1920`, `bink_plane_source_height=1080`, `bink_plane_source_index=1`, then run ER once and inspect visual/log result.

## 2026-06-19 18:08 CST - Format98 Source #1 Is Loading Image, Not BK2 RGB

### Completed

- User ran ER with `bink_plane_source_format=98`, `bink_plane_source_width=1920`, `bink_plane_source_height=1080`, `bink_plane_source_index=1`.
- Observed result: a loading-screen image was shifted/inserted into the main menu instead of the BK2 video.
- Read the fresh ER log and confirmed:
  - target `MENU_DummyMovie` descriptor was still correctly stored at `64x36`, `title_index=#1`;
  - source #1 was `DXGI_FORMAT(98) 1920x1080`;
  - bridge applied that source to the title descriptor.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The `DXGI_FORMAT(98) 1920x1080` source #1 is not a converted BK2 RGB texture. It is a static/loading resource. The direct-RGB-source path is looking less likely, but source #2 should be tested once because the log shows a second `DXGI_FORMAT(98) 1920x1080` candidate in the same early cluster.

### Unresolved

- Need test `DXGI_FORMAT(98) 1920x1080 source_index=2`.
- If source #2 is also static/loading, the remaining color-correct path is likely explicit YUV->RGB composition or locating the native Bink shader output rather than raw SRV substitution.

### Next Step

Change only `bink_plane_source_index=2`, clear the ER log, and run ER once more.

## 2026-06-19 18:06 CST - Format98 Source #2 Probe Prepared

### Completed

- Backed up ER ini to `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini.before_format98_source2_20260619_180543`.
- Changed only `bink_plane_source_index=2`.
- Kept:
  - `bink_plane_source_format=98`
  - `bink_plane_source_width=1920`
  - `bink_plane_source_height=1080`
  - target `64x36/#1`
  - `movie_imp_trigger=true`
  - `probe_title_srv=true`
  - `probe_draw_calls=false`
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

This is the final cheap direct-RGB candidate in the observed early `1920x1080 DXGI_FORMAT(98)` pair. If it is also a loading/static image, the direct descriptor-substitution approach cannot provide correct color by selecting an existing `DXGI_FORMAT(98)` 1920x1080 source.

### Unresolved

- Need fresh ER run with `bink_plane_source_index=2`.
- Need visual/log result.

### Next Step

Run ER once. If source #2 is also static/loading or otherwise not the BK2 with correct colors, move to a planned YUV->RGB solution or a more precise native Bink shader-output probe.

## 2026-06-19 18:22 CST - Format98 Source #2 Is Also Loading Image

### Completed

- User tested `bink_plane_source_format=98`, `bink_plane_source_width=1920`, `bink_plane_source_height=1080`, `bink_plane_source_index=2`.
- Observed result: source #2 is also a loading/static image, not the BK2 with correct colors.
- Read the ER log and confirmed:
  - target `MENU_DummyMovie` descriptor was still stored at `64x36`, `title_index=#1`;
  - `DXGI_FORMAT(98) 1920x1080` source candidates #1 and #2 were found;
  - source #2 was stored and bridged into the title descriptor.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The cheap direct-RGB descriptor substitution path is exhausted for the observed `1920x1080 DXGI_FORMAT(98)` pair. Both candidates are loading/static resources. The only confirmed movie source remains `DXGI_FORMAT(61)`, which is visually the BK2 luma/R8 plane and produces a red image when sampled as normal RGBA.

### Unresolved

- Need identify the remaining Bink chroma/UV planes, likely half-sized or differently formatted resources.
- Need know whether GFX respects SRV `Shader4ComponentMapping`; if it does, `R,R,R,1` can at least convert the current red luma plane into grayscale and prove the descriptor mapping is controllable.
- Full correct color still requires either YUV->RGB composition or finding the native Bink shader's final RGB output.

### Next Step

Implement the `Bink Plane Inventory and R8 Swizzle Probe` plan from `PLANS.md`: extend the SRV hook with a narrow plane inventory and a default-off `bink_plane_source_swizzle_rrr1` option, then configure ER to test `DXGI_FORMAT(61)` source #1 with `R,R,R,1` mapping.

## 2026-06-19 18:34 CST - Bink Plane Inventory and R8 Swizzle Probe Prepared

### Completed

- Wrote the `Bink Plane Inventory and R8 Swizzle Probe` plan to `PLANS.md`.
- Added read-only Bink plane inventory logging to the existing `CreateShaderResourceView` hook:
  - default-off through `bink_plane_probe_all`;
  - filters to movie-like 16:9 source/half-source dimensions and interesting formats;
  - logs resource format, SRV format, dimensions, mapping, descriptor, and caller.
- Added optional R8 source swizzle for the bridge:
  - default-off through `bink_plane_source_swizzle_rrr1`;
  - only applies when the stored source format is `DXGI_FORMAT(61)`;
  - rewrites only the copied SRV desc component mapping to `R,R,R,1` for the `MENU_DummyMovie` descriptor.
- Added config parsing and example ini entries for:
  - `bink_plane_probe_all`
  - `bink_plane_source_swizzle_rrr1`
- Built release successfully twice after formatting; only pre-existing warnings remained:
  - `log_draw_submit_arg_refs` unused;
  - `DecodedFrame.duration` unused.
- Deployed the rebuilt DLL to `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`, backing up the previous DLL.
- Backed up and updated ER ini for the next test:
  - target remains `64x36`, `bink_plane_target_title_index=1`;
  - source is back to `1920x1080`, `DXGI_FORMAT(61)`, `bink_plane_source_index=1`;
  - `bink_plane_probe_all=true`;
  - `bink_plane_source_swizzle_rrr1=true`;
  - `probe_draw_calls=false`.
- Cleared `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\dx12_title_texture.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The correct visible target is still solved: `MENU_DummyMovie` is `64x36/#1`. Direct `DXGI_FORMAT(98) 1920x1080` sources #1 and #2 are loading/static images, not BK2 RGB output. The next useful test is whether the confirmed R8 movie/luma plane can be sampled as grayscale through SRV component mapping, while the inventory log searches for likely UV/chroma planes.

### Unresolved

- Need user run ER once with the current ini.
- Need observe whether the main-menu BK2 becomes grayscale, remains red, turns black, or breaks UI.
- Need inspect new `bink inventory` log lines for half-size or same-size Bink chroma candidates.
- Full correct color remains unsolved until a UV/chroma source and conversion/binding path are identified.

### Next Step

Run ER once with the current config. Expected key log lines:

- `bink inventory #...`
- `bink plane candidate #1 ... mapping=...`
- `stored bink plane source #1`
- `bink bridge applied source 1920x1080 fmt=61 swizzle_rrr1=true`

Visual interpretation:

- grayscale BK2: swizzle works; proceed to locating UV/chroma and YUV->RGB composition;
- red BK2: component mapping does not affect the GFX sampling path;
- loading/static image: source matching changed unexpectedly;
- black/no UI/crash: revert swizzle and inspect the log.

## 2026-06-19 18:46 CST - R8 Swizzle Produced Grayscale BK2

### Completed

- User ran ER with `bink_plane_source_format=61`, source index `#1`, and `bink_plane_source_swizzle_rrr1=true`.
- Observed result: BK2 appears in the main menu behind logo/menu UI as grayscale.
- Read the fresh ER log and confirmed:
  - source `DXGI_FORMAT(61) 1920x1080 #1` was stored before the title descriptor;
  - target `MENU_DummyMovie` descriptor was stored later at `64x36`, title index `#1`;
  - bridge applied with `swizzle_rrr1=true`;
  - inventory logged two `DXGI_FORMAT(61) 1920x1080` resources and one nearby `DXGI_FORMAT(28) 1920x1080` resource in the Bink-open cluster.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\PLANS.md`

### Current Judgment

The GFX/Scaleform title layer accepts the bridged Bink descriptor and respects SRV component mapping. This confirms the current `MENU_DummyMovie` bridge is stable enough for production-style binding. The remaining color issue is not layer/binding anymore; it is choosing or producing a correct RGB source. The newly observed `DXGI_FORMAT(28) 1920x1080` resource is the best next direct-source candidate because it appears immediately after the two R8 Bink planes.

### Unresolved

- Need test whether the `DXGI_FORMAT(28) 1920x1080` inventory resource is a usable RGB movie texture.
- If format 28 is not usable, full color likely requires explicit YUV/chroma composition or hooking the native Bink shader output more narrowly.
- The inventory filter did not show obvious `960x540` chroma textures; they may be hidden behind a different aspect/format, array resource, or created outside the current filter.

### Next Step

Prepare a config-only test for `DXGI_FORMAT(28) 1920x1080 source_index=1`, with `bink_plane_source_swizzle_rrr1=false`, keeping the confirmed `64x36/#1` target and global draw hook disabled.

## 2026-06-19 18:58 CST - Color BK2 Main Menu Bridge Succeeded

### Completed

- User ran ER with the `DXGI_FORMAT(28) 1920x1080 source_index=1` config.
- Observed result: BK2 appears in full color behind the title/logo/menu UI.
- Read the fresh ER log and confirmed:
  - `DXGI_FORMAT(61) 1920x1080` planes appeared first but were not selected;
  - `DXGI_FORMAT(28) 1920x1080` appeared as `bink plane candidate #1`;
  - source #1 was stored before the title descriptor;
  - visible `MENU_DummyMovie` target descriptor was stored at `64x36`, title index `#1`;
  - bridge applied `source 1920x1080 fmt=28 swizzle_rrr1=false` to the title descriptor.
- Backed up the ER ini and converted it to a lower-noise success config:
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `bink_plane_hijack=true`
  - target `64x36`, title index `#1`
  - source `1920x1080`, format `28`, source index `#1`
  - `bink_plane_probe_all=false`
  - `bink_plane_source_swizzle_rrr1=false`
  - `probe_movie_ins=false`
  - `probe_title_srv=false`
  - `probe_draw_calls=false`
- Cleared the ER log for a clean verification run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The practical ER restoration path is now proven. ER can open/play `movie:/00001010.bk2`, and the DLL can bridge the native Bink/Movie RGB texture (`DXGI_FORMAT(28) 1920x1080`) into the visible `05_001_title_logo.gfx` `MENU_DummyMovie` descriptor (`DXGI_FORMAT(98) 64x36`) while preserving title/logo/menu UI above it.

This avoids the unstable global D3D draw hook and avoids a custom YUV->RGB path. The direct RGB source is available in the Bink-open SRV sequence, so the final mechanism is descriptor rebinding, not shader conversion.

### Unresolved

- Need one more clean verification with the lower-noise config.
- Current code may still emit some generic SRV candidate logs even when `probe_title_srv=false`; if log noise remains high during clean verification, add a small code cleanup to gate generic candidate logging behind probe/debug switches.
- Need decide whether `movie_imp_trigger` should remain the final ER playback entry point or whether a closer NR-style automatic title binding should still be pursued for polish.

### Next Step

Run ER once with the current lower-noise success config. Expected visual result is the same full-color BK2 behind title/logo/menu UI. Expected key log, if any, should still include bridge application to the `64x36/#1` target; probe-only inventory and movie-ins details should be absent.

## 2026-06-19 19:08 CST - Low-Noise Config Atlas Overwrite Fixed

### Completed

- User ran the first lower-noise config and observed the result changed back to a pure red title background.
- Read the ER log and found the exact failure:
  - `DXGI_FORMAT(28) 1920x1080` source was captured correctly;
  - `MENU_DummyMovie 64x36/#1` target was stored correctly;
  - bridge applied `source 1920x1080 fmt=28 swizzle_rrr1=false`;
  - immediately afterward, the old atlas/debug-fill hijack path also ran on the same title descriptor:
    - `RGBA atlas texture ready 64x36`
    - `hijacked title-sized descriptor #1`
- Root cause: `hijack_title_index=1` remained configured, and the SRV hook could still execute `hijack_descriptor` even when only `bink_plane_hijack=true` was intended.
- Wrote the `Prevent Atlas Hijack From Overwriting Bink Bridge` plan to `PLANS.md`.
- Modified `lib.rs` so bink-only mode is treated as bridge/probe-only when both are false:
  - `enable_title_hijack=false`
  - `enable_dynamic_title=false`
  - `bink_plane_hijack=true`
- Built release successfully; only pre-existing warnings remained.
- Deployed the rebuilt DLL to `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`, backing up the previous DLL.
- Kept the success source/target config:
  - source `DXGI_FORMAT(28) 1920x1080`, source index `#1`;
  - target `64x36`, title index `#1`;
  - `probe_title_srv=false`;
  - `bink_plane_probe_all=false`;
  - `probe_draw_calls=false`.
- Cleared the ER log for the next verification run.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\fromsoftware-rs-0.14.0\examples\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`

### Current Judgment

The color bridge itself did not regress. The red result came from a second descriptor overwrite by the old atlas/debug-fill hijack path after the successful Bink RGB bridge. The new code prevents atlas replacement in bink-only mode, so the `DXGI_FORMAT(28)` bridge should remain visible.

### Unresolved

- Need one more ER run with the rebuilt DLL to confirm the atlas overwrite no longer occurs.
- If red still appears, inspect the fresh log for any remaining `RGBA atlas texture ready` or `hijacked title-sized descriptor` lines.

### Next Step

Run ER once with the current config. Expected result: full-color BK2 behind title/logo/menu UI. Expected log: `bink bridge applied source 1920x1080 fmt=28`, with no `RGBA atlas texture ready` and no `hijacked title-sized descriptor`.

## 2026-06-19 19:16 CST - Final Low-Noise Color Verification Passed

### Completed

- User ran ER with the rebuilt no-atlas-overwrite DLL and current low-noise config.
- Observed result: full-color BK2 video appears behind title/logo/menu UI.
- Read the fresh ER log and confirmed:
  - `movie_imp_trigger` still starts `movie:/00001010.bk2`;
  - `DXGI_FORMAT(28) 1920x1080` appears as `bink plane candidate #1`;
  - visible target `MENU_DummyMovie` descriptor is stored at `title_index=#1`;
  - bridge applies `source 1920x1080 fmt=28 swizzle_rrr1=false`;
  - no `RGBA atlas texture ready` line appears;
  - no `hijacked title-sized descriptor` line appears.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`

### Current Judgment

The practical restoration is stable in the current low-noise configuration. The final working path is:

1. ER opens/plays `movie:/00001010.bk2` through the MovieImp/CSMovieIns path.
2. The SRV hook captures the movie RGB resource `DXGI_FORMAT(28) 1920x1080`, source index `#1`.
3. The hook stores the visible `MENU_DummyMovie` target descriptor at `64x36`, title index `#1`.
4. The hook writes the movie RGB SRV into that target descriptor.
5. The title/logo/menu UI remains above the moving BK2 background.

This avoids global draw hooks, avoids custom YUV conversion, and avoids old atlas/debug-fill overwrite in bink-only mode.

### Unresolved

- Current trigger is still DLL-driven (`movie_imp_trigger=true`) rather than a reconstructed NR high-level `Main/Movie`/`MENU_DummyMovie` binding.
- Generic SRV candidate logging still occurs because the SRV hook logs BC7 candidates even when `probe_title_srv=false`; it is not functionally harmful, but log noise can be cleaned later.

### Next Step

Optional polish stage:

1. Gate generic SRV candidate logging behind `probe_title_srv` or `bink_plane_probe_all`.
2. Consider making the final config/example explicit for the successful ER title BK2 bridge.
3. Decide whether to continue reverse-engineering NR's higher-level title binding or accept the current DLL-driven MovieImp trigger plus descriptor bridge as the working implementation.

## 2026-06-19 18:54 CST - Project Migration to _Project Completed

### Completed

- Re-read `AGENTS.md`, `TASK_STATUS.md`, and `PLANS.md` before continuing.
- Wrote the migration plan to `PLANS.md` before moving files.
- Created an isolated project copy at `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg`.
- Copied the verified `dynamic-title-bg` source, README, and example ini from `fromsoftware-rs-0.14.0\examples\dynamic-title-bg`.
- Replaced the copied workspace-based `Cargo.toml` with standalone package metadata and explicit path dependencies back to the original `fromsoftware-rs-0.14.0` crates.
- Copied and regenerated a local `Cargo.lock` for the standalone project.
- Added `MIGRATION.md` documenting that `_Project\dynamic-title-bg` is now the isolated working copy and recording the currently verified ER BK2 bridge settings.
- Verified the new project builds from its own folder with `cargo build --release --offline`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\Cargo.toml`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\Cargo.lock`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\MIGRATION.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\README.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\...`

### Current Judgment

The working `dynamic-title-bg` project has been successfully migrated into a separate `_Project` folder without modifying or deleting the original example. Future code work should happen in `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg` unless a task explicitly targets the original `fromsoftware-rs-0.14.0\examples` location.

The standalone build output is:

`F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\target\release\dynamic_title_bg.dll`

### Unresolved

- The deployed ER DLL at `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll` still comes from the prior original-workspace build unless explicitly copied from the new `_Project` output.
- Existing old notes and deployment commands may still mention `fromsoftware-rs-0.14.0\examples\dynamic-title-bg`; they should be updated if we standardize on the new project path.
- Build verification used `--offline` because the first standalone build attempted to refresh the `hudhook` git dependency and hit a network timeout. The offline build succeeded using local Cargo cache and lock data.

### Next Step

If continuing implementation, switch build/deploy commands to the new project path:

```powershell
cd F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg
cargo build --release --offline
Copy-Item -LiteralPath .\target\release\dynamic_title_bg.dll -Destination F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll -Force
```

## 2026-06-19 18:59 CST - Status Files Migrated and Rust File Roles Reviewed

### Completed

- Copied root `PLANS.md` into `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`.
- Copied root `TASK_STATUS.md` into `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`.
- Reviewed the standalone project's `.rs` files and current deployed ER ini.
- Confirmed the currently successful runtime path uses only:
  - `movie_imp_trigger=true`
  - `bink_plane_hijack=true`
  - source `DXGI_FORMAT(28) 1920x1080 #1`
  - target `MENU_DummyMovie 64x36 #1`
- Confirmed the true current implementation core is:
  - `src\lib.rs`
  - `src\bink_probe.rs`
  - `src\dx12_title_texture.rs`

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`

### Current Judgment

For future work inside the isolated project, the project-local files should be treated as the active task records:

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`

The current working BK2 title background feature is not using the old ImGui/MediaFoundation overlay path, SYSTEX trigger path, native movie trigger path, or global D3D12 draw probe. Those remain historical experiments or optional debug paths.

### Unresolved

- The root `F:\GoldenAge\fromsoftware-rs\TASK_STATUS.md` and `PLANS.md` still exist as older copies. Future work should read the project-local copies first unless explicitly working at the repository root.
- The deploy DLL has not yet been recopied from `_Project\dynamic-title-bg\target\release`.
- The codebase still contains several historical probe modules that can be cleaned later, but they are not blocking the verified BK2 bridge.

### Next Step

If cleaning the project, first preserve the three core files and then decide whether to archive or remove unused experiment modules:

- keep/core: `lib.rs`, `bink_probe.rs`, `dx12_title_texture.rs`
- likely archive/debug-only: `dx12_draw_probe.rs`, `engine_flag_probe.rs`, `native_movie.rs`, `systex_movie.rs`, `video.rs`

## 2026-06-19 19:11 CST - Legacy Rust Modules Separated

### Completed

- Wrote the cleanup plan to project-local `PLANS.md` before editing.
- Moved unused experiment modules out of `src` into `legacy_unused_rs`:
  - `dx12_draw_probe.rs`
  - `engine_flag_probe.rs`
  - `native_movie.rs`
  - `systex_movie.rs`
  - `video.rs`
- Added `legacy_unused_rs\README.md` describing each archived module and why it is not in the current build.
- Rewrote `src\lib.rs` into a smaller DLL/config entry point that only wires the current active paths:
  - MovieImp/Bink probes from `bink_probe.rs`
  - title SRV/Bink bridge from `dx12_title_texture.rs`
- Removed the old ImGui/hudhook/MediaFoundation overlay path from the active build.
- Removed unused dependencies from `Cargo.toml`; normal dependency tree now only uses `ilhook` and `windows`.
- Cleaned dead per-frame atlas upload code from `dx12_title_texture.rs`.
- Replaced `dynamic-title-bg.example.ini` with the current low-noise BK2 bridge config.
- Updated `README.md` to describe the current BK2 bridge implementation instead of the old MP4 overlay prototype.
- Verified with `cargo build --release --offline`; build passed with no warnings.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\Cargo.toml`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\Cargo.lock`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\README.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\dx12_title_texture.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\legacy_unused_rs\README.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\legacy_unused_rs\dx12_draw_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\legacy_unused_rs\engine_flag_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\legacy_unused_rs\native_movie.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\legacy_unused_rs\systex_movie.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\legacy_unused_rs\video.rs`

### Current Judgment

The active project is now focused on the verified BK2 title background solution. Active source files are only:

- `src\lib.rs`
- `src\bink_probe.rs`
- `src\dx12_title_texture.rs`

Archived files remain available as reference material under `legacy_unused_rs`, but are not compiled.

The latest standalone DLL output is:

`F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\target\release\dynamic_title_bg.dll`

It is approximately `798720` bytes after dependency cleanup.

### Unresolved

- The new cleaned DLL has not been deployed to `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll` in this stage.
- `bink_probe.rs` still contains multiple historical probe functions because they share constants and helpers with the working MovieImp trigger. Those can be split later if desired.
- `dx12_title_texture.rs` still includes static atlas/debug-fill replacement support because it is useful for descriptor targeting, even though normal BK2 bridge use does not need atlas replacement.

### Next Step

Deploy the cleaned DLL when ready:

```powershell
Copy-Item -LiteralPath F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\target\release\dynamic_title_bg.dll -Destination F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll -Force
```

## 2026-06-19 19:23 CST - Deployment INI Minimized

### Completed

- Re-read project-local `TASK_STATUS.md` and `PLANS.md`.
- Checked deployed DLL and confirmed `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll` is the cleaned `798720` byte build.
- Backed up the previous verbose deployment ini to:
  - `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini.before_minimal_20260619_192211`
- Replaced `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini` with the minimal active BK2 bridge config:
  - `movie_imp_trigger=true`
  - `movie_imp_path=movie:/00001010.bk2`
  - `bink_plane_hijack=true`
  - target `MENU_DummyMovie 64x36`, index `1`
  - source `DXGI_FORMAT(28) 1920x1080`, index `1`
  - optional probes disabled
- Verified `cargo build --release --offline` still passes with no warnings.

### Modified Files

- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`

### Current Judgment

The deployment ini no longer needs the many old function/probe keys. It is still useful as a small runtime switch/config file because current code defaults keep the feature disabled unless `movie_imp_trigger=true` and `bink_plane_hijack=true` are set.

Completely removing the ini would require a code change: make the cleaned DLL default to the working ER BK2 bridge settings. That is feasible, but keeping the small ini is safer because it allows disabling or retargeting the bridge without rebuilding the DLL.

### Unresolved

- Need a fresh ER run if we want visual confirmation after the minimal ini rewrite.
- No-ini mode is not implemented yet.

### Next Step

Run ER once with the minimized ini. Expected result is unchanged: full-color `movie:/00001010.bk2` behind title/logo/menu UI.

## 2026-06-19 19:41 CST - Default-Quiet Logging Implemented

### Completed

- Re-read project-local `TASK_STATUS.md`, `PLANS.md`, and `AGENTS.md` before continuing.
- Wrote the default-quiet logging plan to project-local `PLANS.md`.
- Added `log_enabled` / `enable_log` / `log` config parsing, defaulting to `false`.
- Removed unconditional early log writes from `DllMain`.
- Changed config loading so `dynamic-title-bg.log` is only selected after the ini is parsed and logging is explicitly enabled.
- Kept all probe/bridge module logging disabled by passing `None` as `log_path` when `log_enabled=false`.
- Added `log_enabled=false` to:
  - `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\dynamic-title-bg.example.ini`
  - `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- Built the standalone project with `cargo build --release --offline`; build passed with no warnings.
- Deployed the rebuilt DLL to `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`, backing up the previous deployed DLL first.
- Verified with `rg` that no `append_module_log` remains and that logging now routes through `append_log_path(config.log_path.as_ref(), ...)`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`

### Current Judgment

Normal ER use should no longer append to `dynamic-title-bg.log` with the current deployment ini. Existing old log files may remain on disk, but the rebuilt DLL should not add new lines unless `log_enabled=true` is set.

### Unresolved

- A fresh ER run can visually confirm the BK2 bridge still works with the rebuilt DLL and no new log output.
- If future debugging is needed, set `log_enabled=true` in the deployment ini before running ER.

### Next Step

Run ER once with the current deployment ini. Expected result: full-color BK2 behind title/logo/menu UI and no new lines appended to `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.log`.

## 2026-06-19 20:12 CST - Title-Target-Gated Movie Trigger Implemented

### Completed

- Re-read project-local `TASK_STATUS.md` and `PLANS.md` before continuing.
- Created missing project-local `AGENTS.md` with the long-running task protocol.
- Wrote the title-target-gated trigger plan to project-local `PLANS.md`.
- Added `movie_imp_trigger_on_title_target` config parsing, with aliases:
  - `movie_imp_wait_for_title_target`
  - `movie_imp_trigger_after_title_target`
- Refactored the ER MovieImp trigger so the actual setup can be fired once by either:
  - the old fixed delay from DLL attach; or
  - a callback fired after the configured title target descriptor appears.
- Added an optional one-shot callback to `dx12_title_texture`.
- The callback fires when the configured visible `MENU_DummyMovie` target descriptor is stored (`64x36`, title index `#1` in the current config).
- In callback mode, `movie_imp_delay_ms` is now used as a post-title-target delay.
- Updated example and deployment ini:
  - `movie_imp_trigger_on_title_target=true`
  - `movie_imp_delay_ms=500`
  - `log_enabled=false`
- Ran `cargo fmt`.
- Built the standalone project with `cargo build --release --offline`; build passed with no warnings.
- Deployed the rebuilt DLL to `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`, backing up the previous deployed DLL first.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\AGENTS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\lib.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\bink_probe.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\src\dx12_title_texture.rs`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\dynamic-title-bg.example.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic-title-bg.ini`
- `F:\GoldenAge\dll\dynamic_title_bg\dynamic_title_bg.dll`

### Current Judgment

This should stop the BK2 from starting during the pre-title loading page. The normal playback path now waits until the visible title background descriptor is observed, then waits another 500ms before calling ER MovieImp. The bridge still uses the same proven source/target descriptors:

- target: `MENU_DummyMovie 64x36`, title index `#1`
- source: `DXGI_FORMAT(28) 1920x1080`, source index `#1`

### Unresolved

- Need a fresh ER run to confirm the video no longer starts during the loading page.
- If it still starts slightly too early, increase `movie_imp_delay_ms` from `500` to `1000` or `1500`.
- Because `log_enabled=false`, normal verification should be visual/audio. To inspect timing, temporarily set `log_enabled=true`.

### Next Step

Run ER once with the current deployment ini. Expected result: no BK2 playback during the pre-main-menu loading page; BK2 starts shortly after the main menu/title background descriptor appears.

## 2026-06-19 20:25 CST - GitHub Upload Prep Started

### Completed

- Re-read project-local `TASK_STATUS.md`, `PLANS.md`, and `AGENTS.md`.
- Read the GitHub publish workflow guidance.
- Checked local tooling:
  - `git` is available.
  - `gh` is not installed or not on `PATH`.
  - the project folder was not yet a git repository.
- Updated the GitHub upload plan in `PLANS.md`.
- User explicitly requested `_Asset` should also be uploaded, so `_Asset` is now included.
- Added `.gitignore` to exclude:
  - `target/`
  - deployed binaries/debug artifacts
  - logs
  - machine-local `dynamic-title-bg.ini`
  - editor metadata
- Updated `README.md` asset wording to say `_Asset` is included, while runtime BK2/deployed DLL/log/local ini are not.
- Removed the old upstream `repository` URL from `Cargo.toml` to avoid pointing this standalone project at the wrong GitHub repo.
- Added `LICENSE-MIT` and `LICENSE-APACHE` to match `Cargo.toml`.
- Verified `cargo build --release --offline` still passes.
- Initialized a local git repository on branch `main`.
- Staged the intended files, including:
  - source files
  - docs/status/plan files
  - Cargo files
  - license files
  - example ini
  - `_Asset/EldenRing01.png`
  - `_Asset/EldenRing02.png`
  - `_Asset/gfx/05_000_title.gfx`
  - `_Asset/gfx/05_001_title_logo.gfx`
- Confirmed ignored files include `target/release/dynamic_title_bg.dll` and `dynamic-title-bg.ini`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\.gitignore`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\Cargo.toml`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\LICENSE-APACHE`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\LICENSE-MIT`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\PLANS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\README.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\.git\...`

### Current Judgment

The repository is locally initialized and staged correctly for an initial GitHub upload, including `_Asset`. The upload cannot complete yet because:

- Git commit failed due to missing local author identity.
- GitHub CLI `gh` is unavailable, so automatic GitHub repo creation/push is not available from this environment.
- No GitHub remote URL has been provided yet.

### Unresolved

- Need user-provided git identity:
  - `user.name`
  - `user.email`
- Need either:
  - a GitHub remote URL for an already-created repository; or
  - GitHub CLI installed/logged in so the repository can be created and pushed from command line.

### Next Step

Set local git identity, commit the staged files, then add a GitHub remote and push `main`.

## 2026-06-19 20:33 CST - GitHub Initial Upload Completed

### Completed

- Re-read project-local `TASK_STATUS.md`, `PLANS.md`, and `AGENTS.md`.
- Confirmed SSH private key exists at `C:\Users\33333\.ssh\id_ed25519_github`.
- Added the project repository URL to `Cargo.toml`:
  - `https://github.com/KamiyamaShiki0704/dynamic_title`
- Set local-only git identity:
  - `user.name=KamiyamaShiki0704`
  - `user.email=KamiyamaShiki0704@users.noreply.github.com`
- Created initial commit:
  - `98fb104 Initial dynamic title background bridge`
- Added SSH remote:
  - `git@github.com:KamiyamaShiki0704/dynamic_title.git`
- Verified SSH/GitHub access with the provided key.
- Pushed `main` to GitHub successfully.
- Confirmed local branch tracks `origin/main`.

### Modified Files

- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\Cargo.toml`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\TASK_STATUS.md`
- `F:\GoldenAge\fromsoftware-rs\_Project\dynamic-title-bg\.git\...`

### Current Judgment

The project is now uploaded to GitHub:

`https://github.com/KamiyamaShiki0704/dynamic_title`

The pushed commit includes `_Asset`, source, docs, example ini, licenses, and archived experiment modules. Ignored local outputs/config remain excluded.

### Unresolved

- This status update itself is local-only because it was written after the initial push.
- GitHub CLI `gh` is still unavailable, but it is no longer needed for this upload because SSH push worked.

### Next Step

Optionally commit and push this final status update later, or leave `TASK_STATUS.md` as local working-state notes.
