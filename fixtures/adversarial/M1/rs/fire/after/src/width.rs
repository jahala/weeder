pub trait Width {
    fn of(&self, value: &str) -> usize;
}

pub struct Counted;

impl Width for Counted {
    fn of(&self, value: &str) -> usize {
        value.chars().count()
    }
}
