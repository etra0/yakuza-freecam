.data

PUBLIC get_camera_data
PUBLIC get_camera_data_end

PUBLIC get_controller_input
PUBLIC get_controller_input_end

get_camera_data PROC
  push r11
  lea r11,[relpos+0200h-09h];
  relpos:
  pushf
  push rax
  mov eax, [r11-010h]
  test eax, eax
  pop rax
  je not_zero
  movaps xmm1,[r11] ; focus
  movaps xmm0,[r11+020h] ; position
  movaps xmm3,[r11+040h] ; rotation ?? 
  movaps [r9],xmm3
  ; fov stuff
  push rax
  mov rax,[r11+060h]
  mov [rbx+0ACh],rax
  pop rax


not_zero:
  movaps [r11],xmm1
  movaps [r11+020h],xmm0
  ; load rotation
  movaps xmm3,[r9]
  movaps [r11+040h],xmm3 ; camera rotation

  ; load fov
  push rax
  mov rax,[rbx+0ACh]
  mov [r11+060h],rax
  pop rax

  popf
  pop r11
  ; original code
  movaps [rbp-020h],xmm1
  movaps [rbp-030h],xmm0
  ; end original code
  ret
get_camera_data_end::
get_camera_data ENDP

get_controller_input PROC
  push rax
  lea rax,[relpos+200h-8h]
  relpos:
  mov [rax],rbx
  pop rax

  ; original code
  mov r14,r9
  mov rsi,r8
  ret
get_controller_input_end::
get_controller_input ENDP
END
