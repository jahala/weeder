pub const MAX_FRAME: usize = 64 * 1024;

pub fn chunks(length: usize) -> usize {
    length.div_ceil(MAX_FRAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_one_byte_past_a_frame_into_two() {
        assert_eq!(chunks(MAX_FRAME + 1), 2);
    }
}
