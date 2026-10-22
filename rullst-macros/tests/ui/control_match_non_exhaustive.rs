extern crate self as rullst;

/// Stand-ins for `rullst::html`: the case fails at the `match` itself.
pub mod html {
    pub struct Markup;

    impl Markup {
        pub fn from_trusted(_: String) -> Self {
            Markup
        }
    }
}

use rullst_macros::html;

enum Plan {
    Free,
    Pro,
    Team,
}

fn main() {
    let plan = Plan::Team;
    let _ = html! {
        @match plan {
            Plan::Free => { "free" }
            Plan::Pro => { "pro" }
        }
    };
    let _ = (Plan::Free, Plan::Pro);
}
