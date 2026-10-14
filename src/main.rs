//! md301ctl —— 华硕 ProArt MD301 鼠标离线配置工具（Rust）
//!
//! 功能等价于 ASUS Gear Link 网页版对 MD301 的设备设置部分，但**完全离线**：
//! 直接通过 HID 与鼠标（或 2.4G 接收器）通信，不访问任何网站、不需要浏览器。
//!
//! 协议由 ASUS Gear Link 网页版（gearlink.asus.com.cn）的 JS 逆向得到，
//! 并已在本机设备上逐条读写验证。

mod hid;
mod proto;

use std::process::ExitCode;
mod gui;
use std::thread::sleep;
use std::time::Duration;

use proto::*;

const SLEEP_LABELS: [(u8, &str); 6] = [
    (0, "1 分钟"),
    (1, "2 分钟"),
    (2, "3 分钟"),
    (3, "5 分钟"),
    (4, "10 分钟"),
    (255, "永不"),
];

const BATTERY_TYPES: [(u8, &str); 2] = [(0, "干电池"), (1, "可充电锂电池")];

fn sleep_label(i: u8) -> String {
    SLEEP_LABELS
        .iter()
        .find(|(k, _)| *k == i)
        .map(|(_, v)| format!("{} ({})", v, i))
        .unwrap_or_else(|| format!("未知({})", i))
}

fn battery_label(i: u8) -> String {
    BATTERY_TYPES
        .iter()
        .find(|(k, _)| *k == i)
        .map(|(_, v)| format!("{} ({})", v, i))
        .unwrap_or_else(|| format!("未知({})", i))
}

fn sensor_label(m: u8) -> String {
    SENSOR_MODES
        .iter()
        .find(|(k, _)| *k == m)
        .map(|(_, v)| format!("{} ({})", v, m))
        .unwrap_or_else(|| format!("未知({})", m))
}

const HELP: &str = r#"md301ctl —— 华硕 ProArt MD301 鼠标离线配置工具

用法: md301ctl <命令> [参数...]      全局选项: --json

  gui                           打开图形界面（离线可用；直接双击 exe 也会开）
  list                          列出本机检测到的 ASUS HID 接口
  info                          设备信息（固件版本 / 配置档 / 当前 DPI）
  status                        一次性输出 info + battery + perf + buttons
  spec                          [--json] 静态规格：能力表 + 全部功能码（GUI 用）

  battery                       电量、充电状态、休眠与低电量设置
  power set [选项]              修改电源设置
                                  --sleep <0-4|255>   0=1分钟 1=2分钟 2=3分钟 3=5分钟 4=10分钟 255=永不
                                  --low <0-100>       低电量提醒阈值(%)
                                  --battery <0|1>     0=干电池 1=可充电锂电池
                                  --effect <0|1>      满电灯效开关

  dpi                           显示 4 档 DPI 及当前生效档位
  dpi set <档位 1-3> <DPI>      写入某档 DPI（50 的倍数，官方上限 8000）
  dpi switch <档位 1-3>         立即切换到某个 DPI 档
  dpi presets <1-4>             启用几档 DPI

  split show                    显示 X/Y 分离 DPI
  split set <档位 1-4> <X> <Y>  写入分离 DPI

  perf                          性能信息（DPI / 回报率 / 按键响应 / 直线修正）
  perf polling <Hz>             回报率: 125 250 500 1000 2000 4000 8000
  perf snapping <on|off>        直线修正（角度捕捉）
  perf tuning <0-65535>         角度微调
  perf response <0-5>           按键响应（6 档：12/16/20/24/28/32 ms）
  perf sensor <0-4> <13k|20k>   传感器模式: 0=动态 1=办公 2=低功耗 3=高性能 4=有线
  perf save                     把当前设置保存到当前配置档（写入鼠标 Flash）

  buttons                       显示按键映射（基础 + 扩展 + 附加）
  buttons names [关键字]        列出可用功能码（可过滤）
  buttons map <按键> <功能>     修改按键映射，例如:
                                  md301ctl buttons map 左后侧键 中键
                                  md301ctl buttons map 0x1E4 MouseMiddleButton
  buttons map-shift <按键> <功能>   修改 Shift 层按键映射

  profile                       显示当前配置档与总数
  profile set <1-3>             切换配置档
  profile save                  保存当前设置到当前配置档

  monitor [秒数]                监听事件通道（DPI / 回报率变化），默认 30 秒

  raw <hex> [--timeout ms]      发送原始命令并打印应答，例如: md301ctl raw 12 07 00 00

