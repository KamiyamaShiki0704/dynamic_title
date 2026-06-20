use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use ilhook::x64::{CallbackOption, HookFlags, Registers, hook_closure_retn};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
    D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
    D3D12_FENCE_FLAG_NONE, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT,
    D3D12_HEAP_TYPE_UPLOAD, D3D12_MEMORY_POOL_UNKNOWN, D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
    D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_DESC,
    D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_NONE,
    D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_GENERIC_READ,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATES,
    D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_SHADER_RESOURCE_VIEW_DESC,
    D3D12_SHADER_RESOURCE_VIEW_DESC_0, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_SUBRESOURCE_FOOTPRINT,
    D3D12_TEX2D_SRV, D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
    D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
    D3D12_TEXTURE_DATA_PITCH_ALIGNMENT, D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
    D3D12_TEXTURE_LAYOUT_UNKNOWN, D3D12CreateDevice, ID3D12CommandAllocator, ID3D12CommandList,
    ID3D12CommandQueue, ID3D12Device, ID3D12Fence, ID3D12GraphicsCommandList, ID3D12Resource,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_BC7_UNORM, DXGI_FORMAT_BC7_UNORM_SRGB, DXGI_FORMAT_R8G8B8A8_UNORM,
    DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIFactory2,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, WaitForSingleObject};
use windows::core::{Interface, Result};

type CreateShaderResourceViewFn = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *const D3D12_SHADER_RESOURCE_VIEW_DESC,
    usize,
);

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static CREATE_SRV_HOOK_INSTALLED: AtomicUsize = AtomicUsize::new(0);
static CANDIDATE_COUNT: AtomicUsize = AtomicUsize::new(0);
static TITLE_MATCH_COUNT: AtomicUsize = AtomicUsize::new(0);
static SRV_COUNT: AtomicUsize = AtomicUsize::new(0);
static HIJACK_COUNT: AtomicUsize = AtomicUsize::new(0);
static BINK_PLANE_MATCH_COUNT: AtomicUsize = AtomicUsize::new(0);
static BINK_PLANE_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);
static STORED_TITLE_DESCRIPTOR: AtomicUsize = AtomicUsize::new(0);
static BINK_SOURCE_CAPTURE_ENABLED: AtomicUsize = AtomicUsize::new(0);
static STORED_BINK_PLANE: Mutex<Option<BinkPlaneSource>> = Mutex::new(None);
static DYNAMIC_TEXTURE: Mutex<Option<DynamicTexture>> = Mutex::new(None);
static ATLAS_SETTINGS: OnceLock<AtlasSettings> = OnceLock::new();
static TITLE_TARGET_CALLBACK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
static TITLE_TARGET_CALLBACK_FIRED: AtomicUsize = AtomicUsize::new(0);

fn encode_shader_4_component_mapping(src0: u32, src1: u32, src2: u32, src3: u32) -> u32 {
    (src0 & 0x7) | ((src1 & 0x7) << 3) | ((src2 & 0x7) << 6) | ((src3 & 0x7) << 9) | (1 << 12)
}

fn shader_mapping_rrr1() -> u32 {
    encode_shader_4_component_mapping(0, 0, 0, 5)
}

#[derive(Clone, Copy)]
pub(crate) struct AtlasRect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Default for AtlasRect {
    fn default() -> Self {
        Self {
            x: 0,
            y: 64,
            width: 2960,
            height: 1696,
        }
    }
}

struct AtlasSettings {
    rgba_path: Option<PathBuf>,
    rect: AtlasRect,
    hijack_title_index: Option<usize>,
    target_width: u32,
    target_height: u32,
    require_bc7: bool,
    probe_only: bool,
    debug_fill_rgba: Option<[u8; 4]>,
    bink_plane_hijack: bool,
    bink_plane_target_title_index: usize,
    bink_plane_auto_source: bool,
    bink_plane_source_index: usize,
    bink_plane_source_width: u32,
    bink_plane_source_height: u32,
    bink_plane_source_format: i32,
    bink_plane_probe_all: bool,
    bink_plane_source_swizzle_rrr1: bool,
}

