use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Win32::Foundation::{HINSTANCE, HMODULE};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

mod bink_probe;
mod dx12_title_texture;

static STARTED: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
/// # Safety
pub unsafe extern "C" fn DllMain(hmodule: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    if reason != DLL_PROCESS_ATTACH {
        return 1;
    }
    if STARTED.swap(true, Ordering::SeqCst) {
        return 1;
    }

    let hmodule_raw = hmodule.0 as usize;
    std::thread::spawn(move || {
        let hmodule = HINSTANCE(hmodule_raw as *mut _);
        let config = Config::load(hmodule);
        append_log_path(config.log_path.as_ref(), "DLL_PROCESS_ATTACH");

        if config.probe_bink_open {
            bink_probe::install_async(
                config.log_path.clone(),
                config
                    .bink_replace_to
                    .clone()
                    .map(|to_path| bink_probe::BinkReplaceRule {
                        from_contains: config.bink_replace_from.clone(),
                        to_path,
                    }),
            );
        }
        if config.probe_movie_wrapper {
            bink_probe::install_movie_wrapper_probe(config.log_path.clone());
        }
        if config.probe_bink_texture_open {
            bink_probe::install_bink_texture_open_probe(
                config.log_path.clone(),
                config.bink_texture_force_present_flag,
                config.bink_texture_force_present_option,
                config.bink_texture_copy_present_option_after_open,
            );
        }
        if config.probe_movie_ins {
            bink_probe::install_movie_ins_probe(config.log_path.clone());
        }
        if config.probe_movie_step {
            bink_probe::install_movie_step_probe(config.log_path.clone());
        }
        if config.probe_movie_tick {
            bink_probe::install_movie_tick_probe(config.log_path.clone());
        }
        if config.probe_movie_render {
            bink_probe::install_movie_render_probe(config.log_path.clone());
        }
        if config.probe_movie_draw_submit {
            bink_probe::install_movie_draw_submit_probe(config.log_path.clone());
        }
        if config.probe_staffroll_screen {
            bink_probe::install_staffroll_screen_probe(
                config.log_path.clone(),
                config.probe_staffroll_broad,
            );
        }
        if config.probe_staffroll_ctor {
            bink_probe::install_staffroll_ctor_probe(config.log_path.clone());
        }
        if config.movie_imp_trigger && !config.movie_imp_trigger_on_title_target {
            bink_probe::trigger_er_movie_imp_once_after_delay(
                config.log_path.clone(),
                config.movie_imp_path.clone(),
                config.movie_imp_delay,
                config.movie_imp_volume,
            );
        }

        if config.probe_title_srv
            || config.enable_title_hijack
            || config.bink_plane_hijack
            || (config.movie_imp_trigger && config.movie_imp_trigger_on_title_target)
        {
            let title_target_callback: Option<Box<dyn Fn() + Send + Sync>> =
                if config.movie_imp_trigger && config.movie_imp_trigger_on_title_target {
                    let log_path = config.log_path.clone();
                    let movie_path = config.movie_imp_path.clone();
                    let delay = config.movie_imp_delay;
                    let volume = config.movie_imp_volume;
                    Some(Box::new(move || {
                        bink_probe::trigger_er_movie_imp_once(
                            log_path.clone(),
                            movie_path.clone(),
                            delay,
                            volume,
                            "title target descriptor",
                        );
                    }))
                } else {
                    None
                };
            dx12_title_texture::install(
                config.log_path.as_ref(),
                config.atlas_rgba_path.clone(),
                config.atlas_rect,
                config.hijack_title_index,
                config.hijack_resource_width,
                config.hijack_resource_height,
                config.hijack_require_bc7,
                (config.probe_title_srv || config.bink_plane_hijack) && !config.enable_title_hijack,
                config.atlas_debug_fill,
                config.bink_plane_hijack,
                config.bink_plane_target_title_index,
                config.bink_plane_source_index,
                config.bink_plane_source_width,
                config.bink_plane_source_height,
                config.bink_plane_source_format,
                config.bink_plane_probe_all,
                config.bink_plane_source_swizzle_rrr1,
                title_target_callback,
            );
        }
    });

    1
}

