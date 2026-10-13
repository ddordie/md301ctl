//! Minimal, dependency-free Windows HID layer for the ASUS ProArt MD301.
//!
//! Talks to the two vendor interfaces of the mouse/dongle:
//!   * control : Usage Page 0xFF01, Usage 1  (64-byte IN/OUT/FEATURE reports)
//!   * event   : Usage Page 0xFFC1, Usage 1  (64-byte IN reports only)
//!
//! Framing (matches what ASUS Gear Link does through WebHID):
//!   write : 65 bytes -> [0x00 report-id byte] + [64-byte payload]
//!   read  : 64 bytes -> payload only

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::c_void;
use std::fmt;
use std::io;

pub type HANDLE = *mut c_void;
pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ: u32 = 1;
const FILE_SHARE_WRITE: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;

const DIGCF_PRESENT: u32 = 0x02;
const DIGCF_DEVICEINTERFACE: u32 = 0x10;

const ERROR_IO_PENDING: u32 = 997;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 258;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub d1: u32,
    pub d2: u16,
    pub d3: u16,
    pub d4: [u8; 8],
}

/// {4d1e55b2-f16f-11cf-88cb-001111000030}  GUID_DEVINTERFACE_HID
const GUID_DEVINTERFACE_HID: GUID = GUID {
    d1: 0x4d1e55b2,
    d2: 0xf16f,
    d3: 0x11cf,
    d4: [0x88, 0xcb, 0x00, 0x11, 0x11, 0x00, 0x00, 0x30],
};

#[repr(C)]
struct SP_DEVICE_INTERFACE_DATA {
    cb_size: u32,
    interface_class_guid: GUID,
    flags: u32,
    reserved: usize,
}

#[repr(C)]
struct SP_DEVINFO_DATA {
    cb_size: u32,
    class_guid: GUID,
    dev_inst: u32,
    reserved: usize,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct HIDD_ATTRIBUTES {
    pub size: u32,
    pub vendor_id: u16,
    pub product_id: u16,
    pub version_number: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HIDP_CAPS {
    pub usage: u16,
    pub usage_page: u16,
    pub input_report_byte_length: u16,
    pub output_report_byte_length: u16,
    pub feature_report_byte_length: u16,
    pub reserved: [u16; 17],
    pub number_link_collection_nodes: u16,
    pub number_input_button_caps: u16,
    pub number_input_value_caps: u16,
    pub number_input_data_indices: u16,
    pub number_output_button_caps: u16,
    pub number_output_value_caps: u16,
    pub number_output_data_indices: u16,
    pub number_feature_button_caps: u16,
    pub number_feature_value_caps: u16,
    pub number_feature_data_indices: u16,
}

impl Default for HIDP_CAPS {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Default)]
struct OVERLAPPED {
    internal: usize,
    internal_high: usize,
    union: [u8; 8],
    h_event: HANDLE,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *mut c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE;
    fn ReadFile(h: HANDLE, buf: *mut u8, to_read: u32, read: *mut u32, ov: *mut OVERLAPPED) -> i32;
    fn WriteFile(h: HANDLE, buf: *const u8, to_write: u32, written: *mut u32, ov: *mut OVERLAPPED) -> i32;
    fn CloseHandle(h: HANDLE) -> i32;
    fn CreateEventW(attrs: *mut c_void, manual_reset: i32, initial: i32, name: *const u16) -> HANDLE;
    fn ResetEvent(h: HANDLE) -> i32;
    fn WaitForSingleObject(h: HANDLE, ms: u32) -> u32;
    fn GetOverlappedResult(h: HANDLE, ov: *mut OVERLAPPED, transferred: *mut u32, wait: i32) -> i32;
    fn CancelIoEx(h: HANDLE, ov: *mut OVERLAPPED) -> i32;
    fn GetLastError() -> u32;
}

#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetAttributes(h: HANDLE, attrs: *mut HIDD_ATTRIBUTES) -> i32;
    fn HidD_GetPreparsedData(h: HANDLE, preparsed: *mut *mut c_void) -> i32;
    fn HidD_FreePreparsedData(preparsed: *mut c_void) -> i32;
    fn HidP_GetCaps(preparsed: *mut c_void, caps: *mut HIDP_CAPS) -> i32;
    fn HidD_GetProductString(h: HANDLE, buf: *mut u16, len: u32) -> i32;
    fn HidD_GetManufacturerString(h: HANDLE, buf: *mut u16, len: u32) -> i32;
}