struct DynamicTexture {
    device_ptr: usize,
    device: ID3D12Device,
    resource: ID3D12Resource,
    command_queue: ID3D12CommandQueue,
    command_allocator: ID3D12CommandAllocator,
    command_list: ID3D12GraphicsCommandList,
    fence: ID3D12Fence,
    fence_event: HANDLE,
    fence_value: u64,
    width: u32,
    height: u32,
    state: D3D12_RESOURCE_STATES,
}

unsafe impl Send for DynamicTexture {}

struct BinkPlaneSource {
    device_ptr: usize,
    resource: ID3D12Resource,
    srv_desc: Option<D3D12_SHADER_RESOURCE_VIEW_DESC>,
    width: u64,
    height: u32,
    format: i32,
}

unsafe impl Send for BinkPlaneSource {}

fn is_near_16x9(width: u64, height: u32) -> bool {
    width > 0
        && height > 0
        && ((width as i128 * 9) - (height as i128 * 16)).abs()
            <= (width.max(height as u64) as i128 / 32).max(1)
}

impl Drop for DynamicTexture {
    fn drop(&mut self) {
        if !self.fence_event.is_invalid() {
            let _ = unsafe { CloseHandle(self.fence_event) };
        }
    }
}

pub(crate) fn install(
    log_path: Option<&PathBuf>,
    atlas_rgba_path: Option<PathBuf>,
    atlas_rect: AtlasRect,
    hijack_title_index: Option<usize>,
    target_width: u32,
    target_height: u32,
    require_bc7: bool,
    probe_only: bool,
    debug_fill_rgba: Option<[u8; 4]>,
    bink_plane_hijack: bool,
    bink_plane_target_title_index: usize,
    bink_plane_auto_source: bool,
    bink_plane_source_index: usize,
    bink_plane_source_width: u32,
    bink_plane_source_height: u32,
    bink_plane_source_format: i32,
    bink_plane_probe_all: bool,
    bink_plane_source_swizzle_rrr1: bool,
    title_target_callback: Option<Box<dyn Fn() + Send + Sync>>,
) {
    if let Some(path) = log_path {
        let _ = LOG_PATH.set(path.clone());
    }
    if let Some(callback) = title_target_callback {
        let _ = TITLE_TARGET_CALLBACK.set(callback);
    }
    let _ = ATLAS_SETTINGS.set(AtlasSettings {
        rgba_path: atlas_rgba_path,
        rect: atlas_rect,
        hijack_title_index,
        target_width,
        target_height,
        require_bc7,
        probe_only,
        debug_fill_rgba,
        bink_plane_hijack,
        bink_plane_target_title_index,
        bink_plane_auto_source,
        bink_plane_source_index,
        bink_plane_source_width,
        bink_plane_source_height,
        bink_plane_source_format,
        bink_plane_probe_all,
        bink_plane_source_swizzle_rrr1,
    });
    if CREATE_SRV_HOOK_INSTALLED.load(Ordering::Acquire) != 0 {
        return;
    }

    match create_shader_resource_view_addr() {
        Ok(addr) => {
            append_log(&format!(
                "dx12 title texture probe: CreateShaderResourceView=0x{addr:X}"
            ));
            let hook = unsafe {
                hook_closure_retn(
                    addr,
                    |registers, original| create_shader_resource_view_hook(registers, original),
                    CallbackOption::None,
                    HookFlags::empty(),
                )
            };
            match hook {
                Ok(hook) => {
                    let _ = Box::leak(Box::new(hook));
                    CREATE_SRV_HOOK_INSTALLED.store(1, Ordering::Release);
                    append_log("dx12 title texture probe: SRV hook installed");
                }
                Err(err) => {
                    append_log(&format!(
                        "dx12 title texture probe: SRV hook failed: {err:?}"
                    ));
                }
            }
        }
        Err(err) => {
            append_log(&format!(
                "dx12 title texture probe: failed to resolve CreateShaderResourceView: {err:?}"
            ));
        }
    }
}

pub(crate) fn reset_bink_bridge_cycle(reason: &str) {
    BINK_SOURCE_CAPTURE_ENABLED.store(0, Ordering::Release);
    BINK_PLANE_MATCH_COUNT.store(0, Ordering::Release);
    TITLE_TARGET_CALLBACK_FIRED.store(0, Ordering::Release);
    let has_video_source = STORED_BINK_PLANE
        .lock()
        .map(|source| source.is_some())
        .unwrap_or(false);
    append_log(&format!(
        "dx12 title texture probe: reset bink bridge cycle reason={reason} froze_video_source={} preserved_title_descriptor=0x{:X}",
        has_video_source,
        STORED_TITLE_DESCRIPTOR.load(Ordering::Acquire)
    ));
}

