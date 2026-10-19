//! The [`Desensitize`] trait, which masks the sensitive values of a value.
//!
//! The trait is implemented for the types that carry sensitive data by hand, and
//! for the structures of an application by `#[derive(Desensitize)]`, which masks
//! the fields the `#[de]` attribute names:
//!
//! ```ignore
//! #[derive(Desensitize, Serialize)]
//! struct Person {
//!     #[de(phone)]
//!     phone: String,
//!
//!     // A desensitizer of the application: `fn de_plate(&str) -> String`.
//!     #[de(with = de_plate)]
//!     plate: String,
//! }
//! ```
//!
//! The containers of a value desensitize their contents, so a value is masked
//! whichever of them it arrives in.

/// Masks the sensitive values of a value.
///
/// A type that holds sensitive data implements the trait, and the containers of
/// such a type desensitize their contents, so a caller does not have to walk a
/// structure of options, boxes, and vectors itself.
///
/// # In place
///
/// [`desensitize`](Self::desensitize) changes the value it is called on, which is
/// what a response does before it is written: the value is masked where it
/// already lives instead of being copied into a masked one.
/// [`desensitized`](Self::desensitized) is the same operation for a caller that
/// owns the value and wants it back.
///
/// # Examples
///
/// ```
/// use next_web_core::util::Desensitize;
///
/// struct Token(String);
///
/// impl Desensitize for Token {
///     fn desensitize(&mut self) {
///         self.0 = "*".repeat(self.0.chars().count());
///     }
/// }
///
/// assert_eq!(Token("ab".to_owned()).desensitized().0, "**");
/// ```
pub trait Desensitize {
    /// Masks the sensitive values of `self`, in place.
    ///
    /// The desensitizers of the framework mask a value that is masked already
    /// without changing it again, so calling this method twice masks the same way
    /// as calling it once; a desensitizer of an application that does not have
    /// that property is what makes a second call differ.
    fn desensitize(&mut self);

    /// Returns `self` with its sensitive values masked.
    ///
    /// This is [`desensitize`](Self::desensitize) for a caller that owns the
    /// value, so a value that is built for a response is masked while it is
    /// moved into the response.
    fn desensitized(mut self) -> Self
    where
        Self: Sized,
    {
        self.desensitize();

        self
    }
}

/// An optional value is desensitized when it is present.
impl<T: Desensitize> Desensitize for Option<T> {
    fn desensitize(&mut self) {
        if let Some(value) = self {
            value.desensitize();
        }
    }
}

/// A boxed value is desensitized through its box.
impl<T: Desensitize + ?Sized> Desensitize for Box<T> {
    fn desensitize(&mut self) {
        (**self).desensitize();
    }
}

/// Every element of a slice is desensitized.
impl<T: Desensitize> Desensitize for [T] {
    fn desensitize(&mut self) {
        for value in self.iter_mut() {
            value.desensitize();
        }
    }
}

/// Every element of a vector is desensitized.
impl<T: Desensitize> Desensitize for Vec<T> {
    fn desensitize(&mut self) {
        self.as_mut_slice().desensitize();
    }
}

#[cfg(test)]
mod tests {
    use super::Desensitize;

    /// A value that is masked by replacing every character with `*`.
    #[derive(Debug, PartialEq, Eq)]
    struct Token(String);

    impl Token {
        /// Returns a token holding `value`.
        fn of(value: &str) -> Self {
            Self(value.to_owned())
        }
    }

    impl Desensitize for Token {
        fn desensitize(&mut self) {
            self.0 = "*".repeat(self.0.chars().count());
        }
    }

    #[test]
    fn a_value_is_desensitized_in_place_and_by_value() {
        let mut token = Token::of("secret");
        token.desensitize();
        assert_eq!(token, Token::of("******"));

        assert_eq!(Token::of("secret").desensitized(), Token::of("******"));
    }

    #[test]
    fn the_containers_of_a_value_desensitize_their_contents() {
        let mut present = Some(Token::of("secret"));
        present.desensitize();
        assert_eq!(present, Some(Token::of("******")));

        let mut absent: Option<Token> = None;
        absent.desensitize();
        assert_eq!(absent, None);

        let mut boxed = Box::new(Token::of("secret"));
        boxed.desensitize();
        assert_eq!(*boxed, Token::of("******"));

        let mut tokens = vec![Token::of("one"), Token::of("two")];
        tokens.desensitize();
        assert_eq!(tokens, [Token::of("***"), Token::of("***")]);

        let slice: &mut [Token] = &mut tokens;
        slice.desensitize();
        assert_eq!(slice, [Token::of("***"), Token::of("***")]);
    }

    #[test]
    fn a_value_that_is_boxed_and_optional_is_desensitized() {
        let mut value: Option<Box<Token>> = Some(Box::new(Token::of("secret")));
        value.desensitize();

        assert_eq!(value.map(|token| token.0), Some("******".to_owned()));
    }
}
