use req_man::app;

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    app::build().launch().await?;

    Ok(())
}
