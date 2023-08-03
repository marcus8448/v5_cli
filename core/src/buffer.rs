use std::ffi::CStr;
use std::io::Result;
use std::mem::size_of;

pub trait WriteBuffer {
    fn write_u8(&mut self, value: u8) -> Result<()>;
    fn write_i8(&mut self, value: i8) -> Result<()>;

    fn write_u16(&mut self, value: u16) -> Result<()>;
    fn write_i16(&mut self, value: i16) -> Result<()>;

    fn write_u32(&mut self, value: u32) -> Result<()>;
    fn write_i32(&mut self, value: i32) -> Result<()>;

    fn write_u64(&mut self, value: u64) -> Result<()>;
    fn write_i64(&mut self, value: i64) -> Result<()>;

    fn write_u128(&mut self, value: u128) -> Result<()>;
    fn write_i128(&mut self, value: i128) -> Result<()>;

    fn write_f32(&mut self, value: f32) -> Result<()>;
    fn write_f64(&mut self, value: f64) -> Result<()>;

    fn write(&mut self, slice: &[u8]) -> Result<()>;

    fn write_str(&mut self, string: &str, target_len: usize) -> Result<()>;

    fn pad(&mut self, amount: usize) -> Result<()>;

    fn get_data(self) -> Box<[u8]>;
}

pub trait ReadBuffer {
    fn read_u8(&mut self) -> Result<u8>;
    fn read_i8(&mut self) -> Result<i8>;

    fn read_u16(&mut self) -> Result<u16>;
    fn read_i16(&mut self) -> Result<i16>;

    fn read_u32(&mut self) -> Result<u32>;
    fn read_i32(&mut self) -> Result<i32>;

    fn read_u64(&mut self) -> Result<u64>;
    fn read_i64(&mut self) -> Result<i64>;

    fn read_u128(&mut self) -> Result<u128>;
    fn read_i128(&mut self) -> Result<i128>;

    fn read_f32(&mut self) -> Result<f32>;
    fn read_f64(&mut self) -> Result<f64>;

    fn read(&mut self, slice: &mut [u8]) -> Result<()>;

    fn read_str(&mut self, target_len: usize) -> Result<String>;

    fn read_padded_str(&mut self, target_len: usize) -> Result<String>;

    fn skip(&mut self, amount: usize) -> Result<()>;

    fn read_all(&mut self) -> Box<[u8]>;
}

pub struct FixedWriteBuffer<const LENGTH: usize> {
    pos: usize,
    data: [u8; LENGTH]
}

impl<const LENGTH: usize> FixedWriteBuffer<LENGTH> {
    pub fn new() -> Self {
        FixedWriteBuffer {
            pos: 0,
            data: [0; LENGTH]
        }
    }
}

impl<const LENGTH: usize> WriteBuffer for FixedWriteBuffer<LENGTH> {
    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u8>();
        Ok(())
    }

    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i8>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i8>();
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u16>();
        Ok(())
    }

    fn write_i16(&mut self, value: i16) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i16>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i16>();
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u32>();
        Ok(())
    }

    fn write_i32(&mut self, value: i32) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i32>();
        Ok(())
    }

    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u64>();
        Ok(())
    }

    fn write_i64(&mut self, value: i64) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i64>();
        Ok(())
    }

    fn write_u128(&mut self, value: u128) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<u128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<u128>();
        Ok(())
    }

    fn write_i128(&mut self, value: i128) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<i128>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<i128>();
        Ok(())
    }

    fn write_f32(&mut self, value: f32) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f32>();
        Ok(())
    }

    fn write_f64(&mut self, value: f64) -> Result<()> {
        self.data[self.pos..self.pos + size_of::<f64>()].copy_from_slice(&value.to_le_bytes());
        self.pos += size_of::<f64>();
        Ok(())
    }

    fn write(&mut self, slice: &[u8]) -> Result<()> {
        self.data[self.pos..self.pos + slice.len()].copy_from_slice(slice);
        self.pos += slice.len();
        Ok(())
    }

    fn write_str(&mut self, string: &str, target_len: usize) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len);
        self.data[self.pos..self.pos + string.len()].copy_from_slice(string.as_bytes());
        self.pos += string.len() + 1;
        self.data[self.pos - 1] = 0; // null terminator
        self.pad((target_len - 1) - string.len())?;
        Ok(())
    }

    fn pad(&mut self, amount: usize) -> Result<()> {
        for x in 0..amount {
            self.data[self.pos + x] = 0;
        }
        self.pos += amount;
        Ok(())
    }

    fn get_data(self) -> Box<[u8]> {
        Box::new(self.data)
    }
}

pub struct DynamicWriteBuffer {
    data: Vec<u8>
}

impl DynamicWriteBuffer {
    pub fn new() -> Self {
        DynamicWriteBuffer {
            data: Vec::with_capacity(128)
        }
    }
}

