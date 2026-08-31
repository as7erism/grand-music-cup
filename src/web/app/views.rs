use maud::{DOCTYPE, Markup, html};

use crate::{
    model::user::User,
    web::app::auth::{LOG_IN_PATH, SIGN_UP_PATH},
};

fn reactive_header(page_title: &str, user: Option<&User>) -> Markup {
    html! {
        (DOCTYPE)
        meta charset="utf-8";
        script src="/assets/htmx.min.js" {}
        link rel="icon" href="/assets/favicon.png" type="image/x-icon";
        link rel="stylesheet" href="/assets/style.css";
        title { (page_title) }

        nav .bg-pink-50.py-4.px-8.flex.items-center.justify-between.text-2xl {
            div {
                a href="/" .text-mauve-700.hover:text-mauve-500 { "grand music cup" }
            }
            div {
                @if let Some(user) = user {
                    (user.display_name())
                } @else {
                    a href=(LOG_IN_PATH) .text-mauve-700.hover:text-mauve-500 { "log in" }
                    " / "
                    a href=(SIGN_UP_PATH) .text-mauve-700.hover:text-mauve-500 { "sign up" }
                }
            }
        }
    }
}

fn static_header(page_title: &str, user: Option<&User>) -> Markup {
    html! {
        (DOCTYPE)
        meta charset="utf-8";
        link rel="icon" href="/assets/favicon.png" type="image/x-icon";
        link rel="stylesheet" href="/assets/style.css";
        title { (page_title) }

        nav .bg-pink-50.py-4.px-8.flex.items-center.justify-between.text-2xl {
            div {
                a href="/" .text-mauve-700.hover:text-mauve-500 { "grand music cup" }
            }
            div {
                @if let Some(user) = user {
                    (user.display_name())
                } @else {
                    a href=(LOG_IN_PATH) .text-mauve-700.hover:text-mauve-500 { "log in" }
                    " / "
                    a href=(SIGN_UP_PATH) .text-mauve-700.hover:text-mauve-500 { "sign up" }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {}
}

pub fn reactive_page(page_title: &str, content: Markup, user: Option<&User>) -> Markup {
    html! {
        (reactive_header(page_title, user))
        (content)
        (footer())
    }
}

pub fn static_page(page_title: &str, content: Markup, user: Option<&User>) -> Markup {
    html! {
        (static_header(page_title, user))
        (content)
        (footer())
    }
}
