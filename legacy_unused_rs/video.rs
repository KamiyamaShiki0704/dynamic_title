use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Once;
use std::time::Duration;

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFMediaType, IMFSourceReader, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
    MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE, MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING,
    MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READERF_ENDOFSTREAM,
    MF_SOURCE_READERF_STREAMTICK, MF_VERSION, MFCreateAttributes, MFCreateMediaType,
    MFCreateSourceReaderFromURL, MFMediaType_Video, MFSTARTUP_FULL, MFStartup, MFVideoFormat_RGB32,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::PCWSTR;

static MF_START: Once = Once::new();

pub struct DecodedFrame {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub duration: Duration,
}

pub struct MediaFoundationVideo {
    path: Vec<u16>,
    reader: IMFSourceReader,
    source_width: u32,
    source_height: u32,
    frame_width: u32,
    frame_height: u32,
    cpu_resize: Option<(u32, u32)>,
    frame_duration: Duration,
}

impl MediaFoundationVideo {
    pub fn open(
        path: &Path,
        output_size: Option<(u32, u32)>,
        log_path: Option<&PathBuf>,
    ) -> Result<Self, String> {
        append_log(log_path, "mf open: begin");
        let mut init_result = Ok(());
        MF_START.call_once(|| {
            append_log(log_path, "mf open: CoInitializeEx");
            let coinit = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if coinit.is_err() && coinit != RPC_E_CHANGED_MODE {
                init_result = Err(format!("CoInitializeEx failed: {coinit:?}"));
                return;
            }
            append_log(log_path, "mf open: MFStartup");
            init_result = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }
                .map_err(|err| format!("MFStartup failed: {err}"));
        });
        init_result?;

        let path = path_to_wide(path)?;
        append_log(log_path, "mf open: create reader");
        let reader = create_reader(&path, log_path)?;

        append_log(log_path, "mf open: create media type");
        let output_type = unsafe { MFCreateMediaType() }
            .map_err(|err| format!("MFCreateMediaType failed: {err}"))?;
        set_output_type(&output_type, output_size)?;
        if let Some((width, height)) = output_size {
            append_log(
                log_path,
                &format!("mf open: requested output size {width}x{height}"),
            );
        }
        append_log(log_path, "mf open: set current media type");
        let requested_resize_result = unsafe {
            reader.SetCurrentMediaType(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                None,
                &output_type,
            )
        };
        let cpu_resize = if let Err(err) = requested_resize_result {
            if output_size.is_none() {
                return Err(format!("SetCurrentMediaType failed: {err}"));
            }
            append_log(
                log_path,
                &format!("mf resize output rejected, falling back to CPU resize: {err}"),
            );
            let output_type = unsafe { MFCreateMediaType() }
                .map_err(|err| format!("fallback MFCreateMediaType failed: {err}"))?;
            set_output_type(&output_type, None)?;
            unsafe {
                reader
                    .SetCurrentMediaType(
                        MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                        None,
                        &output_type,
                    )
                    .map_err(|err| format!("fallback SetCurrentMediaType failed: {err}"))?;
            }
            output_size
        } else {
            None
        };

        append_log(log_path, "mf open: get current media type");
        let current_type = unsafe {
            reader
                .GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32)
                .map_err(|err| format!("GetCurrentMediaType failed: {err}"))?
        };
        let (source_width, source_height) =
            get_ratio_attribute(&current_type, &MF_MT_FRAME_SIZE)
                .ok_or_else(|| "missing MF_MT_FRAME_SIZE".to_string())?;
        let (frame_width, frame_height) = cpu_resize.unwrap_or((source_width, source_height));
        let frame_duration = get_ratio_attribute(&current_type, &MF_MT_FRAME_RATE)
            .and_then(|(num, den)| {
                if num == 0 {
                    None
                } else {
                    Some(Duration::from_secs_f64(den as f64 / num as f64))
                }
            })
            .unwrap_or_else(|| Duration::from_secs_f64(1.0 / 30.0));

        Ok(Self {
            path,
            reader,
            source_width,
            source_height,
            frame_width,
            frame_height,
            cpu_resize,
            frame_duration,
        })
    }

    pub fn width(&self) -> u32 {
        self.frame_width
    }

    pub fn height(&self) -> u32 {
        self.frame_height
    }

    pub fn next_frame(&mut self) -> Result<DecodedFrame, String> {
        loop {
            let mut flags = 0u32;
            let mut sample = None;
            unsafe {
                self.reader
                    .ReadSample(
                        MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                        0,
                        None,
                        Some(&mut flags),
                        None,
                        Some(&mut sample),
                    )
                    .map_err(|err| format!("ReadSample failed: {err}"))?;
            }

            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                self.restart()?;
                continue;
            }
            if flags & MF_SOURCE_READERF_STREAMTICK.0 as u32 != 0 {
                continue;
            }

            let Some(sample) = sample else {
                continue;
            };
            let buffer = unsafe {
                sample
                    .ConvertToContiguousBuffer()
                    .map_err(|err| format!("ConvertToContiguousBuffer failed: {err}"))?
            };

            let mut data_ptr = ptr::null_mut();
            let mut current_len = 0u32;
            unsafe {
                buffer
                    .Lock(&mut data_ptr, None, Some(&mut current_len))
                    .map_err(|err| format!("buffer lock failed: {err}"))?;
            }

            let source_len = (self.source_width as usize) * (self.source_height as usize) * 4;
            let copy_len = source_len.min(current_len as usize);
            let frame_len = (self.frame_width as usize) * (self.frame_height as usize) * 4;
            let mut rgba = vec![0u8; frame_len];
            unsafe {
                let src = std::slice::from_raw_parts(data_ptr, copy_len);
                if self.cpu_resize.is_some() {
                    resize_bgra_to_rgba(
                        src,
                        self.source_width,
                        self.source_height,
                        &mut rgba,
                        self.frame_width,
                        self.frame_height,
                    );
                } else {
                    bgra_to_rgba(src, &mut rgba[..copy_len]);
                }
                buffer
                    .Unlock()
                    .map_err(|err| format!("buffer unlock failed: {err}"))?;
            }

            return Ok(DecodedFrame {
                rgba,
                width: self.frame_width,
                height: self.frame_height,
                duration: self.frame_duration,
            });
        }
    }

    fn restart(&mut self) -> Result<(), String> {
        self.reader = create_reader(&self.path, None).map_err(|err| format!("loop {err}"))?;
        let output_type = unsafe { MFCreateMediaType() }
            .map_err(|err| format!("loop MFCreateMediaType failed: {err}"))?;
        set_output_type(&output_type, None)?;
        unsafe {
            self.reader
                .SetCurrentMediaType(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    None,
                    &output_type,
                )
                .map_err(|err| format!("loop SetCurrentMediaType failed: {err}"))?;
        }
        Ok(())
    }
}

