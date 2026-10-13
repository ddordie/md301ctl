//! 零依赖 Win32 GUI：直接手写 FFI 调用 user32 / gdi32 / comctl32，
//! 不引入任何第三方 crate，保证离线可编译、单文件 exe。
#![allow(non_snake_case, non_camel_case_types, clippy::too_many_lines)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use std::io;
use std::ptr::null_mut;

use crate::proto::{
    func_name, response_ms, Md301, BUTTON_RESPONSES, DPI_LEVELS, DPI_MAX, DPI_MIN, FUNC_TABLE,
    POLLING_RATES,
};

// ------------------------------------------------------------------ FFI 类型
type HWND = *mut c_void;
type HINSTANCE = *mut c_void;
type HGDIOBJ = *mut c_void;
type HBRUSH = *mut c_void;
type HCURSOR = *mut c_void;
type HICON = *mut c_void;
type HMENU = *mut c_void;
type HDC = *mut c_void;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
struct WNDCLASSW {
    style: u32,
    lpfnWndProc: Option<WndProc>,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HICON,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    wParam: WPARAM,
    lParam: LPARAM,
    time: u32,
    pt: [i32; 2],
    lPrivate: u32,
}

#[repr(C)]
struct INITCOMMONCONTROLSEX {
    dwSize: u32,
    dwICC: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(c: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: HMENU,
        inst: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT;
    fn GetMessageW(m: *mut MSG, h: HWND, min: u32, max: u32) -> i32;
    fn TranslateMessage(m: *const MSG) -> i32;
    fn DispatchMessageW(m: *const MSG) -> LRESULT;
    fn PostQuitMessage(code: i32);
    fn ShowWindow(h: HWND, cmd: i32) -> i32;
    fn UpdateWindow(h: HWND) -> i32;
    fn SendMessageW(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT;
    fn SetWindowTextW(h: HWND, s: *const u16) -> i32;
    fn MessageBoxW(h: HWND, text: *const u16, cap: *const u16, t: u32) -> i32;
    fn LoadCursorW(inst: HINSTANCE, name: *const u16) -> HCURSOR;
    fn GetDC(h: HWND) -> HDC;
    fn ReleaseDC(h: HWND, dc: HDC) -> i32;
    fn SetProcessDPIAware() -> i32;
    fn DestroyWindow(h: HWND) -> i32;
    fn GetSystemMetrics(i: i32) -> i32;
    fn SetWindowLongPtrW(h: HWND, i: i32, v: isize) -> isize;
    fn GetWindowLongPtrW(h: HWND, i: i32) -> isize;
    fn FreeConsole() -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateFontW(
        h: i32,
        w: i32,
        esc: i32,
        orient: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        prec: u32,
        clip: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> HGDIOBJ;
    fn GetDeviceCaps(dc: HDC, index: i32) -> i32;
}

#[link(name = "comctl32")]
unsafe extern "system" {
    fn InitCommonControlsEx(p: *const INITCOMMONCONTROLSEX) -> i32;
}

// ------------------------------------------------------------------- 常量
const WM_CREATE: u32 = 0x0001;
const WM_DESTROY: u32 = 0x0002;
const WM_CLOSE: u32 = 0x0010;
const WM_COMMAND: u32 = 0x0111;
const WM_HSCROLL: u32 = 0x0114;
const WM_SETFONT: u32 = 0x0030;

const WS_OVERLAPPEDWINDOW_FIXED: u32 = 0x00CA_0000; // 有标题栏/关闭/最小化，不可缩放
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_GROUP: u32 = 0x0002_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_VSCROLL: u32 = 0x0020_0000;

const BS_PUSHBUTTON: u32 = 0x0000_0000;
const BS_DEFPUSHBUTTON: u32 = 0x0000_0001;
const BS_AUTORADIOBUTTON: u32 = 0x0000_0009;
const BS_GROUPBOX: u32 = 0x0000_0007;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const SS_LEFT: u32 = 0x0000_0000;
const TBS_HORZ: u32 = 0x0000;
const TBS_NOTICKS: u32 = 0x0010;

const BM_SETCHECK: u32 = 0x00F1;
const CB_ADDSTRING: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;
const CB_RESETCONTENT: u32 = 0x014B;
const TBM_SETRANGEMIN: u32 = 0x0407;
const TBM_SETRANGEMAX: u32 = 0x0408;
const TBM_SETPOS: u32 = 0x0405;
const TBM_GETPOS: u32 = 0x0400;
const TBM_SETLINESIZE: u32 = 0x0403;
const TBM_SETPAGESIZE: u32 = 0x0401;
const TB_ENDTRACK: usize = 8;

const GWLP_USERDATA: i32 = -21;
const MB_YESNO: u32 = 0x0000_0004;
const MB_ICONWARNING: u32 = 0x0000_0030;
const IDYES: i32 = 6;
const SW_SHOW: i32 = 5;

const DPI_STEP: u32 = 50;

// 控件 ID
const ID_INFO: i32 = 1000;
const ID_DPI_APPLY: i32 = 1060;
const ID_RELOAD: i32 = 1061;
const ID_DPI_COUNT: i32 = 1040;
const ID_POLLING: i32 = 1050;
const ID_RESPONSE: i32 = 1051;
const ID_PROFILE: i32 = 1070;
const ID_PROFILE_SET: i32 = 1071;
const ID_SAVE: i32 = 1072;
const ID_RESET: i32 = 1073;
const ID_BTN_APPLY: i32 = 1090;
const ID_STATUS: i32 = 1100;
const ID_SLIDER0: i32 = 1010;
const ID_VALUE0: i32 = 1020;
const ID_RADIO0: i32 = 1030;
const ID_BTN0: i32 = 1080;

/// 8 个可控按键的物理功能码（顺序与 KeyMap::entries() 一致）
const BTN_CODES: [u16; 8] = [0x1F0, 0x1F1, 0x1F2, 0x1E4, 0x1E5, 0x1E6, 0x1E8, 0x1E9];

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct State {
    dev: Option<Md301>,
    info: HWND,
    status: HWND,
    sliders: [HWND; 4],
    values: [HWND; 4],
    radios: [HWND; 4],
    dpi_count: HWND,
    polling: HWND,
    response: HWND,
    profile: HWND,
    buttons: [HWND; 8],
    func_list: Vec<(u16, String)>,
    /// 刷新控件期间置位，避免程序化的下拉框变更被当成用户操作而触发写入
    loading: bool,
    /// 设备上"最后一次读到"的值：只有选中的值与之不同才写，避免把界面里的旧值
    /// 在同值/虚假通知时写回设备（实测：展开下拉框也会送来 WM_COMMAND）。
    last_polling: u8,
    last_response: u16,
}

unsafe fn set_text(h: HWND, s: &str) {
    let w = wide(s);
    SetWindowTextW(h, w.as_ptr());
}

unsafe fn ctl(parent: HWND, class: &str, text: &str, style: u32, r: [i32; 4], id: i32) -> HWND {
    let c = wide(class);
    let t = wide(text);
    CreateWindowExW(
        0,
        c.as_ptr(),
        t.as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        r[0],
        r[1],
        r[2],
        r[3],
        parent,
        id as HMENU,
        null_mut(),
        null_mut(),
    )
}

unsafe fn add_combo_items(h: HWND, items: &[String]) {
    for it in items {
        let w = wide(it);
        SendMessageW(h, CB_ADDSTRING, 0, w.as_ptr() as LPARAM);
    }
}

fn scale() -> f32 {
    unsafe {
        let dc = GetDC(null_mut());
        if dc.is_null() {
            return 1.0;
        }
        let dpi = GetDeviceCaps(dc, 88); // LOGPIXELSX
        ReleaseDC(null_mut(), dc);
        if dpi <= 0 { 1.0 } else { dpi as f32 / 96.0 }
    }
}

fn px(k: f32, v: i32) -> i32 {
    (v as f32 * k) as i32
}

/// 让 `md301ctl gui` 不带控制台窗口
pub fn run() -> io::Result<()> {
    unsafe {
        SetProcessDPIAware();
        FreeConsole();
        let icc = INITCOMMONCONTROLSEX { dwSize: 8, dwICC: 0x4004 }; // STANDARD | BAR_CLASSES
        InitCommonControlsEx(&icc);

        let k = scale();
        let inst = GetModuleHandleW_();
        let cls = wide("Md301CtlGuiWnd");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), 32512 as *const u16), // IDC_ARROW
            hbrBackground: 5 as HBRUSH,                            // COLOR_WINDOW+1
            lpszMenuName: null_mut(),
            lpszClassName: cls.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "RegisterClassW 失败"));
        }

        let title = wide("ASUS ProArt MD301 离线配置");
        let (cw, ch) = (px(k, 566), px(k, 672));
        let hwnd = CreateWindowExW(
            0,
            cls.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW_FIXED,
            (GetSystemMetrics(0) - cw) / 2,
            (GetSystemMetrics(1) - ch) / 3,
            cw + px(k, 8),
            ch + px(k, 30),
            null_mut(),
            null_mut(),
            inst,
            null_mut(),
        );
        if hwnd.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "CreateWindowExW 失败"));
        }
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

