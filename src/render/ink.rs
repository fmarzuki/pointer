//! Anotasi: coretan pena/stabilo, bentuk (kotak sudut membulat, elips, panah, garis), dan teks,
//! dengan geser dan hapus per objek, undo, dan clear.
//!
//! Coretan yang sudah selesai digambar ke layer `base` (dirender ulang hanya saat undo, clear,
//! atau mulai/selesai geser), sedangkan coretan yang sedang dibuat atau digeser digambar utuh
//! sebagai satu path tiap frame agar stabilo semi-transparan tidak menumpuk di sambungan segmen.

use tiny_skia::{
    LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform,
};

use super::{Bounds, rgba, text};

/// Jarak minimum antar titik (piksel fisik); titik yang lebih rapat diabaikan.
const MIN_STEP: f32 = 1.5;
/// Kotak/elips yang lebih sempit dari ini (lebar atau tinggi) dianggap klik tak sengaja.
const MIN_RECT: f32 = 4.0;
/// Panah/garis yang lebih pendek dari ini dianggap klik tak sengaja.
const MIN_ARROW: f32 = 8.0;
/// Jarak minimum (piksel fisik) dari garis objek agar kursor dianggap "mengenai" objek.
const HIT_TOLERANCE: f32 = 8.0;
/// Warna garis seleksi objek yang disorot/digeser (aksen hijau menu).
const SELECTION: (u8, u8, u8) = (47, 212, 122);
/// Jarak garis seleksi dari tepi objek.
const SELECTION_PAD: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Highlighter,
    /// Kotak sudut membulat dari titik awal drag ke posisi kursor.
    Rect,
    /// Panah dari titik awal drag, kepala panah di posisi kursor.
    Arrow,
    /// Elips di dalam kotak dari titik awal drag ke posisi kursor.
    Ellipse,
    /// Garis lurus dari titik awal drag ke posisi kursor.
    Line,
    /// Teks: klik untuk menaruh, lalu ketik di kotak input.
    Text,
}

impl Tool {
    /// Alat di flyout Shape menu radial, berurutan seperti di flyout.
    pub const SHAPES: [Tool; 5] = [Tool::Rect, Tool::Arrow, Tool::Ellipse, Tool::Line, Tool::Text];

    /// Alat dari flyout Shape (bentuk dan teks), lawan dari gambar bebas.
    pub fn is_shape(self) -> bool {
        Self::SHAPES.contains(&self)
    }

    /// Bentuk yang dibuat dengan drag: hanya titik awal dan titik akhir yang dipakai.
    fn is_drag_shape(self) -> bool {
        matches!(self, Tool::Rect | Tool::Arrow | Tool::Ellipse | Tool::Line)
    }

    /// Pena -> stabilo -> kotak -> panah -> elips -> garis -> pena (klik tengah).
    pub fn next(self) -> Tool {
        match self {
            Tool::Pen => Tool::Highlighter,
            Tool::Highlighter => Tool::Rect,
            Tool::Rect => Tool::Arrow,
            Tool::Arrow => Tool::Ellipse,
            Tool::Ellipse => Tool::Line,
            Tool::Line => Tool::Text,
            Tool::Text => Tool::Pen,
        }
    }
}

/// Alat dan warna aktif, disimpan di tingkat aplikasi agar sama di semua monitor dan
/// bertahan saat overlay dibuat ulang. `last_*` dipakai menu radial (sektor Gambar/Bentuk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InkSettings {
    pub tool: Tool,
    pub color: usize,
    pub last_freehand: Tool,
    pub last_shape: Tool,
}

impl Default for InkSettings {
    fn default() -> Self {
        Self {
            tool: Tool::Pen,
            color: 0,
            last_freehand: Tool::Pen,
            last_shape: Tool::Rect,
        }
    }
}

impl InkSettings {
    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        if tool.is_shape() {
            self.last_shape = tool;
        } else {
            self.last_freehand = tool;
        }
    }

    pub fn next_tool(&mut self) {
        self.set_tool(self.tool.next());
    }

    pub fn next_color(&mut self, palette_len: usize) {
        self.color = (self.color + 1) % palette_len.max(1);
    }

    /// Sektor Gambar: bila sudah menggambar bebas, tukar pena <-> stabilo; selain itu
    /// kembali ke alat gambar bebas terakhir.
    pub fn select_freehand(&mut self, drawing: bool) {
        let tool = match self.tool {
            Tool::Pen if drawing => Tool::Highlighter,
            Tool::Highlighter if drawing => Tool::Pen,
            _ => self.last_freehand,
        };
        self.set_tool(tool);
    }

    /// Sektor Bentuk: bila sudah memakai bentuk, ganti ke bentuk berikutnya; selain itu
    /// kembali ke bentuk terakhir.
    pub fn select_shape(&mut self, drawing: bool) {
        let tool = match self.tool {
            Tool::Rect if drawing => Tool::Arrow,
            Tool::Arrow if drawing => Tool::Ellipse,
            Tool::Ellipse if drawing => Tool::Line,
            Tool::Line if drawing => Tool::Text,
            Tool::Text if drawing => Tool::Rect,
            _ => self.last_shape,
        };
        self.set_tool(tool);
    }
}

#[derive(Debug, Clone)]
pub struct InkStyle {
    pub palette: Vec<(u8, u8, u8)>,
    /// Lebar dalam piksel fisik.
    pub pen_width: f32,
    pub highlighter_width: f32,
    pub highlighter_alpha: f32,
    /// Jari-jari sudut kotak dalam piksel fisik.
    pub rect_radius: f32,
    /// Ukuran huruf objek teks dalam piksel fisik.
    pub text_size: f32,
}

impl InkStyle {
    /// Panjang sisi kepala panah.
    fn arrow_head(&self) -> f32 {
        (self.pen_width * 4.0).max(14.0)
    }
}

