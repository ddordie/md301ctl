//! ASUS ProArt MD301 protocol layer (reverse engineered from ASUS Gear Link).
//!
//! Report layout on the control interface (usage page 0xFF01, usage 1):
//!   byte0      command      (0x12 = GET, 0x51 = SET, 0x50 = profile/save)
//!   byte1      key/sub-command
//!   byte2..3   index, little endian
//!   byte4..63  payload
//! Responses echo (command, key, index) in bytes 0..3 and carry data from byte 4.

#![allow(dead_code)]

use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::hid::{enumerate_asus, HidDev, Interface};

pub const GET: u8 = 0x12;
pub const SET: u8 = 0x51;
pub const PROFILE_CMD: u8 = 0x50;
pub const DPI_BASE: u32 = 50;
pub const PAYLOAD: usize = 64;

pub const USAGE_PAGE_CONTROL: u16 = 0xFF01;
pub const USAGE_PAGE_EVENT: u16 = 0xFFC1;

/// MD301 官方页面只提供这 4 档回报率。
///
/// 协议枚举里还有 4/5/6 = 2000/4000/8000 Hz，但官方 JS 决定可选列表的规则是：
/// 蓝牙 → 125/250；有 Booster 或 PID ∈ {P723,P724,P725,P728,P729,P731} → 7 档；
/// 其余（含 MD301 的 7379 / 7381）→ 就这 4 档。
pub const POLLING_RATES: [(u8, u32); 4] = [(0, 125), (1, 250), (2, 500), (3, 1000)];

/// 协议枚举全集：只用于解读设备里可能残留的越界值，以及 `raw` 命令。
/// 按键响应档位——官方 bundle 里的 `Ec` 表：**写入值就是档位索引 0..5**，单位 ms
/// （该型号 caps `supportsButtonResponse:true`；官方 UI 滑块 `min:0,max:5,step:1`）。
pub const BUTTON_RESPONSES: [(u8, u32); 6] = [
    (0, 12),
    (1, 16),
    (2, 20),
    (3, 24),
    (4, 28),
    (5, 32),
];

/// 按键响应值 → 毫秒；不在表内返回 None。
pub fn response_ms(v: u16) -> Option<u32> {
    BUTTON_RESPONSES
        .iter()
        .find(|(i, _)| *i as u16 == v)
        .map(|(_, ms)| *ms)
}

/// 官方 caps（bundle 里的 `Cr` 对象）给这个型号的上限：`maxDPI:8000`、`maxDpiCount:3`
/// （且 `minDpiCount == maxDpiCount`，即档位数固定 3，官方 UI 不允许改）。
pub const DPI_MAX: u32 = 8000;
pub const DPI_MIN: u32 = 100;
pub const DPI_LEVELS: usize = 3;

pub const POLLING_RATES_ALL: [(u8, u32); 7] = [
    (0, 125),
    (1, 250),
    (2, 500),
    (3, 1000),
    (4, 2000),
    (5, 4000),
    (6, 8000),
];

/// Sensor modes.
pub const SENSOR_MODES: [(u8, &str); 5] = [
    (0, "Dynamic"), (1, "Office"), (2, "LowPower"), (3, "HighPerformance"), (4, "Corded"),
];

