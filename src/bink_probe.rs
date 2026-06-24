use std::ffi::{CString, c_void};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use eldenring::cs::{
    CSFeManHudState, CSFeManImp, CSMenuManImp, CSNowLoadingHelper, CSTaskGroupIndex, CSTaskImp,
    WorldChrMan,
};
use fromsoftware_shared::FromStatic;
use ilhook::x64::{CallbackOption, HookFlags, Registers, hook_closure_retn};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleA, GetProcAddress};
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
    PAGE_EXECUTE_WRITECOPY, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY, VirtualQuery,
};
use windows::core::s;

type BinkOpenFn = unsafe extern "system" fn(*const i8, u32) -> *mut c_void;
type ErMovieInsSetupFn =
    unsafe extern "system" fn(*mut c_void, u8, *const u16, f32, u8, u8, u32) -> u8;
type StepperSignalFn = unsafe extern "system" fn(*mut c_void, u32) -> usize;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static REPLACE_RULE: OnceLock<BinkReplaceRule> = OnceLock::new();
static BINK_OPEN_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static BINK_OPEN_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_WRAPPER_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_WRAPPER_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_INS_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_INS_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_IMP_TRIGGER_STARTED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STEP_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STEP_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STATE0_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STATE1_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_RESOURCE_READY_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_RESOURCE_READY_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_TICK_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_TICK_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_RENDER_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_RENDER_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_RENDER_PROBE_ENABLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_DRAW_SUBMIT_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_DRAW_SUBMIT_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STOP_MONITOR_ENABLED: AtomicUsize = AtomicUsize::new(1);
static MOVIE_STOP_MONITOR_STARTED: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STOP_MONITOR_INTERVAL_MS: AtomicUsize = AtomicUsize::new(100);
static MOVIE_STOP_MONITOR_GRACE_MS: AtomicUsize = AtomicUsize::new(2000);
static STAFFROLL_STATUS_SLOT_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SLOT_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SETUP_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_ONESHOT_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SCENE_A_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SCENE_B_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_CTOR_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_STATUS_SLOT_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SLOT_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_BROAD_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SETUP_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_ONESHOT_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SCENE_A_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_SCENE_B_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static STAFFROLL_CTOR_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static BINK_TEXTURE_OPEN_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static BINK_TEXTURE_OPEN_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static BINK_TEXTURE_FORCE_PRESENT_FLAG: OnceLock<bool> = OnceLock::new();
static BINK_TEXTURE_FORCE_PRESENT_OPTION: OnceLock<bool> = OnceLock::new();
static BINK_TEXTURE_COPY_PRESENT_OPTION_AFTER_OPEN: OnceLock<bool> = OnceLock::new();
static MOVIE_INS_LAYOUT: OnceLock<MovieInsLayout> = OnceLock::new();
static LAST_MOVIE_PARENT: AtomicUsize = AtomicUsize::new(0);
static LAST_MOVIE_RENDER_RESULT: AtomicUsize = AtomicUsize::new(0);
static LAST_MOVIE_DRAW_ARG: AtomicUsize = AtomicUsize::new(0);
static LAST_MOVIE_INNER: AtomicUsize = AtomicUsize::new(0);

const ER_MOVIE_OPEN_WRAPPER_RVA: usize = 0x1E83EE0;
const NR_BINK_TEXTURE_OPEN_RVA: usize = 0x21152A0;
const NR_MOVIE_INS_INIT_PLAY_RVA: usize = 0xF6A0E0;
const ER_MOVIE_INS_SETUP_RVA: usize = 0xE20F90;
const ER_MOVIE_INS_INIT_PLAY_RVA: usize = 0xE212E0;
const ER_MOVIE_STATE_STEP_RVA: usize = 0xE20920;
const ER_MOVIE_STATE0_RVA: usize = 0xE212B0;
const ER_MOVIE_STATE1_RVA: usize = 0xE21750;
const ER_MOVIE_RESOURCE_READY_RVA: usize = 0x1EDC930;
const ER_MOVIE_STATE1_READY_CALLSITE_RVA: usize = 0xE217D7;
const ER_MOVIE_TICK_RVA: usize = 0xE21B70;
const ER_MOVIE_RENDER_STATE_RVA: usize = 0xE215C0;
const ER_MOVIE_DRAW_SUBMIT_RVA: usize = 0x1AEA9A0;
const ER_STAFFROLL_LAST_SLOT_RVA: usize = 0x746E80;
const NR_STAFFROLL_MOVIE_STATUS_SLOT_RVA: usize = 0x975A70;
const NR_STAFFROLL_MOVIE_SLOT_RVA: usize = 0x78FEE0;
const NR_STAFFROLL_MOVIE_SETUP_RVA: usize = 0x78FA80;
const NR_STAFFROLL_ONESHOT_LAMBDA_RVA: usize = 0x9764A0;
const NR_STAFFROLL_SCENE_LAMBDA_A_RVA: usize = 0x9764E0;
const NR_STAFFROLL_SCENE_LAMBDA_B_RVA: usize = 0x9765F0;
const NR_STAFFROLL_CTOR_RVA: usize = 0x974E50;
const ER_STAFFROLL_CTOR_RVA: usize = 0x8BDD60;
const NR_CS_MOVIE_IMP_GLOBAL_RVA: usize = 0x442E0A8;
const ER_CS_MOVIE_IMP_GLOBAL_RVA: usize = 0x45878A8;
const CS_MOVIE_IMP_MOVIE_INS_OFFSET: usize = 0x38;
const NR_STAFFROLL_SLOT00_RVA: usize = 0x7783B0;
const NR_STAFFROLL_SLOT01_RVA: usize = 0x9757B0;
const NR_STAFFROLL_SLOT02_RVA: usize = 0x975A70;
const NR_STAFFROLL_SLOT03_RVA: usize = 0x778430;
const NR_STAFFROLL_SLOT04_RVA: usize = 0x78C980;
const NR_STAFFROLL_SLOT05_RVA: usize = 0x78D250;
const NR_STAFFROLL_SLOT06_RVA: usize = 0x78DB50;
const NR_STAFFROLL_SLOT07_RVA: usize = 0x78EBD0;
const NR_STAFFROLL_SLOT08_RVA: usize = 0x78EBA0;
const NR_STAFFROLL_SLOT09_RVA: usize = 0x78EAB0;
const NR_STAFFROLL_SLOT10_RVA: usize = 0x778420;
const NR_STAFFROLL_SLOT11_RVA: usize = 0x7783F0;
const NR_STAFFROLL_SLOT12_RVA: usize = 0x78CAC0;
const NR_STAFFROLL_SLOT13_RVA: usize = 0x78E1A0;
const NR_STAFFROLL_SLOT14_RVA: usize = 0x778410;
const NR_STAFFROLL_SLOT15_RVA: usize = 0x78EA60;
const NR_STAFFROLL_SLOT16_RVA: usize = 0x78DEA0;
const NR_STAFFROLL_SLOT17_RVA: usize = 0x78FEE0;

#[derive(Clone)]
pub(crate) struct BinkReplaceRule {
    pub(crate) from_contains: String,
    pub(crate) to_path: PathBuf,
}

struct MovieImpTriggerSettings {
    path: String,
    delay: Duration,
    volume: f32,
    setup_flag: u8,
    present: u8,
    unknown: u8,
    option: u32,
}

pub(crate) fn install_async(log_path: Option<PathBuf>, replace_rule: Option<BinkReplaceRule>) {
    if let Some(path) = log_path.as_ref() {
        let _ = LOG_PATH.set(path.clone());
    }
    if let Some(rule) = replace_rule {
        append_log(&format!(
            "bink probe: replace rule from contains \"{}\" to \"{}\"",
            rule.from_contains,
            rule.to_path.display()
        ));
        let _ = REPLACE_RULE.set(rule);
    }
    if BINK_OPEN_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(move || {
        append_log("bink probe: waiting for bink2w64.dll");
        for _ in 0..300 {
            match unsafe { GetModuleHandleA(s!("bink2w64.dll")) } {
                Ok(module) => {
                    let Some(proc) = (unsafe { GetProcAddress(module, s!("BinkOpen")) }) else {
                        append_log("bink probe: bink2w64.dll loaded but BinkOpen was not found");
                        return;
                    };
                    let addr = proc as usize;
                    append_log(&format!("bink probe: BinkOpen=0x{addr:X}"));
                    match unsafe {
                        hook_closure_retn(
                            addr,
                            |registers, original| bink_open_hook(registers, original),
                            CallbackOption::None,
                            HookFlags::empty(),
                        )
                    } {
                        Ok(hook) => {
                            let _ = Box::leak(Box::new(hook));
                            BINK_OPEN_HOOK_INSTALLED.store(1, Ordering::Release);
                            append_log("bink probe: BinkOpen hook installed");
                        }
                        Err(err) => {
                            append_log(&format!("bink probe: BinkOpen hook failed: {err:?}"));
                        }
                    }
                    return;
                }
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
        append_log("bink probe: timed out waiting for bink2w64.dll");
    });
}

pub(crate) fn install_movie_wrapper_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    if MOVIE_WRAPPER_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("movie wrapper probe: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let addr = base + ER_MOVIE_OPEN_WRAPPER_RVA;
        append_log(&format!(
            "movie wrapper probe: hooking eldenring.exe+0x{ER_MOVIE_OPEN_WRAPPER_RVA:X} addr=0x{addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| movie_wrapper_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                MOVIE_WRAPPER_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("movie wrapper probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("movie wrapper probe: hook failed: {err:?}"));
            }
        }
    });
}

pub(crate) fn install_bink_texture_open_probe(
    log_path: Option<PathBuf>,
    force_present_flag: bool,
    force_present_option: bool,
    copy_present_option_after_open: bool,
) {
    if let Some(path) = log_path.as_ref() {
        let _ = LOG_PATH.set(path.clone());
    }
    let _ = BINK_TEXTURE_FORCE_PRESENT_FLAG.set(force_present_flag);
    let _ = BINK_TEXTURE_FORCE_PRESENT_OPTION.set(force_present_option);
    let _ = BINK_TEXTURE_COPY_PRESENT_OPTION_AFTER_OPEN.set(copy_present_option_after_open);
    if force_present_flag {
        append_log("bink texture open probe: force present flag enabled (+0x53 = 1 after open)");
    }
    if force_present_option {
        append_log(
            "bink texture open probe: force present option enabled (r8+0x14 = 1 before open)",
        );
    }
    if copy_present_option_after_open {
        append_log(
            "bink texture open probe: copy present option enabled (rcx+0x53 = r8+0x14 after open)",
        );
    }
    if BINK_TEXTURE_OPEN_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2));
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("bink texture open probe: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let module_name = main_module_name(exe);
        let rva = if module_name.contains("nightreign.exe") {
            NR_BINK_TEXTURE_OPEN_RVA
        } else {
            ER_MOVIE_OPEN_WRAPPER_RVA
        };
        let addr = base + rva;
        append_log(&format!(
            "bink texture open probe: module=\"{module_name}\" hooking main.exe+0x{rva:X} addr=0x{addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| bink_texture_open_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                BINK_TEXTURE_OPEN_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("bink texture open probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("bink texture open probe: hook failed: {err:?}"));
            }
        }
    });
}