pub(crate) fn enable_bink_bridge_source_capture(reason: &str) {
    BINK_SOURCE_CAPTURE_ENABLED.store(1, Ordering::Release);
    if let Ok(mut source) = STORED_BINK_PLANE.lock() {
        *source = None;
    }
    BINK_PLANE_MATCH_COUNT.store(0, Ordering::Release);
    append_log(&format!(
        "dx12 title texture probe: enabled bink source capture reason={reason}"
    ));
}

fn create_shader_resource_view_addr() -> Result<usize> {
    let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }?;
    let adapter = unsafe { factory.EnumAdapters(0) }?;
    let mut device = None;
    unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }?;
    let device: ID3D12Device = device.expect("D3D12CreateDevice returned success without device");
    Ok(device.vtable().CreateShaderResourceView as usize)
}

fn create_shader_resource_view_hook(registers: *mut Registers, original: usize) -> usize {
    let original: CreateShaderResourceViewFn = unsafe { std::mem::transmute(original) };
    let registers = unsafe { &*registers };
    let device = registers.rcx as *mut c_void;
    let resource = registers.rdx as *mut c_void;
    let desc = registers.r8 as *const D3D12_SHADER_RESOURCE_VIEW_DESC;
    let descriptor = registers.r9 as usize;
    let caller = unsafe { registers.get_stack(0) as usize };

    unsafe { original(device, resource, desc, descriptor) };
    inspect_srv(device, original, resource, desc, descriptor, caller);
    0
}

fn inspect_srv(
    device: *mut c_void,
    original: CreateShaderResourceViewFn,
    resource: *mut c_void,
    desc: *const D3D12_SHADER_RESOURCE_VIEW_DESC,
    descriptor: usize,
    caller: usize,
) {
    let seen = SRV_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if resource.is_null() {
        return;
    }

    let Some(resource_ref) = (unsafe { ID3D12Resource::from_raw_borrowed(&resource) }) else {
        return;
    };
    let resource_desc = unsafe { resource_ref.GetDesc() };
    let srv_format = if desc.is_null() {
        resource_desc.Format
    } else {
        unsafe { (*desc).Format }
    };
    let view_dimension = if desc.is_null() {
        None
    } else {
        Some(unsafe { (*desc).ViewDimension })
    };

    let settings = ATLAS_SETTINGS.get();
    let target_width = settings
        .map(|settings| settings.target_width)
        .unwrap_or(4096);
    let target_height = settings
        .map(|settings| settings.target_height)
        .unwrap_or(2048);
    let require_bc7 = settings
        .map(|settings| settings.require_bc7)
        .unwrap_or(true);
    let is_target_sized =
        resource_desc.Width == target_width as u64 && resource_desc.Height == target_height;
    let is_bc7 = resource_desc.Format == DXGI_FORMAT_BC7_UNORM
        || resource_desc.Format == DXGI_FORMAT_BC7_UNORM_SRGB
        || srv_format == DXGI_FORMAT_BC7_UNORM
        || srv_format == DXGI_FORMAT_BC7_UNORM_SRGB;
    let is_texture2d = view_dimension
        .map(|dimension| dimension == D3D12_SRV_DIMENSION_TEXTURE2D)
        .unwrap_or(true);

    if is_target_sized || (is_bc7 && is_texture2d) {
        let candidate = CANDIDATE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        append_log(&format!(
            "dx12 title texture probe: candidate #{candidate} srv#{seen} resource={resource:p} desc={:?} {}x{} array={} mips={} srv_format={:?} view={:?} descriptor=0x{descriptor:X} caller=0x{caller:X}",
            resource_desc.Format,
            resource_desc.Width,
            resource_desc.Height,
            resource_desc.DepthOrArraySize,
            resource_desc.MipLevels,
            srv_format,
            view_dimension,
        ));
    } else if seen <= 16 {
        append_log(&format!(
            "dx12 title texture probe: early srv#{seen} resource={resource:p} desc={:?} {}x{} mips={} srv_format={:?} view={:?}",
            resource_desc.Format,
            resource_desc.Width,
            resource_desc.Height,
            resource_desc.MipLevels,
            srv_format,
            view_dimension,
        ));
    }

    maybe_hijack_title_with_bink_plane(device, original, resource, desc, &resource_desc);
    maybe_log_bink_plane_inventory(
        resource,
        desc,
        &resource_desc,
        srv_format,
        is_texture2d,
        descriptor,
        caller,
    );

    if is_probable_title_resource(
        resource_desc.Width,
        resource_desc.Height,
        resource_desc.MipLevels,
        is_bc7,
        target_width,
        target_height,
        require_bc7,
    ) {
        let title_index = TITLE_MATCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if settings
            .map(|settings| {
                settings.bink_plane_hijack && settings.bink_plane_target_title_index == title_index
            })
            .unwrap_or(false)
        {
            STORED_TITLE_DESCRIPTOR.store(descriptor, Ordering::Release);
            append_log(&format!(
                "dx12 title texture probe: stored title descriptor title_index=#{title_index} descriptor=0x{descriptor:X} resource={resource:p}"
            ));
            fire_title_target_callback_once();
            maybe_apply_bink_plane_to_title(device, original);
        }
        if ATLAS_SETTINGS
            .get()
            .map(|settings| settings.probe_only)
            .unwrap_or(false)
        {
            append_log(&format!(
                "dx12 title texture probe: probe-only title_index=#{title_index} descriptor=0x{descriptor:X} resource={resource:p} caller=0x{caller:X}"
            ));
            return;
        }
        if !should_hijack_title_index(title_index) {
            append_log(&format!(
                "dx12 title texture probe: skipped title-sized descriptor title_index=#{title_index} descriptor=0x{descriptor:X}"
            ));
            return;
        }
        match hijack_descriptor(device, original, descriptor) {
            Ok(()) => {
                let count = HIJACK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                append_log(&format!(
                    "dx12 title texture probe: hijacked title-sized descriptor #{count} title_index=#{title_index} descriptor=0x{descriptor:X}"
                ));
            }
            Err(err) => {
                append_log(&format!(
                    "dx12 title texture probe: hijack failed descriptor=0x{descriptor:X}: {err:?}"
                ));
            }
        }
    }
}