/// (code, firmware name, Chinese alias/description)
pub const FUNC_TABLE: [(u16, &str, &str); 156] = [
    (0, "Escape", ""),
    (1, "F1", ""),
    (2, "F2", ""),
    (3, "F3", ""),
    (4, "F4", ""),
    (5, "F5", ""),
    (6, "F6", ""),
    (7, "F7", ""),
    (8, "F8", ""),
    (9, "F9", ""),
    (10, "F10", ""),
    (11, "PrintScreen", ""),
    (12, "Numpad7", ""),
    (13, "Numpad8", ""),
    (14, "Numpad9", ""),
    (15, "NumpadAdd", ""),
    (16, "MS_LeftButton", ""),
    (17, "MS_LeftBackward", ""),
    (19, "AV_PlayPause", ""),
    (20, "Profile_0", ""),
    (21, "GP_JS_UpArrow", ""),
    (22, "TP_SlideForward", ""),
    (23, "HW_Reset", ""),
    (24, "LeverUp", ""),
    (25, "MS_Gesture_RightClick_Down", ""),
    (26, "MS_Gesture_DPISwitch_Down", ""),
    (27, "MS_Gesture_Backward_Down", ""),
    (28, "M1", ""),
    (37, "WS_ShowDesktop", ""),
    (38, "WS_SwitchToTheLeftDesktop", ""),
    (39, "MacroKey", ""),
    (50, "Fn_Escape", ""),
    (51, "Fn_F1", ""),
    (52, "Fn_F2", ""),
    (53, "Fn_F3", ""),
    (54, "Fn_F4", ""),
    (55, "Fn_F5", ""),
    (56, "Fn_F6", ""),
    (57, "Fn_F7", ""),
    (58, "Fn_F8", ""),
    (59, "Fn_F9", ""),
    (60, "Fn_F10", ""),
    (61, "Fn_PrintScreen", ""),
    (62, "Fn_Numpad7", ""),
    (63, "Fn_Numpad8", ""),
    (164, "Lever_Up", ""),
    (165, "Lever_Mid", ""),
    (166, "Lever_Down", ""),
    (193, "TouchPanel_SlideForward", ""),
    (194, "TouchPanel_SlideBackward", ""),
    (195, "TouchPanel_ClickForward", ""),
    (196, "TouchPanel_ClickMiddle", ""),
    (197, "TouchPanel_ClickBackward", ""),
    (198, "TouchPanel_DoubleClickMiddle", ""),
    (199, "TouchPanel_ZoomIn", ""),
    (200, "TouchPanel_ZoomOut", ""),
    (209, "WindowLockKey", ""),
    (210, "BrightnessButton", ""),
    (211, "Copilot", ""),
    (240, "Off", "关闭"),
    (241, "Mouse_Profile1", ""),
    (242, "Mouse_Profile2", ""),
    (243, "Mouse_Profile3", ""),
    (244, "Mouse_Profile4", ""),
    (245, "MultimediaVolumeDown", ""),
    (246, "MultimediaVolumeUp", ""),
    (249, "Mouse_BTHostSwitch", ""),
    (255, "DefaultKey", ""),
    (256, "Backquote", ""),
    (257, "Digit3", ""),
    (258, "Digit4", ""),
    (259, "Digit5", ""),
    (260, "Digit6", ""),
    (261, "Digit7", ""),
    (262, "Digit8", ""),
    (263, "Digit9", ""),
    (264, "Digit0", ""),
    (265, "Minus", ""),
    (266, "Equal", ""),
    (267, "ScrollLock", ""),
    (268, "Numpad4", ""),
    (269, "Numpad5", ""),
    (270, "Numpad6", ""),
    (272, "MS_RightButton", ""),
    (273, "MS_LeftForward", ""),
    (275, "AV_Stop", ""),
    (276, "Profile_1", ""),
    (277, "GP_JS_DownArrow", ""),
    (278, "TP_SlideBackward", ""),
    (280, "LeverMid", ""),
    (281, "MS_Gesture_RightClick_Left", ""),
    (282, "MS_Gesture_DPISwitch_Left", ""),
    (283, "MS_Gesture_Backward_Left", ""),
    (284, "M2", ""),
    (293, "WS_SwitchApp", ""),
    (294, "WS_SwitchToTheRightDesktop", ""),
    (295, "CombinationKey", ""),
    (448, "Mouse_RapidFireToggle", ""),
    (449, "Mouse_RapidFireShift", ""),
    (472, "Joystick_ForwardAxes0", ""),
    (473, "Joystick_ReverseAxes0", ""),
    (474, "Joystick_ForwardAxes1", ""),
    (475, "Joystick_ReverseAxes1", ""),
    (481, "MouseRightBackward", "右键前侧"),
    (482, "MouseRightForward", "右键后侧"),
    (483, "MouseDoubleClick", "双击"),
    (484, "MouseLeftBackward", "左后侧键"),
    (485, "MouseLeftForward", "左前侧键"),
    (486, "MouseDPISwitch", "DPI 切换"),
    (487, "MouseTargetFocus", "目标聚焦"),
    (488, "MouseScrollUp", "滚轮上滚"),
    (489, "MouseScrollDown", "滚轮下滚"),
    (490, "MouseSideKey1", "侧键1"),
    (491, "MouseSideKey2", "侧键2"),
    (492, "MouseSideKey3", "侧键3"),
    (493, "MouseSideKey4", "侧键4"),
    (494, "MouseSideKey5", "侧键5"),
    (495, "MouseSideKey6", "侧键6"),
    (496, "MouseLeftButton", "鼠标左键"),
    (497, "MouseRightButton", "鼠标右键"),
    (498, "MouseMiddleButton", "鼠标中键"),
    (499, "MouseDpiUp", "DPI 增加"),
    (500, "MouseDpiDown", "DPI 降低"),
    (511, "KeyDisabled", "禁用"),
    (512, "Tab", ""),
    (513, "KeyE", ""),
    (514, "KeyR", ""),
    (515, "KeyT", ""),
    (516, "KeyY", ""),
    (517, "KeyU", ""),
    (518, "KeyI", ""),
    (519, "KeyO", ""),
    (520, "KeyP", ""),
    (521, "BracketLeft", ""),
    (522, "BracketRight", ""),
    (523, "Pause", ""),
    (524, "Numpad1", ""),
    (525, "Numpad2", ""),
    (526, "Numpad3", ""),
    (527, "NumpadEnter", ""),
    (528, "MS_MiddleButton", ""),
    (529, "MS_RightBackward", ""),
    (530, "MS_DoubleClick", ""),
    (531, "AV_PrevTrack", ""),
    (532, "Profile_2", ""),
    (533, "GP_JS_LeftArrow", ""),
    (534, "TP_ClickForward", ""),
    (535, "MS_SideKey_1", ""),
    (536, "LeverDown", ""),
    (537, "MS_Gesture_RightClick_Right", ""),
    (538, "MS_Gesture_DPISwitch_Right", ""),
    (539, "MS_Gesture_Backward_Right", ""),
    (540, "M3", ""),
    (549, "WS_CloseApp", ""),
    (550, "WS_AddNewDesktop", ""),
    (65535, "Undefined", "无"),
];

