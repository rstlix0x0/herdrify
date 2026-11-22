//! Parses a GitHub Actions workflow and prints its triggers, jobs, runners and step actions.
use yaml_rust2::{Yaml, YamlLoader};

fn main() {
    let path = std::env::args().nth(1).expect("workflow path");
    let text = std::fs::read_to_string(&path).expect("read workflow");
    let docs = YamlLoader::load_from_str(&text).expect("valid YAML");
    let w = &docs[0];
    println!("documents: {}", docs.len());
    println!("name: {:?}", w["name"].as_str());
    // `on` is a plain string key in YAML 1.2 (yaml-rust2), not the boolean `true`.
    let on = &w["on"];
    let triggers: Vec<String> = on.as_hash().map(|h| h.keys().filter_map(|k| k.as_str().map(str::to_owned)).collect()).unwrap_or_default();
    println!("on: {triggers:?}");
    println!("on.push.branches: {:?}", on["push"]["branches"].as_vec().map(|v| v.iter().filter_map(Yaml::as_str).collect::<Vec<_>>()));
    println!("permissions.contents: {:?}", w["permissions"]["contents"].as_str());
    for (k, job) in w["jobs"].as_hash().expect("jobs map") {
        let name = k.as_str().unwrap_or("?");
        let os = job["strategy"]["matrix"]["os"].as_vec().map(|v| v.iter().filter_map(Yaml::as_str).collect::<Vec<_>>());
        println!("job {name}: runs-on={:?} matrix.os={os:?}", job["runs-on"].as_str());
        for s in job["steps"].as_vec().expect("steps") {
            println!("  step uses={:?} run={:?}", s["uses"].as_str(), s["run"].as_str().map(|r| r.lines().next().unwrap_or("")));
        }
    }
}