fn fire_title_target_callback_once() {
    if TITLE_TARGET_CALLBACK_FIRED.swap(1, Ordering::AcqRel) != 0 {
        return;
    }
    let Some(callback) = TITLE_TARGET_CALLBACK.get() else {
        return;
    };
    append_log("dx12 title texture probe: title target callback fired");
    callback();
}

fn maybe_log_bink_plane_inventory(
    resource: *mut c_void,
    desc: *const D3D12_SHADER_RESOURCE_VIEW_DESC,
    resource_desc: &D3D12_RESOURCE_DESC,
    srv_format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT,
    is_texture2d: bool,
    descriptor: usize,
    caller: usize,
) {
    let Some(settings) = ATLAS_SETTINGS.get() else {
        return;
    };
    if !settings.bink_plane_probe_all || !is_texture2d {
        return;
    }

    let width = resource_desc.Width;
    let height = resource_desc.Height;
    let source_width = settings.bink_plane_source_width as u64;
    let source_height = settings.bink_plane_source_height;
    if source_width == 0 || source_height == 0 {
        return;
    }

    let resource_format = resource_desc.Format.0;
    let srv_format_raw = srv_format.0;
    let interesting_format = matches!(
        resource_format,
        28 | 49 | 60 | 61 | 62 | 87 | 88 | 97 | 98 | 99
    ) || matches!(
        srv_format_raw,
        28 | 49 | 60 | 61 | 62 | 87 | 88 | 97 | 98 | 99
    );
    if !interesting_format {
        return;
    }

    let near_16x9 = is_near_16x9(width, height);
    let large_enough = width >= (source_width / 4).max(1)
        && height >= (source_height / 4).max(1)
        && width <= source_width
        && height <= source_height;
    if !near_16x9 || !large_enough {
        return;
    }

    let count = BINK_PLANE_PROBE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if count > 96 {
        return;
    }
    let mapping = if desc.is_null() {
        D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING
    } else {
        unsafe { (*desc).Shader4ComponentMapping }
    };
    append_log(&format!(
        "dx12 title texture probe: bink inventory #{count} resource={resource:p} desc={:?} {}x{} array={} mips={} srv_format={:?} mapping=0x{mapping:X} descriptor=0x{descriptor:X} caller=0x{caller:X}",
        resource_desc.Format,
        width,
        height,
        resource_desc.DepthOrArraySize,
        resource_desc.MipLevels,
        srv_format,
    ));
}

