#[test]
fn version_nonempty() {
    assert!(!tracegrep::placeholder::version().is_empty());
}
