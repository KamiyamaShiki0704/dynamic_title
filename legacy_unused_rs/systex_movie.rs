use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use eldenring::cs::{
    CSFeManHudState, CSFeManImp, CSMenuManImp, CSNowLoadingHelper, CSTaskGroupIndex, CSTaskImp,
};
use fromsoftware_shared::FromStatic;
use ilhook::x64::{CallbackOption, HookFlags, Registers, hook_closure_retn};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_READONLY,
    PAGE_READWRITE, VirtualQuery,
};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static FACTORY_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static FACTORY_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static LAST_MOVIE_OBJECT: AtomicUsize = AtomicUsize::new(0);

const ER_SYSTEX_MOVIE_FACTORY_RVA: usize = 0xE21BA0;
const ER_MOVIE_START_RVA: usize = 0xE20F90;

type Generic4Fn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type MovieStartFn = unsafe extern "system" fn(usize, u32, *const u16, f32, u32, u32, u32) -> u8;

pub(crate) fn install_factory_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    if FACTORY_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2));
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("systex movie factory probe: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let addr = base + ER_SYSTEX_MOVIE_FACTORY_RVA;
        append_log(&format!(
            "systex movie factory probe: hooking eldenring.exe+0x{ER_SYSTEX_MOVIE_FACTORY_RVA:X} addr=0x{addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| factory_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                FACTORY_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("systex movie factory probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("systex movie factory probe: hook failed: {err:?}"));
            }
        }
    });
}

pub(crate) fn trigger_captured_once_after_delay(
    log_path: Option<PathBuf>,
    path: String,
    delay: Duration,
    volume: f32,
    gated: bool,
    stop_on_gate: bool,
    stop_after: Option<Duration>,
    present_option: bool,
) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }

    std::thread::spawn(move || {
        append_log(&format!(
            "systex movie trigger: waiting {delay:?} path=\"{path}\" volume={volume:.3} gated={gated} stop_on_gate={stop_on_gate} stop_after={stop_after:?} present_option={present_option}"
        ));
        std::thread::sleep(delay);

        let deadline = Instant::now() + Duration::from_secs(30);
        let mut object = LAST_MOVIE_OBJECT.load(Ordering::Acquire);
        while object == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
            object = LAST_MOVIE_OBJECT.load(Ordering::Acquire);
        }

        if object == 0 {
            append_log("systex movie trigger: no captured movie object; skipping");
            return;
        }
        if !is_readable(object, 0x140) {
            append_log(&format!(
                "systex movie trigger: captured object is not readable object=0x{object:X}; skipping"
            ));
            return;
        }

        if gated {
            let gate_deadline = Instant::now() + Duration::from_secs(60);
            let mut last_snapshot = TitleGateSnapshot::capture();
            append_log(&format!(
                "systex movie trigger: waiting for title gate: {}",
                last_snapshot.summary()
            ));
            let mut last_dump = Instant::now();
            while !last_snapshot.allows_playback() && Instant::now() < gate_deadline {
                std::thread::sleep(Duration::from_millis(100));
                let snapshot = TitleGateSnapshot::capture();
                if snapshot != last_snapshot {
                    append_log(&format!(
                        "systex movie trigger: title gate state changed: {}",
                        snapshot.summary()
                    ));
                    last_snapshot = snapshot;
                }
                if last_dump.elapsed() >= Duration::from_secs(1) {
                    append_log(&format!(
                        "systex movie trigger: active task groups: {}",
                        active_task_group_summary()
                    ));
                    last_dump = Instant::now();
                }
            }
            if !last_snapshot.allows_playback() {
                append_log(&format!(
                    "systex movie trigger: title gate did not open; skipping: {}",
                    last_snapshot.summary()
                ));
                return;
            }
            append_log(&format!(
                "systex movie trigger: title gate open; starting movie: {}",
                last_snapshot.summary()
            ));
        }

        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("systex movie trigger: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let movie_start_addr = base + ER_MOVIE_START_RVA;

        let active = unsafe { read_u8(object + 0x130) };
        let state = unsafe { read_u32(object + 0x40) };
        let field_f0 = unsafe { read_f32(object + 0xF0) };
        let present_before = unsafe { read_u8(object + 0xF4) };
        if present_option {
            unsafe {
                std::ptr::write_volatile((object + 0xF4) as *mut u8, 1);
            }
        }
        let present_after = unsafe { read_u8(object + 0xF4) };
        append_log(&format!(
            "systex movie trigger: object=0x{object:X} active[+130]=0x{active:02X} state[+40]=0x{state:X} movie_start=0x{movie_start_addr:X} f32[+F0]={field_f0:.3} present[+F4]={present_before:02X}->{present_after:02X}"
        ));

        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);

        let movie_start: MovieStartFn = unsafe { std::mem::transmute(movie_start_addr) };
        let present_arg = u32::from(present_option);
        let result = unsafe { movie_start(object, 1, wide.as_ptr(), volume, present_arg, 0, 0) };
        let active_after = unsafe { read_u8(object + 0x130) };
        let state_after = unsafe { read_u32(object + 0x40) };
        append_log(&format!(
            "systex movie trigger: movie_start returned {result} active[+130]=0x{active_after:02X} state[+40]=0x{state_after:X}"
        ));

        if stop_on_gate {
            monitor_gate_and_stop(object);
        }
        if let Some(stop_after) = stop_after {
            stop_movie_after_delay(object, stop_after);
        }
    });
}

