mod backdrop;
mod games;
mod paper;
mod pfp;
mod pubs;
mod rng;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use games::Games;
use backdrop::{mark, millis, Backdrop, Stats, HISTORY, HOLD, LOCATIONS, MAX_SAMPLES, SAMPLES};
use paper::Paper;
use pubs::{About, Awards, Blogs, OtherWorks, Experience, Publications};

pub fn watch(device: &wgpu::Device, name: &'static str) -> RwSignal<bool> {
    let lost = RwSignal::new(false);
    device.on_uncaptured_error(std::sync::Arc::new(move |error: wgpu::Error| {
        leptos::logging::error!("{name} device error: {error}");
    }));
    device.set_device_lost_callback(move |reason, message| {
        leptos::logging::error!("{name} device lost ({reason:?}): {message}");
        lost.set(true);
    });
    return lost;
}

pub fn hidden() -> bool {
    let document = web_sys::window().unwrap().document().unwrap();
    return document.hidden();
}

#[derive(Clone)]
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub async fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&Default::default()).await.ok()?;
    let (device, queue) = adapter.request_device(&Default::default()).await.ok()?;
    return Some(Gpu { instance, adapter, device, queue });
}

pub fn disabled(name: &str) -> bool {
    let search = web_sys::window().unwrap().location().search();
    return search.unwrap_or_default().contains(name);
}

const TINT: f32 = 0.5;
const EXPOSURE: f32 = 1.0;
const OPACITY: f32 = 0.18;

fn scale(element: &web_sys::HtmlElement) -> f32 {
    let style = web_sys::window().unwrap().get_computed_style(element).ok().flatten();
    let matrix = style.and_then(|style| style.get_property_value("transform").ok());
    let Some(matrix) = matrix else { return 1.0 };
    let Some(first) = matrix.trim_start_matches("matrix(").split(',').next() else {
        return 1.0;
    };
    return first.trim().parse().unwrap_or(1.0);
}

fn option(
    name: &'static str,
    frac: impl Fn() -> f32 + Send + 'static,
    value: impl Fn() -> String + Send + 'static,
    reset: impl Fn() + 'static,
    input: impl IntoView,
) -> impl IntoView {
    return view! {
        <label class="opt" style=move || format!("--v: {:.4}", frac())>
            <span class="name">{name}</span>
            <span class="value">{value}</span>
            <button class="reset" type="button" title="Reset" on:click=move |_| reset()>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"
                     stroke-linecap="round" stroke-linejoin="round">
                    <path d="M3 12a9 9 0 1 0 3-6.7L3 8"/>
                    <path d="M3 3v5h5"/>
                </svg>
            </button>
            {input}
        </label>
    };
}

fn control(title: &'static str, body: impl IntoView, action: impl Fn() + 'static) -> impl IntoView {
    return view! {
        <button class="player-button" type="button" title=title on:click=move |_| action()>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                 stroke-linecap="round" stroke-linejoin="round">
                {body}
            </svg>
        </button>
    };
}

