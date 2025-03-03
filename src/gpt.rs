#[cfg(test)]
#[path = "gpt_test.rs"]
mod tests;

use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::error::Error;
use async_std::sync::Arc;
use async_std::channel;
use futures::future;

#[derive(Clone, Debug)]
pub struct QueryRequest {
    pub language: String,
    pub text: String,
    pub text_checksum: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub text: String,
    pub text_translation: String,
    pub text_checksum: u64,
}

#[derive(Default, Clone)]
pub struct Translator {
    client: Client,
    api_key: String,
    model: String,
}

impl Translator {
    pub fn new() -> Result<Arc<Self>> {
        let key_file_path = dirs::home_dir().unwrap().join(".openrouter/key");
        let api_key = fs::read_to_string(key_file_path)?.trim().to_string();

        return Ok(Arc::new(Self {
            client: Client::new(),
            api_key : api_key,
            model: "gpt-3.5-turbo".to_string(),
        }));
    }

    async fn run(&self, language: &str, text: &str) -> Result<String, Box<dyn Error>> {
        let system_prompt = format!(
            "You are a Language Translator. Detect the source language and translate it to \"{}\". Always just return the translation of the prompt. If there is nothing to translate, just return the original prompt.",
            language
        );

        let response = self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": text}
                ]
            }))
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        let translated_text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(translated_text)
    }

    /// Submits a QueryRequest to the GPT service and returns translation.
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResult, Box<dyn Error>> {
        let text_translation = self.run(&request.language, &request.text).await?;
        let query_result = QueryResult {
            text: request.text,
            text_translation: text_translation,
            text_checksum: request.text_checksum
        };

        Ok(query_result)
    }

    /// Submits many QueryRequest to the GPT service in parallel, and
    /// returns an array of translations.
    pub async fn query_many(
        self: Arc<Self>,
        requests: Vec<QueryRequest>,
        mut concurrency: usize,
    ) -> Result<Vec<QueryResult>> {
        let (nodes_send_master, nodes_receive_master) = channel::unbounded::<QueryRequest>();
        let (matches_send_master, matches_receive_master) = channel::unbounded::<QueryResult>();

        // Creates a queue to process multiple code comment leaves concurrently.
        let mut threads = Vec::new();
        concurrency = *([concurrency, requests.len()].iter().min().unwrap());

        for _ in 0..concurrency {
            let matches_send = matches_send_master.clone();
            let nodes_receive = nodes_receive_master.clone();
            let this = Arc::clone(&self);
            let thread = tokio::spawn(async move {
                while let Ok(query_request) = nodes_receive.recv().await {
                    let res = this.query(query_request).await.unwrap();
                    matches_send.send(res).await.unwrap();
                }
            });
            threads.push(thread);
        }

        for query_request in requests.iter() {
            // TODO this should be able to pass a pointer.
            nodes_send_master.send(query_request.to_owned()).await?;
        }

        // Waits for queue to empty.
        nodes_send_master.close();
        future::join_all(threads).await;
        matches_send_master.close();

        // Formats and process' results, extracting results for the deduping hashmap,
        // and mapping them back to code comment blocks.
        let mut query_results: Vec<QueryResult> = vec![];
        while let Ok(m) = matches_receive_master.recv().await {
            query_results.push(m);
        }

        return Ok(query_results);
    }
}
