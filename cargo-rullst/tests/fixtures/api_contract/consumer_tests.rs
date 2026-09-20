use crate::contract::{op_save_lesson as save, Contract, ContractError};
#[test]
fn codecs_validate_wire_values_presence_and_output() {
    let contract = Contract::new().unwrap();
    let body = serde_json::json!({"title":"Olá 🌍", "note":null, "attempts":1, "tags":["x"], "active":true, "details":{"lower":-9007199254740991_i64,"upper":9007199254740991_i64,"maybe_number":null,"maybe_flag":null,"maybe_items":[null,true,false],"child":{"code":"nested"}}});
    let bytes = serde_json::to_vec(&body).unwrap();
    let request = save::decode_request(&contract, &bytes).unwrap();
    assert_eq!(request.note, None);
    assert_eq!(request.nickname, None);
    assert_eq!(request.title, "Olá 🌍");
    assert_eq!(request.details.as_ref().unwrap().lower,-9_007_199_254_740_991);
    let reply = crate::contract::DtoLessonReply { owner:"alice".into(), request };
    let (status, encoded) = save::encode_response(&contract, &save::Response::Status200(reply.clone())).unwrap();
    assert_eq!(status,200);
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(value["request"]["note"], serde_json::Value::Null);
    assert!(value["request"].get("nickname").is_none());
    for (field,value) in [("nickname",serde_json::Value::Null),("attempts",serde_json::json!(101)),("attempts",serde_json::json!(1.5)),("unexpected",serde_json::json!(true)),("tags",serde_json::json!(["x","x","x","x","x"]))] {
        let mut changed = body.clone(); changed[field] = value;
        assert!(matches!(save::decode_request(&contract,&serde_json::to_vec(&changed).unwrap()),Err(ContractError::Payload)));
    }
    let mut changed = body.clone(); changed.as_object_mut().unwrap().remove("note");
    assert!(save::decode_request(&contract,&serde_json::to_vec(&changed).unwrap()).is_err());
    let decimal = String::from_utf8(bytes.clone()).unwrap().replace("\"attempts\":1", "\"attempts\":1.0");
    assert_eq!(save::decode_request(&contract, decimal.as_bytes()).unwrap().attempts, 1);
    let fractional = String::from_utf8(bytes.clone()).unwrap().replace("\"attempts\":1", "\"attempts\":1.00000000000000001");
    assert!(save::decode_request(&contract, fractional.as_bytes()).is_err());
    let duplicate = String::from_utf8(bytes).unwrap().replacen('{', "{\"title\":\"duplicate\",", 1);
    assert!(save::decode_request(&contract,duplicate.as_bytes()).is_err());
    assert!(save::decode_request(&contract,&vec![b' ';crate::contract::MAX_WIRE_BYTES+1]).is_err());
    let mut invalid = reply; invalid.request.attempts=101;
    assert!(save::encode_response(&contract,&save::Response::Status200(invalid)).is_err());
    let params=save::decode_params(&contract,&[("owner","alice")],&[("verbose","true"),("limit","12")]).unwrap();
    assert_eq!(params.q_limit,Some(12)); assert_eq!(params.q_verbose,Some(true));
    for query in [vec![("verbose","1")],vec![("limit","01")],vec![("limit","101")],vec![("limit","1"),("limit","2")],vec![("extra","x")]] {
        assert!(save::decode_params(&contract,&[("owner","alice")],&query).is_err());
    }
    assert!(save::decode_params(&contract,&[],&[]).is_err());
}
