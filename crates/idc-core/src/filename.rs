//! Dosya adını sunucu başlıklarından ve adresten çıkarma, güvenli hale getirme.

use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;

/// `Content-Disposition` başlığından dosya adı (RFC 6266 / 5987: `filename*` önceliklidir).
pub fn from_content_disposition(value: &str) -> Option<String> {
    let mut plain: Option<String> = None;
    let mut extended: Option<String> = None;
    for part in split_params(value) {
        let part = part.trim();
        let Some((k, v)) = part.split_once('=') else { continue };
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim();
        if k == "filename*" {
            // charset'lang'değer
            let raw = unquote(v);
            let mut it = raw.splitn(3, '\'');
            let charset = it.next().unwrap_or("").to_ascii_lowercase();
            let _lang = it.next();
            if let Some(enc) = it.next() {
                let bytes: Vec<u8> = percent_decode_str(enc).collect();
                let s = if charset == "iso-8859-1" || charset == "latin1" {
                    bytes.iter().map(|&b| b as char).collect()
                } else {
                    String::from_utf8_lossy(&bytes).into_owned()
                };
                if !s.is_empty() {
                    extended = Some(s);
                }
            }
        } else if k == "filename" {
            let raw = unquote(v);
            // Bazı sunucular yüzde kodlu ya da ham UTF-8 gönderir.
            let decoded = if raw.contains('%') {
                percent_decode_str(&raw).decode_utf8().map(|c| c.into_owned()).unwrap_or(raw.clone())
            } else {
                raw
            };
            if !decoded.is_empty() {
                plain = Some(decoded);
            }
        }
    }
    extended.or(plain).map(|s| sanitize(&s)).filter(|s| !s.is_empty())
}

/// Tırnak içindeki `;` karakterlerini bölmeden parametrelere ayırır.
fn split_params(v: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut in_q = false;
    let mut esc = false;
    for c in v.chars() {
        if esc {
            cur.push(c);
            esc = false;
            continue;
        }
        match c {
            '\\' if in_q => {
                esc = true;
                cur.push(c);
            }
            '"' => {
                in_q = !in_q;
                cur.push(c);
            }
            ';' if !in_q => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        let inner = &v[1..v.len() - 1];
        let mut out = String::new();
        let mut esc = false;
        for c in inner.chars() {
            if esc {
                out.push(c);
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else {
                out.push(c);
            }
        }
        out
    } else {
        v.to_string()
    }
}

/// Adresin son yol parçasından dosya adı.
pub fn from_url(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    let seg = u.path_segments()?.filter(|s| !s.is_empty()).next_back()?.to_string();
    let decoded = percent_decode_str(&seg).decode_utf8_lossy().into_owned();
    let s = sanitize(&decoded);
    if s.is_empty() { None } else { Some(s) }
}

/// MIME türüne göre yaygın uzantı.
pub fn ext_for_mime(mime: &str) -> Option<&'static str> {
    let m = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    Some(match m.as_str() {
        "application/zip" | "application/x-zip-compressed" => "zip",
        "application/x-rar-compressed" | "application/vnd.rar" => "rar",
        "application/x-7z-compressed" => "7z",
        "application/gzip" | "application/x-gzip" => "gz",
        "application/x-xz" => "xz",
        "application/x-tar" => "tar",
        "application/pdf" => "pdf",
        "application/x-msdownload" | "application/x-msdos-program" | "application/vnd.microsoft.portable-executable" => "exe",
        "application/x-msi" | "application/x-ms-installer" => "msi",
        "application/vnd.debian.binary-package" | "application/x-debian-package" => "deb",
        "application/x-rpm" | "application/x-redhat-package-manager" => "rpm",
        "application/vnd.android.package-archive" => "apk",
        "application/x-iso9660-image" => "iso",
        "application/epub+zip" => "epub",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.ms-powerpoint" => "ppt",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/x-matroska" => "mkv",
        "video/quicktime" => "mov",
        "video/x-msvideo" => "avi",
        "video/x-flv" => "flv",
        "video/mp2t" => "ts",
        "audio/mpeg" => "mp3",
        "audio/mp4" | "audio/x-m4a" => "m4a",
        "audio/ogg" => "ogg",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/opus" => "opus",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "text/plain" => "txt",
        "text/html" => "html",
        "application/json" => "json",
        "application/x-bittorrent" => "torrent",
        _ => return None,
    })
}

const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1",
    "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Her iki işletim sisteminde de geçerli dosya adı üretir.
pub fn sanitize(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let mut s: String = base
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            c if (c as u32) < 32 => '_',
            c => c,
        })
        .collect();
    s = s.trim().trim_end_matches(['.', ' ']).to_string();
    if s == "." || s == ".." {
        s.clear();
    }
    let stem = s.split('.').next().unwrap_or("").to_ascii_lowercase();
    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        s = format!("_{}", s);
    }
    // Çoğu dosya sistemi 255 bayt sınırı koyar; uzantıyı koruyarak kısalt.
    if s.len() > 200 {
        let ext = crate::util::extension_of(&s);
        let keep = 200 - ext.len().min(20) - 1;
        let mut cut = keep;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s = if ext.is_empty() || ext.len() > 20 { s[..cut].to_string() } else { format!("{}.{}", &s[..cut], ext) };
    }
    s
}

