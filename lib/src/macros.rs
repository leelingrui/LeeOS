#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => 
    ({
        $crate::print::__print(format_args!($($arg)*))
    });
}