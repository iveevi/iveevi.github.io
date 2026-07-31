use std::cell::Cell;
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
const HOLD: f32 = 10.0;
const FADE: f32 = 2.5;
const ATTEMPTS: u32 = 24;
pub const SAMPLES: usize = 1 << 16;
pub const MAX_SAMPLES: usize = 1 << 17;
const PACKING: f32 = 0.60;
const SAFETY: f32 = 0.6;
const SAMPLE_COUNT: u32 = 4;

#[derive(Clone, Copy)]
struct Dart {
    x: f32,
    y: f32,
}

struct Grid {
    cells: Vec<i32>,
    cols: usize,
    rows: usize,
    cell: f32,
    radius: f32,
}

impl Grid {
    fn new(width: usize, height: usize, radius: f32) -> Grid {
        let cell = radius / std::f32::consts::SQRT_2;
        let cols = (width as f32 / cell) as usize + 1;
        let rows = (height as f32 / cell) as usize + 1;
        return Grid {
            cells: vec![-1; cols * rows],
            cols,
            rows,
            cell,
            radius,
        };
    }

    fn insert(&mut self, index: usize, dart: Dart) {
        let col = (dart.x / self.cell) as usize;
        let row = (dart.y / self.cell) as usize;
        self.cells[row * self.cols + col] = index as i32;
    }

    fn accepts(&self, darts: &[Dart], candidate: Dart) -> bool {
        let col = (candidate.x / self.cell) as i32;
        let row = (candidate.y / self.cell) as i32;
        let lo_col = (col - 2).max(0) as usize;
        let hi_col = (col + 3).min(self.cols as i32) as usize;
        let lo_row = (row - 2).max(0) as usize;
        let hi_row = (row + 3).min(self.rows as i32) as usize;

        return (lo_row..hi_row)
            .flat_map(|y| (lo_col..hi_col).map(move |x| y * self.cols + x))
            .map(|i| self.cells[i])
            .filter(|&index| index >= 0)
            .all(|index| separated(darts[index as usize], candidate, self.radius));
    }
}

fn separated(existing: Dart, candidate: Dart, radius: f32) -> bool {
    let dx = existing.x - candidate.x;
    let dy = existing.y - candidate.y;
    return dx * dx + dy * dy >= radius * radius;
}

fn propose(
    rng: &mut Pcg,
    anchor: Dart,
    grid: &Grid,
    darts: &[Dart],
    width: f32,
    height: f32,
) -> Option<Dart> {
    let angle = rng.range(0.0, std::f32::consts::TAU);
    let offset = rng.range(grid.radius, 2.0 * grid.radius);
    let margin = 0.5 * grid.radius;
    let candidate = Dart {
        x: anchor.x + offset * angle.cos(),
        y: anchor.y + offset * angle.sin(),
    };

    let within = candidate.x > margin
        && candidate.y > margin
        && candidate.x < width - 1.0 - margin
        && candidate.y < height - 1.0 - margin;
    if !within || !grid.accepts(darts, candidate) {
        return None;
    }

    return Some(candidate);
}

fn spawn_dart(darts: &mut Vec<Dart>, active: &mut Vec<usize>, grid: &mut Grid, dart: Dart) {
    let index = darts.len();
    darts.push(dart);
    active.push(index);
    grid.insert(index, dart);
}