impl InkStyle {
    fn width(&self, tool: Tool) -> f32 {
        match tool {
            Tool::Highlighter => self.highlighter_width,
            _ => self.pen_width,
        }
    }

    fn alpha(&self, tool: Tool) -> f32 {
        match tool {
            Tool::Highlighter => self.highlighter_alpha,
            _ => 1.0,
        }
    }
}

#[derive(Debug, Clone)]
struct InkStroke {
    tool: Tool,
    color: (u8, u8, u8),
    points: Vec<(f32, f32)>,
    /// Isi objek teks (alat `Text`); titik pertama = sudut kiri atas.
    text: Option<String>,
    /// Identitas objek (nomor urut saat dibuat), dipakai riwayat undo untuk geser.
    id: u64,
}

impl InkStroke {
    fn translate(&mut self, dx: f32, dy: f32) {
        for p in &mut self.points {
            p.0 += dx;
            p.1 += dy;
        }
    }
}

/// Operasi yang bisa dibatalkan.
#[derive(Debug, Clone)]
enum Op {
    Add { id: u64 },
    Move { id: u64, dx: f32, dy: f32 },
    /// Objek dihapus dari posisi tumpukan `index`; disimpan utuh agar bisa dikembalikan.
    Delete { stroke: InkStroke, index: usize },
}

/// Satu langkah riwayat. `seq` adalah urutan global lintas monitor, agar undo
/// membatalkan langkah terbaru di monitor mana pun.
#[derive(Debug, Clone)]
struct Step {
    seq: u64,
    op: Op,
}

/// Objek yang sedang digeser: dikeluarkan dari daftar `strokes` (dan dari layer base)
/// selama drag, lalu dikembalikan ke posisi tumpukan semula.
#[derive(Debug)]
struct Drag {
    stroke: InkStroke,
    index: usize,
    last: (f32, f32),
    total: (f32, f32),
}

#[derive(Debug)]
pub struct Ink {
    strokes: Vec<InkStroke>,
    current: Option<InkStroke>,
    history: Vec<Step>,
    drag: Option<Drag>,
    /// Objek di bawah kursor (disorot dengan garis seleksi, bisa digeser).
    hover: Option<u64>,
    /// Teks yang sedang diketik di kotak input: (sudut kiri atas, isi), tampil sebagai pratinjau.
    pending_text: Option<((f32, f32), String)>,
    pub tool: Tool,
    pub color: usize,
}

impl Default for Ink {
    fn default() -> Self {
        Self {
            strokes: Vec::new(),
            current: None,
            history: Vec::new(),
            drag: None,
            hover: None,
            pending_text: None,
            tool: Tool::Pen,
            color: 0,
        }
    }
}

impl Ink {
    pub fn begin(&mut self, x: f32, y: f32, seq: u64, style: &InkStyle) {
        let color = style.palette[self.color % style.palette.len()];
        self.hover = None;
        self.current = Some(InkStroke {
            tool: self.tool,
            color,
            points: vec![(x, y)],
            text: None,
            id: seq,
        });
    }

    // ---- Teks ----

    /// Tampilkan pratinjau teks yang sedang diketik (`None` = hapus pratinjau).
    pub fn set_pending_text(&mut self, pending: Option<((f32, f32), String)>) {
        self.hover = None;
        self.pending_text = pending;
    }

    /// Tambahkan objek teks dengan warna aktif. Teks kosong/hanya spasi diabaikan.
    pub fn add_text(&mut self, at: (f32, f32), content: &str, seq: u64, base: Option<&mut Pixmap>, style: &InkStyle) -> bool {
        let content = content.trim_end();
        if content.trim().is_empty() {
            return false;
        }
        let s = InkStroke {
            tool: Tool::Text,
            color: style.palette[self.color % style.palette.len()],
            points: vec![at],
            text: Some(content.to_string()),
            id: seq,
        };
        if let Some(base) = base {
            draw_stroke(base, &s, style);
        }
        self.history.push(Step { seq, op: Op::Add { id: seq } });
        self.strokes.push(s);
        true
    }

    /// Tambah titik ke coretan aktif. `true` bila titik dipakai.
    pub fn extend(&mut self, x: f32, y: f32) -> bool {
        let Some(s) = &mut self.current else { return false };
        let &(lx, ly) = s.points.last().expect("coretan selalu punya titik awal");
        if (x - lx).hypot(y - ly) < MIN_STEP {
            return false;
        }
        if s.tool.is_drag_shape() {
            // Bentuk hanya butuh titik awal + titik akhir yang mengikuti kursor.
            s.points.truncate(1);
        }
        s.points.push((x, y));
        true
    }

    /// Selesaikan coretan aktif: gambar ke layer `base` (bila ada) lalu simpan.
    /// `true` bila coretan disimpan; bentuk yang terlalu kecil dibuang.
    pub fn end(&mut self, base: Option<&mut Pixmap>, style: &InkStyle) -> bool {
        let Some(s) = self.current.take() else { return false };
        let (w, h) = rect_size(&s.points);
        let too_small = match s.tool {
            Tool::Rect | Tool::Ellipse => w < MIN_RECT || h < MIN_RECT,
            Tool::Arrow | Tool::Line => w.hypot(h) < MIN_ARROW,
            Tool::Pen | Tool::Highlighter | Tool::Text => false,
        };
        if too_small {
            return false;
        }
        if let Some(base) = base {
            draw_stroke(base, &s, style);
        }
        self.history.push(Step { seq: s.id, op: Op::Add { id: s.id } });
        self.strokes.push(s);
        true
    }

    pub fn is_drawing(&self) -> bool {
        self.current.is_some()
    }

    pub fn last_seq(&self) -> Option<u64> {
        self.history.last().map(|s| s.seq)
    }

