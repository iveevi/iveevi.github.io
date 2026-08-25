use leptos::prelude::*;

use crate::pfp::Pfp;

const PUBLICATIONS: &str = include_str!("../publications.toml");
const CV: &str = include_str!("../cv.toml");
const ABOUT: &str = include_str!("../about.toml");
pub const ME: &str = "Venkataram Sivaram";

#[derive(Clone, serde::Deserialize)]
struct Publication {
    title: String,
    authors: String,
    icon: String,
    venue: String,
    year: String,
    url: String,
    summary: String,
}

#[derive(serde::Deserialize)]
struct PublicationList {
    publication: Vec<Publication>,
}

#[derive(Clone, serde::Deserialize)]
struct CvEntry {
    period: String,
    icon: String,
    title: String,
    summary: String,
    url: Option<String>,
}

fn authors(list: &str) -> impl IntoView + use<> {
    return list
        .split(", ")
        .enumerate()
        .map(|(index, name)| {
            let name = name.to_string();
            let body = match name == ME {
                true => view! { <b>{name}</b> }.into_any(),
                false => name.into_any(),
            };
            return view! { {(index > 0).then(|| ", ")}{body} };
        })
        .collect_view();
}

fn rich(text: &str) -> impl IntoView + use<> {
    let mut parts: Vec<AnyView> = Vec::new();
    let mut rest = text;

    loop {
        let Some(open) = rest.find('[') else { break };
        let Some(split) = rest[open..].find("](").map(|i| open + i) else { break };
        let Some(end) = rest[split..].find(')').map(|i| split + i) else { break };

        let label = rest[open + 1..split].to_string();
        let url = rest[split + 2..end].to_string();
        parts.push(rest[..open].to_string().into_any());
        parts.push(view! { <a href=url>{label}</a> }.into_any());
        rest = &rest[end + 1..];
    }

    parts.push(rest.to_string().into_any());
    return parts;
}

fn logo(src: String) -> impl IntoView {
    return view! { <img class="logo" src=src alt=""/> };
}

#[derive(Clone, serde::Deserialize)]
struct Link {
    label: String,
    url: String,
}

#[derive(Clone, serde::Deserialize)]
struct Article {
    title: String,
    venue: String,
    year: String,
    url: String,
    summary: String,
    links: Option<Vec<Link>>,
}

#[derive(serde::Deserialize)]
struct Cv {
    work: Vec<CvEntry>,
    award: Vec<CvEntry>,
    other: Vec<Article>,
}

fn publications() -> Vec<Publication> {
    let list: PublicationList = toml::from_str(PUBLICATIONS).unwrap();
    return list.publication;
}

fn cv() -> Cv {
    return toml::from_str(CV).unwrap();
}

fn entry_title(entry: &CvEntry) -> AnyView {
    let title = entry.title.clone();
    return match entry.url.clone() {
        Some(url) => view! { <a href=url>{title}</a> }.into_any(),
        None => title.into_any(),
    };
}

fn cv_list(entries: Vec<CvEntry>) -> impl IntoView {
    return entries
        .into_iter()
        .map(|entry| {
            view! {
                <div class="pub">
                    <div class="pub-head">
                        {logo(entry.icon.clone())}
                        <h3>{entry_title(&entry)}</h3>
                        <span class="pub-year">{entry.period.clone()}</span>
                    </div>
                    <p class="summary">{rich(&entry.summary)}</p>
                </div>
            }
        })
        .collect_view();
}

#[component]
pub fn Publications() -> impl IntoView {
    let items = publications()
        .into_iter()
        .map(|p| {
            view! {
                <div class="pub">
                    <div class="pub-head">
                        {logo(p.icon)}
                        <h3><a href=p.url>{p.title}</a></h3>
                        <span class="pub-year">{p.year}</span>
                    </div>
                    <div class="pub-venue">{p.venue}</div>
                    <p>{authors(&p.authors)}</p>
                    <p class="summary">{rich(&p.summary)}</p>
                </div>
            }
        })
        .collect_view();
    return view! {
        <h2>"Publications"</h2>
        {items}
    };
}

#[component]
pub fn About() -> impl IntoView {
    #[derive(serde::Deserialize)]
    struct AboutText {
        paragraphs: Vec<String>,
    }
    let about: AboutText = toml::from_str(ABOUT).unwrap();
    let paragraphs = about
        .paragraphs
        .into_iter()
        .map(|text| view! { <p>{rich(&text)}</p> })
        .collect_view();
    return view! {
        <h2>"About"</h2>
        {(!crate::disabled("nopfp")).then(|| view! { <Pfp/> })}
        {paragraphs}
        <p class="contacts">
            <a href="mailto:iveevi@mit.edu">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                     stroke-linecap="round" stroke-linejoin="round">
                    <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
                    <polyline points="22,6 12,13 2,6"/>
                </svg>
                "iveevi@mit.edu"
            </a>
            <a href="https://github.com/iveevi">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                     stroke-linecap="round" stroke-linejoin="round">
                    <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
                </svg>
                "github.com/iveevi"
            </a>
            <a href="https://www.linkedin.com/in/venkataram-sivaram-65a36a226">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                     stroke-linecap="round" stroke-linejoin="round">
                    <path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-4 0v7h-4v-7a6 6 0 0 1 6-6z"/>
                    <rect x="2" y="9" width="4" height="12"/>
                    <circle cx="4" cy="4" r="2"/>
                </svg>
                "linkedin"
            </a>
        </p>
    };
}

#[component]
pub fn Experience() -> impl IntoView {
    return view! {
        <h2>"Experience"</h2>
        {cv_list(cv().work)}
    };
}

#[component]
pub fn Awards() -> impl IntoView {
    return view! {
        <h2>"Awards"</h2>
        {cv_list(cv().award)}
    };
}

#[component]
pub fn OtherWorks() -> impl IntoView {
    let articles = cv()
        .other
        .into_iter()
        .map(|a| {
            view! {
                <div class="pub">
                    <div class="pub-head">
                        <h3><a href=a.url>{a.title}</a></h3>
                        <span class="pub-year">{a.year}</span>
                    </div>
                    <div class="pub-venue">{a.venue}</div>
                    <p class="summary">{rich(&a.summary)}</p>
                    {a.links.map(|links| view! {
                        <p class="pub-links">
                            {links
                                .into_iter()
                                .map(|link| view! { <a href=link.url>{link.label}</a> })
                                .collect_view()}
                        </p>
                    })}
                </div>
            }
        })
        .collect_view();
    return view! {
        <h2>"Other Works"</h2>
        {articles}
    };
}

#[component]
pub fn Blogs() -> impl IntoView {
    return view! {
        <h2>"Blogs"</h2>
        <p>"Coming soon!"</p>
    };
}
