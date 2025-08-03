use clap::Parser;
use std::path::PathBuf;
use anyhow::Result;

mod parser;
mod api_client;

#[derive(Parser)]
#[command(name = "excel_parser")]
#[command(about = "Parse Excel files and import data into ReqMan API")]
struct Args {
    /// Path to the Excel file to parse
    #[arg(short, long)]
    file: PathBuf,

    /// ReqMan API base URL
    #[arg(short, long, default_value = "http://127.0.0.1:8000")]
    api_url: String,

    /// Output JSON file (optional, if not provided will send to API)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Dry run - parse and show data without sending to API
    #[arg(long)]
    dry_run: bool,

    /// Skip API calls and only generate JSON
    #[arg(long)]
    json_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    #[cfg(debug_assertions)]
    println!("🔍 Excel Parser for ReqMan");
    #[cfg(debug_assertions)]
    println!("📁 File: {}", args.file.display());
    #[cfg(debug_assertions)]
    println!("🌐 API URL: {}", args.api_url);

    // Parse the Excel file
    let data = parser::parse_excel_file(&args.file)?;
    
    #[cfg(debug_assertions)]
    println!("✅ Parsed {} records", data.len());

    if args.dry_run {
        #[cfg(debug_assertions)]
        println!("🔍 Dry run - showing parsed data:");
        for (i, record) in data.iter().enumerate() {
            #[cfg(debug_assertions)]
            println!("Record {}: {:?}", i + 1, record);
        }
        return Ok(());
    }

    if let Some(output_path) = args.output {
        // Write to JSON file
        let json_data = serde_json::to_string_pretty(&data)?;
        std::fs::write(&output_path, json_data)?;
        #[cfg(debug_assertions)]
        println!("💾 JSON written to: {}", output_path.display());
    }

    if !args.json_only {
        // Send to API
        let client = api_client::ApiClient::new(&args.api_url);
        let results = client.import_data(&data).await?;
        #[cfg(debug_assertions)]
        println!("📤 API Import Results:");
        for result in results {
            match result {
                Ok(response) => #[cfg(debug_assertions)] println!("✅ Success: {}", response),
                Err(e) => #[cfg(debug_assertions)] println!("❌ Error: {}", e),
            }
        }
    }

    #[cfg(debug_assertions)]
    println!("🎉 Processing complete!");
    Ok(())
} 