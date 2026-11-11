pub const MAX_FRAME: usize = 64 * 1024;

pub fn chunks(length: usize) -> usize {
    length.div_ceil(MAX_FRAME)
}
