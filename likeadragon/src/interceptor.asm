.data
EXTERN _camera_struct: qword

PUBLIC get_camera_data_end

;; Function that intercepts the values written into the camera
.code
get_camera_data PROC
  lea rdx, [r8 + 40h]
  lea rcx, [rdi + 20h]
  mov _camera_struct, rcx
  vmovups xmm0, [rbx + 10h]
get_camera_data_end::
  ALIGN 16
  

get_camera_data ENDP

END
