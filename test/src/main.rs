mod backdrop;
mod paper;
mod pfp;
mod pubs;
mod rng;

use leptos::prelude::*;

use backdrop::{Backdrop, LOCATIONS, MAX_SAMPLES, SAMPLES};
use paper::Paper;
use pubs::{About, Awards, Blogs, OtherWorks, Experience, Publications};

pub fn watch(device: &wgpu::Device, name: &'static str) {
    device.on_uncaptured_error(std::sync::Arc::new(move |error: wgpu::Error| {
        leptos::logging::error!("{name} device error: {error}");
    }));
    device.set_device_lost_callback(move |reason, message| {
        leptos::logging::error!("{name} device lost ({reason:?}): {message}");
    });
}

pub fn disabled(name: &str) -> bool {
    let search = web_sys::window().unwrap().location().search();
    return search.unwrap_or_default().contains(name);
}

const TINT: f32 = 0.3;
const EXPOSURE: f32 = 1.0;

fn row(name: &'static str, value: impl Fn() -> String + Send + 'static, reset: impl Fn() + 'static) -> impl IntoView {
    return view! {
        <span class="row">
            {name}
            <span class="trailing">
                <span class="value">{value}</span>
                <button class="reset" type="button" title="Reset" on:click=move |_| reset()>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"
                         stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 12a9 9 0 1 0 3-6.7L3 8"/>
                        <path d="M3 3v5h5"/>
                    </svg>
                </button>
            </span>
        </span>
    };
}

#[component]
fn Settings(tint: RwSignal<f32>, exposure: RwSignal<f32>, resolution: RwSignal<usize>) -> impl IntoView {
    let exponent = (MAX_SAMPLES as f32).log2();
    return view! {
        <details class="settings">
            <summary class="card">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"
                     stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="3.2"/>
                    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
                </svg>
            </summary>
            <div class="settings-panel card">
                <label>
                    {row("Color noise", move || format!("{:.2}", tint.get()), move || tint.set(TINT))}
                    <input
                        type="range" min="0" max="1" step="0.01"
                        prop:value=move || tint.get()
                        on:input:target=move |e| tint.set(e.target().value().parse().unwrap_or(0.0))
                    />
                </label>
                <label>
                    {row("Exposure", move || format!("{:.2}", exposure.get()), move || exposure.set(EXPOSURE))}
                    <input
                        type="range" min="0.3" max="2" step="0.01"
                        prop:value=move || exposure.get()
                        on:input:target=move |e| exposure.set(e.target().value().parse().unwrap_or(1.0))
                    />
                </label>
                <label>
                    {row("Tessellation", move || resolution.get().to_string(), move || resolution.set(SAMPLES))}
                    <input
                        type="range" min="10" max=exponent.to_string() step="0.25"
                        prop:value=move || (resolution.get() as f32).log2()
                        on:change:target=move |e| {
                            let v: f32 = e.target().value().parse().unwrap_or(16.0);
                            resolution.set(v.exp2() as usize);
                        }
                    />
                </label>
            </div>
        </details>
    };
}

#[component]
fn Home() -> impl IntoView {
    let tint = RwSignal::new(TINT);
    let exposure = RwSignal::new(EXPOSURE);
    let resolution = RwSignal::new(SAMPLES);
    let caption = RwSignal::new(0);
    return view! {
        {(!disabled("nobg")).then(|| view! { <Backdrop tint exposure resolution caption/> })}
        <Settings tint exposure resolution/>
        <p class="caption">{move || LOCATIONS[caption.get()]}</p>
        <main class="content">
            <section><About/></section>
            <section><Publications/></section>
            <section><Experience/></section>
            <section><Awards/></section>
            <section><OtherWorks/></section>
            <section><Blogs/></section>
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
    return view! {
        {move || match route.get().strip_prefix("#/") {
            Some(slug) => view! { <Paper slug=slug.to_string()/> }.into_any(),
            None => view! { <Home/> }.into_any(),
        }}
    };
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
