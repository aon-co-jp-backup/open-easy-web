//! `aruaru-llm`(`GET/POST /v1/classify`)への薄いHTTPクライアント。
//! システムメモリ/ディスク使用状況円グラフの「その他」カテゴリ分類に
//! 使う(2026-09-07新設)。到達不能・タイムアウト時は`Err`を返すのみで
//! パニックしない——呼び出し元は分類失敗時「分類不能」として扱う。

use std::time::Duration;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:4600";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

fn base_url() -> String {
    std::env::var("ARUARU_LLM_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

#[derive(serde::Serialize)]
struct ClassifyRequest<'a> {
    items: &'a [String],
    categories: &'a [String],
}

#[derive(serde::Deserialize)]
struct ClassifiedItemDto {
    item: String,
    category: String,
}

#[derive(serde::Deserialize)]
struct ClassifyResponse {
    results: Vec<ClassifiedItemDto>,
}

/// `items`の各要素を`categories`のいずれかへ分類する(aruaru-llmの
/// embeddingコサイン類似度分類、`generic_classify.rs`参照)。
/// 到達不能・非200・パース失敗はすべて`Err`として正直に返す。
pub async fn classify_many(items: &[String], categories: &[String]) -> Result<Vec<(String, String)>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("client build failed: {e}"))?;
    let url = format!("{}/v1/classify", base_url());
    let resp = client
        .post(&url)
        .json(&ClassifyRequest { items, categories })
        .send()
        .await
        .map_err(|e| format!("request to aruaru-llm failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("aruaru-llm returned HTTP {}", resp.status()));
    }
    let body: ClassifyResponse = resp.json().await.map_err(|e| format!("invalid response JSON: {e}"))?;
    Ok(body.results.into_iter().map(|r| (r.item, r.category)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_many_short_circuits_on_empty_items() {
        // 空配列はネットワーク呼び出し自体を行わず即座にOk([])を返すこと
        // を、非同期ランタイムを起動して確認する(ネットワーク到達性に
        // 依存しないテスト)。
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(classify_many(&[], &["a".to_string()]));
        assert_eq!(result.unwrap(), Vec::<(String, String)>::new());
    }
}
