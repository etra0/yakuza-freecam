#![allow(non_upper_case_globals)]

use memory_rs::scoped_no_mangle;
use std::sync::atomic::AtomicUsize;

scoped_no_mangle! {
    // Pointer to the camera struct (the lookat is at +0x80 offset
    g_camera_struct: usize = 0;

    // Boolean that says if the camera is active
    g_camera_active: u8 = 0x0;

    // Address to jmp back after the injection
    g_get_camera_data: usize = 0x0;
    g_get_timestop: usize = 0x0;
    g_get_timestop_rip: usize = 0x0;
    g_get_timestop_first_offset: usize = 0x0;
    g_get_controller: usize = 0x0;

    // Global engine speed to be written by the main dll
    g_engine_speed: f32 = 1.;
}

/// This pointer will contain the function that either steam or 
/// ms store version uses, since steam overrides the xinput in order
/// to be able to use more controller options.
pub static controller_input_function: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    pub static asm_get_camera_data: u8;
    pub static asm_get_timestop: u8;
    pub static asm_get_controller: u8;
}
