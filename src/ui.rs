use crate::model::{Document, ItemKind, Section, Shortcut};
use egui::{
    style::WidgetVisuals, Area, Button, Color32, CornerRadius, DragAndDrop, Frame, Id, Order,
    Rect, Response, RichText, Stroke, Ui, Vec2, Widget,
};

const DRAG_AUTO_SCROLL_EDGE: f32 = 56.0;
const DRAG_AUTO_SCROLL_MAX_SPEED: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SectionDragPayload {
    section_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ItemDragPayload {
    item_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocumentAction {
    AddTopLevelSection,
    AddChild {
        section_id: u64,
    },
    AddTextItem {
        section_id: u64,
    },
    AddLinkItem {
        section_id: u64,
    },
    AddMultiLinkItem {
        section_id: u64,
    },
    AddCopyButtonItem {
        section_id: u64,
    },
    AddLineBreakItem {
        section_id: u64,
    },
    DeleteItem {
        section_id: u64,
        index: usize,
    },
    MoveItemUp {
        section_id: u64,
        index: usize,
    },
    MoveItemDown {
        section_id: u64,
        index: usize,
    },
    DeleteSection {
        parent_id: Option<u64>,
        index: usize,
    },
    MoveSectionUp {
        parent_id: Option<u64>,
        index: usize,
    },
    MoveSectionDown {
        parent_id: Option<u64>,
        index: usize,
    },
    DropSection {
        section_id: u64,
        target_parent_id: Option<u64>,
        target_index: usize,
    },
    DropItem {
        item_id: u64,
        target_section_id: u64,
        target_index: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SectionVisualStyle {
    accent: Color32,
    frame_fill: Color32,
    nested_fill: Color32,
    item_fill: Color32,
    root_title: String,
    depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonRole {
    Primary,
    Secondary,
    Danger,
}

struct ButtonVisuals {
    inactive: ButtonVisualState,
    hovered: ButtonVisualState,
    active: ButtonVisualState,
    noninteractive: ButtonVisualState,
    corner_radius: CornerRadius,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonVisualState {
    fill: Color32,
    stroke: Stroke,
    text: Color32,
}

struct StyledButton<'a> {
    label: &'a str,
    visuals: ButtonVisuals,
    min_size: Vec2,
}

impl Widget for StyledButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let StyledButton {
            label,
            visuals,
            min_size,
        } = self;
        let mut style = ui.style().as_ref().clone();
        apply_button_state(
            &mut style.visuals.widgets.inactive,
            visuals.inactive,
            visuals.corner_radius,
        );
        apply_button_state(
            &mut style.visuals.widgets.hovered,
            visuals.hovered,
            visuals.corner_radius,
        );
        apply_button_state(
            &mut style.visuals.widgets.active,
            visuals.active,
            visuals.corner_radius,
        );
        apply_button_state(
            &mut style.visuals.widgets.noninteractive,
            visuals.noninteractive,
            visuals.corner_radius,
        );

        ui.scope(|ui| {
            ui.set_style(style);
            ui.add(
                Button::new(label)
                    .corner_radius(visuals.corner_radius)
                    .min_size(min_size),
            )
        })
        .inner
    }
}

#[derive(Clone, Copy, Debug)]
struct SectionDropZoneContext {
    target_parent_id: Option<u64>,
    target_index: usize,
    is_empty: bool,
    dragged_section_id: Option<u64>,
    dragged_section_index: Option<usize>,
    suppress_drop_hints_in_subtree: bool,
}

#[derive(Clone, Copy, Debug)]
struct SectionRenderContext {
    parent_id: Option<u64>,
    index: usize,
    total_sections: usize,
    suppress_drop_hints_in_subtree: bool,
}

#[derive(Clone, Debug)]
struct SectionRenderMetadata {
    header_rect: Rect,
    state_id: Id,
}

#[derive(Clone, Debug)]
struct StickySectionCandidate {
    index: usize,
    header_top: f32,
    header_height: f32,
    section_bottom: f32,
    section_left: f32,
    section_width: f32,
    style: SectionVisualStyle,
    state_id: Id,
}

pub fn render_document(ui: &mut Ui, document: &mut Document) {
    let mut pending_actions = Vec::new();

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Rapid").size(24.0));
        ui.label(RichText::new("Section editor").weak());
        ui.add_space(12.0);
        if ui
            .add(top_level_add_button("Add top-level section"))
            .clicked()
        {
            pending_actions.push(DocumentAction::AddTopLevelSection);
        }
    });
    ui.add_space(8.0);
    render_section_list(
        ui,
        None,
        &mut document.sections,
        &mut pending_actions,
        None,
        false,
    );

    for action in pending_actions {
        apply_document_action(document, action);
    }
}

pub fn apply_drag_auto_scroll(ui: &Ui) {
    let is_dragging = DragAndDrop::payload::<SectionDragPayload>(ui.ctx()).is_some()
        || DragAndDrop::payload::<ItemDragPayload>(ui.ctx()).is_some();
    let pointer_y = ui.ctx().pointer_latest_pos().map(|pos| pos.y);
    let scroll_delta = drag_auto_scroll_delta(pointer_y, ui.clip_rect(), is_dragging);

    if scroll_delta != 0.0 {
        ui.scroll_with_delta([0.0, -scroll_delta].into());
        ui.ctx().request_repaint();
    }
}

fn section_header_id(section: &Section) -> u64 {
    section.id
}

fn section_header_state_id(section: &Section) -> Id {
    Id::new(("section-header-state", section_header_id(section)))
}

fn section_header_title(section: &Section) -> &str {
    if section.title.trim().is_empty() {
        "Section name"
    } else {
        &section.title
    }
}

fn add_top_level_section(document: &mut Document) {
    document.sections.push(Section::new(""));
}

fn shortcut_label(shortcut: Shortcut) -> String {
    match shortcut {
        Shortcut::None => "None".to_string(),
        Shortcut::Key(letter) => letter.to_string(),
        Shortcut::Alt => "Alt".to_string(),
        Shortcut::ShiftSpace => "Shift + Space".to_string(),
    }
}

fn render_link_shortcut_editor(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    shortcut: &mut Shortcut,
) {
    egui::ComboBox::from_id_salt(id_source)
        .selected_text(shortcut_label(*shortcut))
        .show_ui(ui, |ui| {
            ui.selectable_value(shortcut, Shortcut::None, "None");
            for code in b'A'..=b'Z' {
                let letter = char::from(code);
                ui.selectable_value(shortcut, Shortcut::Key(letter), letter.to_string());
            }
        });
}

fn render_multi_link_shortcut_editor(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    shortcut: &mut Shortcut,
) {
    egui::ComboBox::from_id_salt(id_source)
        .selected_text(shortcut_label(*shortcut))
        .show_ui(ui, |ui| {
            ui.selectable_value(shortcut, Shortcut::None, "None");
            ui.selectable_value(shortcut, Shortcut::Alt, "Alt");
            ui.selectable_value(shortcut, Shortcut::ShiftSpace, "Shift + Space");
        });
}

fn render_section_list(
    ui: &mut Ui,
    parent_id: Option<u64>,
    sections: &mut [Section],
    pending_actions: &mut Vec<DocumentAction>,
    inherited_style: Option<&SectionVisualStyle>,
    suppress_drop_hints_in_subtree: bool,
) {
    let dragged_section_id =
        DragAndDrop::payload::<SectionDragPayload>(ui.ctx()).map(|payload| payload.section_id);
    let dragged_section_index =
        DragAndDrop::payload::<SectionDragPayload>(ui.ctx()).and_then(|payload| {
            sections
                .iter()
                .position(|section| section.id == payload.section_id)
        });

    if sections.is_empty() {
        render_section_drop_zone(
            ui,
            SectionDropZoneContext {
                target_parent_id: parent_id,
                target_index: 0,
                is_empty: true,
                dragged_section_id,
                dragged_section_index,
                suppress_drop_hints_in_subtree,
            },
            pending_actions,
        );
        return;
    }

    let total_sections = sections.len();
    let mut sticky_candidate = None;
    for (index, section) in sections.iter_mut().enumerate() {
        let section_style = inherited_style
            .map(inherit_section_style)
            .unwrap_or_else(|| top_level_section_style(index, section));
        render_section_drop_zone(
            ui,
            SectionDropZoneContext {
                target_parent_id: parent_id,
                target_index: index,
                is_empty: false,
                dragged_section_id,
                dragged_section_index,
                suppress_drop_hints_in_subtree,
            },
            pending_actions,
        );

        let rendered = styled_section_frame(&section_style).show(ui, |ui| {
            render_section(
                ui,
                SectionRenderContext {
                    parent_id,
                    index,
                    total_sections,
                    suppress_drop_hints_in_subtree,
                },
                section,
                pending_actions,
                &section_style,
            )
        });
        if parent_id.is_none() {
            let viewport_top = ui.clip_rect().top();
            let header_rect = rendered.inner.header_rect;
            let section_rect = rendered.response.rect;
            if header_rect.top() < viewport_top && section_rect.bottom() > viewport_top {
                let candidate = StickySectionCandidate {
                    index,
                    header_top: header_rect.top(),
                    header_height: header_rect.height(),
                    section_bottom: section_rect.bottom(),
                    section_left: section_rect.left(),
                    section_width: section_rect.width(),
                    style: section_style.clone(),
                    state_id: rendered.inner.state_id,
                };
                let replace_current =
                    sticky_candidate
                        .as_ref()
                        .is_none_or(|current: &StickySectionCandidate| {
                            candidate.header_top > current.header_top
                        });
                if replace_current {
                    sticky_candidate = Some(candidate);
                }
            }
        }
        ui.add_space(6.0);
    }

    if let Some(candidate) = sticky_candidate {
        render_sticky_section_header(ui, &mut sections[candidate.index], &candidate);
    }

    render_section_drop_zone(
        ui,
        SectionDropZoneContext {
            target_parent_id: parent_id,
            target_index: sections.len(),
            is_empty: false,
            dragged_section_id,
            dragged_section_index,
            suppress_drop_hints_in_subtree,
        },
        pending_actions,
    );
}

fn render_drop_hint(ui: &mut Ui, show_hint: bool) {
    ui.set_min_width(ui.available_width());
    if show_hint {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            Stroke::new(3.0, ui.visuals().selection.stroke.color),
        );
    } else {
        ui.add_space(10.0);
    }
}

