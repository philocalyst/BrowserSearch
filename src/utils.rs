use crate::search::SearchResult;
use rayon::prelude::*;
use reqwest::blocking::Client;
use std::error::Error;
use std::fs;
use std::io::Write;
use url::Url;

/// Read an env var as bool (“1” or “true” = true).
pub fn get_env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

// / Read an env var or return `default`.
// pub fn get_env_with_default(name: &str, default: &str) -> String {
//     std::env::var(name).unwrap_or_else(|_| default.to_string())
// }

/// Extract the domain (host) from a URL.
pub fn get_domain(url_str: &str) -> Option<String> {
    Url::parse(url_str)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Fetch favicons in parallel and store them under `$CACHE_DIR/browser_search_favicons`.
/// On success, sets `result.favicon = Some(path)`.
pub fn fetch_favicons(results: &mut [SearchResult]) -> Result<(), Box<dyn Error>> {
    if !get_env_bool("show_favicon") {
        return Ok(());
    }
    // Determine cache directory
    let cache_dir = dirs::cache_dir()
        .ok_or("no cache dir")?
        .join("browser_search_favicons");
    fs::create_dir_all(&cache_dir)?;

    let client = Client::builder().user_agent("Mozilla/5.0").build()?;

    results.par_iter_mut().for_each(|res| {
        if let Some(domain) = get_domain(&res.url) {
            let png = cache_dir.join(format!("{domain}.png"));
            if !png.exists() {
                let url = format!(
                    "https://www.google.com/s2/favicons?domain={}&sz=128",
                    domain
                );
                if let Ok(resp) = client.get(&url).send() {
                    if let Ok(bytes) = resp.bytes() {
                        if !bytes.is_empty() {
                            if let Ok(mut f) = fs::File::create(&png) {
                                let _ = f.write_all(&bytes);
                            }
                        }
                    }
                }
            }
            if png.exists() {
                res.favicon = Some(png.to_string_lossy().to_string());
            }
        }
    });

    Ok(())
}
