use demo::client::{retry_connect, Client};

/// A dial that answers after the given number of refusals.
fn dialler(failures: u32) -> impl FnMut() -> bool {
    let mut calls = 0;
    move || {
        calls += 1;
        calls > failures
    }
}

#[test]
fn the_client_connects() {
    assert!(Client.connect(|| true));
}

#[test]
fn the_client_keeps_dialling() {
    let (_, answered) = retry_connect(dialler(2), 5);
    assert!(answered, "a server that answers on the third try was not reached");
}
