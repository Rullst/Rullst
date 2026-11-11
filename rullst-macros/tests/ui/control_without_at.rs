use rullst_macros::html;

fn main() {
    let ok = true;
    let _ = html! {
        <div>if ok { <p>"yes"</p> }</div>
    };
}
