use crate::config::{Config, Position};
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, DropDown, Entry, HeaderBar,
    Label, ListBox, ListBoxRow, Orientation, Scale, ScrolledWindow, SelectionMode, Separator,
    Stack, StringList, Switch, ToggleButton,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, info};

const POSITION_OPTIONS: [(&str, Position); 9] = [
    ("Top Left", Position::TopLeft),
    ("Top Center", Position::TopCenter),
    ("Top Right", Position::TopRight),
    ("Middle Left", Position::MiddleLeft),
    ("Center", Position::Center),
    ("Middle Right", Position::MiddleRight),
    ("Bottom Left", Position::BottomLeft),
    ("Bottom Center", Position::BottomCenter),
    ("Bottom Right", Position::BottomRight),
];

pub fn create_settings_window(
    app: &Application,
    config: Rc<RefCell<Config>>,
    on_save: impl Fn(Config) + 'static,
) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Settings")
        .default_width(1100)
        .default_height(750)
        .resizable(true)
        .build();

    // Main layout: Horizontal box replacing NavigationSplitView
    let main_box = GtkBox::new(Orientation::Horizontal, 0);

    // Sidebar
    let sidebar_box = GtkBox::new(Orientation::Vertical, 0);
    sidebar_box.set_width_request(250);
    sidebar_box.add_css_class("sidebar"); // Use standard sidebar class if available

    let sidebar_header = HeaderBar::new();
    sidebar_header.set_show_title_buttons(false); // Only show buttons on the main content side usually, or consistent
                                                  // Actually, in a split view, often the left side has a header or just empty space.
                                                  // Let's just put a label or keep it simple.
    let sidebar_title = Label::builder()
        .label("Settings")
        .css_classes(vec!["title"])
        .build();
    sidebar_header.set_title_widget(Some(&sidebar_title));
    sidebar_box.append(&sidebar_header);

    let sidebar_list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(vec!["navigation-sidebar"])
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let keystroke_row = create_sidebar_row("Keystrokes", "input-keyboard-symbolic");
    let bubble_row = create_sidebar_row("Bubbles", "comment-symbolic");

    sidebar_list.append(&keystroke_row);
    sidebar_list.append(&bubble_row);

    let sidebar_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&sidebar_list)
        .build();

    sidebar_box.append(&sidebar_scroll);
    main_box.append(&sidebar_box);

    // Separator
    let separator = Separator::new(Orientation::Vertical);
    main_box.append(&separator);

    // Content
    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.set_hexpand(true);

    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    content_box.append(&header_bar);

    let stack = Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_vexpand(true);

    let config_ref = config.borrow();
    let keystroke_page = create_keystroke_page(&config_ref);
    let bubble_page = create_bubble_page(&config_ref);

    stack.add_named(&keystroke_page.page, Some("keystrokes"));
    stack.add_named(&bubble_page, Some("bubbles"));

    content_box.append(&stack);

    let footer = GtkBox::builder()
        .css_classes(vec!["toolbar"])
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .halign(Align::End)
        .build();

    let reset_btn = Button::with_label("Reset to Defaults");
    reset_btn.add_css_class("destructive-action");

    let save_btn = Button::with_label("Save Settings");
    save_btn.add_css_class("suggested-action");
    save_btn.add_css_class("pill");

    footer.append(&reset_btn);
    footer.append(&save_btn);
    content_box.append(&footer);

    main_box.append(&content_box);
    window.set_child(Some(&main_box));

    // Signals
    let stack_clone = stack.clone();
    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let name = if row.index() == 0 {
                "keystrokes"
            } else {
                "bubbles"
            };
            stack_clone.set_visible_child_name(name);
        }
    });
    sidebar_list.select_row(Some(&keystroke_row));

    let window_clone = window.clone();
    let config_clone = config.clone();

    save_btn.connect_clicked(move |_| {
        let theme = if keystroke_page.theme_light.is_active() {
            "light"
        } else if keystroke_page.theme_dark.is_active() {
            "dark"
        } else {
            "system"
        };

        let ks_pos_idx = keystroke_page.position_dropdown.selected();
        let ks_position = POSITION_OPTIONS
            .get(ks_pos_idx as usize)
            .map(|(_, p)| *p)
            .unwrap_or(Position::BottomCenter);

        let mut new_config = config_clone.borrow().clone();
        new_config.keystroke_theme = theme.to_string();
        new_config.display_timeout_ms = (keystroke_page.duration_adj.value() * 1000.0) as u64;
        new_config.max_keys = keystroke_page.max_keys_adj.value() as usize;
        new_config.position = ks_position;
        new_config.keystroke_draggable = keystroke_page.draggable_switch.is_active();
        new_config.keystroke_hotkey = keystroke_page.hotkey_entry.text().to_string();

        debug!("Saving settings...");
        *config_clone.borrow_mut() = new_config.clone();
        if let Err(e) = new_config.save() {
            tracing::warn!("Failed to save config: {}", e);
        } else {
            info!("Config saved");
        }
        on_save(new_config);
        window_clone.close();
    });

    let window_clone = window.clone();
    reset_btn.connect_clicked(move |_| {
        window_clone.close();
    });

    window
}

