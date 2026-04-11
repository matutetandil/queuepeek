use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{self, App, Popup, ProfileMode};
use crate::ui;

pub fn handle_profile_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.popup != Popup::None {
        super::popup::handle_popup_key(app, code, modifiers);
        return;
    }

    match app.profile_mode {
        ProfileMode::Select => handle_profile_select_key(app, code),
        ProfileMode::Add | ProfileMode::Edit(_) => handle_profile_form_key(app, code),
        ProfileMode::ConfirmDelete => handle_profile_delete_key(app, code),
    }
}

fn handle_profile_select_key(app: &mut App, code: KeyCode) {
    let names = app.config.profile_names();
    let total = names.len() + 2;

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            let i = app.profile_list_state.selected().unwrap_or(0);
            if i + 1 < total {
                app.profile_list_state.select(Some(i + 1));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.profile_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.profile_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => {
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                let name = names[selected].clone();
                app.connect_profile(&name);
            } else if selected == names.len() {
                app.profile_form.clear();
                app.profile_mode = ProfileMode::Add;
            } else if selected == names.len() + 1 {
                app.popup = Popup::ThemePicker;
                let current = app.config.theme.as_deref().unwrap_or("slack");
                let idx = ui::theme::theme_names().iter().position(|&n| n == current).unwrap_or(0);
                app.popup_list_state.select(Some(idx));
            }
        }
        KeyCode::Char('a') => {
            app.profile_form.clear();
            app.profile_mode = ProfileMode::Add;
        }
        KeyCode::Char('e') => {
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                let name = &names[selected];
                if let Some(profile) = app.config.profiles.get(name) {
                    app.profile_form = app::ProfileForm::from_profile(name, profile);
                    app.profile_mode = ProfileMode::Edit(name.clone());
                }
            }
        }
        KeyCode::Char('d') => {
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                app.profile_mode = ProfileMode::ConfirmDelete;
            }
        }
        KeyCode::Char('t') => {
            app.popup = Popup::ThemePicker;
            let current = app.config.theme.as_deref().unwrap_or("slack");
            let idx = ui::theme::theme_names().iter().position(|&n| n == current).unwrap_or(0);
            app.popup_list_state.select(Some(idx));
        }
        _ => {}
    }
}

fn handle_profile_form_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.profile_mode = ProfileMode::Select;
            app.profile_form.error.clear();
        }
        KeyCode::Tab | KeyCode::Down => {
            let next = (app.profile_form.focused_field + 1) % app::ProfileForm::field_count();
            app.profile_form.focused_field = next;
        }
        KeyCode::BackTab | KeyCode::Up => {
            let count = app::ProfileForm::field_count();
            let prev = (app.profile_form.focused_field + count - 1) % count;
            app.profile_form.focused_field = prev;
        }
        KeyCode::Backspace => {
            app.profile_form.pop_char();
        }
        KeyCode::Char(c) => {
            app.profile_form.push_char(c);
        }
        KeyCode::Enter => {
            if app.profile_form.focused_field == 0 {
                app.popup = Popup::BackendTypePicker;
                let idx = app::BACKEND_TYPES.iter()
                    .position(|&t| t == app.profile_form.profile_type)
                    .unwrap_or(0);
                app.popup_list_state.select(Some(idx));
                return;
            }
            if app.profile_form.focused_field == 7 {
                app.profile_form.tls = !app.profile_form.tls;
                return;
            }
            match app.profile_form.to_profile() {
                Ok(profile) => {
                    let name = app.profile_form.name.clone();
                    if let ProfileMode::Edit(ref old_name) = app.profile_mode {
                        if old_name != &name {
                            app.config.delete_profile(old_name);
                        }
                    }
                    app.config.add_profile(name.clone(), profile);
                    let _ = app.config.save(app.config_path.as_deref());
                    app.profile_mode = ProfileMode::Select;
                    app.connect_profile(&name);
                }
                Err(e) => {
                    app.profile_form.error = e;
                }
            }
        }
        _ => {}
    }
}

fn handle_profile_delete_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let names = app.config.profile_names();
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                let name = names[selected].clone();
                app.config.delete_profile(&name);
                let _ = app.config.save(app.config_path.as_deref());
                let new_len = app.config.profile_names().len() + 2;
                if selected >= new_len {
                    app.profile_list_state.select(Some(new_len.saturating_sub(1)));
                }
            }
            app.profile_mode = ProfileMode::Select;
        }
        _ => {
            app.profile_mode = ProfileMode::Select;
        }
    }
}
