use super::*;

#[test]
fn models_document_lists_public_items_in_file_order() {
    let project = tempfile::tempdir().expect("project");
    let models = project.path().join("models");
    fs::create_dir_all(&models).expect("models dir");
    fs::write(
        models.join("user.rs"),
        "pub struct User {\n    pub id: i64,\n    pub email: Option<String>,\n    secret: String,\n}\nstruct Hidden;\npub enum Role { Admin, Member }\n",
    )
    .expect("user");
    fs::write(models.join("mod.rs"), "pub mod user;\n").expect("mod");
    fs::write(
        models.join("a_post.rs"),
        "pub struct Post { pub tags: Vec<&'static str> }\n",
    )
    .expect("post");
    fs::write(models.join("notes.txt"), "ignored").expect("txt");

    let document = models_document(&models);
    assert_eq!(document["schema"], "rullst.cli-models.v1");
    assert_eq!(document["complete"], true);
    assert!(document["incomplete_reason"].is_null());
    assert_eq!(document["files"][0]["file"], "a_post.rs");
    assert_eq!(
        document["files"][0]["items"][0]["fields"][0],
        json!({"name": "tags", "type": "Vec<&'static str>"})
    );
    assert_eq!(document["files"][1]["file"], "user.rs");
    let items = &document["files"][1]["items"];
    assert_eq!(items.as_array().expect("items").len(), 2);
    assert_eq!(items[0]["kind"], "struct");
    assert_eq!(items[0]["name"], "User");
    assert_eq!(
        items[0]["fields"],
        json!([
            {"name": "id", "type": "i64"},
            {"name": "email", "type": "Option<String>"}
        ])
    );
    assert_eq!(items[0]["variants"], json!([]));
    assert_eq!(items[1]["kind"], "enum");
    assert_eq!(items[1]["variants"], json!(["Admin", "Member"]));
    assert_eq!(items[1]["fields"], json!([]));
}

#[test]
fn models_document_reports_what_it_could_not_read() {
    let project = tempfile::tempdir().expect("project");
    let missing = models_document(&project.path().join("absent"));
    assert_eq!(missing["complete"], false);
    assert_eq!(missing["files"], json!([]));
    assert!(
        missing["incomplete_reason"]
            .as_str()
            .expect("reason")
            .ends_with("directory not found")
    );

    let models = project.path().join("models");
    fs::create_dir_all(&models).expect("models dir");
    fs::write(models.join("broken.rs"), "pub struct {").expect("broken");
    let document = models_document(&models);
    assert_eq!(document["complete"], false);
    assert_eq!(
        document["incomplete_reason"],
        "broken.rs could not be parsed as Rust"
    );
    assert_eq!(document["files"], json!([]));
}

#[cfg(unix)]
#[test]
fn models_document_never_follows_symlinks() {
    let project = tempfile::tempdir().expect("project");
    let models = project.path().join("models");
    fs::create_dir_all(&models).expect("models dir");
    fs::write(project.path().join("outside.rs"), "pub struct Out;\n").expect("outside");
    std::os::unix::fs::symlink(project.path().join("outside.rs"), models.join("link.rs"))
        .expect("link");
    let document = models_document(&models);
    assert_eq!(document["files"], json!([]));
    assert_eq!(
        document["incomplete_reason"],
        "link.rs is not a readable regular file"
    );
}

#[test]
fn schema_document_prefers_the_project_snapshot_and_tags_both_shapes() {
    let project = tempfile::tempdir().expect("project");
    let snapshot = project.path().join("rullst-schema.json");
    let derived = json!({"source": "src", "models": []});

    let from_source = schema_document(&snapshot, derived.clone()).expect("derived");
    assert_eq!(from_source["schema"], "rullst.cli-schema.v1");
    assert_eq!(from_source["source"], "src");
    assert_eq!(from_source["models"], json!([]));

    fs::write(&snapshot, "{\"tables\": [1]}").expect("snapshot");
    let provided = schema_document(&snapshot, derived.clone()).expect("snapshot");
    assert_eq!(provided["schema"], "rullst.cli-schema.v1");
    assert_eq!(provided["source"], "rullst-schema.json");
    assert_eq!(provided["snapshot"], json!({"tables": [1]}));
    assert!(provided.get("models").is_none());

    fs::write(&snapshot, "not json").expect("broken snapshot");
    assert!(schema_document(&snapshot, derived).is_err());
}
