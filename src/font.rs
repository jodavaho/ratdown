// The display font: either the built-in block pixel font or a figlet font.
// The block font uses 3x5 bitmap glyphs rendered with U+2588 FULL BLOCK,
// two characters per pixel to keep pixels roughly square in terminal cells.
// Letters are single-case; unknown characters are skipped, as in figlet.
use crate::figlet::FigletFont;

const PX_ON: &str = "\u{2588}\u{2588}";
const PX_OFF: &str = "  ";

pub enum Font {
    Block,
    Figlet(FigletFont),
}

impl Font {
    pub fn render(&self, text: &str) -> String {
        match self {
            Font::Block => render_block(text),
            Font::Figlet(f) => f.render(text),
        }
    }
}

fn render_block(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut rows = vec![String::new(); 5];
    let mut first = true;
    for ch in text.chars() {
        let Some(g) = glyph(ch.to_ascii_uppercase()) else {
            continue;
        };
        for (row, pattern) in rows.iter_mut().zip(g.iter()) {
            if !first {
                row.push_str(PX_OFF);
            }
            for p in pattern.chars() {
                row.push_str(if p == '#' { PX_ON } else { PX_OFF });
            }
        }
        first = false;
    }
    let mut out = String::new();
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    out
}

fn glyph(c: char) -> Option<&'static [&'static str; 5]> {
    let g: &[&str; 5] = match c {
        '0' => &["###", "# #", "# #", "# #", "###"],
        '1' => &[" # ", "## ", " # ", " # ", "###"],
        '2' => &["###", "  #", "###", "#  ", "###"],
        '3' => &["###", "  #", "###", "  #", "###"],
        '4' => &["# #", "# #", "###", "  #", "  #"],
        '5' => &["###", "#  ", "###", "  #", "###"],
        '6' => &["###", "#  ", "###", "# #", "###"],
        '7' => &["###", "  #", "  #", "  #", "  #"],
        '8' => &["###", "# #", "###", "# #", "###"],
        '9' => &["###", "# #", "###", "  #", "###"],
        'A' => &[" # ", "# #", "###", "# #", "# #"],
        'B' => &["## ", "# #", "## ", "# #", "## "],
        'C' => &["###", "#  ", "#  ", "#  ", "###"],
        'D' => &["## ", "# #", "# #", "# #", "## "],
        'E' => &["###", "#  ", "###", "#  ", "###"],
        'F' => &["###", "#  ", "###", "#  ", "#  "],
        'G' => &["###", "#  ", "# #", "# #", "###"],
        'H' => &["# #", "# #", "###", "# #", "# #"],
        'I' => &["###", " # ", " # ", " # ", "###"],
        'J' => &["  #", "  #", "  #", "# #", "###"],
        'K' => &["# #", "# #", "## ", "# #", "# #"],
        'L' => &["#  ", "#  ", "#  ", "#  ", "###"],
        'M' => &["# #", "###", "###", "# #", "# #"],
        'N' => &["###", "# #", "# #", "# #", "# #"],
        'O' => &["###", "# #", "# #", "# #", "###"],
        'P' => &["###", "# #", "###", "#  ", "#  "],
        'Q' => &["###", "# #", "# #", "###", "  #"],
        'R' => &["###", "# #", "## ", "# #", "# #"],
        'S' => &["###", "#  ", "###", "  #", "###"],
        'T' => &["###", " # ", " # ", " # ", " # "],
        'U' => &["# #", "# #", "# #", "# #", "###"],
        'V' => &["# #", "# #", "# #", "# #", " # "],
        'W' => &["# #", "# #", "###", "###", "# #"],
        'X' => &["# #", "# #", " # ", "# #", "# #"],
        'Y' => &["# #", "# #", " # ", " # ", " # "],
        'Z' => &["###", "  #", " # ", "#  ", "###"],
        ' ' => &["   ", "   ", "   ", "   ", "   "],
        ':' => &[" ", "#", " ", "#", " "],
        '.' => &[" ", " ", " ", " ", "#"],
        ',' => &["  ", "  ", "  ", " #", "# "],
        '-' => &["   ", "   ", "###", "   ", "   "],
        '+' => &["   ", " # ", "###", " # ", "   "],
        '/' => &["  #", "  #", " # ", "#  ", "#  "],
        '!' => &["#", "#", "#", " ", "#"],
        '?' => &["###", "  #", " ##", "   ", " # "],
        '\'' => &["#", "#", " ", " ", " "],
        '(' => &[" #", "# ", "# ", "# ", " #"],
        ')' => &["# ", " #", " #", " #", "# "],
        _ => return None,
    };
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_pattern(patterns: &[&str]) -> String {
        let mut out = String::new();
        for p in patterns {
            for c in p.chars() {
                out.push_str(if c == '#' { PX_ON } else { PX_OFF });
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn block_renders_ten() {
        let expected = from_pattern(&[
            " #  ###",
            "##  # #",
            " #  # #",
            " #  # #",
            "### ###",
        ]);
        assert_eq!(render_block("10"), expected);
    }

    #[test]
    fn block_colon_is_narrow() {
        let expected = from_pattern(&[" ", "#", " ", "#", " "]);
        assert_eq!(render_block(":"), expected);
    }

    #[test]
    fn block_is_five_rows_of_equal_width() {
        let rendered = render_block("1h 5m 30s");
        let lines: Vec<&str> = rendered.trim_end_matches('\n').split('\n').collect();
        assert_eq!(lines.len(), 5);
        let width = lines[0].chars().count();
        assert!(width > 0);
        for line in lines {
            assert_eq!(line.chars().count(), width);
        }
    }

    #[test]
    fn block_folds_case_and_skips_unknown() {
        assert_eq!(render_block("done"), render_block("DONE"));
        assert_eq!(render_block("a~b"), render_block("ab"));
        assert_eq!(render_block(""), "");
    }
}
