use req_man::app;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    app::build().launch().await?;

    Ok(())
}
