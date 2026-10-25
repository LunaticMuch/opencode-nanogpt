use crate::api::models::ModelEntry;
use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, size},
};
use std::collections::HashSet;
use std::io::{stdout, Write};
use std::time::Duration;

pub fn select_models(
    models: &[ModelEntry],
    existing: &HashSet<String>,
) -> Result<Option<Vec<String>>> {
    enable_raw_mode().context("enabling raw mode")?;
    let result = run_select(models, existing);
    let _ = disable_raw_mode();
    // Leave a clean screen behind.
    let mut out = stdout();
    let _ = execute!(out, Clear(ClearType::All), MoveTo(0, 0));
    let _ = out.flush();
    result
}

fn run_select(
    models: &[ModelEntry],
    existing: &HashSet<String>,
) -> Result<Option<Vec<String>>> {
    let mut filter = String::new();
    let mut checked: Vec<bool> = models.iter().map(|m| existing.contains(&m.id)).collect();
    let mut cursor: usize = 0;

    loop {
        let filtered = filtered_indices(models, &filter);
        if cursor >= filtered.len() && !filtered.is_empty() {
            cursor = filtered.len() - 1;
        }

        render(models, &filtered, &checked, cursor, &filter)?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        let selected = filtered
                            .iter()
                            .enumerate()
                            .filter_map(|(_, &idx)| if checked[idx] { Some(models[idx].id.clone()) } else { None })
                            .collect();
                        // If nothing checked, return whatever is selected by cursor? Keep explicit.
                        return Ok(Some(selected));
                    }
                    KeyCode::Up => {
                        if cursor > 0 {
                            cursor -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if !filtered.is_empty() && cursor + 1 < filtered.len() {
                            cursor += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(&idx) = filtered.get(cursor) {
                            checked[idx] = !checked[idx];
                        }
                    }
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        for &idx in &filtered {
                            checked[idx] = true;
                        }
                    }
                    KeyCode::Backspace => {
                        filter.pop();
                        cursor = 0;
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            filter.push(c);
                            cursor = 0;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn filtered_indices(models: &[ModelEntry], filter: &str) -> Vec<usize> {
    if filter.is_empty() {
        return (0..models.len()).collect();
    }
    let f = filter.to_lowercase();
    models
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.id.to_lowercase().contains(&f)
                || m.name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&f)
        })
        .map(|(i, _)| i)
        .collect()
}

fn render(
    models: &[ModelEntry],
    filtered: &[usize],
    checked: &[bool],
    cursor: usize,
    filter: &str,
) -> Result<()> {
    let mut out = stdout();
    let (cols, rows) = size().unwrap_or((80, 24));
    execute!(out, Clear(ClearType::All), MoveTo(0, 0))?;

    execute!(
        out,
        SetForegroundColor(Color::Cyan),
        Print("Select models (Space toggle, Enter confirm, Esc cancel, type to filter)\r\n"),
        ResetColor
    )?;
    execute!(out, Print(format!("Filter: {}\r\n", filter)))?;
    execute!(
        out,
        Print(format!(
            "Showing {} of {} ({} selected)\r\n\r\n",
            filtered.len(),
            models.len(),
            checked.iter().filter(|&&b| b).count()
        ))
    )?;

    let header_rows: u16 = 4;
    let list_height = rows.saturating_sub(header_rows + 1) as usize;
    let start = if filtered.len() <= list_height {
        0
    } else {
        cursor
            .saturating_sub(list_height / 2)
            .min(filtered.len() - list_height)
    };

    for i in 0..list_height {
        let fi = start + i;
        if fi >= filtered.len() {
            break;
        }
        let idx = filtered[fi];
        let marker = if checked[idx] { "[x]" } else { "[ ]" };
        let cursor_mark = if fi == cursor { ">" } else { " " };
        let name = models[idx].name.as_deref().unwrap_or("");
        let mut line = format!("{} {} {}  {}", cursor_mark, marker, models[idx].id, name);
        if line.len() > cols as usize {
            line.truncate(cols as usize);
        }
        if fi == cursor {
            execute!(out, SetForegroundColor(Color::Yellow))?;
        }
        execute!(out, Print(format!("{}\r\n", line)))?;
        if fi == cursor {
            execute!(out, ResetColor)?;
        }
    }

    execute!(
        out,
        MoveTo(0, rows.saturating_sub(1)),
        Print("Ctrl-A: select all filtered   Backspace: delete filter char")
    )?;
    out.flush()?;
    Ok(())
}