    /// Batalkan langkah terakhir (hapus objek baru atau kembalikan geseran).
    /// `true` bila ada yang berubah (layer base perlu dirender ulang).
    pub fn undo(&mut self) -> bool {
        let Some(step) = self.history.pop() else { return false };
        match step.op {
            Op::Add { id } => self.strokes.retain(|s| s.id != id),
            Op::Move { id, dx, dy } => {
                if let Some(s) = self.strokes.iter_mut().find(|s| s.id == id) {
                    s.translate(-dx, -dy);
                }
            }
            Op::Delete { stroke, index } => {
                let index = index.min(self.strokes.len());
                self.strokes.insert(index, stroke);
            }
        }
        self.hover = None;
        true
    }

    pub fn clear(&mut self) -> bool {
        let had = !self.strokes.is_empty() || self.current.is_some() || self.drag.is_some();
        self.strokes.clear();
        self.history.clear();
        self.current = None;
        self.drag = None;
        self.hover = None;
        had
    }

    // ---- Seleksi dan geser objek ----

    /// Objek teratas yang garisnya berada dekat titik `p`.
    pub fn hit(&self, p: (f32, f32), style: &InkStyle) -> Option<u64> {
        self.strokes
            .iter()
            .rev()
            .find(|s| distance_to_stroke(s, p, style) <= (style.width(s.tool) / 2.0 + 4.0).max(HIT_TOLERANCE))
            .map(|s| s.id)
    }

    /// Hapus satu objek (dicatat di riwayat dengan urutan `seq` agar bisa di-undo).
    /// `true` bila objek ditemukan; pemanggil harus merender ulang layer base.
    pub fn delete(&mut self, id: u64, seq: u64) -> bool {
        let Some(index) = self.strokes.iter().position(|s| s.id == id) else { return false };
        let stroke = self.strokes.remove(index);
        self.history.push(Step { seq, op: Op::Delete { stroke, index } });
        if self.hover == Some(id) {
            self.hover = None;
        }
        true
    }

    /// Perbarui objek yang disorot kursor. `true` bila berubah (perlu gambar ulang).
    pub fn set_hover(&mut self, id: Option<u64>) -> bool {
        let changed = self.hover != id;
        self.hover = id;
        changed
    }

    pub fn hover(&self) -> Option<u64> {
        self.hover
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Mulai geser objek `id` dari titik `at`. Pemanggil harus merender ulang layer base
    /// (objek dikeluarkan dari base selama digeser).
    pub fn start_drag(&mut self, id: u64, at: (f32, f32)) -> bool {
        let Some(index) = self.strokes.iter().position(|s| s.id == id) else { return false };
        let stroke = self.strokes.remove(index);
        self.drag = Some(Drag { stroke, index, last: at, total: (0.0, 0.0) });
        self.hover = Some(id);
        true
    }

    pub fn drag_to(&mut self, p: (f32, f32)) {
        if let Some(d) = &mut self.drag {
            let (dx, dy) = (p.0 - d.last.0, p.1 - d.last.1);
            d.stroke.translate(dx, dy);
            d.last = p;
            d.total.0 += dx;
            d.total.1 += dy;
        }
    }

    /// Selesaikan geser: kembalikan objek ke posisi tumpukan semula dan catat di riwayat
    /// (dengan urutan `seq`). Pemanggil harus merender ulang layer base.
    pub fn end_drag(&mut self, seq: u64) -> bool {
        let Some(d) = self.drag.take() else { return false };
        let id = d.stroke.id;
        let index = d.index.min(self.strokes.len());
        self.strokes.insert(index, d.stroke);
        let (dx, dy) = d.total;
        if dx.hypot(dy) >= 0.5 {
            self.history.push(Step { seq, op: Op::Move { id, dx, dy } });
        }
        true
    }

    /// Kotak pembatas semua yang berubah tiap frame: coretan aktif, objek yang digeser,
    /// dan garis seleksi objek yang disorot.
    pub fn active_bounds(&self, style: &InkStyle) -> Option<Bounds> {
        let current = self.current.as_ref().map(|s| stroke_bounds(s, style));
        let selected = self.selected().map(|s| pad_bounds(stroke_bounds(s, style), SELECTION_PAD + 2.0));
        let pending = self.pending_text.as_ref().map(|(at, t)| pad_bounds(text_box(*at, t, style), SELECTION_PAD + 4.0));
        super::union_bounds(super::union_bounds(current, selected), pending)
    }

    /// Objek yang diberi garis seleksi: yang sedang digeser, atau yang disorot kursor.
    fn selected(&self) -> Option<&InkStroke> {
        match &self.drag {
            Some(d) => Some(&d.stroke),
            None => self.hover.and_then(|id| self.strokes.iter().find(|s| s.id == id)),
        }
    }

    /// Gambar semua coretan yang sudah selesai (ke layer base yang sudah dibersihkan).
    pub fn draw_committed(&self, pm: &mut Pixmap, style: &InkStyle) {
        for s in &self.strokes {
            draw_stroke(pm, s, style);
        }
    }

    /// Gambar coretan aktif, objek yang sedang digeser, dan garis seleksi.
    pub fn draw_current(&self, pm: &mut Pixmap, style: &InkStyle) {
        if let Some(d) = &self.drag {
            draw_stroke(pm, &d.stroke, style);
        }
        if let Some(s) = &self.current {
            draw_stroke(pm, s, style);
        }
        if let Some(((x, y), t)) = &self.pending_text {
            let color = style.palette[self.color % style.palette.len()];
            text::draw_block(pm, t, *x, *y, style.text_size, rgba(color, 1.0));
            // Caret di akhir baris terakhir dan kotak putus-putus penanda posisi.
            let last = t.rsplit('\n').next().unwrap_or("");
            let lines = t.split('\n').count() as f32;
            let line_h = style.text_size * 1.25;
            let cx = x + text::measure(last, style.text_size) + 2.0;
            let cy = y + (lines - 1.0) * line_h;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy + line_h * 0.12);
            pb.line_to(cx, cy + line_h * 0.88);
            if let Some(p) = pb.finish() {
                let mut paint = Paint { anti_alias: true, ..Paint::default() };
                paint.set_color(rgba(color, 1.0));
                pm.stroke_path(&p, &paint, &Stroke { width: 2.0, ..Stroke::default() }, Transform::identity(), None);
            }
            dashed_box(pm, pad_bounds(text_box((*x, *y), t, style), SELECTION_PAD - 2.0));
        }
        if let Some(s) = self.selected() {
            dashed_box(pm, pad_bounds(stroke_bounds(s, style), SELECTION_PAD - 2.0));
        }
    }