说明:
  * 鼠标射频休眠时不会正常应答（只回占位数据）。移动或点击鼠标唤醒后再执行命令。
  * 修改后若需断电保持，请执行 "md301ctl perf save" 或 "md301ctl profile save"。
  * DPI 编码步进为 50，即 DPI 必须是 50 的倍数（如 400 800 1200 1600 2000）。
"#;

fn main() -> ExitCode {
    // 让 `md301ctl status | head` 这类提前关闭管道的用法安静退出，而不是打印 panic
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        if !msg.contains("failed printing to stdout") {
            eprintln!("{}", msg);
        }
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        // 无参数（例如双击 exe）直接打开图形界面
        return match gui::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("错误: {}", e);
                ExitCode::FAILURE
            }
        };
    }
    if args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        println!("{}", HELP);
        return ExitCode::SUCCESS;
    }
    let json = args.iter().any(|a| a == "--json");
    let argv: Vec<String> = args.iter().filter(|a| *a != "--json").cloned().collect();

    match run(&argv, json) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn need_more(cmd: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("参数不足，用法: md301ctl {}（md301ctl --help 查看全部）", cmd),
    ))
}

fn opt(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn parse_u32(s: &str, what: &str) -> std::io::Result<u32> {
    s.parse::<u32>().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} 不是合法数字: {}", what, s),
        )
    })
}

