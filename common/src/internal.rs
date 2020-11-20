use memory_rs::internal::memory::{write_aob};
/// Struct that contains an entry point relative to the executable,
/// the original bytes (`f_orig`) and the bytes to be injected (`f_rep`)
///
pub struct Injection {
    /// Entry point relative to the executable
    pub entry_point: usize,
    /// Original bytes
    pub f_orig: Vec<u8>,
    /// Bytes to be injected
    pub f_rep: Vec<u8>,
}


impl Injection {
    pub fn new(entry_point: usize, f_rep: Vec<u8>) -> Injection {
        let aob_size = f_rep.len();
        let slice = unsafe { std::slice::from_raw_parts(entry_point as *const u8, aob_size) };
        let mut f_orig = Vec::new();
        f_orig.extend_from_slice(slice);

        Injection {
            entry_point,
            f_orig,
            f_rep
        }
    }

    pub fn inject(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            write_aob(self.entry_point, &(self.f_rep))?;
        }
        Ok(())
    }

    pub fn remove_injection(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            write_aob(self.entry_point, &(self.f_orig))?;
        }
        Ok(())
    }
}

