//! The designer's server (`repo.rs`, `http.rs`, `registry.rs`): the scenarios of its Python
//! tests, ported with it in spec 39 step 2c. Each test works on a repo of its own in a fresh
//! temp folder; a fake launcher stands in for Godot.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::http::{read_request, Request, Server, API_VERSION, STATIC_FILES};
use super::registry::{read_registry, read_units, targets};
use super::repo::{find_godot, is_valid_name, to_native_path, write_json, Repo};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh folder for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("breach-server-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A repo in `root` with a copy of the checked-in terrain library, and the commands the fake
/// launcher was given.
fn repo_in(root: &Path, godot: Option<&str>) -> (Repo, Arc<Mutex<Vec<Vec<String>>>>) {
    let terrain = root.join("content/terrain");
    fs::create_dir_all(&terrain).unwrap();
    let library = repo_root().join("content/terrain/terrain_library.json");
    fs::copy(library, terrain.join("terrain_library.json")).unwrap();
    let launched = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&launched);
    let launcher = Box::new(move |command: &[String]| {
        record.lock().unwrap().push(command.to_vec());
        Ok(())
    });
    (Repo::new(root.to_path_buf(), godot.map(String::from), launcher), launched)
}

/// The designer's demo map, as it saves it.
fn demo() -> Value {
    let path = repo_root().join("content/maps_src/demo_map.designer.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn names_that_could_leave_their_folder_are_refused() {
    for name in ["test_map", "demo-map", "Map01"] {
        assert!(is_valid_name(name), "{name}");
    }
    for name in ["", "../x", "a/b", "a\\b", "..", "a.b", "a b", &"x".repeat(65)] {
        assert!(!is_valid_name(name), "{name}");
    }
}

#[test]
fn saving_a_map_writes_its_export_and_the_map_in_the_schema() {
    let root = scratch("save");
    let (repo, _) = repo_in(&root, None);

    let saved = repo.save_map("m", &demo()).unwrap();

    let source = root.join("content/maps_src/m.designer.json");
    let raw = fs::read_to_string(&source).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&raw).unwrap(), demo());
    assert!(raw.ends_with("}\n") && !raw.contains("\r\n") && raw.contains("\n  \"format\": "));
    assert_eq!(saved["saved"], "content/maps_src/m.designer.json");
    assert_eq!(saved["import"]["ok"], true, "{saved}");
    assert_eq!(saved["import"]["output_path"], "content/maps/m.json");
    assert!(saved["import"]["warnings"][0].as_str().unwrap().contains("structures"));
    let written = fs::read_to_string(root.join("content/maps/m.json")).unwrap();
    let checked_in = fs::read_to_string(repo_root().join("content/maps/demo_map.json")).unwrap();
    assert_eq!(written, checked_in);
}

#[test]
fn a_map_the_crate_refuses_is_saved_but_reported_and_not_written() {
    let root = scratch("refused");
    let (repo, _) = repo_in(&root, None);
    let mut export = demo();
    export["tiles"][0]["terrain"] = json!("NOT_A_TERRAIN");

    let saved = repo.save_map("m", &export).unwrap();

    assert!(root.join("content/maps_src/m.designer.json").is_file());
    assert_eq!(saved["import"]["ok"], false);
    assert!(saved["import"]["errors"][0].as_str().unwrap().contains("NOT_A_TERRAIN"));
    assert!(!root.join("content/maps/m.json").exists());
}

#[test]
fn a_map_that_breaks_a_rule_is_reported() {
    let root = scratch("rule");
    let (repo, _) = repo_in(&root, None);
    let mut export = demo();
    export["nodes"][1]["fields"]["yield_food_per_tick"] = json!(0);

    let saved = repo.save_map("m", &export).unwrap();

    assert!(saved["import"]["errors"][0]
        .as_str()
        .unwrap()
        .contains("yield_food_per_tick must be > 0"));
}

#[test]
fn a_bad_name_or_a_body_that_isnt_a_designer_map_is_refused_before_anything_is_written() {
    let root = scratch("bad-save");
    let (repo, _) = repo_in(&root, None);
    assert!(repo.save_map("../escape", &demo()).is_err());
    assert!(repo.save_map("m", &json!({"hello": "world"})).is_err());
    assert!(!root.join("content/maps_src").exists());
}

