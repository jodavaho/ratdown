// Port of the pyfiglet rendering engine (loader + horizontal smushing),
// restricted to left-to-right rendering without width-based line wrapping.
// termdown falls back to plain text when the output does not fit anyway.
use std::collections::HashMap;

const SM_EQUAL: u32 = 1;
const SM_LOWLINE: u32 = 2;
const SM_HIERARCHY: u32 = 4;
const SM_PAIR: u32 = 8;
const SM_BIGX: u32 = 16;
const SM_HARDBLANK: u32 = 32;
const SM_KERN: u32 = 64;
const SM_SMUSH: u32 = 128;

struct Glyph {
    width: usize,
    lines: Vec<Vec<char>>,
}

pub struct FigletFont {
    pub height: usize,
    hard_blank: char,
    smush_mode: u32,
    chars: HashMap<u32, Glyph>,
}

impl FigletFont {
    pub fn parse(data: &str) -> Result<FigletFont, String> {
        let mut lines = data.lines();
        let header = lines.next().ok_or("empty font file")?;
        if !(header.starts_with("flf2") || header.starts_with("tlf2")) {
            return Err("not a valid figlet font".to_string());
        }
        let rest: String = header.chars().skip(5).collect();
        let toks: Vec<&str> = rest.split_whitespace().collect();
        if toks.len() < 6 {
            return Err("malformed font header".to_string());
        }
        let hard_blank = toks[0].chars().next().ok_or("missing hardblank")?;
        let height: usize = toks[1].parse().map_err(|_| "bad height in header")?;
        let old_layout: i32 = toks[4].parse().map_err(|_| "bad layout in header")?;
        let comment_lines: usize = toks[5].parse().map_err(|_| "bad comment count in header")?;
        let full_layout: Option<u32> = toks.get(7).and_then(|t| t.parse().ok());
        let smush_mode = match full_layout {
            Some(fl) => fl,
            None => {
                if old_layout == 0 {
                    SM_KERN
                } else if old_layout < 0 {
                    0
                } else {
                    ((old_layout as u32) & 31) | SM_SMUSH
                }
            }
        };
        for _ in 0..comment_lines {
            lines.next();
        }

        let read_glyph = |lines: &mut std::str::Lines| -> Result<Glyph, String> {
            let mut out: Vec<Vec<char>> = Vec::with_capacity(height);
            let mut width = 0;
            let mut endmark: Option<char> = None;
            for _ in 0..height {
                let raw = lines.next().ok_or("unexpected end of font file")?;
                if endmark.is_none() {
                    endmark = raw.trim_end().chars().last().or_else(|| raw.chars().last());
                }
                let mut line: Vec<char> = raw.chars().collect();
                if let Some(em) = endmark {
                    let mut stripped = 0;
                    while stripped < 2 && line.last() == Some(&em) {
                        line.pop();
                        stripped += 1;
                    }
                }
                width = width.max(line.len());
                out.push(line);
            }
            for l in &mut out {
                while l.len() < width {
                    l.push(' ');
                }
            }
            Ok(Glyph { width, lines: out })
        };

        let mut chars = HashMap::new();
        for code in 32u32..127 {
            let g = read_glyph(&mut lines)?;
            if g.width > 0 {
                chars.insert(code, g);
            }
        }
        // Extended characters: pyfiglet only honors hex-tagged code lines.
        loop {
            let Some(line) = lines.next() else { break };
            let tok = line.trim().split(' ').next().unwrap_or("");
            if tok.is_empty() {
                continue;
            }
            let hexpart = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X"));
            if let Some(hexpart) = hexpart {
                if let Ok(code) = u32::from_str_radix(hexpart, 16) {
                    match read_glyph(&mut lines) {
                        Ok(g) if g.width > 0 => {
                            chars.insert(code, g);
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
        }

        Ok(FigletFont {
            height,
            hard_blank,
            smush_mode,
            chars,
        })
    }

    pub fn render(&self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        let mut buffer: Vec<Vec<char>> = vec![Vec::new(); self.height];
        let mut prev_w = 0usize;
        for ch in text.chars() {
            let Some(g) = self.chars.get(&(ch as u32)) else {
                continue;
            };
            let cur_w = g.width;
            let max_smush = self.smush_amount(&buffer, g, prev_w, cur_w);
            for row in 0..self.height {
                let add_right = &g.lines[row];
                let add_left = &mut buffer[row];
                for i in 0..max_smush {
                    let idx = add_left.len() as isize - max_smush as isize + i as isize;
                    if idx >= 0 && (idx as usize) < add_left.len() {
                        let l = add_left[idx as usize];
                        let r = add_right[i];
                        add_left[idx as usize] =
                            self.smush_chars(l, r, prev_w, cur_w).unwrap_or(r);
                    }
                }
                add_left.extend(add_right.iter().skip(max_smush));
            }
            prev_w = cur_w;
        }
        let mut out = String::new();
        for row in &buffer {
            for &c in row {
                out.push(if c == self.hard_blank { ' ' } else { c });
            }
            out.push('\n');
        }
        out
    }

    fn smush_amount(&self, buffer: &[Vec<char>], g: &Glyph, prev_w: usize, cur_w: usize) -> usize {
        if self.smush_mode & (SM_SMUSH | SM_KERN) == 0 {
            return 0;
        }
        let mut max_smush = cur_w as isize;
        for row in 0..self.height {
            let ll = &buffer[row];
            let lr = &g.lines[row];

            let mut rstrip_len = ll.len();
            while rstrip_len > 0 && ll[rstrip_len - 1] == ' ' {
                rstrip_len -= 1;
            }
            let mut linebd = rstrip_len as isize - 1;
            if linebd < 0 {
                linebd = 0;
            }
            let ch1: Option<char> = if (linebd as usize) < ll.len() {
                Some(ll[linebd as usize])
            } else {
                linebd = 0;
                None
            };

            let lstrip = lr.iter().take_while(|&&c| c == ' ').count();
            let (charbd, ch2): (usize, Option<char>) = if lstrip < lr.len() {
                (lstrip, Some(lr[lstrip]))
            } else {
                (lr.len(), None)
            };

            let mut amt = charbd as isize + ll.len() as isize - 1 - linebd;
            match ch1 {
                None | Some(' ') => amt += 1,
                Some(c1) => {
                    if let Some(c2) = ch2 {
                        if self.smush_chars(c1, c2, prev_w, cur_w).is_some() {
                            amt += 1;
                        }
                    }
                }
            }
            if amt < max_smush {
                max_smush = amt;
            }
        }
        max_smush.max(0) as usize
    }

    fn smush_chars(&self, l: char, r: char, prev_w: usize, cur_w: usize) -> Option<char> {
        if l.is_whitespace() {
            return Some(r);
        }
        if r.is_whitespace() {
            return Some(l);
        }
        if prev_w < 2 || cur_w < 2 {
            return None;
        }
        if self.smush_mode & SM_SMUSH == 0 {
            return None;
        }
        if self.smush_mode & 63 == 0 {
            // universal smushing: the later character wins, hardblanks lose
            if l == self.hard_blank {
                return Some(r);
            }
            if r == self.hard_blank {
                return Some(l);
            }
            return Some(r);
        }
        if self.smush_mode & SM_HARDBLANK != 0 && l == self.hard_blank && r == self.hard_blank {
            return Some(l);
        }
        if l == self.hard_blank || r == self.hard_blank {
            return None;
        }
        if self.smush_mode & SM_EQUAL != 0 && l == r {
            return Some(l);
        }
        let mut smushes: Vec<(&str, &str)> = Vec::new();
        if self.smush_mode & SM_LOWLINE != 0 {
            smushes.push(("_", "|/\\[]{}()<>"));
        }
        if self.smush_mode & SM_HIERARCHY != 0 {
            smushes.extend([
                ("|", "|/\\[]{}()<>"),
                ("\\/", "[]{}()<>"),
                ("[]", "{}()<>"),
                ("{}", "()<>"),
                ("()", "<>"),
            ]);
        }
        for (a, b) in smushes {
            if a.contains(l) && b.contains(r) {
                return Some(r);
            }
            if a.contains(r) && b.contains(l) {
                return Some(l);
            }
        }
        if self.smush_mode & SM_PAIR != 0 {
            for pair in [[l, r], [r, l]] {
                if pair == ['[', ']'] || pair == ['{', '}'] || pair == ['(', ')'] {
                    return Some('|');
                }
            }
        }
        if self.smush_mode & SM_BIGX != 0 {
            if l == '/' && r == '\\' {
                return Some('|');
            }
            if l == '\\' && r == '/' {
                return Some('Y');
            }
            if l == '>' && r == '<' {
                return Some('X');
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn univers() -> FigletFont {
        FigletFont::parse(include_str!("../fonts/univers.flf")).unwrap()
    }

    fn assert_matches_pyfiglet(rendered: &str, fixture: &str) {
        let got: Vec<String> = rendered
            .trim_end_matches('\n')
            .split('\n')
            .map(|l| l.trim_end().to_string())
            .collect();
        let want: Vec<String> = fixture
            .trim_end_matches('\n')
            .split('\n')
            .map(|l| l.trim_end().to_string())
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn header_parsed() {
        let f = univers();
        assert_eq!(f.height, 11);
        assert_eq!(f.smush_mode, 128);
        assert_eq!(f.hard_blank, '$');
    }

    #[test]
    fn render_10_matches_pyfiglet() {
        assert_matches_pyfiglet(
            &univers().render("10"),
            include_str!("../tests/data/univers_10.txt"),
        );
    }

    #[test]
    fn render_1h5m_matches_pyfiglet() {
        assert_matches_pyfiglet(
            &univers().render("1h 5m"),
            include_str!("../tests/data/univers_1h5m.txt"),
        );
    }

    #[test]
    fn render_colon_matches_pyfiglet() {
        assert_matches_pyfiglet(
            &univers().render("0:12"),
            include_str!("../tests/data/univers_colon.txt"),
        );
    }

    #[test]
    fn render_long_matches_pyfiglet() {
        assert_matches_pyfiglet(
            &univers().render("1d 23h 45m 6s"),
            include_str!("../tests/data/univers_long.txt"),
        );
    }
}
