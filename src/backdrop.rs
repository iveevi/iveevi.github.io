use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, CssStyleDeclaration, HtmlCanvasElement, HtmlImageElement};

use crate::rng::Pcg;

const SHADER: &str = include_str!("../gen/backdrop.wgsl");

const IMAGES: usize = 7;

pub const LOCATIONS: [&str; IMAGES] = [
    "Cascade Canyon, Grand Teton National Park, WY",
    "Oxbow Bend, Grand Teton National Park, WY",
    "Jackson Lake, Grand Teton National Park, WY",
    "Torrey Pines State Beach, San Diego, CA",
    "La Jolla, San Diego, CA",
    "Point Loma, San Diego, CA",
    "La Jolla Shores, San Diego, CA",
];
pub const HOLD: f32 = 10.0;
const FADE: f32 = 2.5;
const QUICK: f32 = 0.6;
const SHIMMER: f32 = 8.0;
const SETTLE: f32 = 0.25;
const CANDIDATES: f32 = 1.6;
pub const SAMPLES: usize = 1 << 16;
pub const MAX_SAMPLES: usize = 1 << 17;
const PACKING: f32 = 0.60;
const SAFETY: f32 = 0.6;

#[derive(Clone, Copy)]
struct Dart {
    x: f32,
    y: f32,
}

struct Bins {
    starts: Vec<u32>,
    items: Vec<u32>,
    cols: usize,
    rows: usize,
    cell: f32,
}

impl Bins {
    fn new(darts: &[Dart], width: f32, height: f32, cell: f32) -> Bins {
        let cols = (width / cell).ceil().max(1.0) as usize;
        let rows = (height / cell).ceil().max(1.0) as usize;
        let mut counts = vec![0u32; cols * rows + 1];

        let slot = |dart: &Dart| {
            let col = ((dart.x / cell) as usize).min(cols - 1);
            let row = ((dart.y / cell) as usize).min(rows - 1);
            return row * cols + col;
        };

        for dart in darts {
            counts[slot(dart) + 1] += 1;
        }
        for index in 1..counts.len() {
            counts[index] += counts[index - 1];
        }

        let mut cursor = counts.clone();
        let mut items = vec![0u32; darts.len()];
        for (index, dart) in darts.iter().enumerate() {
            let bucket = slot(dart);
            items[cursor[bucket] as usize] = index as u32;
            cursor[bucket] += 1;
        }

        return Bins {
            starts: counts,
            items,
            cols,
            rows,
            cell,
        };
    }

    fn bucket(&self, index: usize) -> &[u32] {
        let lo = self.starts[index] as usize;
        let hi = self.starts[index + 1] as usize;
        return &self.items[lo..hi];
    }

    fn span(&self, dart: Dart) -> (usize, usize, usize, usize) {
        let col = (dart.x / self.cell) as i32;
        let row = (dart.y / self.cell) as i32;
        return (
            (col - 1).max(0) as usize,
            (col + 2).min(self.cols as i32) as usize,
            (row - 1).max(0) as usize,
            (row + 2).min(self.rows as i32) as usize,
        );
    }
}

#[derive(Clone, Copy)]
struct Entry {
    weight: f32,
    index: u32,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        return self.weight == other.weight;
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Entry) -> Option<std::cmp::Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Entry) -> std::cmp::Ordering {
        return self
            .weight
            .partial_cmp(&other.weight)
            .unwrap_or(std::cmp::Ordering::Equal);
    }
}

fn falloff(squared: f32, ceiling: f32) -> f32 {
    let ratio = 1.0 - squared / ceiling;
    let square = ratio * ratio;
    return square * square;
}

fn squared(a: Dart, b: Dart) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    return dx * dx + dy * dy;
}

