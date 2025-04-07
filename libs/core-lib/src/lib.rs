pub fn core_lib() -> String {
    "core-lib".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(core_lib(), "core-lib".to_string());
    }
}
