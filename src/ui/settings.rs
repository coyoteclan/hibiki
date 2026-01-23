use crate::application::config_service::ConfigService;
use crate::domain::config::Position;
use gtk4::prelude::*;
use gtk4::{
    glib, Adjustment, Align, Application, ApplicationWindow, Box as GtkBox, Button, DropDown,
    EventControllerKey, Label, ListBox, ListBoxRow, Orientation, Scale, ScrolledWindow,
    SelectionMode, SpinButton, Stack, StackSwitcher, StringList, Switch, ToggleButton,
};
use std::cell::RefCell;
use std::rc::Rc;

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
    config_service: ConfigService,
) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Settings")
        .default_width(500)
        .default_height(600)
        .resizable(true)
        .build();

    let main_box = GtkBox::new(Orientation::Vertical, 0);

    let stack = Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_vexpand(true);

    let switcher = StackSwitcher::builder()
        .stack(&stack)
        .halign(Align::Center)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    main_box.append(&switcher);

    let keystroke_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();
    let keystroke_content = create_keystroke_settings(&config_service, &window);
    keystroke_scroll.set_child(Some(&keystroke_content));
    stack.add_titled(&keystroke_scroll, Some("keystroke"), "Keystrokes");

    let bubble_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();
    let bubble_content = create_bubble_settings(&config_service, &window);
    bubble_scroll.set_child(Some(&bubble_content));
    stack.add_titled(&bubble_scroll, Some("bubble"), "Bubbles");

    main_box.append(&stack);
    window.set_child(Some(&main_box));

    window
}

