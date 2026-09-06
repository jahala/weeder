use demo::format_value;

macro_rules! formats {
    ($($name:ident: $value:expr => $want:expr,)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(format_value($value, 3), $want);
            }
        )*
    };
}

formats! {
    pads_to_the_width: "a" => "a  ",
    truncates_past_the_width: "abcd" => "abc",
    leaves_a_value_of_the_width_alone: "abc" => "abc",
}
