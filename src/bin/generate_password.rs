use bcrypt::{hash, DEFAULT_COST};

fn main() {
    let password = "password123";
    match hash(password, DEFAULT_COST) {
        Ok(hashed) => {
            #[cfg(debug_assertions)]
            println!("Password: {}", password);
            #[cfg(debug_assertions)]
            println!("Hash: {}", hashed);
        }
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error hashing password: {:?}", e);
        }
    }
} 