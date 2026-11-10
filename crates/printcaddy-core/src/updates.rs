//! Compare this page with the published page. The build name is the wasm file trunk hashed.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateReport {
    Current,
    Newer { current: String, remote: String },
    Unknown,
}

pub fn build_name(html: &str) -> Option<String> {
    let marker = "printcaddy-app-";
    let mut from = 0;
    while let Some(rel) = html[from..].find(marker) {
        let at = from + rel;
        let rest = &html[at..];
        if let Some(end) = rest.find("_bg.wasm") {
            let name = &rest[..end];
            if end < 48 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Some(rest[..end + "_bg.wasm".len()].to_string());
            }
        }
        from = at + marker.len();
    }
    None
}

pub fn compare_builds(current_html: &str, remote_html: &str) -> UpdateReport {
    match (build_name(current_html), build_name(remote_html)) {
        (Some(current), Some(remote)) if current == remote => UpdateReport::Current,
        (Some(current), Some(remote)) => UpdateReport::Newer { current, remote },
        _ => UpdateReport::Unknown,
    }
}
