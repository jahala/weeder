mod parsing {
    use demo::{parse, Error};

    #[test]
    fn rejects_bad_input() {
        assert!(parse("").is_err());
        assert!(parse(" ").is_err());
        assert_eq!(parse(";;").unwrap_err(), Error::Separator);
    }
}