#[test]
fn saved_maps_list_with_whether_their_map_is_written_and_read_back() {
    let root = scratch("list");
    let (repo, _) = repo_in(&root, None);
    assert_eq!(repo.list_maps(), json!([]));
    repo.save_map("b", &json!({"format": "breach-designer-map"})).unwrap();
    repo.save_map("a", &demo()).unwrap();

    let listed = repo.list_maps();

    let names: Vec<&str> =
        listed.as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(
        (listed[0]["imported"].clone(), listed[1]["imported"].clone()),
        (json!(true), json!(false))
    );
    assert_eq!(repo.read_map("a").unwrap(), Some(demo()));
    assert_eq!(repo.read_map("absent").unwrap(), None);
}

#[test]
fn viewing_a_saved_map_opens_the_godot_viewer_on_it() {
    let root = scratch("view");
    let (repo, launched) = repo_in(&root, Some("godot"));
    repo.save_map("m", &demo()).unwrap();

    let viewed = repo.view_map("m").unwrap();

    assert_eq!(viewed["launched"], true);
    let command = launched.lock().unwrap()[0].clone();
    assert_eq!(command[0], "godot");
    assert!(!command.contains(&"--headless".to_string()));
    assert!(command.contains(&"res://presentation/map_viewer.tscn".to_string()));
    assert_eq!(command.last().unwrap(), "--map=m");
}

#[test]
fn without_godot_or_the_map_file_nothing_is_launched() {
    let root = scratch("no-view");
    let (without_godot, launched) = repo_in(&root, None);
    assert!(without_godot.view_map("m").is_err());
    let (unsaved, _) = repo_in(&root, Some("godot"));
    assert!(unsaved.view_map("m").unwrap_err().contains("save the map first"));
    assert!(unsaved.view_map("../x").is_err());
    assert!(launched.lock().unwrap().is_empty());
}

#[test]
fn a_godot_that_cannot_start_is_reported() {
    let root = scratch("no-start");
    let failing = Box::new(|_: &[String]| Err(std::io::Error::other("no such file")));
    let (saver, _) = repo_in(&root, None);
    saver.save_map("m", &demo()).unwrap();
    let repo = Repo::new(root.clone(), Some("godot".into()), failing);
    assert!(repo.view_map("m").unwrap_err().contains("could not start Godot"));
}

#[test]
fn a_library_round_trips_and_an_unchanged_write_reports_no_change() {
    let root = scratch("library");
    let (repo, _) = repo_in(&root, None);
    let value = json!([{"id": "player", "display_name": "Player"}]);
    assert_eq!(repo.read_library("factions").unwrap(), Value::Null);
    assert_eq!(
        repo.save_library("factions", &value).unwrap(),
        json!({"changed": true, "import": null})
    );
    assert_eq!(repo.save_library("factions", &value).unwrap()["changed"], false);
    assert_eq!(repo.read_library("factions").unwrap(), value);
    for library in ["../factions", "secrets", ""] {
        assert!(repo.save_library(library, &json!([])).is_err(), "{library}");
    }
}

#[test]
fn the_terrain_library_reads_in_the_designers_shape_and_writes_in_the_schema() {
    let root = scratch("terrain");
    let (repo, _) = repo_in(&root, None);
    let mut designer = repo.read_library("terrain").unwrap();
    let fields =
        designer["terrains"].as_array().unwrap().iter().find(|t| t["id"] == "FIELDS").unwrap();
    assert_eq!(fields["label"], "Fields");
    assert_eq!(repo.save_library("terrain", &designer).unwrap()["changed"], false);

    designer["terrains"][0]["label"] = json!("Meadow");
    let saved = repo.save_library("terrain", &designer).unwrap();

    assert_eq!(
        (saved["changed"].clone(), saved["import"]["ok"].clone()),
        (json!(true), json!(true))
    );
    let written = fs::read_to_string(root.join("content/terrain/terrain_library.json")).unwrap();
    assert!(written.contains("\"display_name\": \"Meadow\""));
    assert!(!root.join("content/designer/terrain.json").exists());
}