#[component]
fn Player(
    route: RwSignal<String>,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
    caption: RwSignal<usize>,
    jump: RwSignal<i32>,
) -> impl IntoView {
    let track = NodeRef::<leptos::html::Span>::new();
    let text = NodeRef::<leptos::html::Span>::new();
    let fill = NodeRef::<leptos::html::Span>::new();
    let scrolling = RwSignal::new(false);
    let open = RwSignal::new(false);
    // leptos patches the node in place, so the fill animation needs a manual restart
    Effect::new(move || {
        caption.get();
        let Some(fill) = fill.get() else { return };
        let fill: web_sys::HtmlElement = fill.into();
        let _ = fill.style().set_property("--start", &format!("{:.4}", scale(&fill)));
        let _ = fill.style().set_property("animation-name", "none");
        let _ = fill.offset_width();
        let _ = fill.style().remove_property("animation-name");
    });
    Effect::new(move || {
        caption.get();
        request_animation_frame(move || {
            let (Some(track), Some(text)) = (track.get_untracked(), text.get_untracked()) else {
                return;
            };
            let track: web_sys::HtmlElement = track.into();
            let text: web_sys::HtmlElement = text.into();
            scrolling.set(text.scroll_width() + 6 >= track.client_width());
        });
    });
    return view! {
        <svg class="glass-defs" aria-hidden="true">
            <filter id="glass" x="0%" y="0%" width="100%" height="100%"
                    color-interpolation-filters="sRGB">
                <feImage preserveAspectRatio="none" result="mapx" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100' preserveAspectRatio='none'%3E%3ClinearGradient id='g' x1='0' x2='1'%3E%3Cstop offset='0' stop-color='rgb(255,128,128)'/%3E%3Cstop offset='0.14' stop-color='rgb(190,128,128)'/%3E%3Cstop offset='0.5' stop-color='rgb(128,128,128)'/%3E%3Cstop offset='0.86' stop-color='rgb(66,128,128)'/%3E%3Cstop offset='1' stop-color='rgb(0,128,128)'/%3E%3C/linearGradient%3E%3Crect width='100' height='100' fill='url(%23g)'/%3E%3C/svg%3E"/>
                <feImage preserveAspectRatio="none" result="mapy" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100' preserveAspectRatio='none'%3E%3ClinearGradient id='g' y1='0' y2='1' x1='0' x2='0'%3E%3Cstop offset='0' stop-color='rgb(128,255,128)'/%3E%3Cstop offset='0.14' stop-color='rgb(128,190,128)'/%3E%3Cstop offset='0.5' stop-color='rgb(128,128,128)'/%3E%3Cstop offset='0.86' stop-color='rgb(128,66,128)'/%3E%3Cstop offset='1' stop-color='rgb(128,0,128)'/%3E%3C/linearGradient%3E%3Crect width='100' height='100' fill='url(%23g)'/%3E%3C/svg%3E"/>
                <feDisplacementMap in="SourceGraphic" in2="mapx" scale="60"
                                   xChannelSelector="R" yChannelSelector="B" result="bentx"/>
                <feDisplacementMap in="bentx" in2="mapy" scale="34"
                                   xChannelSelector="B" yChannelSelector="G" result="bent"/>
                <feGaussianBlur in="bent" stdDeviation="2"/>
            </filter>
        </svg>
        <div class="dock">
        <div class="player glass">
        <Settings open tint exposure resolution/>
        <div class="player-row">
            {move || route.get().starts_with("#/").then(|| view! {
                <a class="player-button" href="#" title="Back home">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                         stroke-linecap="round" stroke-linejoin="round">
                        <path d="M19 12H5"/>
                        <polyline points="12 19 5 12 12 5"/>
                    </svg>
                </a>
            })}
            {control("Previous picture", view! {
                <polygon points="19 20 9 12 19 4 19 20"/>
                <line x1="5" y1="19" x2="5" y2="5"/>
            }, move || jump.update(|jump| *jump -= 1))}
            <span class="player-center">
                <span class="player-track" class:scrolling=move || scrolling.get() node_ref=track>
                    <span class="reel">
                        <span class="copy" node_ref=text>{move || LOCATIONS[caption.get()]}</span>
                        {move || scrolling.get().then(|| view! {
                            <span class="copy">{move || LOCATIONS[caption.get()]}</span>
                        })}
                    </span>
                </span>
                <span class="player-progress">
                    <span class="player-fill" node_ref=fill style=format!("--hold: {HOLD}s")/>
                </span>
            </span>
            {control("Next picture", view! {
                <polygon points="5 4 15 12 5 20 5 4"/>
                <line x1="19" y1="5" x2="19" y2="19"/>
            }, move || jump.update(|jump| *jump += 1))}
            {control("Settings", view! {
                <circle cx="12" cy="12" r="3.2"/>
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            }, move || open.update(|open| *open = !*open))}
        </div>
        </div>
        {move || open.get().then(|| view! {
            <span class="settings-shade" on:click=move |_| open.set(false)/>
        })}
        </div>
    };
}

