use super::mapping::Mapping;
pub struct Tls {
    pub data: Vec<u8>,
    pub callbacks: Vec<usize>,
}
impl Tls {
    pub fn prepare(mapped: &Mapping, bytes: &[u8]) -> Result<Self, String> {
        let pe = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
        let opt = pe + 24;
        let dirs = u32::from_le_bytes(bytes[opt + 108..opt + 112].try_into().unwrap());
        let mut result = Self {
            data: Vec::new(),
            callbacks: Vec::new(),
        };
        if dirs <= 9 {
            return Ok(result);
        }
        let rva = u32::from_le_bytes(bytes[opt + 184..opt + 188].try_into().unwrap()) as usize;
        if rva == 0 {
            return Ok(result);
        }
        let tls = mapped.read(mapped.base() + rva, 40)?;
        let q = |offset| u64::from_le_bytes(tls[offset..offset + 8].try_into().unwrap()) as usize;
        let (start, end, index, callbacks) = (q(0), q(8), q(16), q(24));
        let zero = u32::from_le_bytes(tls[32..36].try_into().unwrap()) as usize;
        let size = end.checked_sub(start).ok_or("invalid TLS template range")?;
        let total = size
            .checked_add(zero)
            .filter(|n| *n <= 16 * 1024 * 1024)
            .ok_or("TLS template too large")?;
        result.data = vec![0; total.max(1)];
        if size > 0 {
            result.data[..size].copy_from_slice(mapped.read(start, size)?);
        }
        mapped.write_u32(index, 0)?;
        if callbacks != 0 {
            for i in 0..64 {
                let bytes = mapped.read(callbacks + i * 8, 8)?;
                let address = u64::from_le_bytes(bytes.try_into().unwrap()) as usize;
                if address == 0 {
                    return Ok(result);
                }
                if !mapped.executable(address) {
                    return Err("TLS callback outside executable section".into());
                }
                result.callbacks.push(address);
            }
            return Err("unterminated TLS callback array".into());
        }
        Ok(result)
    }
    pub fn attach(&self, base: usize) {
        for &address in &self.callbacks {
            eprintln!("TLS callback {address:#x}: DLL_PROCESS_ATTACH");
            let callback: extern "win64" fn(usize, u32, usize) =
                unsafe { std::mem::transmute(address) };
            callback(base, 1, 0);
        }
    }
}
