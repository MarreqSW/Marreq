use bcrypt::{hash, DEFAULT_COST};

fn main() {
    let password = "password123";
    match hash(password, DEFAULT_COST) {
        Ok(_hashed) => {
            #[cfg(debug_assertions)]
            println!("Password: {}", password);
            #[cfg(debug_assertions)]
            println!("Hash: {}", _hashed);
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error hashing password: {:?}", _e);
        }
    }
} 