pub fn func_name(code: u16) -> String {
    for (c, n, a) in FUNC_TABLE.iter() {
        if *c == code {
            return if a.is_empty() {
                format!("{} (0x{:X})", n, code)
            } else {
                format!("{} / {} (0x{:X})", a, n, code)
            };
        }
    }
    format!("Unknown(0x{:X})", code)
}

pub fn find_func(spec: &str) -> Option<u16> {
    let s = spec.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u16::from_str_radix(hex, 16).ok();
    }
    if let Ok(v) = s.parse::<u16>() {
        return Some(v);
    }
    let low = s.to_lowercase();
    for (c, n, a) in FUNC_TABLE.iter() {
        if n.to_lowercase() == low || (!a.is_empty() && a.to_lowercase() == low) {
            return Some(*c);
        }
    }
    // substring match as a fallback (e.g. "leftbutton")
    for (c, n, a) in FUNC_TABLE.iter() {
        if n.to_lowercase().contains(&low) || (!a.is_empty() && a.contains(s)) {
            return Some(*c);
        }
    }
    None
}

pub fn polling_label(idx: u8) -> String {
    for (i, hz) in POLLING_RATES_ALL.iter() {
        if *i == idx {
            return if *i < POLLING_RATES.len() as u8 {
                format!("{} Hz", hz)
            } else {
                format!("{} Hz（MD301 不提供该档，官方仅 125/250/500/1000）", hz)
            };
        }
    }
    format!("?({})", idx)
}

pub fn polling_index_from_hz(hz: u32) -> Option<u8> {
    POLLING_RATES.iter().find(|(_, h)| *h == hz).map(|(i, _)| *i)
}

/// 射频休眠占位应答判定。
///
/// 实测：醒着时 `getMouseDeviceInfo` 的版本字节是 0x29、配置档数是 3；
/// 休眠时设备回 0x20 且配置档数为 0，其余字段也全是假的——不能当数据用。
pub fn is_sleep_stub(r: &[u8]) -> bool {
    r.len() >= 9 && (r[8] == 0 || le32(r, 4) == 0x20)
}

// ---------------------------------------------------------------- typed data

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub version_raw: u32,
    pub version: String,
    pub dongle_version_raw: u32,
    pub dongle_version: String,
    pub profile_count: u8,
    pub effect_count: u8,
    pub current_profile: u8,
    pub dpi_index: u8,
    pub dpi_value: u32,
    pub dpi_levels: u8,
}

