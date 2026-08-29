//! GameSir G7 Pro 8K's private HID protocol: report IDs, field offsets, heartbeat/status reports.

pub const REPORT_HEARTBEAT: u8 = 0x0f;
pub const REPORT_STATUS: u8 = 0x12;

// G7 Pro 8K enhanced status report
const LX_OFFSET: usize = 1;
const LY_OFFSET: usize = 2;
const RX_OFFSET: usize = 3;
const RY_OFFSET: usize = 4;
const BUTTONS1_OFFSET: usize = 5; // dpad (low nibble) + X/A/B/Y (high nibble)
const BUTTONS2_OFFSET: usize = 6; // LB/RB/LT digital/RT digital/View/Menu/LS/RS
const LT_OFFSET: usize = 8;
const RT_OFFSET: usize = 9;
const CHARGING_OFFSET: usize = 35;
const BATTERY_OFFSET: usize = 36;
const BUTTONS3_OFFSET: usize = 60; // Home/Share/L4/R4/M (paddles/front button, firmware-only)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Status {
    pub battery: u8,
    pub charging: bool,

    /// Whether this frame actually has a controller behind it, rather than being a
    /// placeholder frame from an idle/not-yet-synced receiver (report ID matches,
    /// but the sticks and battery are all 0).
    /// Based on gamesir-linux-tools' has_live_pad()/_streams_live_data(): sticks at
    /// rest read close to 128, never really 0, so any nonzero field counts as live.
    pub live: bool,
}

/// Heartbeat report: 0f f2 00 00 ...
pub fn heartbeat_report() -> [u8; 64] {
    let mut report = [0u8; 64];

    report[0] = REPORT_HEARTBEAT;
    report[1] = 0xf2;

    report
}

/// Rumble test report: 0f 20 66 55 <left> <right> 00 00 ...
/// Commands share the same output report ID (0x0f) as the heartbeat, distinguished
/// by the subcommand byte at report[1].
/// Based on gamesir-linux-tools' control.py: rumble(l, r) -> send_cmd(0x0F, 0x20, 0x66, 0x55, l, r)
pub fn rumble_report(left: u8, right: u8) -> [u8; 64] {
    let mut report = [0u8; 64];

    report[0] = REPORT_HEARTBEAT;
    report[1] = 0x20;
    report[2] = 0x66;
    report[3] = 0x55;
    report[4] = left;
    report[5] = right;

    report
}

/// Parse battery status from a raw report obtained by read().
/// Returns None if it's not a valid 0x12 status report, or a field is clearly invalid.
pub fn parse_status(buf: &[u8]) -> Option<Status> {
    if buf.len() <= BATTERY_OFFSET {
        return None;
    }

    if buf[0] != REPORT_STATUS {
        return None;
    }

    let battery = buf[BATTERY_OFFSET];

    // Guard against treating a clearly invalid value as a percentage
    if battery > 100 {
        return None;
    }

    let charging = (buf[CHARGING_OFFSET] & 0x01) != 0;

    let live = buf[LX_OFFSET] != 0
        || buf[LY_OFFSET] != 0
        || buf[RX_OFFSET] != 0
        || buf[RY_OFFSET] != 0
        || battery != 0;

    Some(Status {
        battery,
        charging,
        live,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dpad {
    #[default]
    Neutral,
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

fn parse_dpad(nibble: u8) -> Dpad {
    match nibble {
        0x0 => Dpad::Up,
        0x1 => Dpad::UpRight,
        0x2 => Dpad::Right,
        0x3 => Dpad::DownRight,
        0x4 => Dpad::Down,
        0x5 => Dpad::DownLeft,
        0x6 => Dpad::Left,
        0x7 => Dpad::UpLeft,
        _ => Dpad::Neutral, // the common "not pressed" values are 8 or 15
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Buttons {
    pub dpad: Dpad,
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub lb: bool,
    pub rb: bool,
    pub lt_digital: bool,
    pub rt_digital: bool,
    pub view: bool,
    pub menu: bool,
    pub ls: bool,
    pub rs: bool,
    pub home: bool,
    pub share: bool,
    pub l4: bool,
    pub r4: bool,
    pub m: bool,
}

impl Buttons {
    /// Names of all currently pressed buttons, in a fixed order.
    pub fn pressed(&self) -> Vec<&'static str> {
        let flags: &[(bool, &str)] = &[
            (self.a, "A"),
            (self.b, "B"),
            (self.x, "X"),
            (self.y, "Y"),
            (self.lb, "LB"),
            (self.rb, "RB"),
            (self.lt_digital, "LT"),
            (self.rt_digital, "RT"),
            (self.view, "View"),
            (self.menu, "Menu"),
            (self.ls, "LS"),
            (self.rs, "RS"),
            (self.home, "Home"),
            (self.share, "Share"),
            (self.l4, "L4"),
            (self.r4, "R4"),
            (self.m, "M"),
        ];

        flags
            .iter()
            .filter(|(pressed, _)| *pressed)
            .map(|(_, name)| *name)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub lt: u8,
    pub rt: u8,
    pub buttons: Buttons,

    /// See Status::live — a placeholder frame has all-zero sticks, which happens to
    /// collide with dpad's 0x0 (Up) encoding. Without filtering this out, a
    /// misleading "dpad=Up" line gets printed right when the controller connects.
    pub live: bool,
}

/// Parse stick/trigger/button state from a raw report obtained by read().
/// Byte mapping is based on gamesir-linux-tools' enhanced.py: parse_enhanced().
pub fn parse_input(buf: &[u8]) -> Option<Input> {
    if buf.len() <= BUTTONS3_OFFSET {
        return None;
    }

    if buf[0] != REPORT_STATUS {
        return None;
    }

    let b5 = buf[BUTTONS1_OFFSET];
    let b6 = buf[BUTTONS2_OFFSET];
    let b60 = buf[BUTTONS3_OFFSET];

    let buttons = Buttons {
        dpad: parse_dpad(b5 & 0x0f),
        x: b5 & 0x10 != 0,
        a: b5 & 0x20 != 0,
        b: b5 & 0x40 != 0,
        y: b5 & 0x80 != 0,
        lb: b6 & 0x01 != 0,
        rb: b6 & 0x02 != 0,
        lt_digital: b6 & 0x04 != 0,
        rt_digital: b6 & 0x08 != 0,
        view: b6 & 0x10 != 0,
        menu: b6 & 0x20 != 0,
        ls: b6 & 0x40 != 0,
        rs: b6 & 0x80 != 0,
        home: b60 & 0x01 != 0,
        share: b60 & 0x02 != 0,
        l4: b60 & 0x08 != 0,
        r4: b60 & 0x10 != 0,
        m: b60 & 0x20 != 0,
    };

    let live = buf[LX_OFFSET] != 0
        || buf[LY_OFFSET] != 0
        || buf[RX_OFFSET] != 0
        || buf[RY_OFFSET] != 0
        || buf[BATTERY_OFFSET] != 0;

    Some(Input {
        lx: buf[LX_OFFSET],
        ly: buf[LY_OFFSET],
        rx: buf[RX_OFFSET],
        ry: buf[RY_OFFSET],
        lt: buf[LT_OFFSET],
        rt: buf[RT_OFFSET],
        buttons,
        live,
    })
}

pub fn dump(data: &[u8]) {
    for (i, byte) in data.iter().enumerate() {
        print!("{byte:02x} ");

        if (i + 1) % 16 == 0 {
            println!();
        }
    }

    if data.len().is_multiple_of(16) {
        println!();
    }
}
