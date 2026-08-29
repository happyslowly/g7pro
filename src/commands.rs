use crate::{device, protocol};
use hidapi::{HidApi, HidDevice, HidError};
use std::{
    error::Error,
    io, thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const READ_TIMEOUT_MS: i32 = 100;
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(500);

fn find_device(api: &HidApi) -> Result<device::G7ProDeviceInfo> {
    device::find_g7pro(api).ok_or_else(|| "G7 Pro 8K vendor HID interface not found".into())
}

fn open_device(info: &device::G7ProDeviceInfo, api: &HidApi) -> Result<HidDevice> {
    info.open(api).map_err(|e| {
        if let HidError::IoError { error } = &e
            && error.kind() == io::ErrorKind::PermissionDenied
        {
            eprintln!("Permission denied opening the device");
            eprintln!("Run with sudo for now; later you can add a udev rule to drop sudo.");
        }

        e.into()
    })
}

fn print_device_info(info: &device::G7ProDeviceInfo) {
    println!(
        "Found: {:04x} ({})",
        info.product_id(),
        info.product_name().unwrap_or("unknown"),
    );
}

fn write_report(device: &HidDevice, report: &[u8; 64]) -> Result<()> {
    device.write(report)?;

    Ok(())
}

/// --watch / --buttons are both long-running modes: turning the controller on or
/// off makes the USB device behind the receiver fully re-enumerate (product_id
/// changes), which invalidates the already-open HidDevice handle and errors out.
/// The whole process must not exit here — it needs to rediscover and reopen.
fn with_reconnect(mut action: impl FnMut() -> Result<()>) -> Result<()> {
    loop {
        match action() {
            Ok(()) => return Ok(()),

            Err(e) => {
                let permission_denied = matches!(
                    e.downcast_ref::<HidError>(),
                    Some(HidError::IoError { error }) if error.kind() == io::ErrorKind::PermissionDenied
                );

                if permission_denied {
                    return Err(e);
                }

                println!("{e}, retrying device discovery in 2s...");

                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

/// Read the battery level once, or keep monitoring it with `watch`.
pub fn battery(watch: bool, raw: bool) -> Result<()> {
    if !watch {
        return read_battery(watch, raw);
    }

    with_reconnect(|| read_battery(watch, raw))
}

fn read_battery(watch: bool, raw: bool) -> Result<()> {
    println!("Searching for GameSir G7 Pro 8K...");

    let api = HidApi::new()?;

    let info = find_device(&api)?;

    print_device_info(&info);

    let device = open_device(&info, &api)?;

    let heartbeat = protocol::heartbeat_report();

    let timeout = Duration::from_secs(5);

    let mut last_heartbeat = Instant::now() - HEARTBEAT_INTERVAL;

    let started = Instant::now();

    let mut last_state: Option<protocol::Status> = None;

    let mut connected = false;

    loop {
        if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            write_report(&device, &heartbeat)?;

            last_heartbeat = Instant::now();
        }

        let mut buf = [0u8; 64];

        let n = device.read_timeout(&mut buf, READ_TIMEOUT_MS)?;

        if n > 0 {
            if raw {
                println!("RX {n} bytes:");
                protocol::dump(&buf[..n]);
            }

            if let Some(status) = protocol::parse_status(&buf[..n]) {
                if !status.live {
                    //
                    // The receiver/interface is there, but there's no controller
                    // actually connected behind it (stick centers and battery are
                    // all 0, which isn't a normal state).
                    //
                    if connected {
                        connected = false;
                        last_state = None;

                        if watch {
                            println!("Controller not connected");
                        }
                    }
                } else {
                    connected = true;

                    if watch {
                        if last_state != Some(status) {
                            if status.charging {
                                println!("Battery: {}% (charging)", status.battery);
                            } else {
                                println!("Battery: {}%", status.battery);
                            }

                            last_state = Some(status);
                        }
                    } else {
                        //
                        // Default single-shot mode: exit as soon as the first
                        // valid state is read.
                        //
                        if status.charging {
                            println!("{}% charging", status.battery);
                        } else {
                            println!("{}%", status.battery);
                        }

                        return Ok(());
                    }
                }
            }
        }

        if !watch && started.elapsed() >= timeout {
            return Err("no valid 0x12 status report received within 5 seconds".into());
        }
    }
}

pub fn rumble_test() -> Result<()> {
    println!("Searching for GameSir G7 Pro 8K...");

    let api = HidApi::new()?;

    let info = find_device(&api)?;

    print_device_info(&info);

    let device = open_device(&info, &api)?;

    // Send one heartbeat first to make sure the command channel is alive.
    write_report(&device, &protocol::heartbeat_report())?;

    thread::sleep(Duration::from_millis(50));

    println!("Rumble test...");

    write_report(&device, &protocol::rumble_report(0xc0, 0xc0))?;

    thread::sleep(Duration::from_millis(400));

    write_report(&device, &protocol::rumble_report(0x00, 0x00))?;

    println!("Done");

    Ok(())
}

pub fn button_test() -> Result<()> {
    with_reconnect(read_buttons)
}

fn read_buttons() -> Result<()> {
    println!("Searching for GameSir G7 Pro 8K...");

    let api = HidApi::new()?;

    let info = find_device(&api)?;

    print_device_info(&info);

    let device = open_device(&info, &api)?;

    println!("Button test: press any button/stick to see its value, Ctrl+C to exit.");

    let heartbeat = protocol::heartbeat_report();

    let mut last_heartbeat = Instant::now() - HEARTBEAT_INTERVAL;

    let mut last_input: Option<protocol::Input> = None;

    loop {
        if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            write_report(&device, &heartbeat)?;

            last_heartbeat = Instant::now();
        }

        let mut buf = [0u8; 64];

        let n = device.read_timeout(&mut buf, READ_TIMEOUT_MS)?;

        if n == 0 {
            continue;
        }

        let Some(input) = protocol::parse_input(&buf[..n]) else {
            continue;
        };

        if !input.live {
            continue;
        }

        if last_input != Some(input) {
            print_input(&input);

            last_input = Some(input);
        }
    }
}

fn print_input(input: &protocol::Input) {
    let pressed = input.buttons.pressed();

    let pressed = if pressed.is_empty() {
        "-".to_string()
    } else {
        pressed.join(" ")
    };

    println!(
        "L({:3},{:3}) R({:3},{:3}) LT={:3} RT={:3} dpad={:?} | {}",
        input.lx, input.ly, input.rx, input.ry, input.lt, input.rt, input.buttons.dpad, pressed
    );
}