fn maybe_hijack_title_with_bink_plane(
    device: *mut c_void,
    original: CreateShaderResourceViewFn,
    resource: *mut c_void,
    desc: *const D3D12_SHADER_RESOURCE_VIEW_DESC,
    resource_desc: &D3D12_RESOURCE_DESC,
) {
    let Some(settings) = ATLAS_SETTINGS.get() else {
        return;
    };
    if !settings.bink_plane_hijack {
        return;
    }
    if BINK_SOURCE_CAPTURE_ENABLED.load(Ordering::Acquire) == 0 {
        return;
    }
    if settings.bink_plane_auto_source {
        maybe_auto_store_bink_plane_source(device, original, resource, desc, resource_desc);
        return;
    }
    if resource_desc.Width != settings.bink_plane_source_width as u64
        || resource_desc.Height != settings.bink_plane_source_height
        || resource_desc.Format.0 != settings.bink_plane_source_format
    {
        return;
    }

    let plane_index = BINK_PLANE_MATCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let target_descriptor = STORED_TITLE_DESCRIPTOR.load(Ordering::Acquire);
    let mapping = if desc.is_null() {
        D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING
    } else {
        unsafe { (*desc).Shader4ComponentMapping }
    };
    append_log(&format!(
        "dx12 title texture probe: bink plane candidate #{plane_index} resource={resource:p} desc={:?} {}x{} mapping=0x{mapping:X} target_descriptor=0x{target_descriptor:X}",
        resource_desc.Format, resource_desc.Width, resource_desc.Height,
    ));
    if plane_index != settings.bink_plane_source_index {
        return;
    }

    if !store_bink_plane_source(device, resource, desc, resource_desc) {
        return;
    }
    append_log(&format!(
        "dx12 title texture probe: stored bink plane source #{plane_index} resource={resource:p} descriptor_ready={}",
        target_descriptor != 0
    ));
    maybe_apply_bink_plane_to_title(device, original);
}

fn maybe_auto_store_bink_plane_source(
    device: *mut c_void,
    original: CreateShaderResourceViewFn,
    resource: *mut c_void,
    desc: *const D3D12_SHADER_RESOURCE_VIEW_DESC,
    resource_desc: &D3D12_RESOURCE_DESC,
) {
    let Some(settings) = ATLAS_SETTINGS.get() else {
        return;
    };
    let target_descriptor = STORED_TITLE_DESCRIPTOR.load(Ordering::Acquire);
    if target_descriptor == 0 {
        return;
    }

    let source_format = if settings.bink_plane_source_format > 0 {
        settings.bink_plane_source_format
    } else {
        28
    };
    if resource_desc.Format.0 != source_format
        || resource_desc.MipLevels != 1
        || resource_desc.Width < 640
        || resource_desc.Height < 360
        || !is_near_16x9(resource_desc.Width, resource_desc.Height)
    {
        return;
    }

    if STORED_BINK_PLANE
        .lock()
        .expect("stored bink plane mutex poisoned")
        .is_some()
    {
        return;
    }

    let auto_index = BINK_PLANE_MATCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    append_log(&format!(
        "dx12 title texture probe: auto bink source candidate #{auto_index} resource={resource:p} desc={:?} {}x{} target_descriptor=0x{target_descriptor:X}",
        resource_desc.Format, resource_desc.Width, resource_desc.Height
    ));
    if !store_bink_plane_source(device, resource, desc, resource_desc) {
        return;
    }
    append_log(&format!(
        "dx12 title texture probe: auto stored bink source #{auto_index} resource={resource:p} {}x{} fmt={}",
        resource_desc.Width, resource_desc.Height, resource_desc.Format.0
    ));
    maybe_apply_bink_plane_to_title(device, original);
}