pub(crate) fn install_movie_ins_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    if MOVIE_INS_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(|| {
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("movie ins probe: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let module_name = main_module_name(exe);
        let (rva, layout) = if module_name.contains("nightreign.exe") {
            (NR_MOVIE_INS_INIT_PLAY_RVA, NR_MOVIE_INS_LAYOUT)
        } else {
            (ER_MOVIE_INS_INIT_PLAY_RVA, ER_MOVIE_INS_LAYOUT)
        };
        let _ = MOVIE_INS_LAYOUT.set(layout);
        let addr = base + rva;
        append_log(&format!(
            "movie ins probe: module=\"{module_name}\" layout={} hooking main.exe+0x{rva:X} addr=0x{addr:X}",
            layout.name
        ));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| movie_ins_init_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                MOVIE_INS_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("movie ins probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("movie ins probe: hook failed: {err:?}"));
            }
        }
    });
}

pub(crate) fn trigger_er_movie_imp_once_after_delay(
    log_path: Option<PathBuf>,
    path: String,
    delay: Duration,
    volume: f32,
    setup_flag: u8,
    present: u8,
    unknown: u8,
    option: u32,
) {
    trigger_er_movie_imp_once(
        log_path,
        path,
        delay,
        volume,
        setup_flag,
        present,
        unknown,
        option,
        "DLL attach delay",
    );
}

pub(crate) fn trigger_er_movie_imp_once(
    log_path: Option<PathBuf>,
    path: String,
    delay: Duration,
    volume: f32,
    setup_flag: u8,
    present: u8,
    unknown: u8,
    option: u32,
    reason: &'static str,
) {
    if let Some(path) = log_path.as_ref() {
        let _ = LOG_PATH.set(path.clone());
    }
    let settings = MovieImpTriggerSettings {
        path,
        delay,
        volume,
        setup_flag,
        present,
        unknown,
        option,
    };
    if MOVIE_IMP_TRIGGER_STARTED.swap(1, Ordering::AcqRel) != 0 {
        return;
    }

    std::thread::spawn(move || {
        append_log(&format!(
            "movie imp trigger: waiting {:?} after {reason} before ER CSMovieIns setup path=\"{}\" volume={:.3} setup_flag={} present={} unknown={} option={}",
            settings.delay,
            settings.path,
            settings.volume,
            settings.setup_flag,
            settings.present,
            settings.unknown,
            settings.option
        ));
        std::thread::sleep(settings.delay);
        for attempt in 1..=120 {
            if world_player_active() {
                append_log("movie imp trigger: skipped setup because world player is active");
                MOVIE_IMP_TRIGGER_STARTED.store(0, Ordering::Release);
                return;
            }
            if run_er_movie_imp_trigger(
                settings.path.clone(),
                settings.volume,
                settings.setup_flag,
                settings.present,
                settings.unknown,
                settings.option,
            ) {
                return;
            }
            if attempt == 1 || attempt % 10 == 0 {
                append_log(&format!(
                    "movie imp trigger: setup retry pending attempt={attempt}/120"
                ));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        append_log("movie imp trigger: setup retries exhausted");
        MOVIE_IMP_TRIGGER_STARTED.store(0, Ordering::Release);
    });
}

pub(crate) fn configure_movie_imp_stop_monitor(
    log_path: Option<PathBuf>,
    enabled: bool,
    check_interval: Duration,
    grace: Duration,
) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    MOVIE_STOP_MONITOR_ENABLED.store(enabled as usize, Ordering::Release);
    MOVIE_STOP_MONITOR_INTERVAL_MS.store(
        check_interval.as_millis().clamp(10, usize::MAX as u128) as usize,
        Ordering::Release,
    );
    MOVIE_STOP_MONITOR_GRACE_MS.store(
        grace.as_millis().min(usize::MAX as u128) as usize,
        Ordering::Release,
    );
    append_log(&format!(
        "movie imp stop monitor: configured enabled={} interval_ms={} grace_ms={}",
        enabled,
        MOVIE_STOP_MONITOR_INTERVAL_MS.load(Ordering::Acquire),
        MOVIE_STOP_MONITOR_GRACE_MS.load(Ordering::Acquire)
    ));
}

fn run_er_movie_imp_trigger(
    path: String,
    volume: f32,
    setup_flag: u8,
    present: u8,
    unknown: u8,
    option: u32,
) -> bool {
    let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
        append_log("movie imp trigger: main module unavailable");
        return false;
    };
    let module_name = main_module_name(exe);
    if !module_name.contains("eldenring.exe") {
        append_log(&format!(
            "movie imp trigger: skipped for non-ER module \"{module_name}\""
        ));
        return false;
    }

    let base = exe.0 as usize;
    let global_addr = base + ER_CS_MOVIE_IMP_GLOBAL_RVA;
    if !is_readable_memory(global_addr, 8) {
        append_log(&format!(
            "movie imp trigger: global[main.exe+0x{ER_CS_MOVIE_IMP_GLOBAL_RVA:X}] unreadable"
        ));
        return false;
    }

    let imp = unsafe { read_usize(global_addr) };
    if imp == 0 || !is_readable_memory(imp + CS_MOVIE_IMP_MOVIE_INS_OFFSET, 8) {
        append_log(&format!(
            "movie imp trigger: global[main.exe+0x{ER_CS_MOVIE_IMP_GLOBAL_RVA:X}]=0x{imp:X} has no readable CSMovieIns"
        ));
        return false;
    }

    let movie_ins = unsafe { read_usize(imp + CS_MOVIE_IMP_MOVIE_INS_OFFSET) };
    if movie_ins == 0 {
        append_log(&format!(
            "movie imp trigger: CSMovieImp=0x{imp:X} [+38]=<null>"
        ));
        return false;
    }

    let _ = MOVIE_INS_LAYOUT.set(ER_MOVIE_INS_LAYOUT);
    LAST_MOVIE_PARENT.store(movie_ins, Ordering::Release);
    append_log(&format!(
        "movie imp trigger: CSMovieImp=0x{imp:X} CSMovieIns[+38]=0x{movie_ins:X} setup=main.exe+0x{ER_MOVIE_INS_SETUP_RVA:X}"
    ));
    log_movie_ins(0, movie_ins, "trigger-before");

    let setup_addr = base + ER_MOVIE_INS_SETUP_RVA;
    let setup: ErMovieInsSetupFn = unsafe { std::mem::transmute(setup_addr) };
    let wide_path = path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        setup(
            movie_ins as *mut c_void,
            setup_flag,
            wide_path.as_ptr(),
            volume.clamp(0.0, 1.0),
            present,
            unknown,
            option,
        )
    };

    append_log(&format!("movie imp trigger: setup returned 0x{result:02X}"));
    if result != 0 {
        crate::dx12_title_texture::enable_bink_bridge_source_capture("movie imp setup succeeded");
        unsafe {
            std::ptr::write_unaligned((imp + 0x40) as *mut usize, movie_ins);
        }
        let step_obj = imp + 0x08;
        if is_readable_memory(step_obj, 8) {
            let step_vtable = unsafe { read_usize(step_obj) };
            if step_vtable != 0 && is_readable_memory(step_vtable + 0x20, 8) {
                let signal_addr = unsafe { read_usize(step_vtable + 0x20) };
                let signal: StepperSignalFn = unsafe { std::mem::transmute(signal_addr) };
                let signal_result = unsafe { signal(step_obj as *mut c_void, 0x12) };
                append_log(&format!(
                    "movie imp trigger: signaled CSMovieImp stepper object=0x{step_obj:X} vtable=0x{step_vtable:X} slot[+20]=0x{signal_addr:X} event=0x12 result=0x{signal_result:X}"
                ));
            } else {
                append_log(&format!(
                    "movie imp trigger: stepper signal skipped unreadable vtable=0x{step_vtable:X}"
                ));
            }
        } else {
            append_log(&format!(
                "movie imp trigger: stepper signal skipped unreadable object=0x{step_obj:X}"
            ));
        }
    }
    log_movie_ins(0, movie_ins, "trigger-after");
    if result != 0 && MOVIE_STOP_MONITOR_ENABLED.load(Ordering::Acquire) != 0 {
        start_movie_imp_stop_monitor(movie_ins);
    }
    result != 0
}

pub(crate) fn install_movie_step_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    if MOVIE_STEP_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2));
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("movie step probe: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let addr = base + ER_MOVIE_STATE_STEP_RVA;
        append_log(&format!(
            "movie step probe: hooking eldenring.exe+0x{ER_MOVIE_STATE_STEP_RVA:X} addr=0x{addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| movie_step_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                MOVIE_STEP_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("movie step probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("movie step probe: hook failed: {err:?}"));
            }
        }

        let state0_addr = base + ER_MOVIE_STATE0_RVA;
        append_log(&format!(
            "movie state probe: hooking state0 eldenring.exe+0x{ER_MOVIE_STATE0_RVA:X} addr=0x{state0_addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                state0_addr,
                |registers, original| movie_state0_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                append_log("movie state probe: state0 hook installed");
            }
            Err(err) => {
                append_log(&format!("movie state probe: state0 hook failed: {err:?}"));
            }
        }

        let state1_addr = base + ER_MOVIE_STATE1_RVA;
        append_log(&format!(
            "movie state probe: hooking state1 eldenring.exe+0x{ER_MOVIE_STATE1_RVA:X} addr=0x{state1_addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                state1_addr,
                |registers, original| movie_state1_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                append_log("movie state probe: state1 hook installed");
            }
            Err(err) => {
                append_log(&format!("movie state probe: state1 hook failed: {err:?}"));
            }
        }

        let ready_addr = base + ER_MOVIE_RESOURCE_READY_RVA;
        append_log(&format!(
            "movie resource ready probe: hooking eldenring.exe+0x{ER_MOVIE_RESOURCE_READY_RVA:X} addr=0x{ready_addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                ready_addr,
                |registers, original| movie_resource_ready_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                MOVIE_RESOURCE_READY_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("movie resource ready probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("movie resource ready probe: hook failed: {err:?}"));
            }
        }
    });
}

