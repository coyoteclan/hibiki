use crate::application::config_service::ConfigService;
use crate::application::feedback_service::FeedbackService;
use crate::application::routing::{RoutingEngine, RoutingResult};
use crate::compositor::LayoutEvent;
use crate::domain::CaptureState;
use crate::input::{
    InputControlCommand, KeyEvent, KeyListener, LayoutManager, ListenerConfig, ListenerHandle,
};
use crate::tray::{TrayAction, TrayHandle};
use crate::ui::{
    create_bubble_window, create_launcher_window, create_settings_window, create_window,
    setup_drag, show_launcher, show_settings, BubbleDisplayWidget, DisplayMode, KeyDisplayWidget,
};
use anyhow::Result;
use async_channel::{bounded, Receiver};
use gtk4::glib::{self, ControlFlow};
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub struct App {
    gtk_app: Application,
    config_service: ConfigService,
}

#[derive(Default)]
struct RuntimeState {
    mode: Option<DisplayMode>,
    routing_engine: RoutingEngine,
    keystroke_window: Option<ApplicationWindow>,
    bubble_window: Option<ApplicationWindow>,
    launcher_window: Option<ApplicationWindow>,
    settings_window: Option<ApplicationWindow>,
    mode_active: Option<Arc<AtomicBool>>,
    listener_handle: Option<ListenerHandle>,
    layout_manager: Option<LayoutManager>,
    feedback_service: Option<FeedbackService>,
    drag_controllers: Vec<gtk4::EventController>,
}

impl App {
    #[must_use]
    pub fn new(config_service: ConfigService) -> Self {
        let gtk_app = Application::builder()
            .application_id("dev.linuxmobile.keystroke")
            .build();

        Self { gtk_app, config_service }
    }

    #[must_use]
    pub fn run_with_tray(self, tray_rx: Receiver<TrayAction>, tray_handle: TrayHandle) -> i32 {
        let config_service = self.config_service.clone();

        self.gtk_app.connect_activate(move |app| {
            activate(app, config_service.clone(), tray_rx.clone(), tray_handle.clone());
        });

        let exit_code = self.gtk_app.run_with_args::<&str>(&[]);

        exit_code.into()
    }

    #[must_use]
    pub fn run(self) -> i32 {
        let config_service = self.config_service.clone();

        self.gtk_app.connect_activate(move |app| {
            activate_without_tray(app, config_service.clone());
        });

        let exit_code = self.gtk_app.run_with_args::<&str>(&[]);

        exit_code.into()
    }
}

fn activate_without_tray(app: &Application, config_service: ConfigService) {
    info!("Activating keystroke application (no tray)");

    let state = Rc::new(RefCell::new(RuntimeState::default()));
    state.borrow_mut().feedback_service = Some(FeedbackService::new(app));
    
    let config = config_service.get_config();
    state.borrow_mut().routing_engine.update_config(
        &config.keystroke_hotkey, 
        &config.bubble_hotkey
    );

    load_css_defaults();

    setup_launcher_and_modes(app, &state, config_service);
}

fn activate(
    app: &Application,
    config_service: ConfigService,
    tray_rx: Receiver<TrayAction>,
    tray_handle: TrayHandle,
) {
    info!("Activating keystroke application");

    let state = Rc::new(RefCell::new(RuntimeState::default()));
    state.borrow_mut().feedback_service = Some(FeedbackService::new(app));
    
    let config = config_service.get_config();
    state.borrow_mut().routing_engine.update_config(
        &config.keystroke_hotkey, 
        &config.bubble_hotkey
    );

    load_css_defaults();

    setup_launcher_and_modes(app, &state, config_service.clone());

    setup_tray_handling(
        Rc::clone(&state),
        config_service,
        app.clone(),
        tray_rx,
        tray_handle,
    );
}

