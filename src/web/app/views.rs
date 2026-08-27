use maud::{DOCTYPE, Markup, html};

use crate::database::User;

fn header(page_title: &str, user: Option<&User>) -> Markup {
    html! {
        (DOCTYPE)
        meta charset="utf-8";
        link rel="icon" href="/assets/favicon.png" type="image/x-icon";
        link rel="stylesheet" href="/assets/style.css";
        title { (page_title) }

        nav {
            ul {
                li { a href="/" { "grand music league" } }
            }
            @if let Some(user) = user {
                (user.display_name())
            } @else {
                ul {
                    li { a href="/log-in" { "log in" } }
                    li { a href="/sign-up" { "sign up" } }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {
        p { "hi" }
    }
}

pub fn page(page_title: &str, content: Markup, user: Option<&User>) -> Markup {
    html! {
        (header(page_title, user))
        (content)
        (footer())
    }
}