fn blue_noise(width: f32, height: f32, samples: usize) -> Vec<Dart> {
    let margin = 1.5;
    let mut rng = Pcg::new(0x51ed);
    let pool = (samples as f32 * CANDIDATES) as usize;
    let darts: Vec<Dart> = (0..pool)
        .map(|_| Dart {
            x: rng.range(margin, width - 1.0 - margin),
            y: rng.range(margin, height - 1.0 - margin),
        })
        .collect();

    let root = 3.0_f32.sqrt();
    let limit = 2.0 * (width * height / (2.0 * root * samples as f32)).sqrt();
    let ceiling = limit * limit;

    let bins = Bins::new(&darts, width, height, limit);
    let mut weights = vec![0.0_f32; pool];

    for row in 0..bins.rows {
        for col in 0..bins.cols {
            for index in bins.bucket(row * bins.cols + col) {
                let index = *index as usize;
                let dart = darts[index];
                let (lo_col, hi_col, lo_row, hi_row) = bins.span(dart);
                let mut total = 0.0;
                for y in lo_row..hi_row {
                    for x in lo_col..hi_col {
                        for other in bins.bucket(y * bins.cols + x) {
                            let other = *other as usize;
                            if other <= index {
                                continue;
                            }
                            let distance = squared(dart, darts[other]);
                            if distance < ceiling {
                                let shed = falloff(distance, ceiling);
                                total += shed;
                                weights[other] += shed;
                            }
                        }
                    }
                }
                weights[index] += total;
            }
        }
    }

    let mut heap: std::collections::BinaryHeap<Entry> = (0..pool)
        .map(|index| Entry {
            weight: weights[index],
            index: index as u32,
        })
        .collect();

    let mut alive = vec![true; pool];
    let mut remaining = pool;

    while remaining > samples {
        let Some(entry) = heap.pop() else { break };
        let index = entry.index as usize;
        if !alive[index] {
            continue;
        }
        if entry.weight != weights[index] {
            heap.push(Entry {
                weight: weights[index],
                index: entry.index,
            });
            continue;
        }

        alive[index] = false;
        remaining -= 1;

        let dart = darts[index];
        let (lo_col, hi_col, lo_row, hi_row) = bins.span(dart);
        for y in lo_row..hi_row {
            for x in lo_col..hi_col {
                for other in bins.bucket(y * bins.cols + x) {
                    let other = *other as usize;
                    if other == index || !alive[other] {
                        continue;
                    }
                    let distance = squared(dart, darts[other]);
                    if distance < ceiling {
                        weights[other] -= falloff(distance, ceiling);
                    }
                }
            }
        }
    }

    return (0..pool)
        .filter(|index| alive[*index])
        .map(|index| darts[index])
        .collect();
}

fn point(x: f32, y: f32) -> delaunator::Point {
    return delaunator::Point {
        x: x as f64,
        y: y as f64,
    };
}

fn frame_points(darts: &[Dart], width: f32, height: f32, spacing: f32) -> Vec<delaunator::Point> {
    let columns = (width / spacing) as usize + 2;
    let rows = (height / spacing) as usize + 2;
    let mut points: Vec<delaunator::Point> =
        darts.iter().map(|dart| point(dart.x, dart.y)).collect();

    points.extend((0..columns).flat_map(|i| {
        let x = (width - 1.0) * i as f32 / (columns - 1) as f32;
        return [point(x, 0.0), point(x, height - 1.0)];
    }));

    points.extend((1..rows - 1).flat_map(|i| {
        let y = (height - 1.0) * i as f32 / (rows - 1) as f32;
        return [point(0.0, y), point(width - 1.0, y)];
    }));

    return points;
}

fn opposite_height(a: &delaunator::Point, b: &delaunator::Point, c: &delaunator::Point) -> f64 {
    let area = ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs();
    let base = ((c.x - b.x).powi(2) + (c.y - b.y).powi(2)).sqrt();
    if base < 1e-9 {
        return 0.0;
    }

    return area / base;
}

fn lowest_altitude(points: &[delaunator::Point], facet: &[usize]) -> f64 {
    return (0..3)
        .map(|k| {
            return opposite_height(
                &points[facet[k]],
                &points[facet[(k + 1) % 3]],
                &points[facet[(k + 2) % 3]],
            );
        })
        .fold(f64::MAX, f64::min);
}

fn kernel_radii(points: &[delaunator::Point], triangles: &[usize]) -> Vec<f32> {
    let mut radii = vec![f32::MAX; points.len()];
    for facet in triangles.chunks(3) {
        let bound = 0.5 * lowest_altitude(points, facet) as f32;
        for corner in facet {
            radii[*corner] = radii[*corner].min(bound);
        }
    }

    return radii;
}

