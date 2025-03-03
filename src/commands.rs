#[cfg(test)]
#[path = "commands_test.rs"]
mod tests;

use std::collections::HashMap;
use anyhow::Result;
use crate::cache;
use crate::gpt;
use crate::gpt::QueryResult;
use crate::parse;
use whatlang::detect;
use rayon::prelude::*;

fn detect_language(comment: &str) -> String {
    if let Some(info) = detect(comment) {
        return info.lang().code().to_string();
    }
    "unknown".to_string()
}

pub async fn translate(
    filepath: String,
    concurrency: usize,
    language: String,
) -> Result<Vec<QueryResult>> {
    let cached_match_map = cache::get_cached_matches(&filepath).await?;
    let code_comments = parse::parse_code_comments(&filepath).await?;

    fn is_in_target_language(comment: &str, language: &str) -> bool {
        detect_language(comment) == language
    }

    // Filter out comments already in the target language
    let filtered_comments: Vec<_> = code_comments
        .par_iter()
        .filter(|comment| !is_in_target_language(&comment.text, &language))
        .collect();

    // Creates a hashmap with checksums of all the code comments. This is later used
    // to dedupe requests to GPT for codebases that have reoccuring
    // comments in the same file.
    let mut comments_checksum_map: HashMap<u64, String> = HashMap::new();
    for code_comment in filtered_comments.iter() {
        comments_checksum_map.insert(code_comment.text_checksum, code_comment.text.to_owned());
    }

    let mut result_match_map: HashMap<u64, QueryResult> = HashMap::new();
    let query_requests = comments_checksum_map.clone()
        .into_iter()
        .filter(|(text_checksum, _text)| {
            if cached_match_map.contains_key(text_checksum) {
                let res = cached_match_map.get(text_checksum).unwrap();
                result_match_map.insert(text_checksum.to_owned(), res.to_owned());
                return false;
            }

            return true;
        })
        .map(|(text_checksum, text)| {
            return gpt::QueryRequest {
                language: language.to_owned(),
                text,
                text_checksum,
            };
        })
        .collect();

    let gpt_translator_client = gpt::Translator::new()?;
    let query_results = gpt_translator_client.query_many(query_requests, concurrency).await?;
    for query_result in query_results.iter() {
        let original_comment = query_result.text.clone();
        let mut text_translation = query_result.text_translation.clone();

        // Check if the original comment starts with "//" and ensure the translated comment does too
        if original_comment.trim_start().starts_with("//") && !text_translation.trim_start().starts_with("//") {
            text_translation = format!("// {}", text_translation);
        }

        // Insert the possibly modified translated text into the result map
        result_match_map.insert(query_result.text_checksum, QueryResult {
            text_translation: text_translation,
            ..query_result.clone()
        });
    }

    cache::save_cached_matches(&filepath, &result_match_map).await?;

    // Collect the values of result_match_map into a vector and return it
    let result_values: Vec<QueryResult> = result_match_map.values().cloned().collect();
    return Ok(result_values);
}
