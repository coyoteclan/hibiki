use crate::application::config_service::ConfigService;
use crate::application::typography_service::TypographyService;
use crate::domain::config::{AudioConfig, Position};
use crate::infrastructure::audio::SoundPackLoader;
use gtk4::prelude::*;
use gtk4::{
    glib, Adjustment, Align, Application, ApplicationWindow, Box as GtkBox, Button,
    EventControllerKey, Grid, Label, ListBox, ListBoxRow, Orientation, Overflow, Scale,
    ScrolledWindow, SearchEntry, SelectionMode, Stack, Switch, ToggleButton,
};
use std::cell::RefCell;

use std::rc::Rc;

pub fn create_settings_window(
    app: &Application,
    config_service: ConfigService,
    typography_service: TypographyService,
) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Settings")
        .default_width(850)
        .default_height(650)
        .resizable(true)
        .build();

    let main_box = GtkBox::new(Orientation::Horizontal, 0);

    let stack = Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let sidebar = gtk4::StackSidebar::builder()
        .stack(&stack)
        .width_request(200)
        .css_classes(vec!["background"])
        .build();

    let separator = gtk4::Separator::new(Orientation::Vertical);

    main_box.append(&sidebar);
    main_box.append(&separator);

    let keystroke_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    let keystroke_content =
        create_keystroke_settings(&config_service, &typography_service, &window);
    keystroke_scroll.set_child(Some(&keystroke_content));
    stack.add_titled(&keystroke_scroll, Some("keystroke"), "Keystrokes");

    let bubble_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    let bubble_content = create_bubble_settings(&config_service, &typography_service, &window);
    bubble_scroll.set_child(Some(&bubble_content));
    stack.add_titled(&bubble_scroll, Some("bubble"), "Bubbles");

    main_box.append(&stack);
    window.set_child(Some(&main_box));

    window
}