#[derive(Debug, Clone)]
pub struct PowerInfo {
    pub battery_percent: u8,
    pub sleep_time_index: u8,
    pub low_power_percent: u8,
    pub battery_voltage_raw: u16,
    pub is_charging: bool,
    pub battery_type: u8,
    pub full_charging_effect: u8,
}

#[derive(Debug, Clone)]
pub struct Performance {
    pub dpi: [u32; 4],
    pub polling_usbrf: u8,
    pub polling_ble: u8,
    pub polling_booster: u8,
    pub button_response: u16,
    pub angle_snapping: u16,
    pub angle_tuning: u16,
}

#[derive(Debug, Clone)]
pub struct KeyMap {
    pub left_button: u16,
    pub right_button: u16,
    pub middle_button: u16,
    pub side_left_backward: u16,
    pub side_left_forward: u16,
    pub dpi1: u16,
    pub scroll_up: u16,
    pub scroll_down: u16,
}

impl KeyMap {
    pub fn entries(&self) -> [(&'static str, u16); 8] {
        [
            ("左键", self.left_button),
            ("右键", self.right_button),
            ("中键", self.middle_button),
            ("左侧后键", self.side_left_backward),
            ("左侧前键", self.side_left_forward),
            ("DPI 切换键", self.dpi1),
            ("滚轮上滚", self.scroll_up),
            ("滚轮下滚", self.scroll_down),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ExtKeyMap {
    pub side_right_backward: u16,
    pub side_right_forward: u16,
    pub side: [u16; 6],
}

#[derive(Debug, Clone)]
pub struct AddKeyMap {
    pub dpi2: u16,
    pub joystick_up: u16,
    pub joystick_down: u16,
    pub joystick_left: u16,
    pub joystick_right: u16,
    pub kb_left: u16,
    pub kb_right: u16,
}

// ------------------------------------------------------------------ protocol

pub struct Md301 {
    pub ctrl: HidDev,
    pub evt: Option<HidDev>,
    pub ctrl_iface: Interface,
    pub evt_iface: Option<Interface>,
}

fn le16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

fn le32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn ver(raw: u32) -> String {
    if raw == 0 {
        return String::new();
    }
    format!(
        "{}.{}.{}",
        (raw >> 16) & 0xFF,
        (raw >> 8) & 0xFF,
        raw & 0xFF
    )
}

impl Md301 {
    /// Find and open the control + event interfaces of a connected MD301 (any of its PIDs).
    pub fn open() -> io::Result<Md301> {
        let ifaces = enumerate_asus()?;
        let mut candidates: Vec<&Interface> = ifaces
            .iter()
            .filter(|i| {
                i.usage_page == USAGE_PAGE_CONTROL && i.usage == 1 && i.output_len >= 64
            })
            .collect();
        // When several ASUS devices are attached, prefer the MD301 itself
        // (RF dongle PID 0x1CD3, wired PID 0x1CD5, or a product name saying so).
        candidates.sort_by_key(|i| {
            let known_pid = matches!(i.pid, 0x1CD3 | 0x1CD5);
            let name = i.product.to_uppercase();
            let known_name = name.contains("MD301") || name.contains("PROART");
            match (known_pid, known_name) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 3,
            }
        });
        let ctrl_iface: Interface = (*candidates
            .first()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "未找到 MD301 控制接口 (Usage Page 0xFF01 / Usage 1)。请确认鼠标已通过 USB 线或 2.4G 接收器连接。",
                )
            })?)
        .clone();
        let evt_iface = ifaces
            .iter()
            .find(|i| i.usage_page == USAGE_PAGE_EVENT && i.usage == 1)
            .cloned();
        let ctrl = HidDev::open(&ctrl_iface)?;
        let evt = match &evt_iface {
            Some(i) => HidDev::open(i).ok(),
            None => None,
        };
        Ok(Md301 { ctrl, evt, ctrl_iface, evt_iface })
    }

    fn build(cmd: u8, key: u8, index: u16, data: &[u8]) -> [u8; PAYLOAD] {
        let mut buf = [0u8; PAYLOAD];
        buf[0] = cmd;
        buf[1] = key;
        buf[2] = (index & 0xFF) as u8;
        buf[3] = (index >> 8) as u8;
        let n = data.len().min(PAYLOAD - 4);
        buf[4..4 + n].copy_from_slice(&data[..n]);
        buf
    }

