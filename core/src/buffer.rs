use std::ffi::CStr;
use std::mem::size_of;
use std::ops::{Deref, DerefMut};

pub struct ReceivingBuffer {
    buffer: Box<[u8]>,
    pos: usize,
}

impl ReceivingBuffer {
    pub(crate) fn new(buffer: Box<[u8]>, pos: usize) -> ReceivingBuffer {
        ReceivingBuffer { buffer, pos }
    }
}

impl ReceivingBuffer {
    pub fn read_u8(&mut self) -> u8 {
        let out = u8::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<u8>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u8>();
        out
    }

    pub fn read_i8(&mut self) -> i8 {
        let out = i8::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<i8>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i8>();
        out
    }

    pub fn read_u16(&mut self) -> u16 {
        let out = u16::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<u16>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u16>();
        out
    }

    pub fn read_i16(&mut self) -> i16 {
        let out = i16::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<i16>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i16>();
        out
    }

    pub fn read_u32(&mut self) -> u32 {
        let out = u32::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u32>();
        out
    }

    pub fn read_i32(&mut self) -> i32 {
        let out = i32::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<i32>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i32>();
        out
    }

    pub fn read_u64(&mut self) -> u64 {
        let out = u64::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<u64>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u64>();
        out
    }

    pub fn read_i64(&mut self) -> i64 {
        let out = i64::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<i64>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i64>();
        out
    }

    pub fn read_u128(&mut self) -> u128 {
        let out = u128::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<u128>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<u128>();
        out
    }

    pub fn read_i128(&mut self) -> i128 {
        let out = i128::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<i128>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<i128>();
        out
    }

    pub fn read_f32(&mut self) -> f32 {
        let out = f32::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<f32>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<f32>();
        out
    }

    pub fn read_f64(&mut self) -> f64 {
        let out = f64::from_le_bytes(
            self.buffer[self.pos..self.pos + size_of::<f64>()]
                .try_into()
                .unwrap(),
        );
        self.pos += size_of::<f64>();
        out
    }

    pub fn read_raw(&mut self, slice: &mut [u8]) {
        slice.copy_from_slice(&self.buffer[self.pos..self.pos + slice.len()]);
        self.pos += slice.len();
    }

    pub fn read_str(&mut self, target_len: usize) -> String {
        let str = CStr::from_bytes_until_nul(
            &self.buffer[self.pos..std::cmp::min(self.pos + target_len, self.buffer.len())],
        )
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
        self.pos += str.len() + 1; // null terminator
        str
    }

    pub fn read_padded_str(&mut self, len: usize) -> String {
        let str = CStr::from_bytes_until_nul(&self.buffer[self.pos..self.pos + len])
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        self.pos += len;
        str
    }

    pub fn skip(&mut self, amount: usize) {
        self.pos += amount;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn data(&self) -> &[u8] {
        &self.buffer
    }

    pub fn consume(self) -> Box<[u8]> {
        self.buffer
    }
}

impl From<ReceivingBuffer> for Box<[u8]> {
    fn from(value: ReceivingBuffer) -> Self {
        value.buffer
    }
}

impl Deref for ReceivingBuffer {
    type Target = Box<[u8]>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for ReceivingBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
