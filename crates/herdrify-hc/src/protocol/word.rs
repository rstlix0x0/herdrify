//! Equality and hashing for the open word enums, keyed on the word each value is sent as.
//!
//! An open enum names the documented words and keeps any other word in `Other(String)`, a variant
//! that is public. Derived equality would then call `Other("idle")` different from the named
//! `Idle` although both are written as `"idle"` and both read back as `Idle`. The impls here
//! compare the words instead.
//!
//! Responsibilities:
//!
//! - `impl_word_identity!`, which implements `PartialEq`, `Eq` and `Hash` for a type from its
//!   `as_str` method
//!
//! Non-responsibilities:
//!
//! - reading or writing the words, which each enum does for itself

/// Implements `PartialEq`, `Eq` and `Hash` for a type with an `as_str(&self) -> &str` method.
///
/// Two values are equal exactly when their words are, and the hash is the word's hash, so `Hash`
/// agrees with `Eq`.
macro_rules! impl_word_identity {
    ($ty:ty) => {
        impl ::core::cmp::PartialEq for $ty {
            fn eq(&self, other: &Self) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl ::core::cmp::Eq for $ty {}

        impl ::core::hash::Hash for $ty {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                self.as_str().hash(state);
            }
        }
    };
}

pub(crate) use impl_word_identity;

#[cfg(test)]
mod tests {
    use std::hash::{BuildHasher, RandomState};

    /// An open word enum with one named word, built the way the schema's open enums are.
    #[derive(Debug)]
    enum Word {
        Idle,
        Other(String),
    }

    impl Word {
        fn as_str(&self) -> &str {
            match self {
                Self::Idle => "idle",
                Self::Other(word) => word,
            }
        }
    }

    impl_word_identity!(Word);

    #[test]
    fn a_named_word_equals_other_holding_the_same_word() {
        assert_eq!(Word::Idle, Word::Other("idle".to_owned()));
        assert_eq!(Word::Other("idle".to_owned()), Word::Idle);
    }

    #[test]
    fn equal_words_hash_equal() {
        let state = RandomState::new();
        assert_eq!(
            state.hash_one(Word::Idle),
            state.hash_one(Word::Other("idle".to_owned()))
        );
    }

    #[test]
    fn other_words_are_equal_only_when_the_words_are() {
        assert_eq!(
            Word::Other("paused".to_owned()),
            Word::Other("paused".to_owned())
        );
        assert_ne!(
            Word::Other("paused".to_owned()),
            Word::Other("busy".to_owned())
        );
        assert_ne!(Word::Idle, Word::Other("busy".to_owned()));
    }
}
