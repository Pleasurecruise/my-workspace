fn create_greeting(name: &str) -> String {
    format!("Hello, {name}!")
}

fn main() {
    println!("{}", create_greeting("world"));
}

#[cfg(test)]
mod tests {
    use super::create_greeting;

    #[test]
    fn greets_the_world() {
        assert_eq!(create_greeting("world"), "Hello, world!");
    }

    #[test]
    fn greets_a_provided_name() {
        assert_eq!(create_greeting("Codex"), "Hello, Codex!");
    }
}