#[derive(Clone)]
struct Config {
    atlas_rgba_path: Option<PathBuf>,
    atlas_rect: dx12_title_texture::AtlasRect,
    hijack_title_index: Option<usize>,
    hijack_resource_width: u32,
    hijack_resource_height: u32,
    hijack_require_bc7: bool,
    atlas_debug_fill: Option<[u8; 4]>,
    enable_title_hijack: bool,
    probe_bink_open: bool,
    probe_bink_texture_open: bool,
    bink_texture_force_present_flag: bool,
    bink_texture_force_present_option: bool,
    bink_texture_copy_present_option_after_open: bool,
    probe_movie_wrapper: bool,
    probe_movie_ins: bool,
    probe_movie_step: bool,
    probe_movie_tick: bool,
    probe_movie_render: bool,
    probe_movie_draw_submit: bool,
    probe_staffroll_screen: bool,
    probe_staffroll_broad: bool,
    probe_staffroll_ctor: bool,
    movie_imp_trigger: bool,
    movie_imp_trigger_on_title_target: bool,
    movie_imp_path: String,
    movie_imp_delay: Duration,
    movie_imp_volume: f32,
    bink_replace_from: String,
    bink_replace_to: Option<PathBuf>,
    probe_title_srv: bool,
    bink_plane_hijack: bool,
    bink_plane_target_title_index: usize,
    bink_plane_source_index: usize,
    bink_plane_source_width: u32,
    bink_plane_source_height: u32,
    bink_plane_source_format: i32,
    bink_plane_probe_all: bool,
    bink_plane_source_swizzle_rrr1: bool,
    log_enabled: bool,
    log_path: Option<PathBuf>,
}

