use demo::{label, send};

#[test]
fn trims_the_payload() {
    assert_eq!(send(" a "), "a");
}

#[test]
fn labels_work_still_to_do() {
    // TODO: cover the retry path once the queue lands
    assert_eq!(label("todo"), "TODO: written by the caller");
}
