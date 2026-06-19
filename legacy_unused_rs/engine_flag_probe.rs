use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use fromsoftware_shared::program::Program;
use ilhook::x64::{CallbackOption, HookFlags, Registers, hook_closure_retn};
use pelite::pe64::Pe;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static FLAG_SEEN_MASK: AtomicUsize = AtomicUsize::new(0);
static FLAG_TRUE_MASK: AtomicUsize = AtomicUsize::new(0);

type EngineFlagFn = unsafe extern "system" fn(*const u8) -> bool;

const ER_ENGINE_FLAG_PATTERN: &[pelite::pattern::Atom] =
    pelite::pattern!("' 48 0F BE 01 48 8D 0D ? ? ? ? 48 FF 24 C1");

pub(crate) fn install(log_path: Option<PathBuf>) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    if HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2));
        let Some(addr) = find_engine_flag_fn() else {
            append_log("engine flag probe: pattern not found");
            return;
        };
        append_log(&format!("engine flag probe: hooking addr=0x{addr:X}"));
        match unsafe {
            hook_closure_retn(
                addr,
                |registers, original| engine_flag_hook(registers, original),
                CallbackOption::None,
                HookFlags::empty(),
            )
        } {
            Ok(hook) => {
                let _ = Box::leak(Box::new(hook));
                HOOK_INSTALLED.store(1, Ordering::Release);
                append_log("engine flag probe: hook installed");
                start_snapshot_logger();
            }
            Err(err) => {
                append_log(&format!("engine flag probe: hook failed: {err:?}"));
            }
        }
    });
}

fn start_snapshot_logger() {
    std::thread::spawn(|| {
        for second in 1..=120 {
            std::thread::sleep(Duration::from_secs(1));
            let seen = FLAG_SEEN_MASK.load(Ordering::Relaxed);
            let truth = FLAG_TRUE_MASK.load(Ordering::Relaxed);
            append_log(&format!(
                "engine flag probe: snapshot t={second}s seen=0x{seen:X} true=0x{truth:X} f1={} f2={} f3={} f4={} f5={} f6={}",
                bit(truth, 1),
                bit(truth, 2),
                bit(truth, 3),
                bit(truth, 4),
                bit(truth, 5),
                bit(truth, 6),
            ));
        }
    });
}

fn bit(mask: usize, bit: usize) -> bool {
    mask & (1usize << bit) != 0
}

fn find_engine_flag_fn() -> Option<usize> {
    let program = Program::current();
    let mut matches = program.scanner().matches_code(ER_ENGINE_FLAG_PATTERN);
    let mut save = [0; 1];
    if !matches.next(&mut save) {
        return None;
    }
    let rva = save[0];
    let va = program.rva_to_va(rva).ok()? as usize;
    Some(va)
}

fn engine_flag_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let flag_ptr = registers.rcx as *const u8;
    let flag = unsafe { flag_ptr.read() };
    let original: EngineFlagFn = unsafe { std::mem::transmute(original) };
    let result = unsafe { original(flag_ptr) };

    let count = CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if flag < usize::BITS as u8 {
        let bit = 1usize << flag;
        let seen_before = FLAG_SEEN_MASK.fetch_or(bit, Ordering::Relaxed);
        let true_before = FLAG_TRUE_MASK.load(Ordering::Relaxed);
        let was_true = true_before & bit != 0;
        if result {
            FLAG_TRUE_MASK.fetch_or(bit, Ordering::Relaxed);
        } else {
            FLAG_TRUE_MASK.fetch_and(!bit, Ordering::Relaxed);
        }
        let first_seen = seen_before & bit == 0;
        let changed = was_true != result;
        if first_seen || changed || count <= 16 {
            let caller = unsafe { registers.get_stack(0) as usize };
            append_log(&format!(
                "engine flag probe: call #{count} flag={flag} result={result} first_seen={first_seen} changed={changed} ptr=0x{:X} caller=0x{caller:X} caller_rva={}",
                flag_ptr as usize,
                caller_rva(caller)
            ));
        }
    }

    result as usize
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

fn append_log(message: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}