fn store_bink_plane_source(
    device: *mut c_void,
    resource: *mut c_void,
    desc: *const D3D12_SHADER_RESOURCE_VIEW_DESC,
    resource_desc: &D3D12_RESOURCE_DESC,
) -> bool {
    let Some(resource_ref) = (unsafe { ID3D12Resource::from_raw_borrowed(&resource) }) else {
        append_log("dx12 title texture probe: failed to borrow bink plane resource");
        return false;
    };
    let srv_desc = if desc.is_null() {
        None
    } else {
        Some(unsafe { *desc })
    };
    let mut stored = STORED_BINK_PLANE
        .lock()
        .expect("stored bink plane mutex poisoned");
    *stored = Some(BinkPlaneSource {
        device_ptr: device as usize,
        resource: resource_ref.clone(),
        srv_desc,
        width: resource_desc.Width,
        height: resource_desc.Height,
        format: resource_desc.Format.0,
    });
    true
}

fn maybe_apply_bink_plane_to_title(device: *mut c_void, original: CreateShaderResourceViewFn) {
    let Some(settings) = ATLAS_SETTINGS.get() else {
        return;
    };
    if !settings.bink_plane_hijack {
        return;
    }
    let target_descriptor = STORED_TITLE_DESCRIPTOR.load(Ordering::Acquire);
    if target_descriptor == 0 {
        append_log("dx12 title texture probe: bink bridge waiting for target descriptor");
        return;
    }

    let stored = STORED_BINK_PLANE
        .lock()
        .expect("stored bink plane mutex poisoned");
    let Some(source) = stored.as_ref() else {
        append_log(&format!(
            "dx12 title texture probe: bink bridge waiting for source plane target_descriptor=0x{target_descriptor:X}"
        ));
        return;
    };
    if source.device_ptr != device as usize {
        append_log(&format!(
            "dx12 title texture probe: bink bridge skipped device mismatch source=0x{:X} current=0x{:X}",
            source.device_ptr, device as usize
        ));
        return;
    }

    let mut desc_storage = source.srv_desc.clone();
    let swizzled = settings.bink_plane_source_swizzle_rrr1 && source.format == 61;
    if swizzled {
        if let Some(desc) = desc_storage.as_mut() {
            desc.Shader4ComponentMapping = shader_mapping_rrr1();
        }
    }
    let desc_ptr = desc_storage
        .as_ref()
        .map(|desc| desc as *const D3D12_SHADER_RESOURCE_VIEW_DESC)
        .unwrap_or(ptr::null());
    unsafe {
        original(
            device,
            source.resource.as_raw(),
            desc_ptr,
            target_descriptor,
        )
    };
    append_log(&format!(
        "dx12 title texture probe: bink bridge applied source {}x{} fmt={} swizzle_rrr1={} to title descriptor=0x{target_descriptor:X}",
        source.width, source.height, source.format, swizzled
    ));
}

fn is_probable_title_resource(
    width: u64,
    height: u32,
    mip_levels: u16,
    is_bc7: bool,
    target_width: u32,
    target_height: u32,
    require_bc7: bool,
) -> bool {
    (!require_bc7 || is_bc7)
        && width == target_width as u64
        && height == target_height
        && mip_levels == 1
}

fn should_hijack_title_index(title_index: usize) -> bool {
    ATLAS_SETTINGS
        .get()
        .and_then(|settings| settings.hijack_title_index)
        .map(|wanted| wanted == 0 || wanted == title_index)
        .unwrap_or(true)
}

fn hijack_descriptor(
    device: *mut c_void,
    original: CreateShaderResourceViewFn,
    descriptor: usize,
) -> Result<()> {
    let mut texture = DYNAMIC_TEXTURE
        .lock()
        .expect("dynamic texture mutex poisoned");
    let device_ptr = device as usize;
    if texture
        .as_ref()
        .map(|texture| texture.device_ptr != device_ptr)
        .unwrap_or(true)
    {
        append_log("dx12 title texture probe: creating RGBA title atlas texture");
        *texture = Some(unsafe { DynamicTexture::new(device)? });
    }

    let Some(texture) = texture.as_ref() else {
        return Ok(());
    };
    let srv_desc = texture.srv_desc();
    unsafe { original(device, texture.resource.as_raw(), &srv_desc, descriptor) };
    Ok(())
}