fn run(argv: &[String], json: bool) -> std::io::Result<()> {
    let cmd = argv[0].as_str();
    let rest = &argv[1..];

    if cmd == "list" {
        return cmd_list();
    }
    if cmd == "gui" {
        return gui::run();
    }

    let mut dev = Md301::open()?;

    match cmd {
        "info" => return cmd_info(&mut dev, json),
        "status" => {
            if json {
                // GUI 契约：一次调用、一个对象（多块 JSON 前端没法可靠解析）
                return cmd_status_json(&mut dev);
            }
            cmd_info(&mut dev, false)?;
            println!();
            cmd_battery(&mut dev, false)?;
            println!();
            cmd_perf(&mut dev, false)?;
            println!();
            cmd_buttons(&mut dev, false)?;
            Ok(())
        }
        "spec" => return cmd_spec_json(),
        "battery" => cmd_battery(&mut dev, json),
        "power" => match rest.first().map(|s| s.as_str()) {
            Some("set") => cmd_power_set(&mut dev, &rest[1..]),
            _ => cmd_battery(&mut dev, json),
        },
        "dpi" => match rest.first().map(|s| s.as_str()) {
            Some("set") => {
                if rest.len() < 3 {
                    return need_more("dpi set <档位 1-3> <DPI>");
                }
                let idx: u8 = rest[1].parse().map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "档位必须是 1-3（该型号官方 caps: maxDpiCount 3）",
                    )
                })?;
                if !(1..=proto::DPI_LEVELS).contains(&(idx as usize)) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "档位必须是 1-3（该型号官方 caps: maxDpiCount 3，第 4 档写入会被设备忽略）",
                    ));
                }
                let dpi = parse_u32(&rest[2], "DPI")?;
                if !(proto::DPI_MIN..=proto::DPI_MAX).contains(&dpi) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "DPI 超出官方范围 {}-{}（该型号 caps maxDPI:8000）",
                            proto::DPI_MIN,
                            proto::DPI_MAX
                        ),
                    ));
                }
                let r = dev.set_dpi(idx, dpi, None)?;
                println!("已写入 DPI 档 {} = {}", idx, dpi);
                println!("  应答: {}", hex(&r));
                cmd_dpi(&mut dev, json)
            }
            Some("switch") => {
                if rest.len() < 2 {
                    return need_more("dpi switch <档位 1-3>");
                }
                let idx: u8 = rest[1].parse().map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "档位必须是 1-3（该型号官方 caps: maxDpiCount 3）",
                    )
                })?;
                if !(1..=proto::DPI_LEVELS).contains(&(idx as usize)) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "档位必须是 1-3（该型号官方 caps: maxDpiCount 3）",
                    ));
                }
                dev.switch_dpi(idx)?;
                println!("已切换到 DPI 档 {}", idx);
                cmd_dpi(&mut dev, json)
            }
            Some("presets") => {
                if rest.len() < 2 {
                    return need_more("dpi presets <1-4>");
                }
                let n: u8 = rest[1].parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "档位数必须是 1-4")
                })?;
                dev.set_dpi_preset_count(n)?;
                println!("已启用 {} 档 DPI", n);
                cmd_dpi(&mut dev, json)
            }
            _ => cmd_dpi(&mut dev, json),
        },
        "split" => match rest.first().map(|s| s.as_str()) {
            Some("set") => {
                if rest.len() < 4 {
                    return need_more("split set <档位 1-4> <X> <Y>");
                }
                let idx: u8 = rest[1].parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "档位必须是 1-4")
                })?;
                let x = parse_u32(&rest[2], "X")?;
                let y = parse_u32(&rest[3], "Y")?;
                dev.set_split_dpi(idx, x, y)?;
                println!("已写入分离 DPI 档 {} = X {} / Y {}", idx, x, y);
                cmd_split(&mut dev, json)
            }
            _ => cmd_split(&mut dev, json),
        },
        "perf" => match rest.first().map(|s| s.as_str()) {
            Some("polling") => {
                if rest.len() < 2 {
                    return need_more("perf polling <125|250|500|1000|2000|4000|8000>");
                }
                let hz = parse_u32(&rest[1], "回报率")?;
                let idx = polling_index_from_hz(hz).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "回报率只能是 125/250/500/1000（MD301 官方只提供这 4 档）",
                    )
                })?;
                dev.set_polling_rate(idx)?;
                println!("回报率已设为 {} Hz", hz);
                cmd_perf(&mut dev, json)
            }
            Some("snapping") => {
                if rest.len() < 2 {
                    return need_more("perf snapping <on|off>");
                }
                let on = matches!(rest[1].as_str(), "on" | "1" | "true" | "开");
                let want = if on { 1u16 } else { 0u16 };
                let before = dev.performance()?.angle_snapping;
                dev.set_angle_snapping(on)?;
                let after = dev.performance()?.angle_snapping;
                println!("直线修正已{}", if on { "开启" } else { "关闭" });
                if after == before && after != want {
                    println!(
                        "注意: 设备没有改变该设置（仍为 {}）—— MD301 不支持直线修正，写入被忽略。",
                        after
                    );
                }
                cmd_perf(&mut dev, json)
            }
            Some("tuning") => {
                if rest.len() < 2 {
                    return need_more("perf tuning <0-65535>");
                }
                let v = parse_u32(&rest[1], "角度微调")? as u16;
                dev.set_angle_tuning(v)?;
                println!("角度微调已设为 {}", v);
                cmd_perf(&mut dev, json)
            }
            Some("response") => {
                if rest.len() < 2 {
                    return need_more("perf response <值>");
                }
                let v = parse_u32(&rest[1], "按键响应")?;
                if v > 5 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "按键响应只能是 0-5（官方 6 档：12/16/20/24/28/32 ms）",
                    ));
                }
                let v = v as u8;
                let before = dev.performance()?.button_response;
                dev.set_button_response(v)?;
                let after = dev.performance()?.button_response;
                println!(
                    "按键响应已设为 {} ms（值 {}）",
                    proto::response_ms(v as u16).unwrap_or(0),
                    v
                );
                if after != v as u16 {
                    println!(
                        "注意: 设备未接受该值（仍为 {}，写入前为 {}）。官方可选值为 0-5（12/16/20/24/28/32 ms）。",
                        after, before
                    );
                }
                cmd_perf(&mut dev, json)
            }
            Some("sensor") => {
                if rest.len() < 3 {
                    return need_more("perf sensor <0-4> <13k|20k>");
                }
                let m: u8 = rest[1].parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "传感器模式必须是 0-4")
                })?;
                let fps13 = matches!(rest[2].as_str(), "13k" | "13K" | "13");
                if dev.sensor_mode().is_err() {
                    println!("注意: 本设备读不到传感器模式（无应答），MD301 不支持该设置，写入会被忽略。");
                }
                dev.set_sensor_mode(m, fps13)?;
                println!(
                    "传感器模式={} 帧率={}",
                    sensor_label(m),
                    if fps13 { "13K" } else { "20K" }
                );
                cmd_perf(&mut dev, json)
            }
            Some("save") => {
                let r = dev.save_profile()?;
                println!("已保存到当前配置档（写入鼠标 Flash）。应答: {}", hex(&r));
                Ok(())
            }
            _ => cmd_perf(&mut dev, json),
        },
        "buttons" => match rest.first().map(|s| s.as_str()) {
            Some("names") => {
                let filter = rest.get(1).cloned().unwrap_or_default().to_lowercase();
                for (code, name, alias) in FUNC_TABLE.iter() {
                    let line = if alias.is_empty() {
                        format!("{:>5}  0x{:04X}  {}", code, code, name)
                    } else {
                        format!("{:>5}  0x{:04X}  {} / {}", code, code, alias, name)
                    };
                    if filter.is_empty() || line.to_lowercase().contains(&filter) {
                        println!("{}", line);
                    }
                }
                Ok(())
            }
            Some("map") | Some("map-shift") => {
                let shift = rest[0] == "map-shift";
                if rest.len() < 3 {
                    return need_more("buttons map <按键> <功能>");
                }
                let button = proto::find_button(&rest[1]).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "无法识别的按键: {}（可用: 左键/右键/中键/左侧后键/左侧前键/DPI 切换键/滚轮上滚/滚轮下滚，或 0x1F0 这样的代码）",
                            rest[1]
                        ),
                    )
                })?;
                let func = find_func(&rest[2]).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "无法识别的功能: {}（用 `md301ctl buttons names` 查看可用名称）",
                            rest[2]
                        ),
                    )
                })?;
                if shift {
                    dev.map_shift_button(button, func)?;
                } else {
                    dev.map_button(button, func)?;
                }
                println!(
                    "{} 已映射: {} → {}",
                    if shift { "Shift 层按键" } else { "按键" },
                    func_name(button),
                    func_name(func)
                );
                cmd_buttons(&mut dev, json)
            }
            _ => cmd_buttons(&mut dev, json),
        },
        "profile" => match rest.first().map(|s| s.as_str()) {
            Some("set") => {
                if rest.len() < 2 {
                    return need_more("profile set <1-3>");
                }
                let p: u8 = rest[1].parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "配置档必须是 1-3")
                })?;
                dev.set_profile(p)?;
                sleep(Duration::from_millis(300));
                println!("已切换到配置档 {}", p);
                cmd_info(&mut dev, json)
            }
            Some("save") => {
                let r = dev.save_profile()?;
                println!("已保存到当前配置档。应答: {}", hex(&r));
                Ok(())
            }
            Some("reset-all") => {
                if !rest.iter().any(|a| a == "--yes") {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "这会清空设备上的全部配置档并恢复出厂设置，不可撤销。确认请加 --yes: md301ctl profile reset-all --yes",
                    ));
                }
                let r = dev.reset_all_profiles()?;
                println!("已恢复出厂设置（全部配置档已清空）。应答: {}", hex(&r));
                Ok(())
            }
            _ => cmd_info(&mut dev, json),
        },
        "monitor" => {
            let secs: u64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(30);
            cmd_monitor(&mut dev, secs)
        }
        "raw" => {
            let payload = parse_hex(&rest.first().cloned().unwrap_or_default())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            if payload.is_empty() || payload.len() > 64 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "raw 需要一个 1..64 字节的十六进制字符串",
                ));
            }
            let timeout: u32 = opt(rest, "--timeout")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1200);
            let cmd_b = payload[0];
            let key = payload.get(1).copied().unwrap_or(0);
            let index = if payload.len() >= 4 {
                u16::from_le_bytes([payload[2], payload[3]])
            } else {
                0
            };
            let data = if payload.len() > 4 { &payload[4..] } else { &[][..] };
            match dev.request(cmd_b, key, index, data, timeout)? {
                Some(r) => {
                    if json {
                        println!("{{\"response\":\"{}\"}}", hex(&r));
                    } else {
                        println!("应答: {}", hex(&r));
                    }
                }
                None => println!("(无应答 —— 可能是鼠标处于射频休眠，请先移动鼠标唤醒)"),
            }
            Ok(())
        }
        other => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("未知命令: {}（md301ctl --help 查看用法）", other),
        )),
    }
}