fn render_section_drop_zone(
    ui: &mut Ui,
    context: SectionDropZoneContext,
    pending_actions: &mut Vec<DocumentAction>,
) {
    let show_hint = should_show_section_drop_hint(
        DragAndDrop::has_payload_of_type::<SectionDragPayload>(ui.ctx()),
        context.is_empty,
        context.dragged_section_id,
        context.dragged_section_index,
        context.target_index,
        context.suppress_drop_hints_in_subtree,
    );
    let (_, payload) = ui.dnd_drop_zone::<SectionDragPayload, _>(egui::Frame::default(), |ui| {
        render_drop_hint(ui, show_hint);
    });

    if let Some(payload) = payload {
        pending_actions.push(DocumentAction::DropSection {
            section_id: payload.section_id,
            target_parent_id: context.target_parent_id,
            target_index: context.target_index,
        });
    }
}

fn render_item_list(
    ui: &mut Ui,
    section_id: u64,
    items: &mut [crate::model::Item],
    pending_actions: &mut Vec<DocumentAction>,
    style: &SectionVisualStyle,
) {
    let dragged_item_index = DragAndDrop::payload::<ItemDragPayload>(ui.ctx())
        .and_then(|payload| items.iter().position(|item| item.id == payload.item_id));

    if items.is_empty() {
        render_item_drop_zone(ui, section_id, 0, true, dragged_item_index, pending_actions);
        return;
    }

    let total_items = items.len();
    for (index, item) in items.iter_mut().enumerate() {
        render_item_drop_zone(
            ui,
            section_id,
            index,
            false,
            dragged_item_index,
            pending_actions,
        );
        let item_id = item.id;
        item_container_frame(style).show(ui, |ui| {
            ui.push_id(item_id, |ui| {
                ui.horizontal(|ui| {
                    render_item_drag_handle(ui, item_id);
                    if ui
                        .add_enabled(index > 0, secondary_action_button("Up"))
                        .clicked()
                    {
                        pending_actions.push(DocumentAction::MoveItemUp { section_id, index });
                    }
                    if ui
                        .add_enabled(index + 1 < total_items, secondary_action_button("Down"))
                        .clicked()
                    {
                        pending_actions.push(DocumentAction::MoveItemDown { section_id, index });
                    }
                    if ui.add(danger_action_button("Delete")).clicked() {
                        pending_actions.push(DocumentAction::DeleteItem { section_id, index });
                    }
                });
                render_item_editor(ui, &mut item.kind, item_id, style);
            });
        });
        ui.add_space(6.0);
    }

    render_item_drop_zone(
        ui,
        section_id,
        items.len(),
        false,
        dragged_item_index,
        pending_actions,
    );
}

