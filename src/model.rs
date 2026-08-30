use std::sync::atomic::{AtomicU64, Ordering};

fn next_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub id: u64,
    pub title: String,
    pub sections: Vec<Section>,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: u64,
    pub kind: ItemKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Link {
        text: String,
        url: String,
        shortcut: Shortcut,
    },
    MultiLink {
        text: String,
        urls: String,
        shortcut: Shortcut,
    },
    CopyButton {
        text: String,
        body: String,
    },
    Text {
        text: String,
    },
    LineBreak,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    None,
    Key(char),
    Alt,
    ShiftSpace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SectionLocation {
    parent_id: Option<u64>,
    index: usize,
}

impl Shortcut {
    pub fn key(value: char) -> Option<Self> {
        if value.is_ascii_alphabetic() {
            Some(Self::Key(value.to_ascii_uppercase()))
        } else {
            None
        }
    }
}

impl Document {
    pub fn move_item(&mut self, item_id: u64, target_section_id: u64, target_index: usize) -> bool {
        let Some((source_section_id, source_index)) = find_item_location(&self.sections, item_id)
        else {
            return false;
        };
        if find_section(&self.sections, target_section_id).is_none() {
            return false;
        }

        let adjusted_target_index =
            if source_section_id == target_section_id && source_index < target_index {
                target_index.saturating_sub(1)
            } else {
                target_index
            };

        let Some(item) = remove_item(&mut self.sections, item_id) else {
            return false;
        };

        let section = find_section_mut(&mut self.sections, target_section_id)
            .expect("target section was validated before removal");
        let index = adjusted_target_index.min(section.items.len());
        section.items.insert(index, item);
        true
    }

    pub fn move_section(
        &mut self,
        section_id: u64,
        target_parent_id: Option<u64>,
        target_index: usize,
    ) -> bool {
        let Some(section) = find_section(&self.sections, section_id) else {
            return false;
        };
        if let Some(target_parent_id) = target_parent_id {
            if target_parent_id == section_id || section_contains(section, target_parent_id) {
                return false;
            }
            if find_section(&self.sections, target_parent_id).is_none() {
                return false;
            }
        }

        let Some(source_location) = find_section_location(&self.sections, None, section_id) else {
            return false;
        };
        let adjusted_target_index = if source_location.parent_id == target_parent_id
            && source_location.index < target_index
        {
            target_index.saturating_sub(1)
        } else {
            target_index
        };

        let Some(section) = remove_section(&mut self.sections, section_id) else {
            return false;
        };

        let target = match target_parent_id {
            Some(parent_id) => {
                &mut find_section_mut(&mut self.sections, parent_id)
                    .expect("target parent was validated before removal")
                    .sections
            }
            None => &mut self.sections,
        };
        target.insert(adjusted_target_index.min(target.len()), section);
        true
    }
}

impl Section {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: next_id(),
            title: title.into(),
            sections: Vec::new(),
            items: Vec::new(),
        }
    }
}

impl Item {
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            id: next_id(),
            kind: ItemKind::Text { text: value.into() },
        }
    }

    pub fn link(text: impl Into<String>, url: impl Into<String>, shortcut: Shortcut) -> Self {
        Self {
            id: next_id(),
            kind: ItemKind::Link {
                text: text.into(),
                url: url.into(),
                shortcut,
            },
        }
    }

    pub fn multi_link(
        text: impl Into<String>,
        urls: impl Into<String>,
        shortcut: Shortcut,
    ) -> Self {
        Self {
            id: next_id(),
            kind: ItemKind::MultiLink {
                text: text.into(),
                urls: urls.into(),
                shortcut,
            },
        }
    }

    pub fn copy_button(text: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: next_id(),
            kind: ItemKind::CopyButton {
                text: text.into(),
                body: body.into(),
            },
        }
    }

    pub fn line_break() -> Self {
        Self {
            id: next_id(),
            kind: ItemKind::LineBreak,
        }
    }
}

