use maud::{DOCTYPE, Markup, html};

use crate::database::User;

fn header(page_title: &str, user: Option<&User>) -> Markup {
    html! {
        (DOCTYPE)
        meta charset="utf-8";
        link rel="icon" href="/assets/favicon.png" type="image/x-icon";
        link rel="stylesheet" href="/assets/style.css";
        title { (page_title) }

        nav .bg-pink-50.py-4.px-8.flex.items-center.justify-between.text-2xl {
            div {
                a href="/" .text-mauve-700.hover:text-mauve-500 { "grand music league" } 
            }
            div {
                @if let Some(user) = user {
                    (user.display_name())
                } @else {
                    a href="/log-in" .text-mauve-700.hover:text-mauve-500 { "log in" }
                    " / "
                    a href="/sign-up" .text-mauve-700.hover:text-mauve-500 { "sign up" }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {}
}

pub fn page(page_title: &str, content: Markup, user: Option<&User>) -> Markup {
    html! {
        (header(page_title, user))
        (content)
        (footer())
    }
}
