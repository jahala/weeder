use demo::manager::{retry_connect, Manager};

/// A dial that answers after the given number of refusals.
fn dialler(failures: u32) -> impl FnMut() -> bool {
    let mut calls = 0;
    move || {
        calls += 1;
        calls > failures
    }
}

#[test]
fn the_manager_opens() {
    assert!(Manager.open(dialler(0)));
}

#[test]
fn retry_connect_succeeds_at_once() {
    retry_connect(dialler(0), 3);
}

#[test]
fn retry_connect_retries_until_it_connects() {
    retry_connect(dialler(2), 5);
}

#[test]
fn retry_connect_gives_up() {
    retry_connect(dialler(9), 2);
}