fn node_bytes(points: &[delaunator::Point], radii: &[f32], width: f32, height: f32) -> Vec<u8> {
    return points
        .iter()
        .zip(radii)
        .flat_map(|(p, radius)| {
            let x = p.x as f32;
            let y = p.y as f32;
            let edge = x < 1.5 || y < 1.5 || x > width - 2.5 || y > height - 2.5;
            let amplitude = if edge { 0.0 } else { SAFETY * radius };
            return [x, y, amplitude];
        })
        .flat_map(|v| v.to_le_bytes())
        .collect();
}

#[derive(Clone, Copy)]
pub struct MeshTiming {
    pub sampler: f32,
    pub frame: f32,
    pub triangulate: f32,
    pub pack: f32,
    pub points: usize,
}

thread_local! {
    pub static MESH_TIMING: Cell<MeshTiming> =
        const { Cell::new(MeshTiming { sampler: 0.0, frame: 0.0, triangulate: 0.0, pack: 0.0, points: 0 }) };
}

struct Mesh {
    nodes: Vec<u8>,
    facets: Vec<u8>,
    count: u32,
}

fn build_mesh(width: u32, height: u32, samples: usize) -> Mesh {
    let started = millis();
    let radius = (PACKING * (width * height) as f32 / samples as f32).sqrt();
    let mut darts = blue_noise(width as f32, height as f32, samples);
    darts.truncate(2 * MAX_SAMPLES);
    let sampled = millis();

    let anchors = frame_points(&darts, width as f32, height as f32, radius);
    let framed = millis();

    let mesh = delaunator::triangulate(&anchors);
    let triangulated = millis();

    let radii = kernel_radii(&anchors, &mesh.triangles);
    let nodes = node_bytes(&anchors, &radii, width as f32, height as f32);

    let mut facets = Vec::with_capacity(4 * mesh.triangles.len());
    for corner in &mesh.triangles {
        facets.extend_from_slice(&(*corner as u32).to_le_bytes());
    }

    MESH_TIMING.with(|timing| {
        timing.set(MeshTiming {
            sampler: sampled - started,
            frame: framed - sampled,
            triangulate: triangulated - framed,
            pack: millis() - triangulated,
            points: anchors.len(),
        })
    });

    return Mesh {
        nodes,
        facets,
        count: mesh.triangles.len() as u32,
    };
}

fn cover_source(image: &HtmlImageElement, width: f64, height: f64) -> [f64; 4] {
    let source_width = image.natural_width() as f64;
    let source_height = image.natural_height() as f64;
    let scale = (width / source_width).max(height / source_height);
    let visible_width = width / scale;
    let visible_height = height / scale;
    return [
        0.5 * (source_width - visible_width),
        0.5 * (source_height - visible_height),
        visible_width,
        visible_height,
    ];
}

fn scratch_canvas(width: u32, height: u32) -> HtmlCanvasElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_width(width);
    canvas.set_height(height);
    return canvas;
}

#[allow(dead_code)]
fn cover_draw(canvas: &HtmlCanvasElement, image: &HtmlImageElement) {
    let Ok(Some(context)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(context) = context.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    let [sx, sy, sw, sh] = cover_source(image, width, height);
    let _ = context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        image, sx, sy, sw, sh, 0.0, 0.0, width, height,
    );
}

fn cover_pixels(canvas: &HtmlCanvasElement, image: &HtmlImageElement) -> Vec<u8> {
    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    let [sx, sy, sw, sh] = cover_source(image, width, height);
    context
        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            image, sx, sy, sw, sh, 0.0, 0.0, width, height,
        )
        .unwrap();

    let data = context.get_image_data(0.0, 0.0, width, height).unwrap();
    return data.data().0;
}

struct Backdrop {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: RefCell<wgpu::SurfaceConfiguration>,
    canvas: HtmlCanvasElement,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    groups: RefCell<[wgpu::BindGroup; 2]>,
    textures: RefCell<[wgpu::Texture; 2]>,
    last: Cell<(f32, f32, f32, f32)>,
    dirty: Cell<bool>,
    params: wgpu::Buffer,
    positions: wgpu::Buffer,
    indices: wgpu::Buffer,
    scratch: RefCell<HtmlCanvasElement>,
    count: Cell<u32>,
    samples: Cell<usize>,
    width: Cell<u32>,
    height: Cell<u32>,
    origin: f32,
    current: Cell<usize>,
    index: Cell<usize>,
    switched: Cell<f32>,
    fade: Cell<f32>,
    loading: Cell<bool>,
    wrapper: CssStyleDeclaration,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
    jump: RwSignal<i32>,
    stats: RwSignal<Stats>,
    lost: RwSignal<bool>,
    frames: Cell<u32>,
    drawn: Cell<u32>,
    window: Cell<f32>,
    previous: Cell<f32>,
    worst: Cell<f32>,
    draw_ms: Cell<f32>,
    mesh_ms: Cell<f32>,
    swap_ms: Cell<f32>,
    upload_mb: Cell<f32>,
    swaps: Cell<u32>,
    peak: Cell<f32>,
    history: Cell<[f32; HISTORY]>,
    cursor: Cell<usize>,
    gpu_ms: f32,
    surface_ms: f32,
    boot_ms: Cell<f32>,
    pending: Cell<(u32, u32)>,
    settled_at: Cell<f32>,
}