    /// Fire-and-forget write.
    pub fn send(&mut self, cmd: u8, key: u8, index: u16, data: &[u8]) -> io::Result<()> {
        let payload = Self::build(cmd, key, index, data);
        let mut wire = Vec::with_capacity(PAYLOAD + 1);
        wire.push(0x00); // report ID
        wire.extend_from_slice(&payload);
        self.ctrl.write_report(&wire)
    }

    /// Write and wait for the matching response (command, key, index).
    pub fn request(
        &mut self,
        cmd: u8,
        key: u8,
        index: u16,
        data: &[u8],
        timeout_ms: u32,
    ) -> io::Result<Option<[u8; PAYLOAD]>> {
        self.send(cmd, key, index, data)?;
        let want = (cmd, key, (index & 0xFF) as u8, (index >> 8) as u8);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
        let mut buf = [0u8; 65];
        let mut last: Option<[u8; PAYLOAD]> = None;
        while Instant::now() < deadline {
            match self.ctrl.read_report(&mut buf, 120)? {
                Some(n) if n > 0 => {
                    let r = normalize(&buf[..n]);
                    if (r[0], r[1], r[2], r[3]) == want {
                        return Ok(Some(r));
                    }
                    last = Some(r);
                }
                _ => {}
            }
        }
        Ok(last)
    }