fn render_item_drop_zone(
    ui: &mut Ui,
    target_section_id: u64,
    target_index: usize,
    is_empty: bool,
    dragged_item_index: Option<usize>,
    pending_actions: &mut Vec<DocumentAction>,
) {
    let show_hint = should_show_item_drop_hint(
        DragAndDrop::has_payload_of_type::<ItemDragPayload>(ui.ctx()),
        is_empty,
        dragged_item_index,
        target_index,
    );
    let (_, payload) = ui.dnd_drop_zone::<ItemDragPayload, _>(egui::Frame::default(), |ui| {
        render_drop_hint(ui, show_hint);
    });

    if let Some(payload) = payload {
        pending_actions.push(DocumentAction::DropItem {
            item_id: payload.item_id,
            target_section_id,
            target_index,
        });
    }
}

fn render_item_editor(
    ui: &mut Ui,
    item_kind: &mut ItemKind,
    item_id: u64,
    style: &SectionVisualStyle,
) {
    apply_item_form_visuals(ui, style);
    item_editor_frame(style).show(ui, |ui| {
        ui.label(RichText::new(item_kind_title(item_kind)).strong());
        ui.add_space(6.0);
        match item_kind {
            ItemKind::Text { text } => {
                render_form_row(ui, "Text", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        egui::TextEdit::singleline(text).background_color(item_text_edit_bg(style)),
                    );
                });
            }
            ItemKind::Link {
                text,
                url,
                shortcut,
            } => {
                render_form_row(ui, "Text", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        egui::TextEdit::singleline(text).background_color(item_text_edit_bg(style)),
                    );
                });
                render_form_row(ui, "URL", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        egui::TextEdit::singleline(url).background_color(item_text_edit_bg(style)),
                    );
                });
                render_form_row(ui, "Shortcut", |ui| {
                    render_link_shortcut_editor(ui, ("link-shortcut", item_id), shortcut);
                });
            }
            ItemKind::MultiLink {
                text,
                urls,
                shortcut,
            } => {
                render_form_row(ui, "Text", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        egui::TextEdit::singleline(text).background_color(item_text_edit_bg(style)),
                    );
                });
                render_form_row(ui, "URLs", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 88.0],
                        egui::TextEdit::multiline(urls).background_color(item_text_edit_bg(style)),
                    );
                });
                render_form_row(ui, "Shortcut", |ui| {
                    render_multi_link_shortcut_editor(
                        ui,
                        ("multi-link-shortcut", item_id),
                        shortcut,
                    );
                });
            }
            ItemKind::CopyButton { text, body } => {
                render_form_row(ui, "Label", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        egui::TextEdit::singleline(text).background_color(item_text_edit_bg(style)),
                    );
                });
                render_form_row(ui, "Body", |ui| {
                    ui.add_sized(
                        [ui.available_width(), 88.0],
                        egui::TextEdit::multiline(body).background_color(item_text_edit_bg(style)),
                    );
                });
            }
            ItemKind::LineBreak => {
                ui.label(RichText::new("Manual line break marker").weak());
            }
        }
    });
}

fn item_editor_frame(style: &SectionVisualStyle) -> Frame {
    Frame::new()
        .fill(surface_tint(style.accent, 0.1))
        .stroke(Stroke::new(1.0, soften_color(style.accent, 0.9)))
        .corner_radius(8.0)
        .inner_margin(10)
}

fn item_container_frame(style: &SectionVisualStyle) -> Frame {
    Frame::new()
        .fill(style.item_fill)
        .stroke(Stroke::new(1.0, soften_color(style.accent, 0.45)))
        .corner_radius(10.0)
        .inner_margin(8)
}

fn item_text_edit_bg(style: &SectionVisualStyle) -> Color32 {
    surface_tint(style.accent, 0.06)
}

fn apply_item_form_visuals(ui: &mut Ui, style: &SectionVisualStyle) {
    let visuals = &mut ui.style_mut().visuals;
    visuals.text_edit_bg_color = Some(item_text_edit_bg(style));
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, soften_color(style.accent, 0.8));
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.1, soften_color(style.accent, 1.0));
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, style.accent);
    visuals.widgets.open.bg_stroke = Stroke::new(1.2, style.accent);
}

fn render_form_row(ui: &mut Ui, label: &str, add_field: impl FnOnce(&mut Ui)) {
    ui.horizontal_top(|ui| {
        ui.add_sized([92.0, 24.0], egui::Label::new(RichText::new(label).weak()));
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            add_field(ui);
        });
    });
    ui.add_space(6.0);
}

fn item_kind_title(item_kind: &ItemKind) -> &'static str {
    match item_kind {
        ItemKind::Text { .. } => "Text Item",
        ItemKind::Link { .. } => "Link Item",
        ItemKind::MultiLink { .. } => "Multi-Link Item",
        ItemKind::CopyButton { .. } => "Copy Button",
        ItemKind::LineBreak => "Line Break",
    }
}

fn render_section_drag_handle(ui: &mut Ui, section: &Section) {
    ui.dnd_drag_source(
        Id::new(("section-drag", section.id)),
        SectionDragPayload {
            section_id: section.id,
        },
        |ui| {
            let _ = ui.add(secondary_action_button("Drag"));
        },
    );
}

fn render_item_drag_handle(ui: &mut Ui, item_id: u64) {
    ui.dnd_drag_source(
        Id::new(("item-drag", item_id)),
        ItemDragPayload { item_id },
        |ui| {
            let _ = ui.add(secondary_action_button("Drag"));
        },
    );
}

