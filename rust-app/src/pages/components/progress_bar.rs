use leptos::prelude::*;

#[component]
pub fn ProgressBar(
    value: i32,
    max: i32,
    #[prop(optional)] label: Option<String>,
) -> impl IntoView {
    let percentage = if max > 0 {
        (value as f64 / max as f64 * 100.0).round() as i32
    } else {
        0
    };

    view! {
        <div class="progress-container">
            {label.map(|l| view! { <span class="progress-label">{l}</span> })}
            <div class="progress-bar">
                <div
                    class="progress-fill"
                    style=format!("width: {}%", percentage)
                ></div>
            </div>
            <span class="progress-text">{format!("{}/{} ({}%)", value, max, percentage)}</span>
        </div>
    }
}