    /// Like `request` but retries a few times: the RF link drops commands while the
    /// mouse is asleep, so the first attempt is often only a wake-up nudge.
    pub fn request_retry(
        &mut self,
        cmd: u8,
        key: u8,
        index: u16,
        data: &[u8],
        timeout_ms: u32,
    ) -> io::Result<[u8; PAYLOAD]> {
        let mut last_err: Option<[u8; PAYLOAD]> = None;
        for attempt in 0..4 {
            if let Some(r) = self.request(cmd, key, index, data, timeout_ms)? {
                match (r[0], r[1], r[2], r[3]) {
                    (c, k, lo, hi)
                        if c == cmd && k == key && lo == (index & 0xFF) as u8 && hi == (index >> 8) as u8 =>
                    {
                        return Ok(r)
                    }
                    _ => last_err = Some(r),
                }
            }
            if attempt < 3 {
                sleep(Duration::from_millis(250));
            }
        }
        Err(match last_err {
            Some(r) => io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "设备无有效应答（可能是射频休眠）。请移动/点击鼠标唤醒后重试。最后收到: {}",
                    hex(&r)
                ),
            ),
            None => io::Error::new(
                io::ErrorKind::TimedOut,
                "设备无应答。请确认鼠标已开机并处于 2.4G 模式，然后移动鼠标唤醒后重试。",
            ),
        })
    }

    // ------------------------------------------------------------ higher level

    /// True when the mouse answers with real data (it returns a stub while asleep).
    pub fn is_awake(&mut self) -> bool {
        match self.request(GET, 0x00, 0x00, &[], 600) {
            Ok(Some(r)) => r[8] > 0,
            _ => false,
        }
    }

    pub fn info(&mut self) -> io::Result<DeviceInfo> {
        let r = self.request_retry(GET, 0x00, 0x00, &[], 1200)?;
        // 射频休眠时设备回的是占位应答（实测：版本字节 0x20、配置档数 0），
        // 里面每个字段都是假的。必须在这里拦住，否则界面会把垃圾当数据渲染，
        // 用户一点「应用」就把垃圾写进鼠标（真实发生过：DPI 档 1 被写成 1100）。
        if is_sleep_stub(&r) {
            return Err(io::Error::other(format!(
                "设备处于射频休眠，回的是占位应答（{}）。请移动鼠标唤醒后重试。",
                hex(&r[..8])
            )));
        }
        Ok(DeviceInfo {
            version_raw: le32(&r, 4),
            version: ver(le32(&r, 4)),
            dongle_version_raw: le32(&r, 14),
            dongle_version: ver(le32(&r, 14)),
            profile_count: r[8],
            effect_count: r[9],
            current_profile: r[10] + 1,
            dpi_index: r[11],
            dpi_value: (le16(&r, 12) as u32 + 1) * DPI_BASE,
            dpi_levels: r[18],
        })
    }

    pub fn mouse_present(&mut self) -> io::Result<bool> {
        let r = self.request_retry(GET, 0x00, 0x02, &[], 1000)?;
        Ok(r[4] == 1)
    }

    pub fn power(&mut self) -> io::Result<PowerInfo> {
        let r = self.request_retry(GET, 0x07, 0x00, &[], 1200)?;
        Ok(PowerInfo {
            battery_percent: r[4],
            sleep_time_index: r[5],
            low_power_percent: r[6],
            battery_voltage_raw: le16(&r, 7),
            is_charging: r[9] == 1,
            battery_type: r[10],
            full_charging_effect: r[11],
        })
    }

    pub fn performance(&mut self) -> io::Result<Performance> {
        let r = self.request_retry(GET, 0x04, 0x00, &[], 1200)?;
        Ok(Performance {
            dpi: [
                (le16(&r, 4) as u32 + 1) * DPI_BASE,
                (le16(&r, 6) as u32 + 1) * DPI_BASE,
                (le16(&r, 8) as u32 + 1) * DPI_BASE,
                (le16(&r, 10) as u32 + 1) * DPI_BASE,
            ],
            polling_booster: (r[12] >> 4) & 0x0F,
            polling_usbrf: r[12] & 0x0F,
            polling_ble: ((le16(&r, 12) >> 12) & 0x0F) as u8,
            button_response: le16(&r, 14),
            angle_snapping: le16(&r, 16) & 0xFF,
            angle_tuning: le16(&r, 18),
        })
    }

    pub fn split_dpi(&mut self) -> io::Result<[u32; 8]> {
        let r = self.request_retry(GET, 0x04, 0x02, &[], 1200)?;
        let mut out = [0u32; 8];
        for i in 0..8 {
            out[i] = (le16(&r, 4 + i * 2) as u32 + 1) * DPI_BASE;
        }
        Ok(out)
    }

    pub fn dpi_rgb(&mut self) -> io::Result<[[u8; 3]; 4]> {
        let r = self.request_retry(GET, 0x04, 0x03, &[], 1200)?;
        let mut out = [[0u8; 3]; 4];
        for i in 0..4 {
            out[i] = [r[4 + i * 3], r[5 + i * 3], r[6 + i * 3]];
        }
        Ok(out)
    }

    pub fn sensor_mode(&mut self) -> io::Result<(u8, bool)> {
        let r = self.request_retry(GET, 0x04, 0x05, &[], 1200)?;
        Ok((r[4], r[5] == 0))
    }

    pub fn key_map(&mut self) -> io::Result<KeyMap> {
        let r = self.request_retry(GET, 0x05, 0x00, &[], 1200)?;
        Ok(KeyMap {
            left_button: le16(&r, 4),
            right_button: le16(&r, 6),
            middle_button: le16(&r, 8),
            side_left_backward: le16(&r, 10),
            side_left_forward: le16(&r, 12),
            dpi1: le16(&r, 14),
            scroll_up: le16(&r, 16),
            scroll_down: le16(&r, 18),
        })
    }

    pub fn ext_key_map(&mut self) -> io::Result<ExtKeyMap> {
        let r = self.request_retry(GET, 0x05, 0x01, &[], 1200)?;
        Ok(ExtKeyMap {
            side_right_backward: le16(&r, 4),
            side_right_forward: le16(&r, 6),
            side: [le16(&r, 8), le16(&r, 10), le16(&r, 12), le16(&r, 14), le16(&r, 16), le16(&r, 18)],
        })
    }

    pub fn add_key_map(&mut self) -> io::Result<AddKeyMap> {
        let r = self.request_retry(GET, 0x05, 0x02, &[], 1200)?;
        Ok(AddKeyMap {
            dpi2: le16(&r, 4),
            joystick_up: le16(&r, 6),
            joystick_down: le16(&r, 8),
            joystick_left: le16(&r, 10),
            joystick_right: le16(&r, 12),
            kb_left: le16(&r, 14),
            kb_right: le16(&r, 16),
        })
    }

    // ------------------------------------------------------------- writes

    /// DPI index 1..=4, value must be a multiple of 50 (encoded as value/50 - 1).
    pub fn set_dpi(&mut self, index: u8, dpi: u32, rgb: Option<[u8; 3]>) -> io::Result<[u8; PAYLOAD]> {
        if !(1..=4).contains(&index) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "DPI 档位必须是 1-4"));
        }
        if dpi < DPI_BASE || dpi % DPI_BASE != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "DPI 必须是 50 的倍数"));
        }
        let enc = ((dpi / DPI_BASE) - 1) as u16;
        let mut data = vec![(enc & 0xFF) as u8, (enc >> 8) as u8];
        if let Some(c) = rgb {
            data.extend_from_slice(&c);
        }
        self.request_retry(SET, 0x31, (index - 1) as u16, &data, 1200)
    }

    pub fn set_split_dpi(&mut self, index: u8, x: u32, y: u32) -> io::Result<[u8; PAYLOAD]> {
        if !(1..=4).contains(&index) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "DPI 档位必须是 1-4"));
        }
        let ex = ((x / DPI_BASE) - 1) as u16;
        let ey = ((y / DPI_BASE) - 1) as u16;
        let data = [
            (ex & 0xFF) as u8,
            (ex >> 8) as u8,
            (ey & 0xFF) as u8,
            (ey >> 8) as u8,
        ];
        self.request_retry(SET, 0x31, 12 + (index - 1) as u16, &data, 1200)
    }

    pub fn switch_dpi(&mut self, index: u8) -> io::Result<[u8; PAYLOAD]> {
        if !(1..=4).contains(&index) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "DPI 档位必须是 1-4"));
        }
        self.request_retry(SET, 0x31, 0x09, &[index, 0x00], 1200)
    }

    /// 设置档位数。注意：本型号官方 caps 里 minDpiCount == maxDpiCount == 3，
    /// 档位数固定，界面不再调用它（保留给其他型号或手工实验）。
    #[allow(dead_code)]
    pub fn set_dpi_preset_count(&mut self, n: u8) -> io::Result<[u8; PAYLOAD]> {
        if !(1..=4).contains(&n) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "DPI 档位数必须是 1-4"));
        }
        self.request_retry(SET, 0x31, 0x0A, &[n], 1200)
    }

    pub fn set_polling_rate(&mut self, index: u8) -> io::Result<[u8; PAYLOAD]> {
        if POLLING_RATES.iter().all(|(i, _)| *i != index) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "回报率只能是 125/250/500/1000（MD301 官方只提供这 4 档）"));
        }
        self.request_retry(SET, 0x31, 0x04, &[index, 0x00], 1200)
    }

    pub fn set_button_response(&mut self, value: u8) -> io::Result<[u8; PAYLOAD]> {
        self.request_retry(SET, 0x31, 0x05, &[value, 0x00], 1200)
    }

    pub fn set_angle_snapping(&mut self, on: bool) -> io::Result<[u8; PAYLOAD]> {
        self.request_retry(SET, 0x31, 0x06, &[if on { 1 } else { 0 }], 1200)
    }

    pub fn set_angle_tuning(&mut self, value: u16) -> io::Result<[u8; PAYLOAD]> {
        self.request_retry(SET, 0x31, 0x0B, &[(value & 0xFF) as u8, (value >> 8) as u8], 1200)
    }

    pub fn set_sensor_mode(&mut self, mode: u8, fps_13k: bool) -> io::Result<[u8; PAYLOAD]> {
        if SENSOR_MODES.iter().all(|(m, _)| *m != mode) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "传感器模式必须是 0-4"));
        }
        self.request_retry(SET, 0x31, 0x13, &[mode, if fps_13k { 0 } else { 1 }], 1200)
    }

    pub fn map_button(&mut self, button: u16, func: u16) -> io::Result<[u8; PAYLOAD]> {
        let data = [
            (button & 0xFF) as u8,
            (button >> 8) as u8,
            (func & 0xFF) as u8,
            (func >> 8) as u8,
        ];
        self.request_retry(SET, 0x21, 0x00, &data, 1200)
    }

    pub fn map_shift_button(&mut self, button: u16, func: u16) -> io::Result<[u8; PAYLOAD]> {
        let data = [
            (button & 0xFF) as u8,
            (button >> 8) as u8,
            (func & 0xFF) as u8,
            (func >> 8) as u8,
        ];
        self.request_retry(SET, 0x23, 0x00, &data, 1200)
    }

    pub fn set_profile(&mut self, profile: u8) -> io::Result<[u8; PAYLOAD]> {
        if profile < 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "配置档必须是 1-3"));
        }
        self.request_retry(PROFILE_CMD, 0x02, (profile - 1) as u16, &[], 1200)
    }

    pub fn save_profile(&mut self) -> io::Result<[u8; PAYLOAD]> {
        self.request_retry(PROFILE_CMD, 0x03, 0x00, &[], 2000)
    }

    /// Restore factory settings: wipes every stored profile. Destructive.
    pub fn reset_all_profiles(&mut self) -> io::Result<[u8; PAYLOAD]> {
        self.request_retry(PROFILE_CMD, 0x40, 0x00, &[], 3000)
    }

    pub fn set_power_config(
        &mut self,
        sleep_index: u8,
        low_power: u8,
        battery_type: u8,
        charging_effect: u8,
    ) -> io::Result<[u8; PAYLOAD]> {
        if low_power > 100 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "低电量阈值必须是 0-100"));
        }
        let data = [sleep_index, 0x00, low_power, battery_type, charging_effect];
        self.request_retry(SET, 0x37, 0x00, &data, 1200)
    }

    /// Non-blocking read of one event-channel report (asynchronous notifications).
    pub fn read_event(&mut self, timeout_ms: u32) -> io::Result<Option<[u8; PAYLOAD]>> {
        let mut buf = [0u8; 65];
        let evt = match self.evt.as_mut() {
            Some(e) => e,
            None => return Ok(None),
        };
        match evt.read_report(&mut buf, timeout_ms)? {
            Some(n) if n > 0 => Ok(Some(normalize(&buf[..n]))),
            _ => Ok(None),
        }
    }
}

