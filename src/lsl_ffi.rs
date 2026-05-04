// Direct FFI bindings to liblsl.so
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};

#[repr(C)]
pub struct lsl_streaminfo {
    _private: [u8; 0],
}

#[repr(C)]
pub struct lsl_outlet {
    _private: [u8; 0],
}

#[link(name = "lsl")]
extern "C" {
    fn lsl_create_streaminfo(
        name: *const c_char,
        type_: *const c_char,
        channel_count: c_int,
        nominal_srate: c_double,
        channel_format: c_int,
        source_id: *const c_char,
    ) -> *mut lsl_streaminfo;

    fn lsl_create_outlet(
        info: *mut lsl_streaminfo,
        chunk_size: c_int,
        max_buffered: c_int,
    ) -> *mut lsl_outlet;

    fn lsl_push_sample_i(outlet: *mut lsl_outlet, data: *const c_int);

    fn lsl_destroy_outlet(outlet: *mut lsl_outlet);
    fn lsl_destroy_streaminfo(info: *mut lsl_streaminfo);
}

pub struct LslOutlet {
    outlet: *mut lsl_outlet,
    info: *mut lsl_streaminfo,
}

impl LslOutlet {
    pub fn new(name: &str, stream_type: &str, source_id: &str) -> Result<Self, String> {
        let name_c = CString::new(name).map_err(|e| e.to_string())?;
        let type_c = CString::new(stream_type).map_err(|e| e.to_string())?;
        let source_c = CString::new(source_id).map_err(|e| e.to_string())?;

        unsafe {
            let info = lsl_create_streaminfo(
                name_c.as_ptr(),
                type_c.as_ptr(),
                1,   // channel_count
                0.0, // irregular rate
                4,   // cft_int32 (not 2, which is cft_double64!)
                source_c.as_ptr(),
            );

            if info.is_null() {
                return Err("Failed to create stream info".to_string());
            }

            let outlet = lsl_create_outlet(info, 0, 360);

            if outlet.is_null() {
                lsl_destroy_streaminfo(info);
                return Err("Failed to create outlet".to_string());
            }

            Ok(LslOutlet { outlet, info })
        }
    }

    pub fn push_sample(&self, value: i32) -> Result<(), String> {
        unsafe {
            lsl_push_sample_i(self.outlet, &value as *const c_int);
        }
        Ok(())
    }
}

impl Drop for LslOutlet {
    fn drop(&mut self) {
        unsafe {
            if !self.outlet.is_null() {
                lsl_destroy_outlet(self.outlet);
            }
            if !self.info.is_null() {
                lsl_destroy_streaminfo(self.info);
            }
        }
    }
}

// liblsl's per-outlet `lsl_push_sample_*` C entrypoints are documented as
// thread-safe (see liblsl C API docs and matching Java/JNA bindings used by
// the Android sibling). The two pointers we hold (`outlet`, `info`) are
// effectively final after `LslOutlet::new`. Do NOT add mutable state to
// `LslOutlet` without synchronising it.
unsafe impl Send for LslOutlet {}
unsafe impl Sync for LslOutlet {}
