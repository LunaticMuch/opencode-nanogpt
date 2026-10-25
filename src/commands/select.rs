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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{ModelEntry, ModelSubscription};

    fn mk(id: &str, included: Option<bool>, mult: Option<i64>, open_weights: Option<bool>) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            created: None,
            name: Some(id.to_string()),
            description: None,
            context_length: None,
            max_output_tokens: None,
            pricing: None,
            capabilities: None,
            subscription: included.map(|inc| ModelSubscription {
                included: inc,
                input_token_multiplier: mult,
                note: None,
            }),
            open_weights,
            providers: None,
        }
    }

    #[test]
    fn filters_by_text() {
        let models = vec![mk("openai/gpt", Some(true), Some(1), Some(true)), mk("deepseek/v", Some(false), Some(2), Some(false))];
        let out = filtered_indices(&models, "gpt", SubFilter::All, MultFilter::All);
        assert_eq!(out, vec![0]);
    }

    #[test]
    fn filters_subscription_included() {
        let models = vec![mk("a", Some(true), Some(1), None), mk("b", Some(false), Some(2), None)];
        let out = filtered_indices(&models, "", SubFilter::Included, MultFilter::All);
        assert_eq!(out, vec![0]);
    }

    #[test]
    fn filters_subscription_excluded() {
        let models = vec![mk("a", Some(true), Some(1), None), mk("b", Some(false), Some(2), None)];
        let out = filtered_indices(&models, "", SubFilter::Excluded, MultFilter::All);
        assert_eq!(out, vec![1]);
    }

    #[test]
    fn filters_multiplier() {
        let models = vec![mk("a", Some(true), Some(1), None), mk("b", Some(true), Some(2), None)];
        assert_eq!(filtered_indices(&models, "", SubFilter::All, MultFilter::One), vec![0]);
        assert_eq!(filtered_indices(&models, "", SubFilter::All, MultFilter::Two), vec![1]);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubFilter {
    All,
    Included,
    Excluded,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MultFilter {
    All,
    One,
    Two,
}

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
    let mut sub_filter = SubFilter::All;
    let mut mult_filter = MultFilter::All;

    loop {
        let filtered = filtered_indices(models, &filter, sub_filter, mult_filter);
        if cursor >= filtered.len() && !filtered.is_empty() {
            cursor = filtered.len() - 1;
        }

        render(models, &filtered, &checked, cursor, &filter, sub_filter, mult_filter)?;

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
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        sub_filter = match sub_filter {
                            SubFilter::All => SubFilter::Included,
                            SubFilter::Included => SubFilter::Excluded,
                            SubFilter::Excluded => SubFilter::All,
                        };
                        cursor = 0;
                    }
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        mult_filter = match mult_filter {
                            MultFilter::All => MultFilter::One,
                            MultFilter::One => MultFilter::Two,
                            MultFilter::Two => MultFilter::All,
                        };
                        cursor = 0;
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

fn filtered_indices(
    models: &[ModelEntry],
    filter: &str,
    sub_filter: SubFilter,
    mult_filter: MultFilter,
) -> Vec<usize> {
    let f = filter.to_lowercase();
    models
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            let text_ok = f.is_empty()
                || m.id.to_lowercase().contains(&f)
                || m.name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&f);
            let sub_ok = match sub_filter {
                SubFilter::All => true,
                SubFilter::Included => m.subscription.as_ref().map(|s| s.included).unwrap_or(false),
                SubFilter::Excluded => !m
                    .subscription
                    .as_ref()
                    .map(|s| s.included)
                    .unwrap_or(false),
            };
            let mult_ok = match mult_filter {
                MultFilter::All => true,
                MultFilter::One => m
                    .subscription
                    .as_ref()
                    .and_then(|s| s.input_token_multiplier)
                    == Some(1),
                MultFilter::Two => m
                    .subscription
                    .as_ref()
                    .and_then(|s| s.input_token_multiplier)
                    == Some(2),
            };
            text_ok && sub_ok && mult_ok
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
    sub_filter: SubFilter,
    mult_filter: MultFilter,
) -> Result<()> {
    let mut out = stdout();
    let (cols, rows) = size().unwrap_or((80, 24));
    execute!(out, Clear(ClearType::All), MoveTo(0, 0))?;

    let sub_label = match sub_filter {
        SubFilter::All => "all",
        SubFilter::Included => "subscription",
        SubFilter::Excluded => "pay-as-you-go",
    };
    let mult_label = match mult_filter {
        MultFilter::All => "all",
        MultFilter::One => "1x",
        MultFilter::Two => "2x",
    };
    execute!(
        out,
        SetForegroundColor(Color::Cyan),
        Print("Select models (Space toggle, Enter confirm, Esc cancel, type to filter)\r\n"),
        ResetColor
    )?;
    execute!(out, Print(format!("Text filter: {}\r\n", filter)))?;
    execute!(out, Print(format!("Subscription: {}   Token multiplier: {}\r\n", sub_label, mult_label)))?;
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
        let sub = models[idx]
            .subscription
            .as_ref()
            .map(|s| if s.included { "sub" } else { "payg" })
            .unwrap_or("");
        let mult = models[idx]
            .subscription
            .as_ref()
            .and_then(|s| s.input_token_multiplier)
            .map(|m| format!("{}x", m))
            .unwrap_or_default();
        let mut line = format!(
            "{} {} {}  {}  [{} {}]",
            cursor_mark, marker, models[idx].id, name, sub, mult
        );
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
        Print("Ctrl-A: select all filtered   Ctrl-S: subscription filter   Ctrl-T: multiplier filter   Backspace: delete")
    )?;
    out.flush()?;
    Ok(())
}