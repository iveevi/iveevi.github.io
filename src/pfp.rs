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
const ITERATIONS: u32 = 300;
const SIGMA: (f32, f32) = (1.0, 4.0);
const GLINT: f32 = 0.85;

struct Fit {
    device: wgpu::Device,
    queue: wgpu::Queue,
    context: CanvasRenderingContext2d,
    gather: wgpu::ComputePipeline,
    backward: wgpu::ComputePipeline,
    display: wgpu::Texture,
    params_buffer: wgpu::Buffer,
    moments_buffer: wgpu::Buffer,
    step_buffer: wgpu::Buffer,
    shine_buffer: wgpu::Buffer,
    readback: wgpu::Buffer,
    gather_group: wgpu::BindGroup,
    backward_group: wgpu::BindGroup,
    iteration: Cell<u32>,
    resets_applied: Cell<u32>,
    glint: Cell<f32>,
    busy: Cell<bool>,
    hovered: RwSignal<bool>,
    live: RwSignal<bool>,
    lost: RwSignal<bool>,
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

async fn init_fit(canvas: HtmlCanvasElement, target: &[f32], hovered: RwSignal<bool>, live: RwSignal<bool>) -> Option<Fit> {
    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    let crate::Gpu { device, queue, .. } = crate::gpu().await?;
    let lost = crate::watch(&device, "pfp");

    let state_size = (SPLATS * SPLAT_STRIDE * 4) as u64;
    let pixels = (SIZE * SIZE) as u64;
    let params_buffer = storage_buffer(&device, state_size);
    let moments_buffer = storage_buffer(&device, 2 * state_size);
    let blended_buffer = storage_buffer(&device, pixels * 16);
    let target_buffer = storage_buffer(&device, pixels * 16);
    let display = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let display_view = display.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: pixels * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
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

    let gather_entries = [
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&display_view),
        },
        buffer_entry(1, &params_buffer),
        buffer_entry(3, &blended_buffer),
        buffer_entry(5, &shine_buffer),
    ];
    let gather_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &gather.get_bind_group_layout(0),
        entries: &gather_entries,
    });

    let backward_entries = [
        buffer_entry(1, &params_buffer),
        buffer_entry(2, &moments_buffer),
        buffer_entry(3, &blended_buffer),
        buffer_entry(4, &target_buffer),
        buffer_entry(6, &step_buffer),
    ];
    let backward_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &backward.get_bind_group_layout(0),
        entries: &backward_entries,
    });

    return Some(Fit {
        device,
        queue,
        context,
        gather,
        backward,
        display,
        params_buffer,
        moments_buffer,
        step_buffer,
        shine_buffer,
        readback,
        gather_group,
        backward_group,
        iteration: Cell::new(0),
        resets_applied: Cell::new(0),
        glint: Cell::new(0.0),
        busy: Cell::new(false),
        hovered,
        live,
        lost,
    });
}

fn present(fit: &Fit) {
    let Ok(view) = fit.readback.slice(..).get_mapped_range() else {
        return;
    };
    let bytes = view.to_vec();
    drop(view);
    fit.readback.unmap();
    let image = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&bytes),
        SIZE,
        SIZE,
    )
    .unwrap();
    let _ = fit.context.put_image_data(&image, 0.0, 0.0);
    if !fit.live.get_untracked() {
        fit.live.set(true);
    }
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

fn fit_step(fit: &Rc<Fit>, optimize: bool) {
    let step = fit.iteration.get() + 1;
    fit.queue
        .write_buffer(&fit.step_buffer, 0, &step.to_le_bytes());

    let wanted = if fit.hovered.try_get_untracked().unwrap_or(false) { 1.0 } else { 0.0 };
    let glint = fit.glint.get() + 0.09 * (wanted - fit.glint.get());
    fit.glint.set(glint);
    let shine = [(0.016 * now()).fract(), GLINT * glint];
    fit.queue.write_buffer(&fit.shine_buffer, 0, &to_bytes(&shine));

    let mut encoder = fit.device.create_command_encoder(&Default::default());

    let mut pass = encoder.begin_compute_pass(&Default::default());
    pass.set_pipeline(&fit.gather);
    pass.set_bind_group(0, &fit.gather_group, &[]);
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

    let idle = !fit.busy.get();
    if idle {
        let source = fit.display.as_image_copy();
        let destination = wgpu::TexelCopyBufferInfo {
            buffer: &fit.readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * SIZE),
                rows_per_image: Some(SIZE),
            },
        };
        let extent = wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_buffer(source, destination, extent);
    }

    fit.queue.submit([encoder.finish()]);

    if idle {
        fit.busy.set(true);
        let fit = fit.clone();
        let buffer = fit.readback.clone();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            if result.is_ok() {
                present(&fit);
            }
            fit.busy.set(false);
        });
    }
}

fn tick(fit: Rc<Fit>, resets: RwSignal<u32>) {
    let Some(requested) = resets.try_get_untracked() else {
        return;
    };
    if requested != fit.resets_applied.get() {
        fit.resets_applied.set(requested);
        reset_fit(&fit, requested);
    }

    if fit.lost.get_untracked() {
        return;
    }
    if crate::hidden() {
        request_animation_frame(move || tick(fit, resets));
        return;
    }

    let hovered = fit.hovered.try_get_untracked().unwrap_or(false);
    let optimize = fit.iteration.get() < ITERATIONS;
    if optimize || fit.glint.get() > 1e-3 || hovered {
        fit_step(&fit, optimize);
    }

    request_animation_frame(move || tick(fit, resets));
}

async fn still(canvas: &HtmlCanvasElement) {
    let image = HtmlImageElement::new().unwrap();
    image.set_src("face.jpg");
    if JsFuture::from(image.decode()).await.is_err() {
        return;
    }
    let Ok(Some(context)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(context) = context.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };
    let size = SIZE as f64;
    let _ = context.draw_image_with_html_image_element_and_dw_and_dh(&image, 0.0, 0.0, size, size);
}

async fn run(canvas: HtmlCanvasElement, resets: RwSignal<u32>, hovered: RwSignal<bool>, live: RwSignal<bool>) {
    let Some(target) = load_target().await else {
        still(&canvas).await;
        live.set(true);
        return;
    };
    let Some(fit) = init_fit(canvas.clone(), &target, hovered, live).await else {
        still(&canvas).await;
        live.set(true);
        return;
    };
    tick(Rc::new(fit), resets);
}

#[component]
pub fn Pfp() -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let resets = RwSignal::new(0u32);
    let hovered = RwSignal::new(false);
    let live = RwSignal::new(false);
    Effect::new(move || {
        if let Some(canvas) = canvas_ref.get() {
            spawn_local(run(canvas.into(), resets, hovered, live));
        }
    });

    return view! {
        <span
            class="pfp-wrap"
            class:is-live=move || live.get()
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