pub(crate) fn install_movie_tick_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    if MOVIE_TICK_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2));
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("movie tick probe: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let addr = base + ER_MOVIE_TICK_RVA;
        append_log(&format!(
            "movie tick probe: hooking eldenring.exe+0x{ER_MOVIE_TICK_RVA:X} addr=0x{addr:X}"
        ));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| movie_tick_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                MOVIE_TICK_HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("movie tick probe: hook installed");
            }
            Err(err) => {
                append_log(&format!("movie tick probe: hook failed: {err:?}"));
            }
        }
    });
}

pub(crate) fn install_movie_render_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    MOVIE_RENDER_PROBE_ENABLED.store(1, Ordering::Release);
    install_rva_hook_once(
        ER_MOVIE_RENDER_STATE_RVA,
        "movie render probe",
        &MOVIE_RENDER_HOOK_INSTALLED,
        movie_render_hook,
    );
}

pub(crate) fn install_movie_draw_submit_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    install_rva_hook_once(
        ER_MOVIE_DRAW_SUBMIT_RVA,
        "movie draw submit probe",
        &MOVIE_DRAW_SUBMIT_HOOK_INSTALLED,
        movie_draw_submit_hook,
    );
}

pub(crate) fn install_staffroll_screen_probe(log_path: Option<PathBuf>, broad: bool) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }

    std::thread::spawn(move || {
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("staffroll screen probe: main module unavailable");
            return;
        };
        let module_name = main_module_name(exe);
        if module_name.contains("nightreign.exe") {
            append_log("staffroll screen probe: installing Nightreign title/movie probes");
            if broad {
                install_staffroll_broad_slot_probes(exe);
            } else {
                install_rva_hook_once_now(
                    exe,
                    NR_STAFFROLL_MOVIE_STATUS_SLOT_RVA,
                    "staffroll screen probe: NR slot2/movie status",
                    &STAFFROLL_STATUS_SLOT_HOOK_INSTALLED,
                    staffroll_status_slot_hook,
                );
                install_rva_hook_once_now(
                    exe,
                    NR_STAFFROLL_MOVIE_SLOT_RVA,
                    "staffroll screen probe: NR slot17/movie init",
                    &STAFFROLL_SLOT_HOOK_INSTALLED,
                    staffroll_slot_hook,
                );
            }
            install_rva_hook_once_now(
                exe,
                NR_STAFFROLL_MOVIE_SETUP_RVA,
                "staffroll screen probe: NR movie setup",
                &STAFFROLL_SETUP_HOOK_INSTALLED,
                staffroll_setup_hook,
            );
            install_rva_hook_once_now(
                exe,
                NR_STAFFROLL_ONESHOT_LAMBDA_RVA,
                "staffroll screen probe: NR OneShot lambda",
                &STAFFROLL_ONESHOT_HOOK_INSTALLED,
                staffroll_oneshot_hook,
            );
            install_rva_hook_once_now(
                exe,
                NR_STAFFROLL_SCENE_LAMBDA_A_RVA,
                "staffroll screen probe: NR SceneObj lambda A",
                &STAFFROLL_SCENE_A_HOOK_INSTALLED,
                staffroll_scene_a_hook,
            );
            install_rva_hook_once_now(
                exe,
                NR_STAFFROLL_SCENE_LAMBDA_B_RVA,
                "staffroll screen probe: NR SceneObj lambda B",
                &STAFFROLL_SCENE_B_HOOK_INSTALLED,
                staffroll_scene_b_hook,
            );
        } else {
            append_log(
                "staffroll screen probe: installing Elden Ring StaffRollScreen last-slot probe",
            );
            install_rva_hook_once_now(
                exe,
                ER_STAFFROLL_LAST_SLOT_RVA,
                "staffroll screen probe: ER last slot",
                &STAFFROLL_SLOT_HOOK_INSTALLED,
                staffroll_slot_hook,
            );
        }
    });
}

pub(crate) fn install_staffroll_ctor_probe(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }

    std::thread::spawn(move || {
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("staffroll ctor probe: main module unavailable");
            return;
        };
        let module_name = main_module_name(exe);
        let (rva, label) = if module_name.contains("nightreign.exe") {
            (
                NR_STAFFROLL_CTOR_RVA,
                "staffroll ctor probe: NR StaffRollScreen ctor",
            )
        } else {
            (
                ER_STAFFROLL_CTOR_RVA,
                "staffroll ctor probe: ER StaffRollScreen ctor",
            )
        };
        append_log(&format!(
            "staffroll ctor probe: module=\"{module_name}\" selected main.exe+0x{rva:X}"
        ));
        install_rva_hook_once_now(
            exe,
            rva,
            label,
            &STAFFROLL_CTOR_HOOK_INSTALLED,
            staffroll_ctor_hook,
        );
    });
}

fn install_staffroll_broad_slot_probes(exe: HMODULE) {
    append_log("staffroll screen probe: installing broad NR StaffRollScreen vtable slot probes");
    let slots: &[(usize, &'static str, fn(*mut Registers, usize) -> usize)] = &[
        (
            NR_STAFFROLL_SLOT00_RVA,
            "staffroll screen probe: NR broad slot00",
            staffroll_broad_slot00_hook,
        ),
        (
            NR_STAFFROLL_SLOT01_RVA,
            "staffroll screen probe: NR broad slot01",
            staffroll_broad_slot01_hook,
        ),
        (
            NR_STAFFROLL_SLOT02_RVA,
            "staffroll screen probe: NR broad slot02",
            staffroll_broad_slot02_hook,
        ),
        (
            NR_STAFFROLL_SLOT03_RVA,
            "staffroll screen probe: NR broad slot03",
            staffroll_broad_slot03_hook,
        ),
        (
            NR_STAFFROLL_SLOT04_RVA,
            "staffroll screen probe: NR broad slot04",
            staffroll_broad_slot04_hook,
        ),
        (
            NR_STAFFROLL_SLOT05_RVA,
            "staffroll screen probe: NR broad slot05",
            staffroll_broad_slot05_hook,
        ),
        (
            NR_STAFFROLL_SLOT06_RVA,
            "staffroll screen probe: NR broad slot06",
            staffroll_broad_slot06_hook,
        ),
        (
            NR_STAFFROLL_SLOT07_RVA,
            "staffroll screen probe: NR broad slot07",
            staffroll_broad_slot07_hook,
        ),
        (
            NR_STAFFROLL_SLOT08_RVA,
            "staffroll screen probe: NR broad slot08",
            staffroll_broad_slot08_hook,
        ),
        (
            NR_STAFFROLL_SLOT09_RVA,
            "staffroll screen probe: NR broad slot09",
            staffroll_broad_slot09_hook,
        ),
        (
            NR_STAFFROLL_SLOT10_RVA,
            "staffroll screen probe: NR broad slot10",
            staffroll_broad_slot10_hook,
        ),
        (
            NR_STAFFROLL_SLOT11_RVA,
            "staffroll screen probe: NR broad slot11",
            staffroll_broad_slot11_hook,
        ),
        (
            NR_STAFFROLL_SLOT12_RVA,
            "staffroll screen probe: NR broad slot12",
            staffroll_broad_slot12_hook,
        ),
        (
            NR_STAFFROLL_SLOT13_RVA,
            "staffroll screen probe: NR broad slot13",
            staffroll_broad_slot13_hook,
        ),
        (
            NR_STAFFROLL_SLOT14_RVA,
            "staffroll screen probe: NR broad slot14",
            staffroll_broad_slot14_hook,
        ),
        (
            NR_STAFFROLL_SLOT15_RVA,
            "staffroll screen probe: NR broad slot15",
            staffroll_broad_slot15_hook,
        ),
        (
            NR_STAFFROLL_SLOT16_RVA,
            "staffroll screen probe: NR broad slot16",
            staffroll_broad_slot16_hook,
        ),
        (
            NR_STAFFROLL_SLOT17_RVA,
            "staffroll screen probe: NR broad slot17",
            staffroll_broad_slot17_hook,
        ),
    ];

    for (rva, label, hook_fn) in slots {
        install_rva_hook_now_unchecked(exe, *rva, label, *hook_fn);
    }
}

fn install_rva_hook_once(
    rva: usize,
    label: &'static str,
    installed: &'static AtomicUsize,
    hook_fn: fn(*mut Registers, usize) -> usize,
) {
    if installed.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log(&format!("{label}: main module unavailable"));
            return;
        };
        install_rva_hook_once_now(exe, rva, label, installed, hook_fn);
    });
}

fn install_rva_hook_once_now(
    exe: HMODULE,
    rva: usize,
    label: &'static str,
    installed: &'static AtomicUsize,
    hook_fn: fn(*mut Registers, usize) -> usize,
) {
    if installed.load(Ordering::Acquire) != 0 {
        return;
    }

    let base = exe.0 as usize;
    let addr = base + rva;
    append_log(&format!(
        "{label}: hooking main.exe+0x{rva:X} addr=0x{addr:X}"
    ));
    match unsafe { hook_closure_retn(addr, hook_fn, CallbackOption::None, HookFlags::empty()) } {
        Ok(hook) => {
            let _ = Box::leak(Box::new(hook));
            installed.store(1, Ordering::Release);
            append_log(&format!("{label}: hook installed"));
        }
        Err(err) => {
            append_log(&format!("{label}: hook failed: {err:?}"));
        }
    }
}

fn install_rva_hook_now_unchecked(
    exe: HMODULE,
    rva: usize,
    label: &'static str,
    hook_fn: fn(*mut Registers, usize) -> usize,
) {
    let base = exe.0 as usize;
    let addr = base + rva;
    append_log(&format!(
        "{label}: hooking main.exe+0x{rva:X} addr=0x{addr:X}"
    ));
    match unsafe { hook_closure_retn(addr, hook_fn, CallbackOption::None, HookFlags::empty()) } {
        Ok(hook) => {
            let _ = Box::leak(Box::new(hook));
            append_log(&format!("{label}: hook installed"));
        }
        Err(err) => {
            append_log(&format!("{label}: hook failed: {err:?}"));
        }
    }
}

type MovieInsInitFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type MovieStateStepFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type MovieTickFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type MovieRenderFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type MovieDrawSubmitFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type Probe4Fn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

#[derive(Clone, Copy)]
struct MovieInsLayout {
    name: &'static str,
    bink_texture_offset: usize,
    path_offset: usize,
    volume_offset: usize,
    present_offset: usize,
    option_offset: usize,
}

const ER_MOVIE_INS_LAYOUT: MovieInsLayout = MovieInsLayout {
    name: "ER",
    bink_texture_offset: 0xB8,
    path_offset: 0xC0,
    volume_offset: 0xF0,
    present_offset: 0xF4,
    option_offset: 0xF8,
};

const NR_MOVIE_INS_LAYOUT: MovieInsLayout = MovieInsLayout {
    name: "NR",
    bink_texture_offset: 0xC0,
    path_offset: 0xC8,
    volume_offset: 0xF8,
    present_offset: 0xFC,
    option_offset: 0x100,
};