fn cmd_list() -> std::io::Result<()> {
    let ifaces = hid::enumerate_asus()?;
    if ifaces.is_empty() {
        println!("未发现 ASUS (VID 0x0B05) HID 接口。请确认鼠标已通过 USB 线或 2.4G 接收器连接。");
        return Ok(());
    }
    println!(
        "{:<24} {:>5} {:>5} {:>5}  {:<16} {:<26} {}",
        "接口", "In", "Out", "Feat", "UsagePage/Usage", "用途", "产品"
    );
    for i in &ifaces {
        let role = if i.usage_page == USAGE_PAGE_CONTROL && i.usage == 1 {
            "控制通道"
        } else if i.usage_page == USAGE_PAGE_EVENT && i.usage == 1 {
            "事件通道"
        } else {
            ""
        };
        println!(
            "{:<24} {:>5} {:>5} {:>5}  0x{:04X}/0x{:<8X} {:<26} {}",
            i.tag(),
            i.input_len,
            i.output_len,
            i.feature_len,
            i.usage_page,
            i.usage,
            role,
            i.product
        );
    }
    Ok(())
}

/// JSON 字符串转义（名字里可能带引号/反斜杠）
fn jesc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// GUI 契约：静态规格（能力表 + 功能码），一次获取后前端缓存。
/// 选项列表以此为准，前端不要自己写死。
fn cmd_spec_json() -> std::io::Result<()> {
    let mut s = String::from("{\"caps\":{");
    s.push_str(&format!(
        "\"dpi_min\":{},\"dpi_max\":{},\"dpi_levels\":{},\"dpi_step\":50,\"low_power_min\":0,\"low_power_max\":100",
        proto::DPI_MIN,
        proto::DPI_MAX,
        proto::DPI_LEVELS
    ));
    s.push_str(",\"polling_rates\":[");
    for (i, (_, hz)) in proto::POLLING_RATES.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&hz.to_string());
    }
    s.push_str("],\"button_responses\":[");
    for (i, (v, ms)) in proto::BUTTON_RESPONSES.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{{\"value\":{},\"ms\":{}}}", v, ms));
    }
    s.push_str("],\"sleep_times\":[");
    // 官方 SleepTime 表：0/1/2/3/5/10 分钟 + 255=永不（用干净的标签，值在独立字段里）
    for (i, (v, label)) in SLEEP_LABELS.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{{\"value\":{},\"label\":\"{}\"}}", v, jesc(label)));
    }
    s.push(']');
    s.push_str(",\"battery_types\":[");
    for (i, (v, label)) in BATTERY_TYPES.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{{\"value\":{},\"label\":\"{}\"}}", v, jesc(label)));
    }
    s.push_str("]");
    s.push_str("},\"buttons\":[");
    // 名字取自 proto::BUTTON_TABLE（与 buttons map 能解析的名字同一张表）
    for (i, (name, code)) in proto::BUTTON_TABLE.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{{\"name\":\"{}\",\"code\":{}}}", jesc(name), code));
    }
    s.push_str("],\"functions\":[");
    for (i, (code, _en, _zh)) in proto::FUNC_TABLE.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"code\":{},\"label\":\"{}\"}}",
            code,
            jesc(&func_name(*code))
        ));
    }
    s.push_str("]}");
    println!("{}", s);
    Ok(())
}