impl DynamicTexture {
    unsafe fn new(device: *mut c_void) -> Result<Self> {
        let Some(device_ref) = (unsafe { ID3D12Device::from_raw_borrowed(&device) }) else {
            return Err(windows::core::Error::from_win32());
        };
        let device = device_ref.clone();
        let settings = ATLAS_SETTINGS.get();
        let width = settings
            .map(|settings| settings.target_width)
            .unwrap_or(4096);
        let height = settings
            .map(|settings| settings.target_height)
            .unwrap_or(2048);

        let command_queue: ID3D12CommandQueue = unsafe {
            device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                Priority: 0,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            })
        }?;
        let command_allocator: ID3D12CommandAllocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }?;
        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None)
        }?;
        unsafe { command_list.Close()? };
        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }?;
        let fence_event = unsafe { CreateEventW(None, false, false, None)? };

        let mut resource = None;
        unsafe {
            device.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_DEFAULT,
                    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                    CreationNodeMask: 0,
                    VisibleNodeMask: 0,
                },
                D3D12_HEAP_FLAG_NONE,
                &D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                    Alignment: 0,
                    Width: width as u64,
                    Height: height,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                    Flags: D3D12_RESOURCE_FLAG_NONE,
                },
                D3D12_RESOURCE_STATE_COPY_DEST,
                None,
                &mut resource,
            )?;
        }
        let resource: ID3D12Resource =
            resource.expect("CreateCommittedResource returned success without resource");
        let rect = ATLAS_SETTINGS
            .get()
            .map(|settings| settings.rect)
            .unwrap_or_default();
        let mut base_pixels = load_atlas_pixels(width, height);
        if let Some(color) = ATLAS_SETTINGS
            .get()
            .and_then(|settings| settings.debug_fill_rgba)
        {
            fill_rect_rgba(&mut base_pixels, width, height, rect, color);
            append_log(&format!(
                "dx12 title texture probe: applied debug fill rgba=({}, {}, {}, {}) to rect=({}, {}, {}, {})",
                color[0], color[1], color[2], color[3], rect.x, rect.y, rect.width, rect.height
            ));
        }

        let mut texture = Self {
            device_ptr: device.as_raw() as usize,
            device,
            resource,
            command_queue,
            command_allocator,
            command_list,
            fence,
            fence_event,
            fence_value: 1,
            width,
            height,
            state: D3D12_RESOURCE_STATE_COPY_DEST,
        };
        texture.upload_rgba(&base_pixels)?;
        append_log(&format!(
            "dx12 title texture probe: RGBA atlas texture ready {}x{} rect=({}, {}, {}, {})",
            width, height, rect.x, rect.y, rect.width, rect.height
        ));
        Ok(texture)
    }

    fn srv_desc(&self) -> D3D12_SHADER_RESOURCE_VIEW_DESC {
        D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                    PlaneSlice: 0,
                    ResourceMinLODClamp: 0.0,
                },
            },
        }
    }

    fn upload_rgba(&mut self, data: &[u8]) -> Result<()> {
        self.upload_rgba_rect(data, 0, 0, self.width, self.height)
    }

    fn upload_rgba_rect(
        &mut self,
        data: &[u8],
        dst_x: u32,
        dst_y: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        let upload_row_size = width * 4;
        let upload_pitch = upload_row_size.div_ceil(D3D12_TEXTURE_DATA_PITCH_ALIGNMENT)
            * D3D12_TEXTURE_DATA_PITCH_ALIGNMENT;
        let upload_size = height * upload_pitch;

        let mut upload_buffer = None;
        unsafe {
            self.device.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_UPLOAD,
                    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                    CreationNodeMask: 0,
                    VisibleNodeMask: 0,
                },
                D3D12_HEAP_FLAG_NONE,
                &D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                    Alignment: 0,
                    Width: upload_size as u64,
                    Height: 1,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    Format: DXGI_FORMAT_UNKNOWN,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                    Flags: D3D12_RESOURCE_FLAG_NONE,
                },
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &mut upload_buffer,
            )?;
        }
        let upload_buffer: ID3D12Resource =
            upload_buffer.expect("CreateCommittedResource returned success without upload buffer");

        unsafe {
            let mut upload_buffer_ptr = ptr::null_mut();
            upload_buffer.Map(0, None, Some(&mut upload_buffer_ptr))?;
            if upload_row_size == upload_pitch {
                ptr::copy_nonoverlapping(data.as_ptr(), upload_buffer_ptr as *mut u8, data.len());
            } else {
                for y in 0..height {
                    let src = data.as_ptr().add((y * upload_row_size) as usize);
                    let dst = (upload_buffer_ptr as *mut u8).add((y * upload_pitch) as usize);
                    ptr::copy_nonoverlapping(src, dst, upload_row_size as usize);
                }
            }
            upload_buffer.Unmap(0, None);

            self.command_allocator.Reset()?;
            self.command_list.Reset(&self.command_allocator, None)?;

            let barrier_to_copy = if self.state != D3D12_RESOURCE_STATE_COPY_DEST {
                Some(create_barrier(
                    &self.resource,
                    self.state,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                ))
            } else {
                None
            };
            if let Some(barrier) = barrier_to_copy.as_ref() {
                self.command_list
                    .ResourceBarrier(std::slice::from_ref(barrier));
            }

            let dst_location = D3D12_TEXTURE_COPY_LOCATION {
                pResource: ManuallyDrop::new(Some(self.resource.clone())),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };
            let src_location = D3D12_TEXTURE_COPY_LOCATION {
                pResource: ManuallyDrop::new(Some(upload_buffer.clone())),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: 0,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                            Width: width,
                            Height: height,
                            Depth: 1,
                            RowPitch: upload_pitch,
                        },
                    },
                },
            };

            self.command_list.CopyTextureRegion(
                &dst_location,
                dst_x,
                dst_y,
                0,
                &src_location,
                None,
            );
            let barrier_to_srv = create_barrier(
                &self.resource,
                D3D12_RESOURCE_STATE_COPY_DEST,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            );
            self.command_list
                .ResourceBarrier(std::slice::from_ref(&barrier_to_srv));
            self.command_list.Close()?;
            self.command_queue
                .ExecuteCommandLists(&[Some(self.command_list.cast::<ID3D12CommandList>()?)]);
            self.command_queue.Signal(&self.fence, self.fence_value)?;
            self.fence
                .SetEventOnCompletion(self.fence_value, self.fence_event)?;
            if WaitForSingleObject(self.fence_event, INFINITE) != WAIT_OBJECT_0 {
                return Err(windows::core::Error::from_win32());
            }
            self.fence_value += 1;
            if let Some(barrier) = barrier_to_copy {
                drop_barrier(barrier);
            }
            drop_barrier(barrier_to_srv);
            self.state = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
        }

        Ok(())
    }
}

