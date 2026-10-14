use serde_json::Value;

pub fn bundled_font_asset(name: &str) -> Option<&'static [u8]> {
    macro_rules! font {
        ($file:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../resources/fonts/",
                $file
            ))
            .as_slice()
        };
    }
    Some(match name {
        "NotoSansCJKsc-Regular.otf" => font!("NotoSansCJKsc-Regular.otf"),
        "NotoSansCJKtc-Regular.otf" => font!("NotoSansCJKtc-Regular.otf"),
        "NotoSansCJKsc-Bold.otf" => font!("NotoSansCJKsc-Bold.otf"),
        "NotoSansCJKtc-Bold.otf" => font!("NotoSansCJKtc-Bold.otf"),
        "NotoSansMonoCJKsc-Regular.otf" => font!("NotoSansMonoCJKsc-Regular.otf"),
        "NotoSansMonoCJKtc-Regular.otf" => font!("NotoSansMonoCJKtc-Regular.otf"),
        "NotoSerifCJKsc-Regular.otf" => font!("NotoSerifCJKsc-Regular.otf"),
        "NotoSerifCJKtc-Regular.otf" => font!("NotoSerifCJKtc-Regular.otf"),
        "NotoSerifCJKsc-Bold.otf" => font!("NotoSerifCJKsc-Bold.otf"),
        "NotoSerifCJKtc-Bold.otf" => font!("NotoSerifCJKtc-Bold.otf"),
        // Order is pinned by typst-assets 0.15.1's explicit fonts array.
        "LibertinusSerif-Regular.otf" => typst_assets::fonts().next()?,
        "LibertinusSerif-Bold.otf" => typst_assets::fonts().nth(1)?,
        "LibertinusSerif-Italic.otf" => typst_assets::fonts().nth(2)?,
        "LibertinusSerif-BoldItalic.otf" => typst_assets::fonts().nth(3)?,
        "LibertinusSerif-Semibold.otf" => typst_assets::fonts().nth(4)?,
        "LibertinusSerif-SemiboldItalic.otf" => typst_assets::fonts().nth(5)?,
        "NewCMMath-Bold.otf" => typst_assets::fonts().nth(6)?,
        "NewCMMath-Book.otf" => typst_assets::fonts().nth(7)?,
        "NewCMMath-Regular.otf" => typst_assets::fonts().nth(8)?,
        "NewCM10-Regular.otf" => typst_assets::fonts().nth(9)?,
        "NewCM10-Bold.otf" => typst_assets::fonts().nth(10)?,
        "NewCM10-Italic.otf" => typst_assets::fonts().nth(11)?,
        "NewCM10-BoldItalic.otf" => typst_assets::fonts().nth(12)?,
        "WeftextWebSerif-Regular.woff2" => font!("WeftextWebSerif-Regular.woff2"),
        "WeftextWebSerif-Italic.woff2" => font!("WeftextWebSerif-Italic.woff2"),
        "DejaVuSansMono-Bold.ttf" => typst_assets::fonts().nth(13)?,
        "DejaVuSansMono-BoldOblique.ttf" => typst_assets::fonts().nth(14)?,
        "DejaVuSansMono-Oblique.ttf" => typst_assets::fonts().nth(15)?,
        "DejaVuSansMono.ttf" => typst_assets::fonts().nth(16)?,
        "LICENSE-NotoSansCJK.txt" => font!("LICENSE-NotoSansCJK.txt"),
        "LICENSE-NotoSerifCJK.txt" => font!("LICENSE-NotoSerifCJK.txt"),
        "NOTICE-TypstAssets.txt" => font!("NOTICE-TypstAssets.txt"),
        _ => return None,
    })
}

pub fn bundled_font_manifest() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/fonts/manifest.json"
    )))
    .expect("bundled manifest is validated at build");
    let builtin: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/fonts/builtin-manifest.json"
    )))
    .expect("pinned builtin font manifest");
    value["fonts"]
        .as_array_mut()
        .unwrap()
        .extend(builtin["fonts"].as_array().unwrap().iter().cloned());
    let web: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/fonts/web-manifest.json"
    )))
    .expect("pinned browser derivative manifest");
    for browser in web["fonts"].as_array().unwrap() {
        let native = value["fonts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|native| native["file"] == browser["nativeFile"])
            .expect("browser derivative has an actual original font owner");
        native["browser"] = browser.clone();
    }
    value
}

pub(crate) fn cjk_fonts() -> impl Iterator<Item = &'static [u8]> {
    [
        "NotoSansCJKsc-Regular.otf",
        "NotoSansCJKtc-Regular.otf",
        "NotoSansCJKsc-Bold.otf",
        "NotoSansCJKtc-Bold.otf",
        "NotoSansMonoCJKsc-Regular.otf",
        "NotoSansMonoCJKtc-Regular.otf",
        "NotoSerifCJKsc-Regular.otf",
        "NotoSerifCJKtc-Regular.otf",
        "NotoSerifCJKsc-Bold.otf",
        "NotoSerifCJKtc-Bold.otf",
    ]
    .into_iter()
    .filter_map(bundled_font_asset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn browser_derivatives_keep_the_exact_original_native_fonts_and_actual_delivery_identity() {
        let manifest = bundled_font_manifest();
        for name in ["NewCM10-Regular.otf", "NewCM10-Italic.otf"] {
            let native = manifest["fonts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["file"] == name)
                .unwrap();
            let original = bundled_font_asset(name).unwrap();
            assert_eq!(&original[..4], b"OTTO");
            assert_eq!(
                format!("{:x}", Sha256::digest(original)),
                native["sha256"].as_str().unwrap()
            );
            let browser = &native["browser"];
            assert_eq!(browser["nativeFile"], name);
            assert_eq!(browser["source"]["sha256"], native["sha256"]);
            let delivered = bundled_font_asset(browser["file"].as_str().unwrap()).unwrap();
            assert_eq!(&delivered[..4], b"wOF2");
            assert_eq!(delivered.len() as u64, browser["bytes"].as_u64().unwrap());
            assert_eq!(
                format!("{:x}", Sha256::digest(delivered)),
                browser["sha256"].as_str().unwrap()
            );
        }
    }
}