/// GUI 契约：实时状态快照（一个对象，含设备/电源/DPI/性能/按键）。
/// 任何一项读失败都返回错误（例如设备休眠），由前端提示用户。
fn cmd_status_json(dev: &mut Md301) -> std::io::Result<()> {
    let info = dev.info()?;
    let pw = dev.power()?;
    let pf = dev.performance()?;
    let ext = dev.ext_key_map().ok();
    let sensor = dev.sensor_mode().ok(); // 传感器模式要单独读；不支持时给 null
    let base = dev.key_map()?;
    let mut s = String::from("{\"device\":{");
    s.push_str(&format!(
        "\"version\":\"{}\",\"dongle_version\":\"{}\",\"profile_count\":{},\"current_profile\":{},\"dpi_index\":{},\"dpi_value\":{}",
        jesc(&info.version),
        jesc(&info.dongle_version),
        info.profile_count,
        info.current_profile,
        info.dpi_index,
        info.dpi_value
    ));
    s.push_str("},\"power\":{");
    s.push_str(&format!(
        "\"battery\":{},\"charging\":{},\"sleep_index\":{},\"low_power\":{},\"voltage_raw\":{},\"battery_type\":{},\"charging_effect\":{}",
        pw.battery_percent,
        pw.is_charging,
        pw.sleep_time_index,
        pw.low_power_percent,
        pw.battery_voltage_raw,
        pw.battery_type,
        pw.full_charging_effect
    ));
    s.push_str("},\"dpi\":{");
    s.push_str(&format!(
        "\"levels\":[{},{},{}],\"count\":{},\"current_index\":{},\"current_value\":{}",
        pf.dpi[0],
        pf.dpi[1],
        pf.dpi[2],
        proto::DPI_LEVELS,
        info.dpi_index,
        info.dpi_value
    ));
    s.push_str("},\"perf\":{");
    s.push_str(&format!(
        "\"polling_usb_rf\":\"{}\",\"polling_ble\":\"{}\",\"polling_booster\":\"{}\",\"button_response\":{},\"button_response_ms\":{},\"angle_snapping\":{},\"angle_tuning\":{},\"sensor_mode\":{},\"fps_13k\":{}",
        jesc(&polling_label(pf.polling_usbrf)),
        jesc(&polling_label(pf.polling_ble)),
        jesc(&polling_label(pf.polling_booster)),
        pf.button_response,
        proto::response_ms(pf.button_response).map(|m| m.to_string()).unwrap_or("null".into()),
        pf.angle_snapping,
        pf.angle_tuning,
        sensor.map(|(m, _)| m.to_string()).unwrap_or("null".into()),
        sensor.map(|(_, f)| f).unwrap_or(false)
    ));
    s.push_str("},\"buttons\":{");
    // 十个可控按键：基础表 8 个 + 扩展表里侧边滚轮的左右（sideKey5/6，偏移 16/18）
    let mut bi = 0;
    for (name, code) in base.entries().iter() {
        if bi > 0 {
            s.push(',');
        }
        bi += 1;
        s.push_str(&format!("\"{}\":{}", jesc(name), code));
    }
    for (name, val) in [
        ("向左滚动", ext.as_ref().map(|e| e.side[4])),
        ("向右滚动", ext.as_ref().map(|e| e.side[5])),
    ] {
        s.push(',');
        match val {
            Some(v) => s.push_str(&format!("\"{}\":{}", jesc(name), v)),
            None => s.push_str(&format!("\"{}\":null", jesc(name))),
        }
    }
    s.push_str("}}");
    println!("{}", s);
    Ok(())
}

