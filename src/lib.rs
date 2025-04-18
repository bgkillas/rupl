pub mod types;
mod ui;
use crate::types::*;
use crate::ui::Painter;
use rayon::slice::ParallelSliceMut;
use std::f64::consts::{PI, TAU};
fn is_3d(data: &[GraphType]) -> bool {
    data.iter()
        .any(|c| matches!(c, GraphType::Width3D(_, _, _, _, _) | GraphType::Coord3D(_)))
}
//TODO all keys should be optional an settings
//TODO 2d logscale
//TODO labels
//TODO scale axis
impl Graph {
    pub fn new(data: Vec<GraphType>, is_complex: bool, start: f64, end: f64) -> Self {
        let typeface = skia_safe::FontMgr::default()
            .new_from_data(include_bytes!("../terminus.otb"), None)
            .unwrap();
        let font = skia_safe::Font::new(typeface, 16.0);
        Self {
            is_3d: is_3d(&data),
            data,
            cache: None,
            #[cfg(feature = "skia")]
            font,
            bound: Vec2::new(start, end),
            offset3d: Vec3::splat(0.0),
            offset: Vec2::splat(0.0),
            angle: Vec2::splat(PI / 6.0),
            slice: 0,
            switch: false,
            mult: 1.0,
            is_complex,
            show: Show::Complex,
            ignore_bounds: false,
            zoom: 1.0,
            zoom3d: 1.0,
            mouse_held: false,
            screen: Vec2::splat(0.0),
            screen_offset: Vec2::splat(0.0),
            delta: 0.0,
            show_box: true,
            log_scale: false,
            view_x: true,
            color_depth: DepthColor::None,
            box_size: 3.0f64.sqrt(),
            anti_alias: true,
            lines: Lines::Lines,
            buffer: Vec::new(),
            domain_alternate: true,
            var: Vec2::new(start, end),
            last_interact: None,
            main_colors: vec![
                Color::new(255, 85, 85),
                Color::new(85, 85, 255),
                Color::new(255, 85, 255),
                Color::new(85, 255, 85),
                Color::new(85, 255, 255),
                Color::new(255, 255, 85),
            ],
            alt_colors: vec![
                Color::new(170, 0, 0),
                Color::new(0, 0, 170),
                Color::new(170, 0, 170),
                Color::new(0, 170, 0),
                Color::new(0, 170, 170),
                Color::new(170, 170, 0),
            ],
            axis_color: Color::splat(0),
            axis_color_light: Color::splat(220),
            text_color: Color::splat(0),
            background_color: Color::splat(255),
            mouse_position: None,
            mouse_moved: false,
            scale_axis: false,
            disable_lines: false,
            disable_axis: false,
            disable_coord: false,
            graph_mode: GraphMode::Normal,
            prec: 1.0,
            recalculate: false,
            ruler_pos: None,
            cos_phi: 0.0,
            sin_phi: 0.0,
            cos_theta: 0.0,
            sin_theta: 0.0,
            keybinds: Keybinds::default(),
        }
    }
    pub fn set_data(&mut self, data: Vec<GraphType>) {
        self.data = data;
        self.cache = None;
    }
    pub fn clear_data(&mut self) {
        self.data.clear();
        self.cache = None;
    }
    pub fn reset_3d(&mut self) {
        self.is_3d = is_3d(&self.data);
    }
    pub fn set_mode(&mut self, mode: GraphMode) {
        match mode {
            GraphMode::DomainColoring | GraphMode::Slice => self.is_3d = false,
            _ => {
                self.is_3d = is_3d(&self.data);
            }
        }
        self.graph_mode = mode;
    }
    #[cfg(feature = "egui")]
    pub fn update(&mut self, ctx: &egui::Context, no_repaint: bool) -> UpdateResult {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(self.background_color.to_col()))
            .show(ctx, |ui| self.plot_main(ctx, ui, no_repaint));
        self.update_res(no_repaint)
    }
    #[cfg(feature = "skia")]
    pub fn update(
        &mut self,
        no_repaint: bool,
        buffer: &mut softbuffer::Buffer<
            std::rc::Rc<winit::window::Window>,
            std::rc::Rc<winit::window::Window>,
        >,
        width: u32,
        height: u32,
    ) -> UpdateResult {
        self.plot_main(no_repaint, width, height, buffer);
        self.update_res(no_repaint)
    }
    fn update_res(&mut self, no_repaint: bool) -> UpdateResult {
        if self.recalculate && !no_repaint {
            self.recalculate = false;
            if is_3d(&self.data) {
                match self.graph_mode {
                    GraphMode::Normal => UpdateResult::Width3D(
                        self.bound.x + self.offset3d.x,
                        self.bound.x - self.offset3d.y,
                        self.bound.y + self.offset3d.x,
                        self.bound.y - self.offset3d.y,
                        Prec::Mult(self.prec),
                    ),
                    GraphMode::DomainColoring => {
                        let c = self.to_coord(Pos::new(0.0, 0.0));
                        let cf = self.to_coord(self.screen.to_pos());
                        UpdateResult::Width3D(
                            c.0,
                            c.1,
                            cf.0,
                            cf.1,
                            Prec::Dimension(
                                (self.screen.x * self.prec) as usize,
                                (self.screen.y * self.prec) as usize,
                            ),
                        )
                    }
                    GraphMode::Slice => {
                        let c = self.to_coord(Pos::new(0.0, 0.0));
                        let cf = self.to_coord(self.screen.to_pos());
                        if self.view_x {
                            UpdateResult::Width3D(
                                c.0,
                                self.bound.x,
                                cf.0,
                                self.bound.y,
                                Prec::Slice(self.prec, self.view_x, self.slice),
                            )
                        } else {
                            UpdateResult::Width3D(
                                self.bound.x,
                                c.0,
                                self.bound.y,
                                cf.0,
                                Prec::Slice(self.prec, self.view_x, self.slice),
                            )
                        }
                    }
                    GraphMode::SliceFlatten => {
                        if self.view_x {
                            UpdateResult::Width3D(
                                self.var.x,
                                self.bound.x,
                                self.var.y,
                                self.bound.y,
                                Prec::Slice(self.prec, self.view_x, self.slice),
                            )
                        } else {
                            UpdateResult::Width3D(
                                self.bound.x,
                                self.var.x,
                                self.bound.y,
                                self.var.y,
                                Prec::Slice(self.prec, self.view_x, self.slice),
                            )
                        }
                    }
                    GraphMode::SliceDepth => {
                        if self.view_x {
                            UpdateResult::Width3D(
                                self.bound.x - self.offset3d.z,
                                self.bound.x,
                                self.bound.y - self.offset3d.z,
                                self.bound.y,
                                Prec::Slice(self.prec, self.view_x, self.slice),
                            )
                        } else {
                            UpdateResult::Width3D(
                                self.bound.x,
                                self.bound.x - self.offset3d.z,
                                self.bound.y,
                                self.bound.y - self.offset3d.z,
                                Prec::Slice(self.prec, self.view_x, self.slice),
                            )
                        }
                    }

                    _ => UpdateResult::None,
                }
            } else if self.graph_mode == GraphMode::Depth {
                UpdateResult::Width(
                    self.bound.x - self.offset3d.z,
                    self.bound.y - self.offset3d.z,
                    Prec::Mult(self.prec),
                )
            } else if !self.is_3d {
                if self.graph_mode == GraphMode::Flatten {
                    UpdateResult::Width(self.var.x, self.var.y, Prec::Mult(self.prec))
                } else {
                    let c = self.to_coord(Pos::new(0.0, 0.0));
                    let cf = self.to_coord(self.screen.to_pos());
                    UpdateResult::Width(c.0, cf.0, Prec::Mult(self.prec))
                }
            } else {
                UpdateResult::None
            }
        } else {
            UpdateResult::None
        }
    }
    #[cfg(feature = "egui")]
    fn plot_main(&mut self, ctx: &egui::Context, ui: &egui::Ui, no_repaint: bool) {
        if !no_repaint {
            self.keybinds(ui);
            if self.recalculate {
                return;
            }
        }
        let mut painter = Painter::new(ui);
        let rect = ctx.available_rect();
        self.screen = Vec2::new(rect.width() as f64, rect.height() as f64);
        self.delta = if self.is_3d {
            self.screen.x.min(self.screen.y)
        } else {
            self.screen.x
        } / (self.bound.y - self.bound.x);
        let t = Vec2::new(
            self.screen.x / 2.0 - (self.delta * (self.bound.x + self.bound.y) / 2.0),
            self.screen.y / 2.0,
        );
        if t != self.screen_offset {
            if self.graph_mode == GraphMode::DomainColoring {
                self.recalculate = true;
            }
            self.screen_offset = t;
        }
        if !self.is_3d {
            if self.graph_mode != GraphMode::DomainColoring {
                self.write_axis(&mut painter);
                self.plot(&mut painter, ui);
            } else {
                self.plot(&mut painter, ui);
                self.write_axis(&mut painter);
            }
        } else {
            (self.sin_phi, self.cos_phi) = self.angle.x.sin_cos();
            (self.sin_theta, self.cos_theta) = self.angle.y.sin_cos();
            self.plot(&mut painter, ui);
            self.write_axis_3d(&mut painter);
            self.buffer.par_sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            for (_, a, c) in self.buffer.drain(..) {
                match a {
                    Draw::Line(a, b, t) => {
                        painter.line_segment([a, b], t, &c);
                    }
                    Draw::Point(a) => {
                        painter.rect_filled(a, &c);
                    }
                }
            }
        }
        if !self.is_3d {
            self.write_coord(&mut painter);
        } else {
            self.write_angle(&mut painter);
        }
        painter.save()
    }
    #[cfg(feature = "skia")]
    fn plot_main(
        &mut self,
        no_repaint: bool,
        width: u32,
        height: u32,
        buffer: &mut softbuffer::Buffer<
            std::rc::Rc<winit::window::Window>,
            std::rc::Rc<winit::window::Window>,
        >,
    ) {
        if !no_repaint {
            //self.keybinds(ui); TODO
            if self.recalculate {
                return;
            }
        }
        let mut painter = Painter::new(width, height, self.background_color, self.font.clone());
        self.screen = Vec2::new(width as f64, height as f64);
        self.delta = if self.is_3d {
            self.screen.x.min(self.screen.y)
        } else {
            self.screen.x
        } / (self.bound.y - self.bound.x);
        let t = Vec2::new(
            self.screen.x / 2.0 - (self.delta * (self.bound.x + self.bound.y) / 2.0),
            self.screen.y / 2.0,
        );
        if t != self.screen_offset {
            if self.graph_mode == GraphMode::DomainColoring {
                self.recalculate = true;
            }
            self.screen_offset = t;
        }
        if !self.is_3d {
            if self.graph_mode != GraphMode::DomainColoring {
                self.write_axis(&mut painter);
                self.plot(&mut painter);
            } else {
                self.plot(&mut painter);
                self.write_axis(&mut painter);
            }
        } else {
            (self.sin_phi, self.cos_phi) = self.angle.x.sin_cos();
            (self.sin_theta, self.cos_theta) = self.angle.y.sin_cos();
            self.plot(&mut painter);
            self.write_axis_3d(&mut painter);
            self.buffer.par_sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            for (_, a, c) in self.buffer.drain(..) {
                match a {
                    Draw::Line(a, b, t) => {
                        painter.line_segment([a, b], t, &c);
                    }
                    Draw::Point(a) => {
                        painter.rect_filled(a, &c);
                    }
                }
            }
        }
        if !self.is_3d {
            self.write_coord(&mut painter);
        } else {
            self.write_angle(&mut painter);
        }
        painter.save(buffer)
    }
    fn write_coord(&self, painter: &mut Painter) {
        if self.mouse_moved {
            if let Some(pos) = self.mouse_position {
                let p = self.to_coord(pos);
                if !self.disable_coord {
                    let s = if self.graph_mode == GraphMode::DomainColoring {
                        if let GraphType::Width3D(data, sx, sy, ex, ey) = &self.data[0] {
                            let len = data.len().isqrt();
                            let i = ((p.0 - sx) / (ex - sx) * len as f64).round() as usize;
                            let j = ((p.1 - sy) / (ey - sy) * len as f64).round() as usize;
                            let ind = i + len * j;
                            if ind < data.len() {
                                let (x, y) = data[ind].to_options();
                                let (x, y) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
                                format!(
                                    "{:e}\n{:e}\n{:e}\n{:e}\n{:e}\n{}",
                                    p.0,
                                    p.1,
                                    x,
                                    y,
                                    x.hypot(y),
                                    y.atan2(x)
                                )
                            } else {
                                format!("{:e}\n{:e}", p.0, p.1)
                            }
                        } else {
                            format!("{:e}\n{:e}", p.0, p.1)
                        }
                    } else {
                        format!("{:e}\n{:e}", p.0, p.1)
                    };
                    painter.text(
                        Pos::new(0.0, self.screen.y as f32),
                        Align::LeftBottom,
                        s,
                        &self.text_color,
                    );
                }
                if let Some(ps) = self.ruler_pos {
                    let dx = p.0 - ps.x;
                    let dy = p.1 - ps.y;
                    painter.text(
                        self.screen.to_pos(),
                        Align::RightBottom,
                        format!(
                            "{:e}\n{:e}\n{:e}\n{}",
                            dx,
                            dy,
                            (dx * dx + dy * dy).sqrt(),
                            dy.atan2(dx) * 360.0 / TAU
                        ),
                        &self.text_color,
                    );
                    painter.line_segment([pos, self.to_screen(ps.x, ps.y)], 1.0, &self.axis_color);
                }
            }
        }
    }
    fn write_angle(&self, painter: &mut Painter) {
        if !self.disable_coord {
            painter.text(
                Pos::new(0.0, self.screen.y as f32),
                Align::LeftBottom,
                format!(
                    "{}\n{}",
                    (self.angle.x / TAU * 360.0).round(),
                    ((0.25 - self.angle.y / TAU) * 360.0)
                        .round()
                        .rem_euclid(360.0),
                ),
                &self.text_color,
            );
        }
    }
    fn to_screen(&self, x: f64, y: f64) -> Pos {
        let s = self.screen.x / (self.bound.y - self.bound.x);
        let ox = self.screen_offset.x + self.offset.x;
        let oy = self.screen_offset.y + self.offset.y;
        Pos::new(
            ((x * s + ox) * self.zoom) as f32,
            ((oy - y * s) * self.zoom) as f32,
        )
    }
    fn to_coord(&self, p: Pos) -> (f64, f64) {
        let ox = self.offset.x + self.screen_offset.x;
        let oy = self.offset.y + self.screen_offset.y;
        let s = (self.bound.y - self.bound.x) / self.screen.x;
        let x = (p.x as f64 / self.zoom - ox) * s;
        let y = (oy - p.y as f64 / self.zoom) * s;
        (x, y)
    }
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "egui")]
    fn draw_point(
        &self,
        painter: &mut Painter,
        ui: &egui::Ui,
        x: f64,
        y: f64,
        color: &Color,
        last: Option<Pos>,
    ) -> Option<Pos> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let pos = self.to_screen(x, y);
        if !matches!(self.lines, Lines::Lines)
            && pos.x > -2.0
            && pos.x < self.screen.x as f32 + 2.0
            && pos.y > -2.0
            && pos.y < self.screen.y as f32 + 2.0
        {
            painter.rect_filled(pos, color);
        }
        if !matches!(self.lines, Lines::Points) {
            if let Some(last) = last {
                if ui.is_rect_visible(egui::Rect::from_points(&[last.to_pos2(), pos.to_pos2()])) {
                    painter.line_segment([last, pos], 1.0, color);
                }
            }
            Some(pos)
        } else {
            None
        }
    }
    #[cfg(feature = "skia")]
    fn draw_point(
        &self,
        painter: &mut Painter,
        x: f64,
        y: f64,
        color: &Color,
        last: Option<Pos>,
    ) -> Option<Pos> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let pos = self.to_screen(x, y);
        if !matches!(self.lines, Lines::Lines)
            && pos.x > -2.0
            && pos.x < self.screen.x as f32 + 2.0
            && pos.y > -2.0
            && pos.y < self.screen.y as f32 + 2.0
        {
            painter.rect_filled(pos, color);
        }
        if !matches!(self.lines, Lines::Points) {
            if let Some(last) = last {
                painter.line_segment([last, pos], 1.0, color);
            }
            Some(pos)
        } else {
            None
        }
    }
    fn write_axis(&self, painter: &mut Painter) {
        if self.scale_axis {
            let c = self.to_coord(Pos::new(0.0, 0.0));
            let cf = self.to_coord(self.screen.to_pos());
            let r = self.zoom.recip() / 2.0;
            let stx = (c.0 / r).round() * r;
            let sty = (c.1 / r).round() * r;
            let enx = (cf.0 / r).round() * r;
            let eny = (cf.1 / r).round() * r;
            if !stx.is_finite() || !sty.is_finite() || !enx.is_finite() || !eny.is_finite() {
                return;
            }
            let s: isize = 0;
            let f = ((enx - stx) / r).abs() as isize;
            let sy = ((eny - sty) / r).abs() as isize;
            let sf: isize = 0;
            if !self.disable_lines && self.graph_mode != GraphMode::DomainColoring {
                for i in s.saturating_sub(1)..=f.saturating_add(1) {
                    for j in -2..2 {
                        if j != 0 {
                            let x = self.to_screen(stx + r * (i as f64 + j as f64 / 4.0), 0.0).x;
                            painter.vline(x, self.screen.y as f32, 1.0, &self.axis_color_light);
                        }
                    }
                }
                for i in sf.saturating_sub(1)..=sy.saturating_add(1) {
                    for j in -2..2 {
                        if j != 0 {
                            let y = self.to_screen(0.0, sty - r * (i as f64 + j as f64 / 4.0)).y;
                            painter.hline(self.screen.x as f32, y, 1.0, &self.axis_color_light);
                        }
                    }
                }
            }
            for i in s..=f {
                let x = self.to_screen(stx + r * i as f64, 0.0).x;
                painter.vline(x, self.screen.y as f32, 1.0, &self.axis_color);
            }
            for i in sf..=sy {
                let y = self.to_screen(0.0, sty - r * i as f64).y;
                painter.hline(self.screen.x as f32, y, 1.0, &self.axis_color);
            }
            if !self.disable_axis {
                let y = if sty - r * (sy as f64) < 0.0 && sty - r * (sf as f64) > 0.0 {
                    self.to_screen(0.0, 0.0).y
                } else {
                    0.0
                };
                for j in s.saturating_sub(1)..=f {
                    let x = self.to_screen(stx + r * j as f64, 0.0).x;
                    painter.text(
                        Pos::new(x, y),
                        Align::LeftTop,
                        format!("{:e}", stx + r * j as f64),
                        &self.text_color,
                    );
                }
                let x = if stx + r * (s as f64) < 0.0 && stx + r * (f as f64) > 0.0 {
                    self.to_screen(0.0, 0.0).x
                } else {
                    0.0
                };
                for j in sf..=sy.saturating_add(1) {
                    let y = self.to_screen(0.0, sty - r * j as f64).y;
                    painter.text(
                        Pos::new(x, y),
                        Align::LeftTop,
                        format!("{:e}", sty - r * j as f64),
                        &self.text_color,
                    );
                }
            }
        } else {
            let c = self.to_coord(Pos::new(0.0, 0.0));
            let cf = self.to_coord(self.screen.to_pos());
            let s = c.0.ceil() as isize;
            let f = cf.0.floor() as isize;
            let sy = c.1.floor() as isize;
            let sf = cf.1.ceil() as isize;
            if !self.disable_lines && self.graph_mode != GraphMode::DomainColoring {
                let n = (self.zoom.round() * 4.0) as isize;
                let minor = if self.zoom < 1.0 {
                    self.zoom.log2().floor() as isize + 3
                } else if n < 0 || (n as usize).is_power_of_two() {
                    n
                } else {
                    (n as usize).next_power_of_two() as isize
                };
                if minor > 0 {
                    for i in s.saturating_sub(1)..=f.saturating_add(1) {
                        let s = self.screen.x / (self.bound.y - self.bound.x);
                        let ox = self.screen_offset.x + self.offset.x;
                        let n = (((-1.0 / self.zoom - ox) / s - i as f64) * 2.0 * minor as f64)
                            .ceil() as isize;
                        let m = ((((self.screen.x + 1.0) / self.zoom - ox) / s - i as f64)
                            * 2.0
                            * minor as f64)
                            .floor() as isize;
                        for j in n..=m {
                            if j != 0 {
                                let x = self
                                    .to_screen(i as f64 + j as f64 / (2.0 * minor as f64), 0.0)
                                    .x;
                                painter.vline(x, self.screen.y as f32, 1.0, &self.axis_color_light);
                            }
                        }
                    }
                    for i in sf.saturating_sub(1)..=sy.saturating_add(1) {
                        let s = self.screen.x / (self.bound.y - self.bound.x);
                        let oy = self.screen_offset.y + self.offset.y;
                        let n = (((oy + 1.0 / self.zoom) / s - i as f64) * 2.0 * minor as f64)
                            .ceil() as isize;
                        let m = (((oy - (self.screen.y + 1.0) / self.zoom) / s - i as f64)
                            * 2.0
                            * minor as f64)
                            .floor() as isize;
                        for j in m..=n {
                            if j != 0 {
                                let y = self
                                    .to_screen(0.0, i as f64 + j as f64 / (2.0 * minor as f64))
                                    .y;
                                painter.hline(self.screen.x as f32, y, 1.0, &self.axis_color_light);
                            }
                        }
                    }
                }
            }
            for i in if self.zoom > 2.0f64.powi(-6) {
                s..=f
            } else {
                0..=0
            } {
                let is_center = i == 0;
                if !self.disable_lines || (is_center && !self.disable_axis) {
                    let x = self.to_screen(i as f64, 0.0).x;
                    painter.vline(
                        x,
                        self.screen.y as f32,
                        if is_center { 2.0 } else { 1.0 },
                        &self.axis_color,
                    );
                }
            }
            for i in if self.zoom > 2.0f64.powi(-6) {
                sf..=sy
            } else {
                0..=0
            } {
                let is_center = i == 0;
                if (!self.disable_lines && (is_center || self.zoom > 2.0f64.powi(-6)))
                    || (is_center && !self.disable_axis)
                {
                    let y = self.to_screen(0.0, i as f64).y;
                    painter.hline(
                        self.screen.x as f32,
                        y,
                        if is_center { 2.0 } else { 1.0 },
                        &self.axis_color,
                    );
                }
            }
            if !self.disable_axis && self.zoom > 2.0f64.powi(-6) {
                let y = if (sf..=sy).contains(&0) {
                    self.to_screen(0.0, 0.0).y
                } else {
                    0.0
                };
                for j in s.saturating_sub(1)..=f {
                    let x = self.to_screen(j as f64, 0.0).x;
                    painter.text(
                        Pos::new(x, y),
                        Align::LeftTop,
                        j.to_string(),
                        &self.text_color,
                    );
                }
                let x = if (s..=f).contains(&0) {
                    self.to_screen(0.0, 0.0).x
                } else {
                    0.0
                };
                for j in sf..=sy.saturating_add(1) {
                    let y = self.to_screen(0.0, j as f64).y;
                    painter.text(
                        Pos::new(x, y),
                        Align::LeftTop,
                        j.to_string(),
                        &self.text_color,
                    );
                }
            }
        }
    }
    fn vec3_to_pos_depth(&self, p: Vec3) -> (Pos, f32) {
        let x1 = p.x * self.cos_phi + p.y * self.sin_phi;
        let y1 = -p.x * self.sin_phi + p.y * self.cos_phi;
        let z2 = -p.z * self.cos_theta - y1 * self.sin_theta;
        let d = p.z * self.sin_theta - y1 * self.cos_theta;
        let s = self.delta / self.box_size;
        let x = (x1 * s + self.screen.x / 2.0) as f32;
        let y = (z2 * s + self.screen.y / 2.0) as f32;
        (
            Pos::new(x, y),
            (d / ((self.bound.y - self.bound.x) * 3.0f64.sqrt()) + 0.5) as f32,
        )
    }
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    fn draw_point_3d(
        &self,
        x: f64,
        y: f64,
        z: f64,
        color: &Color,
        a: Option<((Pos, f32), Vec3, bool)>,
        b: Option<((Pos, f32), Vec3, bool)>,
    ) -> (Option<((Pos, f32), Vec3, bool)>, Vec<(f32, Draw, Color)>) {
        let mut draws = Vec::with_capacity(4);
        let x = x - self.offset3d.x;
        let y = y + self.offset3d.y;
        let z = z + self.offset3d.z;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return (None, draws);
        }
        let v = Vec3::new(x, y, z);
        let pos = self.vec3_to_pos_depth(v);
        let inside = self.ignore_bounds
            || (x >= self.bound.x
                && x <= self.bound.y
                && y >= self.bound.x
                && y <= self.bound.y
                && z >= self.bound.x
                && z <= self.bound.y);
        if !matches!(self.lines, Lines::Lines) && inside {
            draws.push((pos.1, Draw::Point(pos.0), self.shift_hue(pos.1, z, color)));
        }
        if !matches!(self.lines, Lines::Points) {
            let mut body = |last: ((Pos, f32), Vec3, bool)| {
                if inside && last.2 {
                    let d = (pos.1 + last.0.1) / 2.0;
                    draws.push((
                        d,
                        Draw::Line(last.0.0, pos.0, 1.0),
                        self.shift_hue(d, z, color),
                    ));
                } else if inside {
                    let mut vi = last.1;
                    let xi = vi.x;
                    if xi < self.bound.x {
                        vi = v + (vi - v) * ((self.bound.x - x) / (xi - x));
                    } else if xi > self.bound.y {
                        vi = v + (vi - v) * ((self.bound.y - x) / (xi - x));
                    }
                    let yi = vi.y;
                    if yi < self.bound.x {
                        vi = v + (vi - v) * ((self.bound.x - y) / (yi - y));
                    } else if yi > self.bound.y {
                        vi = v + (vi - v) * ((self.bound.y - y) / (yi - y));
                    }
                    let zi = vi.z;
                    if zi < self.bound.x {
                        vi = v + (vi - v) * ((self.bound.x - z) / (zi - z));
                    } else if zi > self.bound.y {
                        vi = v + (vi - v) * ((self.bound.y - z) / (zi - z));
                    }
                    let last = self.vec3_to_pos_depth(vi);
                    let d = (pos.1 + last.1) / 2.0;
                    draws.push((
                        d,
                        Draw::Line(last.0, pos.0, 1.0),
                        self.shift_hue(d, z, color),
                    ));
                } else if last.2 {
                    let mut vi = v;
                    let v = last.1;
                    let (x, y, z) = (v.x, v.y, v.z);
                    let pos = self.vec3_to_pos_depth(v);
                    let xi = vi.x;
                    if xi < self.bound.x {
                        vi = v + (vi - v) * ((self.bound.x - x) / (xi - x));
                    } else if xi > self.bound.y {
                        vi = v + (vi - v) * ((self.bound.y - x) / (xi - x));
                    }
                    let yi = vi.y;
                    if yi < self.bound.x {
                        vi = v + (vi - v) * ((self.bound.x - y) / (yi - y));
                    } else if yi > self.bound.y {
                        vi = v + (vi - v) * ((self.bound.y - y) / (yi - y));
                    }
                    let zi = vi.z;
                    if zi < self.bound.x {
                        vi = v + (vi - v) * ((self.bound.x - z) / (zi - z));
                    } else if zi > self.bound.y {
                        vi = v + (vi - v) * ((self.bound.y - z) / (zi - z));
                    }
                    let last = self.vec3_to_pos_depth(vi);
                    let d = (pos.1 + last.1) / 2.0;
                    draws.push((
                        d,
                        Draw::Line(last.0, pos.0, 1.0),
                        self.shift_hue(d, z, color),
                    ));
                }
            };
            if let Some(last) = a {
                body(last)
            }
            if let Some(last) = b {
                body(last)
            }
            (Some((pos, Vec3::new(x, y, z), inside)), draws)
        } else {
            (None, draws)
        }
    }
    fn write_axis_3d(&mut self, painter: &mut Painter) {
        let s = (self.bound.y - self.bound.x) / 2.0;
        let vertices = [
            self.vec3_to_pos_depth(Vec3::new(-s, -s, -s)),
            self.vec3_to_pos_depth(Vec3::new(-s, -s, s)),
            self.vec3_to_pos_depth(Vec3::new(-s, s, -s)),
            self.vec3_to_pos_depth(Vec3::new(-s, s, s)),
            self.vec3_to_pos_depth(Vec3::new(s, -s, -s)),
            self.vec3_to_pos_depth(Vec3::new(s, -s, s)),
            self.vec3_to_pos_depth(Vec3::new(s, s, -s)),
            self.vec3_to_pos_depth(Vec3::new(s, s, s)),
        ];
        let edges = [
            (0, 1),
            (1, 3),
            (3, 2),
            (2, 0),
            (4, 5),
            (5, 7),
            (7, 6),
            (6, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let mut xl = 0;
        for (i, v) in vertices[1..].iter().enumerate() {
            if v.0.y > vertices[xl].0.y || (v.0.y == vertices[xl].0.y && v.0.x > vertices[xl].0.x) {
                xl = i + 1
            }
        }
        let mut zl = 0;
        for (i, v) in vertices[1..].iter().enumerate() {
            if (v.0.x < vertices[zl].0.x || (v.0.x == vertices[zl].0.x && v.0.y > vertices[zl].0.y))
                && edges
                    .iter()
                    .any(|(m, n)| (*m == i + 1 || *n == i + 1) && (xl == *m || xl == *n))
            {
                zl = i + 1
            }
        }
        for (k, (i, j)) in edges.iter().enumerate() {
            let s = match k {
                8..=11 => "\nx",
                1 | 3 | 5 | 7 => "\ny",
                0 | 2 | 4 | 6 => "z",
                _ => unreachable!(),
            };
            if (s == "z" && [i, j].contains(&&zl)) || (s != "z" && [i, j].contains(&&xl)) {
                self.buffer.push((
                    if vertices[*i].1 < 0.5 || vertices[*j].1 < 0.5 {
                        0.0
                    } else {
                        1.0
                    },
                    Draw::Line(
                        vertices[*i].0,
                        vertices[*j].0,
                        vertices[*i].1 + vertices[*j].1,
                    ),
                    self.axis_color,
                ));
                if !self.disable_axis {
                    let p = vertices[*i].0 + vertices[*j].0;
                    let align = match s {
                        "\nx" if p.x > self.screen.x as f32 => Align::LeftTop,
                        "\ny" if p.x < self.screen.x as f32 => Align::RightTop,
                        "\nx" => Align::RightTop,
                        "\ny" => Align::LeftTop,
                        "z" => Align::RightCenter,
                        _ => unreachable!(),
                    };
                    let start = vertices[*i.min(j)].0;
                    let end = vertices[*i.max(j)].0;
                    let st = self.bound.x.ceil() as isize;
                    let e = self.bound.y.floor() as isize;
                    let o = if s == "z" {
                        self.offset3d.z
                    } else if s == "\nx" {
                        -self.offset3d.x
                    } else if s == "\ny" {
                        self.offset3d.y
                    } else {
                        unreachable!()
                    };
                    let n = ((st + (e - st) / 2) as f64 - o).to_string();
                    painter.text(
                        p / 2.0,
                        align,
                        if s == "z" {
                            format!("z{}", " ".repeat(n.len()))
                        } else {
                            s.to_string()
                        },
                        &self.text_color,
                    );
                    for i in st..=e {
                        painter.text(
                            start + (end - start) * ((i - st) as f32 / (e - st) as f32),
                            align,
                            (i as f64 - o).to_string(),
                            &self.text_color,
                        );
                    }
                }
            } else if self.show_box {
                self.buffer.push((
                    if vertices[*i].1 < 0.5 || vertices[*j].1 < 0.5 {
                        0.0
                    } else {
                        1.0
                    },
                    Draw::Line(
                        vertices[*i].0,
                        vertices[*j].0,
                        vertices[*i].1 + vertices[*j].1,
                    ),
                    self.axis_color,
                ));
            }
        }
    }
    #[cfg(feature = "egui")]
    fn keybinds(&mut self, ui: &egui::Ui) {
        ui.input(|i| {
            if let Some(mpos) = i.pointer.latest_pos() {
                let mpos = Pos {
                    x: mpos.x,
                    y: mpos.y,
                };
                if let Some(pos) = self.mouse_position {
                    if mpos != pos {
                        self.mouse_moved = true;
                        self.mouse_position = Some(mpos)
                    }
                } else {
                    self.mouse_position = Some(mpos)
                }
            }
            let multi = i.multi_touch();
            let interact = i.pointer.interact_pos().map(|a| Pos { x: a.x, y: a.y });
            match multi {
                Some(multi) => {
                    match multi.zoom_delta.total_cmp(&1.0) {
                        std::cmp::Ordering::Greater => {
                            if self.is_3d {
                                self.box_size /= multi.zoom_delta as f64;
                            } else {
                                self.zoom *= multi.zoom_delta as f64;
                                self.offset.x -= if self.mouse_moved && !self.is_3d {
                                    self.mouse_position.unwrap().x as f64
                                } else {
                                    self.screen_offset.x
                                } / self.zoom
                                    * (multi.zoom_delta as f64 - 1.0);
                                self.offset.y -= if self.mouse_moved && !self.is_3d {
                                    self.mouse_position.unwrap().y as f64
                                } else {
                                    self.screen_offset.y
                                } / self.zoom
                                    * (multi.zoom_delta as f64 - 1.0);
                                self.recalculate = true;
                            }
                        }
                        std::cmp::Ordering::Less => {
                            if self.is_3d {
                                self.box_size /= multi.zoom_delta as f64;
                            } else {
                                self.offset.x += if self.mouse_moved && !self.is_3d {
                                    self.mouse_position.unwrap().x as f64
                                } else {
                                    self.screen_offset.x
                                } / self.zoom
                                    * ((multi.zoom_delta as f64).recip() - 1.0);
                                self.offset.y += if self.mouse_moved && !self.is_3d {
                                    self.mouse_position.unwrap().y as f64
                                } else {
                                    self.screen_offset.y
                                } / self.zoom
                                    * ((multi.zoom_delta as f64).recip() - 1.0);
                                self.zoom *= multi.zoom_delta as f64;
                                self.recalculate = true;
                            }
                        }
                        _ => {}
                    }
                    if self.is_3d {
                        self.angle.x = (self.angle.x - multi.translation_delta.x as f64 / 512.0)
                            .rem_euclid(TAU);
                        self.angle.y = (self.angle.y + multi.translation_delta.y as f64 / 512.0)
                            .rem_euclid(TAU);
                    } else {
                        self.offset.x += multi.translation_delta.x as f64 / self.zoom;
                        self.offset.y += multi.translation_delta.y as f64 / self.zoom;
                        self.recalculate = true;
                        if !self.mouse_held {
                            self.mouse_held = true;
                            self.prec = (self.prec + 1.0).log10();
                        }
                    }
                }
                _ if i.pointer.primary_down()
                    && i.pointer.press_start_time().unwrap_or(0.0) < i.time =>
                {
                    if let (Some(interact), Some(last)) = (interact, self.last_interact) {
                        let delta = interact - last;
                        if self.is_3d {
                            self.angle.x = (self.angle.x - delta.x as f64 / 512.0).rem_euclid(TAU);
                            self.angle.y = (self.angle.y + delta.y as f64 / 512.0).rem_euclid(TAU);
                        } else {
                            self.offset.x += delta.x as f64 / self.zoom;
                            self.offset.y += delta.y as f64 / self.zoom;
                            self.recalculate = true;
                            if !self.mouse_held {
                                self.mouse_held = true;
                                self.prec = (self.prec + 1.0).log10();
                            }
                        }
                    }
                }
                _ if self.mouse_held => {
                    self.mouse_held = false;
                    self.prec = 10.0f64.powf(self.prec) - 1.0;
                    self.recalculate = true;
                }
                _ => {}
            }
            self.last_interact = interact;
            let shift = i.modifiers.shift;
            let (a, b, c) = if shift {
                (
                    4.0 * self.delta
                        / if self.zoom > 1.0 {
                            2.0 * self.zoom
                        } else {
                            1.0
                        },
                    PI / 16.0,
                    4,
                )
            } else {
                (
                    self.delta
                        / if self.zoom > 1.0 {
                            2.0 * self.zoom
                        } else {
                            1.0
                        },
                    PI / 64.0,
                    1,
                )
            };
            if i.key_pressed(egui::Key::A) || i.key_pressed(egui::Key::ArrowLeft) {
                if self.is_3d {
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::SliceDepth) {
                            self.recalculate = true;
                        }
                        self.offset3d.x -= 1.0
                    } else {
                        self.angle.x = ((self.angle.x / b - 1.0).round() * b).rem_euclid(TAU);
                    }
                } else {
                    self.offset.x += a;
                    self.recalculate = true;
                }
            }
            if i.key_pressed(egui::Key::D) || i.key_pressed(egui::Key::ArrowRight) {
                if self.is_3d {
                    if i.key_pressed(egui::Key::ArrowRight) {
                        if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::SliceDepth) {
                            self.recalculate = true;
                        }
                        self.offset3d.x += 1.0
                    } else {
                        self.angle.x = ((self.angle.x / b + 1.0).round() * b).rem_euclid(TAU);
                    }
                } else {
                    self.offset.x -= a;
                    self.recalculate = true;
                }
            }
            if i.key_pressed(egui::Key::W) || i.key_pressed(egui::Key::ArrowUp) {
                if self.is_3d {
                    if i.key_pressed(egui::Key::ArrowUp) {
                        if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::SliceDepth) {
                            self.recalculate = true;
                        }
                        self.offset3d.y -= 1.0
                    } else {
                        self.angle.y = ((self.angle.y / b - 1.0).round() * b).rem_euclid(TAU);
                    }
                } else {
                    if self.graph_mode == GraphMode::DomainColoring {
                        self.recalculate = true;
                    }
                    self.offset.y += a;
                }
            }
            if i.key_pressed(egui::Key::S) || i.key_pressed(egui::Key::ArrowDown) {
                if self.is_3d {
                    if i.key_pressed(egui::Key::ArrowDown) {
                        if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::SliceDepth) {
                            self.recalculate = true;
                        }
                        self.offset3d.y += 1.0
                    } else {
                        self.angle.y = ((self.angle.y / b + 1.0).round() * b).rem_euclid(TAU);
                    }
                } else {
                    if self.graph_mode == GraphMode::DomainColoring {
                        self.recalculate = true;
                    }
                    self.offset.y -= a;
                }
            }
            if i.key_pressed(egui::Key::Z) {
                self.disable_lines = !self.disable_lines;
            }
            if i.key_pressed(egui::Key::X) {
                self.disable_axis = !self.disable_axis;
            }
            if i.key_pressed(egui::Key::C) {
                self.disable_coord = !self.disable_coord;
            }
            if i.key_pressed(egui::Key::V) {
                self.scale_axis = !self.scale_axis;
            }
            if i.key_pressed(egui::Key::R) {
                self.anti_alias = !self.anti_alias;
                self.cache = None;
            }
            if self.is_3d {
                if i.key_pressed(egui::Key::F) {
                    self.offset3d.z += 1.0;
                    if matches!(self.graph_mode, GraphMode::Depth | GraphMode::SliceDepth) {
                        self.recalculate = true;
                    }
                }
                if i.key_pressed(egui::Key::G) {
                    self.offset3d.z -= 1.0;
                    if matches!(self.graph_mode, GraphMode::Depth | GraphMode::SliceDepth) {
                        self.recalculate = true;
                    }
                }
                if i.key_pressed(egui::Key::P) {
                    self.ignore_bounds = !self.ignore_bounds;
                }
                if i.key_pressed(egui::Key::O) {
                    self.color_depth = match self.color_depth {
                        DepthColor::None => DepthColor::Vertical,
                        DepthColor::Vertical => DepthColor::Depth,
                        DepthColor::Depth => DepthColor::None,
                    };
                }
                let mut changed = false;
                if i.key_pressed(egui::Key::Semicolon) && self.box_size > 0.1 {
                    self.box_size -= 0.1;
                    changed = true
                }
                if i.key_pressed(egui::Key::Quote) {
                    self.box_size += 0.1;
                    changed = true
                }
                if changed {
                    if (self.box_size - 1.0).abs() < 0.05 {
                        self.box_size = 1.0
                    }
                    if (self.box_size - 2.0f64.sqrt()).abs() < 0.1 {
                        self.box_size = 2.0f64.sqrt()
                    }
                    if (self.box_size - 3.0f64.sqrt()).abs() < 0.1 {
                        self.box_size = 3.0f64.sqrt()
                    }
                }
                if i.key_pressed(egui::Key::Y) {
                    self.show_box = !self.show_box
                }
                self.angle.x = (self.angle.x - i.raw_scroll_delta.x as f64 / 512.0).rem_euclid(TAU);
                self.angle.y = (self.angle.y + i.raw_scroll_delta.y as f64 / 512.0).rem_euclid(TAU);
            } else {
                let rt = 1.0 + i.raw_scroll_delta.y / 512.0;
                if i.key_pressed(egui::Key::Y) {
                    self.cache = None;
                    self.domain_alternate = !self.domain_alternate
                }
                match rt.total_cmp(&1.0) {
                    std::cmp::Ordering::Greater => {
                        self.zoom *= rt as f64;
                        self.offset.x -= if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().x as f64
                        } else {
                            self.screen_offset.x
                        } / self.zoom
                            * (rt as f64 - 1.0);
                        self.offset.y -= if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().y as f64
                        } else {
                            self.screen_offset.y
                        } / self.zoom
                            * (rt as f64 - 1.0);
                        self.recalculate = true;
                    }
                    std::cmp::Ordering::Less => {
                        self.offset.x += if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().x as f64
                        } else {
                            self.screen_offset.x
                        } / self.zoom
                            * ((rt as f64).recip() - 1.0);
                        self.offset.y += if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().y as f64
                        } else {
                            self.screen_offset.y
                        } / self.zoom
                            * ((rt as f64).recip() - 1.0);
                        self.zoom *= rt as f64;
                        self.recalculate = true;
                    }
                    _ => {}
                }
            }
            if i.key_pressed(egui::Key::Q) {
                if self.is_3d {
                    self.zoom3d *= 2.0;
                    self.bound *= 2.0;
                } else {
                    self.offset.x += if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().x as f64
                    } else {
                        self.screen_offset.x
                    } / self.zoom;
                    self.offset.y += if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().y as f64
                    } else {
                        self.screen_offset.y
                    } / self.zoom;
                    self.zoom /= 2.0;
                }
                self.recalculate = true;
            }
            if i.key_pressed(egui::Key::E) {
                if self.is_3d {
                    self.zoom3d /= 2.0;
                    self.bound /= 2.0;
                } else {
                    self.zoom *= 2.0;
                    self.offset.x -= if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().x as f64
                    } else {
                        self.screen_offset.x
                    } / self.zoom;
                    self.offset.y -= if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().y as f64
                    } else {
                        self.screen_offset.y
                    } / self.zoom;
                }
                self.recalculate = true;
            }
            if matches!(
                self.graph_mode,
                GraphMode::Slice | GraphMode::SliceFlatten | GraphMode::SliceDepth
            ) {
                if i.key_pressed(egui::Key::Period) {
                    self.recalculate = true;
                    self.slice += c
                }
                if i.key_pressed(egui::Key::Comma) {
                    self.recalculate = true;
                    self.slice -= c
                }
                if i.key_pressed(egui::Key::Slash) {
                    self.recalculate = true;
                    self.view_x = !self.view_x
                }
            }
            if i.key_pressed(egui::Key::L) {
                if self.graph_mode == GraphMode::DomainColoring {
                    self.cache = None;
                    self.log_scale = !self.log_scale
                } else {
                    self.lines = match self.lines {
                        Lines::Lines => Lines::Points,
                        Lines::Points => Lines::LinesPoints,
                        Lines::LinesPoints => Lines::Lines,
                    };
                }
            }
            if self.graph_mode == GraphMode::Flatten || self.graph_mode == GraphMode::SliceFlatten {
                let s = if shift {
                    (self.var.y - self.var.x) / 2.0
                } else {
                    (self.var.y - self.var.x) / 4.0
                };
                if i.key_pressed(egui::Key::H) {
                    self.var.x -= s;
                    self.var.y -= s;
                    self.recalculate = true;
                }
                if i.key_pressed(egui::Key::J) {
                    self.var.x += s;
                    self.var.y += s;
                    self.recalculate = true;
                }
                if i.key_pressed(egui::Key::M) {
                    if shift {
                        self.var.x =
                            (self.var.x + self.var.y) / 2.0 - (self.var.y - self.var.x) / 4.0;
                        self.var.y =
                            (self.var.x + self.var.y) / 2.0 + (self.var.y - self.var.x) / 4.0;
                    } else {
                        self.var.x = (self.var.x + self.var.y) / 2.0 - (self.var.y - self.var.x);
                        self.var.y = (self.var.x + self.var.y) / 2.0 + (self.var.y - self.var.x);
                    }
                    self.recalculate = true;
                }
            }
            if i.key_pressed(egui::Key::OpenBracket) {
                self.recalculate = true;
                self.prec /= 2.0;
                self.slice /= 2;
            }
            if i.key_pressed(egui::Key::CloseBracket) {
                self.recalculate = true;
                self.prec *= 2.0;
                self.slice *= 2;
            }
            if i.key_pressed(egui::Key::N) {
                let last = self.ruler_pos;
                self.ruler_pos = self.mouse_position.map(|a| {
                    let a = self.to_coord(a);
                    Vec2::new(a.0, a.1)
                });
                if last == self.ruler_pos {
                    self.ruler_pos = None;
                }
            }
            if self.is_complex && i.key_pressed(egui::Key::I) {
                self.show = match self.show {
                    Show::Complex => Show::Real,
                    Show::Real => Show::Imag,
                    Show::Imag => Show::Complex,
                }
            }
            if i.key_pressed(egui::Key::B) {
                if self.is_complex {
                    self.graph_mode = match self.graph_mode {
                        GraphMode::Normal if shift => {
                            self.recalculate = true;
                            if self.is_3d {
                                self.is_3d = false;
                                GraphMode::DomainColoring
                            } else {
                                self.is_3d = true;
                                GraphMode::Depth
                            }
                        }
                        GraphMode::Slice if shift => {
                            self.is_3d = true;
                            self.recalculate = true;
                            GraphMode::Normal
                        }
                        GraphMode::SliceDepth if shift => {
                            self.is_3d = false;
                            self.recalculate = true;
                            GraphMode::SliceFlatten
                        }
                        GraphMode::SliceFlatten if shift => {
                            self.recalculate = true;
                            GraphMode::Slice
                        }
                        GraphMode::Flatten if shift => {
                            self.recalculate = true;
                            GraphMode::Normal
                        }
                        GraphMode::DomainColoring if shift => {
                            self.is_3d = true;
                            self.recalculate = true;
                            GraphMode::SliceDepth
                        }
                        GraphMode::Depth if shift => {
                            self.is_3d = false;
                            self.recalculate = true;
                            GraphMode::Flatten
                        }
                        GraphMode::Normal => {
                            self.recalculate = true;
                            if self.is_3d {
                                self.is_3d = false;
                                GraphMode::Slice
                            } else {
                                GraphMode::Flatten
                            }
                        }
                        GraphMode::Slice => {
                            self.recalculate = true;
                            GraphMode::SliceFlatten
                        }
                        GraphMode::SliceFlatten => {
                            self.is_3d = true;
                            self.recalculate = true;
                            GraphMode::SliceDepth
                        }
                        GraphMode::SliceDepth => {
                            self.is_3d = false;
                            self.recalculate = true;
                            GraphMode::DomainColoring
                        }
                        GraphMode::Flatten => {
                            self.is_3d = true;
                            self.recalculate = true;
                            GraphMode::Depth
                        }
                        GraphMode::Depth => {
                            self.is_3d = false;
                            self.recalculate = true;
                            GraphMode::Normal
                        }
                        GraphMode::DomainColoring => {
                            self.recalculate = true;
                            self.is_3d = true;
                            GraphMode::Normal
                        }
                    };
                } else {
                    match self.graph_mode {
                        GraphMode::Normal => {
                            if self.is_3d {
                                self.recalculate = true;
                                self.is_3d = false;
                                self.graph_mode = GraphMode::Slice
                            }
                        }
                        GraphMode::Slice => {
                            self.recalculate = true;
                            self.is_3d = true;
                            self.graph_mode = GraphMode::Normal;
                        }
                        _ => {}
                    }
                }
            }
            if i.key_pressed(egui::Key::T) {
                self.offset3d = Vec3::splat(0.0);
                self.offset = Vec2::splat(0.0);
                self.var = self.bound;
                self.zoom = 1.0;
                self.zoom3d = 1.0;
                self.slice = 0;
                self.angle = Vec2::splat(PI / 6.0);
                self.box_size = 3.0f64.sqrt();
                self.prec = 1.0;
                self.mouse_position = None;
                self.mouse_moved = false;
                self.recalculate = true;
            }
        });
    }
    #[cfg(feature = "egui")]
    fn plot(&mut self, painter: &mut Painter, ui: &egui::Ui) {
        let n = self
            .data
            .iter()
            .map(|a| match a {
                GraphType::Coord(_) => 0,
                GraphType::Coord3D(d) => d.len(),
                GraphType::Width(_, _, _) => 0,
                GraphType::Width3D(d, _, _, _, _) => d.len(),
            })
            .sum::<usize>()
            * if self.is_complex && matches!(self.show, Show::Complex) {
                2
            } else {
                1
            }
            * match self.lines {
                Lines::Points => 1,
                Lines::Lines => 2,
                Lines::LinesPoints => 3,
            };
        if self.buffer.capacity() < n {
            self.buffer = Vec::with_capacity(n + 12)
        }
        let mut pts = Vec::with_capacity(n);
        for (k, data) in self.data.iter().enumerate() {
            let (mut a, mut b, mut c) = (None, None, None);
            match data {
                GraphType::Width(data, start, end) => match self.graph_mode {
                    GraphMode::DomainColoring
                    | GraphMode::Slice
                    | GraphMode::SliceFlatten
                    | GraphMode::SliceDepth => unreachable!(),
                    GraphMode::Normal => {
                        for (i, y) in data.iter().enumerate() {
                            let x = (i as f64 / (data.len() - 1) as f64 - 0.5) * (end - start)
                                + (start + end) / 2.0;
                            let (y, z) = y.to_options();
                            a = if !self.show.real() {
                                None
                            } else if let Some(y) = y {
                                self.draw_point(
                                    painter,
                                    ui,
                                    x,
                                    y,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                            b = if !self.show.imag() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point(
                                    painter,
                                    ui,
                                    x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Flatten => {
                        for y in data {
                            let (y, z) = y.to_options();
                            a = if let (Some(y), Some(z)) = (y, z) {
                                self.draw_point(
                                    painter,
                                    ui,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Depth => {
                        for (i, y) in data.iter().enumerate() {
                            let (y, z) = y.to_options();
                            c = if let (Some(x), Some(y)) = (y, z) {
                                let z = (i as f64 / (data.len() - 1) as f64 - 0.5) * (end - start)
                                    + (start + end) / 2.0;
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    c,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        }
                    }
                },
                GraphType::Coord(data) => match self.graph_mode {
                    GraphMode::DomainColoring
                    | GraphMode::Slice
                    | GraphMode::SliceFlatten
                    | GraphMode::SliceDepth => unreachable!(),
                    GraphMode::Normal => {
                        for (x, y) in data {
                            let (y, z) = y.to_options();
                            a = if !self.show.real() {
                                None
                            } else if let Some(y) = y {
                                self.draw_point(
                                    painter,
                                    ui,
                                    *x,
                                    y,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                            b = if !self.show.imag() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point(
                                    painter,
                                    ui,
                                    *x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Flatten => {
                        for (_, y) in data {
                            let (y, z) = y.to_options();
                            a = if let (Some(y), Some(z)) = (y, z) {
                                self.draw_point(
                                    painter,
                                    ui,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Depth => {
                        for (i, y) in data {
                            let (y, z) = y.to_options();
                            c = if let (Some(x), Some(y)) = (y, z) {
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    *i,
                                    &self.main_colors[k % self.main_colors.len()],
                                    c,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        }
                    }
                },
                GraphType::Width3D(data, start_x, start_y, end_x, end_y) => match self.graph_mode {
                    GraphMode::Flatten | GraphMode::Depth => unreachable!(),
                    GraphMode::Normal => {
                        let len = data.len().isqrt();
                        let mut last = Vec::new();
                        let mut cur = Vec::new();
                        let mut lasti = Vec::new();
                        let mut curi = Vec::new();
                        for (i, z) in data.iter().enumerate() {
                            let (i, j) = (i % len, i / len);
                            let x = (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                                + (start_x + end_x) / 2.0;
                            let y = (j as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                                + (start_y + end_y) / 2.0;
                            let (z, w) = z.to_options();
                            let p = if !self.show.real() {
                                None
                            } else if let Some(z) = z {
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    if i == 0 { None } else { cur[i - 1] },
                                    if j == 0 { None } else { last[i] },
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                            cur.push(p);
                            if i == len - 1 {
                                last = std::mem::take(&mut cur);
                            }
                            let p = if !self.show.imag() {
                                None
                            } else if let Some(w) = w {
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    w,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    if i == 0 { None } else { curi[i - 1] },
                                    if j == 0 { None } else { lasti[i] },
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                            curi.push(p);
                            if i == len - 1 {
                                lasti = std::mem::take(&mut curi);
                            }
                        }
                    }
                    GraphMode::Slice => {
                        let len = data.len();
                        let mut body = |i: usize, y: &Complex| {
                            let x = (i as f64 / (len - 1) as f64 - 0.5)
                                * if self.view_x {
                                    end_x - start_x
                                } else {
                                    end_y - start_y
                                }
                                + if self.view_x {
                                    start_x + end_x
                                } else {
                                    start_y + end_y
                                } / 2.0;
                            let (y, z) = y.to_options();
                            a = if !self.show.real() {
                                None
                            } else if let Some(y) = y {
                                self.draw_point(
                                    painter,
                                    ui,
                                    x,
                                    y,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                            b = if !self.show.imag() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point(
                                    painter,
                                    ui,
                                    x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            } else {
                                None
                            };
                        };
                        for (i, y) in data.iter().enumerate() {
                            body(i, y)
                        }
                    }
                    GraphMode::SliceFlatten => {
                        let mut body = |y: &Complex| {
                            let (y, z) = y.to_options();
                            a = if let (Some(y), Some(z)) = (y, z) {
                                self.draw_point(
                                    painter,
                                    ui,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                        };
                        for y in data.iter() {
                            body(y)
                        }
                    }
                    GraphMode::SliceDepth => {
                        let len = data.len();
                        let mut body = |i: usize, y: &Complex| {
                            let (y, z) = y.to_options();
                            c = if let (Some(x), Some(y)) = (y, z) {
                                let z = if self.view_x {
                                    (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                                        + (start_x + end_x) / 2.0
                                } else {
                                    (i as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                                        + (start_y + end_y) / 2.0
                                };
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    c,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        };
                        for (i, y) in data.iter().enumerate() {
                            body(i, y)
                        }
                    }
                    GraphMode::DomainColoring => {
                        let lenx = (self.screen.x * self.prec * self.mult) as usize + 1;
                        let leny = (self.screen.y * self.prec * self.mult) as usize + 1;
                        let texture = if let Some(tex) = &self.cache {
                            tex
                        } else {
                            let mut rgb = Vec::new();
                            for z in data {
                                rgb.extend(self.get_color(z));
                            }
                            let tex = ui.ctx().load_texture(
                                "dc",
                                egui::ColorImage::from_rgb([lenx, leny], &rgb),
                                if self.anti_alias {
                                    egui::TextureOptions::LINEAR
                                } else {
                                    egui::TextureOptions::NEAREST
                                },
                            );
                            self.cache = Some(tex);
                            self.cache.as_ref().unwrap()
                        };
                        painter.image(
                            Texture {
                                texture: texture.id(),
                            },
                            self.screen,
                        );
                    }
                },
                GraphType::Coord3D(data) => match self.graph_mode {
                    GraphMode::Slice
                    | GraphMode::SliceFlatten
                    | GraphMode::SliceDepth
                    | GraphMode::DomainColoring
                    | GraphMode::Flatten
                    | GraphMode::Depth => unreachable!(),
                    GraphMode::Normal => {
                        let mut last = None;
                        let mut lasti = None;
                        for (x, y, z) in data {
                            let (z, w) = z.to_options();
                            last = if !self.show.real() {
                                None
                            } else if let Some(z) = z {
                                let (c, d) = self.draw_point_3d(
                                    *x,
                                    *y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    last,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                            lasti = if !self.show.imag() {
                                None
                            } else if let Some(w) = w {
                                let (c, d) = self.draw_point_3d(
                                    *x,
                                    *y,
                                    w,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    lasti,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        }
                    }
                },
            }
        }
        self.buffer.extend(pts);
    }
    #[cfg(feature = "skia")]
    fn plot(&mut self, painter: &mut Painter) {
        let n = self
            .data
            .iter()
            .map(|a| match a {
                GraphType::Coord(_) => 0,
                GraphType::Coord3D(d) => d.len(),
                GraphType::Width(_, _, _) => 0,
                GraphType::Width3D(d, _, _, _, _) => d.len(),
            })
            .sum::<usize>()
            * if self.is_complex && matches!(self.show, Show::Complex) {
                2
            } else {
                1
            }
            * match self.lines {
                Lines::Points => 1,
                Lines::Lines => 2,
                Lines::LinesPoints => 3,
            };
        if self.buffer.capacity() < n {
            self.buffer = Vec::with_capacity(n + 12)
        }
        let mut pts = Vec::with_capacity(n);
        for (k, data) in self.data.iter().enumerate() {
            let (mut a, mut b, mut c) = (None, None, None);
            match data {
                GraphType::Width(data, start, end) => match self.graph_mode {
                    GraphMode::DomainColoring
                    | GraphMode::Slice
                    | GraphMode::SliceFlatten
                    | GraphMode::SliceDepth => unreachable!(),
                    GraphMode::Normal => {
                        for (i, y) in data.iter().enumerate() {
                            let x = (i as f64 / (data.len() - 1) as f64 - 0.5) * (end - start)
                                + (start + end) / 2.0;
                            let (y, z) = y.to_options();
                            a = if !self.show.real() {
                                None
                            } else if let Some(y) = y {
                                self.draw_point(
                                    painter,
                                    x,
                                    y,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                            b = if !self.show.imag() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point(
                                    painter,
                                    x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Flatten => {
                        for y in data {
                            let (y, z) = y.to_options();
                            a = if let (Some(y), Some(z)) = (y, z) {
                                self.draw_point(
                                    painter,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Depth => {
                        for (i, y) in data.iter().enumerate() {
                            let (y, z) = y.to_options();
                            c = if let (Some(x), Some(y)) = (y, z) {
                                let z = (i as f64 / (data.len() - 1) as f64 - 0.5) * (end - start)
                                    + (start + end) / 2.0;
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    c,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        }
                    }
                },
                GraphType::Coord(data) => match self.graph_mode {
                    GraphMode::DomainColoring
                    | GraphMode::Slice
                    | GraphMode::SliceFlatten
                    | GraphMode::SliceDepth => unreachable!(),
                    GraphMode::Normal => {
                        for (x, y) in data {
                            let (y, z) = y.to_options();
                            a = if !self.show.real() {
                                None
                            } else if let Some(y) = y {
                                self.draw_point(
                                    painter,
                                    *x,
                                    y,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                            b = if !self.show.imag() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point(
                                    painter,
                                    *x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Flatten => {
                        for (_, y) in data {
                            let (y, z) = y.to_options();
                            a = if let (Some(y), Some(z)) = (y, z) {
                                self.draw_point(
                                    painter,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                        }
                    }
                    GraphMode::Depth => {
                        for (i, y) in data {
                            let (y, z) = y.to_options();
                            c = if let (Some(x), Some(y)) = (y, z) {
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    *i,
                                    &self.main_colors[k % self.main_colors.len()],
                                    c,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        }
                    }
                },
                GraphType::Width3D(data, start_x, start_y, end_x, end_y) => match self.graph_mode {
                    GraphMode::Flatten | GraphMode::Depth => unreachable!(),
                    GraphMode::Normal => {
                        let len = data.len().isqrt();
                        let mut last = Vec::new();
                        let mut cur = Vec::new();
                        let mut lasti = Vec::new();
                        let mut curi = Vec::new();
                        for (i, z) in data.iter().enumerate() {
                            let (i, j) = (i % len, i / len);
                            let x = (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                                + (start_x + end_x) / 2.0;
                            let y = (j as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                                + (start_y + end_y) / 2.0;
                            let (z, w) = z.to_options();
                            let p = if !self.show.real() {
                                None
                            } else if let Some(z) = z {
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    if i == 0 { None } else { cur[i - 1] },
                                    if j == 0 { None } else { last[i] },
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                            cur.push(p);
                            if i == len - 1 {
                                last = std::mem::take(&mut cur);
                            }
                            let p = if !self.show.imag() {
                                None
                            } else if let Some(w) = w {
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    w,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    if i == 0 { None } else { curi[i - 1] },
                                    if j == 0 { None } else { lasti[i] },
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                            curi.push(p);
                            if i == len - 1 {
                                lasti = std::mem::take(&mut curi);
                            }
                        }
                    }
                    GraphMode::Slice => {
                        let len = data.len();
                        let mut body = |i: usize, y: &Complex| {
                            let x = (i as f64 / (len - 1) as f64 - 0.5)
                                * if self.view_x {
                                    end_x - start_x
                                } else {
                                    end_y - start_y
                                }
                                + if self.view_x {
                                    start_x + end_x
                                } else {
                                    start_y + end_y
                                } / 2.0;
                            let (y, z) = y.to_options();
                            a = if !self.show.real() {
                                None
                            } else if let Some(y) = y {
                                self.draw_point(
                                    painter,
                                    x,
                                    y,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                            b = if !self.show.imag() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point(
                                    painter,
                                    x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            } else {
                                None
                            };
                        };
                        for (i, y) in data.iter().enumerate() {
                            body(i, y)
                        }
                    }
                    GraphMode::SliceFlatten => {
                        let mut body = |y: &Complex| {
                            let (y, z) = y.to_options();
                            a = if let (Some(y), Some(z)) = (y, z) {
                                self.draw_point(
                                    painter,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    a,
                                )
                            } else {
                                None
                            };
                        };
                        for y in data.iter() {
                            body(y)
                        }
                    }
                    GraphMode::SliceDepth => {
                        let len = data.len();
                        let mut body = |i: usize, y: &Complex| {
                            let (y, z) = y.to_options();
                            c = if let (Some(x), Some(y)) = (y, z) {
                                let z = if self.view_x {
                                    (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                                        + (start_x + end_x) / 2.0
                                } else {
                                    (i as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                                        + (start_y + end_y) / 2.0
                                };
                                let (c, d) = self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    c,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        };
                        for (i, y) in data.iter().enumerate() {
                            body(i, y)
                        }
                    }
                    GraphMode::DomainColoring => {
                        let lenx = (self.screen.x * self.prec * self.mult) as usize + 1;
                        let leny = (self.screen.y * self.prec * self.mult) as usize + 1;
                        let image = if let Some(tex) = &self.cache {
                            tex
                        } else {
                            let mut rgb = Vec::new();
                            for z in data {
                                rgb.extend(self.get_color(z));
                            }
                            let info = skia_safe::ImageInfo::new(
                                (lenx as i32, leny as i32),
                                skia_safe::ColorType::RGB888x,
                                skia_safe::AlphaType::Opaque,
                                None,
                            );
                            self.cache = skia_safe::images::raster_from_data(
                                &info,
                                skia_safe::Data::new_copy(&rgb),
                                lenx,
                            );
                            self.cache.as_ref().unwrap()
                        };
                        painter.image(image, self.screen);
                    }
                },
                GraphType::Coord3D(data) => match self.graph_mode {
                    GraphMode::Slice
                    | GraphMode::SliceFlatten
                    | GraphMode::SliceDepth
                    | GraphMode::DomainColoring
                    | GraphMode::Flatten
                    | GraphMode::Depth => unreachable!(),
                    GraphMode::Normal => {
                        let mut last = None;
                        let mut lasti = None;
                        for (x, y, z) in data {
                            let (z, w) = z.to_options();
                            last = if !self.show.real() {
                                None
                            } else if let Some(z) = z {
                                let (c, d) = self.draw_point_3d(
                                    *x,
                                    *y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    last,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                            lasti = if !self.show.imag() {
                                None
                            } else if let Some(w) = w {
                                let (c, d) = self.draw_point_3d(
                                    *x,
                                    *y,
                                    w,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    lasti,
                                    None,
                                );
                                pts.extend(d);
                                c
                            } else {
                                None
                            };
                        }
                    }
                },
            }
        }
        self.buffer.extend(pts);
    }
    fn get_color(&self, z: &Complex) -> [u8; 3] {
        let (x, y) = z.to_options();
        let (x, y) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
        let hue = 6.0 * (1.0 - y.atan2(x) / TAU);
        let abs = x.hypot(y);
        let (sat, val) = if self.domain_alternate {
            let sat = (if self.log_scale { abs.log10() } else { abs } * PI)
                .sin()
                .abs()
                .powf(0.125);
            let n1 = x.abs() / (x.abs() + 1.0);
            let n2 = y.abs() / (y.abs() + 1.0);
            let n3 = (n1 * n2).powf(0.0625);
            let n4 = abs.atan() * 2.0 / PI;
            let lig = 0.8 * (n3 * (n4 - 0.5) + 0.5);
            let val = if lig < 0.5 {
                lig * (1.0 + sat)
            } else {
                lig * (1.0 - sat) + sat
            };
            let sat = if val == 0.0 {
                0.0
            } else {
                2.0 * (1.0 - lig / val)
            };
            (sat, val)
        } else {
            let t1 = (if self.log_scale { x.abs().log10() } else { x } * PI).sin();
            let t2 = (if self.log_scale { y.abs().log10() } else { y } * PI).sin();
            let sat = (1.0 + if self.log_scale { abs.log10() } else { abs }.fract()) / 2.0;
            let val = (t1 * t2).abs().powf(0.125);
            (sat, val)
        };
        hsv2rgb(hue, sat, val)
    }
    fn shift_hue(&self, diff: f32, z: f64, color: &Color) -> Color {
        match self.color_depth {
            DepthColor::Vertical => shift_hue((z / (2.0 * self.bound.y)) as f32, color),
            DepthColor::Depth => shift_hue(diff, color),
            DepthColor::None => *color,
        }
    }
}
fn hsv2rgb(hue: f64, sat: f64, val: f64) -> [u8; 3] {
    if sat == 0.0 {
        return rgb2val(val, val, val);
    }
    let i = hue.floor();
    let f = hue.fract();
    let p = val * (1.0 - sat);
    let q = val * (1.0 - sat * f);
    let t = val * (1.0 - sat * (1.0 - f));
    match i as usize % 6 {
        0 => rgb2val(val, t, p),
        1 => rgb2val(q, val, p),
        2 => rgb2val(p, val, t),
        3 => rgb2val(p, q, val),
        4 => rgb2val(t, p, val),
        _ => rgb2val(val, p, q),
    }
}
fn rgb2val(r: f64, g: f64, b: f64) -> [u8; 3] {
    [(255.0 * r) as u8, (255.0 * g) as u8, (255.0 * b) as u8]
}
pub fn get_lch(color: [f32; 3]) -> (f32, f32, f32) {
    let c = (color[1].powi(2) + color[2].powi(2)).sqrt();
    let h = color[2].atan2(color[1]);
    (color[0], c, h)
}
#[allow(clippy::excessive_precision)]
pub fn rgb_to_oklch(color: &mut [f32; 3]) {
    let mut l = 0.4122214694707629 * color[0]
        + 0.5363325372617349 * color[1]
        + 0.0514459932675022 * color[2];
    let mut m = 0.2119034958178251 * color[0]
        + 0.6806995506452344 * color[1]
        + 0.1073969535369405 * color[2];
    let mut s = 0.0883024591900564 * color[0]
        + 0.2817188391361215 * color[1]
        + 0.6299787016738222 * color[2];
    l = l.cbrt();
    m = m.cbrt();
    s = s.cbrt();
    color[0] = 0.210454268309314 * l + 0.7936177747023054 * m - 0.0040720430116193 * s;
    color[1] = 1.9779985324311684 * l - 2.42859224204858 * m + 0.450593709617411 * s;
    color[2] = 0.0259040424655478 * l + 0.7827717124575296 * m - 0.8086757549230774 * s;
}
#[allow(clippy::excessive_precision)]
fn oklch_to_rgb(color: &mut [f32; 3]) {
    let mut l = color[0] + 0.3963377773761749 * color[1] + 0.2158037573099136 * color[2];
    let mut m = color[0] - 0.1055613458156586 * color[1] - 0.0638541728258133 * color[2];
    let mut s = color[0] - 0.0894841775298119 * color[1] - 1.2914855480194092 * color[2];
    l = l.powi(3);
    m = m.powi(3);
    s = s.powi(3);
    color[0] = 4.07674163607596 * l - 3.3077115392580635 * m + 0.2309699031821046 * s;
    color[1] = -1.2684379732850317 * l + 2.6097573492876887 * m - 0.3413193760026572 * s;
    color[2] = -0.0041960761386754 * l - 0.7034186179359363 * m + 1.7076146940746116 * s;
}
fn shift_hue_by(color: &mut [f32; 3], diff: f32) {
    let diff = std::f32::consts::TAU * diff;
    let (_, c, hue) = get_lch(*color);
    let mut new_hue = (hue + diff) % std::f32::consts::TAU;
    if new_hue.is_sign_negative() {
        new_hue += std::f32::consts::TAU;
    }
    color[1] = c * new_hue.cos();
    color[2] = c * new_hue.sin();
}
fn shift_hue(diff: f32, color: &Color) -> Color {
    let mut color = [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ];
    rgb_to_oklch(&mut color);
    shift_hue_by(&mut color, diff);
    oklch_to_rgb(&mut color);
    Color::new(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
    )
}