struct KeystrokeWidgets {
    page: ScrolledWindow, // Changed from PreferencesPage
    theme_light: ToggleButton,
    theme_dark: ToggleButton,
    duration_adj: gtk4::Adjustment,
    max_keys_adj: gtk4::Adjustment,
    position_dropdown: DropDown, // Changed from ComboRow
    draggable_switch: Switch,    // Changed from SwitchRow
    hotkey_entry: Entry,
}

fn create_sidebar_row(title: &str, icon_name: &str) -> ListBoxRow {
    let row = ListBoxRow::new();
    let box_ = GtkBox::new(Orientation::Horizontal, 12);
    box_.set_margin_top(8);
    box_.set_margin_bottom(8);
    box_.set_margin_start(12);
    box_.set_margin_end(12);

    let icon = gtk4::Image::from_icon_name(icon_name);
    let label = Label::new(Some(title));

    box_.append(&icon);
    box_.append(&label);
    row.set_child(Some(&box_));
    row
}

fn create_group(title: &str, _subtitle: Option<&str>, content_list: &ListBox) -> GtkBox {
    let group_box = GtkBox::new(Orientation::Vertical, 6);
    group_box.set_margin_top(12);
    group_box.set_margin_bottom(12);
    group_box.set_margin_start(24);
    group_box.set_margin_end(24);

    let title_label = Label::builder()
        .label(title)
        .xalign(0.0)
        .css_classes(vec!["heading"])
        .build();
    group_box.append(&title_label);

    // Add styling to listbox to look like a group
    content_list.add_css_class("boxed-list");
    group_box.append(content_list);

    group_box
}

fn create_row(title: &str, subtitle: Option<&str>) -> (ListBoxRow, GtkBox) {
    let row = ListBoxRow::new();
    // row.set_activatable(false); // Usually controls shouldn't trigger row activation in the list sense

    let box_ = GtkBox::new(Orientation::Horizontal, 12);
    box_.set_margin_top(8);
    box_.set_margin_bottom(8);
    box_.set_margin_start(12);
    box_.set_margin_end(12);
    box_.set_valign(Align::Center);

    let text_box = GtkBox::new(Orientation::Vertical, 2);
    text_box.set_valign(Align::Center);
    text_box.set_hexpand(true);

    let title_lbl = Label::builder()
        .label(title)
        .xalign(0.0)
        .css_classes(vec!["body"])
        .build();
    text_box.append(&title_lbl);

    if let Some(sub) = subtitle {
        let sub_lbl = Label::builder()
            .label(sub)
            .xalign(0.0)
            .css_classes(vec!["caption", "dim-label"])
            .build();
        text_box.append(&sub_lbl);
    }

    box_.append(&text_box);
    row.set_child(Some(&box_));

    (row, box_)
}

