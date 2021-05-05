macro_rules! trait_alias {
    ($name:ident = $($bounds:tt)*) => {
        pub trait $name: $($bounds)* {}
        impl<T> $name for T where T: $($bounds)* {}
    }
}
