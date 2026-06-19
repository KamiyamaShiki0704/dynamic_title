use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use windows::Win32::System::LibraryLoader::GetModuleHandleA;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

const OUTER_MOVIE_SINGLETON_PTR_RVA: usize = 0x45878A8;
const MOVIE_START_RVA: usize = 0xE20F90;

type MovieStartFn = unsafe extern "system" fn(usize, u32, *const u16, f32, u32, u32, u32) -> u8;

pub(crate) fn trigger_once_after_delay(
    log_path: Option<PathBuf>,
    path: String,
    delay: Duration,
    volume: f32,
) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }

    std::thread::spawn(move || {
        append_log(&format!(
            "native movie trigger: waiting {delay:?} path=\"{path}\" volume={volume:.3}"
        ));
        std::thread::sleep(delay);

        let Ok(exe) = (unsafe { GetModuleHandleA(None) }) else {
            append_log("native movie trigger: main module unavailable");
            return;
        };
        let base = exe.0 as usize;
        let outer_slot = base + OUTER_MOVIE_SINGLETON_PTR_RVA;
        let movie_start_addr = base + MOVIE_START_RVA;

        let outer = unsafe { read_usize(outer_slot) };
        if outer == 0 {
            append_log(&format!(
                "native movie trigger: outer singleton is null at eldenring.exe+0x{OUTER_MOVIE_SINGLETON_PTR_RVA:X}"
            ));
            return;
        }

        let inner = unsafe { read_usize(outer + 0x38) };
        if inner == 0 {
            append_log(&format!(
                "native movie trigger: inner movie pointer is null outer=0x{outer:X}"
            ));
            return;
        }

        let active = unsafe { read_u8(inner + 0x130) };
        let state = unsafe { read_u32(inner + 0x40) };
        append_log(&format!(
            "native movie trigger: outer=0x{outer:X} inner=0x{inner:X} active[+130]=0x{active:02X} state[+40]=0x{state:X} movie_start=0x{movie_start_addr:X}"
        ));

        if active != 0 {
            append_log("native movie trigger: inner movie is already active; skipping");
            return;
        }

        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);

        let movie_start: MovieStartFn = unsafe { std::mem::transmute(movie_start_addr) };
        let result = unsafe { movie_start(inner, 1, wide.as_ptr(), volume, 0, 0, 1) };
        let active_after = unsafe { read_u8(inner + 0x130) };
        let state_after = unsafe { read_u32(inner + 0x40) };
        append_log(&format!(
            "native movie trigger: movie_start returned {result} active[+130]=0x{active_after:02X} state[+40]=0x{state_after:X}"
        ));
    });
}

unsafe fn read_usize(addr: usize) -> usize {
    unsafe { std::ptr::read_unaligned(addr as *const usize) }
}

unsafe fn read_u32(addr: usize) -> u32 {
    unsafe { std::ptr::read_unaligned(addr as *const u32) }
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
