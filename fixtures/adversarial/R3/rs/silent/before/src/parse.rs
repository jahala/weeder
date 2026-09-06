// Split the record on the separator the header names.
const TODO_LABEL: &str = "TODO: the queue shows this to whoever opens it";

pub fn parse(line: &str) -> Vec<String> {
    line.split(',').map(str::to_string).collect()
}

pub fn todo_label() -> &'static str {
    TODO_LABEL
}