macro_rules! staffroll_broad_slot_hook {
    ($name:ident, $label:literal) => {
        fn $name(registers: *mut Registers, original: usize) -> usize {
            staffroll_probe_hook(
                registers,
                original,
                $label,
                &STAFFROLL_BROAD_CALL_COUNT,
                true,
            )
        }
    };
}

staffroll_broad_slot_hook!(staffroll_broad_slot00_hook, "broad_slot00");
staffroll_broad_slot_hook!(staffroll_broad_slot01_hook, "broad_slot01");
staffroll_broad_slot_hook!(staffroll_broad_slot02_hook, "broad_slot02");
staffroll_broad_slot_hook!(staffroll_broad_slot03_hook, "broad_slot03");
staffroll_broad_slot_hook!(staffroll_broad_slot04_hook, "broad_slot04");
staffroll_broad_slot_hook!(staffroll_broad_slot05_hook, "broad_slot05");
staffroll_broad_slot_hook!(staffroll_broad_slot06_hook, "broad_slot06");
staffroll_broad_slot_hook!(staffroll_broad_slot07_hook, "broad_slot07");
staffroll_broad_slot_hook!(staffroll_broad_slot08_hook, "broad_slot08");
staffroll_broad_slot_hook!(staffroll_broad_slot09_hook, "broad_slot09");
staffroll_broad_slot_hook!(staffroll_broad_slot10_hook, "broad_slot10");
staffroll_broad_slot_hook!(staffroll_broad_slot11_hook, "broad_slot11");
staffroll_broad_slot_hook!(staffroll_broad_slot12_hook, "broad_slot12");
staffroll_broad_slot_hook!(staffroll_broad_slot13_hook, "broad_slot13");
staffroll_broad_slot_hook!(staffroll_broad_slot14_hook, "broad_slot14");
staffroll_broad_slot_hook!(staffroll_broad_slot15_hook, "broad_slot15");
staffroll_broad_slot_hook!(staffroll_broad_slot16_hook, "broad_slot16");
staffroll_broad_slot_hook!(staffroll_broad_slot17_hook, "broad_slot17");

fn staffroll_status_slot_hook(registers: *mut Registers, original: usize) -> usize {
    staffroll_probe_hook(
        registers,
        original,
        "status_slot",
        &STAFFROLL_STATUS_SLOT_CALL_COUNT,
        true,
    )
}

fn staffroll_slot_hook(registers: *mut Registers, original: usize) -> usize {
    staffroll_probe_hook(
        registers,
        original,
        "slot",
        &STAFFROLL_SLOT_CALL_COUNT,
        true,
    )
}

fn staffroll_setup_hook(registers: *mut Registers, original: usize) -> usize {
    staffroll_probe_hook(
        registers,
        original,
        "setup",
        &STAFFROLL_SETUP_CALL_COUNT,
        true,
    )
}

fn staffroll_oneshot_hook(registers: *mut Registers, original: usize) -> usize {
    staffroll_probe_hook(
        registers,
        original,
        "oneshot_lambda",
        &STAFFROLL_ONESHOT_CALL_COUNT,
        false,
    )
}

fn staffroll_scene_a_hook(registers: *mut Registers, original: usize) -> usize {
    staffroll_probe_hook(
        registers,
        original,
        "scene_lambda_a",
        &STAFFROLL_SCENE_A_CALL_COUNT,
        false,
    )
}

fn staffroll_scene_b_hook(registers: *mut Registers, original: usize) -> usize {
    staffroll_probe_hook(
        registers,
        original,
        "scene_lambda_b",
        &STAFFROLL_SCENE_B_CALL_COUNT,
        false,
    )
}

fn staffroll_ctor_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = STAFFROLL_CTOR_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;

    append_log(&format!(
        "staffroll ctor probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
        caller_rva(caller)
    ));
    log_staffroll_ctor_fields(count, "before", rcx);

    let original: Probe4Fn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    append_log(&format!(
        "staffroll ctor probe: return #{count} result=0x{result:X}"
    ));
    log_staffroll_ctor_fields(count, "after", rcx);
    result
}

fn staffroll_probe_hook(
    registers: *mut Registers,
    original: usize,
    label: &'static str,
    call_count: &'static AtomicUsize,
    log_staffroll_fields: bool,
) -> usize {
    let registers = unsafe { &*registers };
    let count = call_count.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;

    if count <= 40 {
        append_log(&format!(
            "staffroll screen probe: {label} call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
        if log_staffroll_fields {
            log_staffroll_fields_preview(label, count, "before", rcx);
        } else {
            log_scene_obj_preview(label, count, "arg_rdx", rdx);
        }
    }

    let original: Probe4Fn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if count <= 40 {
        append_log(&format!(
            "staffroll screen probe: {label} return #{count} result=0x{result:X}"
        ));
        if log_staffroll_fields {
            log_staffroll_fields_preview(label, count, "after", rcx);
        } else {
            log_scene_obj_preview(label, count, "arg_rdx_after", rdx);
        }
    }

    result
}

fn movie_ins_init_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_INS_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;

    append_log(&format!(
        "movie ins probe: init call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
        caller_rva(caller)
    ));
    log_movie_ins(count, rcx, "before");

    let original: MovieInsInitFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    append_log(&format!(
        "movie ins probe: init return #{count} result=0x{result:X}"
    ));
    log_movie_ins(count, rcx, "after");
    result
}

fn movie_step_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_STEP_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;
    let target = LAST_MOVIE_PARENT.load(Ordering::Acquire);
    let matches_target =
        target != 0 && (rcx == target || rdx == target || r8 == target || r9 == target);

    if count <= 12 || matches_target {
        log_movie_step(count, "before", caller, rcx, rdx, r8, r9);
    }

    let original: MovieStateStepFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if count <= 12 || matches_target {
        append_log(&format!(
            "movie step probe: return #{count} result=0x{result:X}"
        ));
        log_movie_step(count, "after", caller, rcx, rdx, r8, r9);
    }
    result
}

fn movie_state0_hook(registers: *mut Registers, original: usize) -> usize {
    movie_state_hook("state0", &MOVIE_STATE0_CALL_COUNT, registers, original)
}

fn movie_state1_hook(registers: *mut Registers, original: usize) -> usize {
    movie_state_hook("state1", &MOVIE_STATE1_CALL_COUNT, registers, original)
}

fn movie_resource_ready_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_RESOURCE_READY_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;
    let matches_state1 = caller_main_rva(caller) == Some(ER_MOVIE_STATE1_READY_CALLSITE_RVA);

    let original: Probe4Fn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if matches_state1 || count <= 8 {
        log_movie_resource_ready(count, caller, rcx, rdx, r8, r9, result);
    }
    result
}

fn movie_state_hook(
    label: &str,
    counter: &AtomicUsize,
    registers: *mut Registers,
    original: usize,
) -> usize {
    let registers = unsafe { &*registers };
    let count = counter.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;
    let target = LAST_MOVIE_PARENT.load(Ordering::Acquire);
    let matches_target =
        target != 0 && (rcx == target || rdx == target || r8 == target || r9 == target);

    if count <= 8 || matches_target {
        log_movie_state_fields(label, count, "before", caller, rcx, rdx, r8, r9);
    }

    let original: Probe4Fn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if count <= 8 || matches_target {
        append_log(&format!(
            "movie state probe: {label} return #{count} result=0x{result:X}"
        ));
        log_movie_state_fields(label, count, "after", caller, rcx, rdx, r8, r9);
    }
    result
}

fn movie_tick_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_TICK_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;
    let target = LAST_MOVIE_PARENT.load(Ordering::Acquire);
    let matches_target =
        target != 0 && (rcx == target || rdx == target || r8 == target || r9 == target);

    if count <= 12 || matches_target {
        append_log(&format!(
            "movie tick probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
        log_movie_step(count, "tick-object", caller, rcx, rdx, r8, r9);
    }

    let original: MovieTickFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if count <= 12 || matches_target {
        append_log(&format!(
            "movie tick probe: return #{count} result=0x{result:X}"
        ));
    }
    result
}

fn movie_render_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_RENDER_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;
    let target = LAST_MOVIE_PARENT.load(Ordering::Acquire);
    let matches_target =
        target != 0 && (rcx == target || rdx == target || r8 == target || r9 == target);
    let probe_enabled = MOVIE_RENDER_PROBE_ENABLED.load(Ordering::Acquire) != 0;
    let draw_arg = if rcx != 0 && is_readable_memory(rcx + 0xA8, 8) {
        unsafe { read_usize(rcx + 0xA8) }
    } else {
        0
    };
    let inner = if rcx != 0 && is_readable_memory(rcx + 0xB8, 8) {
        unsafe { read_usize(rcx + 0xB8) }
    } else {
        0
    };

    if probe_enabled && matches_target {
        append_log(&format!(
            "movie render probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
        log_movie_ins(count, rcx, "render");
    }

    let original: MovieRenderFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };
    if matches_target {
        LAST_MOVIE_RENDER_RESULT.store(result, Ordering::Release);
        LAST_MOVIE_DRAW_ARG.store(draw_arg, Ordering::Release);
        LAST_MOVIE_INNER.store(inner, Ordering::Release);
    }

    if probe_enabled && matches_target {
        append_log(&format!(
            "movie render probe: return #{count} result=0x{result:X} tracked_parent=0x{target:X} tracked_draw_arg=0x{draw_arg:X} tracked_inner=0x{inner:X}"
        ));
    }
    result
}

fn movie_draw_submit_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_DRAW_SUBMIT_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;
    let stack_20 = unsafe { registers.get_stack(4) as usize };
    let target = LAST_MOVIE_PARENT.load(Ordering::Acquire);
    let render_result = LAST_MOVIE_RENDER_RESULT.load(Ordering::Acquire);
    let draw_arg = LAST_MOVIE_DRAW_ARG.load(Ordering::Acquire);
    let inner = LAST_MOVIE_INNER.load(Ordering::Acquire);
    if target == 0 && render_result == 0 && draw_arg == 0 && inner == 0 {
        let original: MovieDrawSubmitFn = unsafe { std::mem::transmute(original) };
        return unsafe { original(rcx, rdx, r8, r9) };
    }

    let matches_target = tracked_arg_match(rcx, target, render_result, draw_arg, inner)
        || tracked_arg_match(rdx, target, render_result, draw_arg, inner)
        || tracked_arg_match(r8, target, render_result, draw_arg, inner)
        || tracked_arg_match(r9, target, render_result, draw_arg, inner)
        || tracked_arg_match(stack_20, target, render_result, draw_arg, inner);

    if matches_target {
        append_log(&format!(
            "movie draw submit probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8_movie=0x{r8:X} r9_draw_arg=0x{r9:X} stack20=0x{stack_20:X} tracked parent=0x{target:X} render_result=0x{render_result:X} draw_arg=0x{draw_arg:X} inner=0x{inner:X}",
            caller_rva(caller)
        ));
    }

    let original: MovieDrawSubmitFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if matches_target {
        append_log(&format!(
            "movie draw submit probe: return #{count} result=0x{result:X}"
        ));
    }
    result
}

