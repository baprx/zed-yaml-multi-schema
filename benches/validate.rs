use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::Arc;
use zed_yaml_multi_schema::resolver::{SchemaFetcher, SchemaResolver};

fn fixtures() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert(
        "values.schema.json",
        include_str!("fixtures/bjws/values.schema.json"),
    );
    m.insert(
        "configmap.json",
        include_str!("fixtures/bjws/configmap.json"),
    );
    m.insert(
        "containers.json",
        include_str!("fixtures/bjws/containers.json"),
    );
    m.insert(
        "controllers.json",
        include_str!("fixtures/bjws/controllers.json"),
    );
    m.insert(
        "definitions.json",
        include_str!("fixtures/bjws/definitions.json"),
    );
    m.insert("envVars.json", include_str!("fixtures/bjws/envVars.json"));
    m.insert("gw-api.json", include_str!("fixtures/bjws/gw-api.json"));
    m.insert("ingress.json", include_str!("fixtures/bjws/ingress.json"));
    m.insert("k8s-api.json", include_str!("fixtures/bjws/k8s-api.json"));
    m.insert(
        "networkpolicy.json",
        include_str!("fixtures/bjws/networkpolicy.json"),
    );
    m.insert(
        "persistence.json",
        include_str!("fixtures/bjws/persistence.json"),
    );
    m.insert("pod.json", include_str!("fixtures/bjws/pod.json"));
    m.insert(
        "podMonitor.json",
        include_str!("fixtures/bjws/podMonitor.json"),
    );
    m.insert(
        "rawResource.json",
        include_str!("fixtures/bjws/rawResource.json"),
    );
    m.insert("rbac.json", include_str!("fixtures/bjws/rbac.json"));
    m.insert("route.json", include_str!("fixtures/bjws/route.json"));
    m.insert("secret.json", include_str!("fixtures/bjws/secret.json"));
    m.insert("service.json", include_str!("fixtures/bjws/service.json"));
    m.insert(
        "serviceAccount.json",
        include_str!("fixtures/bjws/serviceAccount.json"),
    );
    m.insert(
        "serviceMonitor.json",
        include_str!("fixtures/bjws/serviceMonitor.json"),
    );
    m
}

struct BenchFetcher {
    files: HashMap<&'static str, &'static str>,
}

impl SchemaFetcher for BenchFetcher {
    fn read_local(&self, _path: &str) -> Result<String, String> {
        Err("not used in this benchmark".into())
    }

    fn fetch_remote(&self, url: &str) -> Result<String, String> {
        let name = url.rsplit('/').next().unwrap_or(url);
        self.files
            .get(name)
            .map(|s| s.to_string())
            .ok_or_else(|| format!("unexpected URL in benchmark: {url}"))
    }
}

fn bench_validate(c: &mut Criterion) {
    let files = fixtures();
    let schema: serde_json::Value = serde_json::from_str(files["values.schema.json"]).unwrap();
    let value: yaml_serde::Value =
        yaml_serde::from_str("controllers:\n  main:\n    enabled: true\n").unwrap();

    c.bench_function("validate_bjws_schema", |b| {
        b.iter(|| {
            // Cold every iteration on purpose: one full build+validate cycle.
            let mut resolver = SchemaResolver::new(
                Arc::new(BenchFetcher {
                    files: files.clone(),
                }),
                std::path::Path::new("/root"),
            );
            let validator = resolver.validator_for("bench-ref", &schema).unwrap();
            zed_yaml_multi_schema::validator::validate(&validator, &value).unwrap()
        })
    });
}

fn bench_typing_session(c: &mut Criterion) {
    let files = fixtures();
    let schema: serde_json::Value = serde_json::from_str(files["values.schema.json"]).unwrap();

    // Stand-in for ~20 keystrokes worth of on_change calls against a block
    // whose # $schema= reference doesn't change — only the value does.
    let values: Vec<yaml_serde::Value> = (1..=20)
        .map(|replicas| {
            yaml_serde::from_str(&format!(
                "controllers:\n  main:\n    enabled: true\n    replicas: {replicas}\n"
            ))
            .unwrap()
        })
        .collect();

    c.bench_function("validate_20_keystrokes_same_schema", |b| {
        b.iter(|| {
            // Resolver constructed fresh per iteration: first validator_for
            // call pays the build cost, the other 19 hit the cache — this
            // mirrors a real typing session starting from a file already open.
            let mut resolver = SchemaResolver::new(
                Arc::new(BenchFetcher {
                    files: files.clone(),
                }),
                std::path::Path::new("/root"),
            );
            for value in &values {
                let validator = resolver.validator_for("bench-ref", &schema).unwrap();
                zed_yaml_multi_schema::validator::validate(&validator, value).unwrap();
            }
        })
    });
}

criterion_group!(benches, bench_validate, bench_typing_session);
criterion_main!(benches);