fn cmd_info(dev: &mut Md301, json: bool) -> std::io::Result<()> {
    let info = dev.info()?;
    if json {
        println!(
            "{{\"version\":\"{}\",\"version_raw\":{},\"dongle_version\":\"{}\",\"profile_count\":{},\"current_profile\":{},\"dpi_index\":{},\"dpi_value\":{}}}",
            info.version,
            info.version_raw,
            info.dongle_version,
            info.profile_count,
            info.current_profile,
            info.dpi_index,
            info.dpi_value
        );
        return Ok(());
    }
    println!("设备            : ASUS ProArt MD301");
    println!(
        "鼠标固件        : {}",
        if info.version.is_empty() { "-".to_string() } else { info.version.clone() }
    );
    println!(
        "接收器固件      : {}",
        if info.dongle_version.is_empty() { "-".to_string() } else { info.dongle_version.clone() }
    );
    println!("配置档          : {} / {}", info.current_profile, info.profile_count);
    println!("当前 DPI 档     : {}", info.dpi_index);
    println!("当前 DPI 值     : {}", info.dpi_value);
    if info.dpi_levels > 0 {
        println!("启用档位数      : {}", info.dpi_levels);
    }
    Ok(())
}

fn cmd_battery(dev: &mut Md301, json: bool) -> std::io::Result<()> {
    let p = dev.power()?;
    if json {
        println!(
            "{{\"battery\":{},\"charging\":{},\"sleep_index\":{},\"low_power\":{},\"voltage_raw\":{},\"battery_type\":{},\"charging_effect\":{}}}",
            p.battery_percent,
            p.is_charging,
            p.sleep_time_index,
            p.low_power_percent,
            p.battery_voltage_raw,
            p.battery_type,
            p.full_charging_effect
        );
        return Ok(());
    }
    println!(
        "电量            : {}%{}",
        p.battery_percent,
        if p.is_charging { "（充电中）" } else { "" }
    );
    println!("电池类型        : {}", battery_label(p.battery_type));
    println!("电池电压(原始)  : {}", p.battery_voltage_raw);
    println!("休眠时间        : {}", sleep_label(p.sleep_time_index));
    println!("低电量提醒      : {}%", p.low_power_percent);
    println!(
        "满电灯效        : {}",
        if p.full_charging_effect == 1 { "开启" } else { "关闭" }
    );
    Ok(())
}

