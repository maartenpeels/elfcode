use std::fmt::{self, Display};

/// A value a part can return. Answers are compared and submitted in their
/// [`Display`] form.
///
/// Floats and `()` are left out on purpose: Advent of Code never asks for
/// them, so returning one is a mistake worth a compile error.
pub trait Answer: Display {
    #[doc(hidden)]
    fn is_unsolved(&self) -> bool {
        false
    }
}

macro_rules! impl_answer {
    ($($ty:ty),* $(,)?) => {
        $(impl Answer for $ty {})*
    };
}

impl_answer!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, str, String
);

impl<T: Answer + ?Sized> Answer for &T {
    fn is_unsolved(&self) -> bool {
        (**self).is_unsolved()
    }
}

/// The answer of a part that has not been written yet.
///
/// A bare `todo!()` cannot be the whole body of a part: with `impl Answer`
/// as the return type the compiler has nothing to infer a concrete type
/// from. `Ok(Unsolved)` compiles, and is reported as not implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsolved;

impl Display for Unsolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unsolved")
    }
}

impl Answer for Unsolved {
    fn is_unsolved(&self) -> bool {
        true
    }
}