impl Config {
    fn load(hmodule: HINSTANCE) -> Self {
        let default_log_path = module_path(hmodule)
            .and_then(|path| path.parent().map(|dir| dir.join("dynamic-title-bg.log")));
        let mut config = Self {
            atlas_rgba_path: None,
            atlas_rect: dx12_title_texture::AtlasRect::default(),
            hijack_title_index: None,
            hijack_resource_width: 64,
            hijack_resource_height: 36,
            hijack_require_bc7: false,
            atlas_debug_fill: None,
            enable_title_hijack: false,
            probe_bink_open: false,
            probe_bink_texture_open: false,
            bink_texture_force_present_flag: false,
            bink_texture_force_present_option: false,
            bink_texture_copy_present_option_after_open: false,
            probe_movie_wrapper: false,
            probe_movie_ins: false,
            probe_movie_step: false,
            probe_movie_tick: false,
            probe_movie_render: false,
            probe_movie_draw_submit: false,
            probe_staffroll_screen: false,
            probe_staffroll_broad: false,
            probe_staffroll_ctor: false,
            movie_imp_trigger: false,
            movie_imp_trigger_on_title_target: false,
            movie_imp_path: "movie:/00001010.bk2".to_string(),
            movie_imp_delay: Duration::from_secs(8),
            movie_imp_volume: 0.7,
            bink_replace_from: "10010010.bk2".to_string(),
            bink_replace_to: None,
            probe_title_srv: false,
            bink_plane_hijack: false,
            bink_plane_target_title_index: 1,
            bink_plane_source_index: 1,
            bink_plane_source_width: 1920,
            bink_plane_source_height: 1080,
            bink_plane_source_format: 28,
            bink_plane_probe_all: false,
            bink_plane_source_swizzle_rrr1: false,
            log_enabled: false,
            log_path: None,
        };

        let mut loaded_config_path = None;
        for path in config_paths(hmodule) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            loaded_config_path = Some(path.clone());
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                    continue;
                }
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let key = key.trim().to_ascii_lowercase();
                let value = value.trim().trim_matches('"');
                match key.as_str() {
                    "log_enabled" | "enable_log" | "log" => {
                        config.log_enabled = parse_bool(value, false);
                    }
                    "atlas_rgba" => config.atlas_rgba_path = Some(PathBuf::from(value)),
                    "atlas_rect" => {
                        if let Some(rect) = parse_atlas_rect(value) {
                            config.atlas_rect = rect;
                        }
                    }
                    "hijack_title_index" => {
                        if let Ok(index) = value.parse::<usize>() {
                            config.hijack_title_index = Some(index);
                        }
                    }
                    "hijack_resource_width" | "hijack_target_width" => {
                        if let Ok(width) = value.parse::<u32>() {
                            if width > 0 {
                                config.hijack_resource_width = width;
                            }
                        }
                    }
                    "hijack_resource_height" | "hijack_target_height" => {
                        if let Ok(height) = value.parse::<u32>() {
                            if height > 0 {
                                config.hijack_resource_height = height;
                            }
                        }
                    }
                    "hijack_require_bc7" => {
                        config.hijack_require_bc7 = parse_bool(value, false);
                    }
                    "atlas_debug_fill" => {
                        config.atlas_debug_fill = parse_rgba(value);
                    }
                    "enable_title_hijack" | "title_hijack" => {
                        config.enable_title_hijack = parse_bool(value, false);
                    }
                    "probe_bink_open" | "probe_bink" => {
                        config.probe_bink_open = parse_bool(value, false);
                    }
                    "probe_bink_texture_open" | "probe_bink_texture" => {
                        config.probe_bink_texture_open = parse_bool(value, false);
                    }
                    "bink_texture_force_present_flag" | "force_bink_texture_present_flag" => {
                        config.bink_texture_force_present_flag = parse_bool(value, false);
                    }
                    "bink_texture_force_present_option"
                    | "force_bink_texture_present_option"
                    | "bink_texture_force_option_present" => {
                        config.bink_texture_force_present_option = parse_bool(value, false);
                    }
                    "bink_texture_copy_present_option_after_open"
                    | "copy_bink_texture_present_option_after_open"
                    | "bink_texture_copy_present_option" => {
                        config.bink_texture_copy_present_option_after_open =
                            parse_bool(value, false);
                    }
                    "probe_movie_wrapper" | "probe_movie_open_wrapper" => {
                        config.probe_movie_wrapper = parse_bool(value, false);
                    }
                    "probe_movie_ins" | "probe_movie_instance" => {
                        config.probe_movie_ins = parse_bool(value, false);
                    }
                    "probe_movie_step" | "probe_movie_state_step" => {
                        config.probe_movie_step = parse_bool(value, false);
                    }
                    "probe_movie_tick" | "probe_movie_update" => {
                        config.probe_movie_tick = parse_bool(value, false);
                    }
                    "probe_movie_render" | "probe_movie_draw" => {
                        config.probe_movie_render = parse_bool(value, false);
                    }
                    "probe_movie_draw_submit" | "probe_movie_submit" => {
                        config.probe_movie_draw_submit = parse_bool(value, false);
                    }
                    "probe_staffroll_screen" | "probe_staffroll" | "probe_title_staffroll" => {
                        config.probe_staffroll_screen = parse_bool(value, false);
                    }
                    "probe_staffroll_broad" | "probe_staffroll_slots" => {
                        config.probe_staffroll_broad = parse_bool(value, false);
                    }
                    "probe_staffroll_ctor" | "probe_staffroll_constructor" => {
                        config.probe_staffroll_ctor = parse_bool(value, false);
                    }
                    "movie_imp_trigger" | "enable_movie_imp_trigger" => {
                        config.movie_imp_trigger = parse_bool(value, false);
                    }
                    "movie_imp_trigger_on_title_target"
                    | "movie_imp_wait_for_title_target"
                    | "movie_imp_trigger_after_title_target" => {
                        config.movie_imp_trigger_on_title_target = parse_bool(value, false);
                    }
                    "movie_imp_path" => {
                        config.movie_imp_path = value.to_string();
                    }
                    "movie_imp_delay_ms" => {
                        if let Ok(ms) = value.parse::<u64>() {
                            config.movie_imp_delay = Duration::from_millis(ms);
                        }
                    }
                    "movie_imp_volume" => {
                        if let Ok(volume) = value.parse::<f32>() {
                            if volume.is_finite() {
                                config.movie_imp_volume = volume.clamp(0.0, 1.0);
                            }
                        }
                    }
                    "bink_replace_from" => {
                        config.bink_replace_from = value.to_string();
                    }
                    "bink_replace_to" => {
                        config.bink_replace_to = Some(PathBuf::from(value));
                    }
                    "probe_title_srv" | "probe_srv" => {
                        config.probe_title_srv = parse_bool(value, false);
                    }
                    "bink_plane_hijack" | "bink_plane_to_title" => {
                        config.bink_plane_hijack = parse_bool(value, false);
                    }
                    "bink_plane_target_title_index" | "bink_plane_title_index" => {
                        if let Ok(index) = value.parse::<usize>() {
                            config.bink_plane_target_title_index = index.max(1);
                        }
                    }
                    "bink_plane_source_index" | "bink_plane_index" => {
                        if let Ok(index) = value.parse::<usize>() {
                            config.bink_plane_source_index = index.max(1);
                        }
                    }
                    "bink_plane_source_width" => {
                        if let Ok(width) = value.parse::<u32>() {
                            if width > 0 {
                                config.bink_plane_source_width = width;
                            }
                        }
                    }
                    "bink_plane_source_height" => {
                        if let Ok(height) = value.parse::<u32>() {
                            if height > 0 {
                                config.bink_plane_source_height = height;
                            }
                        }
                    }
                    "bink_plane_source_format" => {
                        if let Ok(format) = value.parse::<i32>() {
                            config.bink_plane_source_format = format;
                        }
                    }
                    "bink_plane_probe_all" | "probe_bink_planes" => {
                        config.bink_plane_probe_all = parse_bool(value, false);
                    }
                    "bink_plane_source_swizzle_rrr1" | "bink_plane_swizzle_rrr1" => {
                        config.bink_plane_source_swizzle_rrr1 = parse_bool(value, false);
                    }
                    _ => {}
                }
            }
            break;
        }

        if config.log_enabled {
            config.log_path = default_log_path;
            if let Some(path) = loaded_config_path {
                append_log_path(
                    config.log_path.as_ref(),
                    &format!("loaded config {}", path.display()),
                );
            }
        }

        config
    }
}

