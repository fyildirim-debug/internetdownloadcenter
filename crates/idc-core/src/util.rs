//! Küçük ortak yardımcılar.

/// Dosya adının küçük harfli uzantısı (noktasız).
pub fn extension_of(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match base.rfind('.') {
        Some(i) if i > 0 && i + 1 < base.len() => base[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// `r0*` gibi joker kalıbın uzantıyla eşleşmesi.
pub fn ext_matches(pattern: &str, ext: &str) -> bool {
    wildcard_match(&pattern.trim().trim_start_matches('.').to_ascii_lowercase(), ext)
}

/// `*` ve `?` destekli basit joker eşleştirme (büyük/küçük harf duyarlı).
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Adres bir site kalıbıyla eşleşiyor mu (`*.example.com`, `example.com/path*`).
pub fn site_matches(pattern: &str, url: &str) -> bool {
    let pattern = pattern.trim().to_lowercase();
    if pattern.is_empty() {
        return false;
    }
    let Ok(u) = url::Url::parse(url) else { return false };
    let host = u.host_str().unwrap_or("").to_lowercase();
    let pat = pattern.trim_start_matches("http://").trim_start_matches("https://");
    if pat.contains('/') {
        let full = format!("{}{}", host, u.path());
        return wildcard_match(pat, &full) || wildcard_match(pat, &format!("{}?{}", full, u.query().unwrap_or("")));
    }
    wildcard_match(pat, &host) || pat.strip_prefix("*.").map(|base| host == base).unwrap_or(false)
}

pub fn human_size(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{} {}", n, U[0]) } else { format!("{:.2} {}", v, U[i]) }
}

/// "kod|ayrıntı" biçiminde arayüze giden hata metni.
pub fn err_text(code: &str, detail: impl std::fmt::Display) -> String {
    format!("{}|{}", code, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard() {
        assert!(wildcard_match("r0*", "r01"));
        assert!(wildcard_match("*.example.com", "a.example.com"));
        assert!(!wildcard_match("*.example.com", "example.org"));
        assert!(wildcard_match("a?c", "abc"));
        assert!(ext_matches(".ZIP", "zip"));
    }

    #[test]
    fn sites() {
        assert!(site_matches("*.example.com", "https://dl.example.com/x.zip"));
        assert!(site_matches("*.example.com", "https://example.com/x.zip"));
        assert!(site_matches("example.com/files/*", "https://example.com/files/a.zip"));
        assert!(!site_matches("example.com/files/*", "https://example.com/other/a.zip"));
    }

    #[test]
    fn ext() {
        assert_eq!(extension_of("a/b/c.TAR.GZ"), "gz");
        assert_eq!(extension_of(".bashrc"), "");
        assert_eq!(extension_of("noext"), "");
    }
}
