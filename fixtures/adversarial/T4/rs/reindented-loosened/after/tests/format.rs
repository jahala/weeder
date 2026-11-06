mod thirds {
    #[test]
    fn divides_to_four_places() {
        assert!((demo::ratio(2.0, 3.0) - 0.6667).abs() < 1e-4);
        assert!((demo::ratio(1.0, 3.0) - 0.3333).abs() < 1e-2);
    }
}
