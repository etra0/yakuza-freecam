.data
EXTERN _get_camera_data: qword
EXTERN _get_timestop: qword

EXTERN _camera_struct: qword
EXTERN _camera_active: byte
EXTERN _engine_speed: dword

; Function that intercepts the values written into the camera
.code
get_camera_data PROC
    pushf
    mov al, _camera_active

    cmp _camera_active, 0
    je original

    cmp _camera_struct, 0
    jne force_ret
    mov _camera_struct, rcx

    force_ret:
    popf
    ret

    original:
    popf
    push rdi
    sub rsp, 40h
    mov qword ptr [rsp + 20h], 0FFFFFFFFFFFFFFFEh
    jmp qword ptr [_get_camera_data]

get_camera_data ENDP

get_timestop PROC
    pushf
    push rax
    mov al, _camera_active
    cmp _camera_active, 0
    je @f
    vmovss xmm8, _engine_speed
    ; vmovss xmm6, _engine_speed
    vmovss xmm3, _engine_speed

    @@:
    mov rax, 143E529DCh
    vmovss dword ptr [rax], xmm8
    mov rax, 143E529E0h
    vmovss dword ptr [rax], xmm6
    mov rax, 143E529ECh
    vmovss dword ptr [rax], xmm3

    pop rax
    popf
    jmp [_get_timestop]
get_timestop ENDP

END
