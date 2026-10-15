use rullst_macros::html;

fn main() {
    let _ = html! {
        <input type="checkbox" checked?="checked" />
    };
}
