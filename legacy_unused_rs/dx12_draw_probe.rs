use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use ilhook::x64::{CallbackOption, HookFlags, Registers, hook_closure_retn};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_GPU_DESCRIPTOR_HANDLE, D3D12_VERTEX_BUFFER_VIEW,
    D3D12_VIEWPORT, D3D12CreateDevice, ID3D12CommandAllocator, ID3D12Device,
    ID3D12GraphicsCommandList,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIFactory2,
};
use windows::core::{Interface, Result};

type DrawIndexedInstancedFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, i32, u32);
type DrawInstancedFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32);
type RSSetViewportsFn = unsafe extern "system" fn(*mut c_void, u32, *const D3D12_VIEWPORT);
type RSSetScissorRectsFn = unsafe extern "system" fn(*mut c_void, u32, *const RECT);
type SetGraphicsRootDescriptorTableFn =
    unsafe extern "system" fn(*mut c_void, u32, D3D12_GPU_DESCRIPTOR_HANDLE);
type IASetVertexBuffersFn =
    unsafe extern "system" fn(*mut c_void, u32, u32, *const D3D12_VERTEX_BUFFER_VIEW);

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static INSTALLED: AtomicUsize = AtomicUsize::new(0);
static DRAW_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static STATE: Mutex<Option<HashMap<usize, CommandListState>>> = Mutex::new(None);
static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

const MAX_DRAW_LOGS: usize = 600;
const FLUSH_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Default)]
struct CommandListState {
    viewport: Option<D3D12_VIEWPORT>,
    scissor: Option<RECT>,
    root_descriptor: [u64; 8],
    root_descriptor_set: [bool; 8],
    vertex_buffer: Option<D3D12_VERTEX_BUFFER_VIEW>,
}

struct CommandListMethods {
    draw_indexed_instanced: usize,
    draw_instanced: usize,
    rs_set_viewports: usize,
    rs_set_scissor_rects: usize,
    set_graphics_root_descriptor_table: usize,
    ia_set_vertex_buffers: usize,
}

pub(crate) fn install(log_path: Option<PathBuf>, install_delay: Duration) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path);
    }
    let _ = STARTED_AT.set(Instant::now());
    if INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    std::thread::spawn(move || {
        append_log(&format!(
            "dx12 draw probe: delayed install in {} ms",
            install_delay.as_millis()
        ));
        std::thread::sleep(install_delay);
        install_now();
    });
}

fn install_now() {
    if INSTALLED.swap(1, Ordering::AcqRel) != 0 {
        return;
    }

    match command_list_methods() {
        Ok(methods) => {
            append_log(&format!(
                "dx12 draw probe: methods DrawIndexed=0x{:X} Draw=0x{:X} Viewports=0x{:X} Scissors=0x{:X} RootTable=0x{:X} VB=0x{:X}",
                methods.draw_indexed_instanced,
                methods.draw_instanced,
                methods.rs_set_viewports,
                methods.rs_set_scissor_rects,
                methods.set_graphics_root_descriptor_table,
                methods.ia_set_vertex_buffers
            ));
            let hooks = unsafe {
                vec![
                    hook_closure_retn(
                        methods.draw_indexed_instanced,
                        |registers, original| draw_indexed_instanced_hook(registers, original),
                        CallbackOption::None,
                        HookFlags::empty(),
                    ),
                    hook_closure_retn(
                        methods.draw_instanced,
                        |registers, original| draw_instanced_hook(registers, original),
                        CallbackOption::None,
                        HookFlags::empty(),
                    ),
                    hook_closure_retn(
                        methods.rs_set_viewports,
                        |registers, original| rs_set_viewports_hook(registers, original),
                        CallbackOption::None,
                        HookFlags::empty(),
                    ),
                    hook_closure_retn(
                        methods.rs_set_scissor_rects,
                        |registers, original| rs_set_scissor_rects_hook(registers, original),
                        CallbackOption::None,
                        HookFlags::empty(),
                    ),
                    hook_closure_retn(
                        methods.set_graphics_root_descriptor_table,
                        |registers, original| {
                            set_graphics_root_descriptor_table_hook(registers, original)
                        },
                        CallbackOption::None,
                        HookFlags::empty(),
                    ),
                    hook_closure_retn(
                        methods.ia_set_vertex_buffers,
                        |registers, original| ia_set_vertex_buffers_hook(registers, original),
                        CallbackOption::None,
                        HookFlags::empty(),
                    ),
                ]
            };

            let mut installed = Vec::new();
            for hook in hooks {
                match hook {
                    Ok(hook) => installed.push(hook),
                    Err(err) => append_log(&format!("dx12 draw probe: hook failed: {err:?}")),
                }
            }
            let installed_count = installed.len();
            std::mem::forget(installed);
            if installed_count > 0 {
                std::thread::spawn(|| {
                    std::thread::sleep(FLUSH_DELAY);
                    flush_buffered_logs();
                    append_log("dx12 draw probe: sample flushed");
                });
            }
            append_log(&format!(
                "dx12 draw probe: installed {installed_count} hooks"
            ));
        }
        Err(err) => append_log(&format!(
            "dx12 draw probe: failed to resolve command list methods: {err:?}"
        )),
    }
}

