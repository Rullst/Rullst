use rullst_macros::html;

fn main() {
    let mut count = 2;
    let _ = html! {
        <p>
            @while count > 0 { "tick" }
        </p>
    };
    count -= 1;
    let _ = count;
}
