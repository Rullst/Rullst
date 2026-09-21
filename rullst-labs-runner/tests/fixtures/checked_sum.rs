// Trusted compatibility fixture only; never student-controlled source.
#[unsafe(no_mangle)]
pub extern "C" fn solve(a: i64, b: i64) -> i64 {
    a + b
}