fn load_atlas_pixels(width: u32, height: u32) -> Vec<u8> {
    let expected_len = (width * height * 4) as usize;
    let Some(path) = ATLAS_SETTINGS
        .get()
        .and_then(|settings| settings.rgba_path.as_ref())
    else {
        append_log("dx12 title texture probe: atlas_rgba not configured, using transparent base");
        return vec![0u8; expected_len];
    };

    match fs::read(path) {
        Ok(data) if data.len() == expected_len => {
            append_log(&format!(
                "dx12 title texture probe: loaded atlas RGBA base {} bytes from {}",
                data.len(),
                path.display()
            ));
            data
        }
        Ok(data) => {
            append_log(&format!(
                "dx12 title texture probe: atlas RGBA size mismatch from {}: got {}, expected {}",
                path.display(),
                data.len(),
                expected_len
            ));
            vec![0u8; expected_len]
        }
        Err(err) => {
            append_log(&format!(
                "dx12 title texture probe: failed to read atlas RGBA {}: {err:?}",
                path.display()
            ));
            vec![0u8; expected_len]
        }
    }
}

fn fill_rect_rgba(data: &mut [u8], width: u32, height: u32, rect: AtlasRect, color: [u8; 4]) {
    let x0 = rect.x.min(width);
    let y0 = rect.y.min(height);
    let x1 = rect.x.saturating_add(rect.width).min(width);
    let y1 = rect.y.saturating_add(rect.height).min(height);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for y in y0..y1 {
        let mut i = ((y * width + x0) * 4) as usize;
        for _ in x0..x1 {
            if i + 4 <= data.len() {
                data[i..i + 4].copy_from_slice(&color);
            }
            i += 4;
        }
    }
}

fn create_barrier(
    resource: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: ManuallyDrop::new(Some(resource.clone())),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    }
}

fn drop_barrier(barrier: D3D12_RESOURCE_BARRIER) {
    let transition = ManuallyDrop::into_inner(unsafe { barrier.Anonymous.Transition });
    let _ = ManuallyDrop::into_inner(transition.pResource);
}

fn append_log(message: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}
