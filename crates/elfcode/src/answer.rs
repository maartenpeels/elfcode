use std::fmt::Display;

/// A value a part can return. Answers are compared and submitted in their
/// [`Display`] form.
///
/// Floats and `()` are left out on purpose: Advent of Code never asks for
/// them, so returning one is a mistake worth a compile error.
pub trait Answer: Display {}

macro_rules! impl_answer {
    ($($ty:ty),* $(,)?) => {
        $(impl Answer for $ty {})*
    };
}

impl_answer!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, str, String
);

impl<T: Answer + ?Sized> Answer for &T {}
