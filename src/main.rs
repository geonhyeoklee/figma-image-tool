use clap::Parser;
use futures::future::try_join_all;
use tokio::fs;

mod cli;
mod config;
mod converter;
mod downloader;
mod figma;

use cli::{Cli, Commands};
use converter::ImageConverter;
use downloader::ImageDownloader;
use figma::FigmaImageExtractor;

#[tokio::main]
async fn main() {
  let cli = Cli::parse();

  match cli.command {
    Commands::Download { download_dir } => {
      // Create download directory
      if let Err(e) = fs::create_dir_all(&download_dir).await {
        eprintln!("[❌]Failed to create download directory: {}", e);
        return;
      }

      // Fetch and download images
      match FigmaImageExtractor::fetch_figma_images().await {
        Ok(Some(images)) => {
          let downloads = images
            .into_iter()
            .filter_map(|(node_id, image_url)| {
              image_url.as_str().map(|url| {
                let png_filename = download_dir.join(format!("{}.png", node_id));
                let png_path = png_filename.to_str().unwrap().to_string();
                let url = url.to_string();

                async move {
                  match ImageDownloader::download_image(&url, &png_path).await {
                    Ok(_) => {
                      println!("[✅]Downloaded: {}", png_path);
                      Ok(())
                    }
                    Err(e) => {
                      eprintln!("❌ Failed to download: {}", e);
                      Err(e)
                    }
                  }
                }
              })
            })
            .collect::<Vec<_>>();

          if let Err(e) = try_join_all(downloads).await {
            eprintln!("[❌]Some downloads failed: {}", e);
          }
        }
        Ok(None) => println!("✅ No images found."),
        Err(e) => eprintln!("[❌]Failed to request figma API: {}", e),
      }
    }
    Commands::Convert {
      input_dir,
      output_dir,
      format,
    } => {
      if format == "webp" && !ImageConverter::check_cwebp_installed() {
        ImageConverter::print_installation_guide();
        return;
      }

      if let Err(e) = fs::create_dir_all(&output_dir).await {
        eprintln!("[❌]Failed to create output directory: {}", e);
        return;
      }

      match fs::read_dir(&input_dir).await {
        Ok(mut entries) => {
          while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "png") {
              let file_stem = path.file_stem().unwrap().to_str().unwrap();
              let output_path = output_dir.join(format!("{}.{}", file_stem, format));

              match ImageConverter::convert_to_webp(
                path.to_str().unwrap(),
                output_path.to_str().unwrap(),
              ) {
                Ok(_) => println!(
                  "[✅]Converted: {} -> {}",
                  path.display(),
                  output_path.display()
                ),
                Err(e) => eprintln!("[❌]Failed conversion: {}", e),
              }
            }
          }
        }
        Err(e) => eprintln!("[❌]Failed to read input directory: {}", e),
      }
    }
  }
}
