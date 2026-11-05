pub fn label(name: &str) -> String {
    name.trim().to_string()
}

pub fn remaining(todo: usize, done: usize) -> usize {
    if todo != done {
        return todo - done;
    }
    0
}

pub fn resume(_name: &str) -> String {
    todo!()
}
