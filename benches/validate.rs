use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::Arc;
use zed_yaml_multi_schema::resolver::SchemaFetcher;

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
        // Match by trailing filename rather than the full URL, so this
        // doesn't care whether jsonschema resolves relative $refs against
        // raw.githubusercontent.com/.../schemas/x.json or some other base.
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
            zed_yaml_multi_schema::validator::validate(
                &schema,
                &value,
                Arc::new(BenchFetcher {
                    files: files.clone(),
                }),
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_validate);
criterion_main!(benches);
