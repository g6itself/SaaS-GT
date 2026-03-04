use leptos::prelude::*;

#[component]
pub fn GameCard(
    id: String,
    title: String,
    #[prop(optional)] cover_image_url: Option<String>,
    total_achievements: i32,
    unlocked_achievements: i32,
) -> impl IntoView {
    let completion = if total_achievements > 0 {
        (unlocked_achievements as f64 / total_achievements as f64 * 100.0).round() as i32
    } else {
        0
    };

    let href = format!("/games/{}", id);

    view! {
        <a href=href class="game-card">
            {cover_image_url.map(|url| view! { <img src=url alt=title.clone() class="game-cover"/> })}
            <div class="game-info">
                <h4>{title}</h4>
                <div class="game-progress">
                    <div class="progress-bar">
                        <div class="progress-fill" style=format!("width: {}%", completion)></div>
                    </div>
                    <span class="progress-text">
                        {format!("{}/{} ({}%)", unlocked_achievements, total_achievements, completion)}
                    </span>
                </div>
            </div>
        </a>
    }
}
