use std::fmt::Write;

const ABOUT: &str = include_str!("../../about.toml");
const PUBLICATIONS: &str = include_str!("../../publications.toml");
const CV: &str = include_str!("../../cv.toml");
const ME: &str = "Venkataram Sivaram";

#[derive(serde::Deserialize)]
struct AboutText {
    paragraphs: Vec<String>,
}

#[derive(serde::Deserialize)]
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

#[derive(serde::Deserialize)]
struct CvEntry {
    period: String,
    icon: String,
    title: String,
    summary: String,
    url: Option<String>,
}

#[derive(serde::Deserialize)]
struct Article {
    title: String,
    venue: String,
    year: String,
    url: String,
    summary: String,
    links: Option<Vec<Link>>,
}

#[derive(serde::Deserialize)]
struct Link {
    label: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct Cv {
    work: Vec<CvEntry>,
    award: Vec<CvEntry>,
    other: Vec<Article>,
}

fn escape(text: &str) -> String {
    return text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
}

fn rich(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let Some(open) = rest.find('[') else { break };
        let Some(split) = rest[open..].find("](").map(|i| open + i) else { break };
        let Some(end) = rest[split..].find(')').map(|i| split + i) else { break };
        let label = &rest[open + 1..split];
        let url = &rest[split + 2..end];
        out.push_str(&escape(&rest[..open]));
        let _ = write!(out, "<a href=\"{}\">{}</a>", escape(url), escape(label));
        rest = &rest[end + 1..];
    }
    out.push_str(&escape(rest));
    return out;
}

fn authors(list: &str) -> String {
    return list
        .split(", ")
        .map(|name| match name == ME {
            true => format!("<b>{}</b>", escape(name)),
            false => escape(name),
        })
        .collect::<Vec<_>>()
        .join(", ");
}

fn entries(out: &mut String, list: &[CvEntry]) {
    for entry in list {
        let title = match &entry.url {
            Some(url) => format!("<a href=\"{}\">{}</a>", escape(url), escape(&entry.title)),
            None => escape(&entry.title),
        };
        let _ = write!(
            out,
            "<div class=\"pub\"><div class=\"pub-head\"><img class=\"logo\" src=\"{}\" alt=\"\"><h3>{}</h3><span class=\"pub-year\">{}</span></div><p class=\"summary\">{}</p></div>",
            escape(&entry.icon),
            title,
            escape(&entry.period),
            rich(&entry.summary)
        );
    }
}

fn main() {
    let about: AboutText = toml::from_str(ABOUT).unwrap();
    let publications: PublicationList = toml::from_str(PUBLICATIONS).unwrap();
    let cv: Cv = toml::from_str(CV).unwrap();

    let mut out = String::new();
    out.push_str("<main class=\"content\" id=\"shell\">");

    out.push_str("<section><h2>About</h2>");
    out.push_str("<span class=\"pfp-wrap\"><span class=\"pfp pfp-preview\"></span></span>");
    for text in &about.paragraphs {
        let _ = write!(out, "<p>{}</p>", rich(text));
    }
    out.push_str(include_str!("contacts.html"));
    out.push_str("</section>");

    out.push_str("<section><h2>Publications</h2>");
    for p in &publications.publication {
        let _ = write!(
            out,
            "<div class=\"pub\"><div class=\"pub-head\"><img class=\"logo\" src=\"{}\" alt=\"\"><h3><a href=\"{}\">{}</a></h3><span class=\"pub-year\">{}</span></div><div class=\"pub-venue\">{}</div><p>{}</p><p class=\"summary\">{}</p></div>",
            escape(&p.icon),
            escape(&p.url),
            escape(&p.title),
            escape(&p.year),
            escape(&p.venue),
            authors(&p.authors),
            rich(&p.summary)
        );
    }
    out.push_str("</section>");

    out.push_str("<section><h2>Experience</h2>");
    entries(&mut out, &cv.work);
    out.push_str("</section>");

    out.push_str("<section><h2>Awards</h2>");
    entries(&mut out, &cv.award);
    out.push_str("</section>");

    out.push_str("<section><h2>Other Works</h2>");
    for article in &cv.other {
        let _ = write!(
            out,
            "<div class=\"pub\"><div class=\"pub-head\"><h3><a href=\"{}\">{}</a></h3><span class=\"pub-year\">{}</span></div><div class=\"pub-venue\">{}</div><p class=\"summary\">{}</p>",
            escape(&article.url),
            escape(&article.title),
            escape(&article.year),
            escape(&article.venue),
            rich(&article.summary)
        );
        if let Some(links) = &article.links {
            out.push_str("<p class=\"pub-links\">");
            for link in links {
                let _ = write!(out, "<a href=\"{}\">{}</a>", escape(&link.url), escape(&link.label));
            }
            out.push_str("</p>");
        }
        out.push_str("</div>");
    }
    out.push_str("</section>");

    out.push_str("<section><h2>Blogs</h2><p>Coming soon!</p></section>");
    out.push_str("<section class=\"games\"><h2>Game Collection</h2></section>");
    out.push_str("</main>");

    print!("{out}");
}