fn cmd_dpi(dev: &mut Md301, json: bool) -> std::io::Result<()> {
    let perf = dev.performance()?;
    let info = dev.info()?;
    if json {
        println!(
            "{{\"dpi\":[{},{},{}],\"levels\":{},\"current_index\":{},\"current_value\":{}}}",
            perf.dpi[0], perf.dpi[1], perf.dpi[2], proto::DPI_LEVELS, info.dpi_index, info.dpi_value
        );
        return Ok(());
    }
    for i in 0..proto::DPI_LEVELS {
        let mark = if (i as u8) + 1 == info.dpi_index { "   ← 当前" } else { "" };
        println!("DPI 档 {}         : {}{}", i + 1, perf.dpi[i], mark);
    }
    println!(
        "生效档位        : {} (值 {})  · 该型号固定 {} 档（官方 caps）",
        info.dpi_index,
        info.dpi_value,
        proto::DPI_LEVELS
    );
    Ok(())
}

fn cmd_split(dev: &mut Md301, json: bool) -> std::io::Result<()> {
    let s = dev.split_dpi()?;
    if json {
        println!(
            "{{\"dpi_x\":[{},{},{},{}],\"dpi_y\":[{},{},{},{}]}}",
            s[0], s[2], s[4], s[6], s[1], s[3], s[5], s[7]
        );
        return Ok(());
    }
    for i in 0..4 {
        println!("分离 DPI 档 {}   : X={} Y={}", i + 1, s[i * 2], s[i * 2 + 1]);
    }
    Ok(())
}

fn cmd_perf(dev: &mut Md301, json: bool) -> std::io::Result<()> {
    let p = dev.performance()?;
    let (mode, fps13, sensor_ok) = match dev.sensor_mode() {
        Ok((m, f)) => (m, f, true),
        Err(_) => (0, false, false),
    };
    let mode_json = if sensor_ok { mode.to_string() } else { "null".to_string() };
    if json {
        println!(
            "{{\"dpi\":[{},{},{},{}],\"polling_usb_rf\":\"{}\",\"polling_ble\":\"{}\",\"polling_booster\":\"{}\",\"button_response\":{},\"angle_snapping\":{},\"angle_tuning\":{},\"sensor_mode\":{},\"fps_13k\":{}}}",
            p.dpi[0], p.dpi[1], p.dpi[2], p.dpi[3],
            polling_label(p.polling_usbrf), polling_label(p.polling_ble), polling_label(p.polling_booster),
            p.button_response, p.angle_snapping, p.angle_tuning, mode_json, fps13
        );
        return Ok(());
    }
    println!(
        "DPI 三档        : {} / {} / {}  （第 4 档未启用）",
        p.dpi[0], p.dpi[1], p.dpi[2]
    );
    println!("回报率 (USB/RF) : {}", polling_label(p.polling_usbrf));
    println!("回报率 (蓝牙)   : {}", polling_label(p.polling_ble));
    println!("回报率 (Booster): {}", polling_label(p.polling_booster));
    match proto::response_ms(p.button_response) {
        Some(ms) => println!("按键响应        : {} ms（值 {}）", ms, p.button_response),
        None => println!("按键响应        : 未知（值 {}）", p.button_response),
    }
    println!("直线修正        : {}", if p.angle_snapping == 1 { "开启" } else { "关闭" });
    println!("角度微调        : {}", p.angle_tuning);
    if sensor_ok {
        println!("传感器模式      : {}", sensor_label(mode));
        println!("帧率            : {}", if fps13 { "13K" } else { "20K" });
    } else {
        println!("传感器模式      : 设备不支持（无应答）");
    }
    Ok(())
}