fn append_log_path(path: Option<&PathBuf>, message: &str) {
    let Some(path) = path else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn config_paths(hmodule: HINSTANCE) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(dll_path) = module_path(hmodule) {
        if let Some(dir) = dll_path.parent() {
            paths.push(dir.join("dynamic-title-bg.ini"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("dynamic-title-bg.ini"));
        }
    }
    if let Ok(dir) = std::env::current_dir() {
        paths.push(dir.join("dynamic-title-bg.ini"));
    }
    paths
}

fn module_path(hmodule: HINSTANCE) -> Option<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(Some(HMODULE(hmodule.0)), &mut buffer) };
    if len == 0 {
        return None;
    }
    buffer.truncate(len as usize);
    Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
}

fn parse_bool(value: &str, default: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn parse_atlas_rect(value: &str) -> Option<dx12_title_texture::AtlasRect> {
    let mut parts = value.split(',').map(|part| part.trim().parse::<u32>().ok());
    let x = parts.next()??;
    let y = parts.next()??;
    let width = parts.next()??;
    let height = parts.next()??;
    if parts.next().is_some() || width == 0 || height == 0 {
        return None;
    }
    Some(dx12_title_texture::AtlasRect {
        x,
        y,
        width,
        height,
    })
}

fn parse_rgba(value: &str) -> Option<[u8; 4]> {
    let mut parts = value.split(',').map(|part| part.trim().parse::<u8>().ok());
    let r = parts.next()??;
    let g = parts.next()??;
    let b = parts.next()??;
    let a = parts.next()??;
    if parts.next().is_some() {
        return None;
    }
    Some([r, g, b, a])
}
