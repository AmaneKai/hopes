mod app;
mod event;
mod models;
mod storage;
mod tui;
mod ui;

use app::{App, AppMode, FocusPane, FormField, ViewMode};
use crossterm::event::{KeyCode, KeyModifiers};
use event::{Event, EventHandler};
use models::Priority;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = tui::init()?;
    let mut app = App::new();
    let events = EventHandler::new(60);
    let mut needs_draw = true;

    loop {
        if needs_draw || app.get_status_message().is_some() {
            terminal.draw(|f| ui::render(f, &mut app))?;
            needs_draw = false;
        }

        if let Event::Key(key) = events.next()? {
            needs_draw = true;
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

            match app.mode {
                AppMode::Normal => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if ctrl => break,

                    KeyCode::Tab | KeyCode::Char('w') if ctrl => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.next_pane();
                    }
                    KeyCode::BackTab => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.prev_pane();
                    }

                    KeyCode::Char('v') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.toggle_view_mode();
                    }
                    KeyCode::Char('s') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.cycle_sort();
                    }

                    KeyCode::Char('j') | KeyCode::Down => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.select_next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.select_prev();
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        app.pending_g = false;
                        app.pending_d = false;
                        if app.view_mode == ViewMode::Kanban {
                            app.kanban_left();
                        } else {
                            app.prev_pane();
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        app.pending_g = false;
                        app.pending_d = false;
                        if app.view_mode == ViewMode::Kanban {
                            app.kanban_right();
                        } else {
                            app.next_pane();
                        }
                    }

                    KeyCode::Char('H') if app.view_mode == ViewMode::Kanban => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.kanban_move_task_left();
                    }
                    KeyCode::Char('L') if app.view_mode == ViewMode::Kanban => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.kanban_move_task_right();
                    }

                    KeyCode::Char('g') => {
                        app.pending_d = false;
                        if app.pending_g {
                            app.jump_to_top();
                            app.pending_g = false;
                        } else {
                            app.pending_g = true;
                        }
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.jump_to_bottom();
                    }
                    KeyCode::Char('H') | KeyCode::Home => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.jump_to_top();
                    }
                    KeyCode::Char('L') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.jump_to_bottom();
                    }
                    KeyCode::Char('d') if ctrl => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.page_down(5);
                    }
                    KeyCode::PageDown => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.page_down(5);
                    }
                    KeyCode::Char('u') if ctrl => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.page_up(5);
                    }
                    KeyCode::PageUp => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.page_up(5);
                    }

                    KeyCode::Char('J') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.move_selected_down();
                    }
                    KeyCode::Char('K') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.move_selected_up();
                    }

                    KeyCode::Char('a') | KeyCode::Char('o') | KeyCode::Char('i') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.open_create_modal();
                    }
                    KeyCode::Char('e') | KeyCode::Char('c') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.open_edit_modal();
                    }
                    KeyCode::Enter => {
                        app.pending_g = false;
                        app.pending_d = false;
                        if app.focus_pane == FocusPane::Nav {
                            app.apply_nav_selection();
                            app.focus_pane = FocusPane::Main;
                        } else {
                            app.open_edit_modal();
                        }
                    }

                    KeyCode::Char(' ') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        if app.focus_pane == FocusPane::Nav {
                            app.apply_nav_selection();
                        } else if app.view_mode == ViewMode::Kanban {
                            app.kanban_move_task_right();
                        } else {
                            app.toggle_status();
                        }
                    }
                    KeyCode::Char('p') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.cycle_priority_forward();
                    }
                    KeyCode::Char('P') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.cycle_priority_backward();
                    }
                    KeyCode::Char(c @ '1'..='5') if app.focus_pane == FocusPane::Nav => {
                        app.pending_g = false;
                        app.pending_d = false;
                        let idx = (c as usize) - ('1' as usize);
                        if idx < app::NavSection::ALL.len() {
                            app.nav_index = idx;
                            app.apply_nav_selection();
                        }
                    }
                    KeyCode::Char(c @ '1'..='4') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        if let Some(prio) = Priority::from_digit(c) {
                            app.set_priority(prio);
                        }
                    }

                    KeyCode::Char('d') => {
                        app.pending_g = false;
                        if app.pending_d {
                            app.delete_selected();
                            app.pending_d = false;
                        } else {
                            app.pending_d = true;
                        }
                    }
                    KeyCode::Char('x') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.delete_selected();
                    }
                    KeyCode::Char('u') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.undo_delete();
                    }

                    KeyCode::Char('/') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.filter_input = app.tag_filter.clone().unwrap_or_default();
                        app.mode = AppMode::Filtering;
                    }
                    KeyCode::Char('t') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.cycle_tag_filter();
                    }
                    KeyCode::Esc => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.clear_filter();
                    }

                    KeyCode::Char('?') => {
                        app.pending_g = false;
                        app.pending_d = false;
                        app.mode = AppMode::Help;
                    }

                    _ => {
                        app.pending_g = false;
                        app.pending_d = false;
                    }
                },

                AppMode::Creating | AppMode::Editing => {
                    if key.code == KeyCode::Esc || (ctrl && key.code == KeyCode::Char('c')) {
                        app.mode = AppMode::Normal;
                        continue;
                    }

                    if (key.code == KeyCode::Enter && app.form_field != FormField::Description)
                        || (ctrl && key.code == KeyCode::Char('s'))
                    {
                        app.commit_form();
                        continue;
                    }

                    if (ctrl && (key.code == KeyCode::Char('j') || key.code == KeyCode::Char('n')))
                        || key.code == KeyCode::Tab
                        || (key.code == KeyCode::Down && app.form_field != FormField::Description)
                    {
                        app.form_field = app.form_field.next();
                        continue;
                    }
                    if (ctrl && (key.code == KeyCode::Char('k') || key.code == KeyCode::Char('p')))
                        || key.code == KeyCode::BackTab
                        || (key.code == KeyCode::Up && app.form_field != FormField::Description)
                    {
                        app.form_field = app.form_field.prev();
                        continue;
                    }

                    match app.form_field {
                        FormField::Title => match key.code {
                            KeyCode::Char('w') if ctrl => {
                                App::delete_word_back(&mut app.form_title)
                            }
                            KeyCode::Char('u') if ctrl => app.form_title.clear(),
                            KeyCode::Char(c) => app.form_title.push(c),
                            KeyCode::Backspace => {
                                app.form_title.pop();
                            }
                            _ => {}
                        },
                        FormField::Description => match key.code {
                            KeyCode::Char('w') if ctrl => {
                                App::delete_word_back(&mut app.form_description)
                            }
                            KeyCode::Char('u') if ctrl => app.form_description.clear(),
                            KeyCode::Enter => app.form_description.push('\n'),
                            KeyCode::Char(c) => app.form_description.push(c),
                            KeyCode::Backspace => {
                                app.form_description.pop();
                            }
                            _ => {}
                        },
                        FormField::Priority => match key.code {
                            KeyCode::Char(' ')
                            | KeyCode::Char('l')
                            | KeyCode::Char('p')
                            | KeyCode::Right => {
                                app.form_priority = app.form_priority.next();
                            }
                            KeyCode::Char('h') | KeyCode::Char('P') | KeyCode::Left => {
                                app.form_priority = app.form_priority.prev();
                            }
                            KeyCode::Char(c @ '1'..='4') => {
                                if let Some(prio) = Priority::from_digit(c) {
                                    app.form_priority = prio;
                                }
                            }
                            _ => {}
                        },
                        FormField::Tags => match key.code {
                            KeyCode::Char('w') if ctrl => App::delete_word_back(&mut app.form_tags),
                            KeyCode::Char('u') if ctrl => app.form_tags.clear(),
                            KeyCode::Char(c) => app.form_tags.push(c),
                            KeyCode::Backspace => {
                                app.form_tags.pop();
                            }
                            _ => {}
                        },
                    }
                }

                AppMode::Filtering => match key.code {
                    KeyCode::Enter => {
                        let query = app.filter_input.trim();
                        if query.is_empty() {
                            app.tag_filter = None;
                        } else {
                            app.tag_filter = Some(query.to_string());
                        }
                        app.refresh_cache();
                        app.table_state.select(Some(0));
                        app.mode = AppMode::Normal;
                    }
                    KeyCode::Esc => {
                        app.mode = AppMode::Normal;
                    }
                    KeyCode::Char('w') if ctrl => App::delete_word_back(&mut app.filter_input),
                    KeyCode::Char('u') if ctrl => app.filter_input.clear(),
                    KeyCode::Char(c) => {
                        app.filter_input.push(c);
                        app.table_state.select(Some(0));
                    }
                    KeyCode::Backspace => {
                        app.filter_input.pop();
                        app.table_state.select(Some(0));
                    }
                    _ => {}
                },

                AppMode::Help => match key.code {
                    KeyCode::Esc
                    | KeyCode::Char('?')
                    | KeyCode::Char('q')
                    | KeyCode::Enter
                    | KeyCode::Char(' ') => {
                        app.mode = AppMode::Normal;
                    }
                    _ => {}
                },
            }
        }
    }

    tui::restore()?;
    Ok(())
}
