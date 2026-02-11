use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::ui::app_state::{AppState, ViewMode};

pub enum InputAction {
    None,
    Quit,
    Refresh,
    Export,
    CopyPath,
    OpenFile,
}

pub fn handle_key_event(key: KeyEvent, state: &mut AppState) -> InputAction {
    match state.view_mode {
        ViewMode::Normal => handle_normal_mode(key, state),
        ViewMode::Help => handle_help_mode(key, state),
        ViewMode::ErrorList => handle_error_list_mode(key, state),
        ViewMode::Scanning => handle_scanning_mode(key, state),
        ViewMode::Export => InputAction::None,
    }
}

fn handle_normal_mode(key: KeyEvent, state: &mut AppState) -> InputAction {
    // Handle Ctrl+C globally
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.should_quit = true;
        return InputAction::Quit;
    }

    // Handle 'g' prefix for 'gg'
    if state.pending_g {
        state.pending_g = false;
        if key.code == KeyCode::Char('g') {
            state.go_to_first();
            return InputAction::None;
        }
        // If not 'g', fall through to normal handling
    }

    match key.code {
        KeyCode::Char('q') => {
            state.should_quit = true;
            InputAction::Quit
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.move_down();
            InputAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.move_up();
            InputAction::None
        }
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right
            if state.focus == crate::ui::app_state::FocusPanel::FileList =>
        {
            state.enter_directory();
            InputAction::None
        }
        KeyCode::Backspace | KeyCode::Char('h') => {
            state.go_back();
            InputAction::None
        }
        KeyCode::Char('g') => {
            state.pending_g = true;
            InputAction::None
        }
        KeyCode::Char('G') => {
            state.go_to_last();
            InputAction::None
        }
        KeyCode::Char('s') => {
            state.toggle_sort();
            InputAction::None
        }
        KeyCode::Char('t') => {
            state.cycle_threshold();
            InputAction::None
        }
        KeyCode::Left | KeyCode::Right => {
            state.toggle_focus();
            InputAction::None
        }
        KeyCode::Tab => {
            state.toggle_focus();
            InputAction::None
        }
        KeyCode::Char('e') => {
            state.toggle_error_list();
            InputAction::None
        }
        KeyCode::Char('?') => {
            state.toggle_help();
            InputAction::None
        }
        KeyCode::Char('r') => InputAction::Refresh,
        KeyCode::Char('x') => InputAction::Export,
        KeyCode::Char('y') => InputAction::CopyPath,
        KeyCode::Char('o') => InputAction::OpenFile,
        _ => InputAction::None,
    }
}

fn handle_help_mode(key: KeyEvent, state: &mut AppState) -> InputAction {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
            state.toggle_help();
            InputAction::None
        }
        _ => InputAction::None,
    }
}

fn handle_error_list_mode(key: KeyEvent, state: &mut AppState) -> InputAction {
    match key.code {
        KeyCode::Char('e') | KeyCode::Esc | KeyCode::Char('q') => {
            state.toggle_error_list();
            InputAction::None
        }
        _ => InputAction::None,
    }
}

fn handle_scanning_mode(key: KeyEvent, state: &mut AppState) -> InputAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.should_quit = true;
        return InputAction::Quit;
    }
    match key.code {
        KeyCode::Char('q') => {
            state.should_quit = true;
            InputAction::Quit
        }
        _ => InputAction::None,
    }
}

pub fn poll_event(timeout: Duration) -> anyhow::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
