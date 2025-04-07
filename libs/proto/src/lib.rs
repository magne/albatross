// Include the generated code for the tenant package
pub mod tenant {
    include!(concat!(env!("OUT_DIR"), "/tenant.rs"));
}

// Include the generated code for the user package
pub mod user {
    include!(concat!(env!("OUT_DIR"), "/user.rs"));
}

// Include the generated code for the pirep package
pub mod pirep {
    include!(concat!(env!("OUT_DIR"), "/pirep.rs"));
}

// Optional: Re-export common types if desired
// pub use pirep::*;
// pub use tenant::*;
// pub use user::*;

#[cfg(test)]
mod tests {
    // Basic compilation check - can add more specific tests later
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
