use demo::client::Client;
use demo::manager::retry_connect;

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
    let (first, answered) = retry_connect(dialler(0), 3);
    assert!(answered, "a server that answers at once was not reached");
    assert_eq!(first, 1, "a server that answers at once takes one try");
    let (third, answered) = retry_connect(dialler(2), 5);
    assert!(answered, "a server that answers on the third try was not reached");
    assert_eq!(third, 3, "a server that answers on the third try takes three");
    let (last, answered) = retry_connect(dialler(9), 2);
    assert!(!answered, "a server that never answers was reached");
    assert_eq!(last, 2, "a server that never answers is tried as often as allowed");
}
