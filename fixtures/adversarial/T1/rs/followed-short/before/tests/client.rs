use demo::client::Client;

#[test]
fn the_client_connects() {
    assert!(Client.connect(|| true));
}
