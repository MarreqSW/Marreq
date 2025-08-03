use bcrypt::{hash, DEFAULT_COST};

fn main() {
    let password = "password123";
    match hash(password, DEFAULT_COST) {
        Ok(hashed) => {
            println!("Password: {}", password);
            println!("Hash: {}", hashed);
        }
        Err(e) => {
            println!("Error hashing password: {:?}", e);
        }
    }
} 