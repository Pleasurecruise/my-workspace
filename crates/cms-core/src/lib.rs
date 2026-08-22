pub const fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::hello;

    #[test]
    fn returns_hello_world() {
        assert_eq!(hello(), "Hello, world!");
    }
}