fn command_list_methods() -> Result<CommandListMethods> {
    let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }?;
    let adapter = unsafe { factory.EnumAdapters(0) }?;
    let mut device = None;
    unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }?;
    let device: ID3D12Device = device.expect("D3D12CreateDevice returned success without device");
    let command_allocator: ID3D12CommandAllocator =
        unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }?;
    let command_list: ID3D12GraphicsCommandList = unsafe {
        device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None)
    }?;
    unsafe { command_list.Close()? };
    let vtable = command_list.vtable();
    Ok(CommandListMethods {
        draw_indexed_instanced: vtable.DrawIndexedInstanced as usize,
        draw_instanced: vtable.DrawInstanced as usize,
        rs_set_viewports: vtable.RSSetViewports as usize,
        rs_set_scissor_rects: vtable.RSSetScissorRects as usize,
        set_graphics_root_descriptor_table: vtable.SetGraphicsRootDescriptorTable as usize,
        ia_set_vertex_buffers: vtable.IASetVertexBuffers as usize,
    })
}

fn draw_indexed_instanced_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let command_list = registers.rcx as *mut c_void;
    let index_count = registers.rdx as u32;
    let instance_count = registers.r8 as u32;
    let start_index = registers.r9 as u32;
    let base_vertex = unsafe { registers.get_stack(1) as u32 as i32 };
    let start_instance = unsafe { registers.get_stack(2) as u32 };

    if should_log_draw(index_count, instance_count) {
        log_draw(
            command_list as usize,
            "DrawIndexed",
            index_count,
            instance_count,
            start_index,
            base_vertex,
            start_instance,
        );
    }

    let original: DrawIndexedInstancedFn = unsafe { std::mem::transmute(original) };
    unsafe {
        original(
            command_list,
            index_count,
            instance_count,
            start_index,
            base_vertex,
            start_instance,
        )
    };
    0
}

fn draw_instanced_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let command_list = registers.rcx as *mut c_void;
    let vertex_count = registers.rdx as u32;
    let instance_count = registers.r8 as u32;
    let start_vertex = registers.r9 as u32;
    let start_instance = unsafe { registers.get_stack(1) as u32 };

    if should_log_draw(vertex_count, instance_count) {
        log_draw(
            command_list as usize,
            "Draw",
            vertex_count,
            instance_count,
            start_vertex,
            0,
            start_instance,
        );
    }

    let original: DrawInstancedFn = unsafe { std::mem::transmute(original) };
    unsafe {
        original(
            command_list,
            vertex_count,
            instance_count,
            start_vertex,
            start_instance,
        )
    };
    0
}

fn rs_set_viewports_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let command_list = registers.rcx as *mut c_void;
    let count = registers.rdx as u32;
    let viewports = registers.r8 as *const D3D12_VIEWPORT;
    if DRAW_LOG_COUNT.load(Ordering::Relaxed) < MAX_DRAW_LOGS && count > 0 && !viewports.is_null() {
        with_state(command_list as usize, |state| {
            state.viewport = Some(unsafe { *viewports });
        });
    }

    let original: RSSetViewportsFn = unsafe { std::mem::transmute(original) };
    unsafe { original(command_list, count, viewports) };
    0
}

fn rs_set_scissor_rects_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let command_list = registers.rcx as *mut c_void;
    let count = registers.rdx as u32;
    let rects = registers.r8 as *const RECT;
    if DRAW_LOG_COUNT.load(Ordering::Relaxed) < MAX_DRAW_LOGS && count > 0 && !rects.is_null() {
        with_state(command_list as usize, |state| {
            state.scissor = Some(unsafe { *rects });
        });
    }

    let original: RSSetScissorRectsFn = unsafe { std::mem::transmute(original) };
    unsafe { original(command_list, count, rects) };
    0
}

