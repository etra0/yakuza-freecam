#![allow(clippy::clippy::missing_safety_doc)]
pub mod globals;

use crate::globals::*;
use anyhow::{Context, Result};
use common::external::{error_message, Camera};
use common::internal::{handle_controller, Input};
use memory_rs::internal::injections::*;
use memory_rs::internal::memory::{resolve_module_path};
use memory_rs::internal::process_info::ProcessInfo;
use memory_rs::{generate_aob_pattern, try_winapi};
use std::io::prelude::*;
use std::sync::atomic::Ordering;
use winapi::shared::minwindef::LPVOID;
use winapi::um::winuser::{self, GetAsyncKeyState};
use winapi::um::xinput;
use nalgebra_glm as glm;

use log::{error, info};
use simplelog::*;

/// Structure parsed from the game.
#[repr(C)]
struct GameCamera {
    pos: [f32; 4],
    focus: [f32; 4],
    rot: [f32; 4],
    /// We simply skip 8 values because we don't know what they are.
    padding_: [f32; 0x8],
    fov: f32,
}

impl GameCamera {
    pub fn consume_input(&mut self, input: &Input) {
        let r_cam_x = self.focus[0] - self.pos[0];
        let r_cam_y = self.focus[1] - self.pos[1];
        let r_cam_z = self.focus[2] - self.pos[2];

        let (r_cam_x, r_cam_z, r_cam_y) = Camera::calc_new_focus_point(
            r_cam_x,
            r_cam_z,
            r_cam_y,
            input.delta_focus.0,
            input.delta_focus.1,
        );

        self.pos[0] += r_cam_x * input.delta_pos.1 + input.delta_pos.0 * r_cam_z;
        self.pos[1] += r_cam_y * input.delta_pos.1;

        self.pos[2] += r_cam_z * input.delta_pos.1 - input.delta_pos.0 * r_cam_x;

        self.focus[0] = self.pos[0] + r_cam_x;
        self.focus[1] = self.pos[1] + r_cam_y;
        self.focus[2] = self.pos[2] + r_cam_z;

        let focus_ = glm::vec3(self.focus[0], self.focus[1], self.focus[2]);
        let pos_ = glm::vec3(self.pos[0], self.pos[1], self.pos[2]);

        let result = Camera::calculate_rotation(focus_, pos_, input.delta_rotation);
        self.rot[0] = result[0];
        self.rot[1] = result[1];
        self.rot[2] = result[2];

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
        let mut path = resolve_module_path(lib as _).unwrap();
        path.push("ylad.log");
        CombinedLogger::init(vec![
            TermLogger::new(
                log::LevelFilter::Info,
                Config::default(),
                TerminalMode::Mixed,
            ),
            WriteLogger::new(
                log::LevelFilter::Info,
                Config::default(),
                std::fs::File::create(path).unwrap(),
            ),
        ])
        .unwrap();

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

/// `use_xinput_from_game` is in charge to check if the pointer
/// `controller_input_function` is already setted. If the pointer is different
/// from zero, it will actually use the function, if it doesn't, will return
/// an empty `XINPUT_STATE` struct.
pub fn use_xinput_from_game(index: u32, xs: &mut xinput::XINPUT_STATE) -> u32 {
    let xstate: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };
    let function_pointer = controller_input_function.load(Ordering::Relaxed);

    if function_pointer == 0 {
        unsafe { std::ptr::copy_nonoverlapping(&xstate, xs, 1) };
        return 0;
    }

    let func: fn(u32, &mut xinput::XINPUT_STATE) -> u32 =
        unsafe { std::mem::transmute(function_pointer as *const u8) };

    func(index, xs)
}

/// This function will be injected in the game, with the purpose of overriding
/// the input getter. This function will use `use_xinput_from_game` to get
/// the input. It'll also check if the camera is active, in the case it is,
/// it will block all input except the pause button because when you alt-tab
/// in the game, the game will pause.
#[no_mangle]
pub unsafe extern "system" fn xinput_interceptor(index: u32, xs: &mut xinput::XINPUT_STATE) -> u32 {
    let result = use_xinput_from_game(index, xs);

    if g_camera_active == 0 {
        return result;
    }
    // check if the pause button was pressed
    let buttons = (*xs).Gamepad.wButtons & 0x10;

    let mut gamepad: xinput::XINPUT_GAMEPAD = std::mem::zeroed();
    gamepad.wButtons = buttons;

    if g_camera_active == 1 {
        std::ptr::copy_nonoverlapping(&gamepad, &mut (*xs).Gamepad, 1);
    }

    result
}

/// In charge of doing all the `Detour` injections type.
fn inject_detourings(proc_inf: &ProcessInfo) -> Result<Vec<Detour>> {
    macro_rules! auto_cast {
        ($val:expr) => {
            &$val as *const u8 as usize
        };
    };

    let mut detours = vec![];

    unsafe {
        // ---- Camera func ----
        let pat = generate_aob_pattern![
            0x90, 0xC5, 0xF8, 0x10, 0x07, 0xC5, 0xF8, 0x11, 0x86, 0x80, 0x00, 0x00, 0x00
        ];

        let camera_func = Detour::new_from_aob(
            pat,
            &proc_inf.region,
            auto_cast!(asm_get_camera_data),
            Some(&mut g_get_camera_data),
            15,
            Some(-0x33)
        ).with_context(|| "camera_func failed")?;

        info!("camera_func found: {:x}", camera_func.entry_point);
        detours.push(camera_func);
        // ----

        // ---- Timestop ----
        let pat = generate_aob_pattern![
            0xC4, 0xE1, 0xFA, 0x2C, 0xC0, 0x89, 0x05, _, _, _, _, 0xC5, 0x7A, 0x11, 0x05, _, _, _,
            _
        ];

        let timestop_ptr = proc_inf.region.scan_aob(&pat)?.with_context(|| "timestop issues")? + 0xB;


        g_get_timestop_rip = timestop_ptr;
        g_get_timestop_first_offset = *((timestop_ptr + 0x4) as *const u32) as usize;
        info!(
            "_get_timestop_first_offset: {:x}",
            g_get_timestop_first_offset
        );

        let timestop_func = Detour::new(
            timestop_ptr,
            16,
            auto_cast!(asm_get_timestop),
            Some(&mut g_get_timestop),
        );

        info!("timestop_func found: {:x}", timestop_func.entry_point);
        detours.push(timestop_func);
        // ----

        // ---- Controller handler
        let pat = generate_aob_pattern![
            0xE8, _, _, _, _, 0x85, 0xC0, 0x0F, 0x85, _, _, _, _, 0x48, 0x8B, 0x44, 0x24, 0x26,
            0x48, 0x8B, 0x8C, 0x24, 0xD0, 0x00, 0x00, 0x00
        ];

        let controller_blocker = Detour::new_from_aob(
            pat,
            &proc_inf.region,
            auto_cast!(asm_get_controller),
            Some(&mut g_get_controller),
            15,
            Some(-0x8),
        )?;

        let controller_blocker_rip = controller_blocker.entry_point + 0x8;
        let controller_blocker_offset = *((controller_blocker_rip + 0x1) as *const u32) as usize;
        let function_pointer = controller_blocker_rip + controller_blocker_offset;
        controller_input_function.store(function_pointer, Ordering::Relaxed);

        info!(
            "controller_blocker found: {:x}",
            controller_blocker.entry_point
        );
        detours.push(controller_blocker);
    }

    detours.iter_mut().inject();
    info!("injections completed succesfully");
    Ok(detours)
}

/// In charge of making all the `Injection` type of injections.
fn make_injections(proc_inf: &ProcessInfo) -> Result<Vec<Injection>> {
    let mut v = vec![];

    let fov = Injection::new_from_aob(
        &proc_inf.region,
        vec![0x90; 6],
        generate_aob_pattern![
            0x89, 0x86, 0xD0, 0x00, 0x00, 0x00, 0x8B, 0x47, 0x54, 0x89, 0x86, 0xD4, 0x00, 0x00,
            0x00
        ],
    )
    .with_context(|| "FoV couldn't be found")?;
    info!("FoV was found at {:x}", fov.entry_point);
    v.push(fov);

    let no_ui = Injection::new_from_aob(
        &proc_inf.region,
        vec![0xC3],
        generate_aob_pattern![
            0x40, 0x55, 0x48, 0x83, 0xEC, 0x20, 0x80, 0xBA, 0xD4, 0x01, 0x00, 0x00, 0x00, 0x48,
            0x8B, 0xEA, 0x0F, 0x84, _, _, _, _
        ],
    )
    .with_context(|| "no_ui couldn't be found")?;
    info!("no_ui was found at {:x}", no_ui.entry_point);
    v.push(no_ui);

    Ok(v)
}

fn write_ui_elements(proc_inf: &ProcessInfo) -> Result<Vec<StaticElement>> {
    let pat = generate_aob_pattern![
        0xC5, 0xE8, 0x57, 0xD2, 0xC5, 0xF8, 0x57, 0xC0, 0x48, 0x8D, 0x54, 0x24, 0x20, 0xC5, 0xB0,
        0x58, 0x08
    ];

    let ptr = proc_inf.region.scan_aob(&pat)?
        .context("Couldn't find UI values")?
        + 0x11;

    let offset = (ptr + 0x2) as *const u32;
    let offset = unsafe { *offset };
    let rip = ptr + 0x6;

    let base_addr_for_static_numbers = rip + (offset as usize);
    info!(
        "base_addr_for_static_numbers: {:x}",
        base_addr_for_static_numbers
    );

    Ok(vec![
        StaticElement::new(base_addr_for_static_numbers),
        StaticElement::new(base_addr_for_static_numbers + 0x4),
        StaticElement::new(base_addr_for_static_numbers + 0x24),
    ])
}

#[allow(unreachable_code)]
fn patch(_: LPVOID) -> Result<()> {
    #[cfg(feature = "non_automatic")]
    common::external::success_message("The injection was made succesfully");

    #[cfg(debug_assertions)]
    unsafe {
        use winapi::um::consoleapi::AllocConsole;
        try_winapi!(AllocConsole());
    }

    info!(
        "Yakuza Like A Dragon freecam v{} by @etra0",
        common::external::get_version()
    );

    let proc_inf = ProcessInfo::new(None)?;
    info!("{:x?}", proc_inf);

    let mut active = false;

    let mut detours = inject_detourings(&proc_inf)?;
    let mut ui_elements: Vec<StaticElement> = write_ui_elements(&proc_inf)?;
    let mut injections = make_injections(&proc_inf)?;

    let mut input = Input::new();

    info!("Starting main loop");

    loop {
        handle_controller(&mut input, use_xinput_from_game);
        input.sanitize();

        #[cfg(debug_assertions)]
        if input.deattach || (unsafe { GetAsyncKeyState(winuser::VK_HOME) } as u32 & 0x8000) != 0 {
            break;
        }

        input.is_active = active;
        if input.change_active {
            active = !active;
            unsafe {
                g_camera_active = active as u8;
            }
            info!("Camera is {}", active);

            input.engine_speed = 1e-3;
            if active {
                injections.iter_mut().inject();
            } else {
                ui_elements.iter_mut().remove_injection();
                injections.iter_mut().remove_injection();

                // We need to set g_camera_struct to 0 since
                // the camera struct can change depending on the scene.
                unsafe {
                    g_camera_struct = 0;
                }
            }

            input.change_active = false;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        unsafe {
            if (g_camera_struct == 0x0) || !active {
                continue;
            }
        }

        let gc = unsafe { (g_camera_struct + 0x80) as *mut GameCamera };

        unsafe {
            g_engine_speed = input.engine_speed;
        }
        ui_elements.iter_mut().inject();

        unsafe {
            (*gc).consume_input(&input);
            println!("{:?}", input);
        }

        input.reset();

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    std::io::stdout().flush()?;

    info!("Dropping values");
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