fn blue_noise(width: usize, height: usize, radius: f32) -> Vec<Dart> {
    let mut grid = Grid::new(width, height, radius);
    let mut rng = Pcg::new(0x51ed);
    let center = Dart {
        x: 0.5 * width as f32,
        y: 0.5 * height as f32,
    };
    let mut darts = vec![center];
    let mut active = vec![0usize];
    grid.insert(0, center);

    while !active.is_empty() {
        let pick = ((rng.uniform() * active.len() as f32) as usize).min(active.len() - 1);
        let anchor = darts[active[pick]];
        let placed = (0..ATTEMPTS)
            .find_map(|_| propose(&mut rng, anchor, &grid, &darts, width as f32, height as f32));

        match placed {
            Some(dart) => spawn_dart(&mut darts, &mut active, &mut grid, dart),
            None => {
                active.swap_remove(pick);
            }
        }
    }

    return darts;
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

struct Mesh {
    nodes: Vec<u8>,
    facets: Vec<u8>,
    count: u32,
}

fn build_mesh(width: u32, height: u32, samples: usize) -> Mesh {
    let radius = (PACKING * (width * height) as f32 / samples as f32).sqrt();
    let mut darts = blue_noise(width as usize, height as usize, radius);
    darts.truncate(2 * MAX_SAMPLES);
    let anchors = frame_points(&darts, width as f32, height as f32, radius);
    let mesh = delaunator::triangulate(&anchors);
    let radii = kernel_radii(&anchors, &mesh.triangles);

    return Mesh {
        nodes: node_bytes(&anchors, &radii, width as f32, height as f32),
        facets: mesh
            .triangles
            .iter()
            .flat_map(|i| (*i as u32).to_le_bytes())
            .collect(),
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
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    groups: [wgpu::BindGroup; 2],
    textures: [wgpu::Texture; 2],
    target: wgpu::TextureView,
    params: wgpu::Buffer,
    positions: wgpu::Buffer,
    indices: wgpu::Buffer,
    scratch: HtmlCanvasElement,
    count: Cell<u32>,
    samples: Cell<usize>,
    width: u32,
    height: u32,
    origin: f32,
    current: Cell<usize>,
    index: Cell<usize>,
    switched: Cell<f32>,
    loading: Cell<bool>,
    wrapper: CssStyleDeclaration,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
}

fn upload_mesh(backdrop: &Backdrop, samples: usize) {
    let mesh = build_mesh(backdrop.width, backdrop.height, samples);
    backdrop
        .queue
        .write_buffer(&backdrop.positions, 0, &mesh.nodes);
    backdrop
        .queue
        .write_buffer(&backdrop.indices, 0, &mesh.facets);
    backdrop.count.set(mesh.count);
    backdrop.samples.set(samples);
}

fn make_target(device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> wgpu::TextureView {
    let descriptor = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    return device
        .create_texture(&descriptor)
        .create_view(&Default::default());
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
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    };
    return device.create_texture(&descriptor);
}

fn upload(backdrop: &Backdrop, slot: usize, pixels: &[u8]) {
    let texture = &backdrop.textures[slot];
    let destination = wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    };
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4 * backdrop.width),
        rows_per_image: Some(backdrop.height),
    };
    let size = wgpu::Extent3d {
        width: backdrop.width,
        height: backdrop.height,
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
        multisample: wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
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
) -> Backdrop {
    let width = canvas.width();
    let height = canvas.height();

    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .unwrap();
    let options = wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    };
    let adapter = instance.request_adapter(&options).await.unwrap();
    let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
    crate::watch(&device, "backdrop");

    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats[0];
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        color_space: wgpu::SurfaceColorSpace::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

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
        target: make_target(&device, format, width, height),
        scratch: scratch_canvas(width, height),
        count: Cell::new(0),
        samples: Cell::new(0),
        pipeline,
        textures,
        groups,
        surface,
        device,
        queue,
        params,
        positions,
        indices,
        width,
        height,
        origin: now(),
        current: Cell::new(0),
        index: Cell::new(0),
        switched: Cell::new(f32::MIN),
        loading: Cell::new(false),
        wrapper,
        tint,
        exposure,
        resolution,
        caption,
    };
    upload_mesh(&backdrop, resolution.get_untracked());

    return backdrop;
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
    return ((time - backdrop.switched.get()) / FADE).clamp(0.0, 1.0);
}

fn draw(backdrop: &Backdrop, time: f32) {
    let wgpu::CurrentSurfaceTexture::Success(frame) = backdrop.surface.get_current_texture() else {
        return;
    };

    let floats = [
        backdrop.width as f32,
        backdrop.height as f32,
        time,
        blend_at(backdrop, time),
        backdrop.tint.get_untracked(),
        backdrop.exposure.get_untracked(),
    ];
    let bytes: Vec<u8> = floats.iter().flat_map(|v| v.to_le_bytes()).collect();
    backdrop.queue.write_buffer(&backdrop.params, 0, &bytes);

    let view = frame.texture.create_view(&Default::default());
    let mut encoder = backdrop.device.create_command_encoder(&Default::default());
    let attachments = [Some(wgpu::RenderPassColorAttachment {
        view: &backdrop.target,
        depth_slice: None,
        resolve_target: Some(&view),
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
    pass.set_bind_group(0, &backdrop.groups[backdrop.current.get()], &[]);
    pass.draw(0..backdrop.count.get(), 0..1);
    drop(pass);

    backdrop.queue.submit([encoder.finish()]);
    backdrop.queue.present(frame);
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

        let pixels = cover_pixels(&backdrop.scratch, &image);
        let slot = 1 - backdrop.current.get();
        upload(&backdrop, slot, &pixels);

        if backdrop.switched.get() == f32::MIN {
            upload(&backdrop, 1 - slot, &pixels);
        }

        backdrop.caption.set(index);
        backdrop.current.set(slot);
        backdrop.switched.set(elapsed(&backdrop));
        backdrop.loading.set(false);
        backdrop
            .wrapper
            .set_property("opacity", "var(--backdrop-opacity)")
            .unwrap();
    });
}

fn now() -> f32 {
    let performance = web_sys::window().unwrap().performance().unwrap();
    return (performance.now() / 1000.0) as f32;
}

fn elapsed(backdrop: &Backdrop) -> f32 {
    return now() - backdrop.origin;
}

fn tick(backdrop: Rc<Backdrop>) {
    let time = elapsed(&backdrop);
    if !backdrop.loading.get() && time - backdrop.switched.get() >= HOLD {
        advance(backdrop.clone());
    }

    let wanted = backdrop.resolution.get_untracked();
    if wanted != backdrop.samples.get() {
        upload_mesh(&backdrop, wanted);
    }

    draw(&backdrop, time);
    request_animation_frame(move || tick(backdrop));
}

async fn run(
    canvas: HtmlCanvasElement,
    wrapper: CssStyleDeclaration,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
) {
    tick(Rc::new(init(canvas, wrapper, tint, exposure, resolution, caption).await));
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
        spawn_local(run(target, element.style(), tint, exposure, resolution, caption));
    });

    return view! {
        <div class="backdrop" node_ref=wrapper>
            <canvas node_ref=canvas_ref></canvas>
        </div>
    };
}
