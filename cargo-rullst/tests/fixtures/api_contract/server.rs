//! Disposable HTTP consumer. Tokens are fixed test identities, never an auth provider.
use api_contract_consumer::contract::{self, op_read_lesson as read, op_save_lesson as save, Contract, DtoFailure, DtoLessonReply, DtoLessonRequest};
use axum::{body::Bytes, extract::{DefaultBodyLimit, Path, Query, State}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::{get, post}, Router};
use std::sync::Arc;

fn identity(headers: &HeaderMap, owner: &str) -> u16 {
    match headers.get("authorization").and_then(|value| value.to_str().ok()) {
        Some("Bearer fixture_alice") if owner == "alice" => 200,
        Some("Bearer fixture_bob") if owner == "bob" => 200,
        Some("Bearer fixture_alice" | "Bearer fixture_bob") => 403,
        _ => 401,
    }
}
fn send(value: Result<(u16, Vec<u8>), contract::ContractError>) -> Response {
    match value {
        Ok((status, body)) => (StatusCode::from_u16(status).unwrap(), [("content-type", "application/json"), ("cache-control", "no-store")], body).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn save_lesson(State(contract): State<Arc<Contract>>, Path(owner): Path<String>, Query(query): Query<Vec<(String,String)>>, headers: HeaderMap, body: Bytes) -> Response {
    let status = identity(&headers, &owner);
    if status != 200 {
        let failure = DtoFailure { code: "denied".into() };
        return send(save::encode_response(&contract, &if status == 401 { save::Response::Status401(failure) } else { save::Response::Status403(failure) }));
    }
    let query = query.iter().map(|(k,v)| (k.as_str(),v.as_str())).collect::<Vec<_>>();
    let params = save::decode_params(&contract, &[("owner", &owner)], &query);
    match (params, save::decode_request(&contract, &body)) {
        (Ok(_), Ok(request)) => send(save::encode_response(&contract, &save::Response::Status200(DtoLessonReply { owner, request }))),
        _ => send(save::encode_response(&contract, &save::Response::Status422(DtoFailure { code: "invalid".into() }))),
    }
}
async fn read_lesson(State(contract): State<Arc<Contract>>, Path(owner): Path<String>, Query(query): Query<Vec<(String,String)>>, headers: HeaderMap) -> Response {
    let status = identity(&headers, &owner);
    if status != 200 {
        let failure = DtoFailure { code: "denied".into() };
        return send(read::encode_response(&contract, &if status == 401 { read::Response::Status401(failure) } else { read::Response::Status403(failure) }));
    }
    let query = query.iter().map(|(k,v)| (k.as_str(),v.as_str())).collect::<Vec<_>>();
    if read::decode_params(&contract, &[("owner", &owner)], &query).is_err() {
        return send(read::encode_response(&contract, &read::Response::Status422(DtoFailure { code: "invalid".into() })));
    }
    send(read::encode_response(&contract, &read::Response::Status200(DtoLessonReply { owner, request: DtoLessonRequest { title: "Olá 🌍".into(), note: None, attempts: 1, tags: vec![], nickname: None, details: None, active: true } })))
}
#[tokio::main]
async fn main() {
    let contract = Arc::new(Contract::new().unwrap());
    let app = Router::new().route(save::PATH, post(save_lesson)).route(read::PATH, get(read_lesson))
        .layer(DefaultBodyLimit::max(contract::MAX_WIRE_BYTES)).with_state(contract);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    std::fs::write(std::env::var("RULLST_API_FIXTURE_READY").unwrap(), listener.local_addr().unwrap().to_string()).unwrap();
    axum::serve(listener, app).await.unwrap();
}