#[test]
fn dropping_a_terrain_a_saved_map_uses_is_refused_and_nothing_is_written() {
    let root = scratch("in-use");
    let (repo, _) = repo_in(&root, None);
    repo.save_map("demo_map", &demo()).unwrap();
    let path = root.join("content/terrain/terrain_library.json");
    let before = fs::read_to_string(&path).unwrap();
    let mut designer = repo.read_library("terrain").unwrap();
    designer["terrains"].as_array_mut().unwrap().retain(|terrain| terrain["id"] != "FIELDS");

    let refused = repo.save_library("terrain", &designer).unwrap();

    assert_eq!(
        (refused["changed"].clone(), refused["import"]["ok"].clone()),
        (json!(false), json!(false))
    );
    let why = refused["import"]["errors"][0].as_str().unwrap();
    assert!(why.contains("terrain 'FIELDS' is still used by demo_map"), "{why}");
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn godot_is_found_from_the_flag_then_the_environment_then_local_config() {
    let root = scratch("godot");
    let config = root.join("local_config.json");
    write_json(&config, &json!({"godot": "from-config"})).unwrap();
    assert_eq!(find_godot(Some("from-cli"), Some("from-env"), &config).unwrap(), "from-cli");
    assert_eq!(find_godot(None, Some("from-env"), &config).unwrap(), "from-env");
    assert_eq!(find_godot(None, None, &config).unwrap(), "from-config");
    assert_eq!(find_godot(None, None, &root.join("absent.json")), None);
}

#[test]
fn git_bash_drive_paths_are_converted_on_windows_only() {
    let expected = if cfg!(windows) { "F:/Code/godot.exe" } else { "/f/Code/godot.exe" };
    assert_eq!(to_native_path("/f/Code/godot.exe"), expected);
}

/// A server on a free port over a fresh repo with a page of its own.
fn served(name: &str) -> (String, PathBuf, Arc<Mutex<Vec<Vec<String>>>>) {
    let root = scratch(name);
    let page = root.join("static");
    fs::create_dir_all(&page).unwrap();
    fs::write(page.join("index.html"), "<p>designer</p>").unwrap();
    let (repo, launched) = repo_in(&root, Some("godot"));
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = Arc::new(Server::new(repo, page));
    std::thread::spawn(move || server.serve(listener));
    (format!("127.0.0.1:{port}"), root, launched)
}

/// The status and body of a request to the server at `address`, with the local Host unless
/// `headers` gives another, and JSON's Content-Type when there is a body.
fn request(
    address: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
    headers: &[(&str, &str)],
) -> (u16, Vec<u8>) {
    let payload = body.map(|value| value.to_string()).unwrap_or_default();
    let mut all: Vec<(String, String)> = vec![("Host".into(), address.into())];
    if body.is_some() {
        all.push(("Content-Type".into(), "application/json".into()));
    }
    for (key, value) in headers {
        all.retain(|(known, _)| !known.eq_ignore_ascii_case(key));
        all.push((key.to_string(), value.to_string()));
    }
    let mut text = format!("{method} {path} HTTP/1.1\r\n");
    for (key, value) in all {
        text.push_str(&format!("{key}: {value}\r\n"));
    }
    text.push_str(&format!("Content-Length: {}\r\n\r\n{payload}", payload.len()));
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(text.as_bytes()).unwrap();
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).unwrap();
    let split = reply.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let status = String::from_utf8_lossy(&reply[9..12]).parse().unwrap();
    (status, reply[split + 4..].to_vec())
}

fn json_of(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}

#[test]
fn the_server_serves_the_page_and_reports_its_health() {
    let (at, _, _) = served("page");
    let (status, body) = request(&at, "GET", "/", None, &[]);
    assert_eq!((status, String::from_utf8_lossy(&body).contains("designer")), (200, true));
    let (_, body) = request(&at, "GET", "/api/health", None, &[]);
    let health = json_of(&body);
    assert_eq!(
        (health["godot_found"].clone(), health["api_version"].clone()),
        (json!(true), json!(API_VERSION))
    );
}

#[test]
fn every_script_the_page_loads_is_served() {
    let page = fs::read_to_string(repo_root().join("tools/designer/index.html")).unwrap();
    for src in page.split("<script src=\"").skip(1).filter_map(|rest| rest.split('"').next()) {
        let path = format!("/{src}");
        assert!(STATIC_FILES.iter().any(|(at, _, _)| *at == path), "{path}");
    }
}

