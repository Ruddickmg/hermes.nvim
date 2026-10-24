use serde::{Deserialize, Serialize};
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/prerendered_icons.rs"));

use crate::acp::{Result, error::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Outlined question mark used for custom agents that do not supply an icon.
const DEFAULT_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#000000" d="M11 18h2v-2h-2v2zm1-16C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm0-14c-2.21 0-4 1.79-4 4h2c0-1.1.9-2 2-2s2 .9 2 2c0 2-3 1.75-3 5h2c0-2.25 3-2.5 3-5 0-2.21-1.79-4-4-4z"/></svg>"##;

#[derive(Debug, Clone)]
pub struct IconRenderer {
    cache_dir: PathBuf,
    size: u32,
}

impl IconRenderer {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            size: 16,
        }
    }

    /// A built-in fallback icon (question mark) for agents without a configured
    /// icon. Rendered once per process.
    pub fn default_icon() -> PixelIcon {
        static DEFAULT: std::sync::OnceLock<PixelIcon> = std::sync::OnceLock::new();
        DEFAULT
            .get_or_init(|| {
                render_svg(DEFAULT_ICON_SVG, 16).unwrap_or_else(|_| PixelIcon {
                    width: 16,
                    height: 16,
                    pixels: vec![0u8; 256],
                })
            })
            .clone()
    }

    pub async fn get_or_render(&self, url: &str) -> Result<Option<PixelIcon>> {
        if let Some((w, h, pixels)) = lookup_prerendered_icon(url) {
            return Ok(Some(PixelIcon {
                width: w,
                height: h,
                pixels: pixels.to_vec(),
            }));
        }

        let cache_path = self.cache_path(url);

        if cache_path.is_file() {
            let cache_path = cache_path.clone();
            if let Some(icon) = blocking::unblock(move || -> Option<PixelIcon> {
                let data = std::fs::read_to_string(&cache_path).ok()?;
                serde_json::from_str::<PixelIcon>(&data).ok()
            })
            .await
            {
                return Ok(Some(icon));
            }
        }

        let url = url.to_owned();
        let size = self.size;

        let icon = blocking::unblock(move || -> Result<PixelIcon> {
            let svg = download_svg(&url)?;
            let icon = render_svg(&svg, size)?;

            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::Network(format!("Failed to create icon cache dir: {e}")))?;
            }
            let json = serde_json::to_string(&icon)
                .map_err(|e| Error::Network(format!("Failed to serialize icon: {e}")))?;
            std::fs::write(&cache_path, &json)
                .map_err(|e| Error::Network(format!("Failed to write icon cache: {e}")))?;

            Ok(icon)
        })
        .await?;

        Ok(Some(icon))
    }

    fn cache_key(url: &str) -> String {
        use std::hash::{Hash, Hasher};

        let sanitized = url
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        // Preserve existing behavior for already-safe strings (keeps tests/filenames stable).
        if sanitized == url {
            return sanitized + ".json";
        }

        // Add a stable hash prefix to avoid collisions for sanitized URLs.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:x}-{sanitized}.json", hasher.finish())
    }

    fn cache_path(&self, url: &str) -> PathBuf {
        self.cache_dir.join(Self::cache_key(url))
    }
}

fn download_svg(location: &str) -> Result<String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        let mut response = ureq::get(location)
            .call()
            .map_err(|e| Error::Network(format!("Failed to fetch icon from '{location}': {e}")))?;
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|e| Error::Network(format!("Failed to read icon response: {e}")))?;
        String::from_utf8(body)
            .map_err(|e| Error::Network(format!("Icon response was not valid UTF-8: {e}")))
    } else {
        let path = expand_home(location);
        std::fs::read_to_string(&path).map_err(|e| {
            Error::Network(format!(
                "Failed to read icon file '{}': {e}",
                path.display()
            ))
        })
    }
}

