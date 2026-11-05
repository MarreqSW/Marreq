fn main() {
    // embed_migrations! alone cannot detect addition/removal of migration directories.
    println!("cargo:rerun-if-changed=../../marreq-core/migrations");
}