fn set_graphics_root_descriptor_table_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let command_list = registers.rcx as *mut c_void;
    let index = registers.rdx as usize;
    let handle = D3D12_GPU_DESCRIPTOR_HANDLE { ptr: registers.r8 };
    if DRAW_LOG_COUNT.load(Ordering::Relaxed) < MAX_DRAW_LOGS && index < 8 {
        with_state(command_list as usize, |state| {
            state.root_descriptor[index] = handle.ptr;
            state.root_descriptor_set[index] = true;
        });
    }

    let original: SetGraphicsRootDescriptorTableFn = unsafe { std::mem::transmute(original) };
    unsafe { original(command_list, index as u32, handle) };
    0
}

fn ia_set_vertex_buffers_hook(registers: *mut Registers, original: usize) -> usize {
    let registers = unsafe { &*registers };
    let command_list = registers.rcx as *mut c_void;
    let start_slot = registers.rdx as u32;
    let count = registers.r8 as u32;
    let views = registers.r9 as *const D3D12_VERTEX_BUFFER_VIEW;
    if DRAW_LOG_COUNT.load(Ordering::Relaxed) < MAX_DRAW_LOGS
        && start_slot == 0
        && count > 0
        && !views.is_null()
    {
        with_state(command_list as usize, |state| {
            state.vertex_buffer = Some(unsafe { *views });
        });
    }

    let original: IASetVertexBuffersFn = unsafe { std::mem::transmute(original) };
    unsafe { original(command_list, start_slot, count, views) };
    0
}

fn should_log_draw(vertex_or_index_count: u32, instance_count: u32) -> bool {
    instance_count == 1 && vertex_or_index_count <= 12
}

fn log_draw(
    command_list: usize,
    kind: &str,
    count: u32,
    instance_count: u32,
    start: u32,
    base_vertex: i32,
    start_instance: u32,
) {
    let ordinal = DRAW_LOG_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if ordinal > MAX_DRAW_LOGS {
        return;
    }

    let state = current_state(command_list);
    let viewport = state
        .viewport
        .map(|v| {
            format!(
                "{:.0},{:.0} {:.0}x{:.0}",
                v.TopLeftX, v.TopLeftY, v.Width, v.Height
            )
        })
        .unwrap_or_else(|| "none".to_string());
    let scissor = state
        .scissor
        .map(|r| {
            format!(
                "{},{},{}x{}",
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top
            )
        })
        .unwrap_or_else(|| "none".to_string());
    let vb = state
        .vertex_buffer
        .map(|v| {
            format!(
                "addr=0x{:X} size={} stride={}",
                v.BufferLocation, v.SizeInBytes, v.StrideInBytes
            )
        })
        .unwrap_or_else(|| "none".to_string());
    let root = state
        .root_descriptor_set
        .iter()
        .enumerate()
        .filter(|(_, set)| **set)
        .map(|(index, _)| format!("r{index}=0x{:X}", state.root_descriptor[index]))
        .collect::<Vec<_>>()
        .join(" ");

    buffer_log(format!(
        "dx12 draw probe: #{ordinal} cmd=0x{command_list:X} {kind} count={count} instances={instance_count} start={start} base={base_vertex} start_instance={start_instance} viewport=[{viewport}] scissor=[{scissor}] vb=[{vb}] root=[{root}]"
    ));
}

fn with_state(command_list: usize, f: impl FnOnce(&mut CommandListState)) {
    if let Ok(mut states) = STATE.try_lock() {
        let states = states.get_or_insert_with(HashMap::new);
        f(states.entry(command_list).or_default());
    }
}

fn current_state(command_list: usize) -> CommandListState {
    STATE
        .try_lock()
        .ok()
        .and_then(|states| {
            states
                .as_ref()
                .and_then(|states| states.get(&command_list).copied())
        })
        .unwrap_or_default()
}

fn buffer_log(message: String) {
    if let Ok(mut buffer) = LOG_BUFFER.try_lock() {
        buffer.push(message);
    }
}

fn flush_buffered_logs() {
    let Ok(mut buffer) = LOG_BUFFER.lock() else {
        return;
    };
    for message in buffer.drain(..) {
        append_log(&message);
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
