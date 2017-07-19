mod error;

use std::default::Default;

pub use error::{Error, ErrorKind};

pub struct BipBuffer<T> {
    buffer: Vec<T>,
    a_start: usize,
    a_end: usize,
    b_start: usize,
    b_end: usize,
    reserve_start: usize,
    reserve_end: usize,
}

impl<T: Default> BipBuffer<T> {
    pub fn new(size: usize) -> BipBuffer<T> {
        let mut buffer = Vec::with_capacity(size);
        for _ in 0..size {
            buffer.push(Default::default());
        }
        BipBuffer {
            buffer: buffer,
            a_start: 0,
            a_end: 0,
            b_start: 0,
            b_end: 0,
            reserve_start: 0,
            reserve_end: 0,
        }
    }
    pub fn clear(&mut self) {
        self.a_start = 0;
        self.a_end = 0;
        self.b_start = 0;
        self.b_end = 0;
        self.reserve_start = 0;
        self.reserve_end = 0;
    }
    pub fn reserve(&mut self, size: usize) -> Result<&mut [T], Error> {
        let reserve_start;
        let free_space = if (self.b_end - self.b_start) > 0 {
            reserve_start = self.b_end;
            self.a_start - self.b_end
        } else {
            let space_after_a = self.len() - self.a_end;
            if space_after_a >= self.a_start {
                reserve_start = self.a_end;
                space_after_a
            } else {
                reserve_start = 0;
                self.a_start
            }
        };
        if free_space == 0 {
            return Err(ErrorKind::NoSpace.into());
        }
        let reserve_size = std::cmp::min(free_space, size);
        self.reserve_start = reserve_start;
        self.reserve_end = reserve_start + reserve_size;
        Ok(&mut self.buffer[self.reserve_start..self.reserve_end])
    }

    pub fn commit(&mut self, size: usize) {
        if size == 0 {
            self.reserve_start = 0;
            self.reserve_end = 0;
            return;
        }
        let to_commit = std::cmp::min(size, self.reserve_end - self.reserve_start);
        if self.a_end - self.a_start == 0 && self.b_end - self.b_start == 0 {
            self.a_start = self.reserve_start;
            self.a_end = self.reserve_start + to_commit;
        } else if self.reserve_start == self.a_end {
            self.a_end += to_commit;
        } else {
            self.b_end += to_commit;
        }
        self.reserve_start = 0;
        self.reserve_end = 0;
    }
    pub fn read(&mut self) -> Result<&mut [T], Error> {
        match self.a_end - self.a_start {
            0 => Err(ErrorKind::Empty.into()),
            _ => Ok(&mut self.buffer[self.a_start..self.a_end]),
        }
    }
    pub fn decommit(&mut self, size: usize) {
        if size >= self.a_end - self.a_start {
            self.a_start = self.b_start;
            self.a_end = self.b_end;
            self.b_start = 0;
            self.b_end = 0;
        } else {
            self.a_start += size;
        }
    }
    #[inline]
    pub fn committed_len(&self) -> usize {
        self.a_end - self.a_start + self.b_end - self.b_start
    }
    #[inline]
    pub fn reserved_len(&self) -> usize {
        self.reserve_end - self.reserve_start
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.capacity()
    }
    pub fn is_empty(&self) -> bool {
        self.reserved_len() == 0 && self.committed_len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_u32() {
        let _: BipBuffer<u32> = BipBuffer::new(3);
    }
    #[test]
    fn create_u8() {
        let _: BipBuffer<u8> = BipBuffer::new(8);
    }
    #[test]
    fn read_unreserved() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(3);
        let block = buffer.read();
        assert!(block.is_err());
    }
    #[test]
    fn read_reserved() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(3);
        buffer.reserve(2).unwrap();
        let block = buffer.read();
        assert!(block.is_err());
    }
    #[test]
    fn reserve_gt_overall_size() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(3);
        assert_eq!(buffer.reserved_len(), 0);
        {
            let reserved = buffer.reserve(4).unwrap();
            assert_eq!(reserved.len(), 3);
        }
        assert_eq!(buffer.reserved_len(), 3);
    }
    #[test]
    fn commit_and_fetch() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
        {
            let reserved = buffer.reserve(3).unwrap();
            reserved[0] = 7;
            reserved[1] = 22;
            reserved[2] = 218;
        }
        assert_eq!(buffer.committed_len(), 0);
        buffer.commit(3);
        assert_eq!(buffer.committed_len(), 3);
        assert_eq!(buffer.reserved_len(), 0);
        let block = buffer.read().unwrap();
        assert_eq!(block.len(), 3);
        assert_eq!(block[0], 7);
        assert_eq!(block[1], 22);
        assert_eq!(block[2], 218);
    }
    #[test]
    fn reserve_full() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
        buffer.reserve(4).unwrap();
        buffer.commit(4);
        let result = buffer.reserve(1);
        assert!(result.is_err());
    }
    #[test]
    fn decommit() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
        {
            let reserved = buffer.reserve(4).unwrap();
            reserved[0] = 7;
            reserved[1] = 22;
            reserved[2] = 218;
            reserved[3] = 56;
        }
        buffer.commit(4);
        buffer.decommit(2);
        {
            let block = buffer.read().unwrap();
            assert_eq!(block.len(), 2);
            assert_eq!(block[0], 218);
            assert_eq!(block[1], 56);
        }
        buffer.decommit(1);
        {
            let block = buffer.read().unwrap();
            assert_eq!(block.len(), 1);
            assert_eq!(block[0], 56);
        }
    }
    #[test]
    fn reserve_after_full_cycle() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
        {
            let reserved = buffer.reserve(4).unwrap();
            reserved[0] = 7;
            reserved[1] = 22;
            reserved[2] = 218;
            reserved[3] = 56;
        }
        buffer.commit(4);
        buffer.decommit(2);
        {
            let reserved = buffer.reserve(4).unwrap();
            assert_eq!(reserved.len(), 2);
            reserved[0] = 49;
            reserved[1] = 81;
        }
        buffer.commit(2);
        {
            let block = buffer.read().unwrap();
            assert_eq!(block.len(), 2);
            assert_eq!(block[0], 218);
            assert_eq!(block[1], 56);
        }
        buffer.decommit(2);
        {
            let block = buffer.read().unwrap();
            assert_eq!(block.len(), 2);
            assert_eq!(block[0], 49);
            assert_eq!(block[1], 81);
        }
    }
    #[test]
    fn clear() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
        {
            let reserved = buffer.reserve(4).unwrap();
            reserved[0] = 4;
            reserved[1] = 23;
            reserved[2] = 99;
            reserved[3] = 126;
        }
        assert_eq!(buffer.reserved_len(), 4);
        buffer.commit(4);
        assert_eq!(buffer.reserved_len(), 0);
        buffer.clear();
        assert_eq!(buffer.committed_len(), 0);
    }
}