fn create_keystroke_settings(config_service: &ConfigService, window: &ApplicationWindow) -> GtkBox {
    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.set_margin_top(12);
    content_box.set_margin_bottom(24);
    content_box.set_margin_start(24);
    content_box.set_margin_end(24);
    content_box.set_valign(Align::Start);
    content_box.set_halign(Align::Center);
    content_box.set_width_request(450);

    let config = config_service.get_config();

    let appearance_list = create_preferences_group("Appearance", &content_box);

    let theme_row = create_action_row("Theme Style", None);
    let theme_box = GtkBox::new(Orientation::Horizontal, 0);
    theme_box.add_css_class("linked");
    theme_box.set_valign(Align::Center);

    let theme_light = ToggleButton::builder().label("Light").build();
    let theme_dark = ToggleButton::builder().label("Dark").build();
    let theme_sys = ToggleButton::builder().label("System").build();
    theme_dark.set_group(Some(&theme_light));
    theme_sys.set_group(Some(&theme_light));

    match config.keystroke_theme.as_str() {
        "light" => theme_light.set_active(true),
        "dark" => theme_dark.set_active(true),
        _ => theme_sys.set_active(true),
    }

    theme_box.append(&theme_light);
    theme_box.append(&theme_dark);
    theme_box.append(&theme_sys);
    theme_row.add_suffix(&theme_box);
    appearance_list.append(&theme_row.row);

    let position_row = create_action_row("Screen Position", Some("Where the visualizer appears"));
    let position_model = StringList::new(&POSITION_OPTIONS.map(|(n, _)| n));
    let position_dropdown = DropDown::builder()
        .model(&position_model)
        .valign(Align::Center)
        .build();

    if let Some(idx) = POSITION_OPTIONS
        .iter()
        .position(|(_, p)| *p == config.position)
    {
        position_dropdown.set_selected(idx as u32);
    }

    position_row.add_suffix(&position_dropdown);
    appearance_list.append(&position_row.row);

    let behavior_list = create_preferences_group("Behavior", &content_box);

    let duration_row = create_action_row(
        "Display Duration",
        Some("How long keys stay visible (seconds)"),
    );
    let duration_adj = Adjustment::new(
        config.display_timeout_ms as f64 / 1000.0,
        0.5,
        10.0,
        0.5,
        1.0,
        0.0,
    );
    let duration_scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&duration_adj)
        .draw_value(true)
        .value_pos(gtk4::PositionType::Right)
        .width_request(120)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    duration_row.add_suffix(&duration_scale);
    behavior_list.append(&duration_row.row);

    let max_keys_row = create_action_row("Max Keys", Some("Maximum number of keys to show"));
    let max_keys_adj = Adjustment::new(config.max_keys as f64, 1.0, 20.0, 1.0, 0.0, 0.0);
    let max_keys_spin = SpinButton::builder()
        .adjustment(&max_keys_adj)
        .valign(Align::Center)
        .build();
    max_keys_row.add_suffix(&max_keys_spin);
    behavior_list.append(&max_keys_row.row);

    let draggable_row = create_action_row("Draggable", Some("Move with mouse"));
    let draggable_switch = Switch::builder()
        .active(config.keystroke_draggable)
        .valign(Align::Center)
        .build();
    draggable_row.add_suffix(&draggable_switch);
    behavior_list.append(&draggable_row.row);

    let hotkey_row = create_action_row("Activation Hotkey", Some("Switch to Keystrokes"));
    let hotkey_btn = Button::builder()
        .label(&config.keystroke_hotkey)
        .valign(Align::Center)
        .build();

    setup_hotkey_capture(
        window,
        &hotkey_btn,
        config_service.clone(),
        HotkeyType::KeystrokeActivation,
    );
    hotkey_row.add_suffix(&hotkey_btn);
    behavior_list.append(&hotkey_row.row);

    let pause_row = create_action_row("Pause Hotkey", Some("Toggle Pause/Resume"));
    let pause_btn = Button::builder()
        .label(&config.pause_hotkey)
        .valign(Align::Center)
        .build();

    setup_hotkey_capture(
        window,
        &pause_btn,
        config_service.clone(),
        HotkeyType::Pause,
    );
    pause_row.add_suffix(&pause_btn);
    behavior_list.append(&pause_row.row);


    let service_c = config_service.clone();
    let theme_light_c = theme_light.clone();
    let theme_dark_c = theme_dark.clone();

    let update_theme = Rc::new(move || {
        let theme = if theme_light_c.is_active() {
            "light"
        } else if theme_dark_c.is_active() {
            "dark"
        } else {
            "system"
        };
        let mut cfg = service_c.get_config();
        if cfg.keystroke_theme != theme {
            cfg.keystroke_theme = theme.to_string();
            let _ = service_c.update_config(cfg);
        }
    });

    let u = update_theme.clone();
    theme_light.connect_toggled(move |_| u());
    let u = update_theme.clone();
    theme_dark.connect_toggled(move |_| u());
    let u = update_theme.clone();
    theme_sys.connect_toggled(move |_| u());

    let service_c = config_service.clone();
    position_dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected();
        let pos = POSITION_OPTIONS
            .get(idx as usize)
            .map(|(_, p)| *p)
            .unwrap_or(Position::BottomCenter);
        let mut cfg = service_c.get_config();
        if cfg.position != pos {
            cfg.position = pos;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    duration_adj.connect_value_changed(move |adj| {
        let val = (adj.value() * 1000.0) as u64;
        let mut cfg = service_c.get_config();
        if cfg.display_timeout_ms != val {
            cfg.display_timeout_ms = val;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    max_keys_adj.connect_value_changed(move |adj| {
        let val = adj.value() as usize;
        let mut cfg = service_c.get_config();
        if cfg.max_keys != val {
            cfg.max_keys = val;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    draggable_switch.connect_state_set(move |_, state| {
        let mut cfg = service_c.get_config();
        if cfg.keystroke_draggable != state {
            cfg.keystroke_draggable = state;
            let _ = service_c.update_config(cfg);
        }
        glib::Propagation::Proceed
    });

    content_box
}

fn create_bubble_settings(config_service: &ConfigService, window: &ApplicationWindow) -> GtkBox {
    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.set_margin_top(12);
    content_box.set_margin_bottom(24);
    content_box.set_margin_start(24);
    content_box.set_margin_end(24);
    content_box.set_valign(Align::Start);
    content_box.set_halign(Align::Center);
    content_box.set_width_request(450);

    let config = config_service.get_config();

    let appearance_list = create_preferences_group("Appearance", &content_box);

    let position_row = create_action_row("Screen Position", Some("Where the bubbles appear"));
    let position_model = StringList::new(&POSITION_OPTIONS.map(|(n, _)| n));
    let position_dropdown = DropDown::builder()
        .model(&position_model)
        .valign(Align::Center)
        .build();

    if let Some(idx) = POSITION_OPTIONS
        .iter()
        .position(|(_, p)| *p == config.bubble_position)
    {
        position_dropdown.set_selected(idx as u32);
    }

    position_row.add_suffix(&position_dropdown);
    appearance_list.append(&position_row.row);

    let behavior_list = create_preferences_group("Behavior", &content_box);

    let duration_row = create_action_row(
        "Display Duration",
        Some("How long bubbles stay visible (seconds)"),
    );
    let duration_adj = Adjustment::new(
        config.bubble_timeout_ms as f64 / 1000.0,
        1.0,
        20.0,
        1.0,
        1.0,
        0.0,
    );
    let duration_scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&duration_adj)
        .draw_value(true)
        .value_pos(gtk4::PositionType::Right)
        .width_request(120)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    duration_row.add_suffix(&duration_scale);
    behavior_list.append(&duration_row.row);

    let draggable_row = create_action_row("Draggable", Some("Move with mouse"));
    let draggable_switch = Switch::builder()
        .active(config.bubble_draggable)
        .valign(Align::Center)
        .build();
    draggable_row.add_suffix(&draggable_switch);
    behavior_list.append(&draggable_row.row);

    let hotkey_row = create_action_row("Activation Hotkey", Some("Switch to Bubbles"));
    let hotkey_btn = Button::builder()
        .label(&config.bubble_hotkey)
        .valign(Align::Center)
        .build();

    setup_hotkey_capture(
        window,
        &hotkey_btn,
        config_service.clone(),
        HotkeyType::BubbleActivation,
    );
    hotkey_row.add_suffix(&hotkey_btn);
    behavior_list.append(&hotkey_row.row);

    let focus_row = create_action_row("Focus Hotkey", Some("Toggle Click-through"));
    let focus_btn = Button::builder()
        .label(&config.toggle_focus_hotkey)
        .valign(Align::Center)
        .build();

    setup_hotkey_capture(
        window,
        &focus_btn,
        config_service.clone(),
        HotkeyType::ToggleFocus,
    );
    focus_row.add_suffix(&focus_btn);
    behavior_list.append(&focus_row.row);


    let service_c = config_service.clone();
    position_dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected();
        let pos = POSITION_OPTIONS
            .get(idx as usize)
            .map(|(_, p)| *p)
            .unwrap_or(Position::TopRight);
        let mut cfg = service_c.get_config();
        if cfg.bubble_position != pos {
            cfg.bubble_position = pos;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    duration_adj.connect_value_changed(move |adj| {
        let val = (adj.value() * 1000.0) as u64;
        let mut cfg = service_c.get_config();
        if cfg.bubble_timeout_ms != val {
            cfg.bubble_timeout_ms = val;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    draggable_switch.connect_state_set(move |_, state| {
        let mut cfg = service_c.get_config();
        if cfg.bubble_draggable != state {
            cfg.bubble_draggable = state;
            let _ = service_c.update_config(cfg);
        }
        glib::Propagation::Proceed
    });

    content_box
}

fn create_preferences_group(title: &str, parent: &GtkBox) -> ListBox {
    let group_box = GtkBox::new(Orientation::Vertical, 6);
    group_box.set_margin_bottom(24);

    let title_label = Label::builder()
        .label(title)
        .xalign(0.0)
        .css_classes(vec!["heading"])
        .margin_bottom(6)
        .build();
    group_box.append(&title_label);

    let list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(vec!["boxed-list"])
        .build();

    group_box.append(&list);
    parent.append(&group_box);
    list
}

pub fn show_settings(window: &ApplicationWindow) {
    window.present();
}

struct ActionRow {
    pub row: ListBoxRow,
    pub suffix_box: GtkBox,
}

impl ActionRow {
    fn add_suffix(&self, widget: &impl IsA<gtk4::Widget>) {
        self.suffix_box.append(widget);
    }
}

fn create_action_row(title: &str, subtitle: Option<&str>) -> ActionRow {
    let row = ListBoxRow::new();
    row.set_activatable(false);
    row.set_height_request(50);

    let box_ = GtkBox::new(Orientation::Horizontal, 12);
    box_.set_margin_start(12);
    box_.set_margin_end(12);
    box_.set_margin_top(8);
    box_.set_margin_bottom(8);
    box_.set_valign(Align::Center);

    let prefix_box = GtkBox::new(Orientation::Horizontal, 6);
    box_.append(&prefix_box);

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

    let suffix_box = GtkBox::new(Orientation::Horizontal, 6);
    suffix_box.set_valign(Align::Center);
    box_.append(&suffix_box);

    row.set_child(Some(&box_));

    ActionRow { row, suffix_box }
}

enum HotkeyType {
    KeystrokeActivation,
    BubbleActivation,
    Pause,
    ToggleFocus,
}

fn setup_hotkey_capture(
    window: &ApplicationWindow,
    button: &Button,
    config_service: ConfigService,
    hotkey_type: HotkeyType,
) {
    let controller = EventControllerKey::new();
    let listening = Rc::new(RefCell::new(false));

    let listening_c = listening.clone();
    let btn_c = button.clone();
    let service_c = config_service.clone();

    controller.connect_key_pressed(move |_, keyval, _keycode, state| {
        if *listening_c.borrow() {
            if is_modifier(keyval) {
                let accelerator = gtk4::accelerator_name(keyval, state);
                btn_c.set_label(&accelerator);
                return glib::Propagation::Stop;
            }

            let accelerator = gtk4::accelerator_name(keyval, state);
            if !accelerator.is_empty() {
                let mut cfg = service_c.get_config();
                match hotkey_type {
                    HotkeyType::KeystrokeActivation => {
                        cfg.keystroke_hotkey = accelerator.to_string()
                    }
                    HotkeyType::BubbleActivation => cfg.bubble_hotkey = accelerator.to_string(),
                    HotkeyType::Pause => cfg.pause_hotkey = accelerator.to_string(),
                    HotkeyType::ToggleFocus => cfg.toggle_focus_hotkey = accelerator.to_string(),
                }

                let _ = service_c.update_config(cfg);

                btn_c.set_label(&accelerator);
                btn_c.remove_css_class("suggested-action");
                *listening_c.borrow_mut() = false;
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });

    window.add_controller(controller);

    let listening_c = listening.clone();
    button.connect_clicked(move |btn| {
        *listening_c.borrow_mut() = true;
        btn.set_label("Press keys...");
        btn.add_css_class("suggested-action");
    });
}

fn is_modifier(key: gtk4::gdk::Key) -> bool {
    use gtk4::gdk::Key;
    matches!(
        key,
        Key::Control_L
            | Key::Control_R
            | Key::Alt_L
            | Key::Alt_R
            | Key::Shift_L
            | Key::Shift_R
            | Key::Super_L
            | Key::Super_R
            | Key::Meta_L
            | Key::Meta_R
            | Key::Caps_Lock
            | Key::Shift_Lock
    )
}
