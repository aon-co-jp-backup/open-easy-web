//! 検出したCPUの命令セットと、各処理に選ばれた実装(open-cpu)。管理画面・`System/GetCpu`用。

pub fn snapshot() -> serde_json::Value {
    let inv = open_cpu::inventory();
    serde_json::json!({
        "arch": inv.arch,
        "source": inv.source,
        "detected": inv.detected_names(),
        "cores": inv.cores.iter().map(|g| serde_json::json!({"name": g.name, "count": g.count})).collect::<Vec<_>>(),
        "byte_scan_impl": open_cpu::selected_byte_impl().to_string(),
        "float_impl": open_cpu::selected_float_impl().to_string(),
        "summary": open_cpu::runtime_report(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn snapshot_has_arch_and_impls() {
        let v = super::snapshot();
        assert!(v["arch"].as_str().is_some_and(|a| !a.is_empty()));
        assert!(["scalar", "neon", "avx2", "avx512bw"].contains(&v["byte_scan_impl"].as_str().unwrap_or("")));
        assert!(v["summary"].as_str().unwrap_or("").contains("open-cpu"));
    }
}
