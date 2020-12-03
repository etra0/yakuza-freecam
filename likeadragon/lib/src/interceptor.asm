.data
EXTERN g_get_camera_data: qword

EXTERN g_get_timestop: qword
EXTERN g_get_timestop_rip: qword
EXTERN g_get_timestop_first_offset: qword
EXTERN g_get_controller: qword

EXTERN xinput_interceptor: qword

EXTERN g_camera_struct: qword
EXTERN g_camera_active: byte
EXTERN g_engine_speed: dword

; Function that intercepts the values written into the camera
.code
asm_get_camera_data PROC
    pushf
    mov al, g_camera_active

    cmp g_camera_active, 0
    je original

    cmp g_camera_struct, 0
    jne force_ret
    mov g_camera_struct, rcx

    force_ret:
    popf
    ret

    original:
    popf
    push rdi
    sub rsp, 40h
    mov qword ptr [rsp + 20h], 0FFFFFFFFFFFFFFFEh
    jmp qword ptr [g_get_camera_data]

asm_get_camera_data ENDP

asm_get_timestop PROC
    pushf
    push rax
    mov al, g_camera_active
    cmp g_camera_active, 0
    je @f
    vmovss xmm8, g_engine_speed
    vmovss xmm6, g_engine_speed
    vmovss xmm3, g_engine_speed

    @@:
    ; If g_get_timestop_rip is 0 we can't start writing to the
    ; right address
    cmp g_get_timestop_rip, 0
    je @f
    cmp g_get_timestop_first_offset, 0
    je @f

    mov rax, g_get_timestop_rip
    add rax, 8h
    add rax, g_get_timestop_first_offset
    vmovss dword ptr [rax], xmm8
    add rax, 4h
    vmovss dword ptr [rax], xmm6
    add rax, 0Ch
    vmovss dword ptr [rax], xmm3

    @@:

    pop rax
    popf
    jmp [g_get_timestop]
asm_get_timestop ENDP

asm_get_controller PROC
    lea rdx, [rsp + 20h]
    mov rsi, r8
    lea rax, xinput_interceptor
    call rax
    test eax, eax

    jmp [g_get_controller]
asm_get_controller ENDP

END
