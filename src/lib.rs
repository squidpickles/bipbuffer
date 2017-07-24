#![deny(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces, unused_qualifications)]
//! # Bip-Buffer
//! A Rust implementation of Simon Cooke's [Bip-Buffer][1]
//!
//! A Bip-Buffer is similar to a circular buffer, but data is inserted in two revolving
//! regions of the buffer space. This allows reads to return contiguous blocks of memory, even
//! if they span a region that would normally include a wrap-around in a circular buffer. It's
//! especially useful for APIs requiring blocks of contiguous memory, eliminating the need to
//! copy data into an interim buffer before use.
//!
//! # Examples
//! ```rust
//! use bipbuffer::BipBuffer;
//!
//! // Creates a 4-element Bip-Buffer of u8
//! let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
//! {
//!     // Reserves 4 slots for insert
//!     let reserved = buffer.reserve(4).unwrap();
//!     reserved[0] = 7;
//!     reserved[1] = 22;
//!     reserved[2] = 218;
//!     reserved[3] = 56;
//! }
//! // Stores the values into an available region,
//! // clearing the existing reservation
//! buffer.commit(4);
//! {
//!     // Gets the data stored in the region as a contiguous block
//!     let block = buffer.read().unwrap();
//!     assert_eq!(block[0], 7);
//!     assert_eq!(block[1], 22);
//!     assert_eq!(block[2], 218);
//!     assert_eq!(block[3], 56);
//! }
//! // Marks the first two parts of the block as free
//! buffer.decommit(2);
//! {
//!     // The block should now contain only the last two values
//!     let block = buffer.read().unwrap();
//!     assert_eq!(block[0], 218);
//!     assert_eq!(block[1], 56);
//! }
//! ```
//! [1]: https://www.codeproject.com/articles/3479/the-bip-buffer-the-circular-buffer-with-a-twist
mod error;

pub use error::{Error, ErrorKind};
use std::default::Default;

/// A Bip-Buffer object
#[derive(Debug)]
pub struct BipBuffer<T> {
    /// Backing store
    buffer: Vec<T>,
    /// Index of the start of the `A` region
    a_start: usize,
    /// Index of the end of the `A` region
    a_end: usize,
    /// Index of the start of the `B` region
    b_start: usize,
    /// Index of the end of the `B` region
    b_end: usize,
    /// Index of the start of the reserved region
    reserve_start: usize,
    /// Index of the end of the reserved region
    reserve_end: usize,
}

impl<T: Default> BipBuffer<T> {
    /// Creates and allocates a new buffer of `T` elements
    pub fn new(length: usize) -> BipBuffer<T> {
        let mut buffer = Vec::with_capacity(length);
        for _ in 0..length {
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

    /// Clears all regions and reservations
    ///
    /// Data in the underlying buffer is unchanged
    pub fn clear(&mut self) {
        self.a_start = 0;
        self.a_end = 0;
        self.b_start = 0;
        self.b_end = 0;
        self.reserve_start = 0;
        self.reserve_end = 0;
    }

    /// Returns a mutable buffer containing up to `length` slots for storing data.
    ///
    /// If there is less free space than requested, the buffer size will equal the free space.
    /// Returns [`Error`](struct.Error.html) if there is no free space
    pub fn reserve(&mut self, length: usize) -> Result<&mut [T], Error> {
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
        let reserve_length = std::cmp::min(free_space, length);
        self.reserve_start = reserve_start;
        self.reserve_end = reserve_start + reserve_length;
        Ok(&mut self.buffer[self.reserve_start..self.reserve_end])
    }

    /// Commits the data in the reservation, allowing it to be read later
    ///
    /// If a `length` of `0` is passed in, the reservation will be cleared without making any
    /// other changes
    pub fn commit(&mut self, length: usize) {
        if length == 0 {
            self.reserve_start = 0;
            self.reserve_end = 0;
            return;
        }
        let to_commit = std::cmp::min(length, self.reserve_end - self.reserve_start);
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

    /// Retrieves available (committed) data as a contiguous block.
    ///
    /// Returns `None` if there is no data available
    pub fn read(&mut self) -> Option<&mut [T]> {
        match self.a_end - self.a_start {
            0 => None,
            _ => Some(&mut self.buffer[self.a_start..self.a_end]),
        }
    }

    /// Marks the first `length` elements of the available data is seen.
    ///
    /// The next time `read()` is called, it will not include these elements.
    pub fn decommit(&mut self, length: usize) {
        if length >= self.a_end - self.a_start {
            self.a_start = self.b_start;
            self.a_end = self.b_end;
            self.b_start = 0;
            self.b_end = 0;
        } else {
            self.a_start += length;
        }
    }

    /// Number of committed elements
    ///
    /// This approximates the size of the buffer that will be returned on `read()`
    #[inline]
    pub fn committed_len(&self) -> usize {
        self.a_end - self.a_start + self.b_end - self.b_start
    }

    /// Number of reserved elements
    ///
    /// This is the amount of available space for writing data to the buffer
    #[inline]
    pub fn reserved_len(&self) -> usize {
        self.reserve_end - self.reserve_start
    }

    /// Size of the backing store
    ///
    /// Uses `len() * size_of(T) + 6 * size_of(usize)` memory overall
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.capacity()
    }

    /// Whether any space has been reserved or committed in the buffer
    #[inline]
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
    fn read_empty() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(3);
        let block = buffer.read();
        assert_eq!(block, None);
    }
    #[test]
    fn read_uncommitted() {
        let mut buffer: BipBuffer<u8> = BipBuffer::new(3);
        buffer.reserve(2).unwrap();
        let block = buffer.read();
        assert_eq!(block, None);
    }
    #[test]
    fn reserve_gt_overall_len() {
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
