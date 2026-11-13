use demo::manager::Manager;

#[test]
fn the_manager_opens() {
    assert!(Manager.open(|| true));
}
