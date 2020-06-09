.data
PUBLIC get_camera_data
PUBLIC get_camera_data_end

PUBLIC get_controller_input
PUBLIC get_controller_input_end

get_camera_data PROC
  push r11
  lea r11,[get_camera_data + 200h];
  pushf
  push rax
  mov eax, [r11-10h]
  test eax, eax
  pop rax
  je not_zero
  movaps xmm4,[r11+40h]
  movaps xmm5,[r11]
  movaps xmm6,[r11+20h] ; 220h
  push rbx
  mov rbx,[r11+60h]
  mov [rax+0ACh],rbx
  pop rbx

not_zero:
  movaps [r11],xmm5
  movaps [r11+20h],xmm6
  movaps [r11+40h],xmm4 ; camera rotation
  
  ; load fov
  push rbx
  mov rbx,[rax+0ACh]
  mov [r11+60h],rbx
  pop rbx

  popf
  pop r11
  movaps [rsp+48h],xmm4 ; adjusted offset of stack pointer + 8
  ret
get_camera_data_end::
get_camera_data ENDP

get_controller_input PROC
  push rax
  lea rax,[get_controller_input+200h]
  mov [rax],rbx
  pop rax

  ; original code
  mov rbp,r9
  mov rsi,r8
  ret
get_controller_input_end::
get_controller_input ENDP

END