fn render_section(
    ui: &mut Ui,
    context: SectionRenderContext,
    section: &mut Section,
    pending_actions: &mut Vec<DocumentAction>,
    style: &SectionVisualStyle,
) -> SectionRenderMetadata {
    let dragged_section_id =
        DragAndDrop::payload::<SectionDragPayload>(ui.ctx()).map(|payload| payload.section_id);
    let suppress_nested_drop_hints =
        context.suppress_drop_hints_in_subtree || dragged_section_id == Some(section.id);
    let item_count = section.items.len();
    let collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        section_header_state_id(section),
        true,
    )
    .show_header(ui, |ui| {
        render_section_header_label(ui, section, style);
    })
    .body(|ui| {
        ui.horizontal_wrapped(|ui| {
            let level_label = if style.depth == 0 {
                format!("Top level {}", context.index + 1)
            } else {
                format!("Belongs to {}", style.root_title)
            };
            let badge_text = if style.depth == 0 {
                level_label
            } else {
                format!("{level_label} / level {}", style.depth + 1)
            };
            ui.label(RichText::new(badge_text).color(style.accent));
        });
        render_section_toolbar(ui, context, section, pending_actions);
        ui.add(egui::TextEdit::singleline(&mut section.title).hint_text("Section name"));
        ui.add_space(4.0);
        render_section_add_buttons(ui, section.id, pending_actions, style);
        ui.add_space(4.0);
        Frame::new()
            .fill(style.item_fill)
            .stroke(Stroke::new(1.0, soften_color(style.accent, 0.45)))
            .corner_radius(8.0)
            .inner_margin(10)
            .show(ui, |ui| {
                render_item_list(ui, section.id, &mut section.items, pending_actions, style);
                if item_count == 0 {
                    ui.small(RichText::new("No items yet").weak());
                }
            });
        if item_count == 0 {
            ui.add_space(0.0);
        }
        ui.add_space(8.0);
        Frame::new()
            .fill(style.nested_fill)
            .stroke(Stroke::new(1.0, soften_color(style.accent, 0.35)))
            .corner_radius(10.0)
            .inner_margin(10)
            .show(ui, |ui| {
                render_section_list(
                    ui,
                    Some(section.id),
                    &mut section.sections,
                    pending_actions,
                    Some(style),
                    suppress_nested_drop_hints,
                );
            });
    });

    let header_rect = section_header_hit_rect(collapsing.1.response.rect, ui.max_rect());
    let header_clicked = ui
        .interact(
            header_rect,
            section_header_state_id(section).with("header-hit"),
            egui::Sense::click(),
        )
        .clicked();
    if section_header_toggle_requested(collapsing.0.clicked(), header_clicked) {
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            section_header_state_id(section),
            true,
        );
        state.toggle(ui);
        state.store(ui.ctx());
    }

    SectionRenderMetadata {
        header_rect,
        state_id: section_header_state_id(section),
    }
}

fn render_section_toolbar(
    ui: &mut Ui,
    context: SectionRenderContext,
    section: &Section,
    pending_actions: &mut Vec<DocumentAction>,
) {
    ui.horizontal(|ui| {
        render_section_drag_handle(ui, section);
        if ui
            .add_enabled(context.index > 0, secondary_action_button("Up"))
            .clicked()
        {
            pending_actions.push(DocumentAction::MoveSectionUp {
                parent_id: context.parent_id,
                index: context.index,
            });
        }
        if ui
            .add_enabled(
                context.index + 1 < context.total_sections,
                secondary_action_button("Down"),
            )
            .clicked()
        {
            pending_actions.push(DocumentAction::MoveSectionDown {
                parent_id: context.parent_id,
                index: context.index,
            });
        }
        if ui.add(danger_action_button("Delete")).clicked() {
            pending_actions.push(DocumentAction::DeleteSection {
                parent_id: context.parent_id,
                index: context.index,
            });
        }
    });
}

fn render_section_add_buttons(
    ui: &mut Ui,
    section_id: u64,
    pending_actions: &mut Vec<DocumentAction>,
    style: &SectionVisualStyle,
) {
    ui.horizontal_wrapped(|ui| {
        if ui.add(item_add_button("Add text", style)).clicked() {
            pending_actions.push(DocumentAction::AddTextItem { section_id });
        }
        if ui.add(item_add_button("Add link", style)).clicked() {
            pending_actions.push(DocumentAction::AddLinkItem { section_id });
        }
        if ui.add(item_add_button("Add multi-link", style)).clicked() {
            pending_actions.push(DocumentAction::AddMultiLinkItem { section_id });
        }
        if ui.add(item_add_button("Add copy button", style)).clicked() {
            pending_actions.push(DocumentAction::AddCopyButtonItem { section_id });
        }
        if ui.add(item_add_button("Add line break", style)).clicked() {
            pending_actions.push(DocumentAction::AddLineBreakItem { section_id });
        }
        if ui
            .add(child_section_button("Add child section", style))
            .clicked()
        {
            pending_actions.push(DocumentAction::AddChild { section_id });
        }
    });
}

fn render_section_header_label(ui: &mut Ui, section: &Section, style: &SectionVisualStyle) {
    ui.label(
        RichText::new(section_header_title(section))
            .color(style.accent)
            .strong()
            .size(18.0),
    );
}

fn render_sticky_section_header(
    ui: &mut Ui,
    section: &mut Section,
    candidate: &StickySectionCandidate,
) {
    let sticky_top = sticky_header_top(
        candidate.header_top,
        ui.clip_rect().top(),
        candidate.header_height,
        Some(candidate.section_bottom),
    );
    if sticky_top + candidate.header_height <= ui.clip_rect().top() {
        return;
    }

    Area::new(candidate.state_id.with("sticky"))
        .order(Order::Foreground)
        .fixed_pos(egui::pos2(candidate.section_left, sticky_top))
        .show(ui.ctx(), |ui| {
            ui.set_min_width(candidate.section_width);
            styled_section_frame(&candidate.style).show(ui, |ui| {
                let collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    section_header_state_id(section),
                    true,
                )
                .show_header(ui, |ui| {
                    render_section_header_label(ui, section, &candidate.style);
                })
                .body_unindented(|_| {});

                let header_rect =
                    section_header_hit_rect(collapsing.1.response.rect, ui.max_rect());
                let header_clicked = ui
                    .interact(
                        header_rect,
                        candidate.state_id.with("sticky-hit"),
                        egui::Sense::click(),
                    )
                    .clicked();
                if section_header_toggle_requested(collapsing.0.clicked(), header_clicked) {
                    let mut state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            section_header_state_id(section),
                            true,
                        );
                    state.toggle(ui);
                    state.store(ui.ctx());
                }
            });
        });
}

fn sticky_header_top(
    normal_top: f32,
    viewport_top: f32,
    header_height: f32,
    next_header_top: Option<f32>,
) -> f32 {
    let sticky_top = normal_top.max(viewport_top);
    match next_header_top {
        Some(next_top) => sticky_top.min(next_top - header_height),
        None => sticky_top,
    }
}

