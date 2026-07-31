use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

use crate::rng::Pcg;

const SHADER: &str = include_str!("../gen/splatfit.wgsl");

const SIZE: u32 = 128;
const SPLATS: usize = 1 << 12;
const SPLAT_STRIDE: usize = 10;
const ITERATIONS: u32 = 1 << 16;
const SIGMA: (f32, f32) = (1.0, 4.0);
const GLINT: f32 = 0.85;

struct Fit {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    gather: wgpu::ComputePipeline,
    backward: wgpu::ComputePipeline,
    params_buffer: wgpu::Buffer,
    moments_buffer: wgpu::Buffer,
    blended_buffer: wgpu::Buffer,
    step_buffer: wgpu::Buffer,
    shine_buffer: wgpu::Buffer,
    backward_group: wgpu::BindGroup,
    iteration: Cell<u32>,
    resets_applied: Cell<u32>,
    glint: Cell<f32>,
    hovered: RwSignal<bool>,
}

fn now() -> f32 {
    let performance = web_sys::window().unwrap().performance().unwrap();
    return (performance.now() / 1000.0) as f32;
}

fn to_bytes(values: &[f32]) -> Vec<u8> {
    return values.iter().flat_map(|v| v.to_le_bytes()).collect();
}

async fn load_target() -> Option<Vec<f32>> {
    let image = HtmlImageElement::new().unwrap();
    image.set_src("face.jpg");

    if JsFuture::from(image.decode()).await.is_err() {
        return None;
    }

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_width(SIZE);
    canvas.set_height(SIZE);
    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();
    let size = SIZE as f64;
    context
        .draw_image_with_html_image_element_and_dw_and_dh(&image, 0.0, 0.0, size, size)
        .unwrap();

    let data = context.get_image_data(0.0, 0.0, size, size).unwrap();
    let raw = data.data().0;
    let pixels = (SIZE * SIZE) as usize;
    let mut target = Vec::with_capacity(pixels * 4);

    for i in 0..pixels {
        target.push(raw[4 * i] as f32 / 255.0);
        target.push(raw[4 * i + 1] as f32 / 255.0);
        target.push(raw[4 * i + 2] as f32 / 255.0);
        target.push(0.0);
    }

    return Some(target);
}

fn random_params(rng: &mut Pcg) -> Vec<f32> {
    let mut params = Vec::with_capacity(SPLATS * SPLAT_STRIDE);

    for _ in 0..SPLATS {
        params.extend_from_slice(&[
            rng.range(0.0, SIZE as f32),
            rng.range(0.0, SIZE as f32),
            rng.range(SIGMA.0, SIGMA.1).ln(),
            rng.range(SIGMA.0, SIGMA.1).ln(),
            rng.range(0.0, std::f32::consts::PI),
            rng.range(0.02, 0.15),
            rng.range(0.0, std::f32::consts::TAU),
            rng.range(-0.25, 0.25),
            rng.range(-0.25, 0.25),
            rng.range(-0.25, 0.25),
        ]);
    }

    return params;
}

fn storage_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    let descriptor = wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    };
    return device.create_buffer(&descriptor);
}

fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    return wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    };
}

fn make_pipeline(device: &wgpu::Device, shader: &wgpu::ShaderModule, entry: &str) -> wgpu::ComputePipeline {
    let descriptor = wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: shader,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    };
    return device.create_compute_pipeline(&descriptor);
}

async fn init_fit(canvas: HtmlCanvasElement, target: &[f32], hovered: RwSignal<bool>) -> Fit {
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
    crate::watch(&device, "pfp");

    let caps = surface.get_capabilities(&adapter);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: SIZE,
        height: SIZE,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        color_space: wgpu::SurfaceColorSpace::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let state_size = (SPLATS * SPLAT_STRIDE * 4) as u64;
    let pixels = (SIZE * SIZE) as u64;
    let params_buffer = storage_buffer(&device, state_size);
    let moments_buffer = storage_buffer(&device, 2 * state_size);
    let blended_buffer = storage_buffer(&device, pixels * 16);
    let target_buffer = storage_buffer(&device, pixels * 16);
    let step_descriptor = wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    };
    let step_buffer = device.create_buffer(&step_descriptor);
    let shine_buffer = device.create_buffer(&step_descriptor);

    let mut rng = Pcg::new(0x5eed);
    queue.write_buffer(&params_buffer, 0, &to_bytes(&random_params(&mut rng)));
    queue.write_buffer(&target_buffer, 0, &to_bytes(target));

    let shader_descriptor = wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    };
    let shader = device.create_shader_module(shader_descriptor);
    let gather = make_pipeline(&device, &shader, "gather");
    let backward = make_pipeline(&device, &shader, "backward");

    let backward_entries = [
        buffer_entry(1, &params_buffer),
        buffer_entry(2, &moments_buffer),
        buffer_entry(3, &blended_buffer),
        buffer_entry(4, &target_buffer),
        buffer_entry(6, &step_buffer),
    ];
    let backward_descriptor = wgpu::BindGroupDescriptor {
        label: None,
        layout: &backward.get_bind_group_layout(0),
        entries: &backward_entries,
    };
    let backward_group = device.create_bind_group(&backward_descriptor);

    return Fit {
        surface,
        device,
        queue,
        gather,
        backward,
        params_buffer,
        moments_buffer,
        blended_buffer,
        step_buffer,
        shine_buffer,
        backward_group,
        iteration: Cell::new(0),
        resets_applied: Cell::new(0),
        glint: Cell::new(0.0),
        hovered,
    };
}