pub const HISTORY: usize = 192;

#[derive(Clone, Copy, PartialEq)]
pub struct Stats {
    pub fps: f32,
    pub drawn: f32,
    pub frame: f32,
    pub worst: f32,
    pub draw_ms: f32,
    pub mesh_ms: f32,
    pub mesh_sampler: f32,
    pub mesh_triangulate: f32,
    pub mesh_frame: f32,
    pub mesh_pack: f32,
    pub mesh_points: usize,
    pub swap_ms: f32,
    pub upload_mb: f32,
    pub heap_mb: f32,
    pub peak_mb: f32,
    pub uptime: f32,
    pub gpu_ms: f32,
    pub surface_ms: f32,
    pub boot_ms: f32,
    pub boot: Boot,
    pub swaps: u32,
    pub samples: usize,
    pub triangles: u32,
    pub width: u32,
    pub height: u32,
    pub history: [f32; HISTORY],
}

impl Default for Stats {
    fn default() -> Self {
        return Stats {
            fps: 0.0,
            drawn: 0.0,
            frame: 0.0,
            worst: 0.0,
            draw_ms: 0.0,
            mesh_ms: 0.0,
            mesh_sampler: 0.0,
            mesh_triangulate: 0.0,
            mesh_frame: 0.0,
            mesh_pack: 0.0,
            mesh_points: 0,
            swap_ms: 0.0,
            upload_mb: 0.0,
            heap_mb: 0.0,
            peak_mb: 0.0,
            uptime: 0.0,
            gpu_ms: 0.0,
            surface_ms: 0.0,
            boot_ms: 0.0,
            boot: Boot { wasm: 0.0, mount: 0.0, first_frame: 0.0, visible: 0.0 },
            swaps: 0,
            samples: 0,
            triangles: 0,
            width: 0,
            height: 0,
            history: [0.0; HISTORY],
        };
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Boot {
    pub wasm: f32,
    pub mount: f32,
    pub first_frame: f32,
    pub visible: f32,
}

thread_local! {
    pub static BOOT: Cell<Boot> = const { Cell::new(Boot {
        wasm: 0.0,
        mount: 0.0,
        first_frame: 0.0,
        visible: 0.0,
    }) };
}

pub fn mark(field: impl FnOnce(&mut Boot)) {
    BOOT.with(|boot| {
        let mut value = boot.get();
        field(&mut value);
        boot.set(value);
    });
}

pub fn millis() -> f32 {
    return web_sys::window().unwrap().performance().unwrap().now() as f32;
}

fn heap_mb() -> f32 {
    let memory = wasm_bindgen::memory();
    let Ok(buffer) = js_sys::Reflect::get(&memory, &"buffer".into()) else {
        return 0.0;
    };
    let Ok(length) = js_sys::Reflect::get(&buffer, &"byteLength".into()) else {
        return 0.0;
    };
    return length.as_f64().unwrap_or(0.0) as f32 / (1024.0 * 1024.0);
}

fn upload_mesh(backdrop: &Backdrop, samples: usize) {
    let started = millis();
    let mesh = build_mesh(backdrop.width.get(), backdrop.height.get(), samples);
    backdrop
        .queue
        .write_buffer(&backdrop.positions, 0, &mesh.nodes);
    backdrop
        .queue
        .write_buffer(&backdrop.indices, 0, &mesh.facets);
    backdrop.count.set(mesh.count);
    backdrop.samples.set(samples);
    backdrop.mesh_ms.set(millis() - started);
    backdrop.dirty.set(true);
}

fn make_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    let descriptor = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    return device.create_texture(&descriptor);
}

#[allow(dead_code)]
fn upload_canvas(backdrop: &Backdrop, slot: usize) {
    let textures = backdrop.textures.borrow();
    let source = wgpu::CopyExternalImageSourceInfo {
        source: wgpu::ExternalImageSource::HTMLCanvasElement(backdrop.scratch.borrow().clone()),
        origin: wgpu::Origin2d::ZERO,
        flip_y: false,
    };
    let destination = wgpu::CopyExternalImageDestInfo {
        texture: &textures[slot],
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
        color_space: wgpu::PredefinedColorSpace::Srgb,
        premultiplied_alpha: false,
    };
    let size = wgpu::Extent3d {
        width: backdrop.width.get(),
        height: backdrop.height.get(),
        depth_or_array_layers: 1,
    };
    backdrop
        .queue
        .copy_external_image_to_texture(&source, destination, size);
}

fn upload(backdrop: &Backdrop, slot: usize, pixels: &[u8]) {
    let textures = backdrop.textures.borrow();
    let texture = &textures[slot];
    let destination = wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    };
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4 * backdrop.width.get()),
        rows_per_image: Some(backdrop.height.get()),
    };
    let size = wgpu::Extent3d {
        width: backdrop.width.get(),
        height: backdrop.height.get(),
        depth_or_array_layers: 1,
    };
    backdrop.queue.write_texture(destination, pixels, layout, size);
}

