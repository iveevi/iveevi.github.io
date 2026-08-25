use leptos::prelude::*;

const GAMES: &str = include_str!("../games.toml");

#[derive(Clone, serde::Deserialize)]
struct Game {
    title: String,
    app: String,
    hours: String,
}

#[derive(Clone, serde::Deserialize)]
struct Tier {
    name: String,
    games: Vec<Game>,
}

#[derive(serde::Deserialize)]
struct TierList {
    tier: Vec<Tier>,
}



fn cover(game: &Game) -> String {
    return format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
        game.app
    );
}

fn spikes(count: usize, outer: f64, inner: f64) -> String {
    let mut path = String::new();
    let step = std::f64::consts::PI / count as f64;
    for index in 0..count * 2 {
        let radius = match index % 2 {
            0 => outer,
            _ => inner,
        };
        let angle = -std::f64::consts::FRAC_PI_2 + step * index as f64;
        let x = 16.0 + radius * angle.cos();
        let y = 16.0 + radius * angle.sin();
        path.push_str(match index {
            0 => "M",
            _ => "L",
        });
        path.push_str(&format!("{x:.2} {y:.2}"));
    }
    path.push('Z');
    return path;
}

fn medal(tier: &str) -> AnyView {
    let class = format!("medal medal-{}", tier.to_lowercase());
    let (count, inner) = match tier {
        "S" => (16, 10.4),
        "A" => (7, 7.6),
        _ => (5, 7.0),
    };
    return view! {
        <svg class=class viewBox="0 0 32 32" aria-hidden="true">
            <path class="rays" d=spikes(count, 15.2, inner)/>
            <circle class="rim" cx="16" cy="16" r="8.6"/>
            <circle class="face" cx="16" cy="16" r="6.9"/>
            <circle class="groove" cx="16" cy="16" r="4.6"/>
            <path class="shine" d="M16 9.1a6.9 6.9 0 0 0-6.9 6.9 6.9 6.9 0 0 0 .35 2.1 9.4 9.4 0 0 1 8.65-8.9 6.9 6.9 0 0 0-2.1-.1z"/>
            <circle class="gloss" cx="13.1" cy="13.2" r="1.5"/>
        </svg>
    }
    .into_any();
}

fn tile(tier: &str, game: Game) -> AnyView {
    let alt = game.title.clone();
    let title = game.title.clone();
    let label = game.hours.clone();
    return view! {
        <div class="game" title=title>
            <img src=cover(&game) alt=alt loading="lazy"/>
            {medal(tier)}
            <span class="pill game-hours">{label}</span>
        </div>
    }
    .into_any();
}

fn band(tier: &Tier) -> AnyView {
    let class = format!("tier tier-{}", tier.name.to_lowercase());
    let name = tier.name.clone();
    let games = tier
        .games
        .clone()
        .into_iter()
        .map(move |game| tile(&name, game))
        .collect_view();
    return view! {
        <div class=class>
            <div class="tier-games">{games}</div>
        </div>
    }
    .into_any();
}

#[component]
pub fn Games() -> impl IntoView {
    let open = RwSignal::new(false);
    let mounted = RwSignal::new(false);
    let mosaic = move || {
        let list: TierList = toml::from_str(GAMES).unwrap();
        return list
            .tier
            .iter()
            .enumerate()
            .map(|(_, tier)| band(tier))
            .collect::<Vec<_>>();
    };
    return view! {
        <div class="games" class:is-open=move || open.get()>
            <h2 on:click=move |_| {
                mounted.set(true);
                open.update(|open| *open = !*open);
            }>"Game Collection"</h2>
            <div class="games-reveal">
                <div class="games-reveal-inner">
                    {move || mounted.get().then(|| view! { <div class="mosaic">{mosaic()}</div> })}
                </div>
            </div>
        </div>
    };
}
