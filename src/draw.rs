// Port of termdown's pad_to_size / draw_text onto ratatui. termdown clips
// too-wide figlet output via curses and falls back to plain text when it is
// too tall; here both overflow cases fall back to the plain-text rendering.
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn pad_to_size(text: &str, x: usize, y: usize) -> String {
    let stripped = text.trim_end();
    let input_lines: Vec<&str> = stripped.split('\n').collect();
    let longest = input_lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let x = x.max(longest);
    let y = y.max(input_lines.len());
    let padding_top = (y - input_lines.len()) / 2;
    let padding_bottom = y - input_lines.len() - padding_top;
    let padding_left = (x - longest) / 2;

    let mut output = String::new();
    for _ in 0..padding_top {
        output.push_str(&" ".repeat(x));
        output.push('\n');
    }
    for line in &input_lines {
        let len = line.chars().count();
        output.push_str(&" ".repeat(padding_left));
        output.push_str(line);
        output.push_str(&" ".repeat(x.saturating_sub(padding_left + len)));
        output.push('\n');
    }
    for _ in 0..padding_bottom {
        output.push_str(&" ".repeat(x));
        output.push('\n');
    }
    output
}

pub fn draw_text(
    frame: &mut Frame,
    text: &str,
    style: Style,
    fallback: &str,
    title: Option<&str>,
    no_figlet_y_offset: i64,
) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    let x = area.width as usize;
    let (effective_y, y_delta) = if no_figlet_y_offset < 0 {
        (area.height as usize, 0u16)
    } else {
        let clamped = no_figlet_y_offset.clamp(0, area.height as i64 - 1) as u16;
        (1usize, clamped)
    };
    let avail_rows = (area.height - y_delta) as usize;

    let compose = |text: &str, title: Option<&str>| -> Vec<String> {
        let mut body = text.to_string();
        if let Some(title) = title {
            let mut padded_title = pad_to_size(title, x, 1);
            if padded_title.trim_end_matches('\n').contains('\n') {
                // extra spacing between figlet title and body, as in termdown
                padded_title.push_str("\n\n\n\n\n");
            }
            let body_line_count = text.split('\n').count();
            body = format!("{}\n{}", padded_title, pad_to_size(text, x, body_line_count));
        }
        pad_to_size(&body, x, effective_y)
            .trim_end_matches('\n')
            .split('\n')
            .map(String::from)
            .collect()
    };

    let fits =
        |lines: &[String]| lines.len() <= avail_rows && lines.iter().all(|l| l.chars().count() <= x);

    let mut lines = compose(text, title);
    if !fits(&lines) {
        lines = compose(fallback, None);
    }
    let rect = Rect {
        x: area.x,
        y: area.y + y_delta,
        width: area.width,
        height: area.height - y_delta,
    };
    frame.render_widget(Paragraph::new(lines.join("\n")).style(style), rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_centers_block() {
        let padded = pad_to_size("ab", 6, 3);
        assert_eq!(padded, "      \n  ab  \n      \n");
    }

    #[test]
    fn pad_grows_to_content() {
        let padded = pad_to_size("abcdef", 2, 1);
        assert_eq!(padded, "abcdef\n");
    }

    #[test]
    fn pad_empty_fills_frame() {
        let padded = pad_to_size("", 3, 2);
        assert_eq!(padded, "   \n   \n");
    }
}
