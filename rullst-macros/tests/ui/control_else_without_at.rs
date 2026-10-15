use rullst_macros::html;

fn main() {
    let signed_in = true;
    let _ = html! {
        <nav>
            @if signed_in { <a href="/logout">"Sign out"</a> } else { <a href="/login">"Sign in"</a> }
        </nav>
    };
}
