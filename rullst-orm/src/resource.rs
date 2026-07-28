use serde_json::Value;

pub trait ApiResource {
    fn to_array(&self) -> Value;
}

pub struct JsonResource<'a, T: ApiResource> {
    pub data: &'a T,
}

impl<'a, T: ApiResource> JsonResource<'a, T> {
    pub fn new(data: &'a T) -> Self {
        Self { data }
    }

    pub fn resolve(&self) -> Value {
        self.data.to_array()
    }
}

pub struct ResourceCollection<'a, T: ApiResource> {
    pub data: &'a [T],
}

impl<'a, T: ApiResource> ResourceCollection<'a, T> {
    pub fn new(data: &'a [T]) -> Self {
        Self { data }
    }

    pub fn resolve(&self) -> Value {
        let array: Vec<Value> = self.data.iter().map(|item| item.to_array()).collect();
        serde_json::json!(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct DummyResource {
        value: i32,
    }

    impl ApiResource for DummyResource {
        fn to_array(&self) -> Value {
            json!({ "value": self.value })
        }
    }

    #[test]
    fn test_json_resource_new_stores_data() {
        let r = DummyResource { value: 42 };
        let jr = JsonResource::new(&r);
        assert_eq!(jr.data.value, 42);
    }

    #[test]
    fn test_json_resource_resolve() {
        let r = DummyResource { value: 7 };
        let jr = JsonResource::new(&r);
        let v = jr.resolve();
        assert_eq!(v["value"], 7);
    }

    struct ComplexResource {
        id: u64,
        name: String,
        tags: Vec<String>,
        metadata: Option<Value>,
    }

    impl ApiResource for ComplexResource {
        fn to_array(&self) -> Value {
            json!({
                "id": self.id,
                "name": self.name,
                "tags": self.tags,
                "metadata": self.metadata,
                "is_active": true,
            })
        }
    }

    #[test]
    fn test_json_resource_resolve_complex() {
        let r = ComplexResource {
            id: 101,
            name: "Alice".to_string(),
            tags: vec!["admin".to_string(), "user".to_string()],
            metadata: Some(json!({ "login_count": 5 })),
        };
        let jr = JsonResource::new(&r);
        let v = jr.resolve();
        assert_eq!(v["id"], 101);
        assert_eq!(v["name"], "Alice");
        assert_eq!(v["tags"].as_array().unwrap().len(), 2);
        assert_eq!(v["tags"][0], "admin");
        assert_eq!(v["metadata"]["login_count"], 5);
        assert_eq!(v["is_active"], true);

        let r_null = ComplexResource {
            id: 102,
            name: "Bob".to_string(),
            tags: vec![],
            metadata: None,
        };
        let jr_null = JsonResource::new(&r_null);
        let v_null = jr_null.resolve();
        assert!(v_null["metadata"].is_null());
        assert_eq!(v_null["tags"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_resource_collection_new_stores_slice() {
        let items = vec![DummyResource { value: 1 }, DummyResource { value: 2 }];
        let rc = ResourceCollection::new(&items);
        assert_eq!(rc.data.len(), 2);
    }

    #[test]
    fn test_resource_collection_resolve_returns_array() {
        let items = vec![DummyResource { value: 10 }, DummyResource { value: 20 }];
        let rc = ResourceCollection::new(&items);
        let v = rc.resolve();
        assert!(v.is_array());
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["value"], 10);
        assert_eq!(arr[1]["value"], 20);
    }

    #[test]
    fn test_resource_collection_empty() {
        let items: Vec<DummyResource> = vec![];
        let rc = ResourceCollection::new(&items);
        let v = rc.resolve();
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_resource_collection_single_item() {
        let items = vec![DummyResource { value: 99 }];
        let rc = ResourceCollection::new(&items);
        let v = rc.resolve();
        assert!(v.is_array());
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["value"], 99);
    }

    #[test]
    fn test_resource_collection_complex_items() {
        let items = vec![
            ComplexResource {
                id: 1,
                name: "Alpha".to_string(),
                tags: vec!["a".to_string()],
                metadata: None,
            },
            ComplexResource {
                id: 2,
                name: "Beta".to_string(),
                tags: vec!["b".to_string(), "c".to_string()],
                metadata: Some(json!({"role": "admin"})),
            },
        ];
        let rc = ResourceCollection::new(&items);
        let v = rc.resolve();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Alpha");
        assert_eq!(arr[1]["metadata"]["role"], "admin");
        assert_eq!(arr[1]["tags"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_resource_collection_resolve_large() {
        let items: Vec<DummyResource> = (0..10_000).map(|i| DummyResource { value: i }).collect();
        let rc = ResourceCollection::new(&items);
        let v = rc.resolve();
        assert!(v.is_array());
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 10_000);
        assert_eq!(arr[9_999]["value"], 9_999);
    }
}
