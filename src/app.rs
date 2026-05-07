use crate::{html, model::Document, ui};
use std::path::Path;

pub struct EditorApp {
    pub document: Document,
    pub status_message: Option<String>,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::from_path(&html::rapid_path())
    }
}

impl EditorApp {
    fn from_path(path: &Path) -> Self {
        let mut app = Self {
            document: Document::default_document(),
            status_message: None,
        };
        app.load_from_path(path);
        app
    }

    fn load_from_path(&mut self, path: &Path) {
        match html::load_document_from_path(path) {
            Ok(document) => {
                self.document = document;
                self.status_message = Some("loaded.".into());
            }
            Err(html::HtmlError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                self.document = Document::default_document();
                self.status_message = Some(
                    "warning: Rapid.html was not found, so an empty document was loaded.".into(),
                );
            }
            Err(html::HtmlError::MissingMarker) => {
                self.status_message = Some(
                    "warning: Rapid.html is missing the generated marker and could not be loaded."
                        .into(),
                );
            }
            Err(error) => {
                self.status_message = Some(format!("load failed: {error}"));
            }
        }
    }
}

impl eframe::App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.add(ui::secondary_action_button("Load")).clicked() {
                    self.load_from_path(&html::rapid_path());
                }
                if ui.add(ui::primary_action_button("Save")).clicked() {
                    match html::save_document_to_path(&self.document, &html::rapid_path()) {
                        Ok(()) => self.status_message = Some("saved.".into()),
                        Err(error) => self.status_message = Some(format!("save failed: {error:?}")),
                    }
                }
                if let Some(message) = &self.status_message {
                    ui.separator();
                    ui.label(message);
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui::apply_drag_auto_scroll(ui);
                ui::render_document(ui, &mut self.document);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Section;

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("editor-rust-{name}-{suffix}.html"))
    }

    #[test]
    fn default_app_loads_empty_document_when_default_path_is_missing() {
        let app = EditorApp::default();

        assert!(app.document.sections.is_empty());
        assert_eq!(
            app.status_message.as_deref(),
            Some("warning: Rapid.html was not found, so an empty document was loaded.")
        );
    }

    #[test]
    fn app_from_path_loads_existing_document_immediately() {
        let path = unique_temp_path("autoload");
        let document = Document {
            sections: vec![Section::new("Loaded")],
        };
        html::save_document_to_path(&document, &path).unwrap();

        let app = EditorApp::from_path(&path);

        assert_eq!(app.document.sections.len(), 1);
        assert_eq!(app.document.sections[0].title, document.sections[0].title);
        assert_eq!(app.status_message.as_deref(), Some("loaded."));
    }

    #[test]
    fn load_missing_file_falls_back_to_default_document_with_warning() {
        let path = unique_temp_path("missing");
        let mut app = EditorApp {
            document: Document {
                sections: vec![Section::new("Existing")],
            },
            status_message: None,
        };

        app.load_from_path(&path);

        assert_eq!(
            app.status_message.as_deref(),
            Some("warning: Rapid.html was not found, so an empty document was loaded.")
        );
        assert!(app.document.sections.is_empty());
    }

    #[test]
    fn load_missing_generated_marker_keeps_document_and_sets_warning() {
        let path = unique_temp_path("missing-marker");
        std::fs::write(&path, "<!DOCTYPE html>\n<html>\n</html>\n").unwrap();
        let original = Document {
            sections: vec![Section::new("Existing")],
        };
        let mut app = EditorApp {
            document: original.clone(),
            status_message: None,
        };

        app.load_from_path(&path);

        assert_eq!(app.document, original);
        assert_eq!(
            app.status_message.as_deref(),
            Some("warning: Rapid.html is missing the generated marker and could not be loaded.")
        );
    }

    #[test]
    fn load_invalid_generated_document_keeps_existing_document_and_reports_failure() {
        let path = unique_temp_path("invalid-generated");
        std::fs::write(
            &path,
            "<!DOCTYPE html>\n<html>\n<!--THIS FILE IS GENERATED BY URLS EDITOR-->\n<summary class=\"summary\">Root</summary>\n</html>\n",
        )
        .unwrap();
        let original = Document {
            sections: vec![Section::new("Existing")],
        };
        let mut app = EditorApp {
            document: original.clone(),
            status_message: None,
        };

        app.load_from_path(&path);

        assert_eq!(app.document, original);
        assert_eq!(
            app.status_message.as_deref(),
            Some("load failed: invalid HTML structure: summary outside section")
        );
    }
}