fn cmd_buttons(dev: &mut Md301, json: bool) -> std::io::Result<()> {
    let base = dev.key_map()?;
    let ext = dev.ext_key_map().ok();
    let add = dev.add_key_map().ok();
    // 十个可控按键：基础表 8 个 + 扩展表里侧边滚轮的左右（sideKey5/6）。
    // 扩展表读不到时这两个值给 null，界面上显示 “—”，不影响其余八项。
    let mut rows: Vec<(&'static str, Option<u16>)> =
        base.entries().iter().map(|(n, c)| (*n, Some(*c))).collect();
    match &ext {
        Some(e) => {
            for (n, c) in e.side_wheel() {
                rows.push((n, Some(c)));
            }
        }
        None => {
            rows.push(("向左滚动", None));
            rows.push(("向右滚动", None));
        }
    }
    if json {
        let mut s = format!("{{\"extReadOk\":{},\"buttons\":{{", ext.is_some());
        for (i, (name, code)) in rows.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            match code {
                Some(c) => s.push_str(&format!("\"{}\":{}", name, c)),
                None => s.push_str(&format!("\"{}\":null", name)),
            }
        }
        s.push_str("}}");
        println!("{}", s);
        return Ok(());
    }
    println!("按键映射:");
    for (name, code) in rows.iter() {
        match code {
            Some(c) => println!("  {:<12} → {}", name, func_name(*c)),
            None => println!("  {:<12} → （扩展表读取失败）", name),
        }
    }
    if let Some(a) = add {
        if a.dpi2 != 0 {
            println!("附加按键映射:");
            println!("  {:<12} → {}", "DPI2 键", func_name(a.dpi2));
        }
    }
    Ok(())
}

fn cmd_power_set(dev: &mut Md301, args: &[String]) -> std::io::Result<()> {
    let cur = dev.power()?;
    let sleep_idx = match opt(args, "--sleep") {
        Some(v) => v.parse::<u8>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "--sleep 需要 0-4 或 255")
        })?,
        None => cur.sleep_time_index,
    };
    let low = match opt(args, "--low") {
        Some(v) => v.parse::<u8>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "--low 需要 0-100")
        })?,
        None => cur.low_power_percent,
    };
    let btype = match opt(args, "--battery") {
        Some(v) => v.parse::<u8>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "--battery 需要 0 或 1")
        })?,
        None => cur.battery_type,
    };
    let effect = match opt(args, "--effect") {
        Some(v) => v.parse::<u8>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "--effect 需要 0 或 1")
        })?,
        None => cur.full_charging_effect,
    };
    dev.set_power_config(sleep_idx, low, btype, effect)?;
    println!(
        "电源设置已更新：休眠={} 低电量={}% 电池={} 满电灯效={}",
        sleep_label(sleep_idx),
        low,
        battery_label(btype),
        effect
    );
    println!("提示: 执行 `md301ctl perf save` 可写入 Flash 永久保存。");
    Ok(())
}

fn cmd_monitor(dev: &mut Md301, secs: u64) -> std::io::Result<()> {
    if dev.evt.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "未找到事件接口 (Usage Page 0xFFC1)",
        ));
    }
    println!(
        "监听事件通道 {} 秒（Ctrl+C 退出）… 提示：切换 DPI 档/配置档时会看到通知。",
        secs
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if let Some(r) = dev.read_event(400)? {
            let sub = u16::from_le_bytes([r[2], r[3]]);
            let desc = match (r[0], r[1], sub) {
                (GET, 0x08, 0x0005) => {
                    let idx = r[4];
                    let v = (u16::from_le_bytes([r[5], r[6]]) as u32 + 1) * DPI_BASE;
                    format!("DPI 切换 → 档 {} 值 {}", idx, v)
                }
                (GET, 0x08, 0x0006) => format!("回报率变化 → {}", polling_label(r[4])),
                (GET, 0x08, other) => format!("事件 0x12:0x08:{:04X}", other),
                (GET, 0x01, 0x0000) => {
                    format!(
                        "设备状态心跳 state={} ({})",
                        r[4],
                        if r[4..20].iter().all(|b| *b == 0) {
                            "空闲/已插线"
                        } else {
                            "活动"
                        }
                    )
                }
                _ => "事件".to_string(),
            };
            println!("[{}] {}  原始: {}", now(), desc, hex(&r[..16]));
        }
    }
    Ok(())
}

fn now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:>10}.{:03}", d.as_secs(), d.subsec_millis())
}
