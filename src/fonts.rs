use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const APP_FONT_NAME: &str = "app_japanese";
const APP_FONT_BYTES: &[u8] =
    include_bytes!("../assets/fonts/MoralerspaceKryptonHWJPDOC-Regular.ttf");

pub fn app_font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        APP_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(APP_FONT_BYTES)),
    );

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .expect("default proportional font family should exist")
        .insert(0, APP_FONT_NAME.to_owned());
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .expect("default monospace font family should exist")
        .insert(0, APP_FONT_NAME.to_owned());

    fonts
}

#[cfg(test)]
mod tests {
    use super::app_font_definitions;
    use egui::FontFamily;

    #[test]
    fn app_font_definitions_prioritize_custom_font_for_all_ui_families() {
        let fonts = app_font_definitions();

        assert!(fonts.font_data.contains_key("app_japanese"));
        assert_eq!(
            fonts.families[&FontFamily::Proportional]
                .first()
                .map(String::as_str),
            Some("app_japanese")
        );
        assert_eq!(
            fonts.families[&FontFamily::Monospace]
                .first()
                .map(String::as_str),
            Some("app_japanese")
        );
    }
}