fn make_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let descriptor = wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    };
    let shader = device.create_shader_module(descriptor);
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    };
    return device.create_render_pipeline(&pipeline_descriptor);
}

async fn init(
    canvas: HtmlCanvasElement,
    wrapper: CssStyleDeclaration,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
    jump: RwSignal<i32>,
    stats: RwSignal<Stats>,
) -> Option<Backdrop> {
    let width = canvas.width();
    let height = canvas.height();

    let boot = millis();
    let crate::Gpu { instance, adapter, device, queue } = crate::gpu().await?;
    let gpu_ms = millis() - boot;
    let lost = crate::watch(&device, "backdrop");

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .ok()?;
    let caps = surface.get_capabilities(&adapter);
    let format = *caps.formats.first()?;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        color_space: Default::default(),
        alpha_mode: *caps.alpha_modes.first()?,
        view_formats: Vec::new(),
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);
    let surface_ms = millis() - boot - gpu_ms;

    let headroom = (2 * MAX_SAMPLES + 8192) as u64;
    let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    let positions = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 12 * headroom,
        usage: storage,
        mapped_at_creation: false,
    });
    let indices = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 24 * headroom,
        usage: storage,
        mapped_at_creation: false,
    });

    let params = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 32,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let pipeline = make_pipeline(&device, format);
    let textures = [
        make_texture(&device, width, height),
        make_texture(&device, width, height),
    ];
    let groups = [
        make_group(&device, &pipeline, &params, &positions, &indices, &sampler, &textures, 0),
        make_group(&device, &pipeline, &params, &positions, &indices, &sampler, &textures, 1),
    ];

    let backdrop = Backdrop {
        scratch: RefCell::new(scratch_canvas(width, height)),
        count: Cell::new(0),
        samples: Cell::new(0),
        pipeline,
        textures: RefCell::new(textures),
        groups: RefCell::new(groups),
        surface,
        config: RefCell::new(config),
        canvas,
        sampler,
        last: Cell::new((f32::NAN, f32::NAN, f32::NAN, f32::NAN)),
        dirty: Cell::new(true),
        device,
        queue,
        params,
        positions,
        indices,
        width: Cell::new(width),
        height: Cell::new(height),
        origin: now(),
        current: Cell::new(0),
        index: Cell::new(0),
        switched: Cell::new(f32::MIN),
        fade: Cell::new(FADE),
        loading: Cell::new(false),
        wrapper,
        tint,
        exposure,
        resolution,
        caption,
        jump,
        stats,
        lost,
        frames: Cell::new(0),
        drawn: Cell::new(0),
        window: Cell::new(now()),
        previous: Cell::new(0.0),
        worst: Cell::new(0.0),
        draw_ms: Cell::new(0.0),
        mesh_ms: Cell::new(0.0),
        swap_ms: Cell::new(0.0),
        upload_mb: Cell::new(0.0),
        swaps: Cell::new(0),
        peak: Cell::new(0.0),
        history: Cell::new([0.0; HISTORY]),
        cursor: Cell::new(0),
        gpu_ms,
        surface_ms,
        boot_ms: Cell::new(0.0),
        pending: Cell::new((width, height)),
        settled_at: Cell::new(0.0),
    };
    upload_mesh(&backdrop, resolution.get_untracked());

    return Some(backdrop);
}

