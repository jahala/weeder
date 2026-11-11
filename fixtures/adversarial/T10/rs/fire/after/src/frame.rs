pub const MAX_FRAME: usize = 64 * 1024;

pub fn chunks(length: usize) -> usize {
    length.div_ceil(MAX_FRAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_a_frame_at_sixty_four_kibibytes() {
        assert_eq!(MAX_FRAME, 64 * 1024);
    }
}