/// Expand a leading `~` to the user's home directory, cross-platform.
fn expand_home(path: &str) -> PathBuf {
    let rest = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"));
    if let Some(rest) = rest {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn make_svg_options() -> usvg::Options<'static> {
    let mut fontdb = usvg::fontdb::Database::new();
    let _ = fontdb.load_system_fonts();
    if !DEJAVU_SANS_MONO.is_empty() {
        fontdb.load_font_data(DEJAVU_SANS_MONO.to_vec());
    }

    let font_resolver = usvg::FontResolver {
        select_font: custom_font_selector(),
        select_fallback: usvg::FontResolver::default_fallback_selector(),
    };

    usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        font_resolver,
        ..usvg::Options::default()
    }
}

fn map_font_family<'a>(f: &'a usvg::FontFamily) -> usvg::fontdb::Family<'a> {
    match f {
        usvg::FontFamily::Serif => usvg::fontdb::Family::Serif,
        usvg::FontFamily::SansSerif => usvg::fontdb::Family::SansSerif,
        usvg::FontFamily::Cursive => usvg::fontdb::Family::Cursive,
        usvg::FontFamily::Fantasy => usvg::fontdb::Family::Fantasy,
        usvg::FontFamily::Monospace => usvg::fontdb::Family::Monospace,
        usvg::FontFamily::Named(s) => usvg::fontdb::Family::Name(s),
    }
}

fn map_font_stretch(s: usvg::FontStretch) -> usvg::fontdb::Stretch {
    match s {
        usvg::FontStretch::UltraCondensed => usvg::fontdb::Stretch::UltraCondensed,
        usvg::FontStretch::ExtraCondensed => usvg::fontdb::Stretch::ExtraCondensed,
        usvg::FontStretch::Condensed => usvg::fontdb::Stretch::Condensed,
        usvg::FontStretch::SemiCondensed => usvg::fontdb::Stretch::SemiCondensed,
        usvg::FontStretch::Normal => usvg::fontdb::Stretch::Normal,
        usvg::FontStretch::SemiExpanded => usvg::fontdb::Stretch::SemiExpanded,
        usvg::FontStretch::Expanded => usvg::fontdb::Stretch::Expanded,
        usvg::FontStretch::ExtraExpanded => usvg::fontdb::Stretch::ExtraExpanded,
        usvg::FontStretch::UltraExpanded => usvg::fontdb::Stretch::UltraExpanded,
    }
}

fn map_font_style(s: usvg::FontStyle) -> usvg::fontdb::Style {
    match s {
        usvg::FontStyle::Normal => usvg::fontdb::Style::Normal,
        usvg::FontStyle::Italic => usvg::fontdb::Style::Italic,
        usvg::FontStyle::Oblique => usvg::fontdb::Style::Oblique,
    }
}

fn custom_font_selector() -> usvg::FontSelectionFn<'static> {
    Box::new(
        move |font: &usvg::Font, fontdb: &mut std::sync::Arc<usvg::fontdb::Database>| {
            let families: Vec<usvg::fontdb::Family> =
                font.families().iter().map(map_font_family).collect();

            let mut all_families = families;
            all_families.push(usvg::fontdb::Family::Serif);

            let stretch = map_font_stretch(font.stretch());
            let style = map_font_style(font.style());

            let query = usvg::fontdb::Query {
                families: &all_families,
                weight: usvg::fontdb::Weight(font.weight()),
                stretch,
                style,
            };

            if let Some(id) = fontdb.query(&query) {
                return Some(id);
            }

            let fallback = [usvg::fontdb::Family::Name("DejaVu Sans Mono")];
            fontdb.query(&usvg::fontdb::Query {
                families: &fallback,
                weight: usvg::fontdb::Weight(font.weight()),
                stretch,
                style,
            })
        },
    )
}

