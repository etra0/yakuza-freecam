use memory_rs::internal::{
    injections::{Detour, Inject, Injection},
    memory::resolve_module_path,
    process_info::ProcessInfo,
};
use winapi::um::consoleapi::AllocConsole;
use winapi::um::libloaderapi::FreeLibraryAndExitThread;
use winapi::um::wincon::FreeConsole;
use winapi::um::winuser;
use winapi::um::xinput;
use winapi::shared::minwindef::LPVOID;

use log::*;
use simplelog::*;

mod camera;
mod dolly;
mod globals;
mod utils;

use camera::*;
use dolly::*;
use globals::*;
use utils::{check_key_press, error_message, handle_keyboard, Input, Keys};

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn write_red(msg: &str) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    writeln!(&mut stdout, "{}", msg)?;
    stdout.reset()
}

unsafe extern "system" fn wrapper(lib: LPVOID) -> u32 {
    AllocConsole();
    {
        let mut path = resolve_module_path(lib).unwrap();
        path.push("yakuza6.log");
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
            Ok(_) => (),
            Err(e) => {
                let msg = format!("Something went wrong:\n{}", e);
                error!("{}", msg);
                error_message(&msg);
            }
        }
    }

    FreeConsole();
    FreeLibraryAndExitThread(lib as _, 0);
    0
}

fn get_camera_function(proc_inf: &ProcessInfo) -> Result<Vec<Detour>, Box<dyn std::error::Error>> {
    let mut results = vec![];
    // Camera stuff
    let pat = memory_rs::generate_aob_pattern![0xC5, 0xF8, 0x11, 0x56, 0x30, 0xC5, 0xF8, 0x10, 0x47, 0x10];
    let cam = unsafe { Detour::new_from_aob(pat, proc_inf, &asm_get_camera_data as *const u8 as
        usize, Some(&mut g_get_camera_data), 15, None)? };

    results.push(cam);

    Ok(results)
}

fn patch(_lib: LPVOID) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Yakuza 6 freecam by @etra0, v{}",
        utils::get_version()
    );
    write_red("If you close this window the game will close. Use HOME to deattach the freecamera (will close this window as well).")?;
    println!("{}", utils::INSTRUCTIONS);
    write_red("Controller input will only be detected if Xinput is used in the Control settings, otherwise use the keyboard.")?;
    let proc_inf = ProcessInfo::new(None)?;

    let mut input = Input::new();

    let mut active = false;

    let mut points: Vec<CameraSnapshot> = vec![];

    // This variable will hold the initial position when the freecamera is activated.
    let mut starting_point: Option<CameraSnapshot> = None;

    let mut detours = get_camera_function(&proc_inf)?;

    detours.inject();

    let mut nops = vec![
        Injection::new(detours[0].entry_point + 0x14, vec![0x90; 5]),
        Injection::new(detours[0].entry_point + 0x1E, vec![0x90; 5]),
        Injection::new(detours[0].entry_point + 0x39, vec![0x90; 3]),
        Injection::new(proc_inf.region.start_address + 0xAE0033, vec![0x90; 6]),
    ];

    let xinput_func =
        |a: u32, b: &mut xinput::XINPUT_STATE| -> u32 { unsafe { xinput::XInputGetState(a, b) } };

    loop {
        utils::handle_controller(&mut input, xinput_func);
            (&mut input);
        input.sanitize();

        if input.deattach || check_key_press(winuser::VK_HOME) {
            info!("Exiting");
            break;
        }

        input.is_active = active;
        if input.change_active {
            active = !active;

            unsafe {
                g_camera_active = active as u8;
            }
            info!("Camera is {}", active);

            if active {
                nops.inject();
            } else {
                nops.remove_injection();
                starting_point = None;
            }

            input.change_active = false;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        unsafe {
            // If we don't have the camera struct we need to skip it right away
            if g_camera_struct == 0x0 {
                continue;
            }

            let gc = g_camera_struct as *mut GameCamera;
            if !active {
                input.fov = (*gc).fov.into();
                continue;
            }

            if starting_point.is_none() {
                starting_point = Some(CameraSnapshot::new(&(*gc)));
            }

            // if let Some(ref p) = starting_point {
            //     (*gc).clamp_distance(&p.pos);
            // }

            if !points.is_empty() {
                let origin = (*gc).pos.into();
                if utils::calc_eucl_distance(&origin, &points[0].pos) > 400. {
                    warn!("Sequence cleaned to prevent game crashing");
                    points.clear();
                }
            }

            if check_key_press(winuser::VK_F9) {
                let cs = CameraSnapshot::new(&(*gc));
                info!("Point added to interpolation: {:?}", cs);
                points.push(cs);
                std::thread::sleep(std::time::Duration::from_millis(400));
            }

            if check_key_press(winuser::VK_F11) {
                info!("Sequence cleaned!");
                points.clear();
                std::thread::sleep(std::time::Duration::from_millis(400));
            }

            if check_key_press(Keys::P as _) & (points.len() > 1) {
                let dur = std::time::Duration::from_secs_f32(input.dolly_duration);
                points.interpolate(&mut (*gc), dur, false);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            if check_key_press(Keys::L as _) & (points.len() > 1) {
                let dur = std::time::Duration::from_secs_f32(input.dolly_duration);
                points.interpolate(&mut (*gc), dur, true);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            /*
            if check_key_press(winuser::VK_F7) {
                input.unlock_character = !input.unlock_character;
                if input.unlock_character {
                    nops.last_mut().unwrap().remove_injection();
                } else {
                    nops.last_mut().unwrap().inject();
                }
                info!("Unlock character: {}", input.unlock_character);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            */

            // if input.unlock_character {
            //     continue;
            // };

            (*gc).consume_input(&input);
            println!("{:?}", *gc);
        }

        input.reset();

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}

memory_rs::main_dll!(wrapper);