fn setup_launcher_and_modes(
    app: &Application,
    state: &Rc<RefCell<RuntimeState>>,
    config_service: ConfigService,
) {
    let app_for_mode = app.clone();
    let state_for_mode = Rc::clone(state);
    let service_for_mode = config_service.clone();
    
    let on_mode_select = move |mode| {
        debug!("Mode selected: {:?}", mode);
        switch_mode(&app_for_mode, &state_for_mode, &service_for_mode, mode);
    };

    let app_for_settings = app.clone();
    let state_for_settings = Rc::clone(state);
    let service_for_settings = config_service.clone();
    
    let on_settings = move || {
        debug!("Opening settings from launcher");
        open_settings(&app_for_settings, &state_for_settings, service_for_settings.clone());
    };

    let launcher = create_launcher_window(app, on_mode_select, on_settings);

    state.borrow_mut().launcher_window = Some(launcher.clone());

    show_launcher(&launcher);
}

fn switch_mode(
    app: &Application,
    state: &Rc<RefCell<RuntimeState>>,
    config_service: &ConfigService,
    mode: DisplayMode,
) {
    close_mode_windows(state);

    state.borrow_mut().mode = Some(mode);

    match mode {
        DisplayMode::Keystroke => {
            if let Err(e) = start_keystroke_mode(app, config_service, Rc::clone(state)) {
                error!("Failed to start keystroke mode: {}", e);
            }
        }
        DisplayMode::Bubble => {
            if let Err(e) = start_bubble_mode(app, config_service, Rc::clone(state)) {
                error!("Failed to start bubble mode: {}", e);
            }
        }
    }
}

fn close_mode_windows(state: &Rc<RefCell<RuntimeState>>) {
    let mut s = state.borrow_mut();

    s.routing_engine.reset_hotkey_state();

    if let Some(active) = s.mode_active.take() {
        active.store(false, Ordering::SeqCst);
    }

    s.listener_handle = None;
    s.layout_manager = None;
    
    if let Some(window) = &s.keystroke_window {
        for controller in &s.drag_controllers {
            window.remove_controller(controller);
        }
    }
    s.drag_controllers.clear();

    if let Some(window) = s.keystroke_window.take() {
        window.close();
    }
    if let Some(window) = s.bubble_window.take() {
        window.close();
    }
}

fn setup_tray_handling(
    state: Rc<RefCell<RuntimeState>>,
    config_service: ConfigService,
    app: Application,
    tray_rx: Receiver<TrayAction>,
    tray_handle: TrayHandle,
) {
    let tray_handle = Rc::new(tray_handle);

    glib::MainContext::default().spawn_local(async move {
        while let Ok(action) = tray_rx.recv().await {
            handle_tray_action(&action, &state, &config_service, &app, &tray_handle);
        }
        debug!("Tray event loop terminated");
    });
}

fn handle_tray_action(
    action: &TrayAction,
    state: &Rc<RefCell<RuntimeState>>,
    config_service: &ConfigService,
    app: &Application,
    tray_handle: &Rc<TrayHandle>,
) {
    match action {
        TrayAction::ShowLauncher => {
            debug!("Handling ShowLauncher action");
            if let Some(launcher) = &state.borrow().launcher_window {
                show_launcher(launcher);
            }
        }
        TrayAction::KeystrokeMode => {
            debug!("Handling KeystrokeMode action");
            switch_mode(app, state, config_service, DisplayMode::Keystroke);
        }
        TrayAction::BubbleMode => {
            debug!("Handling BubbleMode action");
            switch_mode(app, state, config_service, DisplayMode::Bubble);
        }
        TrayAction::OpenSettings => {
            debug!("Handling OpenSettings action");
            open_settings(app, state, config_service.clone());
        }
        TrayAction::TogglePause => {
             debug!("Handling TogglePause action");
            let mut s = state.borrow_mut();
            let new_state = s.routing_engine.toggle_capture();
            let paused = new_state == CaptureState::Paused;

            if let Some(w) = &s.keystroke_window {
                if paused {
                    w.add_css_class("paused");
                } else {
                    w.remove_css_class("paused");
                }
            }
            if let Some(w) = &s.bubble_window {
                if paused {
                    w.add_css_class("paused");
                } else {
                    w.remove_css_class("paused");
                }
            }
            if let Some(service) = &s.feedback_service {
                service.handle_state_change(new_state, s.routing_engine.get_states().1);
            }
            drop(s);
            tray_handle.set_paused(paused);
            info!("Keystroke capture {}", if paused { "paused" } else { "resumed" });
        }
        TrayAction::Quit => {
            debug!("Handling Quit action");
            app.quit();
        }
    }
}

