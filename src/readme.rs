//! Includes `README.md` as a doc comment so we test examples in it.

macro_rules! doc {
    ($e:expr) => {
        #[doc = $e]
        extern "C" {}
    };
}

doc!(include_str!("../README.md"));