fn factory_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = FACTORY_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;

    if count <= 24 {
        append_log(&format!(
            "systex movie factory probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
    }

    let original: Generic4Fn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if result != 0 && is_readable(result, 0x140) {
        LAST_MOVIE_OBJECT.store(result, Ordering::Release);
    }

    if count <= 24 {
        append_log(&format!(
            "systex movie factory probe: return #{count} result=0x{result:X}"
        ));
        log_movie_object(count, result);
    }
    result
}

fn log_movie_object(count: usize, object: usize) {
    if object == 0 {
        append_log(&format!(
            "systex movie factory probe: object #{count} result=<null>"
        ));
        return;
    }
    if !is_readable(object, 0x140) {
        append_log(&format!(
            "systex movie factory probe: object #{count} result=0x{object:X} not readable"
        ));
        return;
    }

    let vtable = unsafe { read_usize(object) };
    let state = unsafe { read_u32(object + 0x40) };
    let field_48 = unsafe { read_u32(object + 0x48) };
    let field_50 = unsafe { read_usize(object + 0x50) };
    let field_58 = unsafe { read_usize(object + 0x58) };
    let field_f0 = unsafe { read_f32(object + 0xF0) };
    let field_f4 = unsafe { read_u8(object + 0xF4) };
    let field_f8 = unsafe { read_u32(object + 0xF8) };
    let active = unsafe { read_u8(object + 0x130) };
    append_log(&format!(
        "systex movie factory probe: object #{count} ptr=0x{object:X} vtbl=0x{vtable:X} state[+40]=0x{state:X} [+48]=0x{field_48:X} [+50]=0x{field_50:X} [+58]=0x{field_58:X} [+F0]={field_f0:.3} [+F4]=0x{field_f4:02X} [+F8]=0x{field_f8:X} active[+130]=0x{active:02X}"
    ));
}

fn caller_rva(caller: usize) -> String {
    let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
        return "main=<unavailable>".to_string();
    };
    let base = exe.0 as usize;
    if caller >= base {
        format!("main+0x{:X}", caller - base)
    } else {
        "main=<below-base>".to_string()
    }
}

