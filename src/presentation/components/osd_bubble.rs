use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation};

/// OSD Bubble component that displays system notifications.
/// Designed as a pill-shaped, semi-transparent overlay.
pub struct OsdBubble {
    container: GtkBox,
    label: Label,
}

impl OsdBubble {
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        container.add_css_class("osd-bubble");

        let label = Label::builder()
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        label.add_css_class("osd-label");

        container.append(&label);

        Self { container, label }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_text(&self, text: &str) {
        self.label.set_text(text);
    }
}
