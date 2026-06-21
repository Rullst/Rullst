use rocket::serde::Serialize;

#[derive(Serialize)]
pub struct Message {
    pub message: String,
}