unsafe fn GetModuleHandleW_() -> HINSTANCE {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    }
    GetModuleHandleW(null_mut())
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let k = scale();
            let st = Box::into_raw(Box::new(build_ui(hwnd, k)));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, st as isize);
            refresh(hwnd, &mut *st);
            0
        }
        WM_COMMAND => {
            let id = (wp & 0xFFFF) as i32;
            // 通知码在高 16 位：按钮 BN_CLICKED=0、下拉框 CBN_SELCHANGE=1。
            // 必须区分，否则"展开下拉框"这类通知会被当成"用户改了设置"。
            let code = ((wp >> 16) & 0xFFFF) as u32; // HIWORD(wParam) = 通知码
            let st = state(hwnd);
            if st.is_null() {
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            let st = &mut *st;
            // 刷新时程序化地重建下拉框也会送来 WM_COMMAND：绝不能当成用户改设置
            if !st.loading {
                on_command(hwnd, st, id, code);
            }
            0
        }
        WM_HSCROLL => {
            let st = state(hwnd);
            if !st.is_null() {
                let st = &mut *st;
                let slider = lp as HWND;
                for i in 0..4 {
                    if st.sliders[i] == slider {
                        let raw = SendMessageW(slider, TBM_GETPOS, 0, 0) as u32;
                        let snapped = snap(raw);
                        if snapped != raw {
                            SendMessageW(slider, TBM_SETPOS, 1, snapped as LPARAM);
                        }
                        set_text(st.values[i], &format!("{}", snapped));
                        if (wp & 0xFFFF) as usize == TB_ENDTRACK {
                            set_text(st.status, "已修改 DPI，点[应用 DPI] 写入鼠标");
                        }
                        break;
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let st = state(hwnd);
            if !st.is_null() {
                drop(Box::from_raw(st));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn state(hwnd: HWND) -> *mut State {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State
}

fn snap(v: u32) -> u32 {
    let v = v.clamp(DPI_MIN, DPI_MAX);
    ((v + DPI_STEP / 2) / DPI_STEP * DPI_STEP).clamp(DPI_MIN, DPI_MAX)
}

unsafe fn build_ui(hwnd: HWND, k: f32) -> State {
    let p = |v: i32| px(k, v);
    let font_face = wide("Microsoft YaHei UI");
    let font = CreateFontW(
        p(-14),
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        134, // GB2312_CHARSET
        0,
        0,
        5, // CLEARTYPE_QUALITY
        0,
        font_face.as_ptr(),
    );

    macro_rules! mk {
        ($class:expr, $text:expr, $style:expr, $r:expr, $id:expr) => {{
            let h = ctl(hwnd, $class, $text, $style, [p($r[0]), p($r[1]), p($r[2]), p($r[3])], $id);
            SendMessageW(h, WM_SETFONT, font as WPARAM, 1);
            h
        }};
    }

    // ---- 分组框（先建，保证在底层）
    mk!("BUTTON", "设备状态", BS_GROUPBOX, [8, 6, 550, 60], -1);
    mk!("BUTTON", "DPI 设置", BS_GROUPBOX, [8, 72, 550, 176], -2);
    mk!("BUTTON", "回报率 / 按键响应", BS_GROUPBOX, [8, 254, 550, 84], -3);
    mk!("BUTTON", "按键映射", BS_GROUPBOX, [8, 344, 550, 232], -4);
    mk!("BUTTON", "配置档", BS_GROUPBOX, [8, 586, 550, 54], -5);

    let info = mk!("STATIC", "", SS_LEFT, [20, 26, 526, 34], ID_INFO);

    let mut sliders = [null_mut(); 4];
    let mut values = [null_mut(); 4];
    let mut radios = [null_mut(); 4];
    // 官方 caps: maxDpiCount 3（min == max，固定 3 档）→ 只建 3 行
    for i in 0..DPI_LEVELS {
        let y = 92 + i as i32 * 28;
        mk!(
            "STATIC",
            &format!("档 {}", i + 1),
            SS_LEFT,
            [20, y + 4, 40, 20],
            -1
        );
        let sl = mk!(
            "msctls_trackbar32",
            "",
            TBS_HORZ | TBS_NOTICKS,
            [62, y, 292, 26],
            ID_SLIDER0 + i as i32
        );
        SendMessageW(sl, TBM_SETRANGEMIN, 0, DPI_MIN as LPARAM);
        SendMessageW(sl, TBM_SETRANGEMAX, 1, DPI_MAX as LPARAM);
        SendMessageW(sl, TBM_SETLINESIZE, 0, DPI_STEP as LPARAM);
        SendMessageW(sl, TBM_SETPAGESIZE, 0, (DPI_STEP * 5) as LPARAM);
        sliders[i] = sl;
        values[i] = mk!("STATIC", "0", SS_LEFT, [360, y + 4, 60, 20], ID_VALUE0 + i as i32);
        let radio_style = if i == 0 {
            BS_AUTORADIOBUTTON | WS_TABSTOP | WS_GROUP
        } else {
            BS_AUTORADIOBUTTON | WS_TABSTOP
        };
        radios[i] = mk!(
            "BUTTON",
            "当前档",
            radio_style,
            [428, y + 2, 90, 22],
            ID_RADIO0 + i as i32
        );
    }
    mk!("STATIC", "档位数", SS_LEFT, [20, 180, 50, 20], -1);
    let dpi_count = mk!(
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP | WS_GROUP,
        [74, 176, 116, 200],
        ID_DPI_COUNT
    );
    mk!("BUTTON", "应用 DPI", BS_PUSHBUTTON | WS_TABSTOP, [202, 176, 96, 26], ID_DPI_APPLY);
    mk!("BUTTON", "重新读取", BS_PUSHBUTTON | WS_TABSTOP, [308, 176, 96, 26], ID_RELOAD);

    mk!("STATIC", "按键响应", SS_LEFT, [20, 302, 68, 20], -1);
    let response = mk!(
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP,
        [92, 298, 130, 240],
        ID_RESPONSE
    );
    mk!("STATIC", "USB/RF", SS_LEFT, [20, 274, 56, 20], -1);
    let polling = mk!(
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP,
        [82, 270, 130, 240],
        ID_POLLING
    );

    let mut buttons = [null_mut(); 8];
    let names = ["左键", "右键", "中键", "左侧后键", "左侧前键", "DPI 切换键", "滚轮上滚", "滚轮下滚"];
    for i in 0..8 {
        let y = 362 + i as i32 * 24;
        mk!("STATIC", names[i], SS_LEFT, [20, y + 4, 84, 20], -1);
        buttons[i] = mk!(
            "COMBOBOX",
            "",
            CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP,
            [108, y, 190, 300],
            ID_BTN0 + i as i32
        );
    }
    mk!(
        "BUTTON",
        "应用按键映射",
        BS_DEFPUSHBUTTON | WS_TABSTOP,
        [320, 530, 150, 26],
        ID_BTN_APPLY
    );

    let profile = mk!(
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP | WS_GROUP,
        [20, 604, 110, 200],
        ID_PROFILE
    );
    mk!("BUTTON", "切换", BS_PUSHBUTTON | WS_TABSTOP, [142, 604, 80, 26], ID_PROFILE_SET);
    mk!("BUTTON", "保存到 Flash", BS_PUSHBUTTON | WS_TABSTOP, [230, 604, 130, 26], ID_SAVE);
    mk!("BUTTON", "恢复出厂", BS_PUSHBUTTON | WS_TABSTOP, [368, 604, 100, 26], ID_RESET);

    let status = mk!("STATIC", "", SS_LEFT, [16, 646, 540, 20], ID_STATUS);

    let func_list: Vec<(u16, String)> = FUNC_TABLE
        .iter()
        .map(|(code, _, _)| (*code, format!("{:04X} {}", code, func_name(*code))))
        .collect();

    State {
        dev: None,
        last_polling: 0xFF,
        last_response: 0xFFFF,
        info,
        status,
        sliders,
        values,
        radios,
        dpi_count,
        polling,
        response,
        profile,
        buttons,
        func_list,
        loading: false,
    }
}

unsafe fn ensure_dev(st: &mut State) -> io::Result<*mut Md301> {
    if st.dev.is_none() {
        st.dev = Some(Md301::open()?);
    }
    Ok(st.dev.as_mut().map(|d| d as *mut Md301).unwrap())
}

/// 读取设备并把所有控件刷新到真实状态
unsafe fn refresh(hwnd: HWND, st: &mut State) {
    st.loading = true;
    refresh_inner(hwnd, st);
    st.loading = false;
}

unsafe fn refresh_inner(hwnd: HWND, st: &mut State) {
    set_text(st.status, "正在读取设备…（若鼠标休眠，请先移动一下鼠标）");
    UpdateWindow(hwnd);

    let dp = match ensure_dev(st) {
        Ok(p) => p,
        Err(e) => {
            set_text(
                st.status,
                &format!("未连接设备：{}。请确认鼠标已开机/接收器已插好，再点[重新读取]。", e),
            );
            return;
        }
    };

    let info = (*dp).info();
    let power = (*dp).power();
    let perf = (*dp).performance();
    let keys = (*dp).key_map();

    match (&info, &power, &perf, &keys) {
        (Ok(i), Ok(pw), Ok(pf), Ok(km)) => {
            set_text(
                st.info,
                &format!(
                    "鼠标固件 {} · 接收器 {} · 配置档 {}/{}\r\n电量 {}%{} · 低电量提醒 {}% · 休眠 {}",
                    i.version,
                    i.dongle_version,
                    i.current_profile,
                    i.profile_count,
                    pw.battery_percent,
                    if pw.is_charging { " · 充电中" } else { "" },
                    pw.low_power_percent,
                    crate::sleep_label(pw.sleep_time_index),
                ),
            );
            for s in 0..DPI_LEVELS {
                let v = pf.dpi[s].clamp(DPI_MIN, DPI_MAX);
                SendMessageW(st.sliders[s], TBM_SETPOS, 1, v as LPARAM);
                set_text(st.values[s], &format!("{}", v));
                SendMessageW(
                    st.radios[s],
                    BM_SETCHECK,
                    if i.dpi_index == (s as u8 + 1) { 1 } else { 0 },
                    0,
                );
            }
            fill_dpi_count(st, i.dpi_levels);
            st.last_polling = pf.polling_usbrf;
            st.last_response = pf.button_response;
            fill_polling(st, pf.polling_usbrf);
            fill_response(st, pf.button_response);
            fill_profiles(st, i.profile_count, i.current_profile);

            // 按键：下拉框列出全部功能码，选中当前值
            let entries = km.entries();
            for (idx, (_name, code)) in entries.iter().enumerate() {
                SendMessageW(st.buttons[idx], CB_RESETCONTENT, 0, 0);
                let mut sel = 0i32;
                for (i, (c, _label)) in st.func_list.iter().enumerate() {
                    if *c == *code {
                        sel = i as i32;
                    }
                }
                let items: Vec<String> = st.func_list.iter().map(|(_, l)| l.clone()).collect();
                add_combo_items(st.buttons[idx], &items);
                SendMessageW(st.buttons[idx], CB_SETCURSEL, sel as WPARAM, 0);
            }
            set_text(
                st.status,
                &format!("已连接 ASUS ProArt MD301 · 固件 {} · 电量 {}%", i.version, pw.battery_percent),
            );
        }
        _ => {
            let e = info
                .err()
                .or(power.err())
                .or(perf.err())
                .or(keys.err())
                .map(|e| e.to_string())
                .unwrap_or_default();
            set_text(st.status, &format!("读取失败：{}", e));
        }
    }
}

unsafe fn fill_dpi_count(st: &State, levels: u8) {
    // 官方 caps 里 minDpiCount == maxDpiCount == 3：这个型号档位数固定，
    // 官方 UI 也不给改，所以这里只作展示（设备不上报该字段，r[18] 恒为 0）。
    let _ = levels;
    SendMessageW(st.dpi_count, CB_RESETCONTENT, 0, 0);
    add_combo_items(st.dpi_count, &[format!("{} 档（固定）", DPI_LEVELS)]);
    SendMessageW(st.dpi_count, CB_SETCURSEL, 0, 0);
}

unsafe fn fill_polling(st: &State, idx: u8) {
    SendMessageW(st.polling, CB_RESETCONTENT, 0, 0);
    let items: Vec<String> = POLLING_RATES.iter().map(|(_, hz)| format!("{} Hz", hz)).collect();
    add_combo_items(st.polling, &items);
    // 设备里若残留越界档位（例如早期版本被写入的 2000Hz），不要假装它是 1000Hz
    let sel = POLLING_RATES
        .iter()
        .position(|(i, _)| *i == idx)
        .map(|i| i as i32)
        .unwrap_or(-1);
    SendMessageW(st.polling, CB_SETCURSEL, sel as WPARAM, 0);
}

unsafe fn fill_response(st: &State, value: u16) {
    SendMessageW(st.response, CB_RESETCONTENT, 0, 0);
    let items: Vec<String> = BUTTON_RESPONSES
        .iter()
        .map(|(v, ms)| format!("{} ms（值 {}）", ms, v))
        .collect();
    add_combo_items(st.response, &items);
    let sel = BUTTON_RESPONSES
        .iter()
        .position(|(v, _)| *v as u16 == value)
        .map(|i| i as i32)
        .unwrap_or(-1);
    SendMessageW(st.response, CB_SETCURSEL, sel as WPARAM, 0);
}

unsafe fn fill_profiles(st: &State, count: u8, current: u8) {
    SendMessageW(st.profile, CB_RESETCONTENT, 0, 0);
    let items: Vec<String> = (1..=count.max(1)).map(|i| format!("配置档 {}", i)).collect();
    add_combo_items(st.profile, &items);
    SendMessageW(st.profile, CB_SETCURSEL, (current.saturating_sub(1)) as WPARAM, 0);
}

/// 写操作前的守门：设备休眠时绝不写。
/// 休眠时设备只回占位应答，若把占位应答里的数字当成目标值写下去，
/// 就会把用户的设置改成垃圾值（DPI 档 1 被写成 1100 就是这么来的）。
unsafe fn guard_awake(st: &mut State) -> Option<*mut Md301> {
    let dp = match ensure_dev(st) {
        Ok(p) => p,
        Err(e) => {
            set_text(st.status, &format!("未连接设备：{}", e));
            return None;
        }
    };
    if !(*dp).is_awake() {
        set_text(
            st.status,
            "鼠标正在射频休眠（只回占位应答），已拒绝写入。请移动鼠标唤醒后再点。",
        );
        return None;
    }
    Some(dp)
}

unsafe fn combo_code(st: &State, combo: HWND) -> Option<u16> {
    let sel = SendMessageW(combo, CB_GETCURSEL, 0, 0);
    if sel < 0 {
        return None;
    }
    st.func_list.get(sel as usize).map(|(c, _)| *c)
}

const BN_CLICKED: u32 = 0;
const CBN_SELCHANGE: u32 = 1;
const CBN_SELENDOK: u32 = 9;

unsafe fn on_command(hwnd: HWND, st: &mut State, id: i32, code: u32) {
    match id {
        ID_RELOAD if code == BN_CLICKED => refresh(hwnd, st),
        ID_DPI_APPLY if code == BN_CLICKED => {
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            let mut vals = [0u32; 4];
            for i in 0..DPI_LEVELS {
                vals[i] = SendMessageW(st.sliders[i], TBM_GETPOS, 0, 0) as u32;
            }
            let mut err: Option<String> = None;
            // 只写 3 个有效档位：该型号固定 3 档（官方 caps maxDpiCount 3），第 4 槽写入
            // 会被设备忽略，档位数也不允许改，所以这里不碰它们。
            for i in 0..DPI_LEVELS {
                if let Err(e) = (*dp).set_dpi(i as u8 + 1, vals[i], None) {
                    err = Some(e.to_string());
                    break;
                }
            }
            match err {
                Some(e) => set_text(st.status, &format!("写入失败：{}", e)),
                None => {
                    refresh(hwnd, st);
                    set_text(st.status, "已写入 DPI，点[保存到 Flash] 永久保存");
                }
            }
        }
        ID_POLLING if code == CBN_SELCHANGE || code == CBN_SELENDOK => {
            let sel = SendMessageW(st.polling, CB_GETCURSEL, 0, 0);
            // 选中的就是设备上已有的值 → 不写（打开下拉框也会发通知）
            if sel < 0 || POLLING_RATES[sel as usize].0 == st.last_polling {
                return;
            }
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            let (idx, hz) = POLLING_RATES[sel as usize];
            match (*dp).set_polling_rate(idx) {
                Ok(_) => {
                    st.last_polling = idx;
                    set_text(
                        st.status,
                        &format!("回报率已设为 {} Hz，点[保存到 Flash] 永久保存", hz),
                    )
                }
                Err(e) => set_text(st.status, &format!("设置回报率失败：{}", e)),
            }
        }
        ID_RESPONSE if code == CBN_SELCHANGE || code == CBN_SELENDOK => {
            let sel = SendMessageW(st.response, CB_GETCURSEL, 0, 0);
            // 选中的就是设备上已有的值 → 不写（打开下拉框也会发通知）
            if sel < 0
                || (sel as usize) >= BUTTON_RESPONSES.len()
                || BUTTON_RESPONSES[sel as usize].0 as u16 == st.last_response
            {
                return;
            }
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            if sel >= 0 && (sel as usize) < BUTTON_RESPONSES.len() {
                // 官方 Ec 表：写入值就是索引本身
                let v = BUTTON_RESPONSES[sel as usize].0;
                let ms = response_ms(v as u16).unwrap_or(0);
                match (*dp).set_button_response(v) {
                    Ok(_) => {
                        st.last_response = v as u16;
                        set_text(
                            st.status,
                            &format!("按键响应已设为 {} ms（值 {}），点[保存到 Flash] 永久保存", ms, v),
                        )
                    }
                    Err(e) => set_text(st.status, &format!("设置按键响应失败：{}", e)),
                }
            }
        }
        ID_PROFILE_SET if code == BN_CLICKED => {
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            let sel = SendMessageW(st.profile, CB_GETCURSEL, 0, 0);
            if sel >= 0 {
                match (*dp).set_profile(sel as u8 + 1) {
                    Ok(_) => {
                        refresh(hwnd, st);
                        set_text(st.status, &format!("已切换到配置档 {}", sel + 1));
                    }
                    Err(e) => set_text(st.status, &format!("切换失败：{}", e)),
                }
            }
        }
        ID_SAVE if code == BN_CLICKED => {
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            match (*dp).save_profile() {
                Ok(_) => set_text(st.status, "已保存到鼠标 Flash（断电保持）"),
                Err(e) => set_text(st.status, &format!("保存失败：{}", e)),
            }
        }
        ID_RESET if code == BN_CLICKED => {
            let msg = wide("恢复出厂会清空全部配置档并把鼠标重置为默认设置，此操作不可撤销。\n\n确定要继续吗？");
            let cap = wide("恢复出厂设置");
            if MessageBoxW(hwnd, msg.as_ptr(), cap.as_ptr(), MB_YESNO | MB_ICONWARNING) != IDYES {
                return;
            }
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            match (*dp).reset_all_profiles() {
                Ok(_) => {
                    refresh(hwnd, st);
                    set_text(st.status, "已恢复出厂设置");
                }
                Err(e) => set_text(st.status, &format!("恢复出厂失败：{}", e)),
            }
        }
        ID_BTN_APPLY if code == BN_CLICKED => {
            let mut wanted: Vec<(u16, u16)> = Vec::new();
            for i in 0..8 {
                if let Some(code) = combo_code(st, st.buttons[i]) {
                    wanted.push((BTN_CODES[i], code));
                }
            }
            // 左键改掉会让人没法用鼠标点按钮，单独确认一次
            if let Some((_, code)) = wanted.iter().find(|(b, _)| *b == 0x1F0) {
                if *code != 0x1F0 {
                    let msg = wide(&format!(
                        "你正在把左键改成 {}。\n改完鼠标左键将不再执行点击（仍可用键盘操作本窗口），确定吗？",
                        func_name(*code)
                    ));
                    let cap = wide("确认修改左键");
                    if MessageBoxW(hwnd, msg.as_ptr(), cap.as_ptr(), MB_YESNO | MB_ICONWARNING) != IDYES {
                        return;
                    }
                }
            }
            let dp = match guard_awake(st) {
                Some(p) => p,
                None => return,
            };
            let mut err = None;
            for (btn, code) in &wanted {
                if let Err(e) = (*dp).map_button(*btn, *code) {
                    err = Some(e.to_string());
                    break;
                }
            }
            match err {
                Some(e) => set_text(st.status, &format!("写入按键映射失败：{}", e)),
                None => {
                    refresh(hwnd, st);
                    set_text(st.status, "按键映射已写入，点[保存到 Flash] 永久保存");
                }
            }
        }
        _ => {
            // 4 档"当前"单选 + 其他
            if code == BN_CLICKED && (ID_RADIO0..ID_RADIO0 + 4).contains(&id) {
                let idx = (id - ID_RADIO0) as u8 + 1;
                let dp = match guard_awake(st) {
                    Some(p) => p,
                    None => return,
                };
                match (*dp).switch_dpi(idx) {
                    Ok(_) => {
                        refresh(hwnd, st);
                        set_text(st.status, &format!("已切换到 DPI 档 {}", idx));
                    }
                    Err(e) => set_text(st.status, &format!("切换 DPI 档失败：{}", e)),
                }
            }
        }
    }
}