#[test]
fn a_map_put_is_saved_written_listed_and_readable() {
    let (at, root, _) = served("put");
    let (status, body) = request(&at, "PUT", "/api/maps/m", Some(&demo()), &[]);
    assert_eq!(status, 200);
    assert_eq!(json_of(&body)["import"]["ok"], true);
    assert!(root.join("content/maps/m.json").is_file());
    let (_, body) = request(&at, "GET", "/api/maps", None, &[]);
    assert_eq!(json_of(&body)[0]["name"], "m");
    let (_, body) = request(&at, "GET", "/api/maps/m", None, &[]);
    assert_eq!(json_of(&body), demo());
}

#[test]
fn missing_traversing_and_untyped_requests_are_refused() {
    let (at, root, _) = served("refuse");
    assert_eq!(request(&at, "GET", "/api/maps/absent", None, &[]).0, 404);
    assert_eq!(request(&at, "PUT", "/api/maps/..%2Fescape", Some(&demo()), &[]).0, 400);
    assert!(!root.join("content/maps_src").exists());
    let plain = [("Content-Type", "text/plain")];
    assert_eq!(request(&at, "PUT", "/api/maps/m", Some(&demo()), &plain).0, 400);
    assert_eq!(request(&at, "GET", "/api/nothing", None, &[]).0, 404);
    assert_eq!(request(&at, "GET", "/api/libraries/secrets", None, &[]).0, 400);
}

#[test]
fn cross_origin_requests_are_forbidden() {
    let (at, _, _) = served("origin");
    let evil = [("Origin", "https://evil.example")];
    assert_eq!(request(&at, "PUT", "/api/maps/m", Some(&demo()), &evil).0, 403);
    assert_eq!(request(&at, "GET", "/api/maps", None, &[("Host", "evil.example")]).0, 403);
}

#[test]
fn a_library_round_trips_through_the_server() {
    let (at, _, _) = served("library-http");
    let value = json!([{"id": "player", "display_name": "Player"}]);
    assert_eq!(json_of(&request(&at, "GET", "/api/libraries/factions", None, &[]).1), Value::Null);
    let (status, body) = request(&at, "PUT", "/api/libraries/factions", Some(&value), &[]);
    assert_eq!((status, json_of(&body)), (200, json!({"changed": true, "import": null})));
    assert_eq!(json_of(&request(&at, "GET", "/api/libraries/factions", None, &[]).1), value);
}

#[test]
fn the_viewer_opens_on_post_for_a_saved_map_only() {
    let (at, _, launched) = served("view-http");
    let (status, body) = request(&at, "POST", "/api/maps/absent/view", Some(&json!({})), &[]);
    assert_eq!(status, 400);
    assert!(json_of(&body)["error"].as_str().unwrap().contains("save the map first"));
    assert_eq!(request(&at, "GET", "/api/maps/m/view", None, &[]).0, 400);
    request(&at, "PUT", "/api/maps/m", Some(&demo()), &[]);
    let (status, body) = request(&at, "POST", "/api/maps/m/view", Some(&json!({})), &[]);
    assert_eq!((status, json_of(&body)["launched"].clone()), (200, json!(true)));
    assert_eq!(launched.lock().unwrap()[0].last().unwrap(), "--map=m");
}

#[test]
fn a_request_reads_its_line_headers_and_body() {
    let text = "PUT /api/maps/m HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\n\r\n{}";
    let read: Request = read_request(&mut text.as_bytes()).unwrap();
    assert_eq!(
        (read.method.as_str(), read.path.as_str(), read.body.as_slice()),
        ("PUT", "/api/maps/m", &b"{}"[..])
    );
}

fn tags() -> BTreeMap<String, Value> {
    [
        ("behaviour", json!({"id": "behaviour", "targets": ["unit", "leader"]})),
        ("combat", json!({"id": "combat", "targets": ["unit", "weapon"]})),
        ("physical", json!({"id": "physical", "targets": ["unit", "material"]})),
    ]
    .into_iter()
    .map(|(id, tag)| (id.to_string(), tag))
    .collect()
}

