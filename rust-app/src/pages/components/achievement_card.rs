use leptos::prelude::*;

#[component]
pub fn AchievementCard(
    name: String,
    #[prop(optional)] description: Option<String>,
    #[prop(optional)] icon_url: Option<String>,
    is_unlocked: bool,
    #[prop(optional)] platform: Option<String>,
) -> impl IntoView {
    let card_class = if is_unlocked {
        "achievement-card unlocked"
    } else {
        "achievement-card locked"
    };

    view! {
        <div class=card_class>
            {icon_url.map(|url| view! { <img src=url alt=name.clone() class="achievement-icon"/> })}
            <div class="achievement-info">
                <h4>{name}</h4>
                {description.map(|d| view! { <p class="achievement-desc">{d}</p> })}
                {platform.map(|p| view! { <span class="badge">{p}</span> })}
            </div>
            <div class="achievement-status">
                {if is_unlocked { "Debloque" } else { "Verrouille" }}
            </div>
        </div>
    }
}