fn tracked_arg_match(
    arg: usize,
    parent: usize,
    render_result: usize,
    draw_arg: usize,
    inner: usize,
) -> bool {
    (parent != 0 && arg == parent)
        || (render_result != 0 && arg == render_result)
        || (draw_arg != 0 && arg == draw_arg)
        || (inner != 0 && arg == inner)
}

fn start_movie_imp_stop_monitor(movie_ins: usize) {
    if MOVIE_STOP_MONITOR_STARTED.swap(1, Ordering::AcqRel) != 0 {
        return;
    }
    std::thread::spawn(move || {
        let interval =
            Duration::from_millis(MOVIE_STOP_MONITOR_INTERVAL_MS.load(Ordering::Acquire) as u64);
        let grace =
            Duration::from_millis(MOVIE_STOP_MONITOR_GRACE_MS.load(Ordering::Acquire) as u64);
        let mut last_snapshot = TitleGateSnapshot::capture();
        append_log(&format!(
            "movie imp stop monitor: started movie_ins=0x{movie_ins:X} grace={grace:?} {}",
            last_snapshot.summary()
        ));
        std::thread::sleep(grace);
        let grace_snapshot = TitleGateSnapshot::capture();
        append_log(&format!(
            "movie imp stop monitor: watching after grace {}",
            grace_snapshot.summary()
        ));
        let mut armed = grace_snapshot.title_ready();
        if armed {
            append_log("movie imp stop monitor: armed from grace snapshot");
        }

        for _ in 0..18000 {
            std::thread::sleep(interval);
            let snapshot = TitleGateSnapshot::capture();
            if snapshot != last_snapshot {
                append_log(&format!(
                    "movie imp stop monitor: gate changed {}",
                    snapshot.summary()
                ));
                last_snapshot = snapshot;
            }

            if snapshot.world_player {
                stop_title_movie_ins(movie_ins, "world player active");
                return;
            }

            if !armed {
                if snapshot.title_ready() {
                    armed = true;
                    append_log(&format!(
                        "movie imp stop monitor: armed from stable title {}",
                        snapshot.summary()
                    ));
                }
                continue;
            }

            if snapshot.should_stop_after_title() {
                stop_title_movie_ins(movie_ins, "left title menu gate");
                return;
            }
        }

        append_log("movie imp stop monitor: timed out without gate close");
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TitleGateSnapshot {
    title_flow: bool,
    title_step: bool,
    ingame_flow: bool,
    common_flow: bool,
    loading: bool,
    hud_default: bool,
    world_player: bool,
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
            world_player: world_player_active(),
        }
    }

    fn title_ready(&self) -> bool {
        !self.ingame_flow && !self.loading && !self.hud_default && !self.world_player
    }

    fn should_stop_after_title(&self) -> bool {
        self.ingame_flow || self.loading || self.hud_default || self.world_player
    }

    fn summary(&self) -> String {
        format!(
            "title_flow={} title_step={} ingame_flow={} common_flow={} loading={} hud_default={} world_player={}",
            self.title_flow,
            self.title_step,
            self.ingame_flow,
            self.common_flow,
            self.loading,
            self.hud_default,
            self.world_player
        )
    }
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

fn hud_is_default() -> bool {
    unsafe { CSFeManImp::instance() }
        .map(|fe_man| fe_man.hud_state == CSFeManHudState::Default)
        .unwrap_or(false)
}

fn world_player_active() -> bool {
    unsafe { WorldChrMan::instance() }
        .map(|world_chr_man| world_chr_man.main_player.is_some())
        .unwrap_or(false)
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

fn stop_title_movie_ins(movie_ins: usize, reason: &str) {
    if movie_ins == 0 || !is_readable_memory(movie_ins, 0x140) {
        append_log(&format!(
            "movie imp stop monitor: cannot stop unreadable movie_ins=0x{movie_ins:X} reason={reason}"
        ));
        return;
    }

    let inner = unsafe { read_usize(movie_ins + ER_MOVIE_INS_LAYOUT.bink_texture_offset) };
    let active_before = unsafe { read_u8(movie_ins + 0x130) };
    let state_before = unsafe { read_u32(movie_ins + 0x40) };
    let state2_before = unsafe { read_u32(movie_ins + 0x44) };
    append_log(&format!(
        "movie imp stop monitor: stopping title movie_ins=0x{movie_ins:X} inner[+B8]=0x{inner:X} reason={reason} active=0x{active_before:02X} state=0x{state_before:X}/0x{state2_before:X}"
    ));

    let mut inner_closed = false;
    if inner != 0 && is_readable_memory(inner, 0x58) {
        let vtable = unsafe { read_usize(inner) };
        if vtable != 0 && is_readable_memory(vtable + 0x10, 8) {
            let close_fn = unsafe { read_usize(vtable + 0x10) };
            type CloseFn = unsafe extern "system" fn(usize) -> usize;
            let close: CloseFn = unsafe { std::mem::transmute(close_fn) };
            let result = unsafe { close(inner) };
            inner_closed = true;
            append_log(&format!(
                "movie imp stop monitor: title inner close returned 0x{result:X} close=0x{close_fn:X}"
            ));
        }
    }

    unsafe {
        std::ptr::write_volatile(
            (movie_ins + ER_MOVIE_INS_LAYOUT.bink_texture_offset) as *mut usize,
            0,
        );
        std::ptr::write_volatile((movie_ins + 0x130) as *mut u8, 0);
    }

    let mut detached_imp_current = false;
    if let Ok(exe) = unsafe { GetModuleHandleA(None) } {
        let base = exe.0 as usize;
        let global_addr = base + ER_CS_MOVIE_IMP_GLOBAL_RVA;
        if is_readable_memory(global_addr, 8) {
            let imp = unsafe { read_usize(global_addr) };
            if imp != 0 && is_readable_memory(imp + 0x48, 8) {
                let field_40 = unsafe { read_usize(imp + 0x40) };
                if field_40 == movie_ins {
                    unsafe {
                        std::ptr::write_volatile((imp + 0x40) as *mut usize, 0);
                    }
                    detached_imp_current = true;
                }
            }
        }
    }

    LAST_MOVIE_PARENT.store(0, Ordering::Release);
    reset_movie_imp_cycle_after_title_stop(reason);
    let active_after = unsafe { read_u8(movie_ins + 0x130) };
    let state_after = unsafe { read_u32(movie_ins + 0x40) };
    let state2_after = unsafe { read_u32(movie_ins + 0x44) };
    append_log(&format!(
        "movie imp stop monitor: stopped title movie_ins=0x{movie_ins:X} inner_closed={inner_closed} imp_current_detached={detached_imp_current} active {active_before:02X}->{active_after:02X} state 0x{state_before:X}/0x{state2_before:X}->0x{state_after:X}/0x{state2_after:X}"
    ));
}

fn reset_movie_imp_cycle_after_title_stop(reason: &str) {
    MOVIE_STOP_MONITOR_STARTED.store(0, Ordering::Release);
    crate::dx12_title_texture::freeze_bink_bridge_after_title_stop(reason);
}

type MovieOpenWrapperFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

fn bink_texture_open_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = BINK_TEXTURE_OPEN_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;

    if count <= 16 {
        append_log(&format!(
            "bink texture open probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
        append_log(&format!(
            "bink texture open probe: stack #{count} {}",
            stack_preview(registers, 18)
        ));
        append_log(&format!(
            "bink texture open probe: rdx ascii=\"{}\" utf16=\"{}\" hex={}",
            unsafe { read_c_string_preview(rdx, 160) },
            unsafe { read_utf16_preview(rdx, 160) },
            unsafe { read_hex_preview(rdx, 96) }
        ));
        if r8 != 0 {
            append_log(&format!(
                "bink texture open probe: r8 fields [00]=0x{:X} [08]=0x{:X} [0C]={:.3} [10]=0x{:X} [14]=0x{:02X}",
                unsafe { read_usize(r8) },
                unsafe { read_u32(r8 + 0x08) },
                unsafe { read_f32(r8 + 0x0C) },
                unsafe { read_u32(r8 + 0x10) },
                unsafe { read_u8(r8 + 0x14) }
            ));
        } else {
            append_log("bink texture open probe: r8 fields <null>");
        }
        log_bink_texture_object(count, rcx, "before");
        scan_movie_parent_candidates(count, rcx);
    }

    if *BINK_TEXTURE_FORCE_PRESENT_OPTION.get().unwrap_or(&false) && r8 != 0 {
        let before = unsafe { read_u8(r8 + 0x14) };
        unsafe {
            std::ptr::write_volatile((r8 + 0x14) as *mut u8, 1);
        }
        let after = unsafe { read_u8(r8 + 0x14) };
        append_log(&format!(
            "bink texture open probe: forced present option #{count} r8=0x{r8:X} [+14] {before:02X}->{after:02X}"
        ));
    }

    let original: MovieOpenWrapperFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    if result != 0
        && *BINK_TEXTURE_COPY_PRESENT_OPTION_AFTER_OPEN
            .get()
            .unwrap_or(&false)
        && rcx != 0
        && r8 != 0
    {
        let option = unsafe { read_u8(r8 + 0x14) };
        if option != 0 {
            let before = unsafe { read_u8(rcx + 0x53) };
            unsafe {
                std::ptr::write_volatile((rcx + 0x53) as *mut u8, option);
            }
            let after = unsafe { read_u8(rcx + 0x53) };
            append_log(&format!(
                "bink texture open probe: copied present option #{count} object=0x{rcx:X} option[+14]=0x{option:02X} [+53] {before:02X}->{after:02X}"
            ));
        } else {
            append_log(&format!(
                "bink texture open probe: present option not set #{count}; leaving [+53] unchanged"
            ));
        }
    }

    if result != 0 && *BINK_TEXTURE_FORCE_PRESENT_FLAG.get().unwrap_or(&false) && rcx != 0 {
        let before = unsafe { read_u8(rcx + 0x53) };
        unsafe {
            std::ptr::write_volatile((rcx + 0x53) as *mut u8, 1);
        }
        let after = unsafe { read_u8(rcx + 0x53) };
        append_log(&format!(
            "bink texture open probe: forced present flag #{count} object=0x{rcx:X} [+53] {before:02X}->{after:02X}"
        ));
    }

    if count <= 16 {
        append_log(&format!(
            "bink texture open probe: return #{count} result=0x{result:X}"
        ));
        log_bink_texture_object(count, rcx, "after");
    }
    result
}

fn movie_wrapper_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let count = MOVIE_WRAPPER_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let caller = unsafe { registers.get_stack(0) as usize };
    let rcx = registers.rcx as usize;
    let rdx = registers.rdx as usize;
    let r8 = registers.r8 as usize;
    let r9 = registers.r9 as usize;

    append_log(&format!(
        "movie wrapper probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{rcx:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
        caller_rva(caller)
    ));
    append_log(&format!(
        "movie wrapper probe: path #{count} rdx_ascii=\"{}\"",
        unsafe { read_c_string_preview(rdx, 260) }
    ));
    append_log(&format!(
        "movie wrapper probe: path #{count} rdx_utf16=\"{}\"",
        unsafe { read_utf16_preview(rdx, 260) }
    ));
    append_log(&format!(
        "movie wrapper probe: path #{count} rdx_hex={}",
        unsafe { read_hex_preview(rdx, 96) }
    ));
    log_movie_object(count, rcx);
    log_movie_options(count, r8);

    let original: MovieOpenWrapperFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(rcx, rdx, r8, r9) };

    append_log(&format!(
        "movie wrapper probe: return #{count} result=0x{result:X}"
    ));
    log_movie_object(count, rcx);
    result
}

fn log_bink_texture_object(count: usize, ptr: usize, phase: &str) {
    if ptr == 0 {
        append_log(&format!(
            "bink texture open probe: object {phase} #{count} rcx=<null>"
        ));
        return;
    }

    let vtable = unsafe { read_usize(ptr) };
    let field_08 = unsafe { read_usize(ptr + 0x08) };
    let field_28 = unsafe { read_usize(ptr + 0x28) };
    let bink = unsafe { read_usize(ptr + 0x40) };
    let field_48 = unsafe { read_usize(ptr + 0x48) };
    let flags_50 = unsafe { read_u8(ptr + 0x50) };
    let flags_51 = unsafe { read_u8(ptr + 0x51) };
    let flags_52 = unsafe { read_u8(ptr + 0x52) };
    let flags_53 = unsafe { read_u8(ptr + 0x53) };

    append_log(&format!(
        "bink texture open probe: object {phase} #{count} rcx=0x{ptr:X} vtable={} [08]=0x{field_08:X} [28]=0x{field_28:X} bink[40]=0x{bink:X} [48]=0x{field_48:X} flags[50..53]={flags_50:02X},{flags_51:02X},{flags_52:02X},{flags_53:02X}",
        caller_rva(vtable)
    ));
}

fn scan_movie_parent_candidates(count: usize, inner: usize) {
    if inner == 0 {
        return;
    }

    let start = inner.saturating_sub(0x1000_0000).max(0x10000);
    let end = inner.saturating_add(0x1000_0000);
    let mut addr = start;
    let mut found = 0usize;

    append_log(&format!(
        "bink texture open probe: scanning parents #{count} inner=0x{inner:X} range=0x{start:X}..0x{end:X}"
    ));

    while addr < end && found < 16 {
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        let queried = unsafe {
            VirtualQuery(
                Some(addr as *const _),
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if queried == 0 {
            break;
        }

        let region_start = mbi.BaseAddress as usize;
        let region_end = region_start.saturating_add(mbi.RegionSize);
        addr = region_end.max(addr.saturating_add(0x1000));

        if mbi.State != MEM_COMMIT || !is_readable_protect(mbi.Protect) {
            continue;
        }

        let scan_start = region_start.max(start);
        let scan_end = region_end.min(end);
        if scan_end <= scan_start || scan_end - scan_start < 8 {
            continue;
        }

        let mut p = scan_start;
        while p + 8 <= scan_end && found < 16 {
            let value = unsafe { std::ptr::read_unaligned(p as *const usize) };
            if value == inner {
                for object_offset in [0xB8usize, 0xC0usize] {
                    if p >= object_offset {
                        let object = p - object_offset;
                        if is_readable_memory(object, 0x150) {
                            found += 1;
                            log_parent_movie_candidate(count, found, object, object_offset, p);
                        }
                    }
                }
            }
            p = p.saturating_add(8);
        }
    }

    append_log(&format!(
        "bink texture open probe: parent scan #{count} found={found}"
    ));
}

fn log_parent_movie_candidate(
    count: usize,
    found: usize,
    object: usize,
    object_offset: usize,
    hit: usize,
) {
    let vtable = unsafe { read_usize(object) };
    let state = unsafe { read_u32(object + 0x40) };
    let field_48 = unsafe { read_u32(object + 0x48) };
    let inner_b8 = unsafe { read_usize(object + 0xB8) };
    let inner_c0 = unsafe { read_usize(object + 0xC0) };
    let er_volume = unsafe { read_f32(object + 0xF0) };
    let er_present = unsafe { read_u8(object + 0xF4) };
    let er_mode = unsafe { read_u32(object + 0xF8) };
    let nr_volume = unsafe { read_f32(object + 0xF8) };
    let nr_present = unsafe { read_u8(object + 0xFC) };
    let nr_mode = unsafe { read_u32(object + 0x100) };
    let active_130 = unsafe { read_u8(object + 0x130) };
    let active_138 = unsafe { read_u8(object + 0x138) };
    if is_main_module_pointer(vtable) && LAST_MOVIE_PARENT.load(Ordering::Acquire) == 0 {
        LAST_MOVIE_PARENT.store(object, Ordering::Release);
        append_log(&format!(
            "bink texture open probe: selected movie parent object=0x{object:X} via +0x{object_offset:X}"
        ));
    }

    append_log(&format!(
        "bink texture open probe: parent #{count}.{found} hit=0x{hit:X} object=0x{object:X} via +0x{object_offset:X} vtbl={} state[+40]=0x{state:X} [+48]=0x{field_48:X} inner[+B8]=0x{inner_b8:X} inner[+C0]=0x{inner_c0:X} ER[+F0]={er_volume:.3} [+F4]=0x{er_present:02X} [+F8]=0x{er_mode:X} NR[+F8]={nr_volume:.3} [+FC]=0x{nr_present:02X} [+100]=0x{nr_mode:X} active[+130]=0x{active_130:02X} [+138]=0x{active_138:02X}",
        caller_rva(vtable)
    ));
}

fn is_readable_memory(addr: usize, size: usize) -> bool {
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
    if queried == 0 || mbi.State != MEM_COMMIT || !is_readable_protect(mbi.Protect) {
        return false;
    }
    let region_start = mbi.BaseAddress as usize;
    let region_end = region_start.saturating_add(mbi.RegionSize);
    addr.checked_add(size).is_some_and(|end| end <= region_end)
}

fn is_main_module_pointer(ptr: usize) -> bool {
    let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
        return false;
    };
    let module = HMODULE(exe.0);
    let base = exe.0 as usize;
    let mut buf = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buf) } as usize;
    if len == 0 {
        return ptr >= base;
    }
    ptr >= base
}

fn log_staffroll_fields_preview(label: &str, count: usize, phase: &str, object: usize) {
    if object == 0 {
        append_log(&format!(
            "staffroll screen probe: {label} {phase} #{count} object=<null>"
        ));
        return;
    }

    append_log(&format!(
        "staffroll screen probe: {label} {phase} #{count} object=0x{object:X} vtbl={} [+188]={} [+198]={} [+5A0]={} [cf0.count]={} [cf0.gen]={} [d50.count]={} [d50.gen]={} [+E00]={} [+EA8]={} [+EBC]={} [+ECC]={}",
        fmt_ptr_field(object, 0x00),
        fmt_ptr_field(object, 0x188),
        fmt_ptr_field(object, 0x198),
        fmt_u8_field(object, 0x5A0),
        fmt_u32_field(object, 0xCF0 + 0x50),
        fmt_u32_field(object, 0xCF0 + 0x58),
        fmt_u32_field(object, 0xD50 + 0x50),
        fmt_u32_field(object, 0xD50 + 0x58),
        fmt_ptr_field(object, 0xE00),
        fmt_u32_field(object, 0xEA8),
        fmt_ptr_field(object, 0xEBC),
        fmt_i32_field(object, 0xECC),
    ));
}

fn log_staffroll_ctor_fields(count: usize, phase: &str, object: usize) {
    if object == 0 {
        append_log(&format!(
            "staffroll ctor probe: {phase} #{count} object=<null>"
        ));
        return;
    }

    append_log(&format!(
        "staffroll ctor probe: {phase} #{count} object=0x{object:X} vtbl={} common[+5A8]={} er[+A38]={} er[+A50]={} er[+A60]={} nr[+DE8]={} nr[+E00]={} nr[+E48]={} nr[+EA8]={} nr[+EB0]={} nr[+EBC]={} nr[+ECC]={}",
        fmt_ptr_field(object, 0x00),
        fmt_ptr_field(object, 0x5A8),
        fmt_ptr_field(object, 0xA38),
        fmt_ptr_field(object, 0xA50),
        fmt_ptr_field(object, 0xA60),
        fmt_ptr_field(object, 0xDE8),
        fmt_ptr_field(object, 0xE00),
        fmt_ptr_field(object, 0xE48),
        fmt_u32_field(object, 0xEA8),
        fmt_ptr_field(object, 0xEB0),
        fmt_ptr_field(object, 0xEBC),
        fmt_i32_field(object, 0xECC),
    ));
}

fn log_scene_obj_preview(label: &str, count: usize, phase: &str, object: usize) {
    if object == 0 {
        append_log(&format!(
            "staffroll screen probe: {label} {phase} #{count} object=<null>"
        ));
        return;
    }

    append_log(&format!(
        "staffroll screen probe: {label} {phase} #{count} object=0x{object:X} vtbl={} [+08]={} [+10]={} [+20]={} [+38]={} ascii[+00]=\"{}\"",
        fmt_ptr_field(object, 0x00),
        fmt_ptr_field(object, 0x08),
        fmt_ptr_field(object, 0x10),
        fmt_u32_field(object, 0x20),
        fmt_f32_field(object, 0x38),
        if is_readable_memory(object, 48) {
            unsafe { read_ascii_preview(object, 48) }
        } else {
            "<unreadable>".to_string()
        },
    ));
}

fn fmt_ptr_field(base: usize, offset: usize) -> String {
    let addr = base.saturating_add(offset);
    if !is_readable_memory(addr, 8) {
        return "<unreadable>".to_string();
    }
    let value = unsafe { read_usize(addr) };
    format!("0x{value:X}({})", caller_rva(value))
}

fn fmt_u32_field(base: usize, offset: usize) -> String {
    let addr = base.saturating_add(offset);
    if !is_readable_memory(addr, 4) {
        return "<unreadable>".to_string();
    }
    let value = unsafe { read_u32(addr) };
    format!("0x{value:X}")
}

fn fmt_i32_field(base: usize, offset: usize) -> String {
    let addr = base.saturating_add(offset);
    if !is_readable_memory(addr, 4) {
        return "<unreadable>".to_string();
    }
    let value = unsafe { read_u32(addr) as i32 };
    format!("{value}")
}

fn fmt_u8_field(base: usize, offset: usize) -> String {
    let addr = base.saturating_add(offset);
    if !is_readable_memory(addr, 1) {
        return "<unreadable>".to_string();
    }
    let value = unsafe { read_u8(addr) };
    format!("0x{value:02X}")
}

fn fmt_f32_field(base: usize, offset: usize) -> String {
    let addr = base.saturating_add(offset);
    if !is_readable_memory(addr, 4) {
        return "<unreadable>".to_string();
    }
    let value = unsafe { read_f32(addr) };
    if value.is_finite() {
        format!("{value:.3}")
    } else {
        "<nan>".to_string()
    }
}

fn is_readable_protect(protect: windows::Win32::System::Memory::PAGE_PROTECTION_FLAGS) -> bool {
    protect == PAGE_READONLY
        || protect == PAGE_READWRITE
        || protect == PAGE_WRITECOPY
        || protect == PAGE_EXECUTE
        || protect == PAGE_EXECUTE_READ
        || protect == PAGE_EXECUTE_READWRITE
        || protect == PAGE_EXECUTE_WRITECOPY
}

fn log_movie_step(
    count: usize,
    phase: &str,
    caller: usize,
    ptr: usize,
    rdx: usize,
    r8: usize,
    r9: usize,
) {
    if ptr == 0 {
        append_log(&format!(
            "movie step probe: {phase} #{count} caller=0x{caller:X} caller_rva={} rcx=<null> rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
        return;
    }

    let vtable = unsafe { read_usize(ptr) };
    let state_table = unsafe { read_usize(ptr + 0x08) };
    let current_state = unsafe { read_u32(ptr + 0x40) };
    let next_state = unsafe { read_u32(ptr + 0x44) };
    let repeat = unsafe { read_u8(ptr + 0x48) };
    let movie_ins_vtable = unsafe { read_usize(ptr + 0x90) };
    let last_entry = unsafe { read_usize(ptr + 0x98) };
    let mut state_slots = String::new();
    if state_table != 0 && is_readable_memory(state_table, 0x60) {
        for state in 0..6usize {
            let primary = unsafe { read_usize(state_table + state * 0x10) };
            let secondary = unsafe { read_usize(state_table + state * 0x10 + 0x08) };
            state_slots.push_str(&format!(
                " s{state}=0x{primary:X}({})/0x{secondary:X}({})",
                caller_rva(primary),
                caller_rva(secondary)
            ));
        }
    } else {
        state_slots.push_str(" <state-table-unreadable>");
    }

    append_log(&format!(
        "movie step probe: {phase} #{count} caller=0x{caller:X} caller_rva={} rcx=0x{ptr:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X} vtable=0x{vtable:X} state_table[+8]=0x{state_table:X} state[40/44]={current_state}/{next_state} repeat[48]=0x{repeat:02X} movie_ins_vtable?[90]=0x{movie_ins_vtable:X} last_entry[98]=0x{last_entry:X} state_slots:{state_slots}",
        caller_rva(caller)
    ));
}

fn log_movie_state_fields(
    label: &str,
    count: usize,
    phase: &str,
    caller: usize,
    ptr: usize,
    rdx: usize,
    r8: usize,
    r9: usize,
) {
    if ptr == 0 {
        append_log(&format!(
            "movie state probe: {label} {phase} #{count} caller=0x{caller:X} caller_rva={} rcx=<null> rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X}",
            caller_rva(caller)
        ));
        return;
    }

    let vtable = unsafe { read_usize(ptr) };
    let state_table = unsafe { read_usize(ptr + 0x08) };
    let current_state = unsafe { read_u32(ptr + 0x40) };
    let next_state = unsafe { read_u32(ptr + 0x44) };
    let repeat = unsafe { read_u8(ptr + 0x48) };
    let bink_texture = unsafe { read_usize(ptr + ER_MOVIE_INS_LAYOUT.bink_texture_offset) };
    let volume = unsafe { read_f32(ptr + ER_MOVIE_INS_LAYOUT.volume_offset) };
    let present = unsafe { read_u8(ptr + ER_MOVIE_INS_LAYOUT.present_offset) };
    let option = unsafe { read_u32(ptr + ER_MOVIE_INS_LAYOUT.option_offset) };
    let flag_130 = unsafe { read_u16(ptr + 0x130) };
    let flag_132 = unsafe { read_u8(ptr + 0x132) };
    let flag_133 = unsafe { read_u8(ptr + 0x133) };
    let flag_134 = unsafe { read_u8(ptr + 0x134) };

    append_log(&format!(
        "movie state probe: {label} {phase} #{count} caller=0x{caller:X} caller_rva={} rcx=0x{ptr:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X} vtable=0x{vtable:X} state_table=0x{state_table:X} state[40/44]={current_state}/{next_state} repeat[48]=0x{repeat:02X} bink[+B8]=0x{bink_texture:X} volume=0x{volume:.3} present=0x{present:02X} option=0x{option:X} flags[130..134]={flag_130:04X},{flag_132:02X},{flag_133:02X},{flag_134:02X}",
        caller_rva(caller)
    ));
}

fn log_movie_resource_ready(
    count: usize,
    caller: usize,
    ptr: usize,
    rdx: usize,
    r8: usize,
    r9: usize,
    result: usize,
) {
    if ptr == 0 || !is_readable_memory(ptr, 0x20) {
        append_log(&format!(
            "movie resource ready probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{ptr:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X} result=0x{result:X} low=0x{:02X} object=<unreadable>",
            caller_rva(caller),
            result & 0xFF
        ));
        return;
    }

    let field_00 = unsafe { read_usize(ptr) };
    let field_08 = unsafe { read_usize(ptr + 0x08) };
    let field_10 = unsafe { read_usize(ptr + 0x10) };
    let field_18 = unsafe { read_u32(ptr + 0x18) };
    let field_08_preview = readable_ptr_preview(field_08, 48);
    let field_10_preview = readable_ptr_preview(field_10, 48);

    append_log(&format!(
        "movie resource ready probe: call #{count} caller=0x{caller:X} caller_rva={} rcx=0x{ptr:X} rdx=0x{rdx:X} r8=0x{r8:X} r9=0x{r9:X} result=0x{result:X} low=0x{:02X} fields [00]=0x{field_00:X} [08]=0x{field_08:X} [10]=0x{field_10:X} [18]=0x{field_18:X} object_hex={} [08]_hex={} [10]_hex={}",
        caller_rva(caller),
        result & 0xFF,
        unsafe { read_hex_preview(ptr, 48) },
        field_08_preview,
        field_10_preview
    ));
}

fn readable_ptr_preview(ptr: usize, bytes: usize) -> String {
    if ptr != 0 && is_readable_memory(ptr, 1) {
        unsafe { read_hex_preview(ptr, bytes) }
    } else {
        "<unreadable>".to_string()
    }
}

fn log_movie_object(count: usize, ptr: usize) {
    if ptr == 0 {
        append_log(&format!("movie wrapper probe: object #{count} rcx=<null>"));
        return;
    }

    let field_08 = unsafe { read_usize(ptr + 0x08) };
    let field_10 = unsafe { read_usize(ptr + 0x10) };
    let field_18 = unsafe { read_u32(ptr + 0x18) };
    let field_40 = unsafe { read_usize(ptr + 0x40) };
    let field_50 = unsafe { read_u8(ptr + 0x50) };
    let field_51 = unsafe { read_u8(ptr + 0x51) };
    let field_52 = unsafe { read_u8(ptr + 0x52) };
    let field_53 = unsafe { read_u8(ptr + 0x53) };

    append_log(&format!(
        "movie wrapper probe: object #{count} [08]=0x{field_08:X} [10]=0x{field_10:X} [18]=0x{field_18:X} [40]=0x{field_40:X} [50..53]={field_50:02X},{field_51:02X},{field_52:02X},{field_53:02X}"
    ));
}

fn log_movie_options(count: usize, ptr: usize) {
    if ptr == 0 {
        append_log(&format!("movie wrapper probe: options #{count} r8=<null>"));
        return;
    }

    let field_00 = unsafe { read_usize(ptr) };
    let field_08 = unsafe { read_u32(ptr + 0x08) };
    let field_0c = unsafe { read_f32(ptr + 0x0C) };
    let field_10 = unsafe { read_u32(ptr + 0x10) };
    let field_14 = unsafe { read_u8(ptr + 0x14) };

    append_log(&format!(
        "movie wrapper probe: options #{count} [00]=0x{field_00:X} [08]=0x{field_08:X} [0C]={field_0c:.3} [10]=0x{field_10:X} [14]=0x{field_14:02X}"
    ));
}

fn log_movie_ins(count: usize, ptr: usize, phase: &str) {
    if ptr == 0 {
        append_log(&format!("movie ins probe: {phase} #{count} rcx=<null>"));
        return;
    }

    let layout = *MOVIE_INS_LAYOUT.get().unwrap_or(&ER_MOVIE_INS_LAYOUT);
    let vtable = unsafe { read_usize(ptr) };
    let draw_arg = unsafe { read_usize(ptr + 0xA8) };
    let movie_obj = unsafe { read_usize(ptr + layout.bink_texture_offset) };
    let volume = unsafe { read_f32(ptr + layout.volume_offset) };
    let present = unsafe { read_u8(ptr + layout.present_offset) };
    let option = unsafe { read_u32(ptr + layout.option_offset) };
    let state_130 = unsafe { read_u16(ptr + 0x130) };
    let state_131 = unsafe { read_u8(ptr + 0x131) };
    let state_132 = unsafe { read_u8(ptr + 0x132) };
    let state_133 = unsafe { read_u8(ptr + 0x133) };
    let state_134 = unsafe { read_u8(ptr + 0x134) };
    let aux_5d8 = unsafe { read_usize(ptr + 0x5D8) };
    let path_ptr = ptr + layout.path_offset;
    let title_raw = unsafe { read_ascii_preview(path_ptr, 128) };
    let title_utf16 = unsafe { read_utf16_preview(path_ptr, 128) };
    let title_hex = unsafe { read_hex_preview(path_ptr, 96) };
    let (fd4_data, fd4_len, fd4_cap, fd4_text, fd4_hex) =
        unsafe { read_fd4_wstring_preview(path_ptr, 260) };

    append_log(&format!(
        "movie ins probe: {phase} #{count} layout={} vtable=0x{vtable:X} draw_arg[+A8]=0x{draw_arg:X} bink_texture[+{:X}]=0x{movie_obj:X} volume[+{:X}]={volume:.3} present[+{:X}]=0x{present:02X} option[+{:X}]=0x{option:X} state[+130..134]={state_130:04X},{state_131:02X},{state_132:02X},{state_133:02X},{state_134:02X} aux[+5D8]=0x{aux_5d8:X}",
        layout.name,
        layout.bink_texture_offset,
        layout.volume_offset,
        layout.present_offset,
        layout.option_offset
    ));
    append_log(&format!(
        "movie ins probe: {phase} #{count} path[+{:X} ascii]=\"{title_raw}\"",
        layout.path_offset
    ));
    append_log(&format!(
        "movie ins probe: {phase} #{count} path[+{:X} utf16]=\"{title_utf16}\"",
        layout.path_offset
    ));
    append_log(&format!(
        "movie ins probe: {phase} #{count} path[+{:X} fd4_wstr] data=0x{fd4_data:X} len={fd4_len} cap={fd4_cap} text=\"{fd4_text}\" hex={fd4_hex}",
        layout.path_offset
    ));
    append_log(&format!(
        "movie ins probe: {phase} #{count} path[+{:X} hex]={title_hex}",
        layout.path_offset
    ));
    log_movie_imp_owner(count, ptr, phase, layout);

    if phase == "after"
        && (is_probable_title_movie_marker(&title_raw)
            || is_probable_title_movie_marker(&title_utf16)
            || is_probable_title_movie_marker(&fd4_text))
    {
        let previous =
            LAST_MOVIE_PARENT.compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Acquire);
        if previous.is_ok() {
            append_log(&format!(
                "movie ins probe: selected movie parent object=0x{ptr:X} from marker \"{title_raw}\""
            ));
        }
    }
}

fn log_movie_imp_owner(count: usize, movie_ins: usize, phase: &str, layout: MovieInsLayout) {
    let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
        append_log(&format!(
            "movie ins probe: {phase} #{count} movie_imp layout={} base=<unavailable>",
            layout.name
        ));
        return;
    };

    let base = exe.0 as usize;
    let global_rva = if layout.name == "NR" {
        NR_CS_MOVIE_IMP_GLOBAL_RVA
    } else {
        ER_CS_MOVIE_IMP_GLOBAL_RVA
    };
    let global_addr = base + global_rva;
    let imp = unsafe { read_usize(global_addr) };
    if imp == 0 {
        append_log(&format!(
            "movie ins probe: {phase} #{count} movie_imp layout={} global[main.exe+0x{global_rva:X}]=<null>",
            layout.name
        ));
        return;
    }

    let imp_vtable = unsafe { read_usize(imp) };
    let imp_movie_ins = unsafe { read_usize(imp + CS_MOVIE_IMP_MOVIE_INS_OFFSET) };
    let field_40 = unsafe { read_usize(imp + 0x40) };
    let field_48 = unsafe { read_usize(imp + 0x48) };
    let field_50 = unsafe { read_u32(imp + 0x50) };
    let field_54 = unsafe { read_u32(imp + 0x54) };
    let relation = if imp_movie_ins == movie_ins {
        "matches-current"
    } else {
        "different"
    };

    append_log(&format!(
        "movie ins probe: {phase} #{count} movie_imp layout={} global[main.exe+0x{global_rva:X}]=0x{imp:X} vtable=0x{imp_vtable:X} [+38]=0x{imp_movie_ins:X} relation={relation} [+40]=0x{field_40:X} [+48]=0x{field_48:X} [+50]=0x{field_50:X} [+54]=0x{field_54:X}",
        layout.name
    ));
}

fn is_probable_title_movie_marker(text: &str) -> bool {
    text.contains("MENU_DummyMovie")
        || text.contains("_01_920_Movie")
        || text.contains("05_001_Title_Logo")
        || text.contains("_05_001_Title_Logo")
        || text.contains("00001010.bk2")
        || text.contains("movie:/00001010")
}

fn bink_open_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let name_ptr = registers.rcx as *const i8;
    let flags = registers.rdx as u32;
    let caller = unsafe { registers.get_stack(0) as usize };
    let caller_rva = caller_rva(caller);
    let name = unsafe { read_bink_name(name_ptr) };
    let replacement = replacement_for(&name);
    let count = BINK_OPEN_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

    append_log(&format!(
        "bink probe: BinkOpen call #{count} name=\"{name}\" flags=0x{flags:X} caller=0x{caller:X} caller_rva={caller_rva}"
    ));
    if count <= 16 {
        append_log(&format!(
            "bink probe: BinkOpen stack #{count} {}",
            stack_preview(registers, 8)
        ));
    }

    let original: BinkOpenFn = unsafe { std::mem::transmute(original) };
    let (call_ptr, _replacement_storage) = if let Some(replacement) = replacement {
        append_log(&format!(
            "bink probe: replacing BinkOpen #{count} with \"{}\"",
            replacement.display()
        ));
        match CString::new(replacement.to_string_lossy().as_bytes()) {
            Ok(path) => (path.as_ptr(), Some(path)),
            Err(err) => {
                append_log(&format!(
                    "bink probe: replacement path contains nul byte, using original: {err:?}"
                ));
                (name_ptr, None)
            }
        }
    } else {
        (name_ptr, None)
    };
    let result = unsafe { original(call_ptr, flags) as usize };

    append_log(&format!(
        "bink probe: BinkOpen return #{count} result=0x{result:X}"
    ));
    result
}

