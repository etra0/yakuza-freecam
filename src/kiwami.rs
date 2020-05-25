use memory_rs::process::process_wrapper::Process;
use winapi::um::winuser;
use winapi::um::winuser::{GetCursorPos, SetCursorPos, GetAsyncKeyState};
use winapi::shared::windef::{POINT};
use std::io::{Error, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};
use std::f32;
use crate::common::{Camera, Injection};

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
    // movaps xmm4,[r11+0x40]
    movaps xmm1,[r11] // focus
    movaps xmm0,[r11+0x20] // position
    movaps xmm3,[r11+0x40] // rotation ?? 
    movaps [rsp+0x88],xmm3
    // fov stuff
    push rax
    mov rax,[r11+0x60]
    mov [rbx+0xAC],rax
    pop rax


not_zero:
    movaps [r11],xmm1
    movaps [r11+0x20],xmm0
    // load rotation
    movaps xmm3,[rsp+0x88]
    movaps [r11+0x40],xmm3 // camera rotation

    // load fov
    push rax
    mov rax,[rbx+0xAC]
    mov [r11+0x60],rax
    pop rax

    popf
    pop r11
    // original code
    movaps [rbp-0x20],xmm1
    movaps [rbp-0x30],xmm0
    // end original code
    ret
    nop;nop;nop;nop;
    ": : : : "volatile", "intel");
}

pub fn main() -> Result<(), Error> {
    let mut mouse_pos: POINT = POINT::default();

    // latest mouse positions
    let mut latest_x = 0;
    let mut latest_y = 0;

    println!("
    INSTRUCTIONS:

    PAUSE - Activate/Deactivate Free Camera
    END - Pause the cinematic
    DEL - Deattach Mouse

    UP, DOWN, LEFT, RIGHT - Move in the direction you're pointing
    CTRL, SPACE - Move UP or DOWN
    PG UP, PG DOWN - Increase/Decrease speed multiplier
    F1, F2 - Increase/Decrease FOV respectively

    WARNING: Once you deattach the camera (PAUSE), your mouse will be set in a fixed
    position, so in order to attach/deattach the mouse to the camera, you can
    press DEL

    WARNING: If you're in freeroam and you stop hearing audio, it's probably
    because you have the paused option activated, simply press END to deactivate it.
    ");

    println!("Waiting for the game to start");
    let yakuza = loop {
        match Process::new("YakuzaKiwami.exe") {
            Ok(p) => break p,
            Err(_) => (),
        }

        thread::sleep(Duration::from_secs(5));
    };
    println!("Game hooked");

    let entry_point: usize = 0x30CC33;
    let entry_point_size: usize = 8;
    let p_shellcode = yakuza.inject_shellcode(entry_point, entry_point_size,
        shellcode as usize as *const u8);

    let mut cam = Camera::new(p_shellcode);

    // function that changes the focal length of the cinematics, when
    // active, nop this
    cam.injections.push(Injection {
        entry_point: 0x187616,
        f_orig: vec![0xF3, 0x0F, 0x11, 0x89, 0xAC, 0x00, 0x00, 0x00],
        f_rep: vec![0x90; 8]
    });

    // WIP: Pause the cinematics of the world.
    let pause_cinematic_f: Vec<u8> = vec![0x41, 0x8A, 0x8D, 0xD1, 0x00, 0x00, 0x00];
    let pause_cinematic_rep: Vec<u8> = vec![0xB1, 0x01, 0x90, 0x90, 0x90, 0x90, 0x90];
    let pause_cinematic_offset = 0x7BB8C;
    let mut pause_world = false;


    let mut active = false;
    let mut capture_mouse = false;

    let mut restart_mouse = false;

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

        let speed_x = ((mouse_pos.x - latest_x) as f32)/duration/100.;
        let speed_y = ((mouse_pos.y - latest_y) as f32)/duration/100.;


        if capture_mouse {
            cam.update_position(&yakuza, speed_x, speed_y);
        }

        latest_x = mouse_pos.x;
        latest_y = mouse_pos.y;

        // to scroll infinitely
        restart_mouse = !restart_mouse;
        unsafe {
            if (GetAsyncKeyState(winuser::VK_PAUSE) as u32 & 0x8000) != 0 {
                active = !active;
                capture_mouse = active;

                let c_status = if active { "Deattached" } else { "Attached" };
                println!("status of camera: {}", c_status);
                if active {
                    cam.deattach(&yakuza);
                } else {
                    cam.attach(&yakuza);
                }
                thread::sleep(Duration::from_millis(500));
            }

            if active & (GetAsyncKeyState(winuser::VK_DELETE) as u32 & 0x8000 != 0) {
                capture_mouse = !capture_mouse;
                let c_status = if !capture_mouse { "Deattached" } else { "Attached" };
                println!("status of mouse: {}", c_status);
                thread::sleep(Duration::from_millis(500));
            }

            if (GetAsyncKeyState(winuser::VK_END) as u32 & 0x8000) != 0 {
                pause_world = !pause_world;
                println!("status of pausing: {}", pause_world);
                if pause_world {
                    yakuza.write_aob(pause_cinematic_offset, &pause_cinematic_rep);
                } else {
                    yakuza.write_aob(pause_cinematic_offset, &pause_cinematic_f);
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
}