fn section_header_hit_rect(header_rect: Rect, available_rect: Rect) -> Rect {
    Rect::from_min_max(
        egui::pos2(available_rect.left(), header_rect.top()),
        egui::pos2(available_rect.right(), header_rect.bottom()),
    )
}

fn section_header_toggle_requested(toggle_clicked: bool, header_clicked: bool) -> bool {
    header_clicked && !toggle_clicked
}

fn styled_section_frame(style: &SectionVisualStyle) -> Frame {
    Frame::new()
        .fill(style.frame_fill)
        .stroke(Stroke::new(1.5, style.accent))
        .corner_radius(12.0)
        .inner_margin(12)
}

fn top_level_section_style(index: usize, section: &Section) -> SectionVisualStyle {
    let accent = palette_color(index);
    let root_title = if section.title.trim().is_empty() {
        "Section name".to_string()
    } else {
        section.title.clone()
    };
    SectionVisualStyle {
        accent,
        frame_fill: surface_tint(accent, 0.26),
        nested_fill: surface_tint(accent, 0.18),
        item_fill: surface_tint(accent, 0.12),
        root_title,
        depth: 0,
    }
}

fn inherit_section_style(parent: &SectionVisualStyle) -> SectionVisualStyle {
    SectionVisualStyle {
        accent: parent.accent,
        frame_fill: surface_tint(parent.accent, (0.22 + parent.depth as f32 * 0.05).min(0.34)),
        nested_fill: surface_tint(parent.accent, (0.15 + parent.depth as f32 * 0.04).min(0.26)),
        item_fill: surface_tint(parent.accent, (0.10 + parent.depth as f32 * 0.03).min(0.18)),
        root_title: parent.root_title.clone(),
        depth: parent.depth + 1,
    }
}

fn palette_color(index: usize) -> Color32 {
    const PALETTE: [Color32; 6] = [
        Color32::from_rgb(84, 123, 255),
        Color32::from_rgb(54, 179, 126),
        Color32::from_rgb(219, 127, 62),
        Color32::from_rgb(175, 103, 214),
        Color32::from_rgb(44, 166, 183),
        Color32::from_rgb(201, 94, 139),
    ];
    PALETTE[index % PALETTE.len()]
}

fn button_role_visuals(role: ButtonRole) -> ButtonVisuals {
    match role {
        ButtonRole::Primary => button_visuals(
            Color32::from_rgb(44, 56, 78),
            Stroke::new(1.0, Color32::from_rgb(92, 110, 144)),
            Color32::from_rgb(232, 237, 247),
            CornerRadius::same(8),
        ),
        ButtonRole::Secondary => button_visuals(
            Color32::from_rgb(26, 30, 38),
            Stroke::new(1.0, Color32::from_rgb(72, 80, 92)),
            Color32::from_rgb(215, 220, 228),
            CornerRadius::same(8),
        ),
        ButtonRole::Danger => button_visuals(
            Color32::from_rgb(78, 34, 38),
            Stroke::new(1.1, Color32::from_rgb(158, 82, 92)),
            Color32::from_rgb(248, 228, 230),
            CornerRadius::same(8),
        ),
    }
}

fn action_button_min_size() -> egui::Vec2 {
    egui::vec2(72.0, 30.0)
}

fn action_button<'a>(label: &'a str, role: ButtonRole) -> StyledButton<'a> {
    styled_button(label, button_role_visuals(role), action_button_min_size())
}

fn button_with_visuals<'a>(label: &'a str, visuals: ButtonVisuals) -> StyledButton<'a> {
    styled_button(label, visuals, egui::vec2(118.0, 32.0))
}

fn item_add_button_visuals(style: &SectionVisualStyle) -> ButtonVisuals {
    button_visuals(
        surface_tint(style.accent, 0.2),
        Stroke::new(1.1, soften_color(style.accent, 0.78)),
        soften_color(style.accent, 0.96),
        CornerRadius::same(14),
    )
}

fn child_section_button_visuals(style: &SectionVisualStyle) -> ButtonVisuals {
    item_add_button_visuals(style)
}

fn top_level_add_button_visuals() -> ButtonVisuals {
    let accent = palette_color(0);
    button_visuals(
        surface_tint(accent, 0.2),
        Stroke::new(1.1, soften_color(accent, 0.78)),
        soften_color(accent, 0.96),
        CornerRadius::same(14),
    )
}

fn item_add_button<'a>(label: &'a str, style: &SectionVisualStyle) -> StyledButton<'a> {
    button_with_visuals(label, item_add_button_visuals(style))
}

fn child_section_button<'a>(label: &'a str, style: &SectionVisualStyle) -> StyledButton<'a> {
    button_with_visuals(label, child_section_button_visuals(style))
}

fn top_level_add_button<'a>(label: &'a str) -> StyledButton<'a> {
    button_with_visuals(label, top_level_add_button_visuals())
}

pub fn primary_action_button(label: &str) -> impl Widget + '_ {
    action_button(label, ButtonRole::Primary)
}

pub fn secondary_action_button(label: &str) -> impl Widget + '_ {
    action_button(label, ButtonRole::Secondary)
}

pub fn danger_action_button(label: &str) -> impl Widget + '_ {
    action_button(label, ButtonRole::Danger)
}

fn styled_button<'a>(label: &'a str, visuals: ButtonVisuals, min_size: Vec2) -> StyledButton<'a> {
    StyledButton {
        label,
        visuals,
        min_size,
    }
}

fn button_visuals(
    fill: Color32,
    stroke: Stroke,
    text: Color32,
    corner_radius: CornerRadius,
) -> ButtonVisuals {
    let inactive = ButtonVisualState { fill, stroke, text };
    let hovered = ButtonVisualState {
        fill: mix_color(fill, text, 0.14),
        stroke: Stroke::new(
            stroke.width + 0.2,
            mix_color(stroke.color, text, 0.18),
        ),
        text: mix_color(text, Color32::WHITE, 0.08),
    };
    let active = ButtonVisualState {
        fill: mix_color(fill, Color32::BLACK, 0.28),
        stroke: Stroke::new(
            stroke.width + 0.55,
            mix_color(stroke.color, Color32::WHITE, 0.16),
        ),
        text: mix_color(text, Color32::WHITE, 0.12),
    };
    let noninteractive = ButtonVisualState {
        fill: mix_color(fill, Color32::from_gray(30), 0.35),
        stroke: Stroke::new(
            stroke.width,
            mix_color(stroke.color, Color32::from_gray(96), 0.45),
        ),
        text: mix_color(text, Color32::from_gray(132), 0.35),
    };

    ButtonVisuals {
        inactive,
        hovered,
        active,
        noninteractive,
        corner_radius,
    }
}

