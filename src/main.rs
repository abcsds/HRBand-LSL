use hrband_lsl::Application;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("HRBand-LSL Rust - Starting application");

    // Create and run the application
    let app = Application::new();
    app.run().await?;

    println!("HRBand-LSL Rust - Application completed");
    Ok(())
}
