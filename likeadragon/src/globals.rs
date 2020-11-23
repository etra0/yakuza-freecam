use memory_rs::scoped_no_mangle;

scoped_no_mangle! {
    // Pointer to the camera struct (the lookat is at +0x80 offset
    _camera_struct: usize = 0;

    // Boolean that says if the camera is active
    _camera_active: u8 = 0x0;

    // Address to jmp back after the injection
    _get_camera_data: usize = 0x0;
    _get_timestop: usize = 0x0;
    _get_timestop_rip: usize = 0x0;
    _get_timestop_first_offset: usize = 0x0;

    // Global engine speed to be written by the main dll
    _engine_speed: f32 = 1.;
}

extern "C" {
    pub static get_camera_data: u8;
    pub static get_timestop: u8;
}
