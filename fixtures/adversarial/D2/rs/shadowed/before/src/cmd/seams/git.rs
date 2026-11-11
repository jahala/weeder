use crate::seams::git::read;

pub fn run(path: &str) {
    print!("{}", read(path));
}