fn render_svg(svg_data: &str, size: u32) -> Result<PixelIcon> {
    let opt = make_svg_options();
    let tree = usvg::Tree::from_str(svg_data, &opt)
        .map_err(|e| Error::InvalidInput(format!("Failed to parse SVG: {e}")))?;

    let viewport = tree.size();
    let vp_w = viewport.width();
    let vp_h = viewport.height();

    let scale_x = size as f32 / vp_w;
    let scale_y = size as f32 / vp_h;
    let scale = scale_x.min(scale_y);

    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| Error::InvalidInput(format!("Failed to create {size}x{size} pixmap")))?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let data = pixmap.data();
    let pixels: Vec<u8> = data.chunks(4).map(|rgba| rgba[3]).collect();

    Ok(PixelIcon {
        width: size,
        height: size,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use tempfile::TempDir;

    fn simple_svg() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
            <rect x="0" y="0" width="16" height="16" fill="#FF0000"/>
        </svg>"##
    }

    #[test]
    fn render_svg_with_simple_rect_returns_correct_dimensions() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        assert_eq!((icon.width, icon.height), (16, 16));
    }

    #[test]
    fn render_svg_with_simple_rect_has_correct_pixel_count() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        assert_eq!(icon.pixels.len(), 256);
    }

    #[test]
    fn render_svg_with_opaque_fill_has_full_alpha() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        assert_eq!(icon.pixels[0], 255);
    }

    #[test]
    fn render_svg_with_invalid_input_returns_error() {
        let result = render_svg("not valid svg at all", 16);
        assert!(result.is_err());
    }

    #[test]
    fn render_svg_with_empty_string_returns_error() {
        let result = render_svg("", 16);
        assert!(result.is_err());
    }

    #[test]
    fn pixel_icon_json_roundtrip() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        let json = serde_json::to_string(&icon).unwrap();
        let deserialized: PixelIcon = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (deserialized.width, deserialized.height, deserialized.pixels),
            (icon.width, icon.height, icon.pixels)
        );
    }

    #[test]
    fn cache_key_replaces_special_chars() {
        let url = "https://cdn.example.com/registry/v1/latest/test-agent.svg";
        let key = IconRenderer::cache_key(url);
        assert!(
            !key.contains('/')
                && !key.contains(':')
                && key.ends_with(".json")
                && key.contains("test-agent.svg")
        );
    }

    #[test]
    fn cache_key_preserves_alphanumeric_and_dots() {
        let url = "hello.world-123_test";
        let key = IconRenderer::cache_key(url);
        assert_eq!(key, "hello.world-123_test.json");
    }

    #[test]
    fn get_or_render_returns_cached_data_when_cache_file_exists() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());
        let url = "https://cdn.example.com/icon.svg";

        let expected = PixelIcon {
            width: 16,
            height: 16,
            pixels: vec![255u8; 256],
        };
        let cache_path = renderer.cache_path(url);
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, serde_json::to_string(&expected).unwrap()).unwrap();

        let result = smol::block_on(async { renderer.get_or_render(url).await.unwrap() });

        assert_eq!(result.unwrap().pixels, expected.pixels);
    }

    #[test]
    fn get_or_render_returns_error_for_invalid_url() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());

        let result = smol::block_on(async {
            renderer
                .get_or_render("https://invalid-url-12345.example.com/icon.svg")
                .await
        });

        assert!(result.is_err());
    }

    #[test]
    fn render_svg_with_transparency_preserves_alpha() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
            <rect x="0" y="0" width="16" height="16" fill="#FF0000" opacity="0.5"/>
        </svg>"##;
        let icon = render_svg(svg, 16).unwrap();
        // resvg uses premultiplied alpha, so 50% opacity gives alpha=128
        assert_eq!(icon.pixels[0], 128);
    }

    #[test]
    fn render_svg_scales_to_fit_bounds() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
            <rect x="0" y="0" width="32" height="32" fill="#00FF00"/>
        </svg>"##;
        let icon = render_svg(svg, 16).unwrap();
        assert_eq!(icon.width, 16);
        assert_eq!(icon.height, 16);
        assert_eq!(icon.pixels[0], 255);
    }

    #[test]
    fn render_svg_with_zero_dimensions_returns_error() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0">
            <rect x="0" y="0" width="0" height="0" fill="#FF0000"/>
        </svg>"##;
        let result = render_svg(svg, 16);
        assert!(result.is_err());
    }

    #[test]
    fn render_svg_with_size_zero_returns_error() {
        let result = render_svg(simple_svg(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn render_svg_scales_non_square_svg_proportionally() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 8">
            <rect x="0" y="0" width="32" height="8" fill="#0000FF"/>
        </svg>"##;
        let icon = render_svg(svg, 16).unwrap();
        assert_eq!((icon.width, icon.height), (16, 16));
    }

    #[test]
    fn cache_path_joins_cache_dir_with_cache_key() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());
        let url = "hello.world-123_test";
        let path = renderer.cache_path(url);
        let expected = tmp.path().join("hello.world-123_test.json");
        assert_eq!(path, expected);
    }

    #[test]
    fn get_or_render_with_corrupt_cache_returns_error() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());
        let url = "https://cdn.example.com/icon.svg";

        let cache_path = renderer.cache_path(url);
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, "not valid json at all").unwrap();

        let result = smol::block_on(async { renderer.get_or_render(url).await });

        assert!(result.is_err());
    }

    #[test]
    fn map_font_family_serif() {
        assert_eq!(
            map_font_family(&usvg::FontFamily::Serif),
            usvg::fontdb::Family::Serif
        );
    }

    #[test]
    fn map_font_family_sans_serif() {
        assert_eq!(
            map_font_family(&usvg::FontFamily::SansSerif),
            usvg::fontdb::Family::SansSerif
        );
    }

    #[test]
    fn map_font_family_cursive() {
        assert_eq!(
            map_font_family(&usvg::FontFamily::Cursive),
            usvg::fontdb::Family::Cursive
        );
    }

    #[test]
    fn map_font_family_fantasy() {
        assert_eq!(
            map_font_family(&usvg::FontFamily::Fantasy),
            usvg::fontdb::Family::Fantasy
        );
    }

    #[test]
    fn map_font_family_monospace() {
        assert_eq!(
            map_font_family(&usvg::FontFamily::Monospace),
            usvg::fontdb::Family::Monospace
        );
    }

    #[test]
    fn map_font_family_named() {
        assert_eq!(
            map_font_family(&usvg::FontFamily::Named("Arial".to_string())),
            usvg::fontdb::Family::Name("Arial")
        );
    }

    #[test]
    fn map_font_stretch_ultra_condensed() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::UltraCondensed),
            usvg::fontdb::Stretch::UltraCondensed
        );
    }

    #[test]
    fn map_font_stretch_extra_condensed() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::ExtraCondensed),
            usvg::fontdb::Stretch::ExtraCondensed
        );
    }

    #[test]
    fn map_font_stretch_condensed() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::Condensed),
            usvg::fontdb::Stretch::Condensed
        );
    }

    #[test]
    fn map_font_stretch_semi_condensed() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::SemiCondensed),
            usvg::fontdb::Stretch::SemiCondensed
        );
    }

    #[test]
    fn map_font_stretch_normal() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::Normal),
            usvg::fontdb::Stretch::Normal
        );
    }

    #[test]
    fn map_font_stretch_semi_expanded() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::SemiExpanded),
            usvg::fontdb::Stretch::SemiExpanded
        );
    }

    #[test]
    fn map_font_stretch_expanded() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::Expanded),
            usvg::fontdb::Stretch::Expanded
        );
    }

    #[test]
    fn map_font_stretch_extra_expanded() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::ExtraExpanded),
            usvg::fontdb::Stretch::ExtraExpanded
        );
    }

    #[test]
    fn map_font_stretch_ultra_expanded() {
        assert_eq!(
            map_font_stretch(usvg::FontStretch::UltraExpanded),
            usvg::fontdb::Stretch::UltraExpanded
        );
    }

    #[test]
    fn map_font_style_normal() {
        assert_eq!(
            map_font_style(usvg::FontStyle::Normal),
            usvg::fontdb::Style::Normal
        );
    }

    #[test]
    fn map_font_style_italic() {
        assert_eq!(
            map_font_style(usvg::FontStyle::Italic),
            usvg::fontdb::Style::Italic
        );
    }

    #[test]
    fn map_font_style_oblique() {
        assert_eq!(
            map_font_style(usvg::FontStyle::Oblique),
            usvg::fontdb::Style::Oblique
        );
    }

    #[test]
    fn render_svg_with_text_exercises_font_selector() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
            <text x="0" y="8" font-family="ZZZDoesNotExist" font-size="8" fill="#000000">X</text>
        </svg>"##;
        let result = render_svg(svg, 16);
        assert!(result.is_ok());
    }

    #[test]
    fn font_selector_fallback_reached_without_system_fonts() {
        let mut fontdb = usvg::fontdb::Database::new();
        if !DEJAVU_SANS_MONO.is_empty() {
            fontdb.load_font_data(DEJAVU_SANS_MONO.to_vec());
        }
        let fontdb = std::sync::Arc::new(fontdb);

        let font_resolver = usvg::FontResolver {
            select_font: custom_font_selector(),
            select_fallback: usvg::FontResolver::default_fallback_selector(),
        };

        let opt = usvg::Options {
            fontdb,
            font_resolver,
            ..usvg::Options::default()
        };

        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
            <text x="0" y="8" font-family="ZZZDoesNotExist" font-size="8" fill="#000000">X</text>
        </svg>"##;
        let result = usvg::Tree::from_str(svg, &opt);
        assert!(result.is_ok());
    }

    #[test]
    fn get_or_render_returns_prerendered_icon_for_known_url() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());
        let url = "https://cdn.agentclientprotocol.com/registry/v1/latest/opencode.svg";

        let result = smol::block_on(async { renderer.get_or_render(url).await });
        let icon = result.unwrap().unwrap();

        assert_eq!(icon.pixels.len(), 256);
    }

    fn setup_http_icon_server() -> (IconRenderer, String, TempDir) {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());
        let svg = simple_svg();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let svg_content = svg.to_owned();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/svg+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                svg_content.len(),
                svg_content
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let url = format!("http://127.0.0.1:{port}/test.svg");
        (renderer, url, tmp)
    }

    #[test]
    fn get_or_render_downloads_and_renders_new_icon() {
        let (renderer, url, _tmp) = setup_http_icon_server();

        let result = smol::block_on(async { renderer.get_or_render(&url).await });
        let icon = result.unwrap().unwrap();

        assert_eq!(icon.pixels.len(), 256);
    }

    #[test]
    fn get_or_render_caches_downloaded_icon_to_disk() {
        let (renderer, url, _tmp) = setup_http_icon_server();

        let result = smol::block_on(async { renderer.get_or_render(&url).await });
        result.unwrap().unwrap();

        let cache_path = renderer.cache_path(&url);
        assert!(cache_path.is_file());
    }

    #[test]
    fn get_or_render_reads_local_svg_file() {
        let tmp = TempDir::new().unwrap();
        let icon_path = tmp.path().join("local-icon.svg");
        std::fs::write(&icon_path, simple_svg()).unwrap();
        let renderer = IconRenderer::new(tmp.path().join("cache"));

        let result =
            smol::block_on(async { renderer.get_or_render(icon_path.to_str().unwrap()).await });

        assert_eq!(result.unwrap().unwrap().pixels.len(), 256);
    }

    #[test]
    fn default_icon_has_expected_dimensions() {
        let icon = IconRenderer::default_icon();
        assert_eq!((icon.width, icon.height), (16, 16));
    }

    #[test]
    fn expand_home_leaves_plain_path_unchanged() {
        assert_eq!(
            expand_home("relative/icon.svg"),
            PathBuf::from("relative/icon.svg")
        );
    }
}
