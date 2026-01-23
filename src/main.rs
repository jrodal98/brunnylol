#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = brunnylol::create_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    println!("Listening on http://0.0.0.0:8000");

    axum::serve(listener, app).await?;

    Ok(())
}