fn replacement_for(name: &str) -> Option<PathBuf> {
    let rule = REPLACE_RULE.get()?;
    if name
        .to_ascii_lowercase()
        .contains(&rule.from_contains.to_ascii_lowercase())
    {
        Some(rule.to_path.clone())
    } else {
        None
    }
}

fn caller_rva(caller: usize) -> String {
    let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
        return "unavailable".to_string();
    };
    let base = exe.0 as usize;
    if caller >= base {
        format!("main.exe+0x{:X}", caller - base)
    } else {
        "outside-main-module".to_string()
    }
}

fn caller_main_rva(caller: usize) -> Option<usize> {
    let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
        return None;
    };
    let base = exe.0 as usize;
    if caller >= base {
        Some(caller - base)
    } else {
        None
    }
}

fn stack_preview(registers: &Registers, depth: usize) -> String {
    (0..depth)
        .map(|index| {
            let addr = unsafe { registers.get_stack(index) as usize };
            format!("[{index}]=0x{addr:X}({})", caller_rva(addr))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main_module_name(module: HMODULE) -> String {
    let mut buffer = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buffer) };
    if len == 0 {
        return "<unknown>".to_string();
    }
    buffer.truncate(len as usize);
    String::from_utf16_lossy(&buffer).to_ascii_lowercase()
}

unsafe fn read_bink_name(ptr: *const i8) -> String {
    if ptr.is_null() {
        return "<null>".to_string();
    }

    let mut bytes = Vec::new();
    for i in 0..1024usize {
        let b = unsafe { *ptr.add(i) as u8 };
        if b == 0 {
            break;
        }
        bytes.push(b);
    }
    if bytes.is_empty() {
        return "<empty>".to_string();
    }

    match String::from_utf8(bytes.clone()) {
        Ok(text) if text.chars().all(|c| !c.is_control() || c == '\t') => text,
        _ => bytes
            .iter()
            .map(|b| {
                if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                }
            })
            .collect(),
    }
}

