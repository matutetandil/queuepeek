use crossterm::event::{KeyCode, KeyModifiers};
use crate::app::{App, Popup, Screen};

pub fn handle_exchange_list_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.popup != Popup::None {
        super::popup::handle_popup_key(app, code, modifiers);
        return;
    }

    if app.exchange_filter_active && app.exchange_filter_focused {
        handle_exchange_filter_key(app, code);
        return;
    }

    if app.exchange_filter_active && code == KeyCode::BackTab {
        app.exchange_filter_focused = true;
        return;
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.popup = if app.popup == Popup::Help { Popup::None } else { Popup::Help };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let flat = super::popup::topology_flat_list(app);
            if !flat.is_empty() && app.topology_selected + 1 < flat.len() {
                app.topology_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.topology_selected = app.topology_selected.saturating_sub(1);
        }
        KeyCode::Enter => {
            let flat = super::popup::topology_flat_list(app);
            if let Some(super::popup::TopologyFlatItem::Exchange(ref name)) = flat.get(app.topology_selected) {
                let name = name.clone();
                if app.topology_expanded.contains(&name) {
                    app.topology_expanded.remove(&name);
                } else {
                    app.topology_expanded.insert(name);
                }
            }
        }
        KeyCode::Char('/') => {
            app.exchange_filter_active = true;
            app.exchange_filter_focused = true;
        }
        KeyCode::Char('b') => {
            let flat = super::popup::topology_flat_list(app);
            let exchange_name = super::popup::topology_selected_exchange(app, &flat);
            if let Some(name) = exchange_name {
                app.binding_form_queue.clear();
                app.binding_form_routing_key.clear();
                app.binding_form_focused = 0;
                app.popup = Popup::AddBinding { exchange: name };
            }
        }
        KeyCode::Char('d') => {
            let flat = super::popup::topology_flat_list(app);
            if let Some(super::popup::TopologyFlatItem::Binding(ref binding)) = flat.get(app.topology_selected) {
                let binding = binding.clone();
                if let Some(ref backend) = app.backend {
                    let backend = backend.clone_backend();
                    let namespace = app.selected_namespace.clone();
                    let tx = app.bg_sender.clone();
                    std::thread::spawn(move || {
                        let result = backend.delete_binding(
                            &namespace,
                            &binding.source,
                            &binding.destination,
                            &binding.properties_key,
                        );
                        let _ = tx.send(crate::app::BgResult::BindingDeleted(result));
                    });
                    app.set_status("Deleting binding...", false);
                }
            }
        }
        KeyCode::Char('i') => {
            let flat = super::popup::topology_flat_list(app);
            if let Some(super::popup::TopologyFlatItem::Exchange(ref name)) = flat.get(app.topology_selected) {
                app.popup = Popup::ExchangeInfo(name.clone());
            }
        }
        KeyCode::Esc => {
            if app.exchange_filter_active {
                if app.exchange_filter_focused {
                    app.exchange_filter_focused = false;
                } else {
                    app.exchange_filter.clear();
                    app.exchange_filter_active = false;
                    app.update_filtered_exchanges();
                }
            } else {
                app.screen = Screen::QueueList;
            }
        }
        _ => {}
    }

    let _ = modifiers; // suppress unused warning
}

fn handle_exchange_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char(c) => {
            app.exchange_filter.push(c);
            app.update_filtered_exchanges();
            app.topology_selected = 0;
        }
        KeyCode::Backspace => {
            app.exchange_filter.pop();
            app.update_filtered_exchanges();
            app.topology_selected = 0;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.exchange_filter_focused = false;
        }
        KeyCode::Esc => {
            if app.exchange_filter.is_empty() {
                app.exchange_filter_active = false;
                app.exchange_filter_focused = false;
            } else {
                app.exchange_filter.clear();
                app.update_filtered_exchanges();
            }
        }
        KeyCode::Enter => {
            app.exchange_filter_focused = false;
        }
        _ => {}
    }
}
