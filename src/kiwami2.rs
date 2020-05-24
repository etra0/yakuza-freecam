use memory_rs::process::process_wrapper::Process;
use winapi::um::winuser;
use winapi::um::winuser::{GetCursorPos, SetCursorPos, GetAsyncKeyState};
use winapi::shared::windef::{POINT};
use std::io::{Error, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};
use std::f32;
use crate::common;

const INITIAL_POS: i32 = 500;

#[naked]
unsafe fn shellcode() {
    llvm_asm!("
    push r11
    lea r11,[rip+0x200-0x9];
    pushf
    push rax
    mov eax, [r11-0x10]
    test eax, eax
    pop rax
    je not_zero
    movaps xmm4,[r11+0x40] // rotation
    movaps xmm10,[r11] // focus
    movaps xmm12,[r11+0x20] // position
    // FOV 
    push rax
    mov rax,[r11+0x60]
    mov [rdx+0x58],rax
    pop rax

not_zero:
    movaps [r11],xmm10
    movaps [r11+0x20],xmm12
    movaps [r11+0x40],xmm4 // camera rotation

    popf
    pop r11
    subps xmm10,xmm12
    movq xmm0,rax
    ret
    nop;nop;nop;nop;
    ": : : : "volatile", "intel");
}

#[naked]
unsafe fn get_pause_value() {
    llvm_asm!("
    push rax
    lea rax,[rdi+0x188]
    mov [rip+0x200-0xF],rax
    pop rax

    // original code
    movzx eax,byte ptr [rdi+0x188]
    ret
    nop;nop;nop;nop;
    ": : : : "volatile", "intel");
}

fn trigger_pause(process: &Process, addr: usize) {
    if addr == 0x0 { return; }
    process.write_value::<u8>(addr, 0x1);
    thread::sleep(Duration::from_millis(20));
    process.write_value::<u8>(addr, 0x0);
}

fn calc_new_focus_point(cam_x: f32, cam_z: f32,
    cam_y: f32, speed_x: f32, speed_y: f32) -> (f32, f32, f32) {

    // use spherical coordinates to add speed
    let theta = cam_z.atan2(cam_x) + speed_x;

    let phi = (cam_x.powi(2) + cam_z.powi(2)).sqrt().atan2(cam_y) +
        speed_y;

    let r = (cam_x.powi(2) + cam_y.powi(2) + cam_z.powi(2)).sqrt();

    let r_cam_x = r*theta.cos()*phi.sin();
    let r_cam_z = r*theta.sin()*phi.sin();
    let r_cam_y = r*phi.cos();

    (r_cam_x, r_cam_z, r_cam_y)
}

pub fn main() -> Result<(), Error> {
    let mut mouse_pos: POINT = POINT::default();

    // latest mouse positions
    let mut latest_x = 0;
    let mut latest_y = 0;

    println!("
    INSTRUCTIONS:

    PAUSE - Activate/Deactivate Free Camera
    END - Deactivate UI
    DEL - Deattach Mouse

    UP, DOWN, LEFT, RIGHT - Move in the direction you're pointing
    PG UP, PG DOWN - Increase/Decrease speed multiplier
    F1, F2 - Increase/Decrease FOV respectively

    WARNING: Don't forget to deactivate the freecam before skipping a cutscene
    (it may cause a game freeze)

    WARNING: Once you deattach the camera (PAUSE), your mouse will be set in a fixed
    position, so in order to attach/deattach the mouse to the camera, you can
    press DEL
    ");

    println!("Waiting for the game to start");
    let yakuza = loop {
        match Process::new("YakuzaKiwami2.exe") {
            Ok(p) => break p,
            Err(_) => (),
        }

        thread::sleep(Duration::from_secs(5));
    };
    println!("Game hooked");

    let entry_point: usize = 0x1F0222B;

    // function that changes the focal length of the cinematics, when
    // active, nop this
    let focal_length_f: Vec<u8> = vec![0x89, 0x86, 0xB8, 0x00, 0x00, 0x00];
    let focal_length_o = 0xB78D87;

    // nop the setcursorpos inside the game
    let set_cursor_call: Vec<u8> = vec![0xFF, 0x15, 0x47, 0x52, 0x4A, 0x00];
    let set_cursor_call_offset = 0x1BA285B;

    // WIP: Pause the cinematics of the world.
    let pause_cinematic_f: Vec<u8> = vec![0x0F, 0x84, 0x5E, 0x02, 0x00, 0x00];
    let pause_cinematic_rep: Vec<u8> = vec![0xE9, 0x5F, 0x02, 0x00, 0x00, 0x90];
    let pause_cinematic_offset = 0xDF6F86;
    let mut pause_world = false;

    let p_shellcode = yakuza.inject_shellcode(entry_point, 9,
        shellcode as usize as *const u8);

    // Pause world entry point
    let pause_value_ep: usize = 0xDF5E1B;
    let pause_value = yakuza.inject_shellcode(pause_value_ep, 7,
        get_pause_value as usize as *const u8);

    // Hide UI stuff
    let hide_ui_f: Vec<u8> = vec![0x41, 0x0F, 0x29, 0x9E, 0x70, 0x01, 0x00, 0x00];
    let hide_ui_rep: Vec<u8> = vec![0x45, 0x0F, 0x29, 0x8E, 0x70, 0x01, 0x00, 0x00];
    let hide_ui_offset: usize = 0x8B2E8C;

    // flashy health bar
    let flashy_bar_f: Vec<u8> = vec![0xC6, 0x04, 0x0B, 0x01];
    let flashy_bar_rep: Vec<u8> = vec![0xC6, 0x04, 0x0B, 0x00];
    let flashy_bar_offset: usize = 0x1B71453;

    let mut active = false;
    let mut capture_mouse = false;

    let mut restart_mouse = false;

    let mut speed_scale = 1.;

    let mut fov = 0.66;


    // fill this value before the initialization
    yakuza.write_value::<f32>(p_shellcode + 0x260, fov);
    loop {
        if capture_mouse & restart_mouse {
            unsafe { SetCursorPos(INITIAL_POS, INITIAL_POS) };
            restart_mouse = !restart_mouse;
            latest_x = INITIAL_POS;
            latest_y = INITIAL_POS;
            continue;
        }

        let start = Instant::now();

        // poll rate
        thread::sleep(Duration::from_millis(10));
        unsafe { GetCursorPos(&mut mouse_pos) };
        let duration = start.elapsed().as_millis() as f32;

        let speed_x = ((mouse_pos.x - latest_x) as f32)/duration/10./(40.-fov*10.);
        let speed_y = ((mouse_pos.y - latest_y) as f32)/duration/10./(40.-fov*10.);

        let c_v_a = yakuza.read_value::<usize>(pause_value+0x200);

        // focus position
        let mut f_cam_x = yakuza.read_value::<f32>(p_shellcode + 0x200);
        let mut f_cam_y = yakuza.read_value::<f32>(p_shellcode + 0x204);
        let mut f_cam_z = yakuza.read_value::<f32>(p_shellcode + 0x208);

        // camera position
        let mut p_cam_x = yakuza.read_value::<f32>(p_shellcode + 0x220);
        let mut p_cam_y = yakuza.read_value::<f32>(p_shellcode + 0x224);
        let mut p_cam_z = yakuza.read_value::<f32>(p_shellcode + 0x228);

        // relative camera position
        let r_cam_x = f_cam_x - p_cam_x;
        let r_cam_y = f_cam_y - p_cam_y;
        let r_cam_z = f_cam_z - p_cam_z;

        let mut dp_forward = 0.;
        let mut dp_sides = 0.;

        unsafe {
            if (GetAsyncKeyState(winuser::VK_UP) as u32 & 0x8000) != 0 {
                dp_forward = 0.1*speed_scale;
            }
            if (GetAsyncKeyState(winuser::VK_DOWN) as u32 & 0x8000) != 0 {
                dp_forward = -0.1*speed_scale;
            }

            if (GetAsyncKeyState(winuser::VK_LEFT) as u32 & 0x8000) != 0 {
                dp_sides = 0.1*speed_scale;
            }
            if (GetAsyncKeyState(winuser::VK_RIGHT) as u32 & 0x8000) != 0 {
                dp_sides = -0.1*speed_scale;
            }

            if (GetAsyncKeyState(winuser::VK_F1) as u32 & 0x8000) != 0 {
                fov += if fov < 3.13 { 0.01 } else { 0. };
            }
            if (GetAsyncKeyState(winuser::VK_F2) as u32 & 0x8000) != 0 {
                fov -= if fov > 0.1 { 0.01 } else { 0. };
            }

        }

        let (r_cam_x, r_cam_z, r_cam_y) = calc_new_focus_point(r_cam_x,
            r_cam_z, r_cam_y, speed_x, speed_y);

        f_cam_x = p_cam_x + r_cam_x + dp_forward*r_cam_x + dp_sides*r_cam_z;
        f_cam_z = p_cam_z + r_cam_z + dp_forward*r_cam_z - dp_sides*r_cam_x;
        f_cam_y = p_cam_y + r_cam_y + dp_forward*r_cam_y;

        p_cam_x = p_cam_x + dp_forward*r_cam_x + dp_sides*r_cam_z;
        p_cam_z = p_cam_z + dp_forward*r_cam_z - dp_sides*r_cam_x;
        p_cam_y = p_cam_y + dp_forward*r_cam_y;

        if capture_mouse {
            yakuza.write_value::<f32>(p_shellcode + 0x200, f_cam_x);
            yakuza.write_value::<f32>(p_shellcode + 0x204, f_cam_y);
            yakuza.write_value::<f32>(p_shellcode + 0x208, f_cam_z);

            yakuza.write_value::<f32>(p_shellcode + 0x220, p_cam_x);
            yakuza.write_value::<f32>(p_shellcode + 0x224, p_cam_y);
            yakuza.write_value::<f32>(p_shellcode + 0x228, p_cam_z);

            yakuza.write_value::<f32>(p_shellcode + 0x240, 0.);
            yakuza.write_value::<f32>(p_shellcode + 0x244, 1.);
            yakuza.write_value::<f32>(p_shellcode + 0x248, 0.);

            // fov
            yakuza.write_value::<f32>(p_shellcode + 0x260, fov);
        }

        latest_x = mouse_pos.x;
        latest_y = mouse_pos.y;

        // to scroll infinitely
        restart_mouse = !restart_mouse;
        unsafe {
            if (GetAsyncKeyState(winuser::VK_PAUSE) as u32 & 0x8000) != 0 {
                active = !active;
                capture_mouse = active;
                yakuza.write_value::<u32>(p_shellcode + 0x1F0, active as u32);

                let c_status = if active { "Deattached" } else { "Attached" };
                println!("status of camera: {}", c_status);

                if active {
                    // nop focal length change
                    yakuza.write_nops(focal_length_o, focal_length_f.len());

                    // nop set cursor pos
                    yakuza.write_nops(set_cursor_call_offset,
                        set_cursor_call.len());

                    yakuza.write_aob(hide_ui_offset, &hide_ui_rep);

                    yakuza.write_aob(flashy_bar_offset, &flashy_bar_rep);

                    // pause stuff
                    yakuza.write_aob(pause_cinematic_offset, &pause_cinematic_rep);
                } else {
                    yakuza.write_aob(focal_length_o, &focal_length_f);

                    yakuza.write_aob(set_cursor_call_offset,
                        &set_cursor_call);

                    yakuza.write_aob(hide_ui_offset, &hide_ui_f);

                    yakuza.write_aob(flashy_bar_offset, &flashy_bar_f);

                    // unpause stuff
                    yakuza.write_aob(pause_cinematic_offset, &pause_cinematic_f);
                }

                trigger_pause(&yakuza, c_v_a);
                thread::sleep(Duration::from_millis(500));
            }

            if active &
                (GetAsyncKeyState(winuser::VK_DELETE) as u32 & 0x8000 != 0) {
                capture_mouse = !capture_mouse;
                let c_status = if !capture_mouse { "Deattached" } else { "Attached" };
                println!("status of mouse: {}", c_status);
                thread::sleep(Duration::from_millis(500));
            }

            if (GetAsyncKeyState(winuser::VK_PRIOR) as u32 & 0x8000) != 0 {
                speed_scale *= 2.;
                println!("Speed increased, {:.2}", speed_scale);
                thread::sleep(Duration::from_millis(100));
            }

            if (GetAsyncKeyState(winuser::VK_NEXT) as u32 & 0x8000) != 0 {
                if speed_scale > 1e-5 {
                    speed_scale /= 2.;
                    println!("Speed decreased, {:.2}", speed_scale);
                } else {
                    println!("Cannot be decreased, {:.2}", speed_scale);
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