fn open_settings(
    app: &Application,
    state: &Rc<RefCell<RuntimeState>>,
    config_service: ConfigService,
) {
    if let Some(ref settings_window) = state.borrow().settings_window {
        show_settings(settings_window);
        return;
    }

    let settings_window = create_settings_window(app, config_service);

    state.borrow_mut().settings_window = Some(settings_window.clone());

    let state_clone = Rc::clone(state);
    settings_window.connect_close_request(move |_| {
        state_clone.borrow_mut().settings_window = None;
        glib::Propagation::Proceed
    });

    show_settings(&settings_window);
}

fn start_keystroke_mode(
    app: &Application,
    config_service: &ConfigService,
    state: Rc<RefCell<RuntimeState>>,
) -> Result<()> {
    info!("Starting keystroke mode");

    let config = config_service.get_config();
    
    {
        let mut s = state.borrow_mut();
        s.routing_engine.reset_mode_state();
    }

    let window = create_window(app, &config)?;

    if config.keystroke_draggable {
        let controllers = setup_drag(&window);
        state.borrow_mut().drag_controllers = controllers;
    }

    let display = Rc::new(RefCell::new(KeyDisplayWidget::new(
        config.max_keys,
        config.display_timeout_ms,
    )));

    window.set_child(Some(display.borrow().widget()));

    let (sender, receiver) = bounded::<KeyEvent>(1024);

    let listener_config = ListenerConfig {
        all_keyboards: config.all_keyboards,
        ..Default::default()
    };

    let listener = KeyListener::new(sender, listener_config);
    let handle = listener.start()?;

    let mode_active = Arc::new(AtomicBool::new(true));
    {
        let mut s = state.borrow_mut();
        s.mode_active = Some(Arc::clone(&mode_active));
        s.listener_handle = Some(handle);
        s.keystroke_window = Some(window.clone());
    }

    let active_key_loop = Arc::clone(&mode_active);
    let state_clone = Rc::clone(&state);
    let display_clone = Rc::clone(&display);
    
    let app_clone = app.clone();
    let config_service_clone = config_service.clone();

    glib::MainContext::default().spawn_local(async move {
        while let Ok(event) = receiver.recv().await {
            if !active_key_loop.load(Ordering::SeqCst) {
                break;
            }

             let routing_result = match &event {
                KeyEvent::Pressed(kd) => state_clone
                    .borrow_mut()
                    .routing_engine
                    .process(kd.key, true, false, DisplayMode::Keystroke),
                KeyEvent::Released(kd) => state_clone
                    .borrow_mut()
                    .routing_engine
                    .process(kd.key, false, false, DisplayMode::Keystroke),
                KeyEvent::AllReleased => RoutingResult::Dispatch(evdev::Key::KEY_RESERVED, false),
            };

            match routing_result {
                RoutingResult::Ignored => continue,
                RoutingResult::StateChanged(capture, focus) => {
                    if let Some(service) = &state_clone.borrow().feedback_service {
                        service.handle_state_change(capture, focus);
                    }
                    continue;
                }
                RoutingResult::SwitchMode(mode) => {
                    let app = app_clone.clone();
                    let state = state_clone.clone();
                    let service = config_service_clone.clone();
                    
                    glib::MainContext::default().spawn_local(async move {
                        switch_mode(&app, &state, &service, mode);
                    });
                    break;
                }
                RoutingResult::Dispatch(_, _) => {
                    let mut display = display_clone.borrow_mut();
                    match event {
                        KeyEvent::Pressed(key) => display.add_key(key),
                        KeyEvent::Released(key) => display.remove_key(&key),
                        KeyEvent::AllReleased => display.clear(),
                    }
                }
            }
        }
        debug!("Keystroke event loop terminated");
    });

    let mut rx = config_service.subscribe();
    let display_c = display.clone();
    let window_c = window.clone();
    let state_c = state.clone();
    let active_c = Arc::clone(&mode_active);

    glib::MainContext::default().spawn_local(async move {
        while rx.changed().await.is_ok() {
            if !active_c.load(Ordering::SeqCst) {
                 break;
            }
            let cfg = rx.borrow().clone();
            
            state_c.borrow_mut().routing_engine.update_config(
                &cfg.keystroke_hotkey, 
                &cfg.bubble_hotkey
            );

            {
                let mut disp = display_c.borrow_mut();
                disp.set_display_timeout(cfg.display_timeout_ms);
                disp.set_max_keys(cfg.max_keys);
            }

            {
                let mut s = state_c.borrow_mut();
                let current_draggable = !s.drag_controllers.is_empty();
                
                if cfg.keystroke_draggable != current_draggable {
                    if cfg.keystroke_draggable {
                         let controllers = setup_drag(&window_c);
                         s.drag_controllers = controllers;
                    } else {
                         for c in &s.drag_controllers {
                             window_c.remove_controller(c);
                         }
                         s.drag_controllers.clear();
                         
                         crate::ui::window::update_position(&window_c, cfg.position, cfg.margin);
                    }
                }
            }

            if !cfg.keystroke_draggable {
                crate::ui::window::update_position(&window_c, cfg.position, cfg.margin);
            }
        }
    });

    let active = Arc::clone(&mode_active);
    let state_clone = Rc::clone(&state);
    setup_keystroke_cleanup_timer(display.clone(), window.clone(), state_clone, active);

    window.present();

    Ok(())
}