fn apply_button_state(
    visuals: &mut WidgetVisuals,
    state: ButtonVisualState,
    corner_radius: CornerRadius,
) {
    visuals.weak_bg_fill = state.fill;
    visuals.bg_fill = state.fill;
    visuals.bg_stroke = state.stroke;
    visuals.corner_radius = corner_radius;
    visuals.fg_stroke = Stroke::new(state.stroke.width, state.text);
}

fn mix_color(base: Color32, accent: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |base_channel: u8, accent_channel: u8| -> u8 {
        let base = base_channel as f32;
        let accent = accent_channel as f32;
        (base + (accent - base) * amount).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        mix(base.r(), accent.r()),
        mix(base.g(), accent.g()),
        mix(base.b(), accent.b()),
    )
}

fn soften_color(accent: Color32, amount: f32) -> Color32 {
    surface_tint(accent, amount * 0.7)
}

fn surface_tint(accent: Color32, amount: f32) -> Color32 {
    mix_color(Color32::from_rgb(4, 6, 10), accent, amount)
}

fn should_show_drop_hint(is_dragging: bool, is_empty: bool) -> bool {
    let _ = is_empty;
    is_dragging
}

fn should_show_item_drop_hint(
    is_dragging: bool,
    is_empty: bool,
    dragged_item_index: Option<usize>,
    target_index: usize,
) -> bool {
    should_show_drop_hint(is_dragging, is_empty)
        && dragged_item_index
            .map(|source_index| target_index != source_index && target_index != source_index + 1)
            .unwrap_or(true)
}

fn should_show_section_drop_hint(
    is_dragging: bool,
    is_empty: bool,
    dragged_section_id: Option<u64>,
    dragged_section_index: Option<usize>,
    target_index: usize,
    suppress_drop_hints_in_subtree: bool,
) -> bool {
    should_show_drop_hint(is_dragging, is_empty)
        && !is_invalid_section_drop_parent(dragged_section_id, suppress_drop_hints_in_subtree)
        && dragged_section_index
            .map(|source_index| target_index != source_index && target_index != source_index + 1)
            .unwrap_or(true)
}

fn is_invalid_section_drop_parent(
    dragged_section_id: Option<u64>,
    suppress_drop_hints_in_subtree: bool,
) -> bool {
    dragged_section_id.is_some() && suppress_drop_hints_in_subtree
}

fn drag_auto_scroll_delta(pointer_y: Option<f32>, viewport: Rect, is_dragging: bool) -> f32 {
    if !is_dragging {
        return 0.0;
    }

    let Some(pointer_y) = pointer_y else {
        return 0.0;
    };

    let top_band_end = viewport.top() + DRAG_AUTO_SCROLL_EDGE;
    if pointer_y < top_band_end {
        let strength = ((top_band_end - pointer_y) / DRAG_AUTO_SCROLL_EDGE).clamp(0.0, 1.0);
        return -strength * DRAG_AUTO_SCROLL_MAX_SPEED;
    }

    let bottom_band_start = viewport.bottom() - DRAG_AUTO_SCROLL_EDGE;
    if pointer_y > bottom_band_start {
        let strength = ((pointer_y - bottom_band_start) / DRAG_AUTO_SCROLL_EDGE).clamp(0.0, 1.0);
        return strength * DRAG_AUTO_SCROLL_MAX_SPEED;
    }

    0.0
}

fn apply_document_action(document: &mut Document, action: DocumentAction) {
    match action {
        DocumentAction::AddTopLevelSection => add_top_level_section(document),
        DocumentAction::AddChild { section_id } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.add_child_section("");
            }
        }
        DocumentAction::AddTextItem { section_id } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.add_text_item();
            }
        }
        DocumentAction::AddLinkItem { section_id } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.add_link_item();
            }
        }
        DocumentAction::AddMultiLinkItem { section_id } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.add_multi_link_item();
            }
        }
        DocumentAction::AddCopyButtonItem { section_id } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.add_copy_button_item();
            }
        }
        DocumentAction::AddLineBreakItem { section_id } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.add_line_break_item();
            }
        }
        DocumentAction::DeleteItem { section_id, index } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id)
                && index < section.items.len()
            {
                section.items.remove(index);
            }
        }
        DocumentAction::MoveItemUp { section_id, index } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.move_item_up(index);
            }
        }
        DocumentAction::MoveItemDown { section_id, index } => {
            if let Some(section) = find_section_mut(&mut document.sections, section_id) {
                section.move_item_down(index);
            }
        }
        DocumentAction::DeleteSection { parent_id, index } => {
            if let Some(section_list) = find_section_list_mut(&mut document.sections, parent_id)
                && index < section_list.len()
            {
                section_list.remove(index);
            }
        }
        DocumentAction::MoveSectionUp { parent_id, index } => {
            if let Some(section_list) = find_section_list_mut(&mut document.sections, parent_id)
                && index > 0
                && index < section_list.len()
            {
                section_list.swap(index, index - 1);
            }
        }
        DocumentAction::MoveSectionDown { parent_id, index } => {
            if let Some(section_list) = find_section_list_mut(&mut document.sections, parent_id)
                && index + 1 < section_list.len()
            {
                section_list.swap(index, index + 1);
            }
        }
        DocumentAction::DropSection {
            section_id,
            target_parent_id,
            target_index,
        } => {
            let _ = document.move_section(section_id, target_parent_id, target_index);
        }
        DocumentAction::DropItem {
            item_id,
            target_section_id,
            target_index,
        } => {
            let _ = document.move_item(item_id, target_section_id, target_index);
        }
    }
}