#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const GUID,
        enumerator: *const u16,
        hwnd: HANDLE,
        flags: u32,
    ) -> HANDLE;
    fn SetupDiEnumDeviceInterfaces(
        set: HANDLE,
        info: *mut SP_DEVINFO_DATA,
        guid: *const GUID,
        index: u32,
        data: *mut SP_DEVICE_INTERFACE_DATA,
    ) -> i32;
    fn SetupDiGetDeviceInterfaceDetailW(
        set: HANDLE,
        data: *mut SP_DEVICE_INTERFACE_DATA,
        detail: *mut u8,
        detail_size: u32,
        required: *mut u32,
        info: *mut SP_DEVINFO_DATA,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(set: HANDLE) -> i32;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn last_error() -> u32 {
    unsafe { GetLastError() }
}

pub fn is_asus_vendor(vid: u16) -> bool {
    vid == 0x0B05
}

/// An interface (top-level HID collection) exposed by an ASUS device.
#[derive(Clone, Debug)]
pub struct Interface {
    pub path: String,
    pub vid: u16,
    pub pid: u16,
    pub version: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub input_len: u16,
    pub output_len: u16,
    pub feature_len: u16,
    pub product: String,
    pub manufacturer: String,
}

impl Interface {
    pub fn tag(&self) -> String {
        // e.g. "VID_0B05&PID_1CD3&MI_03"
        let mut tag = format!("VID_{:04X}&PID_{:04X}", self.vid, self.pid);
        if let Some(rest) = self.path.split("&MI_").nth(1) {
            let mi = rest.split('#').next().unwrap_or("");
            tag.push_str(&format!("&MI_{}", mi));
        }
        tag
    }
}

/// Enumerate every HID interface of every ASUS (VID 0x0B05) device that is present.
pub fn enumerate_asus() -> io::Result<Vec<Interface>> {
    let mut out = Vec::new();
    unsafe {
        let set = SetupDiGetClassDevsW(&GUID_DEVINTERFACE_HID, std::ptr::null(), std::ptr::null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
        if set == INVALID_HANDLE_VALUE {
            return Err(io::Error::from_raw_os_error(last_error() as i32));
        }
        let mut index = 0u32;
        loop {
            let mut did = SP_DEVICE_INTERFACE_DATA {
                cb_size: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                interface_class_guid: GUID_DEVINTERFACE_HID,
                flags: 0,
                reserved: 0,
            };
            if SetupDiEnumDeviceInterfaces(set, std::ptr::null_mut(), &GUID_DEVINTERFACE_HID, index, &mut did) == 0 {
                break;
            }
            index += 1;
            let mut required = 0u32;
            SetupDiGetDeviceInterfaceDetailW(set, &mut did, std::ptr::null_mut(), 0, &mut required, std::ptr::null_mut());
            if required == 0 {
                continue;
            }
            let mut buf = vec![0u8; required as usize];
            // cbSize of SP_DEVICE_INTERFACE_DETAIL_DATA_W: 8 on 64-bit, 6 on 32-bit
            let cb = if std::mem::size_of::<usize>() == 8 { 8u32 } else { 6u32 };
            std::ptr::copy_nonoverlapping(cb.to_le_bytes().as_ptr(), buf.as_mut_ptr(), 4);
            if SetupDiGetDeviceInterfaceDetailW(set, &mut did, buf.as_mut_ptr(), required, &mut required, std::ptr::null_mut()) == 0 {
                continue;
            }
            // path is a NUL-terminated UTF-16 string at offset 4
            let path_units: Vec<u16> = buf[4..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            let path = String::from_utf16_lossy(&path_units);
            if let Some(iface) = probe_interface(&path) {
                if is_asus_vendor(iface.vid) {
                    out.push(iface);
                }
            }
        }
        SetupDiDestroyDeviceInfoList(set);
    }
    Ok(out)
}

/// Open a path read-only and read its attributes / caps / strings.
fn probe_interface(path: &str) -> Option<Interface> {
    unsafe {
        let wpath = wide(path);
        let h = CreateFileW(
            wpath.as_ptr(),
            0, // no access needed for the metadata queries
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if h == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut attrs = HIDD_ATTRIBUTES { size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32, ..Default::default() };
        let ok = HidD_GetAttributes(h, &mut attrs);
        let mut caps = HIDP_CAPS::default();
        let mut preparsed: *mut c_void = std::ptr::null_mut();
        let mut have_caps = false;
        if HidD_GetPreparsedData(h, &mut preparsed) != 0 {
            if HidP_GetCaps(preparsed, &mut caps) >= 0 {
                have_caps = true;
            }
            HidD_FreePreparsedData(preparsed);
        }
        let mut pbuf = [0u16; 128];
        let mut mbuf = [0u16; 128];
        let product = if HidD_GetProductString(h, pbuf.as_mut_ptr(), (pbuf.len() * 2) as u32) != 0 {
            String::from_utf16_lossy(&pbuf[..pbuf.iter().position(|&c| c == 0).unwrap_or(0)])
        } else {
            String::new()
        };
        let manufacturer = if HidD_GetManufacturerString(h, mbuf.as_mut_ptr(), (mbuf.len() * 2) as u32) != 0 {
            String::from_utf16_lossy(&mbuf[..mbuf.iter().position(|&c| c == 0).unwrap_or(0)])
        } else {
            String::new()
        };
        CloseHandle(h);
        if ok == 0 || !have_caps {
            return None;
        }
        Some(Interface {
            path: path.to_string(),
            vid: attrs.vendor_id,
            pid: attrs.product_id,
            version: attrs.version_number,
            usage_page: caps.usage_page,
            usage: caps.usage,
            input_len: caps.input_report_byte_length,
            output_len: caps.output_report_byte_length,
            feature_len: caps.feature_report_byte_length,
            product,
            manufacturer,
        })
    }
}

/// Open interface for reading + writing with overlapped I/O.
pub struct HidDev {
    handle: HANDLE,
    ov: OVERLAPPED,
    event: HANDLE,
    pub iface: Interface,
}

impl fmt::Debug for HidDev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HidDev({})", self.iface.tag())
    }
}

impl HidDev {
    pub fn open(iface: &Interface) -> io::Result<HidDev> {
        unsafe {
            let wpath = wide(&iface.path);
            let h = CreateFileW(
                wpath.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                return Err(io::Error::from_raw_os_error(last_error() as i32));
            }
            let event = CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null());
            if event == INVALID_HANDLE_VALUE {
                CloseHandle(h);
                return Err(io::Error::from_raw_os_error(last_error() as i32));
            }
            let mut ov = OVERLAPPED::default();
            ov.h_event = event;
            Ok(HidDev { handle: h, ov, event, iface: iface.clone() })
        }
    }

    /// Write one report. Windows/this device expect the report-ID byte to be present.
    pub fn write_report(&mut self, data: &[u8]) -> io::Result<()> {
        unsafe {
            ResetEvent(self.event);
            let mut written = 0u32;
            let ok = WriteFile(self.handle, data.as_ptr(), data.len() as u32, &mut written, &mut self.ov);
            if ok == 0 {
                let err = last_error();
                if err != ERROR_IO_PENDING {
                    return Err(io::Error::from_raw_os_error(err as i32));
                }
                if WaitForSingleObject(self.event, 3000) != WAIT_OBJECT_0 {
                    CancelIoEx(self.handle, &mut self.ov);
                    GetOverlappedResult(self.handle, &mut self.ov, &mut written, 1);
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "write timeout"));
                }
                if GetOverlappedResult(self.handle, &mut self.ov, &mut written, 1) == 0 {
                    return Err(io::Error::from_raw_os_error(last_error() as i32));
                }
            }
            Ok(())
        }
    }

    /// Read one report, waiting at most `timeout_ms`.
    pub fn read_report(&mut self, out: &mut [u8], timeout_ms: u32) -> io::Result<Option<usize>> {
        unsafe {
            ResetEvent(self.event);
            let mut read = 0u32;
            let ok = ReadFile(self.handle, out.as_mut_ptr(), out.len() as u32, &mut read, &mut self.ov);
            if ok != 0 {
                return Ok(Some(read as usize));
            }
            let err = last_error();
            if err != ERROR_IO_PENDING {
                return Err(io::Error::from_raw_os_error(err as i32));
            }
            let w = WaitForSingleObject(self.event, timeout_ms);
            if w == WAIT_TIMEOUT {
                CancelIoEx(self.handle, &mut self.ov);
                GetOverlappedResult(self.handle, &mut self.ov, &mut read, 1);
                return Ok(None);
            }
            if w != WAIT_OBJECT_0 {
                return Ok(None);
            }
            if GetOverlappedResult(self.handle, &mut self.ov, &mut read, 1) == 0 {
                let e = last_error();
                if e == 995 {
                    // ERROR_OPERATION_ABORTED (leftover from a previous cancel)
                    return Ok(None);
                }
                return Err(io::Error::from_raw_os_error(e as i32));
            }
            Ok(Some(read as usize))
        }
    }
}

impl Drop for HidDev {
    fn drop(&mut self) {
        unsafe {
            CancelIoEx(self.handle, &mut self.ov);
            let mut n = 0u32;
            GetOverlappedResult(self.handle, &mut self.ov, &mut n, 1);
            CloseHandle(self.event);
            CloseHandle(self.handle);
        }
    }
}
