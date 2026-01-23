use crate::domain::Command;
use evdev::Key;
use std::collections::HashSet;

pub struct GlobalHotKeyManager {
    pressed_keys: HashSet<Key>,
}

impl Default for GlobalHotKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalHotKeyManager {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    pub fn process(&mut self, key: Key, pressed: bool) -> Option<Command> {
        if pressed {
            self.pressed_keys.insert(key);

            let ctrl = self.pressed_keys.contains(&Key::KEY_LEFTCTRL)
                || self.pressed_keys.contains(&Key::KEY_RIGHTCTRL);

            if ctrl {
                match key {
                    Key::KEY_P => return Some(Command::ToggleCapture),
                    Key::KEY_B => return Some(Command::ToggleFocus),
                    _ => {}
                }
            }
        } else {
            self.pressed_keys.remove(&key);
        }

        None
    }
}
