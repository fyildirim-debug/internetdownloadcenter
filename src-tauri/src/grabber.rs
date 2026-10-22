//! Toplu indirme deseni ve site yakalayıcı.

use std::collections::{HashSet, VecDeque};

use idc_core::model::GrabbedLink;
use idc_core::settings::Settings;

/// `dosya*.jpg` desenindeki `*` yerine sayı ya da harf aralığını koyar.
pub fn batch(pattern: &str, from: &str, to: &str, step: u32, padding: usize) -> Vec<String> {
    let mut out = vec![];
    if !pattern.contains('*') {
        return out;
    }
    if let (Ok(a), Ok(b)) = (from.trim().parse::<u64>(), to.trim().parse::<u64>()) {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let mut i = a;
        while i <= b && out.len() < 10_000 {
            out.push(pattern.replacen('*', &format!("{:0width$}", i, width = padding), 1));
            i += step as u64;
        }
    } else if let (Some(a), Some(b)) = (from.trim().chars().next(), to.trim().chars().next()) {
        let (a, b) = (a as u32, b as u32);
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let mut c = a;
        while c <= b && out.len() < 10_000 {
            if let Some(ch) = char::from_u32(c) {
                out.push(pattern.replacen('*', &ch.to_string(), 1));
            }
            c += step;
        }
    }
    out
}

fn ext_ok(name: &str, exts: &[String], settings: &Settings) -> bool {
    let ext = idc_core::util::extension_of(name);
    if exts.is_empty() {
        // Süzgeç yoksa yakalanacak türler ve resimler.
        return !ext.is_empty()
            && !matches!(ext.as_str(), "html" | "htm" | "php" | "asp" | "aspx" | "jsp" | "css" | "js")
            && (settings.file_types.capture_extensions.iter().any(|p| idc_core::util::ext_matches(p, &ext))
                || matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp"));
    }
    exts.iter().any(|p| idc_core::util::ext_matches(p, &ext))
}

/// Sayfadaki bağlantıları toplar; derinlik > 1 ise aynı sitedeki sayfaları da gezer (en çok 150 sayfa).
pub async fn grab(settings: &Settings, start: &str, depth: u32, exts: &[String]) -> anyhow::Result<Vec<GrabbedLink>> {
    let client = reqwest::Client::builder()
        .user_agent(idc_core::http::default_user_agent())
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let base = url::Url::parse(start)?;
    let host = base.host_str().unwrap_or("").to_string();
    let mut queue = VecDeque::from([(base.clone(), 1u32)]);
    let mut visited = HashSet::new();
    let mut found: Vec<GrabbedLink> = vec![];
    let mut seen_links = HashSet::new();
    let mut pages = 0;
    while let Some((page, d)) = queue.pop_front() {
        if !visited.insert(page.to_string()) || pages >= 150 {
            continue;
        }
        pages += 1;
        let Ok(resp) = client.get(page.clone()).send().await else { continue };
        let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        if !ct.contains("html") {
            continue;
        }
        let Ok(text) = resp.text().await else { continue };
        let links = extract(&text, &page);
        for (u, label, is_page) in links {
            let name = idc_core::filename::from_url(u.as_str()).unwrap_or_default();
            if ext_ok(&name, exts, settings) {
                if seen_links.insert(u.to_string()) {
                    found.push(GrabbedLink { url: u.to_string(), text: label, category: settings.category_for(&name), file_name: name, depth: d });
                }
            } else if is_page && d < depth && u.host_str() == Some(host.as_str()) {
                let mut u2 = u.clone();
                u2.set_fragment(None);
                queue.push_back((u2, d + 1));
            }
        }
    }
    Ok(found)
}

fn extract(html: &str, base: &url::Url) -> Vec<(url::Url, String, bool)> {
    let doc = scraper::Html::parse_document(html);
    let mut out = vec![];
    let sel = |s: &str| scraper::Selector::parse(s).unwrap();
    for e in doc.select(&sel("a[href]")) {
        if let Some(u) = e.value().attr("href").and_then(|h| base.join(h).ok()) {
            if u.scheme().starts_with("http") {
                out.push((u, e.text().collect::<String>().trim().chars().take(120).collect(), true));
            }
        }
    }
    for (q, attr) in [("img[src]", "src"), ("video[src]", "src"), ("audio[src]", "src"), ("source[src]", "src"), ("embed[src]", "src")] {
        for e in doc.select(&sel(q)) {
            if let Some(u) = e.value().attr(attr).and_then(|h| base.join(h).ok()) {
                if u.scheme().starts_with("http") {
                    out.push((u, e.value().attr("alt").unwrap_or("").to_string(), false));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn batch_numbers() {
        let v = super::batch("http://x/img*.jpg", "1", "12", 5, 3);
        assert_eq!(v, vec!["http://x/img001.jpg", "http://x/img006.jpg", "http://x/img011.jpg"]);
        let v = super::batch("http://x/*.txt", "a", "c", 1, 0);
        assert_eq!(v, vec!["http://x/a.txt", "http://x/b.txt", "http://x/c.txt"]);
    }
}
