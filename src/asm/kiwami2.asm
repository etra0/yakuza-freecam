.data
PUBLIC get_camera_data
PUBLIC get_camera_data_end

PUBLIC get_pause_value
PUBLIC get_pause_value_end

PUBLIC get_controller_input
PUBLIC get_controller_input_end

;; Function that intercepts the values written into the camera
get_camera_data PROC
    push r11
    lea r11,[relpos+200h-9h];
relpos:
    pushf
    push rax
    mov eax, [r11-10h]
    test eax, eax
    pop rax
    je not_zero
    movaps xmm4,[r11+40h] ; rotation
    movaps xmm10,[r11] ; focus
    movaps xmm12,[r11+20h] ; position
    ; FOV 
    push rax
    mov rax,[r11+60h]
    mov [rdx+58h],rax
    pop rax

not_zero:
    movaps [r11],xmm10
    movaps [r11+20h],xmm12
    movaps [r11+40h],xmm4 ; camera rotation
    push rax
    mov rax,[rdx+58h]
    mov [r11+60h],rax
    pop rax

    popf
    pop r11
    subps xmm10,xmm12
    movq xmm0,rax
    ret
get_camera_data_end::
get_camera_data ENDP

;; Get the focus-window value, useful to set that to 
;; 0 to force the game to pause itself.
get_pause_value PROC
    push rax
    push rbx
    lea rax,[rdi+188h]
    lea rbx,[relpos+200h-13h]
    mov [rbx],rax
relpos:
    pop rbx
    pop rax

    ; original code
    movzx eax,byte ptr [rdi+188h]
    ret
get_pause_value_end::
get_pause_value ENDP

;; Intercept the controller input when controller is detected
get_controller_input PROC
  push rax
  push rbx
  mov rax,[rsp+10h]
  lea rbx,[relpos+200h-11h]
  mov [rbx],rax
relpos:
  pop rbx
  pop rax

  ; original code
  test eax,eax
  mov rax,[rsp+108h+8h] ; adjusted stack offset
  ret
get_controller_input_end::
get_controller_input ENDP

END