    /// Pratinjau alat di posisi kursor: lingkaran seukuran kuas (pena/stabilo) atau
    /// kotak kecil bersudut bulat (alat kotak), dengan warna aktif.
    pub fn draw_preview(&self, pm: &mut Pixmap, (x, y): (f32, f32), style: &InkStyle) {
        let color = style.palette[self.color % style.palette.len()];
        let r = preview_radius(self.tool, style);
        let path = match self.tool {
            Tool::Rect => rounded_rect_path((x - r, y - r), (x + r, y + r), r * 0.4),
            Tool::Arrow => arrow_path((x - r, y + r), (x + r, y - r), r),
            Tool::Ellipse => oval_path((x - r, y - r * 0.7), (x + r, y + r * 0.7)),
            Tool::Line => line_path((x - r, y + r), (x + r, y - r)),
            Tool::Text => {
                text::draw_centered(pm, "T", x + 10.0, y + 10.0, 16.0, rgba(color, 1.0));
                None
            }
            Tool::Pen | Tool::Highlighter => PathBuilder::from_circle(x, y, r),
        };
        if let Some(path) = path {
            let mut paint = Paint {
                anti_alias: true,
                ..Paint::default()
            };
            paint.set_color(rgba(color, style.alpha(self.tool).max(0.5)));
            let stroke = Stroke {
                width: 2.0,
                ..Stroke::default()
            };
            pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    pub fn preview_bounds(&self, (x, y): (f32, f32), style: &InkStyle) -> Bounds {
        let r = preview_radius(self.tool, style) + 3.0;
        (x - r, y - r, x + r, y + r)
    }
}

fn preview_radius(tool: Tool, style: &InkStyle) -> f32 {
    if tool == Tool::Text {
        // Huruf "T" digambar di kanan bawah kursor.
        20.0
    } else if tool.is_shape() {
        8.0
    } else {
        (style.width(tool) / 2.0).max(3.0)
    }
}

/// Garis seleksi putus-putus hijau.
fn dashed_box(pm: &mut Pixmap, (x0, y0, x1, y1): Bounds) {
    if let Some(r) = Rect::from_ltrb(x0, y0, x1, y1) {
        let path = PathBuilder::from_rect(r);
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        paint.set_color(rgba(SELECTION, 0.9));
        let stroke = Stroke {
            width: 1.5,
            dash: StrokeDash::new(vec![6.0, 4.0], 0.0),
            ..Stroke::default()
        };
        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

/// Kotak teks (tanpa padding) dengan sudut kiri atas `at`; minimal selebar caret.
fn text_box(at: (f32, f32), t: &str, style: &InkStyle) -> Bounds {
    let (w, h) = text::block_size(t, style.text_size);
    (at.0, at.1, at.0 + w.max(4.0) + 4.0, at.1 + h)
}

fn pad_bounds((x0, y0, x1, y1): Bounds, pad: f32) -> Bounds {
    (x0 - pad, y0 - pad, x1 + pad, y1 + pad)
}

/// Kotak pembatas satu objek, termasuk tebal garis (dan kepala panah).
fn stroke_bounds(s: &InkStroke, style: &InkStyle) -> Bounds {
    if let (Tool::Text, Some(t)) = (s.tool, &s.text) {
        return pad_bounds(text_box(s.points[0], t, style), 2.0);
    }
    let mut pad = style.width(s.tool) / 2.0 + 2.0;
    if s.tool == Tool::Arrow {
        pad += style.arrow_head();
    }
    let (mut x0, mut y0) = s.points[0];
    let (mut x1, mut y1) = (x0, y0);
    for &(x, y) in &s.points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (x0 - pad, y0 - pad, x1 + pad, y1 + pad)
}

/// Jarak titik `p` ke segmen `a`-`b`.
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len2 = vx * vx + vy * vy;
    let t = if len2 <= f32::EPSILON {
        0.0
    } else {
        (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len2).clamp(0.0, 1.0)
    };
    (p.0 - (a.0 + t * vx)).hypot(p.1 - (a.1 + t * vy))
}

/// Jarak titik ke garis objek (bukan ke bagian dalamnya), agar bisa menggambar di dalam
/// kotak/elips tanpa tak sengaja menggeser bentuknya.
fn distance_to_stroke(s: &InkStroke, p: (f32, f32), style: &InkStyle) -> f32 {
    let (a, b) = (s.points[0], *s.points.last().expect("objek selalu punya titik"));
    match s.tool {
        Tool::Pen | Tool::Highlighter => {
            if s.points.len() == 1 {
                return (p.0 - a.0).hypot(p.1 - a.1);
            }
            s.points
                .windows(2)
                .map(|w| segment_distance(p, w[0], w[1]))
                .fold(f32::INFINITY, f32::min)
        }
        Tool::Line => segment_distance(p, a, b),
        Tool::Text => {
            // Seluruh kotak teks bisa dipegang (0 di dalam kotak).
            let (x0, y0, x1, y1) = text_box(a, s.text.as_deref().unwrap_or(""), style);
            let dx = (x0 - p.0).max(p.0 - x1).max(0.0);
            let dy = (y0 - p.1).max(p.1 - y1).max(0.0);
            dx.hypot(dy)
        }
        Tool::Arrow => {
            // Batang + kedua sayap kepala panah (geometri sama dengan `arrow_path`).
            let mut d = segment_distance(p, a, b);
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            if len > f32::EPSILON {
                let h = style.arrow_head().min(len / 2.0);
                let (bx, by) = (-(b.0 - a.0) / len, -(b.1 - a.1) / len);
                let (sn, cs) = 30f32.to_radians().sin_cos();
                for side in [1.0f32, -1.0] {
                    let w = (b.0 + h * (bx * cs - side * by * sn), b.1 + h * (side * bx * sn + by * cs));
                    d = d.min(segment_distance(p, b, w));
                }
            }
            d
        }
        Tool::Rect => {
            let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
            let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
            let inside = (x0..=x1).contains(&p.0) && (y0..=y1).contains(&p.1);
            if inside {
                (p.0 - x0).min(x1 - p.0).min(p.1 - y0).min(y1 - p.1)
            } else {
                let dx = (x0 - p.0).max(p.0 - x1).max(0.0);
                let dy = (y0 - p.1).max(p.1 - y1).max(0.0);
                dx.hypot(dy)
            }
        }
        Tool::Ellipse => {
            let (cx, cy) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let (rx, ry) = (((b.0 - a.0) / 2.0).abs().max(1.0), ((b.1 - a.1) / 2.0).abs().max(1.0));
            // Pendekatan: selisih "jari-jari ternormalisasi" dikali sumbu terpendek.
            let k = ((p.0 - cx) / rx).hypot((p.1 - cy) / ry);
            (k - 1.0).abs() * rx.min(ry)
        }
    }
}

fn rect_size(pts: &[(f32, f32)]) -> (f32, f32) {
    match (pts.first(), pts.last()) {
        (Some(a), Some(b)) => ((b.0 - a.0).abs(), (b.1 - a.1).abs()),
        _ => (0.0, 0.0),
    }
}

fn draw_stroke(pm: &mut Pixmap, s: &InkStroke, style: &InkStyle) {
    if let (Tool::Text, Some(t)) = (s.tool, &s.text) {
        let (x, y) = s.points[0];
        text::draw_block(pm, t, x, y, style.text_size, rgba(s.color, 1.0));
        return;
    }
    let ends = s.points.first().copied().zip(s.points.last().copied());
    let path = match (s.tool, ends) {
        (Tool::Pen | Tool::Highlighter, _) => smooth_path(&s.points),
        (_, None) => None,
        (Tool::Rect, Some((a, b))) => rounded_rect_path(a, b, style.rect_radius),
        (Tool::Arrow, Some((a, b))) => arrow_path(a, b, style.arrow_head()),
        (Tool::Ellipse, Some((a, b))) => oval_path(a, b),
        (Tool::Line, Some((a, b))) => line_path(a, b),
        (Tool::Text, _) => None,
    };
    let Some(path) = path else { return };
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    paint.set_color(rgba(s.color, style.alpha(s.tool)));
    let stroke = Stroke {
        width: style.width(s.tool),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

/// Kotak dari dua sudut berlawanan (urutan bebas) dengan sudut membulat. Jari-jari
/// dibatasi setengah sisi terpendek agar sudut tidak saling bertabrakan.
pub(super) fn rounded_rect_path(a: (f32, f32), b: (f32, f32), radius: f32) -> Option<Path> {
    // Konstanta Bézier kubik untuk seperempat lingkaran.
    const K: f32 = 0.552_284_8;
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let r = radius.max(0.0).min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    let c = r * (1.0 - K);
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.cubic_to(x1 - c, y0, x1, y0 + c, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - c, x1 - c, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + c, y1, x0, y1 - c, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.cubic_to(x0, y0 + c, x0 + c, y0, x0 + r, y0);
    pb.close();
    pb.finish()
}

/// Garis dari `from` ke `to` dengan kepala panah (dua sayap `head` piksel, 30°) di `to`.
fn arrow_path(from: (f32, f32), to: (f32, f32), head: f32) -> Option<Path> {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let len = dx.hypot(dy);
    if len < f32::EPSILON {
        return None;
    }
    // Kepala panah tidak lebih panjang dari setengah batang.
    let head = head.min(len / 2.0);
    let (ux, uy) = (dx / len, dy / len);
    let (s, c) = 30f32.to_radians().sin_cos();
    let wing = |side: f32| {
        // Putar vektor balik (-u) sebesar ±30°.
        let (bx, by) = (-ux, -uy);
        (
            to.0 + head * (bx * c - side * by * s),
            to.1 + head * (side * bx * s + by * c),
        )
    };
    let (l, r) = (wing(1.0), wing(-1.0));
    let mut pb = PathBuilder::new();
    pb.move_to(from.0, from.1);
    pb.line_to(to.0, to.1);
    pb.move_to(l.0, l.1);
    pb.line_to(to.0, to.1);
    pb.line_to(r.0, r.1);
    pb.finish()
}

fn line_path(a: (f32, f32), b: (f32, f32)) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(a.0, a.1);
    pb.line_to(b.0, b.1);
    pb.finish()
}

/// Elips di dalam kotak dari dua sudut berlawanan (urutan bebas).
fn oval_path(a: (f32, f32), b: (f32, f32)) -> Option<Path> {
    let rect = Rect::from_ltrb(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))?;
    PathBuilder::from_oval(rect)
}

/// Haluskan titik mouse: kurva kuadratik melalui titik tengah antar sampel, dengan titik
/// sampel sebagai kontrol. Satu titik menjadi segmen sangat pendek agar tetap tampil
/// sebagai titik bulat.
fn smooth_path(pts: &[(f32, f32)]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let (x0, y0) = *pts.first()?;
    pb.move_to(x0, y0);
    match pts.len() {
        1 => pb.line_to(x0 + 0.01, y0),
        2 => pb.line_to(pts[1].0, pts[1].1),
        _ => {
            for w in pts.windows(2).skip(1) {
                let (cx, cy) = w[0];
                let (nx, ny) = w[1];
                pb.quad_to(cx, cy, (cx + nx) / 2.0, (cy + ny) / 2.0);
            }
            let &(lx, ly) = pts.last()?;
            pb.line_to(lx, ly);
        }
    }
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> InkStyle {
        InkStyle {
            palette: vec![(255, 0, 0), (0, 0, 255)],
            pen_width: 4.0,
            highlighter_width: 16.0,
            highlighter_alpha: 0.35,
            rect_radius: 6.0,
            text_size: 20.0,
        }
    }

    #[test]
    fn stroke_lifecycle_and_undo() {
        let st = style();
        let mut ink = Ink::default();
        ink.begin(0.0, 0.0, 1, &st);
        assert!(!ink.extend(0.5, 0.5), "titik terlalu rapat diabaikan");
        assert!(ink.extend(10.0, 0.0));
        assert!(ink.is_drawing());
        assert!(ink.end(None, &st));
        assert!(!ink.end(None, &st), "tidak ada coretan aktif");
        assert_eq!(ink.last_seq(), Some(1));
        assert!(ink.undo());
        assert!(!ink.undo());
        assert_eq!(ink.last_seq(), None);
    }

    #[test]
    fn clear_reports_whether_anything_changed() {
        let st = style();
        let mut ink = Ink::default();
        assert!(!ink.clear());
        ink.begin(1.0, 1.0, 1, &st);
        assert!(ink.clear());
        assert!(!ink.is_drawing());
    }

    #[test]
    fn color_and_tool_cycle() {
        let mut s = InkSettings::default();
        s.next_color(2);
        assert_eq!(s.color, 1);
        s.next_color(2);
        assert_eq!(s.color, 0);
        let order = [Tool::Highlighter, Tool::Rect, Tool::Arrow, Tool::Ellipse, Tool::Line, Tool::Text, Tool::Pen];
        for expected in order {
            s.next_tool();
            assert_eq!(s.tool, expected);
        }
        assert_eq!((s.last_freehand, s.last_shape), (Tool::Pen, Tool::Text));
    }

    #[test]
    fn draws_pen_and_translucent_highlighter() {
        let st = style();
        let mut ink = Ink::default();
        ink.begin(5.0, 10.0, 1, &st);
        ink.extend(35.0, 10.0);
        ink.end(None, &st);
        ink.tool = Tool::Highlighter;
        ink.begin(5.0, 30.0, 2, &st);
        ink.extend(35.0, 30.0);
        ink.end(None, &st);
        let mut pm = Pixmap::new(40, 40).unwrap();
        ink.draw_committed(&mut pm, &st);
        assert_eq!(pm.pixel(20, 10).unwrap().alpha(), 255, "pena opak");
        let hl = pm.pixel(20, 30).unwrap().alpha();
        assert!((80..=100).contains(&hl), "stabilo semi-transparan: {hl}");
        assert_eq!(pm.pixel(20, 20).unwrap().alpha(), 0);
    }

    #[test]
    fn active_bounds_cover_stroke_width() {
        let st = style();
        let mut ink = Ink::default();
        assert_eq!(ink.active_bounds(&st), None);
        ink.begin(10.0, 10.0, 1, &st);
        ink.extend(30.0, 20.0);
        assert_eq!(ink.active_bounds(&st), Some((6.0, 6.0, 34.0, 24.0)));
    }

    #[test]
    fn rect_follows_cursor_and_draws_rounded_outline() {
        let st = style();
        let mut ink = Ink { tool: Tool::Rect, ..Ink::default() };
        ink.begin(30.0, 30.0, 1, &st);
        ink.extend(20.0, 20.0);
        ink.extend(5.0, 5.0);
        assert_eq!(ink.current.as_ref().unwrap().points, [(30.0, 30.0), (5.0, 5.0)]);
        let mut base = Pixmap::new(40, 40).unwrap();
        assert!(ink.end(Some(&mut base), &st));
        // Tepi kotak tergambar, bagian dalam kosong (outline).
        assert_eq!(base.pixel(17, 5).unwrap().alpha(), 255, "sisi atas");
        assert_eq!(base.pixel(17, 17).unwrap().alpha(), 0, "bagian dalam");
        // Sudut membulat: titik sudut tajam tidak terisi penuh.
        assert!(base.pixel(4, 4).unwrap().alpha() < 255, "sudut membulat");
    }

    #[test]
    fn tiny_rect_is_discarded() {
        let st = style();
        let mut ink = Ink { tool: Tool::Rect, ..Ink::default() };
        ink.begin(10.0, 10.0, 1, &st);
        ink.extend(12.0, 30.0);
        let mut base = Pixmap::new(40, 40).unwrap();
        assert!(!ink.end(Some(&mut base), &st), "lebar 2 px: dianggap klik");
        assert_eq!(ink.last_seq(), None);
        assert!(base.pixels().iter().all(|p| p.alpha() == 0), "tidak ada yang tergambar");
    }

    #[test]
    fn arrow_draws_shaft_and_head() {
        let st = style();
        let mut ink = Ink { tool: Tool::Arrow, ..Ink::default() };
        ink.begin(5.0, 20.0, 1, &st);
        ink.extend(55.0, 20.0);
        let mut base = Pixmap::new(60, 40).unwrap();
        assert!(ink.end(Some(&mut base), &st));
        assert_eq!(base.pixel(20, 20).unwrap().alpha(), 255, "batang");
        // Sayap kepala panah ada di atas dan bawah batang dekat ujung.
        assert!(base.pixel(46, 15).unwrap().alpha() > 0, "sayap atas");
        assert!(base.pixel(46, 25).unwrap().alpha() > 0, "sayap bawah");
        assert_eq!(base.pixel(20, 10).unwrap().alpha(), 0);
    }

    #[test]
    fn ellipse_outline_and_tiny_shapes_discarded() {
        let st = style();
        let mut ink = Ink { tool: Tool::Ellipse, ..Ink::default() };
        ink.begin(5.0, 5.0, 1, &st);
        ink.extend(45.0, 35.0);
        let mut base = Pixmap::new(50, 40).unwrap();
        assert!(ink.end(Some(&mut base), &st));
        assert_eq!(base.pixel(25, 5).unwrap().alpha(), 255, "tepi atas elips");
        assert_eq!(base.pixel(25, 20).unwrap().alpha(), 0, "bagian dalam kosong");
        assert_eq!(base.pixel(6, 6).unwrap().alpha(), 0, "sudut kotak di luar elips");

        ink.tool = Tool::Arrow;
        ink.begin(10.0, 10.0, 2, &st);
        ink.extend(13.0, 13.0);
        assert!(!ink.end(None, &st), "panah 4 px dianggap klik");
    }

    #[test]
    fn arrow_bounds_include_head() {
        let st = style();
        let mut ink = Ink { tool: Tool::Arrow, ..Ink::default() };
        ink.begin(10.0, 10.0, 1, &st);
        ink.extend(50.0, 10.0);
        let (_, y0, _, y1) = ink.active_bounds(&st).unwrap();
        assert!(y0 <= 10.0 - st.arrow_head() && y1 >= 10.0 + st.arrow_head());
    }

    #[test]
    fn settings_remember_last_freehand_and_shape() {
        let mut s = InkSettings::default();
        s.select_shape(false);
        assert_eq!(s.tool, Tool::Rect, "masuk ke bentuk terakhir");
        s.select_shape(true);
        assert_eq!(s.tool, Tool::Arrow, "sudah memakai bentuk: ganti bentuk");
        s.select_freehand(true);
        assert_eq!(s.tool, Tool::Pen, "dari bentuk: kembali ke gambar bebas terakhir");
        s.select_freehand(true);
        assert_eq!(s.tool, Tool::Highlighter, "sudah menggambar bebas: tukar pena/stabilo");
        s.select_shape(false);
        assert_eq!(s.tool, Tool::Arrow, "bentuk terakhir diingat");
        assert_eq!(s.last_freehand, Tool::Highlighter);
        s.next_color(3);
        s.next_color(3);
        s.next_color(3);
        assert_eq!(s.color, 0);
    }

    fn add(ink: &mut Ink, tool: Tool, a: (f32, f32), b: (f32, f32), seq: u64, st: &InkStyle) {
        ink.tool = tool;
        ink.begin(a.0, a.1, seq, st);
        ink.extend(b.0, b.1);
        assert!(ink.end(None, st));
    }

    #[test]
    fn line_tool_draws_straight_segment() {
        let st = style();
        let mut ink = Ink { tool: Tool::Line, ..Ink::default() };
        ink.begin(5.0, 5.0, 1, &st);
        ink.extend(20.0, 30.0);
        ink.extend(35.0, 5.0);
        assert_eq!(ink.current.as_ref().unwrap().points, [(5.0, 5.0), (35.0, 5.0)], "hanya awal + akhir");
        let mut base = Pixmap::new(40, 20).unwrap();
        assert!(ink.end(Some(&mut base), &st));
        assert_eq!(base.pixel(20, 5).unwrap().alpha(), 255);
        assert_eq!(base.pixel(20, 15).unwrap().alpha(), 0);
    }

    #[test]
    fn hit_test_follows_outline_not_interior() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Rect, (100.0, 100.0), (200.0, 160.0), 1, &st);
        add(&mut ink, Tool::Ellipse, (300.0, 100.0), (400.0, 200.0), 2, &st);
        add(&mut ink, Tool::Line, (0.0, 300.0), (100.0, 300.0), 3, &st);
        assert_eq!(ink.hit((150.0, 102.0), &st), Some(1), "tepi atas kotak");
        assert_eq!(ink.hit((150.0, 130.0), &st), None, "tengah kotak bebas untuk menggambar");
        assert_eq!(ink.hit((350.0, 101.0), &st), Some(2), "tepi atas elips");
        assert_eq!(ink.hit((350.0, 150.0), &st), None, "tengah elips");
        assert_eq!(ink.hit((50.0, 305.0), &st), Some(3));
        assert_eq!(ink.hit((50.0, 330.0), &st), None);
    }

    #[test]
    fn hit_prefers_topmost_object() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Line, (0.0, 50.0), (100.0, 50.0), 1, &st);
        add(&mut ink, Tool::Line, (50.0, 0.0), (50.0, 100.0), 2, &st);
        assert_eq!(ink.hit((50.0, 50.0), &st), Some(2));
    }

