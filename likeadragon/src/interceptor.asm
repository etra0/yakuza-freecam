.data
EXTERN _camera_struct: qword

PUBLIC get_camera_data_end

;; Function that intercepts the values written into the camera
.code
get_camera_data PROC
    mov _camera_struct, rcx
    ret
get_camera_data_end::
    ALIGN 16
  

get_camera_data ENDP

END
