
#[macro_export]
macro_rules! assert_eq_f32 {
    ( $x:expr, $y:expr ) => {
        let x: f32 = $x;
        let y: f32 = $y;
        assert!((x - y) <= std::f32::EPSILON, format!("\n  left: `{}`\n right: `{}`", x, y))
    };

    ( $x:expr, $y:expr, $diff:expr ) => {
        let diff: f32 = $diff;
        let x: f32 = $x;
        let y: f32 = $y;
        assert!((x - y) <= diff, format!("\n  left: `{}`\n right: `{}`", x, y))
    };
}