fn create_font_selector(
    parent: &GtkBox,
    typography_service: &TypographyService,
    config_service: &ConfigService,
    current_font: String,
    update_fn: impl Fn(&mut crate::domain::config::KeystrokeConfig, String) + 'static + Clone,
) {
    let type_section = GtkBox::new(Orientation::Vertical, 12);
    type_section.set_vexpand(true);

    let type_label = Label::builder()
        .label("TYPOGRAPHY")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    type_section.append(&type_label);

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search fonts...")
        .build();
    type_section.append(&search_entry);

    let font_scroll = ScrolledWindow::builder()
        .min_content_height(120)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .css_classes(vec!["font-list-frame"])
        .vexpand(false)
        .overflow(Overflow::Hidden)
        .build();

    let font_list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(vec!["navigation-sidebar"])
        .build();

    let service_c = typography_service.clone();
    let current_font_c = current_font.clone();
    let list_c = font_list.clone();
    let rows_store = Rc::new(RefCell::new(Vec::new()));
    let rows_store_c = rows_store.clone();

    glib::MainContext::default().spawn_local(async move {
        match service_c.get_system_fonts().await {
            Ok(fonts) => {
                let mut store = rows_store_c.borrow_mut();
                let mut current_row: Option<ListBoxRow> = None;
                for font_name in fonts.iter() {
                    let row_box = GtkBox::new(Orientation::Horizontal, 12);
                    row_box.set_margin_start(12);
                    row_box.set_margin_end(12);
                    row_box.set_margin_top(8);
                    row_box.set_margin_bottom(8);

                    let label = Label::builder()
                        .label(font_name)
                        .xalign(0.0)
                        .hexpand(true)
                        .build();

                    let icon = gtk4::Image::from_icon_name("object-select-symbolic");
                    if font_name != &current_font_c {
                        icon.set_opacity(0.0);
                    }

                    row_box.append(&label);
                    row_box.append(&icon);

                    let row = ListBoxRow::builder().child(&row_box).build();

                    unsafe {
                        row.set_data("font-name", font_name.clone());
                    }

                    if font_name == &current_font_c {
                        current_row = Some(row.clone());
                    }

                    list_c.append(&row);
                    store.push(row);
                }

                if let Some(row) = current_row {
                    let row_c = row.clone();
                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        row_c.grab_focus();
                        glib::ControlFlow::Break
                    });
                    list_c.select_row(Some(&row));
                }
            }
            Err(e) => eprintln!("Failed to load fonts: {}", e),
        }
    });

    let rows_store_c = rows_store.clone();
    search_entry.connect_search_changed(move |entry| {
        let text = entry.text().to_lowercase();
        if let Ok(store) = rows_store_c.try_borrow() {
            for row in store.iter() {
                let visible = if text.is_empty() {
                    true
                } else {
                    unsafe {
                        if let Some(ptr) = row.data::<String>("font-name") {
                            let name = ptr.as_ref();
                            name.to_lowercase().contains(&text)
                        } else {
                            true
                        }
                    }
                };
                row.set_visible(visible);
            }
        }
    });

    let service_c = config_service.clone();
    let rows_store_c = rows_store.clone();
    font_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            unsafe {
                if let Some(ptr) = row.data::<String>("font-name") {
                    let name = ptr.as_ref();
                    let mut cfg = service_c.get_config();
                    update_fn(&mut cfg, name.clone());
                    let _ = service_c.update_config(cfg);

                    if let Ok(store) = rows_store_c.try_borrow() {
                        for r in store.iter() {
                            if let Some(ptr_r) = r.data::<String>("font-name") {
                                let r_name = ptr_r.as_ref();
                                let opacity = if *r_name == *name { 1.0 } else { 0.0 };
                                if let Some(child) = r.child() {
                                    if let Some(box_) = child.downcast_ref::<GtkBox>() {
                                        if let Some(last) = box_.last_child() {
                                            if let Some(icon) = last.downcast_ref::<gtk4::Image>() {
                                                icon.set_opacity(opacity);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    font_scroll.set_child(Some(&font_list));
    type_section.append(&font_scroll);
    parent.append(&type_section);
}

fn create_keystroke_settings(
    config_service: &ConfigService,
    typography_service: &TypographyService,
    window: &ApplicationWindow,
) -> GtkBox {
    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.set_margin_start(32);
    content_box.set_margin_end(32);
    content_box.set_valign(Align::Start);
    content_box.set_halign(Align::Fill);

    let header_box = GtkBox::new(Orientation::Horizontal, 12);
    header_box.set_margin_top(16);
    header_box.set_margin_bottom(16);
    let label = Label::builder()
        .label("General")
        .css_classes(vec!["title-2"])
        .build();
    header_box.append(&label);
    content_box.append(&header_box);

    let config = config_service.get_config();

    let general_card = GtkBox::new(Orientation::Vertical, 0);
    general_card.add_css_class("settings-card");
    general_card.set_margin_bottom(24);

    let grid = Grid::builder()
        .column_spacing(48)
        .row_spacing(24)
        .hexpand(true)
        .build();

    general_card.append(&grid);
    content_box.append(&general_card);

    let left_col = GtkBox::new(Orientation::Vertical, 32);
    left_col.set_hexpand(false);
    left_col.set_width_request(250);

    let theme_section = GtkBox::new(Orientation::Vertical, 12);
    let theme_label = Label::builder()
        .label("THEME MODE")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    theme_section.append(&theme_label);

    let theme_box = GtkBox::new(Orientation::Horizontal, 0);
    theme_box.add_css_class("linked");
    theme_box.set_halign(Align::Start);

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

    theme_box.append(&theme_light);
    theme_box.append(&theme_dark);
    theme_box.append(&theme_sys);
    theme_section.append(&theme_box);
    left_col.append(&theme_section);

    let pos_section = GtkBox::new(Orientation::Vertical, 12);
    let pos_label = Label::builder()
        .label("POSITION")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    pos_section.append(&pos_label);

    let pos_grid = Grid::builder()
        .row_spacing(6)
        .column_spacing(6)
        .halign(Align::Start)
        .css_classes(vec!["position-grid"])
        .build();

    let positions = [
        (Position::TopLeft, 0, 0),
        (Position::TopCenter, 1, 0),
        (Position::TopRight, 2, 0),
        (Position::MiddleLeft, 0, 1),
        (Position::Center, 1, 1),
        (Position::MiddleRight, 2, 1),
        (Position::BottomLeft, 0, 2),
        (Position::BottomCenter, 1, 2),
        (Position::BottomRight, 2, 2),
    ];

    let mut first_btn: Option<ToggleButton> = None;

    for (pos, col, row) in positions {
        let btn = ToggleButton::builder()
            .width_request(42)
            .height_request(22)
            .css_classes(vec!["position-toggle"])
            .build();

        if pos == config.position {
            btn.set_active(true);
        }

        if let Some(ref first) = first_btn {
            btn.set_group(Some(first));
        } else {
            first_btn = Some(btn.clone());
        }

        let service_c = config_service.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() {
                let mut cfg = service_c.get_config();
                if cfg.position != pos {
                    cfg.position = pos;
                    let _ = service_c.update_config(cfg);
                }
            }
        });

        pos_grid.attach(&btn, col, row, 1, 1);
    }

    pos_section.append(&pos_grid);
    left_col.append(&pos_section);

    grid.attach(&left_col, 0, 0, 1, 1);

    let right_col = GtkBox::new(Orientation::Vertical, 32);
    right_col.set_hexpand(true);
    right_col.set_vexpand(true);
    right_col.set_width_request(350);

    create_font_selector(
        &right_col,
        typography_service,
        config_service,
        config.font_family.clone(),
        |cfg, name| cfg.font_family = name,
    );

    let size_section = GtkBox::new(Orientation::Vertical, 12);
    let size_header = GtkBox::new(Orientation::Horizontal, 0);

    let size_label = Label::builder()
        .label("SIZE")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .hexpand(true)
        .build();

    let val_label = Label::builder()
        .label(format!("{:.1}x", config.font_size))
        .css_classes(vec!["badge"])
        .build();

    size_header.append(&size_label);
    size_header.append(&val_label);
    size_section.append(&size_header);

    let size_adj = Adjustment::new(config.font_size, 0.5, 4.0, 0.1, 0.5, 0.0);
    let size_scale = Scale::builder()
        .adjustment(&size_adj)
        .draw_value(false)
        .build();

    let service_c = config_service.clone();
    let val_label_c = val_label.clone();
    size_adj.connect_value_changed(move |adj| {
        let val = adj.value();
        val_label_c.set_label(&format!("{:.1}x", val));

        let mut cfg = service_c.get_config();
        if (cfg.font_size - val).abs() > f64::EPSILON {
            cfg.font_size = val;
            let _ = service_c.update_config(cfg);
        }
    });

    size_section.append(&size_scale);
    right_col.append(&size_section);

    grid.attach(&right_col, 1, 0, 1, 1);

    let behavior_label = Label::builder()
        .label("Behavior")
        .css_classes(vec!["title-2"])
        .halign(Align::Start)
        .margin_top(0)
        .margin_bottom(16)
        .build();
    content_box.append(&behavior_label);

    let behavior_card = GtkBox::new(Orientation::Vertical, 0);
    behavior_card.add_css_class("settings-card");
    behavior_card.set_margin_bottom(24);

    let behavior_grid = Grid::builder()
        .column_spacing(48)
        .row_spacing(24)
        .hexpand(true)
        .build();

    behavior_card.append(&behavior_grid);
    content_box.append(&behavior_card);

    let b_left = GtkBox::new(Orientation::Vertical, 32);
    b_left.set_hexpand(true);

    let dur_box = GtkBox::new(Orientation::Vertical, 12);
    let dur_header = GtkBox::new(Orientation::Horizontal, 0);
    let dur_label = Label::builder()
        .label("DISPLAY DURATION")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .hexpand(true)
        .build();

    let dur_val_label = Label::builder()
        .label(format!(
            "{:.1}s",
            config.display_timeout_ms as f64 / 1000.0
        ))
        .css_classes(vec!["badge"])
        .build();

    dur_header.append(&dur_label);
    dur_header.append(&dur_val_label);
    dur_box.append(&dur_header);

    let dur_adj = Adjustment::new(
        config.display_timeout_ms as f64 / 1000.0,
        0.5,
        10.0,
        0.5,
        1.0,
        0.0,
    );
    let dur_scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&dur_adj)
        .draw_value(false)
        .build();
    dur_box.append(&dur_scale);
    b_left.append(&dur_box);

    let max_box = GtkBox::new(Orientation::Horizontal, 12);
    max_box.set_valign(Align::Center);
    let max_label = Label::builder()
        .label("Max Visible Keys")
        .css_classes(vec!["body"])
        .hexpand(true)
        .xalign(0.0)
        .build();
    max_box.append(&max_label);

    let stepper_box = GtkBox::new(Orientation::Horizontal, 0);
    stepper_box.add_css_class("linked");

    let step_minus = Button::builder().label("-").width_request(32).build();

    let step_display = Button::builder()
        .label(config.max_keys.to_string())
        .width_request(40)
        .can_focus(false)
        .build();

    let step_plus = Button::builder().label("+").width_request(32).build();

    stepper_box.append(&step_minus);
    stepper_box.append(&step_display);
    stepper_box.append(&step_plus);
    max_box.append(&stepper_box);
    b_left.append(&max_box);

    let drag_box = GtkBox::new(Orientation::Horizontal, 12);
    let drag_text = GtkBox::new(Orientation::Vertical, 2);
    drag_text.set_hexpand(true);
    let drag_lbl = Label::builder()
        .label("Draggable Overlay")
        .xalign(0.0)
        .css_classes(vec!["body"])
        .build();
    let drag_sub = Label::builder()
        .label("Move visualizer with mouse")
        .xalign(0.0)
        .css_classes(vec!["caption", "dim-label"])
        .build();
    drag_text.append(&drag_lbl);
    drag_text.append(&drag_sub);
    drag_box.append(&drag_text);

    let drag_switch = Switch::builder()
        .active(config.keystroke_draggable)
        .valign(Align::Center)
        .build();
    drag_box.append(&drag_switch);
    b_left.append(&drag_box);

    behavior_grid.attach(&b_left, 0, 0, 1, 1);

    let b_right = GtkBox::new(Orientation::Vertical, 0);
    b_right.set_width_request(350);

    let card_content = GtkBox::new(Orientation::Vertical, 20);

    let hk_title = Label::builder()
        .label("GLOBAL HOTKEYS")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    card_content.append(&hk_title);

    let hk1_box = GtkBox::new(Orientation::Horizontal, 12);
    hk1_box.add_css_class("settings-element");
    hk1_box.set_margin_bottom(8);
    let hk1_lbl = Label::builder()
        .label("Toggle Activation")
        .hexpand(true)
        .xalign(0.0)
        .build();
    let hk1_btn = Button::builder()
        .label(format_hotkey(&config.keystroke_hotkey))
        .valign(Align::Center)
        .build();
    hk1_box.append(&hk1_lbl);
    hk1_box.append(&hk1_btn);
    card_content.append(&hk1_box);

    let hk2_box = GtkBox::new(Orientation::Horizontal, 12);
    hk2_box.add_css_class("settings-element");
    let hk2_lbl = Label::builder()
        .label("Pause/Resume")
        .hexpand(true)
        .xalign(0.0)
        .build();
    let hk2_btn = Button::builder()
        .label(format_hotkey(&config.pause_hotkey))
        .valign(Align::Center)
        .build();
    hk2_box.append(&hk2_lbl);
    hk2_box.append(&hk2_btn);
    card_content.append(&hk2_box);

    b_right.append(&card_content);
    behavior_grid.attach(&b_right, 1, 0, 1, 1);

    let service_c = config_service.clone();
    let dur_val_c = dur_val_label.clone();
    dur_adj.connect_value_changed(move |adj| {
        let val = adj.value();
        dur_val_c.set_label(&format!("{:.1}s", val));

        let ms = (val * 1000.0) as u64;
        let mut cfg = service_c.get_config();
        if cfg.display_timeout_ms != ms {
            cfg.display_timeout_ms = ms;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_minus = config_service.clone();
    let display_minus = step_display.clone();
    step_minus.connect_clicked(move |_| {
        let mut cfg = service_minus.get_config();
        if cfg.max_keys > 1 {
            cfg.max_keys -= 1;
            display_minus.set_label(&cfg.max_keys.to_string());
            let _ = service_minus.update_config(cfg);
        }
    });

    let service_plus = config_service.clone();
    let display_plus = step_display.clone();
    step_plus.connect_clicked(move |_| {
        let mut cfg = service_plus.get_config();
        if cfg.max_keys < 20 {
            cfg.max_keys += 1;
            display_plus.set_label(&cfg.max_keys.to_string());
            let _ = service_plus.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    drag_switch.connect_state_set(move |_, state| {
        let mut cfg = service_c.get_config();
        if cfg.keystroke_draggable != state {
            cfg.keystroke_draggable = state;
            let _ = service_c.update_config(cfg);
        }
        glib::Propagation::Proceed
    });

    setup_hotkey_capture(
        window,
        &hk1_btn,
        config_service.clone(),
        HotkeyType::KeystrokeActivation,
    );

    setup_hotkey_capture(window, &hk2_btn, config_service.clone(), HotkeyType::Pause);

    let service_c = config_service.clone();
    create_audio_group(&content_box, config.audio.clone(), move |modifier| {
        let mut cfg = service_c.get_config();
        modifier(&mut cfg.audio);
        let _ = service_c.update_config(cfg);
    });

    content_box
}

fn create_bubble_settings(
    config_service: &ConfigService,
    typography_service: &TypographyService,
    window: &ApplicationWindow,
) -> GtkBox {
    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.set_margin_start(32);
    content_box.set_margin_end(32);
    content_box.set_valign(Align::Start);
    content_box.set_halign(Align::Fill);

    let header_box = GtkBox::new(Orientation::Horizontal, 12);
    header_box.set_margin_top(16);
    header_box.set_margin_bottom(16);
    let label = Label::builder()
        .label("General")
        .css_classes(vec!["title-2"])
        .build();
    header_box.append(&label);
    content_box.append(&header_box);

    let config = config_service.get_config();

    let general_card = GtkBox::new(Orientation::Vertical, 0);
    general_card.add_css_class("settings-card");
    general_card.set_margin_bottom(24);

    let grid = Grid::builder()
        .column_spacing(48)
        .row_spacing(24)
        .hexpand(true)
        .build();

    general_card.append(&grid);
    content_box.append(&general_card);

    let left_col = GtkBox::new(Orientation::Vertical, 32);
    left_col.set_hexpand(false);
    left_col.set_width_request(250);

    let pos_section = GtkBox::new(Orientation::Vertical, 12);
    let pos_label = Label::builder()
        .label("POSITION")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    pos_section.append(&pos_label);

    let pos_grid = Grid::builder()
        .row_spacing(6)
        .column_spacing(6)
        .halign(Align::Start)
        .css_classes(vec!["position-grid"])
        .build();

    let positions = [
        (Position::TopLeft, 0, 0),
        (Position::TopCenter, 1, 0),
        (Position::TopRight, 2, 0),
        (Position::MiddleLeft, 0, 1),
        (Position::Center, 1, 1),
        (Position::MiddleRight, 2, 1),
        (Position::BottomLeft, 0, 2),
        (Position::BottomCenter, 1, 2),
        (Position::BottomRight, 2, 2),
    ];

    let mut first_btn: Option<ToggleButton> = None;

    for (pos, col, row) in positions {
        let btn = ToggleButton::builder()
            .width_request(42)
            .height_request(22)
            .css_classes(vec!["position-toggle"])
            .build();

        if pos == config.bubble.position {
            btn.set_active(true);
        }

        if let Some(ref first) = first_btn {
            btn.set_group(Some(first));
        } else {
            first_btn = Some(btn.clone());
        }

        let service_c = config_service.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() {
                let mut cfg = service_c.get_config();
                if cfg.bubble.position != pos {
                    cfg.bubble.position = pos;
                    let _ = service_c.update_config(cfg);
                }
            }
        });

        pos_grid.attach(&btn, col, row, 1, 1);
    }

    pos_section.append(&pos_grid);
    left_col.append(&pos_section);

    grid.attach(&left_col, 0, 0, 1, 1);

    let right_col = GtkBox::new(Orientation::Vertical, 32);
    right_col.set_hexpand(true);
    right_col.set_vexpand(true);
    right_col.set_width_request(350);

    create_font_selector(
        &right_col,
        typography_service,
        config_service,
        config.bubble.font_family.clone(),
        |cfg, name| cfg.bubble.font_family = name,
    );

    let size_section = GtkBox::new(Orientation::Vertical, 12);
    let size_header = GtkBox::new(Orientation::Horizontal, 0);

    let size_label = Label::builder()
        .label("SIZE")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .hexpand(true)
        .build();

    let val_label = Label::builder()
        .label(format!("{:.1}x", config.bubble.font_size))
        .css_classes(vec!["badge"])
        .build();

    size_header.append(&size_label);
    size_header.append(&val_label);
    size_section.append(&size_header);

    let size_adj = Adjustment::new(config.bubble.font_size, 0.5, 4.0, 0.1, 0.5, 0.0);
    let size_scale = Scale::builder()
        .adjustment(&size_adj)
        .draw_value(false)
        .build();

    let service_c = config_service.clone();
    let val_label_c = val_label.clone();
    size_adj.connect_value_changed(move |adj| {
        let val = adj.value();
        val_label_c.set_label(&format!("{:.1}x", val));

        let mut cfg = service_c.get_config();
        if (cfg.bubble.font_size - val).abs() > f64::EPSILON {
            cfg.bubble.font_size = val;
            let _ = service_c.update_config(cfg);
        }
    });

    size_section.append(&size_scale);
    right_col.append(&size_section);

    grid.attach(&right_col, 1, 0, 1, 1);

    let behavior_label = Label::builder()
        .label("Behavior")
        .css_classes(vec!["title-2"])
        .halign(Align::Start)
        .margin_top(0)
        .margin_bottom(16)
        .build();
    content_box.append(&behavior_label);

    let behavior_card = GtkBox::new(Orientation::Vertical, 0);
    behavior_card.add_css_class("settings-card");
    behavior_card.set_margin_bottom(24);

    let behavior_grid = Grid::builder()
        .column_spacing(48)
        .row_spacing(24)
        .hexpand(true)
        .build();

    behavior_card.append(&behavior_grid);
    content_box.append(&behavior_card);

    let b_left = GtkBox::new(Orientation::Vertical, 32);
    b_left.set_hexpand(true);

    let dur_box = GtkBox::new(Orientation::Vertical, 12);
    let dur_header = GtkBox::new(Orientation::Horizontal, 0);
    let dur_label = Label::builder()
        .label("DISPLAY DURATION")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .hexpand(true)
        .build();

    let dur_val_label = Label::builder()
        .label(format!("{:.1}s", config.bubble.timeout_ms as f64 / 1000.0))
        .css_classes(vec!["badge"])
        .build();

    dur_header.append(&dur_label);
    dur_header.append(&dur_val_label);
    dur_box.append(&dur_header);

    let dur_adj = Adjustment::new(
        config.bubble.timeout_ms as f64 / 1000.0,
        0.5,
        10.0,
        0.5,
        1.0,
        0.0,
    );
    let dur_scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&dur_adj)
        .draw_value(false)
        .build();
    dur_box.append(&dur_scale);
    b_left.append(&dur_box);

    let drag_box = GtkBox::new(Orientation::Horizontal, 12);
    let drag_text = GtkBox::new(Orientation::Vertical, 2);
    drag_text.set_hexpand(true);
    let drag_lbl = Label::builder()
        .label("Draggable Overlay")
        .xalign(0.0)
        .css_classes(vec!["body"])
        .build();
    let drag_sub = Label::builder()
        .label("Move visualizer with mouse")
        .xalign(0.0)
        .css_classes(vec!["caption", "dim-label"])
        .build();
    drag_text.append(&drag_lbl);
    drag_text.append(&drag_sub);
    drag_box.append(&drag_text);

    let drag_switch = Switch::builder()
        .active(config.bubble.draggable)
        .valign(Align::Center)
        .build();
    drag_box.append(&drag_switch);
    b_left.append(&drag_box);

    behavior_grid.attach(&b_left, 0, 0, 1, 1);

    let b_right = GtkBox::new(Orientation::Vertical, 0);
    b_right.set_width_request(350);

    let card_content = GtkBox::new(Orientation::Vertical, 20);

    let hk_title = Label::builder()
        .label("GLOBAL HOTKEYS")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    card_content.append(&hk_title);

    let hk1_box = GtkBox::new(Orientation::Horizontal, 12);
    hk1_box.add_css_class("settings-element");
    hk1_box.set_margin_bottom(8);
    let hk1_lbl = Label::builder()
        .label("Activation Hotkey")
        .hexpand(true)
        .xalign(0.0)
        .build();
    let hk1_btn = Button::builder()
        .label(format_hotkey(&config.bubble.hotkey))
        .valign(Align::Center)
        .build();
    hk1_box.append(&hk1_lbl);
    hk1_box.append(&hk1_btn);
    card_content.append(&hk1_box);

    let hk2_box = GtkBox::new(Orientation::Horizontal, 12);
    hk2_box.add_css_class("settings-element");
    let hk2_lbl = Label::builder()
        .label("Focus Hotkey")
        .hexpand(true)
        .xalign(0.0)
        .build();
    let hk2_btn = Button::builder()
        .label(format_hotkey(&config.toggle_focus_hotkey))
        .valign(Align::Center)
        .build();
    hk2_box.append(&hk2_lbl);
    hk2_box.append(&hk2_btn);
    card_content.append(&hk2_box);

    b_right.append(&card_content);
    behavior_grid.attach(&b_right, 1, 0, 1, 1);

    let service_c = config_service.clone();
    let dur_val_c = dur_val_label.clone();
    dur_adj.connect_value_changed(move |adj| {
        let val = adj.value();
        dur_val_c.set_label(&format!("{:.1}s", val));

        let ms = (val * 1000.0) as u64;
        let mut cfg = service_c.get_config();
        if cfg.bubble.timeout_ms != ms {
            cfg.bubble.timeout_ms = ms;
            let _ = service_c.update_config(cfg);
        }
    });

    let service_c = config_service.clone();
    drag_switch.connect_state_set(move |_, state| {
        let mut cfg = service_c.get_config();
        if cfg.bubble.draggable != state {
            cfg.bubble.draggable = state;
            let _ = service_c.update_config(cfg);
        }
        glib::Propagation::Proceed
    });

    setup_hotkey_capture(
        window,
        &hk1_btn,
        config_service.clone(),
        HotkeyType::BubbleActivation,
    );

    setup_hotkey_capture(
        window,
        &hk2_btn,
        config_service.clone(),
        HotkeyType::ToggleFocus,
    );

    let service_c = config_service.clone();
    create_audio_group(&content_box, config.bubble.audio.clone(), move |modifier| {
        let mut cfg = service_c.get_config();
        modifier(&mut cfg.bubble.audio);
        let _ = service_c.update_config(cfg);
    });

    content_box
}

pub fn show_settings(window: &ApplicationWindow) {
    window.present();
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
                btn_c.set_label(&format_hotkey(&accelerator));
                return glib::Propagation::Stop;
            }

            let accelerator = gtk4::accelerator_name(keyval, state);
            if !accelerator.is_empty() {
                let mut cfg = service_c.get_config();
                match hotkey_type {
                    HotkeyType::KeystrokeActivation => {
                        cfg.keystroke_hotkey = accelerator.to_string()
                    }
                    HotkeyType::BubbleActivation => cfg.bubble.hotkey = accelerator.to_string(),
                    HotkeyType::Pause => cfg.pause_hotkey = accelerator.to_string(),
                    HotkeyType::ToggleFocus => cfg.toggle_focus_hotkey = accelerator.to_string(),
                }

                let _ = service_c.update_config(cfg);

                btn_c.set_label(&format_hotkey(&accelerator));
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

fn create_audio_group<F>(parent: &GtkBox, config: AudioConfig, update_fn: F)
where
    F: Fn(Box<dyn Fn(&mut AudioConfig)>) + 'static + Clone,
{
    let section_card = GtkBox::new(Orientation::Vertical, 0);
    section_card.add_css_class("settings-card");
    section_card.set_margin_bottom(24);

    let audio_label = Label::builder()
        .label("Key Switches")
        .css_classes(vec!["title-2"])
        .halign(Align::Start)
        .margin_top(24)
        .margin_bottom(16)
        .build();
    parent.append(&audio_label);

    let enable_box = GtkBox::new(Orientation::Horizontal, 12);
    enable_box.set_margin_bottom(24);

    let enable_lbl = Label::builder()
        .label("Enable Sounds")
        .hexpand(true)
        .xalign(0.0)
        .css_classes(vec!["title-4"])
        .build();

    let enabled_switch = Switch::builder()
        .active(config.enabled)
        .valign(Align::Center)
        .build();

    enable_box.append(&enable_lbl);
    enable_box.append(&enabled_switch);
    section_card.append(&enable_box);

    let update_c = update_fn.clone();
    enabled_switch.connect_state_set(move |_, state| {
        update_c(Box::new(move |cfg| cfg.enabled = state));
        glib::Propagation::Proceed
    });

    let grid = Grid::builder()
        .column_spacing(24)
        .row_spacing(24)
        .hexpand(true)
        .build();

    let left_col = GtkBox::new(Orientation::Vertical, 12);
    left_col.set_hexpand(true);

    let pack_label = Label::builder()
        .label("SOUND PACK")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .build();
    left_col.append(&pack_label);

    let search_entry = SearchEntry::builder()
        .placeholder_text("Find switch sound...")
        .build();
    left_col.append(&search_entry);

    let pack_scroll = ScrolledWindow::builder()
        .min_content_height(120)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(false)
        .css_classes(vec!["font-list-frame"])
        .overflow(Overflow::Hidden)
        .build();

    let pack_list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(vec!["navigation-sidebar"])
        .build();
    let pack_list_c = pack_list.clone();

    let current_pack = config.sound_pack.clone();
    let rows_store = Rc::new(RefCell::new(Vec::<ListBoxRow>::new()));
    let rows_store_c = rows_store.clone();

    let packs_future = async move { SoundPackLoader::list_available_packs() };

    glib::MainContext::default().spawn_local(async move {
        let packs = packs_future.await;
        let mut store = rows_store_c.borrow_mut();
        let mut current_row: Option<ListBoxRow> = None;

        for (pack_id, pack_name) in packs.iter() {
            let row_box = GtkBox::new(Orientation::Horizontal, 12);
            row_box.set_margin_start(12);
            row_box.set_margin_end(12);
            row_box.set_margin_top(8);
            row_box.set_margin_bottom(8);

            let label = Label::builder()
                .label(pack_name)
                .xalign(0.0)
                .hexpand(true)
                .build();

            let icon = gtk4::Image::from_icon_name("object-select-symbolic");
            if *pack_id != current_pack {
                icon.set_opacity(0.0);
            }

            row_box.append(&label);
            row_box.append(&icon);

            let row = ListBoxRow::builder().child(&row_box).build();

            unsafe {
                row.set_data("pack-id", pack_id.clone());
                row.set_data("pack-name", pack_name.clone());
            }

            if *pack_id == current_pack {
                current_row = Some(row.clone());
            }

            pack_list_c.append(&row);
            store.push(row);
        }

        if let Some(row) = current_row {
            let row_c = row.clone();
            glib::idle_add_local(move || {
                row_c.grab_focus();
                glib::ControlFlow::Break
            });
            pack_list_c.select_row(Some(&row));
        }
    });

    let rows_store_c = rows_store.clone();
    search_entry.connect_search_changed(move |entry| {
        let text = entry.text().to_lowercase();
        if let Ok(store) = rows_store_c.try_borrow() {
            for row in store.iter() {
                let visible = if text.is_empty() {
                    true
                } else {
                    unsafe {
                        if let Some(ptr) = row.data::<String>("pack-name") {
                            let name = ptr.as_ref();
                            name.to_lowercase().contains(&text)
                        } else {
                            true
                        }
                    }
                };
                row.set_visible(visible);
            }
        }
    });

    let update_c = update_fn.clone();
    let rows_store_c = rows_store.clone();
    pack_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            unsafe {
                if let Some(ptr) = row.data::<String>("pack-id") {
                    let id = ptr.as_ref();
                    let id_clone = id.clone();
                    update_c(Box::new(move |cfg| cfg.sound_pack = id_clone.clone()));

                    if let Ok(store) = rows_store_c.try_borrow() {
                        for r in store.iter() {
                            if let Some(ptr_r) = r.data::<String>("pack-id") {
                                let r_id = ptr_r.as_ref();
                                let opacity = if *r_id == *id { 1.0 } else { 0.0 };
                                if let Some(child) = r.child() {
                                    if let Some(box_) = child.downcast_ref::<GtkBox>() {
                                        if let Some(last) = box_.last_child() {
                                            if let Some(icon) = last.downcast_ref::<gtk4::Image>() {
                                                icon.set_opacity(opacity);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    pack_scroll.set_child(Some(&pack_list));
    left_col.append(&pack_scroll);
    grid.attach(&left_col, 0, 0, 1, 1);

    let right_col = GtkBox::new(Orientation::Vertical, 0);
    right_col.set_width_request(350);
    right_col.set_valign(Align::End);

    let vol_card = GtkBox::new(Orientation::Vertical, 16);
    vol_card.add_css_class("settings-element");

    let vol_header = GtkBox::new(Orientation::Horizontal, 0);
    let vol_label = Label::builder()
        .label("VOLUME")
        .css_classes(vec!["heading", "caption"])
        .halign(Align::Start)
        .hexpand(true)
        .build();

    let vol_val_label = Label::builder()
        .label(format!("{:.0}%", config.volume * 100.0))
        .css_classes(vec!["badge"])
        .build();

    vol_header.append(&vol_label);
    vol_header.append(&vol_val_label);
    vol_card.append(&vol_header);

    let vol_slider_box = GtkBox::new(Orientation::Horizontal, 12);
    let vol_icon_low = gtk4::Image::from_icon_name("audio-volume-low-symbolic");
    let vol_icon_high = gtk4::Image::from_icon_name("audio-volume-high-symbolic");

    let vol_adj = Adjustment::new(config.volume as f64 * 100.0, 0.0, 100.0, 1.0, 10.0, 0.0);
    let vol_scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&vol_adj)
        .draw_value(false)
        .hexpand(true)
        .build();

    vol_slider_box.append(&vol_icon_low);
    vol_slider_box.append(&vol_scale);
    vol_slider_box.append(&vol_icon_high);
    vol_card.append(&vol_slider_box);

    let update_c = update_fn.clone();
    let vol_val_c = vol_val_label.clone();
    vol_adj.connect_value_changed(move |adj| {
        let val = adj.value();
        vol_val_c.set_label(&format!("{:.0}%", val));
        update_c(Box::new(move |cfg| cfg.volume = (val / 100.0) as f32));
    });

    right_col.append(&vol_card);
    grid.attach(&right_col, 1, 0, 1, 1);

    section_card.append(&grid);
    parent.append(&section_card);
}

fn format_hotkey(accelerator: &str) -> String {
    if accelerator.is_empty() {
        return "None".to_string();
    }
    if let Some((key, mods)) = gtk4::accelerator_parse(accelerator) {
        let label = gtk4::accelerator_get_label(key, mods);
        label.replace("+", " + ")
    } else {
        accelerator.to_string()
    }
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