#[test]
fn a_trait_sits_where_its_tags_agree_narrowed_by_only() {
    assert_eq!(targets(&json!({"tags": ["behaviour"]}), &tags()), ["unit", "leader"]);
    assert_eq!(targets(&json!({"tags": ["combat", "physical"]}), &tags()), ["unit"]);
    assert_eq!(targets(&json!({"tags": ["combat"], "only": ["weapon"]}), &tags()), ["weapon"]);
    assert!(targets(&json!({"tags": []}), &tags()).is_empty());
}

const UNIT: &str = r#"{"tags": ["grem", "small"], "traits": {"cunning": 1, "hardened": 2}}"#;

#[test]
fn the_registry_reads_its_kinds_targets_and_usage() {
    let root = scratch("registry");
    let registry = root.join("content/registry");
    fs::create_dir_all(&registry).unwrap();
    let all_tags: Vec<Value> = tags().into_values().collect();
    fs::write(registry.join("tags.json"), json!({"tags": all_tags}).to_string()).unwrap();
    let traits = json!({"traits": [{"id": "cunning", "tags": ["behaviour"]}, {"id": "flows", "tags": []},
        {"id": "climber", "tags": ["physical"], "default": 0}]});
    fs::write(registry.join("traits.json"), traits.to_string()).unwrap();
    fs::create_dir_all(root.join("content/units")).unwrap();
    fs::write(root.join("content/units/grem.json"), UNIT).unwrap();
    fs::create_dir_all(root.join("content/terrain")).unwrap();
    let terrain = json!({"materials": [{"id": "WATER", "traits": {"flows": 1}}]});
    fs::write(root.join("content/terrain/terrain_library.json"), terrain.to_string()).unwrap();

    let read = read_registry(&root);

    assert_eq!(read["statuses"], json!([]));
    assert_eq!(read["traits"][0]["targets"], json!(["unit", "leader"]));
    for used in ["cunning", "hardened", "small"] {
        assert_eq!(read["usage"][used], json!(["content/units/grem.json"]), "{used}");
    }
    assert_eq!(read["usage"]["flows"], json!(["content/terrain/terrain_library.json"]));
    assert_eq!(read["implicit"], json!({"climber": 0}));
}

#[test]
fn the_repos_registry_reads_with_known_targets_usage_and_units() {
    let read = read_registry(&repo_root());
    let by_id = |id: &str| {
        read["traits"].as_array().unwrap().iter().find(|t| t["id"] == id).unwrap().clone()
    };
    assert_eq!(by_id("cunning")["targets"], json!(["unit", "leader"]));
    assert_eq!(by_id("siege")["targets"], json!(["weapon"]));
    assert!(read["usage"]["siege"]
        .as_array()
        .unwrap()
        .contains(&json!("content/weapons/brute_fists.json")));
    assert_eq!(
        (read["implicit"]["climber"].clone(), read["implicit"]["swimmer"].clone()),
        (json!(0), json!(0))
    );
    let grem =
        read["units"].as_array().unwrap().iter().find(|u| u["id"] == "grem").unwrap().clone();
    assert!(grem["traits"]
        .as_array()
        .unwrap()
        .contains(&json!({"id": "swimmer", "level": 0, "implicit": true})));
}

#[test]
fn a_unit_shows_its_own_traits_then_the_implicit_ones_labelled() {
    let root = scratch("units");
    fs::create_dir_all(root.join("content/units")).unwrap();
    fs::write(root.join("content/units/grem.json"), UNIT).unwrap();
    fs::write(
        root.join("content/units/goat.json"),
        UNIT.replace("\"hardened\": 2", "\"climber\": 2"),
    )
    .unwrap();
    let implicit = BTreeMap::from([("climber".to_string(), 0), ("swimmer".to_string(), 0)]);

    let units = read_units(&root, &implicit);

    assert_eq!(units[0]["id"], "goat");
    assert_eq!(
        units[0]["traits"],
        json!([{"id": "climber", "level": 2, "implicit": false}, {"id": "cunning", "level": 1, "implicit": false},
            {"id": "swimmer", "level": 0, "implicit": true}])
    );
    assert_eq!(units[1]["path"], "content/units/grem.json");
    assert!(units[1]["traits"]
        .as_array()
        .unwrap()
        .contains(&json!({"id": "climber", "level": 0, "implicit": true})));
}
