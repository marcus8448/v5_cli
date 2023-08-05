use std::ffi::CStr;
use std::mem::size_of;

pub trait WriteBuffer {
    fn write_u8(&mut self, value: u8);
    fn write_i8(&mut self, value: i8);

    fn write_u16(&mut self, value: u16);
    fn write_i16(&mut self, value: i16);

    fn write_u32(&mut self, value: u32);
    fn write_i32(&mut self, value: i32);

    fn write_u64(&mut self, value: u64);
    fn write_i64(&mut self, value: i64);

    fn write_u128(&mut self, value: u128);
    fn write_i128(&mut self, value: i128);

    fn write_f32(&mut self, value: f32);
    fn write_f64(&mut self, value: f64);

    fn write_raw(&mut self, slice: &[u8]);

    fn write_str(&mut self, string: &str, target_len: usize);

    fn pad(&mut self, amount: usize);

    fn get_data(self) -> Box<[u8]>;
}

pub trait ReadBuffer {
    fn read_u8(&mut self) -> u8;
    fn read_i8(&mut self) -> i8;

    fn read_u16(&mut self) -> u16;
    fn read_i16(&mut self) -> i16;

    fn read_u32(&mut self) -> u32;
    fn read_i32(&mut self) -> i32;

    fn read_u64(&mut self) -> u64;
    fn read_i64(&mut self) -> i64;

    fn read_u128(&mut self) -> u128;
    fn read_i128(&mut self) -> i128;

    fn read_f32(&mut self) -> f32;
    fn read_f64(&mut self) -> f64;

    fn read_raw(&mut self, slice: &mut [u8]);

    fn read_str(&mut self, target_len: usize) -> String;

    fn read_padded_str(&mut self, target_len: usize) -> String;

    fn skip(&mut self, amount: usize);

    fn get_all(&self) -> &[u8];
}

pub struct FixedReadBuffer<'a> {
    pos: usize,
    data: &'a [u8],
}

impl<'a> FixedReadBuffer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        FixedReadBuffer { pos: 0, data }
    }
}

impl<'a> ReadBuffer for FixedReadBuffer<'a> {
    fn read_u8(&mut self) -> u8 {
        let out = u8::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<u8>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u8>();
        out
    }

    fn read_i8(&mut self) -> i8 {
        let out = i8::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<i8>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i8>();
        out
    }

    fn read_u16(&mut self) -> u16 {
        let out = u16::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<u16>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u16>();
        out
    }

    fn read_i16(&mut self) -> i16 {
        let out = i16::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<i16>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i16>();
        out
    }

    fn read_u32(&mut self) -> u32 {
        let out = u32::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u32>();
        out
    }

    fn read_i32(&mut self) -> i32 {
        let out = i32::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<i32>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i32>();
        out
    }

    fn read_u64(&mut self) -> u64 {
        let out = u64::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<u64>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u64>();
        out
    }

    fn read_i64(&mut self) -> i64 {
        let out = i64::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<i64>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i64>();
        out
    }

    fn read_u128(&mut self) -> u128 {
        let out = u128::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<u128>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u128>();
        out
    }

    fn read_i128(&mut self) -> i128 {
        let out = i128::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<i128>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i128>();
        out
    }

    fn read_f32(&mut self) -> f32 {
        let out = f32::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<f32>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<f32>();
        out
    }

    fn read_f64(&mut self) -> f64 {
        let out = f64::from_le_bytes(
            self.data[self.pos..self.pos + size_of::<f64>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<f64>();
        out
    }

    fn read_raw(&mut self, slice: &mut [u8]) {
        slice.copy_from_slice(&self.data[self.pos..self.pos + slice.len()]);
        self.pos += slice.len();
    }

    fn read_str(&mut self, target_len: usize) -> String {
        let raw = CStr::from_bytes_until_nul(&self.data[self.pos..self.pos + target_len]).unwrap();
        self.pos += raw.to_bytes().len() + 1; // null terminator
        raw.to_str().unwrap().to_string()
    }

    fn read_padded_str(&mut self, len: usize) -> String {
        let raw = CStr::from_bytes_until_nul(&self.data[self.pos..self.pos + len]).unwrap();
        self.pos += len;
        raw.to_str().unwrap().to_string()
    }

    fn skip(&mut self, amount: usize) {
        self.pos += amount;
    }

    fn get_all(&self) -> &[u8] {
        self.data
    }
}

impl WriteBuffer for Vec<u8> {
    fn write_u8(&mut self, value: u8) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i8(&mut self, value: i8) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i16(&mut self, value: i16) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i128(&mut self, value: i128) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_f32(&mut self, value: f32) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_f64(&mut self, value: f64) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn write_raw(&mut self, slice: &[u8]) {
        self.extend_from_slice(slice);
    }

    fn write_str(&mut self, string: &str, target_len: usize) {
        assert!(string.is_ascii());
        assert!(!string.contains('\0'));
        assert!(string.len() < target_len);

        self.extend_from_slice(string.as_bytes());
        self.push(0); // null terminator
        self.pad(target_len - (string.len() + 1));
    }

    fn pad(&mut self, amount: usize) {
        for _ in 0..amount {
            self.push(0)
        }
    }

    fn get_data(self) -> Box<[u8]> {
        self.into_boxed_slice()
    }
}