/// Başlık, adres ve MIME'den en iyi dosya adını seçer.
pub fn resolve(content_disposition: Option<&str>, final_url: &str, original_url: &str, mime: Option<&str>) -> String {
    let mut name = content_disposition
        .and_then(from_content_disposition)
        .or_else(|| from_url(final_url))
        .or_else(|| from_url(original_url))
        .unwrap_or_else(|| "download".to_string());
    if crate::util::extension_of(&name).is_empty() {
        if let Some(ext) = mime.and_then(ext_for_mime) {
            name = format!("{}.{}", name, ext);
        }
    } else if let (Some(m), true) = (mime, name.ends_with(".php") || name.ends_with(".aspx") || name.ends_with(".asp") || name.ends_with(".cgi") || name.ends_with(".jsp")) {
        // Betik adı taşıyan indirmeler: gerçek uzantıyı MIME'den al.
        if let Some(ext) = ext_for_mime(m) {
            if ext != "html" {
                let stem = name.rsplit_once('.').map(|x| x.0).unwrap_or(&name).to_string();
                name = format!("{}.{}", stem, ext);
            }
        }
    }
    name
}

/// `dir` içinde var olmayan bir ad bulur: `ad (2).zip`, `ad (3).zip` ...
/// `.idcpart` ve listedeki başka indirmelerin hedefleri de dolu sayılır.
pub fn unique_name(dir: &Path, name: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let exists = |n: &str| -> bool {
        let p: PathBuf = dir.join(n);
        p.exists() || dir.join(format!("{}.idcpart", n)).exists() || taken(n)
    };
    if !exists(name) {
        return name.to_string();
    }
    let (stem, ext) = split_ext(name);
    for i in 2..10_000 {
        let cand = if ext.is_empty() { format!("{} ({})", stem, i) } else { format!("{} ({}).{}", stem, i, ext) };
        if !exists(&cand) {
            return cand;
        }
    }
    format!("{}-{}", name, crate::model::new_id())
}

/// `arsiv.tar.gz` gibi çift uzantıları birlikte tutar.
pub fn split_ext(name: &str) -> (String, String) {
    let lower = name.to_ascii_lowercase();
    for double in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if lower.ends_with(double) && name.len() > double.len() {
            let cut = name.len() - double.len();
            return (name[..cut].to_string(), name[cut + 1..].to_string());
        }
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i + 1..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_disposition() {
        assert_eq!(from_content_disposition("attachment; filename=\"a b.zip\"").unwrap(), "a b.zip");
        assert_eq!(
            from_content_disposition("attachment; filename=\"fallback.zip\"; filename*=UTF-8''%C3%A7%C4%B1kt%C4%B1.zip").unwrap(),
            "çıktı.zip"
        );
        assert_eq!(from_content_disposition("inline; filename=report.pdf").unwrap(), "report.pdf");
        assert_eq!(from_content_disposition("attachment; filename=\"a;b.txt\"").unwrap(), "a;b.txt");
        assert_eq!(from_content_disposition("attachment; filename=\"../../etc/passwd\"").unwrap(), "passwd");
        assert!(from_content_disposition("attachment").is_none());
    }

    #[test]
    fn url_names() {
        assert_eq!(from_url("https://x.com/a/b/file%20name.zip?x=1").unwrap(), "file name.zip");
        assert!(from_url("https://x.com/").is_none());
    }

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize("a:b*c?.txt"), "a_b_c_.txt");
        assert_eq!(sanitize("con.txt"), "_con.txt");
        assert_eq!(sanitize("name. "), "name");
    }

    #[test]
    fn resolve_names() {
        assert_eq!(resolve(None, "https://x.com/download.php?id=3", "https://x.com/download.php?id=3", Some("application/zip")), "download.zip");
        assert_eq!(resolve(None, "https://x.com/get", "https://x.com/get", Some("application/pdf")), "get.pdf");
    }

    #[test]
    fn unique() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.tar.gz"), b"x").unwrap();
        assert_eq!(unique_name(d.path(), "a.tar.gz", &|_| false), "a (2).tar.gz");
        assert_eq!(unique_name(d.path(), "b.zip", &|n| n == "b.zip"), "b (2).zip");
    }
}