impl WriteBuffer for DynamicWriteBuffer {
    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i16(&mut self, value: i16) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i32(&mut self, value: i32) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i64(&mut self, value: i64) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u128(&mut self, value: u128) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_i128(&mut self, value: i128) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_f32(&mut self, value: f32) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_f64(&mut self, value: f64) -> Result<()> {
        self.data.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write(&mut self, slice: &[u8]) -> Result<()> {
        self.data.extend_from_slice(slice);
        Ok(())
    }

    fn write_str(&mut self, string: &str, target_len: usize) -> Result<()> {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len);

        self.data.extend_from_slice(string.as_bytes());
        self.data.push(0); // null terminator
        self.pad(target_len - (string.len() + 1))?;
        Ok(())
    }

    fn pad(&mut self, amount: usize) -> Result<()> {
        for _ in 0..amount {
            self.data.push(0)
        }
        Ok(())
    }

    fn get_data(self) -> Box<[u8]> {
        self.data.into_boxed_slice()
    }
}

pub struct FixedReadBuffer<const LENGTH: usize> {
    pos: usize,
    data: [u8; LENGTH]
}

impl<const LENGTH: usize> FixedReadBuffer<LENGTH> {
    pub fn new(data: [u8; LENGTH]) -> Self {
        FixedReadBuffer {
            pos: 0,
            data
        }
    }
}

impl<const LENGTH: usize> ReadBuffer for FixedReadBuffer<LENGTH> {
    fn read_u8(&mut self) -> Result<u8> {
        let out = u8::from_le_bytes(self.data[self.pos..self.pos + size_of::<u8>()].try_into().unwrap());
        self.pos += size_of::<u8>();
        return Ok(out);
    }

    fn read_i8(&mut self) -> Result<i8> {
        let out = i8::from_le_bytes(self.data[self.pos..self.pos + size_of::<i8>()].try_into().unwrap());
        self.pos += size_of::<i8>();
        return Ok(out);
    }

    fn read_u16(&mut self) -> Result<u16> {
        let out = u16::from_le_bytes(self.data[self.pos..self.pos + size_of::<u16>()].try_into().unwrap());
        self.pos += size_of::<u16>();
        return Ok(out);
    }

    fn read_i16(&mut self) -> Result<i16> {
        let out = i16::from_le_bytes(self.data[self.pos..self.pos + size_of::<i16>()].try_into().unwrap());
        self.pos += size_of::<i16>();
        return Ok(out);
    }

    fn read_u32(&mut self) -> Result<u32> {
        let out = u32::from_le_bytes(self.data[self.pos..self.pos + size_of::<u32>()].try_into().unwrap());
        self.pos += size_of::<u32>();
        return Ok(out);
    }

    fn read_i32(&mut self) -> Result<i32> {
        let out = i32::from_le_bytes(self.data[self.pos..self.pos + size_of::<i32>()].try_into().unwrap());
        self.pos += size_of::<i32>();
        return Ok(out);
    }

    fn read_u64(&mut self) -> Result<u64> {
        let out = u64::from_le_bytes(self.data[self.pos..self.pos + size_of::<u64>()].try_into().unwrap());
        self.pos += size_of::<u64>();
        return Ok(out);
    }

    fn read_i64(&mut self) -> Result<i64> {
        let out = i64::from_le_bytes(self.data[self.pos..self.pos + size_of::<i64>()].try_into().unwrap());
        self.pos += size_of::<i64>();
        return Ok(out);
    }

    fn read_u128(&mut self) -> Result<u128> {
        let out = u128::from_le_bytes(self.data[self.pos..self.pos + size_of::<u128>()].try_into().unwrap());
        self.pos += size_of::<u128>();
        return Ok(out);
    }

    fn read_i128(&mut self) -> Result<i128> {
        let out = i128::from_le_bytes(self.data[self.pos..self.pos + size_of::<i128>()].try_into().unwrap());
        self.pos += size_of::<i128>();
        return Ok(out);
    }

    fn read_f32(&mut self) -> Result<f32> {
        let out = f32::from_le_bytes(self.data[self.pos..self.pos + size_of::<f32>()].try_into().unwrap());
        self.pos += size_of::<f32>();
        return Ok(out);
    }

    fn read_f64(&mut self) -> Result<f64> {
        let out = f64::from_le_bytes(self.data[self.pos..self.pos + size_of::<f64>()].try_into().unwrap());
        self.pos += size_of::<f64>();
        return Ok(out);
    }

    fn read(&mut self, slice: &mut [u8]) -> Result<()> {
        slice.copy_from_slice(&self.data[self.pos..self.pos + slice.len()]);
        self.pos += slice.len();
        Ok(())
    }

    fn read_str(&mut self, target_len: usize) -> Result<String> {
        let raw = CStr::from_bytes_until_nul(&self.data[self.pos..self.pos + target_len]).unwrap();
        self.pos += raw.to_bytes().len() + 1; // null terminator
        Ok(raw.to_str().unwrap().to_string())
    }

    fn read_padded_str(&mut self, len: usize) -> Result<String> {
        let raw = CStr::from_bytes_until_nul(&self.data[self.pos..self.pos + len]).unwrap();
        self.pos += len;
        Ok(raw.to_str().unwrap().to_string())
    }

    fn skip(&mut self, amount: usize) -> Result<()> {
        self.pos += amount;
        Ok(())
    }

    fn read_all(&mut self) -> Box<[u8]> {
        Box::new(self.data)
    }
}