fn make_group(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    params: &wgpu::Buffer,
    positions: &wgpu::Buffer,
    indices: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    textures: &[wgpu::Texture; 2],
    current: usize,
) -> wgpu::BindGroup {
    let previous = textures[1 - current].create_view(&Default::default());
    let latest = textures[current].create_view(&Default::default());
    let entries = [
        wgpu::BindGroupEntry {
            binding: 0,
            resource: params.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: positions.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: indices.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 3,
            resource: wgpu::BindingResource::TextureView(&previous),
        },
        wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::TextureView(&latest),
        },
        wgpu::BindGroupEntry {
            binding: 5,
            resource: wgpu::BindingResource::Sampler(sampler),
        },
    ];
    return device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
}

fn blend_at(backdrop: &Backdrop, time: f32) -> f32 {
    return ((time - backdrop.switched.get()) / backdrop.fade.get()).clamp(0.0, 1.0);
}

fn draw(backdrop: &Rc<Backdrop>, time: f32) -> bool {
    let started = millis();
    let tint = backdrop.tint.get_untracked();
    // the shimmer needs fresh frames, but not every frame
    let beat = if tint > 0.0 { (time * SHIMMER).floor() } else { 0.0 };
    let state = (
        blend_at(backdrop, time),
        tint,
        backdrop.exposure.get_untracked(),
        beat,
    );
    if !backdrop.dirty.get() && state == backdrop.last.get() {
        return false;
    }
    backdrop.last.set(state);
    backdrop.dirty.set(false);

    let floats = [
        backdrop.width.get() as f32,
        backdrop.height.get() as f32,
        time,
        state.0,
        state.1,
        state.2,
    ];
    let bytes: Vec<u8> = floats.iter().flat_map(|v| v.to_le_bytes()).collect();
    backdrop.queue.write_buffer(&backdrop.params, 0, &bytes);

    let frame = match backdrop.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame) => frame,
        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        _ => return false,
    };
    let view = frame.texture.create_view(&Default::default());
    let mut encoder = backdrop.device.create_command_encoder(&Default::default());
    let attachments = [Some(wgpu::RenderPassColorAttachment {
        view: &view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: wgpu::StoreOp::Store,
        },
    })];
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &attachments,
        ..Default::default()
    });
    pass.set_pipeline(&backdrop.pipeline);
    pass.set_bind_group(0, &backdrop.groups.borrow()[backdrop.current.get()], &[]);
    pass.draw(0..backdrop.count.get(), 0..1);
    drop(pass);

    backdrop.queue.submit([encoder.finish()]);
    backdrop.queue.present(frame);
    backdrop.draw_ms.set(millis() - started);
    return true;
}

fn sample(backdrop: &Rc<Backdrop>, time: f32, drawn: bool) {
    let previous = backdrop.previous.get();
    backdrop.previous.set(time);
    if previous > 0.0 {
        let delta = (time - previous) * 1000.0;
        backdrop.worst.set(backdrop.worst.get().max(delta));

        let mut history = backdrop.history.get();
        let cursor = backdrop.cursor.get();
        history[cursor] = delta;
        backdrop.history.set(history);
        backdrop.cursor.set((cursor + 1) % HISTORY);
    }

    backdrop.frames.set(backdrop.frames.get() + 1);
    if drawn {
        backdrop.drawn.set(backdrop.drawn.get() + 1);
    }

    let start = backdrop.window.get();
    let span = time - start;
    if span < 0.5 {
        return;
    }

    let heap = heap_mb();
    backdrop.peak.set(backdrop.peak.get().max(heap));

    let history = backdrop.history.get();
    let frames = backdrop.frames.get() as f32;
    backdrop.stats.set(Stats {
        fps: frames / span,
        drawn: backdrop.drawn.get() as f32 / span,
        frame: span * 1000.0 / frames.max(1.0),
        worst: backdrop.worst.get(),
        draw_ms: backdrop.draw_ms.get(),
        mesh_ms: backdrop.mesh_ms.get(),
        mesh_sampler: MESH_TIMING.with(|t| t.get().sampler),
        mesh_triangulate: MESH_TIMING.with(|t| t.get().triangulate),
        mesh_frame: MESH_TIMING.with(|t| t.get().frame),
        mesh_pack: MESH_TIMING.with(|t| t.get().pack),
        mesh_points: MESH_TIMING.with(|t| t.get().points),
        swap_ms: backdrop.swap_ms.get(),
        upload_mb: backdrop.upload_mb.get(),
        heap_mb: heap,
        peak_mb: backdrop.peak.get(),
        uptime: time,
        gpu_ms: backdrop.gpu_ms,
        surface_ms: backdrop.surface_ms,
        boot_ms: backdrop.boot_ms.get(),
        boot: BOOT.with(|boot| boot.get()),
        swaps: backdrop.swaps.get(),
        samples: backdrop.samples.get(),
        triangles: backdrop.count.get() / 3,
        width: backdrop.width.get(),
        height: backdrop.height.get(),
        history,
    });
    backdrop.window.set(time);
    backdrop.frames.set(0);
    backdrop.drawn.set(0);
    backdrop.worst.set(0.0);
}