unsafe fn read_usize(addr: usize) -> usize {
    unsafe { std::ptr::read_unaligned(addr as *const usize) }
}

unsafe fn read_u32(addr: usize) -> u32 {
    unsafe { std::ptr::read_unaligned(addr as *const u32) }
}

unsafe fn read_u16(addr: usize) -> u16 {
    unsafe { std::ptr::read_unaligned(addr as *const u16) }
}

unsafe fn read_u8(addr: usize) -> u8 {
    unsafe { std::ptr::read_unaligned(addr as *const u8) }
}

unsafe fn read_f32(addr: usize) -> f32 {
    unsafe { std::ptr::read_unaligned(addr as *const f32) }
}

unsafe fn read_ascii_preview(addr: usize, len: usize) -> String {
    let mut text = String::new();
    for i in 0..len {
        let byte = unsafe { read_u8(addr + i) };
        if byte == 0 {
            text.push('.');
        } else if byte.is_ascii_graphic() || byte == b' ' {
            text.push(byte as char);
        } else {
            text.push('.');
        }
    }
    text
}

unsafe fn read_c_string_preview(addr: usize, len: usize) -> String {
    if addr == 0 {
        return "<null>".to_string();
    }

    let mut text = String::new();
    for i in 0..len {
        let byte = unsafe { read_u8(addr + i) };
        if byte == 0 {
            break;
        }
        if byte.is_ascii_graphic() || byte == b' ' || byte == b'\\' || byte == b':' {
            text.push(byte as char);
        } else {
            text.push('.');
        }
    }
    if text.is_empty() {
        "<empty>".to_string()
    } else {
        text
    }
}