#[component]
fn Settings(
    open: RwSignal<bool>,
    tint: RwSignal<f32>,
    exposure: RwSignal<f32>,
    resolution: RwSignal<usize>,
) -> impl IntoView {
    let exponent = (MAX_SAMPLES as f32).log2();
    let opacity = RwSignal::new(OPACITY);
    Effect::new(move || {
        let root = web_sys::window().unwrap().document().unwrap().document_element().unwrap();
        let root: web_sys::HtmlElement = root.dyn_into().unwrap();
        root.style()
            .set_property("--backdrop-opacity", &format!("{:.2}", opacity.get()))
            .unwrap();
    });
    return view! {
        <div class="settings-panel" class:open=move || open.get()>
                {option(
                    "Color noise",
                    move || tint.get(),
                    move || format!("{:.2}", tint.get()),
                    move || tint.set(TINT),
                    view! {
                        <input
                            type="range" min="0" max="1" step="0.01"
                            prop:value=move || tint.get()
                            on:input:target=move |e| tint.set(e.target().value().parse().unwrap_or(0.0))
                        />
                    },
                )}
                {option(
                    "Opacity",
                    move || (opacity.get() - 0.05) / 0.55,
                    move || format!("{:.2}", opacity.get()),
                    move || opacity.set(OPACITY),
                    view! {
                        <input
                            type="range" min="0.05" max="0.6" step="0.01"
                            prop:value=move || opacity.get()
                            on:input:target=move |e| opacity.set(e.target().value().parse().unwrap_or(OPACITY))
                        />
                    },
                )}
                {option(
                    "Exposure",
                    move || (exposure.get() - 0.3) / 1.7,
                    move || format!("{:.2}", exposure.get()),
                    move || exposure.set(EXPOSURE),
                    view! {
                        <input
                            type="range" min="0.3" max="2" step="0.01"
                            prop:value=move || exposure.get()
                            on:input:target=move |e| exposure.set(e.target().value().parse().unwrap_or(1.0))
                        />
                    },
                )}
                {option(
                    "Tessellation",
                    move || ((resolution.get() as f32).log2() - 10.0) / (exponent - 10.0),
                    move || resolution.get().to_string(),
                    move || resolution.set(SAMPLES),
                    view! {
                        <input
                            type="range" min="10" max=exponent.to_string() step="0.25"
                            prop:value=move || (resolution.get() as f32).log2()
                            on:change:target=move |e| {
                                let v: f32 = e.target().value().parse().unwrap_or(16.0);
                                resolution.set(v.exp2() as usize);
                            }
                        />
                    },
                )}
        </div>
    };
}

#[component]
fn Readout(stats: RwSignal<Stats>) -> impl IntoView {
    fn row(
        label: &'static str,
        value: impl Fn() -> String + Send + Sync + 'static,
        warn: impl Fn() -> bool + Send + Sync + 'static,
    ) -> impl IntoView {
        return view! {
            <div class="readout-row">
                <span>{label}</span>
                <b class:is-warn=move || warn()>{move || value()}</b>
            </div>
        };
    }

    let bars = move || {
        let stats = stats.get();
        let peak = stats.history.iter().cloned().fold(20.0_f32, f32::max);
        return (0..HISTORY)
            .map(|index| {
                let value = stats.history[index];
                let height = format!("height: {:.1}%", (value / peak * 100.0).clamp(1.5, 100.0));
                let class = match value {
                    v if v > 33.4 => "bar is-bad",
                    v if v > 18.0 => "bar is-warn",
                    _ => "bar",
                };
                view! { <i class=class style=height></i> }
            })
            .collect_view();
    };

    let scale = move || {
        let stats = stats.get();
        let peak = stats.history.iter().cloned().fold(20.0_f32, f32::max);
        return format!("{:.0} ms", peak);
    };

    let budget = move || {
        let stats = stats.get();
        let peak = stats.history.iter().cloned().fold(20.0_f32, f32::max);
        return format!("bottom: {:.1}%", (16.7 / peak * 100.0).clamp(0.0, 100.0));
    };

    return view! {
        <div class="readout">
            <div class="readout-graph">
                <span class="readout-scale">{scale}</span>
                <span class="readout-budget" style=budget></span>
                {bars}
            </div>
            <div class="readout-note">"frame time · last 192 frames · line = 16.7 ms"</div>
            {row("fps", move || format!("{:.0}", stats.get().fps), || false)}
            {row(
                "drawn/s",
                move || format!("{:.1}", stats.get().drawn),
                move || stats.get().drawn > 20.0,
            )}
            {row(
                "frame avg",
                move || format!("{:.2} ms", stats.get().frame),
                move || stats.get().frame > 18.0,
            )}
            {row(
                "frame worst",
                move || format!("{:.1} ms", stats.get().worst),
                move || stats.get().worst > 33.0,
            )}
            {row(
                "draw cpu",
                move || format!("{:.2} ms", stats.get().draw_ms),
                move || stats.get().draw_ms > 4.0,
            )}
            {row(
                "gpu init",
                move || format!("{:.0} ms", stats.get().gpu_ms),
                move || stats.get().gpu_ms > 200.0,
            )}
            {row(
                "surface cfg",
                move || format!("{:.0} ms", stats.get().surface_ms),
                move || stats.get().surface_ms > 100.0,
            )}
            {row(
                "boot total",
                move || format!("{:.0} ms", stats.get().boot_ms),
                move || stats.get().boot_ms > 800.0,
            )}
            {row(
                "mesh build",
                move || format!("{:.1} ms", stats.get().mesh_ms),
                move || stats.get().mesh_ms > 50.0,
            )}
            {row(
                "  sampler",
                move || format!("{:.0} ms", stats.get().mesh_sampler),
                move || stats.get().mesh_sampler > 50.0,
            )}
            {row(
                "  delaunay",
                move || format!("{:.0} ms", stats.get().mesh_triangulate),
                move || stats.get().mesh_triangulate > 50.0,
            )}
            {row(
                "  frame pts",
                move || format!("{:.0} ms", stats.get().mesh_frame),
                move || stats.get().mesh_frame > 30.0,
            )}
            {row(
                "  radii+pack",
                move || format!("{:.0} ms", stats.get().mesh_pack),
                move || stats.get().mesh_pack > 30.0,
            )}
            {row("  points", move || stats.get().mesh_points.to_string(), || false)}
            {row(
                "image swap",
                move || {
                    let stats = stats.get();
                    format!("{:.0} ms / {:.1} mb", stats.swap_ms, stats.upload_mb)
                },
                move || stats.get().swap_ms > 50.0,
            )}
            {row("swaps", move || stats.get().swaps.to_string(), || false)}
            {row(
                "wasm heap",
                move || {
                    let stats = stats.get();
                    format!("{:.1} / {:.1} mb", stats.heap_mb, stats.peak_mb)
                },
                move || stats.get().heap_mb > 96.0,
            )}
            {row(
                "wasm ready",
                move || format!("{:.0} ms", stats.get().boot.wasm),
                move || stats.get().boot.wasm > 600.0,
            )}
            {row(
                "mounted",
                move || format!("{:.0} ms", stats.get().boot.mount),
                || false,
            )}
            {row(
                "first frame",
                move || format!("{:.0} ms", stats.get().boot.first_frame),
                || false,
            )}
            {row(
                "bg visible",
                move || format!("{:.0} ms", stats.get().boot.visible),
                move || stats.get().boot.visible > 1500.0,
            )}
            {row(
                "uptime",
                move || {
                    let seconds = stats.get().uptime;
                    format!("{:.0}m {:02.0}s", seconds / 60.0, seconds % 60.0)
                },
                || false,
            )}
            {row("samples", move || stats.get().samples.to_string(), || false)}
            {row("triangles", move || stats.get().triangles.to_string(), || false)}
            {row(
                "canvas",
                move || {
                    let stats = stats.get();
                    format!("{}x{}", stats.width, stats.height)
                },
                || false,
            )}
        </div>
    };
}

