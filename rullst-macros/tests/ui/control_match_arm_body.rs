use rullst_macros::html;

fn main() {
    let count = 2;
    let _ = html! {
        <p>
            @match count { 0 => "none", _ => { "some" } }
        </p>
    };
}
