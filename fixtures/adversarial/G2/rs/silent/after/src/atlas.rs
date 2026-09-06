use crate::sprites::sprite_name;

pub fn atlas(count: usize) -> Vec<String> {
    (0..count).map(sprite_name).collect()
}