    #[test]
    fn drag_moves_object_and_undo_restores_position() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Line, (0.0, 50.0), (100.0, 50.0), 1, &st);
        add(&mut ink, Tool::Line, (0.0, 80.0), (100.0, 80.0), 2, &st);
        assert!(ink.start_drag(1, (50.0, 50.0)));
        assert!(ink.is_dragging());
        assert_eq!(ink.hit((50.0, 50.0), &st), None, "objek yang digeser keluar dari daftar");
        ink.drag_to((60.0, 70.0));
        ink.drag_to((70.0, 150.0));
        assert!(ink.end_drag(3));
        assert_eq!(ink.hit((50.0, 50.0), &st), None);
        assert_eq!(ink.hit((40.0, 150.0), &st), Some(1), "pindah +20,+100");
        assert_eq!(ink.strokes[0].id, 1, "urutan tumpukan dipertahankan");
        assert_eq!(ink.last_seq(), Some(3));

        assert!(ink.undo(), "batalkan geser");
        assert_eq!(ink.hit((50.0, 50.0), &st), Some(1), "kembali ke posisi awal");
        assert_eq!(ink.strokes.len(), 2);
        assert!(ink.undo(), "batalkan objek 2");
        assert!(ink.undo(), "batalkan objek 1");
        assert!(ink.strokes.is_empty());
        assert!(!ink.undo());
    }

    #[test]
    fn click_without_moving_records_no_history() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Rect, (10.0, 10.0), (50.0, 50.0), 1, &st);
        assert!(ink.start_drag(1, (10.0, 30.0)));
        assert!(ink.end_drag(2));
        assert_eq!(ink.last_seq(), Some(1), "klik tanpa geser tidak masuk riwayat");
    }

    #[test]
    fn arrow_head_is_grabbable() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Arrow, (10.0, 50.0), (110.0, 50.0), 1, &st);
        // Ujung sayap atas: 16 px ke belakang dan 30° dari batang.
        let h = st.arrow_head();
        let wing = (110.0 - h * 30f32.to_radians().cos(), 50.0 - h * 30f32.to_radians().sin());
        assert_eq!(ink.hit(wing, &st), Some(1));
    }

    #[test]
    fn selection_bounds_cover_hovered_object() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Line, (10.0, 10.0), (50.0, 10.0), 1, &st);
        assert_eq!(ink.active_bounds(&st), None);
        assert!(ink.set_hover(Some(1)));
        assert!(!ink.set_hover(Some(1)), "tidak berubah");
        let (x0, y0, x1, y1) = ink.active_bounds(&st).unwrap();
        assert!(x0 < 10.0 - SELECTION_PAD && x1 > 50.0 + SELECTION_PAD && y0 < 10.0 && y1 > 10.0);
    }

    #[test]
    fn delete_single_object_and_undo_restores_it() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Line, (0.0, 10.0), (100.0, 10.0), 1, &st);
        add(&mut ink, Tool::Line, (0.0, 40.0), (100.0, 40.0), 2, &st);
        add(&mut ink, Tool::Line, (0.0, 70.0), (100.0, 70.0), 3, &st);
        ink.set_hover(Some(2));
        assert!(ink.delete(2, 4));
        assert!(!ink.delete(2, 5), "sudah terhapus");
        assert_eq!(ink.hover(), None);
        assert_eq!(ink.hit((50.0, 40.0), &st), None);
        assert_eq!(ink.hit((50.0, 10.0), &st), Some(1), "objek lain tidak terpengaruh");
        assert_eq!(ink.last_seq(), Some(4));

        assert!(ink.undo());
        assert_eq!(ink.hit((50.0, 40.0), &st), Some(2), "kembali di posisi semula");
        let order: Vec<u64> = ink.strokes.iter().map(|s| s.id).collect();
        assert_eq!(order, [1, 2, 3], "urutan tumpukan dipulihkan");
    }

    #[test]
    fn undo_restores_deleted_object_that_was_moved_before() {
        let st = style();
        let mut ink = Ink::default();
        add(&mut ink, Tool::Line, (0.0, 10.0), (100.0, 10.0), 1, &st);
        ink.start_drag(1, (50.0, 10.0));
        ink.drag_to((50.0, 60.0));
        ink.end_drag(2);
        assert!(ink.delete(1, 3));
        assert!(ink.undo(), "batalkan hapus");
        assert_eq!(ink.hit((50.0, 60.0), &st), Some(1), "kembali di posisi setelah digeser");
        assert!(ink.undo(), "batalkan geser");
        assert_eq!(ink.hit((50.0, 10.0), &st), Some(1));
    }

    #[test]
    fn text_object_add_hit_move_delete_undo() {
        let st = style();
        let mut ink = Ink::default();
        assert!(!ink.add_text((10.0, 10.0), "   ", 1, None, &st), "teks kosong diabaikan");
        let mut base = Pixmap::new(200, 80).unwrap();
        assert!(ink.add_text((10.0, 10.0), "Hello", 1, Some(&mut base), &st));
        assert!(base.pixels().iter().any(|p| p.alpha() > 128), "teks tergambar di base");
        assert_eq!(ink.hit((20.0, 20.0), &st), Some(1), "bagian dalam kotak teks bisa dipegang");
        assert_eq!(ink.hit((150.0, 70.0), &st), None);
        ink.start_drag(1, (20.0, 20.0));
        ink.drag_to((20.0, 50.0));
        ink.end_drag(2);
        assert_eq!(ink.hit((20.0, 50.0), &st), Some(1));
        assert!(ink.delete(1, 3));
        assert_eq!(ink.hit((20.0, 50.0), &st), None);
        assert!(ink.undo() && ink.undo() && ink.undo());
        assert!(ink.strokes.is_empty() && !ink.undo());
    }

    #[test]
    fn pending_text_bounds_cover_preview() {
        let st = style();
        let mut ink = Ink::default();
        ink.set_pending_text(Some(((50.0, 50.0), "Hi\nThere".into())));
        let (x0, y0, x1, y1) = ink.active_bounds(&st).unwrap();
        let (w, h) = text::block_size("Hi\nThere", st.text_size);
        assert!(x0 < 50.0 && y0 < 50.0 && x1 > 50.0 + w && y1 > 50.0 + h);
        ink.set_pending_text(None);
        assert_eq!(ink.active_bounds(&st), None);
    }
}