fn create_keystroke_page(config: &Config) -> KeystrokeWidgets {
    let container = GtkBox::new(Orientation::Vertical, 0);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&container)
        .build();

    // Live Preview
    let preview_list = ListBox::new();
    preview_list.set_selection_mode(SelectionMode::None);

    let preview_row = ListBoxRow::new();
    let preview_card = GtkBox::builder()
        .css_classes(vec!["card"])
        .height_request(150)
        .halign(Align::Fill)
        .valign(Align::Center)
        .build();

    let center_box = GtkBox::new(Orientation::Horizontal, 8);
    center_box.set_halign(Align::Center);
    center_box.set_valign(Align::Center);

    let ctrl_lbl = create_keycap("Ctrl");
    let c_lbl = create_keycap("C");

    center_box.append(&ctrl_lbl);
    center_box.append(&c_lbl);
    preview_card.append(&center_box);
    preview_row.set_child(Some(&preview_card));
    preview_list.append(&preview_row);

    let preview_group = create_group("LIVE PREVIEW", None, &preview_list);
    container.append(&preview_group);

    // Behavior
    let behavior_list = ListBox::new();
    behavior_list.set_selection_mode(SelectionMode::None);

    let (hotkey_row, hotkey_box) = create_row(
        "Hotkey Activation",
        Some("Press keys to set enable/disable toggle"),
    );

    let hotkey_entry = Entry::builder()
        .text(&config.keystroke_hotkey)
        .valign(Align::Center)
        .css_classes(vec!["flat"])
        .width_request(100)
        .build();
    hotkey_box.append(&hotkey_entry);
    behavior_list.append(&hotkey_row);

    let (draggable_row, draggable_box) = create_row(
        "Draggable Overlay",
        Some("Allow moving the overlay with mouse"),
    );
    let draggable_switch = Switch::builder()
        .active(config.keystroke_draggable)
        .valign(Align::Center)
        .build();
    draggable_box.append(&draggable_switch);
    behavior_list.append(&draggable_row);

    let behavior_group = create_group("BEHAVIOR", None, &behavior_list);
    container.append(&behavior_group);

    // Appearance
    let appearance_list = ListBox::new();
    appearance_list.set_selection_mode(SelectionMode::None);

    let (theme_row, theme_box) = create_row("Theme Style", None);

    let theme_toggle_box = GtkBox::builder()
        .css_classes(vec!["linked"])
        .valign(Align::Center)
        .build();

    let theme_light = ToggleButton::builder().label("Light").build();
    let theme_dark = ToggleButton::builder().label("Dark").build();
    let theme_sys = ToggleButton::builder().label("System").build();

    theme_toggle_box.append(&theme_light);
    theme_toggle_box.append(&theme_dark);
    theme_toggle_box.append(&theme_sys);

    theme_dark.set_group(Some(&theme_light));
    theme_sys.set_group(Some(&theme_light));

    match config.keystroke_theme.as_str() {
        "light" => theme_light.set_active(true),
        "dark" => theme_dark.set_active(true),
        _ => theme_sys.set_active(true),
    }

    theme_box.append(&theme_toggle_box);
    appearance_list.append(&theme_row);

    let (duration_row, duration_box) =
        create_row("Display Duration", Some("How long keys stay visible"));

    let duration_adj = gtk4::Adjustment::new(
        config.display_timeout_ms as f64 / 1000.0,
        0.5,
        10.0,
        0.5,
        1.0,
        0.0,
    );

    let scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&duration_adj)
        .draw_value(false)
        .hexpand(true)
        .width_request(150)
        .build();
    scale.add_css_class("accent");

    let val_label = Label::new(Some("3.5s")); // Placeholder for dynamic
                                              // In a real implementation we'd connect signal_value_changed to update label

    let scale_container = GtkBox::new(Orientation::Horizontal, 12);
    scale_container.append(&scale);
    scale_container.append(&val_label);
    duration_box.append(&scale_container);
    appearance_list.append(&duration_row);

    let (max_keys_row, max_keys_box) =
        create_row("Max Keys Displayed", Some("Number of recent keys shown"));

    let max_keys_adj = gtk4::Adjustment::new(config.max_keys as f64, 1.0, 20.0, 1.0, 0.0, 0.0);
    let spin_btn = gtk4::SpinButton::builder()
        .adjustment(&max_keys_adj)
        .valign(Align::Center)
        .build();

    max_keys_box.append(&spin_btn);
    appearance_list.append(&max_keys_row);

    let (position_row, position_box) = create_row("Screen Position", Some("Default anchor point"));

    let position_dropdown = DropDown::builder()
        .model(&StringList::new(&POSITION_OPTIONS.map(|(n, _)| n)))
        .selected(
            POSITION_OPTIONS
                .iter()
                .position(|(_, p)| *p == config.position)
                .unwrap_or(7) as u32,
        )
        .valign(Align::Center)
        .build();

    position_box.append(&position_dropdown);
    appearance_list.append(&position_row);

    let appearance_group = create_group("APPEARANCE", None, &appearance_list);
    container.append(&appearance_group);

    KeystrokeWidgets {
        page: scrolled,
        theme_light,
        theme_dark,
        duration_adj,
        max_keys_adj,
        position_dropdown,
        draggable_switch,
        hotkey_entry,
    }
}

fn create_bubble_page(_config: &Config) -> ScrolledWindow {
    let container = GtkBox::new(Orientation::Vertical, 0);
    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::None);

    let (row, _) = create_row(
        "Bubble Settings",
        Some("Bubble configuration is not yet refactored."),
    );
    list.append(&row);

    let group = create_group("Bubble Settings", None, &list);
    container.append(&group);

    ScrolledWindow::builder().child(&container).build()
}

fn create_keycap(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.add_css_class("keycap");
    lbl.set_width_request(50);
    lbl.set_height_request(50);
    lbl
}

pub fn show_settings(window: &ApplicationWindow) {
    window.present();
}