fn reset_fit(fit: &Fit, generation: u32) {
    let mut rng = Pcg::new(0x5eed ^ generation.wrapping_mul(0x9e3779b9));
    let state_size = SPLATS * SPLAT_STRIDE * 4;
    fit.queue
        .write_buffer(&fit.params_buffer, 0, &to_bytes(&random_params(&mut rng)));
    fit.queue
        .write_buffer(&fit.moments_buffer, 0, &vec![0u8; 2 * state_size]);
    fit.iteration.set(0);
}

fn fit_step(fit: &Fit, optimize: bool) {
    let wgpu::CurrentSurfaceTexture::Success(surface_texture) = fit.surface.get_current_texture()
    else {
        return;
    };
    let step = fit.iteration.get() + 1;
    fit.queue
        .write_buffer(&fit.step_buffer, 0, &step.to_le_bytes());

    let wanted = if fit.hovered.get_untracked() { 1.0 } else { 0.0 };
    let glint = fit.glint.get() + 0.09 * (wanted - fit.glint.get());
    fit.glint.set(glint);
    let shine = [(0.016 * now()).fract(), GLINT * glint];
    fit.queue.write_buffer(&fit.shine_buffer, 0, &to_bytes(&shine));

    let view = surface_texture.texture.create_view(&Default::default());
    let gather_entries = [
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        },
        buffer_entry(1, &fit.params_buffer),
        buffer_entry(3, &fit.blended_buffer),
        buffer_entry(5, &fit.shine_buffer),
    ];
    let gather_descriptor = wgpu::BindGroupDescriptor {
        label: None,
        layout: &fit.gather.get_bind_group_layout(0),
        entries: &gather_entries,
    };
    let gather_group = fit.device.create_bind_group(&gather_descriptor);

    let mut encoder = fit.device.create_command_encoder(&Default::default());

    let mut pass = encoder.begin_compute_pass(&Default::default());
    pass.set_pipeline(&fit.gather);
    pass.set_bind_group(0, &gather_group, &[]);
    pass.dispatch_workgroups(SIZE.div_ceil(8), SIZE.div_ceil(8), 1);
    drop(pass);

    if optimize {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&fit.backward);
        pass.set_bind_group(0, &fit.backward_group, &[]);
        pass.dispatch_workgroups((SPLATS as u32).div_ceil(64), 1, 1);
        drop(pass);
        fit.iteration.set(step);
    }

    fit.queue.submit([encoder.finish()]);
    fit.queue.present(surface_texture);
}

fn tick(fit: Rc<Fit>, resets: RwSignal<u32>) {
    let requested = resets.get_untracked();
    if requested != fit.resets_applied.get() {
        fit.resets_applied.set(requested);
        reset_fit(&fit, requested);
    }

    let optimize = fit.iteration.get() < ITERATIONS;
    if optimize || fit.glint.get() > 1e-3 || fit.hovered.get_untracked() {
        fit_step(&fit, optimize);
    }

    request_animation_frame(move || tick(fit, resets));
}

async fn run(canvas: HtmlCanvasElement, resets: RwSignal<u32>, hovered: RwSignal<bool>) {
    let Some(target) = load_target().await else {
        return;
    };
    let fit = init_fit(canvas, &target, hovered).await;
    tick(Rc::new(fit), resets);
}

#[component]
pub fn Pfp() -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let resets = RwSignal::new(0u32);
    let hovered = RwSignal::new(false);
    Effect::new(move || {
        if let Some(canvas) = canvas_ref.get() {
            spawn_local(run(canvas.into(), resets, hovered));
        }
    });

    return view! {
        <span
            class="pfp-wrap"
            on:click=move |_| resets.update(|r| *r += 1)
            on:mouseenter=move |_| hovered.set(true)
            on:mouseleave=move |_| hovered.set(false)
        >
            <canvas
                class="pfp"
                node_ref=canvas_ref
                width=SIZE.to_string()
                height=SIZE.to_string()
            ></canvas>
            <span class="pfp-hint">{format!("↻ {SPLATS} gabors")}</span>
        </span>
    };
}
