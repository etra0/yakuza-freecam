pub mod globals;

use common::external::{Camera, error_message};
use crate::globals::*;
use memory_rs::internal::memory::Detour;
use memory_rs::internal::process_info::ProcessInfo;
use memory_rs::{try_winapi, generate_aob_pattern};
use std::fs::OpenOptions;
use std::io::prelude::*;
use winapi::shared::minwindef::LPVOID;
use winapi::um::xinput;
use winapi;

use log::{error, info};
use slog::Drain;
use slog::o;
use slog;
use slog_scope;
use slog_stdlog;
use slog_term;

// TODO: remove this
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[repr(C)]
#[derive(Debug)]
struct GameCamera {
    pos: [f32; 4],
    focus: [f32; 4],
    rot: [f32; 4],
}

pub unsafe extern "system" fn wrapper(lib: LPVOID) -> u32 {
    // Logging initialization
    {
        let log_path = "ylad.log";
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .unwrap();

        let decorator = slog_term::PlainSyncDecorator::new(file);
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        let _guard = slog_scope::set_global_logger(logger);

        slog_stdlog::init().unwrap();

        match patch(lib) {
            Ok(_) => {
                info!("Everything executed perfectly");
            }
            Err(e) => {
                let msg = format!("{}", e);
                error!("Error: {}", msg);
                {
                    use winapi::um::wincon::FreeConsole;
                    (FreeConsole());
                }
                error_message(&msg);
            }
        };

        info!("Exiting");
    }

    winapi::um::libloaderapi::FreeLibraryAndExitThread(
        lib as winapi::shared::minwindef::HMODULE,
        0,
    );

    0
}

fn make_injections(proc_inf: &ProcessInfo) -> Result<Vec<Detour>> {
    macro_rules! auto_cast {
        ($val:expr) => {
            &$val as *const u8 as usize
        };
    };

    let mut detours = vec![];

    unsafe {
        let pat = generate_aob_pattern![0x90, 0xC5, 0xF8, 0x10, 0x07, 0xC5,
            0xF8, 0x11, 0x86, 0x80, 0x00, 0x00, 0x00];
        let camera_func = Detour::new_from_aob(pat, proc_inf,
            auto_cast!(get_camera_data), Some(&mut _get_camera_data), 15,
            Some(-0x33))?;

        info!("camera_func found: {:x}", camera_func.entry_point);
        detours.push(camera_func);

        let pat = generate_aob_pattern![0xC5, 0x7A, 0x11, 0x05, 0xC2, 0x32,
            0xF8, 0x01, 0xC5, 0xFA, 0x11, 0x35, 0xBE, 0x32, 0xF8, 0x01];
        let timestop_func = Detour::new_from_aob(pat, proc_inf,
            auto_cast!(get_timestop), Some(&mut _get_timestop), 16, None)?;

        info!("timestop_func found: {:x}", timestop_func.entry_point);
        detours.push(timestop_func);
    }

    info!("Injection completed succesfully");
    Ok(detours)
}

// Asume safety of XInputGameState
fn xinput_get_state(xinput_state: &mut xinput::XINPUT_STATE) -> Result<()> {
    use xinput::XInputGetState;
    let wrapper = |xs| -> u32 {
        let res = unsafe { XInputGetState(0, xs) };
        if res == 0 {
            return 1
        }
        return 0
    };

    try_winapi!(
        wrapper(xinput_state)
    );

    Ok(())
}

struct Input {
    engine_speed: f32,
    // Deltas with X and Y
    delta_pos: (f32, f32),
    delta_focus: (f32, f32),

    delta_altitude: f32,

    change_active: bool,
}

trait Inject {
    fn inject(&mut self);
    fn remove_injection(&mut self);
}

// UIElements of the game. original_value will be copied when the
// struct gets initializated. The value will be deleted again when
// you do a remove_injection
struct UIElement {
    addr: usize,
    original_value: Option<u32>
}


impl UIElement {
    pub fn new(addr: usize) -> UIElement {
        let original_value = unsafe { Some(*(addr as *mut u32)) };

        UIElement { addr, original_value }
    }

}

impl Inject for UIElement {
    fn inject(&mut self) {
        unsafe {
            let ptr = self.addr as *mut u32;
            if self.original_value.is_none() {
                self.original_value = Some(*ptr);
            }
            *ptr = 0;
        }
    }

    fn remove_injection(&mut self) {
        if self.original_value.is_none() {
            return;
        }
        unsafe {
            let ptr = self.addr as *mut u32;
            *ptr = self.original_value.unwrap();
        }

        self.original_value = None;
    }
}

