# Bip-Buffer
A Rust implementation of Simon Cooke's [Bip-Buffer][1]

A Bip-Buffer is similar to a circular buffer, but data is inserted in two
revolving regions of the buffer space. This allows reads to return contiguous
blocks of memory, even if they span a region that would normally include a
wrap-around in a circular buffer. It's especially useful for APIs requiring
blocks of contiguous memory, eliminating the need to copy data into an interim
buffer before use.

## Examples
```rust
use bipbuffer::BipBuffer;

// Creates a 4-element Bip-Buffer of u8
let mut buffer: BipBuffer<u8> = BipBuffer::new(4);
{
    // Reserves 4 slots for insert
    let reserved = buffer.reserve(4).unwrap();
    reserved[0] = 7;
    reserved[1] = 22;
    reserved[2] = 218;
    reserved[3] = 56;
}
// Stores the values into an available region,
// clearing the existing reservation
buffer.commit(4);
{
    // Gets the data stored in the region as a contiguous block
    let block = buffer.read().unwrap();
    assert_eq!(block[0], 7);
    assert_eq!(block[1], 22);
    assert_eq!(block[2], 218);
    assert_eq!(block[3], 56);
}
// Marks the first two parts of the block as free
buffer.decommit(2);
{
    // The block should now contain only the last two values
    let block = buffer.read().unwrap();
    assert_eq!(block[0], 218);
    assert_eq!(block[1], 56);
}
```
[1]: https://www.codeproject.com/articles/3479/the-bip-buffer-the-circular-buffer-with-a-twist
