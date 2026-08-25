use std::time::Duration;

use leptos::prelude::*;

const PAPERS: &str = include_str!("../papers.toml");

#[derive(Clone, serde::Deserialize)]
struct Author {
    name: String,
    url: Option<String>,
}

#[derive(Clone, serde::Deserialize)]
struct PaperData {
    slug: String,
    title: String,
    venue: String,
    icon: String,
    authors: Vec<Author>,
    teaser: String,
    pdf: Option<String>,
    code: Option<String>,
    slides: Option<String>,
    supplementary: Option<String>,
    tldr: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Vec<String>,
    bibtex: String,
}

#[derive(serde::Deserialize)]
struct PaperList {
    paper: Vec<PaperData>,
}

fn authors(list: Vec<Author>) -> impl IntoView {
    return list
        .into_iter()
        .enumerate()
        .map(|(index, author)| {
            let body = match (author.name == crate::pubs::ME, author.url) {
                (true, _) => view! { <b>{author.name}</b> }.into_any(),
                (false, Some(url)) => view! { <a href=url>{author.name}</a> }.into_any(),
                (false, None) => author.name.into_any(),
            };
            return view! { {(index > 0).then(|| ", ")}{body} };
        })
        .collect_view();
}

fn action(url: String, label: &'static str, icon: AnyView) -> impl IntoView {
    return view! {
        <a class="paper-action" href=url target="_blank" rel="noopener">{icon}{label}</a>
    };
}

fn pdf_icon() -> AnyView {
    return view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
             stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
        </svg>
    }
    .into_any();
}

fn code_icon() -> AnyView {
    return view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
             stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
        </svg>
    }
    .into_any();
}

fn slides_icon() -> AnyView {
    return view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
             stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="13" rx="2"/>
            <line x1="12" y1="16" x2="12" y2="20"/>
            <line x1="8" y1="21" x2="16" y2="21"/>
        </svg>
    }
    .into_any();
}

fn cite_icon() -> AnyView {
    return view! {
        <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M5 16.5c0-4.5 2.5-8 6-9.5l.8 1.4c-2 1.1-3.3 2.8-3.6 4.6.3-.1.6-.2 1-.2 1.7 0 3 1.3 3 3.1s-1.4 3.1-3.1 3.1c-2.4 0-4.1-1.9-4.1-2.5zm9 0c0-4.5 2.5-8 6-9.5l.8 1.4c-2 1.1-3.3 2.8-3.6 4.6.3-.1.6-.2 1-.2 1.7 0 3 1.3 3 3.1s-1.4 3.1-3.1 3.1c-2.4 0-4.1-1.9-4.1-2.5z"/>
        </svg>
    }
    .into_any();
}

fn bibtex_button(bibtex: String) -> impl IntoView {
    let copied = RwSignal::new(false);
    let copy = move |_| {
        let clipboard = web_sys::window().unwrap().navigator().clipboard();
        let _ = clipboard.write_text(bibtex.trim());
        copied.set(true);
        set_timeout(move || copied.set(false), Duration::from_millis(1600));
    };
    return view! {
        <button type="button" class="paper-action" class:is-copied=move || copied.get() on:click=copy>
            {cite_icon()}
            {move || if copied.get() { "Copied!" } else { "BibTeX" }}
        </button>
    };
}

fn page(p: PaperData) -> impl IntoView {
    let title = p.title.clone();
    return view! {
        <main class="content">
            <header>
                <h1 class="paper-title">{p.title}</h1>
                <div class="paper-venue">
                    <img class="logo" src=p.icon alt=""/>
                    {p.venue}
                </div>
                <div class="paper-authors">{authors(p.authors)}</div>
            </header>
            <figure class="paper-teaser">
                <img src=p.teaser alt=title/>
            </figure>
            <div class="paper-actions">
                {p.pdf.map(|url| action(url, "Paper", pdf_icon()))}
                {p.code.map(|url| action(url, "Code", code_icon()))}
                {p.slides.map(|url| action(url, "Slides", slides_icon()))}
                {p.supplementary.map(|url| action(url, "Supplementary", pdf_icon()))}
                {bibtex_button(p.bibtex)}
            </div>

            {p.tldr.map(|tldr| view! {
                <section class="paper-section">
                    <h2>"TL;DR"</h2>
                    <p class="summary">{tldr}</p>
                </section>
            })}
            <section class="paper-section">
                <h2>"Abstract"</h2>
                {p.abstract_text.into_iter().map(|text| view! { <p>{text}</p> }).collect_view()}
            </section>
        </main>
    };
}

#[component]
pub fn Paper(slug: String) -> impl IntoView {
    let list: PaperList = toml::from_str(PAPERS).unwrap();
    let paper = list.paper.into_iter().find(|p| p.slug == slug);
    return match paper {
        Some(p) => page(p).into_any(),
        None => view! { <main class="content"><p>"Page not found. "<a href="#">"Back home"</a></p></main> }.into_any(),
    };
}
