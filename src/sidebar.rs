use crate::types::Graph;
use crate::types::*;
use crate::ui::Painter;
impl Graph {
    pub(crate) fn write_side(&mut self, painter: &mut Painter) {
        let offset = std::mem::replace(&mut painter.offset, Pos::new(0.0, 0.0));
        let is_portrait = offset.x == offset.y && offset.x == 0.0;
        if is_portrait {
            painter.offset = Pos::new(0.0, self.screen.x as f32);
            painter.hline(self.screen.x as f32, 0.0, &self.axis_color);
        } else {
            painter.line_segment(
                [
                    Pos::new(0.0, self.screen.y as f32 - 1.0),
                    Pos::new(offset.x, self.screen.y as f32 - 1.0),
                ],
                1.0,
                &self.axis_color,
            );
            painter.vline(offset.x, self.screen.y as f32, &self.axis_color);
            painter.vline(0.0, self.screen.y as f32, &self.axis_color);
        }
        let delta = self.font_size * self.side_height;
        let t = if is_portrait {
            self.screen.y - self.screen.x
        } else {
            self.screen.y
        } as f32;
        let ti = (t / delta).round().max(1.0);
        self.text_scroll_pos.1 = (ti as usize + self.text_scroll_pos.0) - 1;
        let delta = t / ti;
        for i in 0..ti as usize {
            painter.hline(
                if is_portrait {
                    self.screen.x as f32
                } else {
                    offset.x
                },
                i as f32 * delta,
                &self.axis_color,
            )
        }
        if let (Some((a, b, _)), Some((_, y))) = (self.select, self.text_box) {
            painter.highlight(
                a as f32 * self.font_width + 4.0,
                y as f32 * delta + 1.0,
                b as f32 * self.font_width + 4.0,
                (y + 1) as f32 * delta,
                &self.select_color,
            )
        }
        self.display_names(painter, delta);
        if let Some(text_box) = self.text_box {
            let x = text_box.0 as f32 * self.font_width;
            let y = (text_box.1 as isize - self.text_scroll_pos.0 as isize) as f32 * delta;
            painter.line_segment(
                [Pos::new(x + 4.0, y), Pos::new(x + 4.0, y + delta)],
                1.0,
                &self.text_color,
            );
            painter.line_segment(
                [
                    Pos::new(offset.x - 1.0, y),
                    Pos::new(offset.x - 1.0, y + delta),
                ],
                1.0,
                &self.text_color,
            );
        }
        if is_portrait {
            painter.offset = Pos::new(0.0, 0.0)
        };
    }
    pub(crate) fn keybinds_side(&mut self, i: &InputState) -> bool {
        if self.mouse_held {
            return false;
        }
        let mut stop_keybinds = false;
        if let Some(mpos) = i.pointer_pos {
            let x = mpos.x - 4.0;
            let is_portrait = self.draw_offset.x == self.draw_offset.y && self.draw_offset.x == 0.0;
            let mpos = Vec2 { x, y: mpos.y } - self.draw_offset.to_vec();
            let delta = self.font_size * self.side_height;
            let new = (if is_portrait {
                mpos.y - self.screen.x
            } else {
                mpos.y
            } as f32
                / delta)
                .floor();
            let main_graph = if is_portrait {
                mpos.y < self.screen.x
            } else {
                mpos.x > 0.0
            };
            if i.pointer.unwrap_or(false) {
                if main_graph {
                    self.text_box = None
                } else if self.text_box.is_none() {
                    self.text_box = Some((0, 0))
                }
            }
            if !main_graph {
                if self.text_box.is_none() {
                    self.text_box = Some((0, 0));
                }
                if i.raw_scroll_delta.y < 0.0 {
                    let Some(mut text_box) = self.text_box else {
                        unreachable!()
                    };
                    text_box.1 += 1;
                    let n = self.get_name_count(text_box.1);
                    text_box.0 = text_box.0.min(n);
                    self.text_scroll_pos.0 += 1;
                    text_box.1 = self.expand_names(text_box.1);
                    self.text_box = Some(text_box);
                } else if i.raw_scroll_delta.y > 0.0 {
                    let Some(mut text_box) = self.text_box else {
                        unreachable!()
                    };
                    text_box.1 = text_box.1.saturating_sub(1);
                    let n = self.get_name_count(text_box.1);
                    text_box.0 = text_box.0.min(n);
                    self.text_box = Some(text_box);
                    self.text_scroll_pos.0 = self.text_scroll_pos.0.saturating_sub(1);
                }
            }
            if self.text_box.is_some() {
                stop_keybinds = true;
                if i.pointer.unwrap_or(false) {
                    let new = self.expand_names(new as usize);
                    let new = new + self.text_scroll_pos.0;
                    let x = ((x as f32 / self.font_width).round() as usize)
                        .min(self.get_name_count(new));
                    self.text_box = Some((x, new));
                    self.select = Some((x, x, None));
                }
            }
            if i.pointer.is_some() {
                if let Some((_, b)) = self.text_box {
                    let x =
                        ((x as f32 / self.font_width).round() as usize).min(self.get_name_count(b));
                    self.select_move(x);
                } else {
                    self.select = None;
                }
            }
            if i.pointer_right.is_some() {
                if let Some(last) = self.last_right_interact {
                    if let Some(new) = self.side_slider {
                        let delta = ((mpos.x - last.x) / 64.0).exp();
                        let name = self.get_name(new).to_string();
                        let mut body = |s: String| {
                            self.replace_name(new, s);
                            self.name_modified(Some(new));
                        };
                        if let Ok(f) = name.parse::<f64>() {
                            body((f * delta).to_string())
                        } else if let Some((a, b)) = name.rsplit_once('=')
                            && let Ok(f) = b.parse::<f64>()
                        {
                            body(format!("{}={}", a, f * delta))
                        }
                    }
                } else if i.pointer_right.unwrap() && mpos.x < 0.0 {
                    self.side_slider = Some(new as usize);
                } else {
                    self.side_slider = None
                }
                self.last_right_interact = Some(mpos)
            } else {
                self.side_slider = None;
                self.last_right_interact = None
            }
            if x < 0.0 && i.pointer.unwrap_or(false) {
                if let Some(n) = self
                    .blacklist_graphs
                    .iter()
                    .position(|&n| n == new as usize)
                {
                    self.blacklist_graphs.remove(n);
                    if self.index_to_name(new as usize, true).0.is_some() {
                        self.recalculate(Some(new as usize));
                    } else {
                        self.name_modified(Some(n));
                    }
                } else if let (Some(i), _) = self.index_to_name(new as usize, true) {
                    if !matches!(self.names[i].show, Show::None) {
                        self.blacklist_graphs.push(new as usize);
                        self.recalculate(Some(new as usize));
                    }
                } else {
                    self.blacklist_graphs.push(new as usize);
                    self.name_modified(Some(new as usize));
                }
            }
        }
        if !stop_keybinds {
            return false;
        }
        let Some(mut text_box) = self.text_box else {
            unreachable!()
        };
        for key in &i.keys_pressed {
            let down = |g: &Graph, text_box: &mut (usize, usize)| {
                text_box.1 += 1;
                if !matches!(g.menu, Menu::Normal | Menu::Side) && text_box.1 == g.get_name_len() {
                    text_box.1 -= 1;
                }
                text_box.0 = text_box.0.min(g.get_name_count(text_box.1))
            };
            let up = |g: &Graph, text_box: &mut (usize, usize)| {
                text_box.1 = text_box.1.saturating_sub(1);
                text_box.0 = text_box.0.min(g.get_name_count(text_box.1))
            };
            let modify = |g: &mut Graph, text_box: &mut (usize, usize), c: String| {
                if !g.modify_name(
                    text_box.1,
                    text_box.0,
                    if i.modifiers.shift {
                        c.to_ascii_uppercase()
                    } else {
                        c
                    },
                ) {
                    g.name_modified(Some(text_box.1));
                } else {
                    g.name_modified(None);
                }
                text_box.0 += 1;
            };
            match key.into() {
                KeyStr::Character(c) if !i.modifiers.ctrl && !i.modifiers.alt => {
                    let (a, b, _) = self.select.unwrap_or_default();
                    if a != b {
                        self.select = None;
                        let s = self.remove_str(text_box.1, a, b);
                        text_box.0 = a;
                        self.history_push(Change::Str(text_box, s, true));
                    }
                    self.history_push(Change::Char(text_box, c, false));
                    modify(self, &mut text_box, c.to_string())
                }
                KeyStr::Character(a) => match a {
                    'a' => self.select = Some((0, self.get_name_count(text_box.1), None)),
                    'z' if !self.history.is_empty()
                        && self.history_pos != self.history.len()
                        && matches!(self.menu, Menu::Side) =>
                    {
                        self.revert(
                            self.history.len() - self.history_pos - 1,
                            &mut text_box,
                            modify,
                            false,
                        );
                        self.name_modified(None);
                        self.history_pos += 1;
                    }
                    'y' if !self.history.is_empty()
                        && self.history_pos != 0
                        && matches!(self.menu, Menu::Side) =>
                    {
                        self.revert(
                            self.history.len() - self.history_pos,
                            &mut text_box,
                            modify,
                            true,
                        );
                        self.name_modified(None);
                        self.history_pos -= 1;
                    }
                    'c' => {
                        let (a, b, _) = self.select.unwrap_or_default();
                        if a != b {
                            let text = &self.get_name(text_box.1)[a..b].to_string();
                            self.clipboard.as_mut().unwrap().set_text(text)
                        }
                    }
                    'v' => {
                        let s = self.clipboard.as_mut().unwrap().get_text();
                        if !s.is_empty() {
                            let (a, b, _) = self.select.unwrap_or_default();
                            if a != b {
                                self.select = None;
                                let s = self.remove_str(text_box.1, a, b);
                                text_box.0 = a;
                                self.history_push(Change::Str(text_box, s, true));
                                self.name_modified(Some(text_box.1));
                            }
                            self.history_push(Change::Str(text_box, s.clone(), false));
                            for c in s.chars() {
                                modify(self, &mut text_box, c.to_string())
                            }
                        }
                    }
                    'x' => {
                        let (a, b, _) = self.select.unwrap_or_default();
                        if a != b {
                            self.select = None;
                            let text = self.remove_str(text_box.1, a, b);
                            self.clipboard.as_mut().unwrap().set_text(&text);
                            text_box.0 = a;
                            self.history_push(Change::Str(text_box, text, true));
                            self.name_modified(Some(text_box.1));
                        }
                    }
                    _ => {}
                },
                KeyStr::Named(key) => match key {
                    NamedKey::ArrowDown => {
                        self.select = None;
                        down(self, &mut text_box)
                    }
                    NamedKey::ArrowLeft => {
                        if self.select.map(|(a, b, _)| a == b).unwrap_or(true) {
                            self.select = Some((text_box.0, text_box.0, None))
                        }
                        if i.modifiers.ctrl {
                            let mut hit = false;
                            for (i, j) in self
                                .get_name(text_box.1)
                                .chars()
                                .take(text_box.0)
                                .collect::<Vec<char>>()
                                .into_iter()
                                .enumerate()
                                .rev()
                            {
                                if !j.is_alphanumeric() {
                                    if hit {
                                        hit = false;
                                        text_box.0 = i + 1;
                                        break;
                                    }
                                } else {
                                    hit = true;
                                }
                            }
                            if hit {
                                text_box.0 = 0
                            }
                        } else {
                            text_box.0 = text_box.0.saturating_sub(1);
                        }
                        if i.modifiers.shift {
                            self.select_move(text_box.0);
                        } else {
                            self.select = None;
                        }
                    }
                    NamedKey::ArrowRight => {
                        if self.select.map(|(a, b, _)| a == b).unwrap_or(true) {
                            self.select = Some((text_box.0, text_box.0, None))
                        }
                        if i.modifiers.ctrl {
                            let mut hit = false;
                            let s = self.get_name(text_box.1);
                            for (i, j) in s
                                .chars()
                                .skip((text_box.0 + 1).min(s.len() - 1))
                                .enumerate()
                            {
                                if !j.is_alphanumeric() {
                                    if hit {
                                        text_box.0 += i + 1;
                                        break;
                                    }
                                } else {
                                    hit = true;
                                }
                            }
                            if !hit {
                                text_box.0 = s.len()
                            }
                        } else {
                            text_box.0 = (text_box.0 + 1).min(self.get_name_count(text_box.1));
                        }
                        if i.modifiers.shift {
                            self.select_move(text_box.0);
                        } else {
                            self.select = None;
                        }
                    }
                    NamedKey::ArrowUp => {
                        self.select = None;
                        up(self, &mut text_box)
                    }
                    NamedKey::Tab => {
                        if i.modifiers.ctrl {
                            if i.modifiers.shift {
                                up(self, &mut text_box)
                            } else {
                                down(self, &mut text_box)
                            }
                        } else if let Some(get_word_bank) = &self.tab_complete {
                            let mut wait = false;
                            let mut word = String::new();
                            let mut count = 0;
                            let name = self.get_name(text_box.1);
                            for (i, c) in name[..text_box.0].chars().rev().enumerate() {
                                if !wait {
                                    if c.is_alphabetic()
                                        || matches!(
                                            c,
                                            '°' | '\''
                                                | '`'
                                                | '_'
                                                | '∫'
                                                | '$'
                                                | '¢'
                                                | '['
                                                | '('
                                                | '{'
                                        )
                                    {
                                        word.insert(0, c)
                                    } else if i == 0 {
                                        wait = true
                                    } else {
                                        break;
                                    }
                                }
                                if wait {
                                    if c == '(' || c == '{' {
                                        count -= 1;
                                    } else if c == ')' || c == '}' {
                                        count += 1;
                                    }
                                    if count == -1 {
                                        wait = false;
                                    }
                                }
                            }
                            let bank = get_word_bank(&word);
                            let mut new = word.clone();
                            if bank.is_empty() {
                                continue;
                            } else {
                                let bc = bank
                                    .iter()
                                    .map(|b| b.chars().collect::<Vec<char>>())
                                    .collect::<Vec<Vec<char>>>();
                                for (i, c) in bc[0][word.len()..].iter().enumerate() {
                                    if bc.len() == 1
                                        || bc[1..].iter().all(|w| {
                                            w.len() > word.len() + i && w[word.len() + i] == *c
                                        })
                                    {
                                        new.push(*c);
                                        if matches!(c, '(' | '{' | '[') {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            };
                            let new = new.chars().collect::<Vec<char>>();
                            let mut i = word.len();
                            let mut nc = name.chars().collect::<Vec<char>>();
                            while i < new.len() {
                                if nc.len() == i || nc[i] != new[i] {
                                    nc.insert(i, new[i])
                                }
                                i += 1;
                                text_box.0 += 1;
                            }
                            self.replace_name(text_box.1, nc.iter().collect::<String>());
                            self.name_modified(Some(text_box.1));
                        }
                    }
                    NamedKey::Backspace => {
                        let (a, b, _) = self.select.unwrap_or_default();
                        if a != b {
                            self.select = None;
                            let s = self.remove_str(text_box.1, a, b);
                            text_box.0 = a;
                            self.history_push(Change::Str(text_box, s, true));
                            self.name_modified(Some(text_box.1));
                        } else if text_box.0 != 0 {
                            let name = self.get_name(text_box.1).chars().collect::<Vec<char>>();
                            if i.modifiers.ctrl && !end_word(name[text_box.0 - 1]) {
                                for (i, c) in name[..text_box.0].iter().rev().enumerate() {
                                    if c.is_whitespace() || i + 1 == text_box.0 {
                                        let (a, b) = (text_box.0 - i - 1, text_box.0);
                                        let text = self.remove_str(text_box.1, a, b);
                                        text_box.0 -= i + 1;
                                        self.history_push(Change::Str(text_box, text, true));
                                        break;
                                    } else if end_word(*c) {
                                        let (a, b) = (text_box.0 - i, text_box.0);
                                        let text = self.remove_str(text_box.1, a, b);
                                        text_box.0 -= i;
                                        self.history_push(Change::Str(text_box, text, true));
                                        break;
                                    }
                                }
                            } else {
                                let c = self.remove_char(text_box.1, text_box.0 - 1);
                                text_box.0 -= 1;
                                self.history_push(Change::Char(text_box, c, true));
                            }
                            self.name_modified(Some(text_box.1));
                        } else if self.get_name(text_box.1).is_empty() {
                            let b = self.remove_name(text_box.1).unwrap_or(false);
                            self.history_push(Change::Line(text_box.1, b, true));
                            if text_box.1 > 0 {
                                text_box.1 = text_box.1.saturating_sub(1);
                                text_box.0 = self.get_name_count(text_box.1)
                            }
                            self.name_modified(None);
                        }
                    }
                    NamedKey::Enter => {
                        if i.modifiers.ctrl {
                            self.name_modified(None);
                            if i.modifiers.shift {
                                match self.index_to_name(text_box.1, true) {
                                    (Some(i), _) => {
                                        let mut n = self.names.remove(i);
                                        let mut v = std::mem::take(&mut n.vars);
                                        v.push(n.name);
                                        if let Some(n) = self.names.get_mut(i) {
                                            n.vars.splice(0..0, v);
                                        } else {
                                            self.names.push(Name {
                                                name: String::new(),
                                                vars: v,
                                                show: Show::None,
                                            })
                                        }
                                        down(self, &mut text_box);
                                        self.history_push(Change::Line(text_box.1, false, false));
                                    }
                                    (_, Some((i, j))) => {
                                        let name = Name {
                                            name: self.names[i].vars.remove(j),
                                            vars: self.names[i].vars.drain(..j).collect(),
                                            show: Show::None,
                                        };
                                        self.names.insert(i, name);
                                    }
                                    _ => {}
                                }
                            } else {
                                self.insert_name(text_box.1, true);
                                self.history_push(Change::Line(text_box.1, true, false));
                            }
                        } else {
                            self.insert_name(text_box.1 + 1, false);
                            down(self, &mut text_box);
                            self.history_push(Change::Line(text_box.1, false, false));
                        }
                        text_box.0 = 0;
                    }
                    NamedKey::Space => {
                        self.history_push(Change::Char(text_box, ' ', false));
                        modify(self, &mut text_box, " ".to_string())
                    }
                    NamedKey::Home => {
                        text_box.1 = 0;
                        text_box.0 = 0;
                    }
                    NamedKey::End => {
                        text_box.1 = self.get_name_len();
                        text_box.0 = self.get_name_count(text_box.1);
                    }
                    NamedKey::PageUp => {
                        text_box.1 = 0;
                        text_box.0 = 0;
                    }
                    NamedKey::PageDown => {
                        text_box.1 = self.get_name_len();
                        text_box.0 = self.get_name_count(text_box.1);
                    }
                    _ => {}
                },
            }
        }
        let d = self
            .text_scroll_pos
            .0
            .saturating_sub(self.get_name_len().saturating_sub(self.last_visible()));
        if d > 0 {
            self.text_scroll_pos.0 -= d;
            self.text_scroll_pos.1 -= d;
            text_box.1 -= d;
        }
        let (a, b) = self.text_scroll_pos;
        if !(a..=b).contains(&text_box.1) {
            let ta = text_box.1.abs_diff(a);
            let tb = text_box.1.abs_diff(b);
            if ta < tb {
                self.text_scroll_pos.0 -= ta
            } else {
                self.text_scroll_pos.0 += tb
            }
        }
        self.text_box = Some(text_box);
        text_box.1 = self.expand_names(text_box.1);
        #[cfg(feature = "serde")]
        if matches!(self.menu, Menu::Load) {
            self.load(text_box.1)
        }
        true
    }
    pub(crate) fn get_points(&self) -> Vec<(usize, String, Dragable)> {
        let mut pts = Vec::new();
        macro_rules! register {
            ($o: tt, $i: tt) => {
                let o = $o;
                let Some(sp) = o.rsplit_once('=') else {
                    $i += 1;
                    continue;
                };
                let mut v = sp.1.to_string();
                if let Ok(a) = v.parse() {
                    let s = sp.0;
                    if s != "y" {
                        if !matches!(self.graph_mode, GraphMode::Polar) {
                            let a = self.to_screen(a, 0.0).x;
                            pts.push(($i, s.to_string(), Dragable::X(a)));
                        }
                    } else {
                        let a = self.to_screen(0.0, a).y;
                        pts.push(($i, s.to_string(), Dragable::Y(a)));
                    }
                } else if v.len() >= 5 && v.pop().unwrap() == '}' && v.remove(0) == '{' {
                    if v.contains("{") {
                        v.pop();
                        for (k, v) in v.split("}").enumerate() {
                            let mut v = v.to_string();
                            if v.starts_with(",") {
                                v.remove(0);
                            }
                            v.remove(0);
                            let Some(s) = v.rsplit_once(',') else {
                                continue;
                            };
                            let (Ok(mut a), Ok(mut b)) = (s.0.parse::<f64>(), s.1.parse::<f64>())
                            else {
                                continue;
                            };
                            if matches!(self.graph_mode, GraphMode::Polar) {
                                let (s, c) = a.sin_cos();
                                (a, b) = (c * b, s * b);
                            }
                            pts.push((
                                $i,
                                sp.0.to_string(),
                                Dragable::Points((k, self.to_screen(a, b))),
                            ));
                        }
                    } else {
                        let Some(s) = v.rsplit_once(',') else {
                            $i += 1;
                            continue;
                        };
                        let (Ok(mut a), Ok(mut b)) = (s.0.parse::<f64>(), s.1.parse::<f64>())
                        else {
                            $i += 1;
                            continue;
                        };
                        if matches!(self.graph_mode, GraphMode::Polar) {
                            let (s, c) = a.sin_cos();
                            (a, b) = (c * b, s * b);
                        }
                        pts.push(($i, sp.0.to_string(), Dragable::Point(self.to_screen(a, b))));
                    }
                }
            };
        }
        let mut i = 0;
        for name in &self.names {
            for o in &name.vars {
                register!(o, i);
                i += 1;
            }
            let o = &name.name;
            register!(o, i);
            i += 1;
        }
        pts
    }
    pub(crate) fn revert<T>(
        &mut self,
        i: usize,
        text_box: &mut (usize, usize),
        modify: T,
        rev: bool,
    ) where
        T: Fn(&mut Graph, &mut (usize, usize), String),
    {
        let do_rev = |r: bool| -> bool { if rev { r } else { !r } };
        let s = std::mem::replace(&mut self.history[i], Change::None);
        match &s {
            &Change::Char((a, b), _, r) if do_rev(r) => {
                self.remove_char(b, a);
                *text_box = (a, b);
            }
            &Change::Char((a, b), c, _) => {
                *text_box = (a, b);
                modify(self, text_box, c.to_string());
            }
            Change::Str((a, b), s, r) if do_rev(*r) => {
                for _ in 0..s.len() {
                    self.remove_char(*b, *a);
                }
                *text_box = (*a, *b);
            }
            Change::Str((a, b), s, _) => {
                *text_box = (*a, *b);
                for c in s.chars() {
                    modify(self, text_box, c.to_string());
                }
            }
            &Change::Line(b, var, r) if do_rev(r) => {
                self.remove_name(b);
                let b = if var { b } else { b.saturating_sub(1) };
                *text_box = (self.get_name_count(b), b)
            }
            &Change::Line(b, var, _) => {
                self.insert_name(b, var);
                *text_box = (0, b);
            }
            Change::None => unreachable!(),
        }
        self.history[i] = s;
    }
    pub(crate) fn expand_names(&mut self, b: usize) -> usize {
        if !matches!(self.menu, Menu::Side | Menu::Normal) {
            return b.min(self.get_name_len().saturating_sub(1));
        }
        let a = self.get_name_len();
        for i in a..=b {
            self.insert_name(i, false);
        }
        for _ in (b + 1..self.get_name_len()).rev() {
            let n = self.names.last().unwrap();
            if n.name.is_empty() && n.vars.is_empty() {
                self.names.pop();
            } else {
                break;
            }
        }
        b
    }
    pub(crate) fn last_visible(&self) -> usize {
        match self.menu {
            Menu::Side | Menu::Normal => {
                let mut i: usize = 0;
                for n in self.names.iter().rev() {
                    if n.name.is_empty() && n.vars.is_empty() {
                        i += 1
                    } else {
                        break;
                    }
                }
                i + 1
            }
            #[cfg(feature = "serde")]
            Menu::Load => self.file_data.as_ref().unwrap().len() + 1,
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn display_names(&self, painter: &mut Painter, delta: f32) {
        match self.menu {
            Menu::Side | Menu::Normal => {
                let mut text = |s: &str, i: usize, color: (Option<Color>, Option<Color>)| {
                    match color {
                        (Some(a), Some(b)) => {
                            painter.line_segment(
                                [
                                    Pos::new(1.5, i as f32 * delta + 0.5),
                                    Pos::new(1.5, (i as f32 + 0.5) * delta),
                                ],
                                4.0,
                                &a,
                            );
                            painter.line_segment(
                                [
                                    Pos::new(1.5, (i as f32 + 0.5) * delta),
                                    Pos::new(1.5, (i + 1) as f32 * delta),
                                ],
                                4.0,
                                &b,
                            );
                        }
                        (Some(color), None) | (None, Some(color)) => {
                            painter.line_segment(
                                [
                                    Pos::new(1.5, i as f32 * delta + 0.5),
                                    Pos::new(1.5, (i + 1) as f32 * delta),
                                ],
                                4.0,
                                &color,
                            );
                        }
                        (None, None) => {}
                    }
                    self.text_color(
                        Pos::new(4.0, i as f32 * delta + delta / 2.0),
                        Align::LeftCenter,
                        s,
                        painter,
                    )
                };
                let mut j = 0;
                let mut i = 0;
                let mut k = 0;
                for n in self.names.iter() {
                    for v in n.vars.iter() {
                        if j != 0 {
                            j -= 1;
                            continue;
                        }
                        if i >= self.text_scroll_pos.0 {
                            text(
                                v,
                                i - self.text_scroll_pos.0,
                                (
                                    if self.blacklist_graphs.contains(&i) {
                                        Some(self.axis_color_light)
                                    } else {
                                        Some(self.axis_color)
                                    },
                                    None,
                                ),
                            );
                        }
                        i += 1;
                    }
                    if j != 0 {
                        j -= 1;
                        continue;
                    }
                    if !n.name.is_empty() {
                        if i >= self.text_scroll_pos.0 {
                            let real = if n.show.real() && !self.blacklist_graphs.contains(&i) {
                                Some(self.main_colors[k % self.main_colors.len()])
                            } else {
                                None
                            };
                            let imag = if n.show.imag() && !self.blacklist_graphs.contains(&i) {
                                Some(self.alt_colors[k % self.alt_colors.len()])
                            } else {
                                None
                            };
                            text(n.name.as_str(), i - self.text_scroll_pos.0, (real, imag));
                        }
                        k += 1;
                    }
                    i += 1;
                }
            }
            #[cfg(feature = "serde")]
            Menu::Load => {
                for (i, n) in self.file_data.as_ref().unwrap().iter().enumerate() {
                    self.text_color(
                        Pos::new(4.0, i as f32 * delta + delta / 2.0),
                        Align::LeftCenter,
                        &n.0,
                        painter,
                    )
                }
            }
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn get_name(&self, mut i: usize) -> &str {
        match self.menu {
            Menu::Side | Menu::Normal => {
                for name in &self.names {
                    if i < name.vars.len() {
                        return &name.vars[i];
                    }
                    i -= name.vars.len();
                    if i == 0 {
                        return &name.name;
                    }
                    i -= 1;
                }
                ""
            }
            #[cfg(feature = "serde")]
            Menu::Load => self
                .file_data
                .as_ref()
                .unwrap()
                .get(i)
                .map_or("", |(a, _, _)| a),
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn get_name_count(&self, mut i: usize) -> usize {
        match self.menu {
            Menu::Side | Menu::Normal => {
                for name in &self.names {
                    if i < name.vars.len() {
                        return name.vars[i].chars().count();
                    }
                    i -= name.vars.len();
                    if i == 0 {
                        return name.name.chars().count();
                    }
                    i -= 1;
                }
                0
            }
            #[cfg(feature = "serde")]
            Menu::Load => self
                .file_data
                .as_ref()
                .unwrap()
                .get(i)
                .map_or(0, |(a, _, _)| a.chars().count()),
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn get_mut_name(&mut self, mut i: usize) -> &mut String {
        match self.menu {
            Menu::Side | Menu::Normal => {
                for name in self.names.iter_mut() {
                    if i < name.vars.len() {
                        return &mut name.vars[i];
                    }
                    i -= name.vars.len();
                    if i == 0 {
                        return &mut name.name;
                    }
                    i -= 1;
                }
                unreachable!()
            }
            #[cfg(feature = "serde")]
            Menu::Load => &mut self.file_data.as_mut().unwrap()[i].0,
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn get_longest(&self) -> usize {
        match self.menu {
            //TODO make work with constant_eval
            Menu::Side | Menu::Normal => self
                .names
                .iter()
                .map(|n| {
                    n.name
                        .len()
                        .max(n.vars.iter().map(|v| v.len()).max().unwrap_or_default())
                })
                .max()
                .unwrap_or_default(),
            #[cfg(feature = "serde")]
            Menu::Load => self
                .file_data
                .as_ref()
                .unwrap()
                .iter()
                .map(|a| a.0.len())
                .max()
                .unwrap_or_default(),
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn modify_name(&mut self, i: usize, j: usize, char: String) -> bool {
        let s = self.get_mut_name(i);
        let is_empty = s.is_empty();
        let j = s.char_indices().nth(j).map(|(a, _)| a).unwrap_or(s.len());
        s.insert_str(j, &char);
        is_empty
    }
    pub(crate) fn replace_name(&mut self, i: usize, new: String) {
        *self.get_mut_name(i) = new;
    }
    pub(crate) fn remove_name(&mut self, mut i: usize) -> Option<bool> {
        match self.menu {
            Menu::Side | Menu::Normal => {
                if i != self.get_name_len() {
                    let l = self.names.len();
                    for (k, name) in self.names.iter_mut().enumerate() {
                        if i < name.vars.len() {
                            name.vars.remove(i);
                            return Some(true);
                        }
                        i -= name.vars.len();
                        if i == 0 {
                            if name.vars.is_empty() {
                                self.names.remove(k);
                            } else if l > k + 1 {
                                let v = self.names.remove(k).vars;
                                self.names[k].vars.splice(0..0, v);
                            }
                            return Some(false);
                        }
                        i -= 1;
                    }
                }
            }
            #[cfg(feature = "serde")]
            Menu::Load => {
                let d = self.file_data.as_mut().unwrap();
                d.remove(i);
                if self.save_num == Some(i) {
                    self.data.clear();
                    self.save_num = None
                }
                if d.is_empty() {
                    self.save_num = None;
                    self.menu = Menu::Side
                }
                return Some(false);
            }
            Menu::Settings => todo!(),
        }
        None
    }
    pub(crate) fn insert_name(&mut self, j: usize, var: bool) {
        match self.menu {
            Menu::Side | Menu::Normal => {
                if j == self.get_name_len() {
                    self.names.push(Name {
                        vars: if var { vec![String::new()] } else { Vec::new() },
                        name: String::new(),
                        show: Show::None,
                    })
                } else {
                    let mut i = j;
                    for (k, name) in self.names.iter_mut().enumerate() {
                        if i <= name.vars.len() && (i > 0 || var) {
                            name.vars.insert(i, String::new());
                            return;
                        }
                        i = i.saturating_sub(name.vars.len());
                        if i == 0 {
                            if var {
                                name.vars.push(String::new())
                            } else {
                                self.names.insert(
                                    k,
                                    Name {
                                        vars: Vec::new(),
                                        name: String::new(),
                                        show: Show::None,
                                    },
                                );
                            }
                            return;
                        }
                        i -= 1;
                    }
                }
            }
            #[cfg(feature = "serde")]
            Menu::Load => {
                let fd = self.file_data.as_mut().unwrap();
                fd.insert(j, fd[j - 1].clone())
            }
            Menu::Settings => todo!(),
        }
    }
    pub fn index_to_name(
        &self,
        mut i: usize,
        ignore_white: bool,
    ) -> (Option<usize>, Option<(usize, usize)>) {
        match self.menu {
            Menu::Side | Menu::Normal => {
                let mut j = 0;
                for (k, name) in self.names.iter().enumerate() {
                    if i < name.vars.len() {
                        return (None, Some((k - j, i)));
                    }
                    i -= name.vars.len();
                    if i == 0 {
                        return (Some(k - j), None);
                    }
                    i -= 1;
                    if !ignore_white && name.name.is_empty() {
                        j += 1;
                    }
                }
                unreachable!()
            }
            #[cfg(feature = "serde")]
            Menu::Load => (None, None),
            Menu::Settings => (None, None),
        }
    }
    pub(crate) fn remove_char(&mut self, i: usize, j: usize) -> char {
        let s = self.get_mut_name(i);
        let mut c = s.char_indices();
        let j = c.nth(j).unwrap().0;
        let k = c.next().map(|(n, _)| n).unwrap_or(s.len());
        s.drain(j..k).next().unwrap()
    }
    pub(crate) fn remove_str(&mut self, i: usize, j: usize, k: usize) -> String {
        let s = self.get_mut_name(i);
        let mut c = s.char_indices();
        let n = c.nth(j).unwrap().0;
        let k = c
            .nth(k.saturating_sub(j + 1))
            .map(|(n, _)| n)
            .unwrap_or(s.len());
        s.drain(n..k).collect()
    }
    pub(crate) fn get_name_len(&self) -> usize {
        match self.menu {
            Menu::Side | Menu::Normal => {
                let mut i = 0;
                for name in &self.names {
                    i += 1 + name.vars.len()
                }
                i
            }
            #[cfg(feature = "serde")]
            Menu::Load => self.file_data.as_ref().unwrap().len(),
            Menu::Settings => todo!(),
        }
    }
    pub(crate) fn history_push(&mut self, c: Change) {
        if !matches!(self.menu, Menu::Side) {
            return;
        }
        if !self.history.is_empty() {
            self.history.drain(self.history.len() - self.history_pos..);
            self.history_pos = 0;
        }
        self.history.push(c)
    }
    pub(crate) fn select_move(&mut self, x: usize) {
        let (Some((a, b, right)), Some((tx, _))) = (self.select.as_mut(), self.text_box.as_mut())
        else {
            return;
        };
        let da = x.abs_diff(*a);
        let db = x.abs_diff(*b);
        match da.cmp(&db) {
            std::cmp::Ordering::Less => {
                if da == 0 && *right == Some(true) {
                    *right = None;
                    *tx = x;
                    *b = x
                } else {
                    *right = Some(false);
                    *tx = x;
                    *a = x
                }
            }
            std::cmp::Ordering::Equal if x > *b => {
                *right = Some(true);
                *tx = x;
                *b = x
            }
            std::cmp::Ordering::Equal if x < *a => {
                *right = Some(false);
                *tx = x;
                *a = x
            }
            std::cmp::Ordering::Greater => {
                if db == 0 && *right == Some(false) {
                    *right = None;
                    *tx = x;
                    *a = x
                } else {
                    *right = Some(true);
                    *tx = x;
                    *b = x
                }
            }
            std::cmp::Ordering::Equal => {
                if let Some(right) = right {
                    if *right {
                        *tx = x;
                        *b = x
                    } else {
                        *tx = x;
                        *a = x
                    }
                }
            }
        }
    }
}
pub fn end_word(c: char) -> bool {
    matches!(
        c,
        '(' | '{'
            | '['
            | ')'
            | '}'
            | ']'
            | '+'
            | '-'
            | '*'
            | '/'
            | '^'
            | '<'
            | '='
            | '>'
            | '|'
            | '&'
            | '!'
            | '±'
            | '%'
            | ';'
            | ','
    )
}