fn create_reader(path: &[u16], log_path: Option<&PathBuf>) -> Result<IMFSourceReader, String> {
    let mut attributes = None;
    append_log(log_path, "mf create reader: attributes");
    unsafe {
        MFCreateAttributes(&mut attributes, 2)
            .map_err(|err| format!("MFCreateAttributes failed: {err}"))?;
    }
    let attributes = attributes.ok_or_else(|| "MFCreateAttributes returned null".to_string())?;
    set_reader_attributes(&attributes)?;
    append_log(log_path, "mf create reader: from url");
    unsafe { MFCreateSourceReaderFromURL(PCWSTR(path.as_ptr()), &attributes) }
        .map_err(|err| format!("MFCreateSourceReaderFromURL failed: {err}"))
}

fn set_reader_attributes(attributes: &IMFAttributes) -> Result<(), String> {
    unsafe {
        attributes
            .SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)
            .map_err(|err| format!("enable video processing failed: {err}"))?;
    }
    Ok(())
}

fn append_log(path: Option<&PathBuf>, message: &str) {
    let Some(path) = path else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn set_output_type(
    output_type: &IMFMediaType,
    output_size: Option<(u32, u32)>,
) -> Result<(), String> {
    unsafe {
        output_type
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(|err| format!("SetGUID major type failed: {err}"))?;
        output_type
            .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
            .map_err(|err| format!("SetGUID subtype failed: {err}"))?;
        if let Some((width, height)) = output_size {
            output_type
                .SetUINT64(&MF_MT_FRAME_SIZE, pack_ratio(width, height))
                .map_err(|err| format!("SetUINT64 frame size failed: {err}"))?;
        }
    }
    Ok(())
}

fn pack_ratio(numerator: u32, denominator: u32) -> u64 {
    ((numerator as u64) << 32) | denominator as u64
}

fn get_ratio_attribute(media_type: &IMFMediaType, key: &windows::core::GUID) -> Option<(u32, u32)> {
    let value = unsafe { media_type.GetUINT64(key).ok()? };
    Some(((value >> 32) as u32, value as u32))
}

fn bgra_to_rgba(src: &[u8], dst: &mut [u8]) {
    for (src, dst) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = 255;
    }
}

fn resize_bgra_to_rgba(src: &[u8], src_w: u32, src_h: u32, dst: &mut [u8], dst_w: u32, dst_h: u32) {
    let src_w = src_w as usize;
    let src_h = src_h as usize;
    let dst_w = dst_w as usize;
    let dst_h = dst_h as usize;
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return;
    }

    for y in 0..dst_h {
        let src_y = y * src_h / dst_h;
        for x in 0..dst_w {
            let src_x = x * src_w / dst_w;
            let src_i = (src_y * src_w + src_x) * 4;
            let dst_i = (y * dst_w + x) * 4;
            if src_i + 3 >= src.len() || dst_i + 3 >= dst.len() {
                continue;
            }
            dst[dst_i] = src[src_i + 2];
            dst[dst_i + 1] = src[src_i + 1];
            dst[dst_i + 2] = src[src_i];
            dst[dst_i + 3] = 255;
        }
    }
}

fn path_to_wide(path: &Path) -> Result<Vec<u16>, String> {
    use std::os::windows::ffi::OsStrExt;

    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.is_empty() {
        return Err("video path is empty".to_string());
    }
    wide.push(0);
    Ok(wide)
}