unsafe fn read_utf16_preview(addr: usize, len: usize) -> String {
    if addr == 0 {
        return "<null>".to_string();
    }

    let mut units = Vec::new();
    for i in 0..len {
        let unit = unsafe { read_u16(addr + i * 2) };
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    if units.is_empty() {
        "<empty>".to_string()
    } else {
        String::from_utf16_lossy(&units)
    }
}

unsafe fn read_fd4_wstring_preview(
    addr: usize,
    max_len: usize,
) -> (usize, usize, usize, String, String) {
    if addr == 0 || !is_readable_memory(addr, 0x28) {
        return (0, 0, 0, "<unreadable>".to_string(), String::new());
    }

    let len = unsafe { read_usize(addr + 0x18) };
    let cap = unsafe { read_usize(addr + 0x20) };
    let data = if cap >= 8 {
        unsafe { read_usize(addr + 0x08) }
    } else {
        addr + 0x08
    };

    if len == 0 {
        return (data, len, cap, "<empty>".to_string(), String::new());
    }

    let capped_len = len.min(max_len);
    let byte_len = capped_len.saturating_mul(2);
    if data == 0 || !is_readable_memory(data, byte_len.min(0x1000)) {
        return (
            data,
            len,
            cap,
            "<unreadable-data>".to_string(),
            String::new(),
        );
    }

    let text = unsafe { read_utf16_preview(data, capped_len) };
    let hex = unsafe { read_hex_preview(data, byte_len.min(96)) };
    (data, len, cap, text, hex)
}

unsafe fn read_hex_preview(addr: usize, len: usize) -> String {
    let mut text = String::new();
    for i in 0..len {
        if i > 0 {
            text.push(' ');
        }
        let byte = unsafe { read_u8(addr + i) };
        text.push_str(&format!("{byte:02X}"));
    }
    text
}

fn append_log(message: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}