async fn load(index: usize) -> Option<HtmlImageElement> {
    let image = HtmlImageElement::new().unwrap();
    image.set_src(&format!("gallery/{index}.jpg"));
    if JsFuture::from(image.decode()).await.is_err() {
        return None;
    }

    return Some(image);
}

fn advance(backdrop: Rc<Backdrop>) {
    let index = backdrop.index.get();
    backdrop.index.set((index + 1) % IMAGES);
    backdrop.loading.set(true);

    spawn_local(async move {
        let Some(image) = load(index).await else {
            backdrop.loading.set(false);
            return;
        };

        let started = millis();
        let pixels = cover_pixels(&backdrop.scratch.borrow(), &image);
        let slot = 1 - backdrop.current.get();
        upload(&backdrop, slot, &pixels);
        backdrop.upload_mb.set(0.0);
        backdrop.swaps.set(backdrop.swaps.get() + 1);
        backdrop.swap_ms.set(millis() - started);

        if backdrop.switched.get() == f32::MIN {
            upload(&backdrop, 1 - slot, &pixels);
        }

        backdrop.caption.set(index);
        backdrop.current.set(slot);
        backdrop.switched.set(elapsed(&backdrop));
        backdrop.loading.set(false);
        backdrop.dirty.set(true);
        backdrop
            .wrapper
            .set_property("opacity", "var(--backdrop-opacity)")
            .unwrap();
        settled();
    });
}