fn find_section_mut(sections: &mut [Section], section_id: u64) -> Option<&mut Section> {
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

fn find_section_list_mut(
    sections: &mut Vec<Section>,
    parent_id: Option<u64>,
) -> Option<&mut Vec<Section>> {
    match parent_id {
        None => Some(sections),
        Some(parent_id) => find_child_section_list_mut(sections, parent_id),
    }
}

fn find_child_section_list_mut(
    sections: &mut Vec<Section>,
    parent_id: u64,
) -> Option<&mut Vec<Section>> {
    for section in sections {
        if section.id == parent_id {
            return Some(&mut section.sections);
        }
        if let Some(found) = find_child_section_list_mut(&mut section.sections, parent_id) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Item;

    #[test]
    fn add_top_level_section_appends_empty_section() {
        let mut document = Document::default_document();

        add_top_level_section(&mut document);

        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].title, "");
    }

    #[test]
    fn empty_section_title_uses_placeholder_in_header() {
        assert_eq!(section_header_title(&Section::new("")), "Section name");
        assert_eq!(section_header_title(&Section::new("Named")), "Named");
    }

    #[test]
    fn empty_section_title_uses_placeholder_in_manual_header() {
        let section = Section {
            id: 11,
            title: "   \n\t  ".into(),
            sections: vec![Section {
                id: 12,
                title: "Child".into(),
                sections: Vec::new(),
                items: Vec::new(),
            }],
            items: vec![Item::text("Filled")],
        };

        assert_eq!(section_header_title(&section), "Section name");
    }

    #[test]
    fn top_level_styles_use_stable_palette_entries() {
        let first = top_level_section_style(0, &Section::new("Alpha"));
        let second = top_level_section_style(1, &Section::new("Beta"));

        assert_ne!(first.accent, second.accent);
        assert_eq!(first.root_title, "Alpha");
        assert_eq!(first.depth, 0);
    }

    #[test]
    fn business_button_styles_use_distinct_palette_roles() {
        let primary = button_role_visuals(ButtonRole::Primary);
        let secondary = button_role_visuals(ButtonRole::Secondary);
        let danger = button_role_visuals(ButtonRole::Danger);

        assert_ne!(primary.inactive.fill, secondary.inactive.fill);
        assert_ne!(danger.inactive.fill, secondary.inactive.fill);
        assert_ne!(primary.active.fill, primary.inactive.fill);
        assert!(primary.active.stroke.width > primary.inactive.stroke.width);
        assert_eq!(danger.corner_radius.at_least(8), danger.corner_radius);
    }

    #[test]
    fn action_button_min_size_matches_toolbar_and_list_controls() {
        assert_eq!(action_button_min_size(), egui::vec2(72.0, 30.0));
    }

    #[test]
    fn child_sections_inherit_root_title_and_accent() {
        let parent = top_level_section_style(2, &Section::new("Root"));
        let child = inherit_section_style(&parent);

        assert_eq!(child.accent, parent.accent);
        assert_eq!(child.root_title, "Root");
        assert_eq!(child.depth, 1);
        assert_ne!(child.frame_fill, parent.frame_fill);
    }

    #[test]
    fn item_container_frame_uses_section_item_fill_and_accent_stroke() {
        let style = top_level_section_style(0, &Section::new("Alpha"));
        let frame = item_container_frame(&style);

        assert_eq!(frame.fill, style.item_fill);
        assert_eq!(frame.stroke.color, soften_color(style.accent, 0.45));
    }

    #[test]
    fn item_add_button_visuals_use_section_accent() {
        let style = top_level_section_style(1, &Section::new("Alpha"));
        let visuals = item_add_button_visuals(&style);

        assert_eq!(visuals.inactive.fill, surface_tint(style.accent, 0.2));
        assert_eq!(visuals.inactive.stroke.color, soften_color(style.accent, 0.78));
        assert_eq!(visuals.inactive.text, soften_color(style.accent, 0.96));
        assert_ne!(visuals.active.fill, visuals.inactive.fill);
        assert!(visuals.active.stroke.width > visuals.inactive.stroke.width);
    }

    #[test]
    fn child_section_button_visuals_match_item_add_buttons() {
        let style = top_level_section_style(1, &Section::new("Alpha"));
        let item_visuals = item_add_button_visuals(&style);
        let child_visuals = child_section_button_visuals(&style);

        assert_eq!(item_visuals.inactive.fill, child_visuals.inactive.fill);
        assert_eq!(
            item_visuals.inactive.stroke.color,
            child_visuals.inactive.stroke.color
        );
        assert_eq!(item_visuals.inactive.text, child_visuals.inactive.text);
        assert_eq!(item_visuals.active.fill, child_visuals.active.fill);
    }

    #[test]
    fn top_level_add_button_visuals_use_toolbar_accent() {
        let visuals = top_level_add_button_visuals();

        assert_eq!(visuals.inactive.fill, surface_tint(palette_color(0), 0.2));
        assert_eq!(
            visuals.inactive.stroke.color,
            soften_color(palette_color(0), 0.78)
        );
        assert_ne!(visuals.hovered.fill, visuals.inactive.fill);
    }

    #[test]
    fn item_kind_titles_match_editor_types() {
        assert_eq!(
            item_kind_title(&ItemKind::Text {
                text: String::new()
            }),
            "Text Item"
        );
        assert_eq!(
            item_kind_title(&ItemKind::Link {
                text: String::new(),
                url: String::new(),
                shortcut: Shortcut::None,
            }),
            "Link Item"
        );
        assert_eq!(
            item_kind_title(&ItemKind::MultiLink {
                text: String::new(),
                urls: String::new(),
                shortcut: Shortcut::None,
            }),
            "Multi-Link Item"
        );
        assert_eq!(
            item_kind_title(&ItemKind::CopyButton {
                text: String::new(),
                body: String::new(),
            }),
            "Copy Button"
        );
        assert_eq!(item_kind_title(&ItemKind::LineBreak), "Line Break");
    }

    #[test]
    fn section_header_id_is_stable_across_title_changes() {
        let original = Section {
            id: 42,
            title: "First".into(),
            sections: Vec::new(),
            items: Vec::new(),
        };
        let renamed = Section {
            title: "Renamed".into(),
            ..original.clone()
        };

        assert_eq!(section_header_id(&original), section_header_id(&renamed));
    }

    #[test]
    fn section_header_state_uses_stable_id_for_same_section() {
        let section = Section {
            id: 7,
            title: "Alpha".into(),
            sections: Vec::new(),
            items: Vec::new(),
        };

        let renamed = Section {
            title: "Beta".into(),
            ..section.clone()
        };

        assert_eq!(
            section_header_state_id(&section),
            section_header_state_id(&renamed)
        );
    }

    #[test]
    fn sticky_header_top_clamps_to_viewport_top() {
        assert_eq!(sticky_header_top(12.0, 48.0, 20.0, None), 48.0);
    }

    #[test]
    fn section_header_hit_rect_spans_available_width() {
        let header_rect = Rect::from_min_max(egui::pos2(24.0, 10.0), egui::pos2(96.0, 34.0));
        let available_rect = Rect::from_min_max(egui::pos2(8.0, 0.0), egui::pos2(220.0, 80.0));

        let hit_rect = section_header_hit_rect(header_rect, available_rect);

        assert_eq!(hit_rect.left(), available_rect.left());
        assert_eq!(hit_rect.right(), available_rect.right());
        assert_eq!(hit_rect.top(), header_rect.top());
        assert_eq!(hit_rect.bottom(), header_rect.bottom());
    }

    #[test]
    fn section_header_toggle_requested_ignores_toggle_button_click() {
        assert!(section_header_toggle_requested(false, true));
        assert!(!section_header_toggle_requested(true, true));
        assert!(!section_header_toggle_requested(true, false));
    }

    #[test]
    fn section_header_id_distinguishes_duplicate_titles() {
        let left = Section {
            id: 1,
            title: "Same".into(),
            sections: Vec::new(),
            items: Vec::new(),
        };
        let right = Section {
            id: 2,
            title: "Same".into(),
            sections: Vec::new(),
            items: Vec::new(),
        };

        assert_ne!(section_header_id(&left), section_header_id(&right));
    }

    #[test]
    fn apply_drop_section_action_moves_section_via_document_model() {
        let mut document = Document {
            sections: vec![
                Section {
                    id: 1,
                    title: "Parent".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
                Section {
                    id: 2,
                    title: "Moved".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
            ],
        };

        apply_document_action(
            &mut document,
            DocumentAction::DropSection {
                section_id: 2,
                target_parent_id: Some(1),
                target_index: 0,
            },
        );

        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].sections.len(), 1);
        assert_eq!(document.sections[0].sections[0].title, "Moved");
    }

    #[test]
    fn apply_drop_item_action_moves_item_via_document_model() {
        let mut document = Document {
            sections: vec![
                Section {
                    id: 1,
                    title: "Source".into(),
                    sections: Vec::new(),
                    items: vec![Item::text("A")],
                },
                Section {
                    id: 2,
                    title: "Target".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
            ],
        };
        let moved_item_id = document.sections[0].items[0].id;

        apply_document_action(
            &mut document,
            DocumentAction::DropItem {
                item_id: moved_item_id,
                target_section_id: 2,
                target_index: 0,
            },
        );

        assert!(document.sections[0].items.is_empty());
        assert_eq!(document.sections[1].items.len(), 1);
    }

    #[test]
    fn move_section_up_allows_top_level_reordering() {
        let mut document = Document {
            sections: vec![
                Section {
                    id: 1,
                    title: "First".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
                Section {
                    id: 99,
                    title: "Other".into(),
                    sections: Vec::new(),
                    items: Vec::new(),
                },
            ],
        };

        apply_document_action(
            &mut document,
            DocumentAction::MoveSectionUp {
                parent_id: None,
                index: 1,
            },
        );

        assert_eq!(document.sections[0].title, "Other");
        assert_eq!(document.sections[1].title, "First");
    }

    #[test]
    fn drop_hint_is_hidden_when_not_dragging() {
        assert!(!should_show_drop_hint(false, false));
        assert!(!should_show_drop_hint(false, true));
    }

    #[test]
    fn drop_hint_is_shown_only_while_dragging() {
        assert!(should_show_drop_hint(true, false));
        assert!(should_show_drop_hint(true, true));
    }

    #[test]
    fn item_drop_hint_is_hidden_for_drag_origin_and_immediate_following_slot() {
        assert!(!should_show_item_drop_hint(true, false, Some(2), 2));
        assert!(!should_show_item_drop_hint(true, false, Some(2), 3));
    }

    #[test]
    fn item_drop_hint_remains_visible_for_other_slots_while_dragging() {
        assert!(should_show_item_drop_hint(true, false, Some(2), 1));
        assert!(should_show_item_drop_hint(true, false, Some(2), 4));
        assert!(should_show_item_drop_hint(true, true, None, 0));
    }

    #[test]
    fn section_drop_hint_is_hidden_for_drag_origin_and_immediate_following_slot() {
        assert!(!should_show_section_drop_hint(
            true,
            false,
            Some(99),
            Some(2),
            2,
            false,
        ));
        assert!(!should_show_section_drop_hint(
            true,
            false,
            Some(99),
            Some(2),
            3,
            false,
        ));
    }

    #[test]
    fn section_drop_hint_remains_visible_for_other_slots_while_dragging() {
        assert!(should_show_section_drop_hint(
            true,
            false,
            Some(99),
            Some(2),
            1,
            false,
        ));
        assert!(should_show_section_drop_hint(
            true,
            false,
            Some(99),
            Some(2),
            4,
            false,
        ));
        assert!(should_show_section_drop_hint(
            true, true, None, None, 0, false
        ));
    }

    #[test]
    fn section_drop_hint_is_hidden_for_dragged_sections_own_child_list() {
        assert!(is_invalid_section_drop_parent(Some(10), true));
    }

    #[test]
    fn drag_auto_scroll_delta_is_zero_without_dragging() {
        let viewport = Rect::from_min_max([0.0, 100.0].into(), [240.0, 500.0].into());

        assert_eq!(drag_auto_scroll_delta(None, viewport, false), 0.0);
        assert_eq!(drag_auto_scroll_delta(Some(110.0), viewport, false), 0.0);
    }

    #[test]
    fn drag_auto_scroll_delta_scrolls_up_near_top_edge() {
        let viewport = Rect::from_min_max([0.0, 100.0].into(), [240.0, 500.0].into());

        let delta = drag_auto_scroll_delta(Some(114.0), viewport, true);

        assert!(delta < 0.0);
        assert_eq!(delta, -18.0);
    }

    #[test]
    fn drag_auto_scroll_delta_scrolls_down_near_bottom_edge() {
        let viewport = Rect::from_min_max([0.0, 100.0].into(), [240.0, 500.0].into());

        let delta = drag_auto_scroll_delta(Some(486.0), viewport, true);

        assert!(delta > 0.0);
        assert_eq!(delta, 18.0);
    }

    #[test]
    fn drag_auto_scroll_delta_is_zero_away_from_edges() {
        let viewport = Rect::from_min_max([0.0, 100.0].into(), [240.0, 500.0].into());

        assert_eq!(drag_auto_scroll_delta(Some(300.0), viewport, true), 0.0);
    }
}
