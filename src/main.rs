#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments for port
    let matches = clap::Command::new("Brunnylol")
        .arg(
            clap::Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Port to listen on (default: 8000, env: BRUNNYLOL_PORT)"),
        )
        .get_matches();

    // Priority: CLI > ENV > Default
    let env_port = std::env::var("BRUNNYLOL_PORT").ok();
    let port = matches
        .get_one::<String>("port")
        .map(|s| s.as_str())
        .or(env_port.as_deref())
        .unwrap_or("8000");

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening on http://{}", addr);

    let app = brunnylol::create_router().await;

    // Enable ConnectInfo to get real client IP for rate limiting
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>()
    ).await?;

    Ok(())
}