/// Windows hands back 65 bytes for the 64-byte report when the report-ID byte is
/// included; normalise both shapes to the 64-byte payload.
fn normalize(raw: &[u8]) -> [u8; PAYLOAD] {
    let mut out = [0u8; PAYLOAD];
    if raw.len() >= PAYLOAD + 1 {
        out.copy_from_slice(&raw[raw.len() - PAYLOAD..]);
    } else {
        let n = raw.len().min(PAYLOAD);
        out[..n].copy_from_slice(&raw[..n]);
    }
    out
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02X}", x)).collect::<Vec<_>>().join(" ")
}

pub fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace() && *c != ',' && *c != '-').collect();
    if clean.len() % 2 != 0 {
        return Err("十六进制字符数必须是偶数".into());
    }
    (0..clean.len() / 2)
        .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(s: &str) -> Vec<u8> {
        parse_hex(s).unwrap()
    }

    #[test]
    fn awake_reply_is_real() {
        // 实测唤醒态：版本 0x29(→0.0.41)、配置档数 3、当前档 2、DPI 2000
        let r = frame("12 00 00 00 29 00 00 00 03 00 00 02 27 00 29 00");
        assert!(!is_sleep_stub(&r));
    }

    #[test]
    fn sleep_reply_is_a_stub() {
        // 实测休眠占位应答：版本字节 0x20(→0.0.32)、配置档数 0
        let r = frame("12 00 00 00 20 00 00 00 00 00 00 00 00 00 00 00");
        assert!(is_sleep_stub(&r));
    }

    #[test]
    fn zero_profile_count_is_a_stub() {
        let r = frame("12 00 00 00 29 00 00 00 00 00 00 02 27 00 00 00");
        assert!(is_sleep_stub(&r));
    }

    #[test]
    fn too_short_is_not_a_stub() {
        assert!(!is_sleep_stub(&[0x12, 0x00, 0x00, 0x00]));
    }

    #[test]
    fn button_response_table_matches_official() {
        // 官方 Ec 表：值 = 索引，标签 = 12/16/20/24/28/32 ms
        let ms: Vec<u32> = BUTTON_RESPONSES.iter().map(|(_, m)| *m).collect();
        assert_eq!(ms, vec![12, 16, 20, 24, 28, 32]);
        for (i, (v, m)) in BUTTON_RESPONSES.iter().enumerate() {
            assert_eq!(*v as usize, i);
            assert_eq!(response_ms(*v as u16), Some(*m));
        }
        assert_eq!(response_ms(6), None);
    }

    #[test]
    fn dpi_caps_match_official() {
        assert_eq!(DPI_MAX, 8000);
        assert_eq!(DPI_MIN, 100);
        assert_eq!(DPI_LEVELS, 3);
    }
}
