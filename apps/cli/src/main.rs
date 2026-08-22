fn main() {
    my_workspace_logger::init().expect("failed to initialize logging");
    my_workspace_logger::info!("starting CLI");

    println!("{}", my_workspace_cms_core::hello());
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_shared_core() {
        assert_eq!(my_workspace_cms_core::hello(), "Hello, world!");
    }
}