fn start_bubble_mode(
    app: &Application,
    config_service: &ConfigService,
    state: Rc<RefCell<RuntimeState>>,
) -> Result<()> {
    info!("Starting bubble mode");

    let config = config_service.get_config();
    
    {
        let mut s = state.borrow_mut();
        s.routing_engine.reset_mode_state();
    }

    let window = create_bubble_window(app, &config)?;

    if config.bubble_draggable {
        let controllers = setup_drag(&window);
        state.borrow_mut().drag_controllers = controllers;
    }

    let mut display_widget = BubbleDisplayWidget::new(config.bubble_timeout_ms);
    
     let mut layout_manager = if config.auto_detect_layout {
        let lm = LayoutManager::new();
        if let Err(e) = lm.init() {
            warn!("Failed to initialize layout detection: {}", e);
            None
        } else {
            Some(lm)
        }
    } else {
        None
    };

    let initial_layout = if let Some(ref layout) = config.keyboard_layout {
        Some(layout.clone())
    } else if let Some(ref lm) = layout_manager {
        lm.current_layout_name()
    } else {
        None
    };

    if let Some(ref name) = initial_layout {
        info!("Using keyboard layout: {}", name);
        display_widget.set_layout(name);
    }

    let display = Rc::new(RefCell::new(display_widget));

    window.set_child(Some(display.borrow().widget()));

    let (layout_tx, layout_rx) = bounded::<LayoutEvent>(16);
    if let Some(ref mut lm) = layout_manager {
        let tx = layout_tx.clone();
        lm.start_listener(move |event| {
            let _ = tx.try_send(event);
        });
    }

    let (sender, receiver) = bounded::<KeyEvent>(1024);

    let listener_config = ListenerConfig {
        all_keyboards: config.all_keyboards,
        ..Default::default()
    };

    let listener = KeyListener::new(sender, listener_config);
    let handle = listener.start()?;

    {
        let s = state.borrow();
        let (capture, focus) = s.routing_engine.get_states();
        if capture == CaptureState::Active && focus == crate::domain::FocusState::Focused {
            handle.send_command(InputControlCommand::Grab);
        }
    }

    let mode_active = Arc::new(AtomicBool::new(true));
    {
        let mut s = state.borrow_mut();
        s.mode_active = Some(Arc::clone(&mode_active));
        s.listener_handle = Some(handle);
        s.bubble_window = Some(window.clone());
        s.layout_manager = layout_manager;
    }

    let active = Arc::clone(&mode_active);
    let state_clone = Rc::clone(&state);
    let display_clone = Rc::clone(&display);
    
    let app_clone = app.clone();
    let config_service_clone = config_service.clone();

    glib::MainContext::default().spawn_local(async move {
        let app = app_clone.clone();
        let config_service = config_service_clone.clone();
        
        while let Ok(event) = receiver.recv().await {

            if !active.load(Ordering::SeqCst) {
                break;
            }

            let routing_result = match &event {
                KeyEvent::Pressed(kd) => state_clone
                    .borrow_mut()
                    .routing_engine
                    .process(kd.key, true, true, DisplayMode::Bubble),
                KeyEvent::Released(kd) => state_clone
                    .borrow_mut()
                    .routing_engine
                    .process(kd.key, false, true, DisplayMode::Bubble),
                KeyEvent::AllReleased => RoutingResult::Dispatch(evdev::Key::KEY_RESERVED, false),
            };

            match routing_result {
                RoutingResult::Ignored => continue,
                RoutingResult::StateChanged(capture, focus) => {
                    if let Some(handle) = &state_clone.borrow().listener_handle {
                        if capture == CaptureState::Active
                            && focus == crate::domain::FocusState::Focused
                        {
                            handle.send_command(InputControlCommand::Grab);
                        } else {
                            handle.send_command(InputControlCommand::Ungrab);
                        }
                    }

                    if let Some(service) = &state_clone.borrow().feedback_service {
                        service.handle_state_change(capture, focus);
                    }

                    continue;
                }
                RoutingResult::SwitchMode(mode) => {
                    let app = app.clone();
                    let state = state_clone.clone();
                    let service = config_service.clone();
                    
                    glib::MainContext::default().spawn_local(async move {
                        switch_mode(&app, &state, &service, mode);
                    });
                    break;
                }
                RoutingResult::Dispatch(_, _) => {
                    let mut display = display_clone.borrow_mut();
                    match event {
                        KeyEvent::Pressed(key) => {
                            display.process_key(key);
                        }
                        KeyEvent::Released(key) => {
                            display.process_key_release(key);
                        }
                        KeyEvent::AllReleased => {}
                    }
                }
            }
        }
        debug!("Bubble event loop terminated");
    });
    
    let active = Arc::clone(&mode_active);
    let state_clone = Rc::clone(&state);
    let display_clone = Rc::clone(&display);

    glib::MainContext::default().spawn_local(async move {
        while let Ok(event) = layout_rx.recv().await {
            if !active.load(Ordering::SeqCst) {
                break;
            }
            if state_clone.borrow().routing_engine.get_states().0 == CaptureState::Paused {
                continue;
            }

            match event {
                LayoutEvent::LayoutSwitched { name, .. } => {
                    info!("Layout switched to: {}", name);
                    display_clone.borrow_mut().set_layout(&name);
                }
                LayoutEvent::LayoutsChanged { .. } => {}
            }
        }
        debug!("Layout event loop terminated");
    });

    let _active = Arc::clone(&mode_active);
    let _state_clone = Rc::clone(&state);
    let _display_clone = Rc::clone(&display);

    let mut rx = config_service.subscribe();
    let display_c = display.clone();
    let window_c = window.clone();
    let state_c = state.clone();
    let active_c = Arc::clone(&mode_active);

    glib::MainContext::default().spawn_local(async move {
        while rx.changed().await.is_ok() {
            if !active_c.load(Ordering::SeqCst) {
                 break;
            }
            let cfg = rx.borrow().clone();
            
            state_c.borrow_mut().routing_engine.update_config(
                &cfg.keystroke_hotkey, 
                &cfg.bubble_hotkey
            );

            {
                let mut disp = display_c.borrow_mut();
                disp.set_display_duration(cfg.bubble_timeout_ms);
            }

            {
                let mut s = state_c.borrow_mut();
                let current_draggable = !s.drag_controllers.is_empty();
                
                if cfg.bubble_draggable != current_draggable {
                    if cfg.bubble_draggable {
                         let controllers = setup_drag(&window_c);
                         s.drag_controllers = controllers;
                    } else {
                         for c in &s.drag_controllers {
                             window_c.remove_controller(c);
                         }
                         s.drag_controllers.clear();
                         
                         crate::ui::window::update_position(&window_c, cfg.bubble_position, cfg.margin);
                    }
                }
            }
            
            if !cfg.bubble_draggable {
                crate::ui::window::update_position(&window_c, cfg.bubble_position, cfg.margin);
            }
        }
    });

    let active = Arc::clone(&mode_active);
    let state_clone = Rc::clone(&state);
    setup_bubble_cleanup_timer(display.clone(), window.clone(), state_clone, active);

    window.present();

    Ok(())
}

