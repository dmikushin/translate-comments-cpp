use tokio;

use crate::gpt;

#[tokio::test]
async fn test_translation() {
    let translator = gpt::Translator::new().expect("Failed to create translator");
    let result = translator.run("es", "Hello, how are you?").await;
    assert!(result.is_ok());
    let translated_text = result.unwrap();
    assert!(!translated_text.is_empty());
    println!("Translated text: {}", translated_text);
}
