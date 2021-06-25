.data
EXTERN g_get_camera_data: qword
EXTERN g_camera_active: byte
EXTERN g_camera_struct: qword

.code
asm_get_camera_data PROC
    pushf

    ; Steal the camera pointer
    push rbx
    lea rbx, [rsi + 30h]
    mov [g_camera_struct], rbx
    pop rbx

    cmp g_camera_active, 1
    je ending

    original:
    vmovups [rsi + 30h], xmm2
    vmovups xmm0, [rdi + 10h]
    vmovups [rsi + 40h], xmm0

    ending:
    popf
    jmp [g_get_camera_data]
asm_get_camera_data ENDP


END