fn setup_keystroke_cleanup_timer(
    display: Rc<RefCell<KeyDisplayWidget>>,
    window: ApplicationWindow,
    state: Rc<RefCell<RuntimeState>>,
    mode_active: Arc<AtomicBool>,
) {
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if !mode_active.load(Ordering::SeqCst) {
            return ControlFlow::Break;
        }

        if state.borrow().routing_engine.get_states().0 == CaptureState::Paused {
            return ControlFlow::Continue;
        }

        let mut display = display.borrow_mut();
        display.remove_expired();

        if display.has_keys() {
            window.remove_css_class("fading-out");
            window.set_visible(true);
        } else if !window.has_css_class("fading-out") {
            window.add_css_class("fading-out");

            let w = window.clone();
            glib::timeout_add_local_once(Duration::from_millis(200), move || {
                if w.has_css_class("fading-out") {
                    w.set_visible(false);
                }
            });
        }

        ControlFlow::Continue
    });
}

fn setup_bubble_cleanup_timer(
    display: Rc<RefCell<BubbleDisplayWidget>>,
    window: ApplicationWindow,
    state: Rc<RefCell<RuntimeState>>,
    mode_active: Arc<AtomicBool>,
) {
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if !mode_active.load(Ordering::SeqCst) {
            return ControlFlow::Break;
        }

        if state.borrow().routing_engine.get_states().0 == CaptureState::Paused {
            return ControlFlow::Continue;
        }

        let mut display = display.borrow_mut();
        display.remove_expired();

        if display.should_show() {
            window.remove_css_class("fading-out");
            window.set_visible(true);
        } else if !window.has_css_class("fading-out") {
            window.add_css_class("fading-out");

            let w = window.clone();
            glib::timeout_add_local_once(Duration::from_millis(200), move || {
                if w.has_css_class("fading-out") {
                    w.set_visible(false);
                }
            });
        }

        ControlFlow::Continue
    });
}

fn load_css_defaults() {
    let provider = gtk4::CssProvider::new();
    let defaults = include_str!("../style/defaults.css");
    let settings = include_str!("../style/settings.css");
    
    let bubble_css = include_str!("../style/bubble.css");

    provider.load_from_string(&format!("{}\n{}\n{}", defaults, settings, bubble_css));

    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