pub fn settled() {
    let now = millis();
    mark(move |boot| {
        if boot.visible == 0.0 {
            boot.visible = now;
        }
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let Some(element) = document.get_element_by_id("prebackdrop") else {
        return;
    };
    let _ = element.set_class_name("prebackdrop is-done");
}

fn now() -> f32 {
    let performance = web_sys::window().unwrap().performance().unwrap();
    return (performance.now() / 1000.0) as f32;
}

fn elapsed(backdrop: &Backdrop) -> f32 {
    return now() - backdrop.origin;
}

fn tick(backdrop: Rc<Backdrop>) {
    if backdrop.lost.get_untracked() {
        return;
    }
    if crate::hidden() {
        request_animation_frame(move || tick(backdrop));
        return;
    }
    let time = elapsed(&backdrop);
    let step = backdrop.jump.get_untracked();
    // one queued step at a time: swapping the source texture mid-fade snaps
    let settled = !backdrop.loading.get() && blend_at(&backdrop, time) >= 1.0;
    if step != 0 && settled {
        let direction = step.signum();
        backdrop.jump.set(step - direction);
        backdrop.fade.set(QUICK);
        if direction < 0 {
            let index = backdrop.index.get();
            backdrop.index.set((index + IMAGES - 2) % IMAGES);
        }
        advance(backdrop.clone());
    } else if settled && time - backdrop.switched.get() >= HOLD {
        backdrop.fade.set(FADE);
        advance(backdrop.clone());
    }

    rescale(&backdrop, time);

    let wanted = backdrop.resolution.get_untracked();
    if wanted != backdrop.samples.get() {
        upload_mesh(&backdrop, wanted);
    }

    let drawn = draw(&backdrop, time);
    sample(&backdrop, time, drawn);
    request_animation_frame(move || tick(backdrop));
}

async fn run(
    canvas: HtmlCanvasElement,
    wrapper: CssStyleDeclaration,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
    jump: RwSignal<i32>,
    stats: RwSignal<Stats>,
) {
    let opened = millis();
    let backdrop = init(canvas, wrapper.clone(), tint, exposure, resolution, caption, jump, stats).await;
    let Some(backdrop) = backdrop else {
        return still(&wrapper);
    };
    backdrop.boot_ms.set(millis() - opened);
    let now = millis();
    mark(move |boot| boot.first_frame = now);
    tick(Rc::new(backdrop));
}

fn still(wrapper: &CssStyleDeclaration) {
    let _ = wrapper.set_property("background-image", "url(gallery/0.jpg)");
    let _ = wrapper.set_property("background-size", "cover");
    let _ = wrapper.set_property("background-position", "center");
    let _ = wrapper.set_property("filter", "blur(18px) saturate(1.1)");
    let _ = wrapper.set_property("opacity", "var(--backdrop-opacity)");
}

fn reload(backdrop: Rc<Backdrop>) {
    if backdrop.loading.get() {
        return;
    }
    let index = (backdrop.index.get() + IMAGES - 1) % IMAGES;
    backdrop.loading.set(true);
    spawn_local(async move {
        let Some(image) = load(index).await else {
            backdrop.loading.set(false);
            return;
        };
        let pixels = cover_pixels(&backdrop.scratch.borrow(), &image);
        upload(&backdrop, 0, &pixels);
        upload(&backdrop, 1, &pixels);
        backdrop.loading.set(false);
        backdrop.dirty.set(true);
    });
}

fn rescale(backdrop: &Rc<Backdrop>, time: f32) {
    let window = web_sys::window().unwrap();
    let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
    let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
    if width == 0 || height == 0 || (width == backdrop.width.get() && height == backdrop.height.get()) {
        return;
    }

    if backdrop.pending.get() != (width, height) {
        backdrop.pending.set((width, height));
        backdrop.settled_at.set(time);
        return;
    }
    if time - backdrop.settled_at.get() < SETTLE || backdrop.loading.get() {
        return;
    }

    backdrop.width.set(width);
    backdrop.height.set(height);
    backdrop.canvas.set_width(width);
    backdrop.canvas.set_height(height);

    let mut config = backdrop.config.borrow_mut();
    config.width = width;
    config.height = height;
    backdrop.surface.configure(&backdrop.device, &config);
    drop(config);

    *backdrop.scratch.borrow_mut() = scratch_canvas(width, height);
    let textures = [
        make_texture(&backdrop.device, width, height),
        make_texture(&backdrop.device, width, height),
    ];
    *backdrop.groups.borrow_mut() = [
        make_group(&backdrop.device, &backdrop.pipeline, &backdrop.params, &backdrop.positions,
                   &backdrop.indices, &backdrop.sampler, &textures, 0),
        make_group(&backdrop.device, &backdrop.pipeline, &backdrop.params, &backdrop.positions,
                   &backdrop.indices, &backdrop.sampler, &textures, 1),
    ];
    *backdrop.textures.borrow_mut() = textures;

    upload_mesh(backdrop, backdrop.samples.get());
    backdrop.dirty.set(true);
    reload(backdrop.clone());
}

fn resize(canvas: &HtmlCanvasElement) {
    let window = web_sys::window().unwrap();
    let width = window.inner_width().unwrap().as_f64().unwrap();
    let height = window.inner_height().unwrap().as_f64().unwrap();
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);
}

#[component]
pub fn Backdrop(
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
    jump: RwSignal<i32>,
    stats: RwSignal<Stats>,
) -> impl IntoView {
    let wrapper = NodeRef::<leptos::html::Div>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    Effect::new(move || {
        let (Some(canvas), Some(root)) = (canvas_ref.get(), wrapper.get()) else {
            return;
        };
        let element: web_sys::HtmlElement = root.into();
        let target: HtmlCanvasElement = canvas.into();
        resize(&target);
        spawn_local(run(target, element.style(), tint, exposure, resolution, caption, jump, stats));
    });

    return view! {
        <div class="backdrop" node_ref=wrapper>
            <canvas node_ref=canvas_ref></canvas>
        </div>
    };
}