impl<T> Inject for Vec<T> where T: Inject {
    fn inject(&mut self) {
        self
            .iter_mut()
            .for_each(|x| (*x).inject());
    }
    fn remove_injection(&mut self) {
        self
            .iter_mut()
            .for_each(|x| (*x).remove_injection());
    }
}

fn patch(_: LPVOID) -> Result<()> {
    unsafe {
        use winapi::um::consoleapi::AllocConsole;
        try_winapi!(AllocConsole());
    }

    let proc_inf = ProcessInfo::new("YakuzaLikeADragon.exe")?;
    let mut detours = make_injections(&proc_inf)?;

    let mut xs: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };

    let mut active = false;
    let mut current_speed = 1_f32;

    let mut original_ui: Option<Vec<u32>> = None;
    let mut ui_elements: Vec<UIElement> = vec![
        UIElement::new(proc_inf.addr + 0x2829C88),
        UIElement::new(proc_inf.addr + 0x2829C8C),
        UIElement::new(proc_inf.addr + 0x2829CAC),
    ];

    info!("Starting main loop");

    loop {
        xinput_get_state(&mut xs)?;
        let gp = xs.Gamepad;

        if (gp.wButtons & (0x40 | 0x80)) == (0x40 | 0x80) {
            active = !active;
            println!("Camera is {}", active);
            unsafe {
                _camera_active = active as u8;
            }
            current_speed = 1e-4;
            if !active {
                ui_elements.remove_injection();
                unsafe {
                    _camera_struct = 0;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        if (gp.wButtons & 0x100) == 0x100 {
            current_speed -= 0.01;
            if current_speed < 1e-4 {
                current_speed = 1e-4;
            }
        }

        if (gp.wButtons & 0x200) == 0x200 {
            current_speed += 0.01;
        }

        if (gp.wButtons & (0x1000 | 0x4000)) == (0x1000 | 0x4000) {
            info!("Exiting main loop");
            unsafe {
                _camera_active = 0;
            }
            break
        }

        unsafe {
            if _camera_struct == 0x0 {
                // println!("Camera struct is zero");
                continue;
            }
        }

        if !active {
            continue;
        }

        let gc = unsafe { (_camera_struct + 0x80) as *mut GameCamera };
        let rot = [0., 1., 0.];
        unsafe {
            std::ptr::copy_nonoverlapping(rot.as_ptr(), (*gc).rot.as_mut_ptr(),
                3);
        }

        unsafe {
        _engine_speed = current_speed;
        ui_elements.inject();
        }
        let r_cam_x = unsafe { (*gc).focus[0] - (*gc).pos[0] };
        let r_cam_y = unsafe { (*gc).focus[1] - (*gc).pos[1] };
        let r_cam_z = unsafe { (*gc).focus[2] - (*gc).pos[2] };

        let p_speed_x = -(gp.sThumbLX as f32) / ((i16::MAX as f32)*1e2);
        let p_speed_y = (gp.sThumbLY as f32)  / ((i16::MAX as f32)*1e2);
        let speed_x = (gp.sThumbRX as f32)    / ((i16::MAX as f32)*1e2);
        let speed_y = -(gp.sThumbRY as f32)   / ((i16::MAX as f32)*1e2);

        let (r_cam_x, r_cam_z, r_cam_y) =
            Camera::calc_new_focus_point(r_cam_x, r_cam_z, r_cam_y,
                speed_x, speed_y);

        unsafe {
            (*gc).pos[0] = (*gc).pos[0] + r_cam_x*p_speed_y +
                p_speed_x*r_cam_z;
            (*gc).pos[1] = (*gc).pos[1] + r_cam_y*p_speed_y;

            (*gc).pos[2] = (*gc).pos[2] + r_cam_z*p_speed_y -
                p_speed_x*r_cam_x;

            (*gc).focus[0] = (*gc).pos[0] + r_cam_x;
            (*gc).focus[1] = (*gc).pos[1] + r_cam_y;
            (*gc).focus[2] = (*gc).pos[2] + r_cam_z;

            println!("{:?} ; {:x}, {}", (*gc), gp.wButtons, current_speed);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    std::io::stdout().flush()?;

    println!("Dropping values");
    detours.clear();

    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Freeing console");
    unsafe {
        use winapi::um::wincon::FreeConsole;
        try_winapi!(FreeConsole());
    }

    info!("Exiting library");

    Ok(())
}

memory_rs::main_dll!(wrapper);
