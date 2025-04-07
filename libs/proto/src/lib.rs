pub fn proto() -> String {
    "proto".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(proto(), "proto".to_string());
    }
}
