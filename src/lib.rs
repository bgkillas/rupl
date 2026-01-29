mod sidebar;
#[cfg(feature = "skia-vulkan")]
pub mod skia_vulkan;
pub mod types;
mod ui;
use crate::types::*;
use crate::ui::Painter;
#[cfg(feature = "serde")]
use base64::Engine;
#[cfg(feature = "rayon")]
use rayon::slice::ParallelSliceMut;
use std::f64::consts::{PI, TAU};
#[cfg(feature = "serde")]
use std::io::BufRead;
fn is_3d(data: &[GraphType]) -> bool {
    data.iter()
        .any(|c| matches!(c, GraphType::Width3D(_, _, _, _, _) | GraphType::Coord3D(_)))
}
//TODO removing/adding lines should not move disabled spots
#[cfg(target_arch = "wasm32")]
pub use ui::dpr;
#[cfg(target_arch = "wasm32")]
#[cfg(feature = "tiny-skia")]
pub use ui::draw;
#[cfg(target_arch = "wasm32")]
pub use ui::get_canvas;
#[cfg(target_arch = "wasm32")]
pub use ui::resize;
impl Graph {
    ///creates a new struct where data is the initial set of data to be painted
    ///
    ///names are the labels of the functions which will be painted and
    ///must be in order of data vector to get correct colors, empty name strings will be ignored.
    ///
    ///is_complex is weather the graph contains imaginary elements or not,
    ///will change what graph modes are available
    ///
    ///start,end are the initial visual bounds of the box
    #[allow(clippy::field_reassign_with_default)]
    pub fn new(
        data: Vec<GraphType>,
        names: Vec<Name>,
        is_complex: bool,
        start: f64,
        end: f64,
    ) -> Self {
        let is_3d = is_3d(&data);
        let bound = Vec2::new(start, end);
        let mut graph = Graph::default();
        graph.is_3d = is_3d;
        graph.names = names;
        graph.data = data;
        graph.is_complex = is_complex;
        graph.is_3d_data = is_3d;
        graph.bound = bound;
        graph.var = bound;
        graph
    }
    #[cfg(any(feature = "skia-vulkan", feature = "serde"))]
    ///closes vulkan instance or just saves if applicable
    pub fn close(&mut self) {
        #[cfg(feature = "skia-vulkan")]
        {
            self.renderer = None;
        }
        #[cfg(feature = "serde")]
        if self.save_num.is_some() {
            self.save_num = None;
            self.save()
        }
    }
    #[cfg(feature = "skia-vulkan")]
    ///resizes window
    pub fn resize(&mut self) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.invalidate_swapchain();
        }
    }
    #[cfg(feature = "skia-vulkan")]
    ///needed to setup vulkan window
    pub fn resumed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window: std::sync::Arc<winit::window::Window>,
    ) {
        let renderer = self
            .render_ctx
            .renderer_for_window(event_loop, window.clone());
        self.renderer = Some(renderer);
    }
    #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
    ///is cursor dragging a value or not
    pub fn is_drag(&self) -> bool {
        self.side_drag.is_some() || self.side_slider.is_some()
    }
    #[cfg(any(feature = "skia", feature = "tiny-skia-text"))]
    ///sets font
    pub fn set_font(&mut self, bytes: &[u8]) {
        #[cfg(feature = "skia")]
        {
            let typeface = skia_safe::FontMgr::default()
                .new_from_data(bytes, None)
                .unwrap();
            self.font = Some(skia_safe::Font::new(typeface, self.font_size));
        }
        #[cfg(feature = "tiny-skia-text")]
        {
            self.font = bdf2::read(bytes).ok();
            self.font_cache = build_cache(&self.font, self.text_color);
        }
        self.font_width = 0.0;
    }
    ///sets the font color
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
        #[cfg(feature = "tiny-skia-text")]
        {
            self.font_cache = build_cache(&self.font, self.text_color)
        }
    }
    ///sets if complex graph or not
    pub fn set_is_complex(&mut self, new: bool) {
        self.is_complex = new;
        match self.graph_mode {
            GraphMode::Depth | GraphMode::DomainColoring | GraphMode::Flatten => {
                self.graph_mode = GraphMode::Normal;
                self.is_3d = self.is_3d_data;
                self.recalculate(None);
            }
            _ => {}
        }
    }
    //use dark mode default colors
    pub fn set_dark_mode(&mut self) {
        self.axis_color = Color::splat(220);
        self.axis_color_light = Color::splat(35);
        self.set_text_color(Color::splat(255));
        self.background_color = Color::splat(0);
    }
    //use light mode default colors
    pub fn set_light_mode(&mut self) {
        self.axis_color = Color::splat(0);
        self.axis_color_light = Color::splat(220);
        self.set_text_color(Color::splat(0));
        self.background_color = Color::splat(255);
    }
    ///sets font size
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        self.font_width = 0.0;
    }
    ///removes data in nth slot
    pub fn remove_data(&mut self, n: usize) -> GraphType {
        self.data.remove(n)
    }
    ///takes the data storage object
    pub fn take_data(&mut self) -> Vec<GraphType> {
        std::mem::take(&mut self.data)
    }
    ///insert and replace data into nth slot
    pub fn insert_data(&mut self, data: GraphType, n: usize) {
        self.data.insert(n, data);
    }
    ///sets data and resets domain coloring cache
    pub fn set_data(&mut self, data: Vec<GraphType>) {
        self.data = data;
        self.cache = None;
    }
    pub(crate) fn reset_offset(&self, width: f64, height: f64) -> Vec2 {
        let (_, _, screen) = self.get_new_screen(width, height, true);
        let s = screen - self.screen;
        let (a, b) = (self.offset.x / self.screen.x, self.offset.y / self.screen.y);
        let b = (a + b) / 2.0;
        Vec2 {
            x: s.x * a,
            y: s.y * b,
        }
    }
    fn get_new_screen(&self, width: f64, height: f64, offset: bool) -> (f64, f64, Vec2) {
        let fw =
            ((self.get_longest() as f32 * self.font_width) as f64 + 4.0).max(self.side_bar_width);
        let new = (height * self.target_side_ratio)
            .min(width - self.min_side_width.max(fw))
            .max(self.min_screen_width);
        let screen = if !matches!(self.menu, Menu::Normal) && offset {
            if height < width {
                Vec2::new(new, height)
            } else {
                Vec2::new(width, width)
            }
        } else {
            Vec2::new(width, height)
        };
        (fw, new, screen)
    }
    ///sets screen dimensions
    pub fn set_screen(&mut self, width: f64, height: f64, offset: bool, reset: bool) {
        let (fw, new, screen);
        (fw, new, screen) = self.get_new_screen(width, height, offset);
        let mut c = None;
        if screen != self.screen {
            if self.screen != Vec2::splat(0.0) && offset && reset {
                c = Some(self.to_coord((self.screen / 2.0).to_pos()).into());
                self.offset.x += self.reset_offset(width, height).x;
            }
            self.screen = screen;
        }
        self.side_bar_width = if !matches!(self.menu, Menu::Normal) {
            fw
        } else {
            0.0
        };
        self.draw_offset = if !matches!(self.menu, Menu::Normal) && offset && height < width {
            Pos::new((width - new) as f32, 0.0)
        } else {
            Pos::new(0.0, 0.0)
        };
        let t = Vec2::new(
            self.screen.x * 0.5 - (self.delta * (self.bound.x + self.bound.y) * 0.5),
            self.screen.y * 0.5,
        );
        if t != self.screen_offset && offset {
            if self.graph_mode == GraphMode::DomainColoring {
                self.recalculate(None);
            }
            self.screen_offset = t;
        }
        if let Some(c) = c {
            self.offset.y = self.get_new_offset(c).y;
        }
    }
    ///clears data and domain coloring cache
    pub fn clear_data(&mut self) {
        self.data.clear();
        self.cache = None;
    }
    ///resets current 3d view based on the data that is supplied
    pub fn reset_3d(&mut self) {
        self.set_is_3d(is_3d(&self.data));
    }
    ///resets current 3d view based on the data that is supplied, doesn't effect anything if data hasn't changed type
    pub fn reset_3d_if_changed(&mut self) {
        let is_3d = is_3d(&self.data);
        if is_3d != self.is_3d_data {
            self.set_is_3d(is_3d)
        }
    }
    ///sets if the next set of data is expected to be 3d or not
    pub fn set_is_3d(&mut self, new: bool) {
        self.is_3d_data = new;
        match self.graph_mode {
            GraphMode::Normal | GraphMode::Flatten | GraphMode::Polar => self.is_3d = new,
            GraphMode::Slice | GraphMode::DomainColoring | GraphMode::SlicePolar if !new => {
                self.graph_mode = GraphMode::Normal
            }
            GraphMode::Depth => {}
            _ => {}
        }
        if let Some(n) = self.name_updated.as_mut() {
            *n = usize::MAX
        }
    }
    ///sets the current graph_mode and reprocesses is_3d
    pub fn set_mode(&mut self, mode: GraphMode) {
        match mode {
            GraphMode::DomainColoring
            | GraphMode::Slice
            | GraphMode::Flatten
            | GraphMode::SlicePolar => self.is_3d = false,
            GraphMode::Depth => self.is_3d = true,
            _ => {
                self.is_3d = self.is_3d_data;
            }
        }
        self.graph_mode = mode;
        self.recalculate(None);
    }
    fn fast_3d(&self) -> bool {
        self.is_3d && (self.fast_3d || (self.fast_3d_move && self.mouse_held))
    }
    fn prec(&self) -> f64 {
        if self.mouse_held && !self.is_3d && self.reduced_move {
            (self.prec + 1.0).log10()
        } else {
            self.prec
        }
    }
    //has a name modification taken place
    pub fn is_name_modified(&self) -> bool {
        self.name_modified
    }
    ///run before update_res to support switching if a plot is 2d or 3d
    pub fn update_res_name(&self) -> Option<&[Name]> {
        if self.name_modified {
            Some(&self.names)
        } else {
            None
        }
    }
    pub(crate) fn recalculate(&mut self, name: Option<usize>) {
        if let Some(n) = self.name_updated.as_mut() {
            *n = usize::MAX
        } else {
            self.name_updated = name;
        }
        self.recalculate = true;
    }
    pub(crate) fn name_modified(&mut self, name: Option<usize>) {
        if let Some(n) = self.name_updated.as_mut() {
            *n = usize::MAX
        } else {
            self.name_updated = name;
        }
        self.name_modified = true;
    }
    ///will print the string to the right of the function/var
    pub fn set_constant_eval(&mut self, eval: Vec<(usize, String)>) {
        self.constant_eval = eval
    }
    ///if keybinds does something that requires more data to be generated,
    ///will return a corrosponding UpdateResult asking for more data,
    ///meant to be ran before update()
    pub fn update_res(&mut self) -> Option<(Bound, Option<usize>)> {
        if self.recalculate || self.name_modified {
            self.recalculate = false;
            self.name_modified = false;
            let prec = self.prec();
            Some((
                if self.is_3d_data {
                    match self.graph_mode {
                        GraphMode::Normal => Bound::Width3D(
                            self.bound.x / self.zoom_3d.x + self.offset3d.x,
                            self.bound.x / self.zoom_3d.y - self.offset3d.y,
                            self.bound.y / self.zoom_3d.x + self.offset3d.x,
                            self.bound.y / self.zoom_3d.y - self.offset3d.y,
                            Prec::Mult(self.prec),
                        ),
                        GraphMode::Polar => Bound::Width3D(
                            self.bound.x / self.zoom_3d.x + self.offset3d.x,
                            self.bound.x / self.zoom_3d.y - self.offset3d.y,
                            self.bound.y / self.zoom_3d.x + self.offset3d.x,
                            self.bound.y / self.zoom_3d.y - self.offset3d.y,
                            Prec::Mult(self.prec),
                        ),
                        GraphMode::DomainColoring => {
                            let c = self.to_coord(Pos::new(0.0, 0.0));
                            let cf = self.to_coord(self.screen.to_pos());
                            Bound::Width3D(
                                c.0,
                                c.1,
                                cf.0,
                                cf.1,
                                Prec::Dimension(
                                    (self.screen.x * prec * self.mult) as usize,
                                    (self.screen.y * prec * self.mult) as usize,
                                ),
                            )
                        }
                        GraphMode::Slice => {
                            let c = self.to_coord(Pos::new(0.0, 0.0));
                            let cf = self.to_coord(self.screen.to_pos());
                            if self.view_x {
                                Bound::Width3D(
                                    c.0,
                                    self.bound.x,
                                    cf.0,
                                    self.bound.y,
                                    Prec::Slice(prec),
                                )
                            } else {
                                Bound::Width3D(
                                    self.bound.x,
                                    c.0,
                                    self.bound.y,
                                    cf.0,
                                    Prec::Slice(prec),
                                )
                            }
                        }
                        GraphMode::Flatten => {
                            if self.view_x {
                                Bound::Width3D(
                                    self.var.x,
                                    self.bound.x,
                                    self.var.y,
                                    self.bound.y,
                                    Prec::Slice(self.prec),
                                )
                            } else {
                                Bound::Width3D(
                                    self.bound.x,
                                    self.var.x,
                                    self.bound.y,
                                    self.var.y,
                                    Prec::Slice(self.prec),
                                )
                            }
                        }
                        GraphMode::Depth => {
                            if self.view_x {
                                Bound::Width3D(
                                    self.bound.x / self.zoom_3d.z - self.offset3d.z,
                                    self.bound.x,
                                    self.bound.y / self.zoom_3d.z - self.offset3d.z,
                                    self.bound.y,
                                    Prec::Slice(self.prec),
                                )
                            } else {
                                Bound::Width3D(
                                    self.bound.x,
                                    self.bound.x / self.zoom_3d.z - self.offset3d.z,
                                    self.bound.y,
                                    self.bound.y / self.zoom_3d.z - self.offset3d.z,
                                    Prec::Slice(self.prec),
                                )
                            }
                        }
                        GraphMode::SlicePolar => {
                            if self.view_x {
                                Bound::Width3D(
                                    self.var.x,
                                    self.bound.x,
                                    self.var.y,
                                    self.bound.y,
                                    Prec::Slice(self.prec),
                                )
                            } else {
                                Bound::Width3D(
                                    self.bound.x,
                                    self.var.x,
                                    self.bound.y,
                                    self.var.y,
                                    Prec::Slice(self.prec),
                                )
                            }
                        }
                    }
                } else if self.graph_mode == GraphMode::Depth {
                    Bound::Width(
                        self.bound.x / self.zoom_3d.z - self.offset3d.z,
                        self.bound.y / self.zoom_3d.z - self.offset3d.z,
                        Prec::Mult(self.prec),
                    )
                } else if !self.is_3d {
                    if self.graph_mode == GraphMode::Flatten || self.graph_mode == GraphMode::Polar
                    {
                        Bound::Width(self.var.x, self.var.y, Prec::Mult(prec))
                    } else {
                        let c = self.to_coord(Pos::new(0.0, 0.0));
                        let cf = self.to_coord(self.screen.to_pos());
                        Bound::Width(c.0, cf.0, Prec::Mult(prec))
                    }
                } else {
                    return None;
                },
                std::mem::take(&mut self.name_updated)
                    .map(|n| if n == usize::MAX { None } else { Some(n) })
                    .unwrap_or(None),
            ))
        } else {
            None
        }
    }
    #[cfg(feature = "egui")]
    ///repaints the screen
    pub fn update(&mut self, ctx: &egui::Context, ui: &egui::Ui) {
        self.font_width(ctx);
        let rect = ctx.available_rect();
        let (width, height) = (rect.width() as f64, rect.height() as f64);
        self.set_screen(width, height, true, true);
        let mut painter = Painter::new(ui, self.draw_offset);
        let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter, ui);
        self.update_inner(&mut painter, plot, width, height);
    }
    #[cfg(feature = "skia")]
    #[cfg(not(feature = "skia-vulkan"))]
    ///repaints the screen
    pub fn update<T>(&mut self, width: u32, height: u32, buffer: &mut T)
    where
        T: std::ops::DerefMut<Target = [u32]>,
    {
        let mut canvas = std::mem::take(&mut self.canvas);
        if canvas.is_none() {
            canvas = Some(get_surface(width as i32, height as i32));
        }
        let Some(canvas) = canvas else { unreachable!() };
        let mut canvas = if (width as i32, height as i32) == (canvas.width(), canvas.height()) {
            canvas
        } else {
            get_surface(width as i32, height as i32)
        };
        self.get_img(width, height, buffer, &mut canvas);
        self.canvas = Some(canvas);
    }
    #[cfg(feature = "skia")]
    #[cfg(feature = "skia-vulkan")]
    ///repaints the screen
    pub fn update(&mut self) {
        let Some(mut renderer) = std::mem::take(&mut self.renderer) else {
            return;
        };
        renderer.prepare_swapchain();
        self.font_width();
        renderer.draw_and_present(|surface, size| {
            let (width, height) = (size.width, size.height);
            self.set_screen(width as f64, height as f64, true, true);
            let mut painter = Painter::new(
                surface,
                self.background_color,
                self.anti_alias,
                self.draw_offset,
            );
            let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter);
            self.update_inner(&mut painter, plot, width as f64, height as f64);
        });
        self.renderer = Some(renderer);
    }
    #[cfg(feature = "skia")]
    #[cfg(any(feature = "arboard", not(feature = "skia-vulkan")))]
    fn get_img<T>(
        &mut self,
        width: u32,
        height: u32,
        _buffer: &mut T,
        surface: &mut skia_safe::Surface,
    ) where
        T: std::ops::DerefMut<Target = [u32]>,
    {
        self.font_width();
        self.set_screen(width as f64, height as f64, true, true);
        let mut painter = Painter::new(
            surface,
            self.background_color,
            self.anti_alias,
            self.draw_offset,
        );
        let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter);
        self.update_inner(&mut painter, plot, width as f64, height as f64);
        #[cfg(not(target_arch = "wasm32"))]
        painter.save(_buffer);
    }
    #[cfg(feature = "skia")]
    ///get png data
    pub fn get_png(&mut self, width: u32, height: u32) -> ui::Data {
        self.font_width();
        self.set_screen(width as f64, height as f64, true, true);
        let mut surface = get_surface(width as i32, height as i32);
        let mut painter = Painter::new(
            &mut surface,
            self.background_color,
            self.anti_alias,
            self.draw_offset,
        );
        let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter);
        self.update_inner(&mut painter, plot, width as f64, height as f64);
        painter.save_img(&self.image_format)
    }
    #[cfg(feature = "tiny-skia")]
    ///repaints the screen
    pub fn update<T>(&mut self, width: u32, height: u32, buffer: &mut T)
    where
        T: std::ops::DerefMut<Target = [u32]>,
    {
        let mut canvas = std::mem::take(&mut self.canvas);
        if canvas.is_none() {
            canvas = Some(tiny_skia::Pixmap::new(width, height).unwrap());
        }
        let Some(canvas) = canvas else { unreachable!() };
        let canvas = if (width, height) == (canvas.width(), canvas.height()) {
            canvas
        } else {
            tiny_skia::Pixmap::new(width, height).unwrap()
        };
        self.canvas = Some(self.get_img(width, height, buffer, canvas))
    }
    #[cfg(feature = "tiny-skia")]
    fn get_img<T>(
        &mut self,
        width: u32,
        height: u32,
        _buffer: &mut T,
        canvas: tiny_skia::Pixmap,
    ) -> tiny_skia::Pixmap
    where
        T: std::ops::DerefMut<Target = [u32]>,
    {
        self.font_width();
        self.set_screen(width as f64, height as f64, true, true);
        let mut painter = Painter::new(
            self.background_color,
            self.anti_alias,
            self.draw_offset,
            canvas,
        );
        let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter);
        self.update_inner(&mut painter, plot, width as f64, height as f64);
        #[cfg(not(target_arch = "wasm32"))]
        painter.save(_buffer);
        painter.canvas
    }
    #[cfg(feature = "tiny-skia-png")]
    ///get png data
    pub fn get_png(&mut self, width: u32, height: u32) -> ui::Data {
        let canvas = tiny_skia::Pixmap::new(width, height).unwrap();
        self.font_width();
        self.set_screen(width as f64, height as f64, true, true);
        let mut painter = Painter::new(
            self.background_color,
            self.anti_alias,
            self.draw_offset,
            canvas,
        );
        let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter);
        self.update_inner(&mut painter, plot, width as f64, height as f64);
        let data = painter.save_png();
        ui::Data { data }
    }
    #[cfg(feature = "wasm-draw")]
    ///repaints the screen
    pub fn update(&mut self, width: u32, height: u32) {
        self.font_width();
        self.set_screen(width as f64, height as f64, true, true);
        let mut painter = Painter::new(self.background_color, self.anti_alias, self.draw_offset);
        let plot = |painter: &mut Painter, graph: &mut Graph| graph.plot(painter);
        self.update_inner(&mut painter, plot, width as f64, height as f64);
    }
    fn update_inner<F>(&mut self, painter: &mut Painter, plot: F, width: f64, height: f64)
    where
        F: Fn(&mut Painter, &mut Graph) -> Option<Vec<(f32, Draw, Color)>>,
    {
        self.delta = if self.is_3d {
            self.screen.x.min(self.screen.y)
        } else {
            self.screen.x
        } / (self.bound.y - self.bound.x);
        if !self.is_3d {
            if self.graph_mode == GraphMode::DomainColoring {
                plot(painter, self);
                self.write_axis(painter);
            } else if self.is_polar() {
                self.write_polar_axis(painter);
                plot(painter, self);
            } else {
                self.write_axis(painter);
                plot(painter, self);
            }
            self.write_text(painter);
        } else {
            (self.sin_phi, self.cos_phi) = self.angle.x.sin_cos();
            (self.sin_theta, self.cos_theta) = self.angle.y.sin_cos();
            let mut buffer = plot(painter, self);
            self.write_axis_3d(painter, &mut buffer);
            if let Some(mut buffer) = buffer {
                #[cfg(feature = "rayon")]
                buffer.par_sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
                #[cfg(not(feature = "rayon"))]
                buffer.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
                for (_, a, c) in buffer {
                    match a {
                        Draw::Line(a, b, width) => {
                            painter.line_segment([a, b], width, &c);
                        }
                        Draw::Point(a) => {
                            painter.rect_filled(a, &c, self.point_size);
                        }
                    }
                }
            }
        }
        let draw = !matches!(self.menu, Menu::Normal);
        if !self.is_3d {
            self.write_coord(painter);
        } else {
            self.write_angle(painter);
        }
        self.write_label(painter);
        if draw {
            self.set_screen(width, height, false, false);
            if painter.offset.x == painter.offset.y && painter.offset.x == 0.0 {
                painter.clear_below(self.screen, &self.background_color)
            } else {
                painter.clear_offset(self.screen, &self.background_color);
            }
            self.write_side(painter);
            self.set_screen(width, height, true, false);
        }
    }
    fn write_label(&self, painter: &mut Painter) {
        let mut pos = Pos::new(self.screen.x as f32 - 48.0, 0.0);
        let blacklist = self
            .blacklist_graphs
            .iter()
            .filter_map(|i| self.index_to_name(*i, true).0)
            .collect::<Vec<usize>>();
        for (i, Name { name, show, .. }) in self.names.iter().enumerate().filter_map(|(i, n)| {
            if !n.name.is_empty() && !blacklist.contains(&i) {
                Some((i, n))
            } else {
                None
            }
        }) {
            let y = (pos.y + 3.0 * self.font_size / 4.0).round();
            let o = 3.5;
            match self.graph_mode {
                GraphMode::DomainColoring => {}
                GraphMode::Flatten | GraphMode::Depth => {
                    self.text_color(pos, Align::RightTop, name, painter);
                    painter.line_segment(
                        [
                            Pos::new(pos.x + o, y),
                            Pos::new(self.screen.x as f32 - o, y),
                        ],
                        self.line_width,
                        &self.main_colors[i % self.main_colors.len()],
                    );
                }
                GraphMode::SlicePolar | GraphMode::Polar | GraphMode::Normal | GraphMode::Slice => {
                    match show {
                        Show::Real => {
                            self.text_color(pos, Align::RightTop, name, painter);
                            painter.line_segment(
                                [
                                    Pos::new(pos.x + o, y),
                                    Pos::new(self.screen.x as f32 - o, y),
                                ],
                                self.line_width,
                                &self.main_colors[i % self.main_colors.len()],
                            );
                        }
                        Show::Imag => {
                            self.text_color(pos, Align::RightTop, &format!("im:{name}"), painter);
                            painter.line_segment(
                                [
                                    Pos::new(pos.x + o, y),
                                    Pos::new(self.screen.x as f32 - o, y),
                                ],
                                self.line_width,
                                &self.alt_colors[i % self.alt_colors.len()],
                            );
                        }
                        Show::Complex => {
                            self.text_color(pos, Align::RightTop, &format!("re:{name}"), painter);
                            painter.line_segment(
                                [
                                    Pos::new(pos.x + o, y),
                                    Pos::new(self.screen.x as f32 - o, y),
                                ],
                                self.line_width,
                                &self.main_colors[i % self.main_colors.len()],
                            );
                            pos.y += self.font_size;
                            let y = y + self.font_size;
                            self.text_color(pos, Align::RightTop, &format!("im:{name}"), painter);
                            painter.line_segment(
                                [
                                    Pos::new(pos.x + o, y),
                                    Pos::new(self.screen.x as f32 - o, y),
                                ],
                                self.line_width,
                                &self.alt_colors[i % self.alt_colors.len()],
                            );
                        }
                        Show::None => {}
                    }
                }
            }
            pos.y += self.font_size;
        }
    }
    fn write_coord(&self, painter: &mut Painter) {
        if self.mouse_moved
            && let Some(pos) = self.mouse_position
        {
            let p = self.to_coord(pos.to_pos());
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
                                "{:E}\n{:E}\n{:E}\n{:E}\n{:E}\n{}",
                                p.0,
                                p.1,
                                x,
                                y,
                                y.hypot(x),
                                self.angle_type.to_val(y.atan2(x))
                            )
                        } else {
                            format!("{:E}\n{:E}", p.0, p.1)
                        }
                    } else {
                        format!("{:E}\n{:E}", p.0, p.1)
                    }
                } else if matches!(self.graph_mode, GraphMode::Polar | GraphMode::SlicePolar) {
                    format!(
                        "{:E}\n{}",
                        p.1.hypot(p.0),
                        self.angle_type.to_val(p.1.atan2(p.0))
                    )
                } else {
                    format!("{:E}\n{:E}", p.0, p.1)
                };
                self.text(
                    Pos::new(0.0, self.screen.y as f32),
                    Align::LeftBottom,
                    &s,
                    &self.text_color,
                    painter,
                );
            }
            if let Some(ps) = self.ruler_pos {
                let dx = p.0 - ps.x;
                let dy = p.1 - ps.y;
                self.text(
                    self.screen.to_pos(),
                    Align::RightBottom,
                    &format!(
                        "{:E}\n{:E}\n{:E}\n{}",
                        dx,
                        dy,
                        dy.hypot(dx),
                        self.angle_type.to_val(dy.atan2(dx))
                    ),
                    &self.text_color,
                    painter,
                );
                painter.line_segment(
                    [pos.to_pos(), self.to_screen(ps.x, ps.y)],
                    1.0,
                    &self.axis_color,
                );
            }
        }
    }
    #[cfg(feature = "wasm-draw")]
    fn text(
        &self,
        pos: Pos,
        align: Align,
        text: &str,
        color: &Color,
        painter: &mut Painter,
    ) -> f32 {
        painter.text(pos, align, text, color)
    }
    #[cfg(feature = "skia")]
    fn text(&self, pos: Pos, align: Align, text: &str, col: &Color, painter: &mut Painter) -> f32 {
        painter.text(pos, align, text, col, &self.font)
    }
    #[cfg(feature = "tiny-skia")]
    #[cfg(not(feature = "tiny-skia-text"))]
    fn text(&self, _: Pos, _: Align, _: &str, _: &Color, _: &mut Painter) -> f32 {
        0.0
    }
    #[cfg(feature = "tiny-skia-text")]
    fn text(&self, pos: Pos, align: Align, text: &str, _: &Color, painter: &mut Painter) -> f32 {
        painter.text(pos, align, text, &self.font_cache, &self.font)
    }
    fn text_color(&self, mut pos: Pos, align: Align, text: &str, painter: &mut Painter) {
        match align {
            Align::LeftCenter | Align::LeftBottom | Align::LeftTop => {
                for (c, s) in self.color_string(text) {
                    pos.x += self.text(pos, align, s, &c, painter);
                }
            }
            Align::RightCenter | Align::RightBottom | Align::RightTop => {
                for (c, s) in self.color_string(text).iter().rev() {
                    pos.x -= self.text(pos, align, s, c, painter);
                }
            }
            _ => unreachable!(),
        }
    }
    #[cfg(feature = "egui")]
    fn text(&self, pos: Pos, align: Align, text: &str, col: &Color, painter: &mut Painter) -> f32 {
        painter.text(pos, align, text, col, self.font_size)
    }
    fn write_angle(&self, painter: &mut Painter) {
        if !self.disable_coord {
            self.text(
                Pos::new(0.0, self.screen.y as f32),
                Align::LeftBottom,
                &format!(
                    "{}\n{}",
                    (self.angle.x / TAU * 360.0).round(),
                    ((0.25 - self.angle.y / TAU) * 360.0)
                        .round()
                        .rem_euclid(360.0),
                ),
                &self.text_color,
                painter,
            );
        }
    }
    fn to_screen(&self, x: f64, y: f64) -> Pos {
        let s = self.screen.x / (self.bound.y - self.bound.x);
        let ox = self.screen_offset.x + self.offset.x;
        let oy = self.screen_offset.y + self.offset.y;
        Pos::new(
            ((x * s + ox) * self.zoom.x) as f32,
            ((oy - y * s) * self.zoom.y) as f32,
        )
    }
    fn to_coord(&self, p: Pos) -> (f64, f64) {
        let ox = self.offset.x + self.screen_offset.x;
        let oy = self.offset.y + self.screen_offset.y;
        let s = (self.bound.y - self.bound.x) / self.screen.x;
        let x = (p.x as f64 / self.zoom.x - ox) * s;
        let y = (oy - p.y as f64 / self.zoom.y) * s;
        (x, y)
    }
    fn get_new_offset(&self, mut o: Vec2) -> Vec2 {
        let s = (self.bound.y - self.bound.x) / self.screen.x;
        o /= s;
        let x = self.screen.x / (self.zoom.x * 2.0) - o.x - self.screen_offset.x;
        let y = o.y - self.screen_offset.y + self.screen.y / (self.zoom.y * 2.0);
        Vec2::new(x, y)
    }
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
        let is_in = self.in_screen(pos);
        if !matches!(self.lines, Lines::Lines) && is_in {
            painter.rect_filled(pos, color, self.point_size);
        }
        if !matches!(self.lines, Lines::Points) {
            if let Some(last) = last
                && (is_in || self.in_screen(last))
            {
                painter.line_segment([last, pos], self.line_width, color);
            }
            Some(pos)
        } else {
            None
        }
    }
    fn in_screen(&self, p: Pos) -> bool {
        p.x > -2.0
            && p.x < self.screen.x as f32 + 2.0
            && p.y > -2.0
            && p.y < self.screen.y as f32 + 2.0
    }
    fn write_polar_axis(&self, painter: &mut Painter) {
        let o = self.to_screen(0.0, 0.0);
        if !self.disable_lines && !self.disable_axis {
            for y in [self.screen.x as f32, 0.0] {
                for l in [o.x - y, y - o.x] {
                    painter.line_segment(
                        [o, Pos::new(y, o.y + l * (2.0 - 3.0f32.sqrt()))],
                        1.0,
                        &self.axis_color_light,
                    );
                    painter.line_segment(
                        [o, Pos::new(y, o.y + l * (2.0 + 3.0f32.sqrt()))],
                        1.0,
                        &self.axis_color_light,
                    );
                    painter.line_segment([o, Pos::new(y, o.y + l)], 1.0, &self.axis_color_light);
                    painter.line_segment(
                        [o, Pos::new(y, o.y + l / 3.0f32.sqrt())],
                        1.0,
                        &self.axis_color_light,
                    );
                    painter.line_segment(
                        [o, Pos::new(y, o.y + l * 3.0f32.sqrt())],
                        1.0,
                        &self.axis_color_light,
                    );
                }
            }
        }
        if !self.disable_axis {
            let or = self.to_coord(self.screen.to_pos() / 2.0);
            fn norm((x, y): (f64, f64)) -> f64 {
                x.hypot(y)
            }
            let s = if o.x > 0.0
                && (o.x as f64) < self.screen.x
                && o.y > 0.0
                && (o.y as f64) < self.screen.y
            {
                -1.0
            } else {
                1.0
            };
            let (a, b) = if (or.0 >= 0.0) == (or.1 <= 0.0) {
                let (a, b) = (
                    norm(self.to_coord(Pos::new(0.0, 0.0))),
                    norm(self.to_coord(self.screen.to_pos())),
                );
                (s * a.min(b), a.max(b))
            } else {
                let (a, b) = (
                    norm(self.to_coord(Pos::new(0.0, self.screen.y as f32))),
                    norm(self.to_coord(Pos::new(self.screen.x as f32, 0.0))),
                );
                (s * a.min(b), a.max(b))
            };
            let delta = 2.0f64.powf((-self.zoom.x.log2()).round());
            let minor = (self.line_major * self.line_minor) as f64 * self.screen.x
                / (2.0 * self.delta * delta * (self.bound.y - self.bound.x).powi(2));
            let s = self.screen.x / (self.bound.y - self.bound.x);
            let ox = self.screen_offset.x + self.offset.x;
            let nx = (((self.to_screen(a, 0.0).x as f64 / self.zoom.x - ox) / s) * 2.0 * minor)
                .ceil() as isize;
            let mx = (((self.to_screen(b, 0.0).x as f64 / self.zoom.x - ox) / s) * 2.0 * minor)
                .floor() as isize;
            for j in nx.max(1)..=mx {
                if j % 4 != 0 {
                    let x = self.to_screen(j as f64 / (2.0 * minor), 0.0).x;
                    painter.circle(o, x - o.x, &self.axis_color_light, 1.0);
                }
            }
            let minor = minor / self.line_minor as f64;
            let nx = (((self.to_screen(a, 0.0).x as f64 / self.zoom.x - ox) / s) * 2.0 * minor)
                .ceil() as isize;
            let mx = (((self.to_screen(b, 0.0).x as f64 / self.zoom.x - ox) / s) * 2.0 * minor)
                .floor() as isize;
            for j in nx.max(1)..=mx {
                let x = self.to_screen(j as f64 / (2.0 * minor), 0.0).x;
                painter.circle(o, x - o.x, &self.axis_color, 1.0);
            }
            painter.vline(o.x, self.screen.y as f32, &self.axis_color);
            painter.hline(self.screen.x as f32, o.y, &self.axis_color);
        }
    }
    fn write_axis(&self, painter: &mut Painter) {
        let deltax = 2.0f64.powf((-self.zoom.x.log2()).round());
        let deltay = 2.0f64.powf((-self.zoom.y.log2()).round());
        let minorx = (self.line_major * self.line_minor) as f64 * self.screen.x
            / (2.0 * self.delta * deltax * (self.bound.y - self.bound.x).powi(2));
        let minory = (self.line_major * self.line_minor) as f64 * self.screen.x
            / (2.0 * self.delta * deltay * (self.bound.y - self.bound.x).powi(2));
        let s = self.screen.x / (self.bound.y - self.bound.x);
        let ox = self.screen_offset.x + self.offset.x;
        let oy = self.screen_offset.y + self.offset.y;
        if !self.disable_lines && self.graph_mode != GraphMode::DomainColoring {
            let nx = (((-1.0 / self.zoom.x - ox) / s) * 2.0 * minorx).ceil() as isize;
            let ny = (((oy + 1.0 / self.zoom.y) / s) * 2.0 * minory).ceil() as isize;
            let mx =
                ((((self.screen.x + 1.0) / self.zoom.x - ox) / s) * 2.0 * minorx).floor() as isize;
            let my =
                (((oy - (self.screen.y + 1.0) / self.zoom.y) / s) * 2.0 * minory).floor() as isize;
            for j in nx..=mx {
                if j % 4 != 0 {
                    let x = self.to_screen(j as f64 / (2.0 * minorx), 0.0).x;
                    painter.vline(x, self.screen.y as f32, &self.axis_color_light);
                }
            }
            for j in my..=ny {
                if j % 4 != 0 {
                    let y = self.to_screen(0.0, j as f64 / (2.0 * minory)).y;
                    painter.hline(self.screen.x as f32, y, &self.axis_color_light);
                }
            }
        }
        let minorx = minorx / self.line_minor as f64;
        let minory = minory / self.line_minor as f64;
        let nx = (((-1.0 / self.zoom.x - ox) / s) * 2.0 * minorx).ceil() as isize;
        let mx = ((((self.screen.x + 1.0) / self.zoom.x - ox) / s) * 2.0 * minorx).floor() as isize;
        let ny = (((oy + 1.0 / self.zoom.y) / s) * 2.0 * minory).ceil() as isize;
        let my = (((oy - (self.screen.y + 1.0) / self.zoom.y) / s) * 2.0 * minory).floor() as isize;
        if !self.disable_lines {
            for j in nx..=mx {
                let x = self.to_screen(j as f64 / (2.0 * minorx), 0.0).x;
                painter.vline(x, self.screen.y as f32, &self.axis_color);
            }
            for j in my..=ny {
                let y = self.to_screen(0.0, j as f64 / (2.0 * minory)).y;
                painter.hline(self.screen.x as f32, y, &self.axis_color);
            }
        } else if !self.disable_axis {
            if (nx..=mx).contains(&0) {
                let x = self.to_screen(0.0, 0.0).x;
                painter.vline(x, self.screen.y as f32, &self.axis_color);
            }
            if (my..=ny).contains(&0) {
                let y = self.to_screen(0.0, 0.0).y;
                painter.hline(self.screen.x as f32, y, &self.axis_color);
            }
        }
    }
    fn write_text(&self, painter: &mut Painter) {
        let deltax = 2.0f64.powf((-self.zoom.x.log2()).round());
        let deltay = 2.0f64.powf((-self.zoom.y.log2()).round());
        let minorx = self.line_major as f64 * self.screen.x
            / (2.0 * self.delta * deltax * (self.bound.y - self.bound.x).powi(2));
        let minory = self.line_major as f64 * self.screen.x
            / (2.0 * self.delta * deltay * (self.bound.y - self.bound.x).powi(2));
        let s = self.screen.x / (self.bound.y - self.bound.x);
        let ox = self.screen_offset.x + self.offset.x;
        let oy = self.screen_offset.y + self.offset.y;
        let nx = (((-1.0 / self.zoom.x - ox) / s) * 2.0 * minorx).ceil() as isize;
        let mx = ((((self.screen.x + 1.0) / self.zoom.x - ox) / s) * 2.0 * minorx).floor() as isize;
        let ny = (((oy + 1.0 / self.zoom.y) / s) * 2.0 * minory).ceil() as isize;
        let my = (((oy - (self.screen.y + 1.0) / self.zoom.y) / s) * 2.0 * minory).floor() as isize;
        if !self.disable_axis {
            let mut align = false;
            let y = if (my..ny).contains(&0) {
                self.to_screen(0.0, 0.0).y
            } else if my.is_negative() {
                0.0
            } else {
                align = true;
                self.screen.y as f32
            };
            for j in nx.saturating_sub(1)..=mx {
                if self.is_polar() && j == 0 {
                    continue;
                }
                let j = j as f64 / (2.0 * minorx);
                let x = self.to_screen(j, 0.0).x;
                let mut p = Pos::new(x + 2.0, y);
                if !align {
                    p.y = p.y.min(self.screen.y as f32 - self.font_size)
                }
                let mut s = j.to_string();
                if s.len() > 8 {
                    s = format!("{j:E}")
                }
                self.text(
                    p,
                    if align {
                        Align::LeftBottom
                    } else {
                        Align::LeftTop
                    },
                    &s,
                    &self.text_color,
                    painter,
                );
            }
            let mut align = false;
            let x = if (nx..=mx).contains(&0) {
                self.to_screen(0.0, 0.0).x
            } else if mx.is_positive() {
                0.0
            } else {
                align = true;
                self.screen.x as f32
            };
            for j in my..=ny.saturating_add(1) {
                if j == 0 {
                    continue;
                }
                let j = j as f64 / (2.0 * minory);
                let y = self.to_screen(0.0, j).y;
                let mut p = Pos::new(x + 2.0, y);
                let mut s = j.to_string();
                if s.len() > 8 {
                    s = format!("{j:E}")
                }
                if !align {
                    p.x =
                        p.x.min(self.screen.x as f32 - self.font_width * s.len() as f32)
                }
                self.text(
                    p,
                    if align {
                        Align::RightTop
                    } else {
                        Align::LeftTop
                    },
                    &s,
                    &self.text_color,
                    painter,
                );
            }
        }
    }
    fn is_polar(&self) -> bool {
        matches!(self.graph_mode, GraphMode::Polar | GraphMode::SlicePolar)
    }
    #[cfg(feature = "tiny-skia-text")]
    fn font_width(&mut self) {
        if self.font_width == 0.0 {
            if let Some(font) = &self.font {
                self.font_width = ui::char_dimen(font).0 as f32;
            }
        }
    }
    #[cfg(feature = "wasm-draw")]
    fn font_width(&mut self) {
        if self.font_width == 0.0 {
            self.font_width = ui::get_bounds(" ").0;
        }
    }
    #[cfg(feature = "tiny-skia")]
    #[cfg(not(feature = "tiny-skia-text"))]
    fn font_width(&mut self) {}
    #[cfg(feature = "skia")]
    fn font_width(&mut self) {
        if self.font_width == 0.0
            && let Some(font) = &self.font
        {
            self.font_width = font.measure_str(" ", None).0;
        }
    }
    #[cfg(feature = "egui")]
    fn font_width(&mut self, ctx: &egui::Context) {
        if self.font_width == 0.0 {
            let width = ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    " ".to_string(),
                    egui::FontId::monospace(self.font_size),
                    Color::splat(0).to_col(),
                )
                .size()
                .x
            });
            self.font_width = width;
        }
    }
    fn vec3_to_pos_depth(&self, mut p: Vec3, edge: bool) -> (Pos, Option<f32>) {
        if edge {
            p *= self.zoom_3d;
        }
        let x1 = p.x * self.cos_phi + p.y * self.sin_phi;
        let y1 = -p.x * self.sin_phi + p.y * self.cos_phi;
        let z2 = -p.z * self.cos_theta - y1 * self.sin_theta;
        let s = self.delta / self.box_size;
        let x = (x1 * s + self.screen.x * 0.5) as f32;
        let y = (z2 * s + self.screen.y * 0.5) as f32;
        (
            Pos::new(x, y),
            (!self.fast_3d()).then(|| {
                ((p.z * self.sin_theta - y1 * self.cos_theta)
                    / ((self.bound.y - self.bound.x) * 3.0f64.sqrt())
                    + 0.5) as f32
            }),
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
        a: Option<((Pos, Option<f32>), Vec3, bool)>,
        b: Option<((Pos, Option<f32>), Vec3, bool)>,
        buffer: &mut Option<Vec<(f32, Draw, Color)>>,
        painter: &mut Painter,
    ) -> Option<((Pos, Option<f32>), Vec3, bool)> {
        let x = x - self.offset3d.x;
        let y = y + self.offset3d.y;
        let z = z + self.offset3d.z;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return None;
        }
        let v = Vec3::new(x, y, z);
        let pos = self.vec3_to_pos_depth(v, true);
        let inside = self.ignore_bounds
            || (x >= self.bound.x / self.zoom_3d.x
                && x <= self.bound.y / self.zoom_3d.x
                && y >= self.bound.x / self.zoom_3d.y
                && y <= self.bound.y / self.zoom_3d.y
                && z >= self.bound.x / self.zoom_3d.z
                && z <= self.bound.y / self.zoom_3d.z);
        if !matches!(self.lines, Lines::Lines) && inside {
            point(
                buffer,
                self.fast_3d().then_some(painter),
                pos.1,
                pos.0,
                self.shift_hue(pos.1, z, color),
                self.point_size,
            );
        }
        if !matches!(self.lines, Lines::Points) {
            let mut body = |last: ((Pos, Option<f32>), Vec3, bool)| {
                if inside && last.2 {
                    let d = (!self.fast_3d()).then(|| (pos.1.unwrap() + last.0.1.unwrap()) * 0.5);
                    line(
                        buffer,
                        self.fast_3d().then_some(painter),
                        d,
                        last.0.0,
                        pos.0,
                        self.shift_hue(d, z, color),
                        self.line_width,
                    );
                } else if inside {
                    let mut vi = last.1;
                    let xi = vi.x;
                    if xi < self.bound.x / self.zoom_3d.x {
                        vi = v + (vi - v) * ((self.bound.x / self.zoom_3d.x - x) / (xi - x));
                    } else if xi > self.bound.y / self.zoom_3d.x {
                        vi = v + (vi - v) * ((self.bound.y / self.zoom_3d.x - x) / (xi - x));
                    }
                    let yi = vi.y;
                    if yi < self.bound.x / self.zoom_3d.y {
                        vi = v + (vi - v) * ((self.bound.x / self.zoom_3d.y - y) / (yi - y));
                    } else if yi > self.bound.y / self.zoom_3d.y {
                        vi = v + (vi - v) * ((self.bound.y / self.zoom_3d.y - y) / (yi - y));
                    }
                    let zi = vi.z;
                    if zi < self.bound.x / self.zoom_3d.z {
                        vi = v + (vi - v) * ((self.bound.x / self.zoom_3d.z - z) / (zi - z));
                    } else if zi > self.bound.y / self.zoom_3d.z {
                        vi = v + (vi - v) * ((self.bound.y / self.zoom_3d.z - z) / (zi - z));
                    }
                    let last = self.vec3_to_pos_depth(vi, true);
                    let d = (!self.fast_3d()).then(|| (pos.1.unwrap() + last.1.unwrap()) * 0.5);
                    line(
                        buffer,
                        self.fast_3d().then_some(painter),
                        d,
                        last.0,
                        pos.0,
                        self.shift_hue(d, z, color),
                        self.line_width,
                    );
                } else if last.2 {
                    let mut vi = v;
                    let v = last.1;
                    let (x, y, z) = (v.x, v.y, v.z);
                    let pos = self.vec3_to_pos_depth(v, true);
                    let xi = vi.x;
                    if xi < self.bound.x / self.zoom_3d.x {
                        vi = v + (vi - v) * ((self.bound.x / self.zoom_3d.x - x) / (xi - x));
                    } else if xi > self.bound.y / self.zoom_3d.x {
                        vi = v + (vi - v) * ((self.bound.y / self.zoom_3d.x - x) / (xi - x));
                    }
                    let yi = vi.y;
                    if yi < self.bound.x / self.zoom_3d.y {
                        vi = v + (vi - v) * ((self.bound.x / self.zoom_3d.y - y) / (yi - y));
                    } else if yi > self.bound.y / self.zoom_3d.y {
                        vi = v + (vi - v) * ((self.bound.y / self.zoom_3d.y - y) / (yi - y));
                    }
                    let zi = vi.z;
                    if zi < self.bound.x / self.zoom_3d.z {
                        vi = v + (vi - v) * ((self.bound.x / self.zoom_3d.z - z) / (zi - z));
                    } else if zi > self.bound.y / self.zoom_3d.z {
                        vi = v + (vi - v) * ((self.bound.y / self.zoom_3d.z - z) / (zi - z));
                    }
                    let last = self.vec3_to_pos_depth(vi, true);
                    let d = (!self.fast_3d()).then(|| (pos.1.unwrap() + last.1.unwrap()) * 0.5);
                    line(
                        buffer,
                        self.fast_3d().then_some(painter),
                        d,
                        last.0,
                        pos.0,
                        self.shift_hue(d, z, color),
                        self.line_width,
                    );
                }
            };
            if let Some(last) = a {
                body(last)
            }
            if let Some(last) = b {
                body(last)
            }
            Some((pos, Vec3::new(x, y, z), inside))
        } else {
            None
        }
    }
    fn write_axis_3d(
        &mut self,
        painter: &mut Painter,
        buffer: &mut Option<Vec<(f32, Draw, Color)>>,
    ) {
        let s = (self.bound.y - self.bound.x) * 0.5;
        let vertices = [
            self.vec3_to_pos_depth(Vec3::new(-s, -s, -s), false),
            self.vec3_to_pos_depth(Vec3::new(-s, -s, s), false),
            self.vec3_to_pos_depth(Vec3::new(-s, s, -s), false),
            self.vec3_to_pos_depth(Vec3::new(-s, s, s), false),
            self.vec3_to_pos_depth(Vec3::new(s, -s, -s), false),
            self.vec3_to_pos_depth(Vec3::new(s, -s, s), false),
            self.vec3_to_pos_depth(Vec3::new(s, s, -s), false),
            self.vec3_to_pos_depth(Vec3::new(s, s, s), false),
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
        let m = if !self.fast_3d() {
            edges
                .iter()
                .map(|(i, j)| vertices[*i].1.unwrap() + vertices[*j].1.unwrap())
                .sum::<f32>()
                / edges.len() as f32
        } else {
            0.0
        };
        //TODO make text not just on ints
        for (k, (i, j)) in edges.iter().enumerate() {
            #[derive(PartialEq)]
            enum Axis {
                X,
                Y,
                Z,
            }
            let s = match k {
                8..=11 => Axis::X,
                1 | 3 | 5 | 7 => Axis::Y,
                0 | 2 | 4 | 6 => Axis::Z,
                _ => unreachable!(),
            };
            if (s == Axis::Z && [i, j].contains(&&zl)) || (s != Axis::Z && [i, j].contains(&&xl)) {
                if !self.disable_axis || self.show_box {
                    line(
                        buffer,
                        self.fast_3d().then_some(painter),
                        (!self.fast_3d()).then(|| {
                            if vertices[*i].1.unwrap() + vertices[*j].1.unwrap() < m {
                                0.0
                            } else {
                                1.0
                            }
                        }),
                        vertices[*i].0,
                        vertices[*j].0,
                        self.axis_color,
                        2.0,
                    );
                }
                if !self.disable_axis {
                    let p = vertices[*i].0 + vertices[*j].0;
                    let align = match s {
                        Axis::X | Axis::Y => Align::CenterTop,
                        Axis::Z => Align::RightCenter,
                    };
                    let start = vertices[*i.min(j)].0;
                    let end = vertices[*i.max(j)].0;
                    let m = match s {
                        Axis::Z => self.zoom_3d.z,
                        Axis::X => self.zoom_3d.x,
                        Axis::Y => self.zoom_3d.y,
                    };
                    let st = (self.bound.x / m).ceil() as isize;
                    let e = (self.bound.y / m).floor() as isize;
                    let o = match s {
                        Axis::Z => self.offset3d.z,
                        Axis::X => -self.offset3d.x,
                        Axis::Y => self.offset3d.y,
                    };
                    let n = ((st + (e - st) / 2) as f64 - o).to_string();
                    self.text(
                        p * 0.5,
                        align,
                        &match s {
                            Axis::Z => format!("z{}", " ".repeat(n.len())),
                            Axis::X => " \nx".to_string(),
                            Axis::Y => " \ny".to_string(),
                        },
                        &self.text_color,
                        painter,
                    );
                    for i in st..=e {
                        self.text(
                            start + (end - start) * ((i - st) as f32 / (e - st) as f32),
                            align,
                            &(i as f64 - o).to_string(),
                            &self.text_color,
                            painter,
                        );
                    }
                }
            } else if self.show_box {
                line(
                    buffer,
                    self.fast_3d().then_some(painter),
                    (!self.fast_3d()).then(|| {
                        if vertices[*i].1.unwrap() + vertices[*j].1.unwrap() < m {
                            0.0
                        } else {
                            1.0
                        }
                    }),
                    vertices[*i].0,
                    vertices[*j].0,
                    self.axis_color,
                    2.0,
                );
            }
        }
    }
    #[cfg(feature = "egui")]
    ///process the current keys and mouse/touch inputs, see Keybinds for more info,
    ///expected to run before update_res()
    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        ui.input(|i| self.keybinds_inner(&i.into()));
        let s = self.clipboard.as_ref().unwrap().0.clone();
        if !s.is_empty() {
            ui.ctx().copy_text(s)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
    ///process the current keys and mouse/touch inputs, see Keybinds for more info,
    ///expected to run before update_res()
    pub fn keybinds(&mut self, i: &InputState) {
        self.keybinds_inner(i)
    }
    #[cfg(target_arch = "wasm32")]
    #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
    ///process the current keys and mouse/touch inputs, see Keybinds for more info,
    ///expected to run before update_res()
    pub fn keybinds(&mut self, i: &InputState) {
        self.keybinds_inner(i);
        if let Some(s) = self.clipboard.as_ref() {
            crate::ui::write_clipboard(&s.0);
        }
    }
    fn keybinds_inner(&mut self, i: &InputState) {
        #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
        {
            self.request_redraw = false;
        }
        let Some(keybinds) = std::mem::take(&mut self.keybinds) else {
            unreachable!()
        };
        #[cfg(feature = "arboard")]
        if self.clipboard.is_none() {
            if !self.wait_frame {
                self.clipboard = Some(Clipboard(arboard::Clipboard::new().unwrap()))
            }
            self.wait_frame = false;
        }
        #[cfg(any(feature = "egui", target_arch = "wasm32"))]
        if let Some(s) = i.clipboard_override.clone() {
            self.clipboard.as_mut().unwrap().0 = s;
        }
        let ret = if !matches!(self.menu, Menu::Normal) {
            self.keybinds_side(i)
        } else {
            false
        };
        if let Some(mpos) = i.pointer_pos {
            let mpos = Vec2 {
                x: mpos.x,
                y: mpos.y,
            } - self.draw_offset.to_vec();
            if let Some(pos) = self.mouse_position {
                if mpos != pos {
                    self.mouse_moved = true;
                    self.mouse_position = Some(mpos)
                }
            } else {
                self.mouse_position = Some(mpos)
            }
            if let Some(right) = i.pointer_right
                && matches!(self.menu, Menu::Side | Menu::Normal)
            {
                if right && mpos.x > 0.0 {
                    let get_d = |p: &Dragable| -> f32 {
                        match p {
                            Dragable::Point(p) | Dragable::Points((_, p)) => {
                                let dx = p.x - mpos.x as f32;
                                let dy = p.y - mpos.y as f32;
                                dx * dx + dy * dy
                            }
                            Dragable::X(x) => {
                                let dx = (x - mpos.x as f32).abs();
                                dx * dx
                            }
                            Dragable::Y(y) => {
                                let dy = (y - mpos.y as f32).abs();
                                dy * dy
                            }
                        }
                    };
                    let mut pts = self.get_points().into_iter().filter(|(_, _, p)| {
                        let v = 32.0;
                        get_d(p) <= v * v
                    });
                    if let Some(min) = pts.next() {
                        let mut min: (f32, (usize, String, Dragable)) = (get_d(&min.2), min);
                        for (a, b, p) in pts {
                            let d = get_d(&p);
                            if d < min.0 {
                                min = (d, (a, b, p))
                            }
                        }
                        let min = min.1;
                        let (mut a, mut b) = self.to_coord(mpos.to_pos());
                        if matches!(self.graph_mode, GraphMode::Polar) {
                            let r = b.hypot(a);
                            let t = b.atan2(a);
                            (a, b) = (t, r);
                        }
                        let s = (a, b);
                        let mut k = None;
                        self.replace_name(
                            min.0,
                            match min.2 {
                                Dragable::Point(_) => format!("{}={{{},{}}}", min.1, s.0, s.1),
                                Dragable::X(_) => format!("{}={}", min.1, s.0),
                                Dragable::Y(_) => format!("{}={}", min.1, s.1),
                                Dragable::Points((i, _)) => {
                                    k = Some(i);
                                    let mut a = self
                                        .get_name(min.0)
                                        .rsplit_once("=")
                                        .unwrap()
                                        .1
                                        .to_string();
                                    a.pop();
                                    a.pop();
                                    let mut a = a
                                        .split("}")
                                        .filter_map(|i| {
                                            let mut i = i.to_string();
                                            i.remove(0);
                                            i.remove(0);
                                            i.rsplit_once(",")
                                                .map(|(a, b)| (a.to_string(), b.to_string()))
                                        })
                                        .collect::<Vec<(String, String)>>();
                                    a[i] = (s.0.to_string(), s.1.to_string());
                                    format!(
                                        "{}={{{}}}",
                                        min.1,
                                        a.iter()
                                            .map(|s| format!("{{{},{}}}", s.0, s.1))
                                            .collect::<Vec<String>>()
                                            .join(",")
                                    )
                                }
                            },
                        );
                        self.side_drag = Some((min.0, k));
                        self.name_modified(Some(min.0));
                        #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
                        if self.menu == Menu::Side {
                            self.request_redraw = true;
                        }
                    }
                } else if let Some((i, k)) = self.side_drag {
                    let (mut a, mut b) = self.to_coord(mpos.to_pos());
                    if matches!(self.graph_mode, GraphMode::Polar) {
                        let r = b.hypot(a);
                        let t = b.atan2(a);
                        (a, b) = (t, r);
                    }
                    let s = (a, b);
                    let (c, d) = self.get_name(i).rsplit_once('=').unwrap();
                    self.replace_name(
                        i,
                        if let Some(k) = k {
                            let mut a = d.to_string();
                            a.pop();
                            a.pop();
                            let mut a = a
                                .split("}")
                                .filter_map(|i| {
                                    let mut i = i.to_string();
                                    i.remove(0);
                                    i.remove(0);
                                    i.rsplit_once(",")
                                        .map(|(a, b)| (a.to_string(), b.to_string()))
                                })
                                .collect::<Vec<(String, String)>>();
                            a[k] = (s.0.to_string(), s.1.to_string());
                            format!(
                                "{}={{{}}}",
                                c,
                                a.iter()
                                    .map(|s| format!("{{{},{}}}", s.0, s.1))
                                    .collect::<Vec<String>>()
                                    .join(",")
                            )
                        } else if d.contains('{') {
                            format!("{}={{{},{}}}", c, s.0, s.1)
                        } else if c != "y" {
                            format!("{}={}", c, s.0)
                        } else {
                            format!("{}={}", c, s.1)
                        },
                    );
                    self.name_modified(Some(i));
                } else {
                    self.side_drag = None
                }
            } else {
                self.side_drag = None
            }
        }
        if i.keys_pressed(keybinds.toggle_dark_mode) {
            if self.text_color == Color::splat(0) {
                self.set_dark_mode();
            } else {
                self.set_light_mode();
            }
        }
        if i.keys_pressed(keybinds.only_real) {
            self.only_real = !self.only_real
        }
        if i.keys_pressed(keybinds.side) {
            match self.menu {
                Menu::Side => {
                    self.menu = Menu::Normal;
                    self.text_box = None;
                    self.select = None;
                }
                _ => {
                    self.menu = Menu::Side;
                    self.text_box = Some((0, 0));
                }
            }
        }
        if i.keys_pressed(keybinds.settings) {
            match self.menu {
                Menu::Settings => {
                    self.menu = Menu::Normal;
                    self.text_box = None;
                    self.select = None;
                    self.side_drag = None;
                }
                _ => {
                    self.menu = Menu::Settings;
                    self.side_drag = None;
                }
            }
        }
        #[cfg(feature = "serde")]
        if i.keys_pressed(keybinds.load) {
            match self.menu {
                Menu::Load => {
                    self.menu = Menu::Normal;
                    self.text_box = None;
                    self.select = None;
                    self.side_drag = None;
                }
                _ => {
                    self.save();
                    if !self.file_data.as_ref().unwrap().is_empty() {
                        self.menu = Menu::Load;
                        let n = self.save_num.unwrap_or_default();
                        self.text_box = Some((0, n));
                        self.load(n);
                    }
                    self.side_drag = None;
                }
            }
        }
        #[cfg(feature = "serde")]
        if i.keys_pressed(keybinds.save) {
            let tiny = self.to_tiny();
            let seri = bitcode::serialize(&tiny).unwrap();
            let l = seri.len();
            let comp = zstd::bulk::compress(&seri, 22).unwrap();
            let s = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(&comp);
            let l = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(l.to_string());
            self.clipboard
                .as_mut()
                .unwrap()
                .set_text(&format!("{l}@{s}"));
        }
        #[cfg(feature = "serde")]
        if i.keys_pressed(keybinds.full_save) {
            self.save();
        }
        #[cfg(feature = "serde")]
        if i.keys_pressed(keybinds.paste) {
            let data = &self.clipboard.as_mut().unwrap().get_text();
            if let Ok(tiny) = data.try_into() {
                self.apply_tiny(tiny);
            }
        }
        #[cfg(any(feature = "skia", feature = "tiny-skia"))]
        #[cfg(feature = "arboard")]
        if i.keys_pressed(keybinds.save_png) {
            let (x, y) = (self.screen.x as usize, self.screen.y as usize);
            let mut bytes = vec![0; x * y];
            #[cfg(feature = "skia")]
            {
                let mut surface = get_surface(x as i32, y as i32);
                self.get_img(x as u32, y as u32, &mut bytes, &mut surface);
            }
            #[cfg(feature = "tiny-skia")]
            {
                let canvas = tiny_skia::Pixmap::new(x as u32, y as u32).unwrap();
                self.get_img(x as u32, y as u32, &mut bytes, canvas);
            }
            let mut new = Vec::with_capacity(x * y * 4);
            new.extend(bytes.iter().flat_map(|c| {
                let [_, b1, b2, b3] = c.to_be_bytes();
                [b1, b2, b3, 255]
            }));
            self.clipboard.as_mut().unwrap().set_image(x, y, &new)
        }
        if !self.mouse_held && ret {
            self.keybinds = Some(keybinds);
            return;
        }
        match &i.multi {
            Some(multi) => {
                self.last_multi = true;
                match multi.zoom_delta.total_cmp(&1.0) {
                    std::cmp::Ordering::Greater => {
                        if self.is_3d {
                            self.box_size /= multi.zoom_delta;
                        } else {
                            self.zoom *= multi.zoom_delta;
                            self.offset.x -= if self.mouse_moved && !self.is_3d {
                                self.mouse_position.unwrap().x
                            } else {
                                self.screen_offset.x
                            } / self.zoom.x
                                * (multi.zoom_delta - 1.0);
                            self.offset.y -= if self.mouse_moved && !self.is_3d {
                                self.mouse_position.unwrap().y
                            } else {
                                self.screen_offset.y
                            } / self.zoom.y
                                * (multi.zoom_delta - 1.0);
                            self.recalculate(None);
                        }
                    }
                    std::cmp::Ordering::Less => {
                        if self.is_3d {
                            self.box_size /= multi.zoom_delta;
                        } else {
                            self.offset.x += if self.mouse_moved && !self.is_3d {
                                self.mouse_position.unwrap().x
                            } else {
                                self.screen_offset.x
                            } / self.zoom.x
                                * (multi.zoom_delta.recip() - 1.0);
                            self.offset.y += if self.mouse_moved && !self.is_3d {
                                self.mouse_position.unwrap().y
                            } else {
                                self.screen_offset.y
                            } / self.zoom.y
                                * (multi.zoom_delta.recip() - 1.0);
                            self.zoom *= multi.zoom_delta;
                            self.recalculate(None);
                        }
                    }
                    _ => {}
                }
                if self.is_3d {
                    self.angle.x =
                        (self.angle.x - multi.translation_delta.x / 512.0).rem_euclid(TAU);
                    self.angle.y =
                        (self.angle.y + multi.translation_delta.y / 512.0).rem_euclid(TAU);
                    self.mouse_held = true;
                } else {
                    self.offset.x += multi.translation_delta.x / self.zoom.x;
                    self.offset.y += multi.translation_delta.y / self.zoom.y;
                    self.recalculate(None);
                    self.mouse_held = true;
                }
            }
            _ if i.pointer.is_some() => {
                if !i.pointer.unwrap_or(false)
                    && !self.last_multi
                    && let (Some(interact), Some(last)) = (i.pointer_pos, self.last_interact)
                {
                    let delta = interact - last;
                    if self.is_3d {
                        self.angle.x = (self.angle.x - delta.x / 512.0).rem_euclid(TAU);
                        self.angle.y = (self.angle.y + delta.y / 512.0).rem_euclid(TAU);
                        self.mouse_held = true;
                    } else {
                        self.offset.x += delta.x / self.zoom.x;
                        self.offset.y += delta.y / self.zoom.y;
                        self.recalculate(None);
                        self.mouse_held = true;
                    }
                }
                self.last_multi = false;
            }
            _ if self.mouse_held => {
                self.last_multi = false;
                self.mouse_held = false;
                if !self.is_3d {
                    self.recalculate(None);
                }
            }
            _ => {
                self.last_multi = false;
            }
        }
        self.last_interact = i.pointer_pos;
        if ret {
            self.keybinds = Some(keybinds);
            return;
        }
        let (ax, ay, b, c) = (
            self.delta
                / if self.zoom.x > 1.0 {
                    2.0 * self.zoom.x
                } else {
                    1.0
                },
            self.delta
                / if self.zoom.y > 1.0 {
                    2.0 * self.zoom.y
                } else {
                    1.0
                },
            PI / 64.0,
            1,
        );
        if i.keys_pressed(keybinds.left) {
            if self.is_3d {
                self.angle.x = ((self.angle.x / b - 1.0).round() * b).rem_euclid(TAU);
            } else {
                self.offset.x += ax;
                self.recalculate(None);
            }
        }
        if i.keys_pressed(keybinds.right) {
            if self.is_3d {
                self.angle.x = ((self.angle.x / b + 1.0).round() * b).rem_euclid(TAU);
            } else {
                self.offset.x -= ax;
                self.recalculate(None);
            }
        }
        if i.keys_pressed(keybinds.up) {
            if self.is_3d {
                self.angle.y = ((self.angle.y / b - 1.0).round() * b).rem_euclid(TAU);
            } else {
                if self.graph_mode == GraphMode::DomainColoring {
                    self.recalculate(None);
                }
                self.offset.y += ay;
            }
        }
        if i.keys_pressed(keybinds.down) {
            if self.is_3d {
                self.angle.y = ((self.angle.y / b + 1.0).round() * b).rem_euclid(TAU);
            } else {
                if self.graph_mode == GraphMode::DomainColoring {
                    self.recalculate(None);
                }
                self.offset.y -= ay;
            }
        }
        if i.keys_pressed(keybinds.lines) {
            self.disable_lines = !self.disable_lines;
        }
        if i.keys_pressed(keybinds.axis) {
            self.disable_axis = !self.disable_axis;
        }
        if i.keys_pressed(keybinds.coord) {
            self.disable_coord = !self.disable_coord;
        }
        if i.keys_pressed(keybinds.anti_alias) {
            self.anti_alias = !self.anti_alias;
            self.cache = None;
        }
        if self.is_3d {
            let s = (self.bound.y - self.bound.x) / 4.0;
            if i.keys_pressed(keybinds.left_3d) {
                if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::Polar) {
                    self.recalculate(None);
                }
                self.offset3d.x -= s
            }
            if i.keys_pressed(keybinds.right_3d) {
                if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::Polar) {
                    self.recalculate(None);
                }
                self.offset3d.x += s
            }
            if i.keys_pressed(keybinds.down_3d) {
                if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::Polar) {
                    self.recalculate(None);
                }
                self.offset3d.y += s
            }
            if i.keys_pressed(keybinds.up_3d) {
                if !matches!(self.graph_mode, GraphMode::Depth | GraphMode::Polar) {
                    self.recalculate(None);
                }
                self.offset3d.y -= s
            }
            if i.keys_pressed(keybinds.in_3d) {
                self.offset3d.z += s;
                if matches!(self.graph_mode, GraphMode::Depth | GraphMode::Polar) {
                    self.recalculate(None);
                }
            }
            if i.keys_pressed(keybinds.out_3d) {
                self.offset3d.z -= s;
                if matches!(self.graph_mode, GraphMode::Depth | GraphMode::Polar) {
                    self.recalculate(None);
                }
            }
            if i.keys_pressed(keybinds.ignore_bounds) {
                self.ignore_bounds = !self.ignore_bounds;
            }
            if i.keys_pressed(keybinds.color_depth) {
                self.color_depth = match self.color_depth {
                    DepthColor::None => DepthColor::Vertical,
                    DepthColor::Vertical => DepthColor::Depth,
                    DepthColor::Depth => DepthColor::None,
                };
            }
            let mut changed = false;
            if i.keys_pressed(keybinds.zoom_in_3d) && self.box_size > 0.1 {
                self.box_size -= 0.1;
                changed = true
            }
            if i.keys_pressed(keybinds.zoom_out_3d) {
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
            if i.keys_pressed(keybinds.show_box) {
                self.show_box = !self.show_box
            }
            self.angle.x = (self.angle.x - i.raw_scroll_delta.x / 512.0).rem_euclid(TAU);
            self.angle.y = (self.angle.y + i.raw_scroll_delta.y / 512.0).rem_euclid(TAU);
        } else {
            let rt = (i.raw_scroll_delta.y / 512.0).exp();
            if i.keys_pressed(keybinds.domain_alternate) {
                self.cache = None;
                self.domain_alternate = !self.domain_alternate
            }
            let (x, y) = (i.modifiers.ctrl, i.modifiers.shift);
            let a = !(x ^ y);
            match rt.total_cmp(&1.0) {
                std::cmp::Ordering::Greater => {
                    if a {
                        self.zoom *= rt;
                    } else if x {
                        self.zoom.x *= rt;
                    } else if y {
                        self.zoom.y *= rt;
                    }
                    if a || x {
                        self.offset.x -= if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().x
                        } else {
                            self.screen_offset.x
                        } / self.zoom.x
                            * (rt - 1.0);
                    }
                    if a || y {
                        self.offset.y -= if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().y
                        } else {
                            self.screen_offset.y
                        } / self.zoom.y
                            * (rt - 1.0);
                    }
                    self.recalculate(None);
                }
                std::cmp::Ordering::Less => {
                    if a || x {
                        self.offset.x += if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().x
                        } else {
                            self.screen_offset.x
                        } / self.zoom.x
                            * (rt.recip() - 1.0);
                    }
                    if a || y {
                        self.offset.y += if self.mouse_moved && !self.is_3d {
                            self.mouse_position.unwrap().y
                        } else {
                            self.screen_offset.y
                        } / self.zoom.y
                            * (rt.recip() - 1.0);
                    }
                    if a {
                        self.zoom *= rt;
                    } else if x {
                        self.zoom.x *= rt;
                    } else if y {
                        self.zoom.y *= rt;
                    }
                    self.recalculate(None);
                }
                _ => {}
            }
        }
        let (a, x, y, z) = (
            i.keys_pressed(keybinds.zoom_out),
            i.keys_pressed(keybinds.zoom_out_x),
            i.keys_pressed(keybinds.zoom_out_y),
            i.keys_pressed(keybinds.zoom_out_z),
        );
        if a || x || y || z {
            if self.is_3d {
                if a {
                    self.zoom_3d /= 2.0;
                } else if x {
                    self.zoom_3d.x /= 2.0;
                } else if y {
                    self.zoom_3d.y /= 2.0;
                } else {
                    self.zoom_3d.z /= 2.0;
                }
            } else {
                if a || x {
                    self.offset.x += if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().x
                    } else {
                        self.screen_offset.x
                    } / self.zoom.x;
                }
                if a || y {
                    self.offset.y += if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().y
                    } else {
                        self.screen_offset.y
                    } / self.zoom.y;
                }
                if a {
                    self.zoom /= 2.0;
                } else if x {
                    self.zoom.x /= 2.0;
                } else if y {
                    self.zoom.y /= 2.0;
                }
            }
            self.recalculate(None);
        }
        let (a, x, y, z) = (
            i.keys_pressed(keybinds.zoom_in),
            i.keys_pressed(keybinds.zoom_in_x),
            i.keys_pressed(keybinds.zoom_in_y),
            i.keys_pressed(keybinds.zoom_in_z),
        );
        if a || x || y || z {
            if self.is_3d {
                if a {
                    self.zoom_3d *= 2.0;
                } else if x {
                    self.zoom_3d.x *= 2.0;
                } else if y {
                    self.zoom_3d.y *= 2.0;
                } else {
                    self.zoom_3d.z *= 2.0;
                }
            } else {
                if a {
                    self.zoom *= 2.0;
                } else if x {
                    self.zoom.x *= 2.0;
                } else if y {
                    self.zoom.y *= 2.0;
                }
                if a || x {
                    self.offset.x -= if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().x
                    } else {
                        self.screen_offset.x
                    } / self.zoom.x;
                }
                if a || y {
                    self.offset.y -= if self.mouse_moved && !self.is_3d {
                        self.mouse_position.unwrap().y
                    } else {
                        self.screen_offset.y
                    } / self.zoom.y;
                }
            }
            self.recalculate(None);
        }
        if self.is_3d_data
            && matches!(
                self.graph_mode,
                GraphMode::Slice
                    | GraphMode::Flatten
                    | GraphMode::Depth
                    | GraphMode::Polar
                    | GraphMode::SlicePolar
            )
        {
            if i.keys_pressed(keybinds.slice_up) {
                self.recalculate(None);
                self.slice += c
            }
            if i.keys_pressed(keybinds.slice_down) {
                self.recalculate(None);
                self.slice -= c
            }
            if i.keys_pressed(keybinds.slice_view) {
                self.recalculate(None);
                self.view_x = !self.view_x
            }
        }
        if self.graph_mode == GraphMode::DomainColoring && i.keys_pressed(keybinds.log_scale) {
            self.cache = None;
            self.log_scale = !self.log_scale
        }
        if i.keys_pressed(keybinds.line_style) {
            self.lines = match self.lines {
                Lines::Lines => Lines::Points,
                Lines::Points => Lines::LinesPoints,
                Lines::LinesPoints => Lines::Lines,
            };
        }
        let s = (self.var.y - self.var.x) / 4.0;
        if i.keys_pressed(keybinds.var_down) {
            self.var.x -= s;
            self.var.y -= s;
            self.recalculate(None);
        }
        if i.keys_pressed(keybinds.var_up) {
            self.var.x += s;
            self.var.y += s;
            self.recalculate(None);
        }
        if i.keys_pressed(keybinds.var_in) {
            (self.var.x, self.var.y) = (
                (self.var.x + self.var.y) * 0.5 - (self.var.y - self.var.x) / 4.0,
                (self.var.x + self.var.y) * 0.5 + (self.var.y - self.var.x) / 4.0,
            );
            self.recalculate(None);
        }
        if i.keys_pressed(keybinds.var_out) {
            (self.var.x, self.var.y) = (
                (self.var.x + self.var.y) * 0.5 - (self.var.y - self.var.x),
                (self.var.x + self.var.y) * 0.5 + (self.var.y - self.var.x),
            );
            self.recalculate(None);
        }
        if i.keys_pressed(keybinds.prec_up) {
            self.recalculate(None);
            self.prec *= 0.5;
            self.slice /= 2;
        }
        if i.keys_pressed(keybinds.prec_down) {
            self.recalculate(None);
            self.prec *= 2.0;
            self.slice *= 2;
        }
        if i.keys_pressed(keybinds.ruler) {
            let last = self.ruler_pos;
            self.ruler_pos = self.mouse_position.map(|a| {
                let a = self.to_coord(a.to_pos());
                Vec2::new(a.0, a.1)
            });
            if last == self.ruler_pos {
                self.ruler_pos = None;
            }
        }
        if self.is_complex && i.keys_pressed(keybinds.view) {
            self.show = match self.show {
                Show::Complex => Show::Real,
                Show::Real => Show::Imag,
                Show::Imag => Show::Complex,
                Show::None => Show::None,
            }
        }
        let order = match (self.is_complex, self.is_3d_data) {
            (true, true) => vec![
                GraphMode::Normal,
                GraphMode::Polar,
                GraphMode::Slice,
                GraphMode::SlicePolar,
                GraphMode::Flatten,
                GraphMode::Depth,
                GraphMode::DomainColoring,
            ],
            (true, false) => vec![
                GraphMode::Normal,
                GraphMode::Polar,
                GraphMode::Flatten,
                GraphMode::Depth,
            ],
            (false, true) => vec![
                GraphMode::Normal,
                GraphMode::Polar,
                GraphMode::Slice,
                GraphMode::SlicePolar,
            ],
            (false, false) => vec![GraphMode::Normal, GraphMode::Polar],
        };
        if i.keys_pressed(keybinds.mode_up)
            && let Some(pt) = order.iter().position(|c| *c == self.graph_mode)
        {
            self.set_mode(order[((pt as isize + 1) % order.len() as isize) as usize])
        }
        if i.keys_pressed(keybinds.mode_down)
            && let Some(pt) = order.iter().position(|c| *c == self.graph_mode)
        {
            self.set_mode(order[(pt as isize - 1).rem_euclid(order.len() as isize) as usize])
        }
        if i.keys_pressed(keybinds.fast) {
            self.fast_3d = !self.fast_3d;
            self.reduced_move = !self.reduced_move;
            self.recalculate(None);
        }
        if i.keys_pressed(keybinds.reset) {
            self.offset3d = Vec3::splat(0.0);
            self.offset = Vec2::splat(0.0);
            self.var = self.bound;
            self.zoom = Vec2::splat(1.0);
            self.zoom_3d = Vec3::splat(1.0);
            self.slice = 0;
            self.angle = Vec2::splat(PI / 6.0);
            self.box_size = 3.0f64.sqrt();
            self.prec = 1.0;
            self.mouse_position = None;
            self.mouse_moved = false;
            self.recalculate(None);
        }
        self.keybinds = Some(keybinds)
    }
    #[cfg(feature = "serde")]
    pub(crate) fn save(&mut self) {
        if let Some(fd) = self.file_data_raw.as_mut() {
            let n = self.file_data.as_ref().unwrap();
            update_saves(fd, n);
        }
        let offset = self.to_coord((self.screen / 2.0).to_pos()).into();
        let offset = std::mem::replace(&mut self.offset, offset);
        let seri = bitcode::serialize(&self).unwrap();
        self.offset = offset;
        let l = seri.len();
        let comp = zstd::bulk::compress(&seri, 22).unwrap();
        let s = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(&comp);
        let l = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(l.to_string());
        let n = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(
            self.names
                .iter()
                .filter_map(|n| {
                    if n.name.is_empty() {
                        None
                    } else {
                        Some(n.name.as_str())
                    }
                })
                .next()
                .unwrap_or(""),
        );
        if !std::fs::exists(&self.save_file).unwrap() {
            std::fs::File::create(&self.save_file).unwrap();
        }
        if self.file_data_raw.is_none() {
            let file = std::fs::File::open(&self.save_file).unwrap();
            self.file_data_raw = Some(
                std::io::BufReader::new(file)
                    .lines()
                    .map(Result::unwrap)
                    .collect::<Vec<String>>(),
            );
        }
        let Some(file_data) = self.file_data_raw.as_mut() else {
            unreachable!()
        };
        let do_save = self.names.iter().any(|n| !n.name.is_empty()) && !self.data.is_empty();
        if do_save || self.save_num.is_some() {
            if let Some(i) = self.save_num {
                if do_save {
                    let s = format!("{n}@{l}@{s}");
                    file_data[i] = s
                } else {
                    let Some(fd) = &mut self.file_data else {
                        unreachable!()
                    };
                    fd.remove(i);
                    self.save_num = None;
                    update_saves(file_data, fd);
                }
            } else {
                let s = format!("{n}@{l}@{s}");
                self.save_num = Some(file_data.len());
                file_data.push(s);
            }
            let parent = std::path::Path::new(&self.save_file).parent().unwrap();
            if !std::fs::exists(parent).unwrap() {
                std::fs::create_dir_all(parent).unwrap()
            }
            std::fs::write(&self.save_file, file_data.join("\n")).unwrap();
        }
        self.file_data = Some(
            file_data
                .iter()
                .map(|s| {
                    let r = s.rsplitn(3, '@').collect::<Vec<&str>>();
                    let s = |s: &str| {
                        String::from_utf8(
                            base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(s).unwrap(),
                        )
                        .unwrap()
                    };
                    let a = s(r[2]);
                    let c = s(r[1]).parse::<usize>().unwrap();
                    (a, c, r[0].to_string())
                })
                .collect(),
        );
    }
    #[cfg(feature = "serde")]
    pub(crate) fn load(&mut self, j: usize) {
        if Some(j) == self.save_num {
            return;
        }
        self.save();
        let fd = self.file_data.as_ref().unwrap();
        if fd.is_empty() {
            return;
        }
        let (_, n, s) = &fd[j];
        let s = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(s).unwrap();
        let data = zstd::bulk::decompress(&s, *n).unwrap();
        let mut graph: Graph = bitcode::deserialize(&data).unwrap();
        graph.save_num = Some(j);
        graph.file_data = std::mem::take(&mut self.file_data);
        graph.file_data_raw = std::mem::take(&mut self.file_data_raw);
        graph.clipboard = std::mem::take(&mut self.clipboard);
        graph.menu = self.menu;
        #[cfg(any(feature = "skia", feature = "tiny-skia-text"))]
        {
            graph.font = std::mem::take(&mut self.font);
        }
        #[cfg(feature = "tiny-skia-text")]
        {
            graph.font_cache = build_cache(&self.font, self.text_color);
        }
        graph.recalculate(None);
        graph.name_modified(None);
        graph.text_box = self.text_box;
        graph.screen = self.screen;
        graph.screen_offset = self.screen_offset;
        graph.delta = self.delta;
        graph.offset = graph.get_new_offset(graph.offset);
        graph.side_bar_width = self.side_bar_width;
        graph.keybinds = self.keybinds;
        #[cfg(feature = "skia-vulkan")]
        {
            graph.renderer = std::mem::take(&mut self.renderer);
            graph.render_ctx = std::mem::take(&mut self.render_ctx);
        }
        self.save_num = None;
        *self = graph;
    }
    #[cfg(feature = "egui")]
    fn plot(&mut self, painter: &mut Painter, ui: &egui::Ui) -> Option<Vec<(f32, Draw, Color)>> {
        let anti_alias = self.anti_alias;
        let tex = |cache: &mut Option<Image>, lenx: usize, leny: usize, data: &mut Vec<u8>| {
            *cache = Some(Image(ui.ctx().load_texture(
                "dc",
                egui::ColorImage::from_rgb([lenx, leny], &data[0..lenx * leny * 3]),
                if anti_alias {
                    egui::TextureOptions::LINEAR
                } else {
                    egui::TextureOptions::NEAREST
                },
            )));
        };
        self.plot_inner(painter, tex)
    }
    #[cfg(feature = "skia")]
    fn plot(&mut self, painter: &mut Painter) -> Option<Vec<(f32, Draw, Color)>> {
        let tex = |cache: &mut Option<Image>, lenx: usize, leny: usize, data: &mut Vec<u8>| {
            let info = skia_safe::ImageInfo::new(
                (lenx as i32, leny as i32),
                skia_safe::ColorType::RGB888x,
                skia_safe::AlphaType::Opaque,
                None,
            );
            *cache = skia_safe::images::raster_from_data(
                &info,
                unsafe { skia_safe::Data::new_bytes(&data[0..lenx * leny * 4]) },
                4 * lenx,
            )
            .map(Image);
        };
        self.plot_inner(painter, tex)
    }
    #[cfg(feature = "tiny-skia")]
    fn plot(&mut self, painter: &mut Painter) -> Option<Vec<(f32, Draw, Color)>> {
        let tex = |cache: &mut Option<Image>, lenx: usize, leny: usize, data: &mut Vec<u8>| {
            if let Some(Image(pixmap)) = cache.as_mut() {
                pixmap
                    .pixels_mut()
                    .copy_from_slice(bytemuck::cast_slice(&data[0..lenx * leny * 4]));
            } else {
                *cache = tiny_skia::Pixmap::from_vec(
                    data[0..lenx * leny * 4].to_vec(),
                    tiny_skia::IntSize::from_wh(lenx as u32, leny as u32).unwrap(),
                )
                .map(Image);
            }
        };
        self.plot_inner(painter, tex)
    }
    #[cfg(feature = "wasm-draw")]
    fn plot(&mut self, painter: &mut Painter) -> Option<Vec<(f32, Draw, Color)>> {
        let tex = |cache: &mut Option<Image>, lenx: usize, leny: usize, data: &mut Vec<u8>| {
            let slice = &data[0..lenx * leny * 4];
            *cache = Some(Image(
                unsafe { std::mem::transmute::<&[u8], &[u8]>(slice) },
                lenx,
                leny,
            ))
        };
        self.plot_inner(painter, tex)
    }
    fn plot_inner<G>(&mut self, painter: &mut Painter, tex: G) -> Option<Vec<(f32, Draw, Color)>>
    where
        G: Fn(&mut Option<Image>, usize, usize, &mut Vec<u8>),
    {
        let mut buffer: Option<Vec<(f32, Draw, Color)>> = (!self.fast_3d()).then(|| {
            fn su(a: &GraphType) -> usize {
                match a {
                    GraphType::Coord(_) => 0,
                    GraphType::Coord3D(d) => d.len(),
                    GraphType::Width(_, _, _) => 0,
                    GraphType::Width3D(d, _, _, _, _) => d.len(),
                    GraphType::Constant(_, _) => 0,
                    GraphType::Point(_) => 0,
                    GraphType::List(a) => a.iter().map(su).sum(),
                    GraphType::None => 0,
                }
            }
            let n = self.data.iter().map(su).sum::<usize>()
                * if self.is_complex && matches!(self.show, Show::Complex) && !self.only_real {
                    2
                } else {
                    1
                }
                * match self.lines {
                    Lines::Points => 1,
                    Lines::Lines => 2,
                    Lines::LinesPoints => 3,
                };
            Vec::with_capacity(n + 12)
        });
        let mut cache = std::mem::take(&mut self.cache);
        let mut image_buffer = std::mem::take(&mut self.image_buffer);
        for (k, data) in self.data.iter().enumerate() {
            self.plot_type(
                painter,
                &tex,
                &mut buffer,
                k,
                data,
                &mut cache,
                &mut image_buffer,
            );
        }
        self.cache = cache;
        self.image_buffer = image_buffer;
        buffer
    }
    #[allow(clippy::too_many_arguments)]
    fn plot_type<G>(
        &self,
        painter: &mut Painter,
        tex: &G,
        buffer: &mut Option<Vec<(f32, Draw, Color)>>,
        k: usize,
        data: &GraphType,
        cache: &mut Option<Image>,
        image_buffer: &mut Vec<u8>,
    ) where
        G: Fn(&mut Option<Image>, usize, usize, &mut Vec<u8>),
    {
        let (mut a, mut b, mut c) = (None, None, None);
        match data {
            GraphType::None => {}
            GraphType::List(a) => a.iter().for_each(|data| {
                self.plot_type(painter, tex, buffer, k, data, cache, image_buffer)
            }),
            GraphType::Width(data, start, end) => match self.graph_mode {
                GraphMode::DomainColoring | GraphMode::Slice | GraphMode::SlicePolar => {}
                GraphMode::Normal => {
                    for (i, y) in data.iter().enumerate() {
                        let x = (i as f64 / (data.len() - 1) as f64 - 0.5) * (end - start)
                            + (start + end) * 0.5;
                        let (y, z) = y.to_options();
                        b = if !self.show.imag() {
                            None
                        } else if let Some(z) = z {
                            if self.only_real {
                                if z != 0.0 {
                                    (a, b) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point(
                                    painter,
                                    x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            }
                        } else {
                            None
                        };
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
                    }
                }
                GraphMode::Polar => {
                    for (i, y) in data.iter().enumerate() {
                        let x = (i as f64 / (data.len() - 1) as f64 - 0.5) * (end - start)
                            + (start + end) * 0.5;
                        let (s, c) = x.sin_cos();
                        let (y, z) = y.to_options();
                        b = if !self.show.imag() {
                            None
                        } else if let Some(z) = z {
                            if self.only_real {
                                if z != 0.0 {
                                    (a, b) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point(
                                    painter,
                                    c * z,
                                    s * z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            }
                        } else {
                            None
                        };
                        a = if !self.show.real() {
                            None
                        } else if let Some(y) = y {
                            self.draw_point(
                                painter,
                                c * y,
                                s * y,
                                &self.main_colors[k % self.main_colors.len()],
                                a,
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
                                + (start + end) * 0.5;
                            self.draw_point_3d(
                                x,
                                y,
                                z,
                                &self.main_colors[k % self.main_colors.len()],
                                c,
                                None,
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                    }
                }
            },
            GraphType::Coord(data) => match self.graph_mode {
                GraphMode::DomainColoring | GraphMode::Slice | GraphMode::SlicePolar => {}
                GraphMode::Normal => {
                    for (x, y) in data {
                        let (y, z) = y.to_options();
                        b = if !self.show.imag() {
                            None
                        } else if let Some(z) = z {
                            if self.only_real {
                                if z != 0.0 {
                                    (a, b) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point(
                                    painter,
                                    *x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            }
                        } else {
                            None
                        };
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
                    }
                }
                GraphMode::Polar => {
                    for (x, y) in data {
                        let (s, c) = x.sin_cos();
                        let (y, z) = y.to_options();
                        b = if !self.show.imag() {
                            None
                        } else if let Some(z) = z {
                            if self.only_real {
                                if z != 0.0 {
                                    (a, b) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point(
                                    painter,
                                    c * z,
                                    s * z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            }
                        } else {
                            None
                        };
                        a = if !self.show.real() {
                            None
                        } else if let Some(y) = y {
                            self.draw_point(
                                painter,
                                c * y,
                                s * y,
                                &self.main_colors[k % self.main_colors.len()],
                                a,
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
                            self.draw_point_3d(
                                x,
                                y,
                                *i,
                                &self.main_colors[k % self.main_colors.len()],
                                c,
                                None,
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                    }
                }
            },
            GraphType::Width3D(data, start_x, start_y, end_x, end_y) => match self.graph_mode {
                GraphMode::Normal => {
                    let len = data.len().isqrt();
                    let mut last = Vec::with_capacity(len);
                    let mut cur = Vec::with_capacity(len);
                    let mut lasti = Vec::with_capacity(len);
                    let mut curi = Vec::with_capacity(len);
                    for (i, z) in data.iter().enumerate() {
                        let (i, j) = (i % len, i / len);
                        let x = (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                            + (start_x + end_x) * 0.5;
                        let y = (j as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                            + (start_y + end_y) * 0.5;
                        let (z, w) = z.to_options();
                        let p = if !self.show.imag() {
                            None
                        } else if let Some(w) = w {
                            if self.only_real {
                                if w != 0.0 {
                                    curi.push(None);
                                    cur.push(None);
                                    if i == len - 1 {
                                        lasti =
                                            std::mem::replace(&mut curi, Vec::with_capacity(len));
                                        last = std::mem::replace(&mut cur, Vec::with_capacity(len));
                                    }
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point_3d(
                                    x,
                                    y,
                                    w,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    if i == 0 { None } else { curi[i - 1] },
                                    if j == 0 { None } else { lasti[i] },
                                    buffer,
                                    painter,
                                )
                            }
                        } else {
                            None
                        };
                        curi.push(p);
                        if i == len - 1 {
                            lasti = std::mem::replace(&mut curi, Vec::with_capacity(len));
                        }
                        let p = if !self.show.real() {
                            None
                        } else if let Some(z) = z {
                            self.draw_point_3d(
                                x,
                                y,
                                z,
                                &self.main_colors[k % self.main_colors.len()],
                                if i == 0 { None } else { cur[i - 1] },
                                if j == 0 { None } else { last[i] },
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                        cur.push(p);
                        if i == len - 1 {
                            last = std::mem::replace(&mut cur, Vec::with_capacity(len));
                        }
                    }
                }
                GraphMode::Polar => {
                    let len = data.len().isqrt();
                    let mut last = Vec::with_capacity(len);
                    let mut cur = Vec::with_capacity(len);
                    let mut lasti = Vec::with_capacity(len);
                    let mut curi = Vec::with_capacity(len);
                    for (i, z) in data.iter().enumerate() {
                        let (i, j) = (i % len, i / len);
                        let x = (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                            + (start_x + end_x) * 0.5;
                        let y = (j as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                            + (start_y + end_y) * 0.5;
                        let (ct, st) = x.sin_cos();
                        let (ca, sa) = y.sin_cos();
                        let (z, w) = z.to_options();
                        let p = if !self.show.imag() {
                            None
                        } else if let Some(w) = w {
                            if self.only_real {
                                if w != 0.0 {
                                    curi.push(None);
                                    cur.push(None);
                                    if i == len - 1 {
                                        lasti =
                                            std::mem::replace(&mut curi, Vec::with_capacity(len));
                                        last = std::mem::replace(&mut cur, Vec::with_capacity(len));
                                    }
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point_3d(
                                    w * st * ca,
                                    w * st * sa,
                                    w * ct,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    if i == 0 { None } else { curi[i - 1] },
                                    if j == 0 { None } else { lasti[i] },
                                    buffer,
                                    painter,
                                )
                            }
                        } else {
                            None
                        };
                        curi.push(p);
                        if i == len - 1 {
                            lasti = std::mem::replace(&mut curi, Vec::with_capacity(len));
                        }
                        let p = if !self.show.real() {
                            None
                        } else if let Some(z) = z {
                            self.draw_point_3d(
                                z * st * ca,
                                z * st * sa,
                                z * ct,
                                &self.main_colors[k % self.main_colors.len()],
                                if i == 0 { None } else { cur[i - 1] },
                                if j == 0 { None } else { last[i] },
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                        cur.push(p);
                        if i == len - 1 {
                            last = std::mem::replace(&mut cur, Vec::with_capacity(len));
                        }
                    }
                }
                GraphMode::Slice => {
                    let len = data.len();
                    for (i, y) in data.iter().enumerate() {
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
                            } * 0.5;
                        let (y, z) = y.to_options();
                        b = if !self.show.imag() {
                            None
                        } else if let Some(z) = z {
                            if self.only_real {
                                if z != 0.0 {
                                    (a, b) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point(
                                    painter,
                                    x,
                                    z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            }
                        } else {
                            None
                        };
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
                    }
                }
                GraphMode::SlicePolar => {
                    let len = data.len();
                    for (i, y) in data.iter().enumerate() {
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
                            } * 0.5;
                        let (c, s) = x.sin_cos();
                        let (y, z) = y.to_options();
                        b = if !self.show.imag() {
                            None
                        } else if let Some(z) = z {
                            if self.only_real {
                                if z != 0.0 {
                                    (a, b) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point(
                                    painter,
                                    c * z,
                                    s * z,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    b,
                                )
                            }
                        } else {
                            None
                        };
                        a = if !self.show.real() {
                            None
                        } else if let Some(y) = y {
                            self.draw_point(
                                painter,
                                c * y,
                                s * y,
                                &self.main_colors[k % self.main_colors.len()],
                                a,
                            )
                        } else {
                            None
                        };
                    }
                }
                GraphMode::Flatten => {
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
                GraphMode::Depth => {
                    let len = data.len();
                    let mut body = |i: usize, y: &Complex| {
                        let (y, z) = y.to_options();
                        c = if let (Some(x), Some(y)) = (y, z) {
                            let z = if self.view_x {
                                (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                                    + (start_x + end_x) * 0.5
                            } else {
                                (i as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                                    + (start_y + end_y) * 0.5
                            };
                            self.draw_point_3d(
                                x,
                                y,
                                z,
                                &self.main_colors[k % self.main_colors.len()],
                                c,
                                None,
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                    };
                    for (i, y) in data.iter().enumerate() {
                        body(i, y)
                    }
                }
                GraphMode::DomainColoring => {
                    let lenx = (self.screen.x * self.prec() * self.mult) as usize;
                    let leny = (self.screen.y * self.prec() * self.mult) as usize;
                    if cache.is_none() {
                        #[cfg(feature = "egui")]
                        let m = 3;
                        #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
                        let m = 4;
                        let n = lenx * leny * m;
                        let c = image_buffer.len();
                        if c < n {
                            image_buffer.resize(n, 0);
                        }
                        for (i, z) in data.iter().enumerate() {
                            let [r, g, b] = self.get_color(z);
                            image_buffer[m * i] = r;
                            image_buffer[m * i + 1] = g;
                            image_buffer[m * i + 2] = b;
                            #[cfg(any(
                                feature = "skia",
                                feature = "tiny-skia",
                                feature = "wasm-draw"
                            ))]
                            {
                                image_buffer[m * i + 3] = 255;
                            }
                        }
                        tex(cache, lenx, leny, image_buffer);
                    }
                    if let Some(texture) = cache {
                        painter.image(texture, self.screen);
                    }
                }
            },
            GraphType::Coord3D(data) => match self.graph_mode {
                GraphMode::Slice
                | GraphMode::DomainColoring
                | GraphMode::Flatten
                | GraphMode::Depth
                | GraphMode::SlicePolar => {}
                GraphMode::Normal => {
                    let mut last = None;
                    let mut lasti = None;
                    for (x, y, z) in data {
                        let (z, w) = z.to_options();
                        lasti = if !self.show.imag() {
                            None
                        } else if let Some(w) = w {
                            if self.only_real {
                                if w != 0.0 {
                                    (last, lasti) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point_3d(
                                    *x,
                                    *y,
                                    w,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    lasti,
                                    None,
                                    buffer,
                                    painter,
                                )
                            }
                        } else {
                            None
                        };
                        last = if !self.show.real() {
                            None
                        } else if let Some(z) = z {
                            self.draw_point_3d(
                                *x,
                                *y,
                                z,
                                &self.main_colors[k % self.main_colors.len()],
                                last,
                                None,
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                    }
                }
                GraphMode::Polar => {
                    let mut last = None;
                    let mut lasti = None;
                    for (x, y, z) in data {
                        let (ct, st) = x.sin_cos();
                        let (ca, sa) = y.sin_cos();
                        let (z, w) = z.to_options();
                        lasti = if !self.show.imag() {
                            None
                        } else if let Some(w) = w {
                            if self.only_real {
                                if w != 0.0 {
                                    (last, lasti) = (None, None);
                                    continue;
                                }
                                None
                            } else {
                                self.draw_point_3d(
                                    w * st * ca,
                                    w * st * sa,
                                    w * ct,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    lasti,
                                    None,
                                    buffer,
                                    painter,
                                )
                            }
                        } else {
                            None
                        };
                        last = if !self.show.real() {
                            None
                        } else if let Some(z) = z {
                            self.draw_point_3d(
                                z * st * ca,
                                z * st * sa,
                                z * ct,
                                &self.main_colors[k % self.main_colors.len()],
                                last,
                                None,
                                buffer,
                                painter,
                            )
                        } else {
                            None
                        };
                    }
                }
            },
            GraphType::Constant(c, on_x) => match self.graph_mode {
                GraphMode::Normal | GraphMode::Slice => {
                    let len = 17;
                    if self.is_3d {
                        let mut last = Vec::with_capacity(len);
                        let mut cur = Vec::with_capacity(len);
                        let mut lasti = Vec::with_capacity(len);
                        let mut curi = Vec::with_capacity(len);
                        let start_x = self.bound.x + self.offset3d.x;
                        let start_y = self.bound.x - self.offset3d.y;
                        let end_x = self.bound.y + self.offset3d.x;
                        let end_y = self.bound.y - self.offset3d.y;
                        for i in 0..len * len {
                            let (i, j) = (i % len, i / len);
                            let x = (i as f64 / (len - 1) as f64 - 0.5) * (end_x - start_x)
                                + (start_x + end_x) * 0.5;
                            let y = (j as f64 / (len - 1) as f64 - 0.5) * (end_y - start_y)
                                + (start_y + end_y) * 0.5;
                            let (z, w) = c.to_options();
                            let p = if !self.show.imag() {
                                None
                            } else if let Some(w) = w {
                                if self.only_real {
                                    if w != 0.0 {
                                        curi.push(None);
                                        cur.push(None);
                                        if i == len - 1 {
                                            lasti = std::mem::replace(
                                                &mut curi,
                                                Vec::with_capacity(len),
                                            );
                                            last = std::mem::replace(
                                                &mut cur,
                                                Vec::with_capacity(len),
                                            );
                                        }
                                        continue;
                                    }
                                    None
                                } else {
                                    self.draw_point_3d(
                                        x,
                                        y,
                                        w,
                                        &self.alt_colors[k % self.alt_colors.len()],
                                        if i == 0 { None } else { curi[i - 1] },
                                        if j == 0 { None } else { lasti[i] },
                                        buffer,
                                        painter,
                                    )
                                }
                            } else {
                                None
                            };
                            curi.push(p);
                            if i == len - 1 {
                                lasti = std::mem::replace(&mut curi, Vec::with_capacity(len));
                            }
                            let p = if !self.show.real() {
                                None
                            } else if let Some(z) = z {
                                self.draw_point_3d(
                                    x,
                                    y,
                                    z,
                                    &self.main_colors[k % self.main_colors.len()],
                                    if i == 0 { None } else { cur[i - 1] },
                                    if j == 0 { None } else { last[i] },
                                    buffer,
                                    painter,
                                )
                            } else {
                                None
                            };
                            cur.push(p);
                            if i == len - 1 {
                                last = std::mem::replace(&mut cur, Vec::with_capacity(len));
                            }
                        }
                    } else {
                        let start = self.to_coord(Pos::new(0.0, 0.0));
                        let end = self.to_coord(self.screen.to_pos());
                        if *on_x {
                            for i in 0..len {
                                let x = (i as f64 / (len - 1) as f64 - 0.5) * (end.0 - start.0)
                                    + (start.0 + end.0) * 0.5;
                                let (y, z) = c.to_options();
                                b = if !self.show.imag() {
                                    None
                                } else if let Some(z) = z {
                                    if self.only_real {
                                        if z != 0.0 {
                                            (a, b) = (None, None);
                                            continue;
                                        }
                                        None
                                    } else {
                                        self.draw_point(
                                            painter,
                                            x,
                                            z,
                                            &self.alt_colors[k % self.alt_colors.len()],
                                            b,
                                        )
                                    }
                                } else {
                                    None
                                };
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
                            }
                        } else {
                            for i in 0..len {
                                let x = (i as f64 / (len - 1) as f64 - 0.5) * (end.1 - start.1)
                                    + (start.1 + end.1) * 0.5;
                                let (y, z) = c.to_options();
                                b = if !self.show.imag() {
                                    None
                                } else if let Some(z) = z {
                                    if self.only_real {
                                        if z != 0.0 {
                                            (a, b) = (None, None);
                                            continue;
                                        }
                                        None
                                    } else {
                                        self.draw_point(
                                            painter,
                                            z,
                                            x,
                                            &self.alt_colors[k % self.alt_colors.len()],
                                            b,
                                        )
                                    }
                                } else {
                                    None
                                };
                                a = if !self.show.real() {
                                    None
                                } else if let Some(y) = y {
                                    self.draw_point(
                                        painter,
                                        y,
                                        x,
                                        &self.main_colors[k % self.main_colors.len()],
                                        a,
                                    )
                                } else {
                                    None
                                };
                            }
                        }
                    }
                }
                GraphMode::Polar | GraphMode::SlicePolar => {
                    if !self.is_3d {
                        let (y, z) = c.to_options();
                        let s = self.to_screen(0.0, 0.0);
                        if let Some(r) = z {
                            if self.only_real {
                                if r != 0.0 {
                                    return;
                                }
                            } else if r.is_finite() {
                                painter.circle(
                                    s,
                                    self.to_screen(r.abs(), 0.0).x - s.x,
                                    &self.alt_colors[k % self.alt_colors.len()],
                                    self.line_width,
                                )
                            }
                        }
                        if let Some(r) = y
                            && r.is_finite()
                        {
                            painter.circle(
                                s,
                                self.to_screen(r.abs(), 0.0).x - s.x,
                                &self.main_colors[k % self.main_colors.len()],
                                self.line_width,
                            )
                        }
                    }
                }
                GraphMode::DomainColoring | GraphMode::Depth | GraphMode::Flatten => {}
            },
            GraphType::Point(p) => match self.graph_mode {
                GraphMode::DomainColoring //TODO fix dc
                | GraphMode::Flatten
                | GraphMode::Normal
                | GraphMode::Slice => {
                    if !self.is_3d {
                        painter.rect_filled(
                            self.to_screen(p.x, p.y),
                            &self.main_colors[k % self.main_colors.len()], self.point_size
                        )
                    }
                }
                GraphMode::Polar
                | GraphMode::SlicePolar=>{
                    if !self.is_3d {
                        let (s,c) = p.x.sin_cos();
                        painter.rect_filled(
                            self.to_screen(c*p.y, s*p.y),
                            &self.main_colors[k % self.main_colors.len()],
                         self.point_size)
                    }
                }
                GraphMode::Depth => {}
            },
        }
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
            let sat = (1.0 + if self.log_scale { abs.log10() } else { abs }.fract()) * 0.5;
            let val = (t1 * t2).abs().powf(0.125);
            (sat, val)
        };
        hsv2rgb(hue, sat, val)
    }
    fn shift_hue(&self, diff: Option<f32>, z: f64, color: &Color) -> Color {
        match diff {
            Some(diff) => match self.color_depth {
                DepthColor::Vertical => {
                    shift_hue((z / (2.0 * self.bound.y / self.zoom_3d.z)) as f32, color)
                }
                DepthColor::Depth => shift_hue(diff, color),
                DepthColor::None => *color,
            },
            None => *color,
        }
    }
    fn color_string<'a>(&self, input: &'a str) -> Vec<(Color, &'a str)> {
        if self.bracket_color.is_empty() {
            vec![(self.text_color, input)]
        } else {
            let inputi = input;
            let input = input.char_indices().collect::<Vec<(usize, char)>>();
            let mut vec = Vec::new();
            let mut count: isize = (input
                .iter()
                .filter(|(_, a)| matches!(a, ')' | '}' | ']'))
                .count() as isize
                - input
                    .iter()
                    .filter(|(_, a)| matches!(a, '(' | '{' | '['))
                    .count() as isize)
                .max(0);
            let mut i = 0;
            let mut j = 0;
            let mut color = self.text_color;
            while i < input.len() {
                let (m, c) = input[i];
                match c {
                    '(' | '{' | '[' => {
                        let col = self.bracket_color[count as usize % self.bracket_color.len()];
                        if color != col {
                            if j != m {
                                vec.push((color, &inputi[j..m]));
                            }
                            color = col;
                        }
                        vec.push((col, &inputi[m..m + c.len_utf8()]));
                        j = m + 1;
                        count += 1
                    }
                    ')' | '}' | ']' => {
                        count -= 1;
                        let col = self.bracket_color[count as usize % self.bracket_color.len()];
                        if color != col {
                            if j != m {
                                vec.push((color, &inputi[j..m]));
                            }
                            color = col;
                        }
                        vec.push((col, &inputi[m..m + c.len_utf8()]));
                        j = m + 1;
                    }
                    _ => {
                        if color != self.text_color {
                            if j != m {
                                vec.push((color, &inputi[j..m]));
                                j = m;
                            }
                            color = self.text_color;
                        }
                    }
                }
                i += 1;
            }
            if j != i {
                vec.push((color, &inputi[j..]));
            }
            vec
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
    if cfg!(all(feature = "tiny-skia", not(target_arch = "wasm32"))) {
        [(255.0 * b) as u8, (255.0 * g) as u8, (255.0 * r) as u8]
    } else {
        [(255.0 * r) as u8, (255.0 * g) as u8, (255.0 * b) as u8]
    }
}
fn get_lch(color: [f32; 3]) -> (f32, f32, f32) {
    let c = (color[1].powi(2) + color[2].powi(2)).sqrt();
    let h = color[2].atan2(color[1]);
    (color[0], c, h)
}
#[cfg(feature = "tiny-skia-text")]
fn build_cache(
    font: &Option<bdf2::Font>,
    color: Color,
) -> std::collections::HashMap<char, tiny_skia::Pixmap> {
    if let Some(font) = font {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(color.to_col());
        let mut pm = tiny_skia::Pixmap::new(1, 1).unwrap();
        pm.fill_rect(
            tiny_skia::Rect::from_ltrb(0.0, 0.0, 1.0, 1.0).unwrap(),
            &paint,
            tiny_skia::Transform::default(),
            None,
        );
        let pm = pm.as_ref();
        let paint = tiny_skia::PixmapPaint::default();
        let transform = tiny_skia::Transform::default();
        let mut map = std::collections::HashMap::new();
        for (k, glyph) in font.glyphs() {
            let mut pixmap = tiny_skia::Pixmap::new(glyph.width(), glyph.height()).unwrap();
            for y in 0..glyph.height() {
                for x in 0..glyph.width() {
                    if glyph.get(x, y) {
                        pixmap.draw_pixmap(x as i32, y as i32, pm, &paint, transform, None)
                    }
                }
            }
            map.insert(*k, pixmap);
        }
        map
    } else {
        Default::default()
    }
}
#[allow(clippy::excessive_precision)]
fn rgb_to_oklch(color: &mut [f32; 3]) {
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
fn line(
    buffer: &mut Option<Vec<(f32, Draw, Color)>>,
    painter: Option<&mut Painter>,
    depth: Option<f32>,
    start: Pos,
    end: Pos,
    color: Color,
    line_width: f32,
) {
    if let Some(buffer) = buffer {
        buffer.push((depth.unwrap(), Draw::Line(start, end, line_width), color))
    } else if let Some(painter) = painter {
        painter.line_segment([start, end], line_width, &color)
    }
}
fn point(
    buffer: &mut Option<Vec<(f32, Draw, Color)>>,
    painter: Option<&mut Painter>,
    depth: Option<f32>,
    point: Pos,
    color: Color,
    point_size: f32,
) {
    if let Some(buffer) = buffer {
        buffer.push((depth.unwrap(), Draw::Point(point), color))
    } else if let Some(painter) = painter {
        painter.rect_filled(point, &color, point_size)
    }
}
#[cfg(feature = "serde")]
pub(crate) fn update_saves(fd: &mut Vec<String>, n: &[(String, usize, String)]) {
    *fd = n
        .iter()
        .map(|(a, c, d)| {
            let s = |s: &str| base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(s);
            format!("{}@{}@{}", s(a), s(c.to_string().as_str()), d)
        })
        .collect();
}
#[cfg(feature = "serde")]
impl Drop for Graph {
    fn drop(&mut self) {
        if self.save_num.is_some() {
            self.save()
        }
    }
}
#[cfg(feature = "skia")]
pub fn get_surface(width: i32, height: i32) -> skia_safe::Surface {
    skia_safe::surfaces::raster(
        &skia_safe::ImageInfo::new(
            (width, height),
            skia_safe::ColorType::BGRA8888,
            skia_safe::AlphaType::Opaque,
            None,
        ),
        None,
        None,
    )
    .unwrap()
}