fn is_readable(addr: usize, size: usize) -> bool {
    if addr < 0x10000 || size == 0 {
        return false;
    }

    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    let queried = unsafe {
        VirtualQuery(
            Some(addr as *const _),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if queried == 0 || mbi.State != MEM_COMMIT {
        return false;
    }

    let protect = mbi.Protect;
    let readable = protect == PAGE_READONLY
        || protect == PAGE_READWRITE
        || protect == PAGE_EXECUTE_READ
        || protect == PAGE_EXECUTE_READWRITE;
    if !readable {
        return false;
    }

    let region_start = mbi.BaseAddress as usize;
    let region_end = region_start.saturating_add(mbi.RegionSize);
    addr.checked_add(size).is_some_and(|end| end <= region_end)
}

fn loading_screen_active() -> Option<bool> {
    let menu_loading = unsafe { CSMenuManImp::instance() }.ok().map(|menu_man| {
        let timer = menu_man.loading_screen_data.timer;
        timer.is_finite() && timer > 0.25
    });
    let now_loading = unsafe { CSNowLoadingHelper::instance() }
        .ok()
        .map(|helper| helper.is_loading_screen_active());

    match (menu_loading, now_loading) {
        (Some(a), Some(b)) => Some(a || b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn monitor_gate_and_stop(object: usize) {
    std::thread::spawn(move || {
        let mut last_snapshot = TitleGateSnapshot::capture();
        append_log(&format!(
            "systex movie gate monitor: started object=0x{object:X} {}",
            last_snapshot.summary()
        ));

        for _ in 0..3000 {
            std::thread::sleep(Duration::from_millis(100));
            let snapshot = TitleGateSnapshot::capture();
            if snapshot != last_snapshot {
                append_log(&format!(
                    "systex movie gate monitor: gate state changed: {}",
                    snapshot.summary()
                ));
                last_snapshot = snapshot;
            }

            if !last_snapshot.allows_playback() {
                deactivate_movie_object(object, "left title menu gate");
                return;
            }
        }

        append_log("systex movie gate monitor: timed out without gate close");
    });
}

fn hud_is_default() -> bool {
    unsafe { CSFeManImp::instance() }
        .map(|fe_man| fe_man.hud_state == CSFeManHudState::Default)
        .unwrap_or(false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TitleGateSnapshot {
    title_flow: bool,
    title_step: bool,
    ingame_flow: bool,
    common_flow: bool,
    loading: bool,
    hud_default: bool,
}

impl TitleGateSnapshot {
    fn capture() -> Self {
        Self {
            title_flow: task_group_active(CSTaskGroupIndex::GameFlowInGame_TitleMenu),
            title_step: task_group_active(CSTaskGroupIndex::TaskLineIdx_InGame_TitleMenuStep),
            ingame_flow: task_group_active(CSTaskGroupIndex::GameFlowInGame_InGameMenu),
            common_flow: task_group_active(CSTaskGroupIndex::GameFlowInGame_CommonMenu),
            loading: loading_screen_active().unwrap_or(false),
            hud_default: hud_is_default(),
        }
    }

    fn allows_playback(&self) -> bool {
        (self.title_flow || self.title_step)
            && !self.ingame_flow
            && !self.common_flow
            && !self.loading
            && !self.hud_default
    }

    fn summary(&self) -> String {
        format!(
            "title_flow={} title_step={} ingame_flow={} common_flow={} loading={} hud_default={}",
            self.title_flow,
            self.title_step,
            self.ingame_flow,
            self.common_flow,
            self.loading,
            self.hud_default
        )
    }
}

fn task_group_active(index: CSTaskGroupIndex) -> bool {
    let Ok(task) = (unsafe { CSTaskImp::instance() }) else {
        return false;
    };
    let wanted = index as u32;
    task.inner
        .task_base
        .task_groups
        .iter()
        .find(|entry| entry.index == wanted)
        .map(|entry| entry.active)
        .unwrap_or(false)
}

fn active_task_group_summary() -> String {
    let Ok(task) = (unsafe { CSTaskImp::instance() }) else {
        return "CSTaskImp unavailable".to_string();
    };

    let entries = task
        .inner
        .task_base
        .task_groups
        .iter()
        .filter(|entry| entry.active)
        .map(|entry| {
            let name_end = entry
                .name
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.name.len());
            let name = String::from_utf16_lossy(&entry.name[..name_end]);
            format!("{}:{name}", entry.index)
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

fn deactivate_movie_object(object: usize, reason: &str) {
    if !is_readable(object, 0x140) {
        append_log(&format!(
            "systex movie gate monitor: cannot deactivate unreadable object=0x{object:X} reason={reason}"
        ));
        return;
    }

    let active_before = unsafe { read_u8(object + 0x130) };
    let state_before = unsafe { read_u32(object + 0x40) };
    unsafe {
        std::ptr::write_volatile((object + 0x130) as *mut u8, 0);
        std::ptr::write_volatile((object + 0x40) as *mut u32, 0);
        std::ptr::write_volatile((object + 0x44) as *mut u32, 0);
    }
    let active_after = unsafe { read_u8(object + 0x130) };
    let state_after = unsafe { read_u32(object + 0x40) };
    append_log(&format!(
        "systex movie gate monitor: deactivated object=0x{object:X} reason={reason} active {active_before:02X}->{active_after:02X} state 0x{state_before:X}->0x{state_after:X}"
    ));
}

fn stop_movie_after_delay(object: usize, delay: Duration) {
    std::thread::spawn(move || {
        append_log(&format!(
            "systex movie stop timer: will stop object=0x{object:X} after {delay:?}"
        ));
        std::thread::sleep(delay);
        close_movie_object(object, "stop timer elapsed");
    });
}

fn close_movie_object(object: usize, reason: &str) {
    if !is_readable(object, 0x140) {
        append_log(&format!(
            "systex movie close: object unreadable object=0x{object:X} reason={reason}"
        ));
        return;
    }

    let inner = unsafe { read_usize(object + 0xB8) };
    let active_before = unsafe { read_u8(object + 0x130) };
    let state_before = unsafe { read_u32(object + 0x40) };
    append_log(&format!(
        "systex movie close: object=0x{object:X} inner[+B8]=0x{inner:X} reason={reason} active=0x{active_before:02X} state=0x{state_before:X}"
    ));

    if inner != 0 && is_readable(inner, 0x58) {
        let vtable = unsafe { read_usize(inner) };
        let close_fn = unsafe { read_usize(vtable + 0x10) };
        append_log(&format!(
            "systex movie close: closing inner=0x{inner:X} vtable=0x{vtable:X} close=0x{close_fn:X}"
        ));
        type CloseFn = unsafe extern "system" fn(usize) -> usize;
        let close: CloseFn = unsafe { std::mem::transmute(close_fn) };
        let result = unsafe { close(inner) };
        append_log(&format!(
            "systex movie close: inner close returned 0x{result:X}"
        ));
    }

    unsafe {
        std::ptr::write_volatile((object + 0xB8) as *mut usize, 0);
        std::ptr::write_volatile((object + 0x130) as *mut u8, 0);
        std::ptr::write_volatile((object + 0x40) as *mut u32, 0);
        std::ptr::write_volatile((object + 0x44) as *mut u32, 0);
    }
    let active_after = unsafe { read_u8(object + 0x130) };
    let state_after = unsafe { read_u32(object + 0x40) };
    append_log(&format!(
        "systex movie close: state cleared active {active_before:02X}->{active_after:02X} state 0x{state_before:X}->0x{state_after:X}"
    ));
}

unsafe fn read_usize(addr: usize) -> usize {
    unsafe { std::ptr::read_unaligned(addr as *const usize) }
}

unsafe fn read_u32(addr: usize) -> u32 {
    unsafe { std::ptr::read_unaligned(addr as *const u32) }
}

unsafe fn read_f32(addr: usize) -> f32 {
    unsafe { std::ptr::read_unaligned(addr as *const f32) }
}

unsafe fn read_u8(addr: usize) -> u8 {
    unsafe { std::ptr::read_unaligned(addr as *const u8) }
}

fn append_log(message: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}