#[component]
fn Home() -> impl IntoView {
    return view! {
        <main class="content">
            <section><About/></section>
            <section><Publications/></section>
            <section><Experience/></section>
            <section><Awards/></section>
            <section><OtherWorks/></section>
            <section><Blogs/></section>
            <section class="games-section"><Games/></section>
        </main>
    };
}

fn hash() -> String {
    return web_sys::window().unwrap().location().hash().unwrap_or_default();
}

#[component]
fn App() -> impl IntoView {
    let route = RwSignal::new(hash());
    window_event_listener(leptos::ev::hashchange, move |_| {
        route.set(hash());
        web_sys::window().unwrap().scroll_to_with_x_and_y(0.0, 0.0);
    });
    let tint = RwSignal::new(TINT);
    let exposure = RwSignal::new(EXPOSURE);
    let resolution = RwSignal::new(SAMPLES);
    let caption = RwSignal::new(0);
    let jump = RwSignal::new(0);
    let stats = RwSignal::new(Stats::default());
    return view! {
        {(!disabled("nobg")).then(|| view! { <Backdrop tint exposure resolution caption jump stats/> })}
        {disabled("stats").then(|| view! { <Readout stats/> })}
        <Player route tint exposure resolution caption jump/>
        {move || match route.get().strip_prefix("#/") {
            Some(slug) => view! { <Paper slug=slug.to_string()/> }.into_any(),
            None => view! { <Home/> }.into_any(),
        }}
    };
}

fn shed() {
    let document = web_sys::window().unwrap().document().unwrap();
    let Some(shell) = document.get_element_by_id("shell") else {
        return;
    };
    shell.remove();
}

fn main() {
    console_error_panic_hook::set_once();
    let started = millis();
    mark(move |boot| boot.wasm = started);
    leptos::mount::mount_to_body(App);
    let mounted = millis();
    mark(move |boot| boot.mount = mounted);
    shed();
}