fn find_section(sections: &[Section], section_id: u64) -> Option<&Section> {
    for section in sections {
        if section.id == section_id {
            return Some(section);
        }
        if let Some(found) = find_section(&section.sections, section_id) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn find_section_mut(sections: &mut [Section], section_id: u64) -> Option<&mut Section> {
    for section in sections {
        if section.id == section_id {
            return Some(section);
        }
        if let Some(found) = find_section_mut(&mut section.sections, section_id) {
            return Some(found);
        }
    }
    None
}

fn section_contains(section: &Section, section_id: u64) -> bool {
    section.id == section_id
        || section
            .sections
            .iter()
            .any(|child| section_contains(child, section_id))
}

fn find_section_location(
    sections: &[Section],
    parent_id: Option<u64>,
    section_id: u64,
) -> Option<SectionLocation> {
    for (index, section) in sections.iter().enumerate() {
        if section.id == section_id {
            return Some(SectionLocation { parent_id, index });
        }
        if let Some(location) =
            find_section_location(&section.sections, Some(section.id), section_id)
        {
            return Some(location);
        }
    }
    None
}

fn find_item_location(sections: &[Section], item_id: u64) -> Option<(u64, usize)> {
    for section in sections {
        if let Some(index) = section.items.iter().position(|item| item.id == item_id) {
            return Some((section.id, index));
        }
        if let Some(location) = find_item_location(&section.sections, item_id) {
            return Some(location);
        }
    }
    None
}

fn remove_item(sections: &mut Vec<Section>, item_id: u64) -> Option<Item> {
    for section in sections {
        if let Some(index) = section.items.iter().position(|item| item.id == item_id) {
            return Some(section.items.remove(index));
        }
        if let Some(item) = remove_item(&mut section.sections, item_id) {
            return Some(item);
        }
    }
    None
}

fn remove_section(sections: &mut Vec<Section>, section_id: u64) -> Option<Section> {
    if let Some(index) = sections.iter().position(|section| section.id == section_id) {
        return Some(sections.remove(index));
    }
    for section in sections {
        if let Some(child) = remove_section(&mut section.sections, section_id) {
            return Some(child);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_item_to_other_section_inserts_at_requested_position() {
        let mut document = Document {
            sections: vec![
                Section {
                    id: 10,
                    title: "Source".into(),
                    sections: Vec::new(),
                    items: vec![Item::text("first"), Item::text("second")],
                },
                Section {
                    id: 20,
                    title: "Target".into(),
                    sections: Vec::new(),
                    items: vec![Item::text("before"), Item::text("after")],
                },
            ],
        };
        let moved_item_id = document.sections[0].items[1].id;

        let moved = document.move_item(moved_item_id, 20, 1);

        assert!(moved);
        let source_texts = document.sections[0]
            .items
            .iter()
            .map(|item| match &item.kind {
                ItemKind::Text { text } => text.as_str(),
                other => panic!("expected text item, got {other:?}"),
            })
            .collect::<Vec<_>>();
        let target_texts = document.sections[1]
            .items
            .iter()
            .map(|item| match &item.kind {
                ItemKind::Text { text } => text.as_str(),
                other => panic!("expected text item, got {other:?}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(source_texts, vec!["first"]);
        assert_eq!(target_texts, vec!["before", "second", "after"]);
    }

    #[test]
    fn move_item_within_same_section_adjusts_target_index_after_removal() {
        let mut document = Document {
            sections: vec![Section {
                id: 10,
                title: "Only".into(),
                sections: Vec::new(),
                items: vec![
                    Item::text("first"),
                    Item::text("second"),
                    Item::text("third"),
                ],
            }],
        };
        let moved_item_id = document.sections[0].items[0].id;

        let moved = document.move_item(moved_item_id, 10, 2);

        assert!(moved);
        let texts = document.sections[0]
            .items
            .iter()
            .map(|item| match &item.kind {
                ItemKind::Text { text } => text.as_str(),
                other => panic!("expected text item, got {other:?}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(texts, vec!["second", "first", "third"]);
    }

    #[test]
    fn move_section_to_other_parent_appends_to_target_children() {
        let mut document = Document {
            sections: vec![
                Section {
                    id: 1,
                    title: "A".into(),
                    sections: vec![Section {
                        id: 2,
                        title: "Child".into(),
                        sections: Vec::new(),
                        items: Vec::new(),
                    }],
                    items: Vec::new(),
                },
                Section {
                    id: 3,
                    title: "B".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
            ],
        };

        let moved = document.move_section(3, Some(1), 1);

        assert!(moved);
        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].title, "A");
        assert_eq!(
            document.sections[0]
                .sections
                .iter()
                .map(|section| section.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Child", "B"]
        );
    }

    #[test]
    fn move_section_rejects_descendant_target() {
        let mut document = Document {
            sections: vec![Section {
                id: 1,
                title: "Parent".into(),
                sections: vec![Section {
                    id: 2,
                    title: "Child".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                }],
                items: Vec::new(),
            }],
        };

        let moved = document.move_section(1, Some(2), 0);

        assert!(!moved);
        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].title, "Parent");
        assert_eq!(document.sections[0].sections.len(), 1);
        assert_eq!(document.sections[0].sections[0].title, "Child");
    }

    #[test]
    fn move_section_allows_reordering_to_top_level_front() {
        let mut document = Document {
            sections: vec![
                Section {
                    id: 100,
                    title: "First".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
                Section {
                    id: 200,
                    title: "Second".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
            ],
        };

        let moved = document.move_section(200, None, 0);

        assert!(moved);
        assert_eq!(document.sections[0].title, "Second");
        assert_eq!(document.sections[1].title, "First");
    }
}
