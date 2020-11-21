pub mod globals;

use anyhow::{Context, Result};
use common::external::{Camera, error_message, success_message};
use common::internal::{Input, handle_controller};
use crate::globals::*;
use memory_rs::internal::injections::*;
use memory_rs::internal::process_info::ProcessInfo;
use memory_rs::{try_winapi, generate_aob_pattern};
use std::fs::OpenOptions;
use std::io::prelude::*;
use winapi::shared::minwindef::LPVOID;
use winapi;

use log::{error, info};
use slog::Drain;
use slog::o;
use slog;
use slog_scope;
use slog_stdlog;
use slog_term;

#[repr(C)]
struct GameCamera {
    pos: [f32; 4],
    focus: [f32; 4],
    rot: [f32; 4],
    padding_: [f32; 0x8],
    fov: f32
}

impl GameCamera {
    pub fn consume_input(&mut self, input: &Input) {
        let r_cam_x = self.focus[0] - self.pos[0];
        let r_cam_y = self.focus[1] - self.pos[1];
        let r_cam_z = self.focus[2] - self.pos[2];

        let (r_cam_x, r_cam_z, r_cam_y) =
            Camera::calc_new_focus_point(r_cam_x, r_cam_z, r_cam_y,
                input.delta_focus.0, input.delta_focus.1);

        self.pos[0] = self.pos[0] + r_cam_x*input.delta_pos.1 +
            input.delta_pos.0*r_cam_z;
        self.pos[1] = self.pos[1] + r_cam_y*input.delta_pos.1;

        self.pos[2] = self.pos[2] + r_cam_z*input.delta_pos.1 -
            input.delta_pos.0*r_cam_x;

        self.focus[0] = self.pos[0] + r_cam_x;
        self.focus[1] = self.pos[1] + r_cam_y;
        self.focus[2] = self.pos[2] + r_cam_z;

        println!("{:p} ; {}", &self, input.engine_speed);
        self.fov = input.fov;
    }
}

impl std::fmt::Debug for GameCamera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = self as *const GameCamera as usize;
        f.debug_struct("GameCamera")
            .field("self", &format_args!("{:x}", ptr))
            .field("pos", &self.pos)
            .field("focus", &self.focus)
            .field("rot", &self.rot)
            .field("fov", &self.fov)
            .finish()
    }
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

fn inject_detourings(proc_inf: &ProcessInfo) -> Result<Vec<Detour>> {
    macro_rules! auto_cast {
        ($val:expr) => {
            &$val as *const u8 as usize
        };
    };

    let mut detours = vec![];

    unsafe {
        let pat = generate_aob_pattern![
            0x90, 0xC5, 0xF8, 0x10, 0x07, 0xC5, 0xF8, 0x11, 0x86, 0x80, 0x00,
            0x00, 0x00
        ];

        let camera_func = Detour::new_from_aob(pat, proc_inf,
            auto_cast!(get_camera_data), Some(&mut _get_camera_data), 15,
            Some(-0x33))
            .with_context(|| "camera_func failed")?;

        info!("camera_func found: {:x}", camera_func.entry_point);
        detours.push(camera_func);

        let pat = generate_aob_pattern![
            0xC5, 0x7A, 0x11, 0x05, 0xC2, 0x32, 0xF8, 0x01, 0xC5, 0xFA, 0x11,
            0x35, 0xBE, 0x32, 0xF8, 0x01
        ];

        let timestop_func = Detour::new_from_aob(pat, proc_inf,
            auto_cast!(get_timestop), Some(&mut _get_timestop), 16, None)
            .with_context(|| "timestop couldn't be found")?;

        info!("timestop_func found: {:x}", timestop_func.entry_point);
        detours.push(timestop_func);
    }

    info!("Injection completed succesfully");
    Ok(detours)
}

fn make_injections(proc_inf: &ProcessInfo) -> Result<Vec<Injection>> {
    let mut v = vec![];

    let fov = Injection::new_from_aob(proc_inf, vec![0x90; 6],
        generate_aob_pattern![
            0x89, 0x86, 0xD0, 0x00, 0x00, 0x00, 0x8B, 0x47,
            0x54, 0x89, 0x86, 0xD4, 0x00, 0x00, 0x00
    ]).with_context(|| "FoV couldn't be found")?;

    v.push(fov);
    Ok(v)
}

#[cfg(not(feature = "ms_store"))]
fn nope_ui_elements(proc_inf: &ProcessInfo) -> Vec<StaticElement> {
    vec![
            StaticElement::new(proc_inf.addr + 0x2829C88),
            StaticElement::new(proc_inf.addr + 0x2829C8C),
            StaticElement::new(proc_inf.addr + 0x2829CAC),
        ]
}

#[cfg(feature = "ms_store")]
fn nope_ui_elements(proc_inf: &ProcessInfo) -> Vec<StaticElement> {
    vec![]
}


fn patch(_: LPVOID) -> Result<()> {
    #[cfg(feature = "non_automatic")]
    success_message("The injection was made succesfully");

    #[cfg(debug_assertions)]
    unsafe {
        use winapi::um::consoleapi::AllocConsole;
        try_winapi!(AllocConsole());
    }

    let proc_inf = ProcessInfo::new("YakuzaLikeADragon.exe")?;

    let mut active = false;

    let mut detours = inject_detourings(&proc_inf)?;
    let mut ui_elements: Vec<StaticElement> = nope_ui_elements(&proc_inf);
    let mut injections = make_injections(&proc_inf)?;
    let mut input = Input::new();

    info!("Starting main loop");

    loop {
        handle_controller(&mut input);
        input.sanitize();

        if input.deattach {
            break;
        }

        if input.change_active {
            active = !active;
            unsafe {
                _camera_active = active as u8;
            }
            println!("Camera is {}", active);

            input.engine_speed = 1e-4;

            if active {
                injections.inject();
            } else {
                ui_elements.remove_injection();
                injections.remove_injection();

                // We need to set _camera_struct to 0 since 
                // the camera struct can change depending on the scene.
                unsafe {
                    _camera_struct = 0;
                }
            }

            input.change_active = false;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        unsafe {
            if (_camera_struct == 0x0) || !active {
                continue;
            }
        }

        let gc = unsafe { (_camera_struct + 0x80) as *mut GameCamera };
        let rot = [0., 1., 0.];
        unsafe {
            std::ptr::copy_nonoverlapping(rot.as_ptr(), (*gc).rot.as_mut_ptr(),
                3);
        }

        unsafe {
            _engine_speed = input.engine_speed;
        }
        ui_elements.inject();

        unsafe {
            (*gc).consume_input(&input);
        }

        input.reset();

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    std::io::stdout().flush()?;

    println!("Dropping values");
    detours.clear();

    std::thread::sleep(std::time::Duration::from_secs(2));

    #[cfg(debug_assertions)]
    unsafe {
        info!("Freeing console");
        use winapi::um::wincon::FreeConsole;
        try_winapi!(FreeConsole());
    }

    info!("Exiting library");

    Ok(())
}

memory_rs::main_dll!(wrapper);
