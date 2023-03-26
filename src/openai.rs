use serde::{Deserialize, Serialize};
use std::env;

use crate::user::User;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAIMessage {
    content: String,
    role: String,
}

pub async fn send_image_prompt_to_openai(message: &str) -> Result<String, String> {
    println!("Sending image prompt {message} to opanAI");

    let chatgpt_api_url = "https://api.openai.com/v1/images/generations";

    let request_body =
        json!({ "prompt": message, "n": 1, "size": "1024x1024", "response_format": "url" });

    let client = reqwest::Client::new();
    let response = client
        .post(chatgpt_api_url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                env::var("OPENAI_API_KEY").expect("Missing env variable OPENAI_API_KEY")
            )
            .as_str(),
        )
        .body(request_body.to_string())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    if let Some(error) = json["error"]["message"].as_str() {
        Err(error.to_string())
    } else {
        Ok(json["data"][0]["url"]
            .as_str()
            .expect("No response from OpenAI")
            .to_string())
    }
}

pub async fn send_text_to_chatgpt(message: &str, user: &User) -> Result<String, String> {
    println!("Sending {message} to ChatGPT");

    let chatgpt_api_url = "https://api.openai.com/v1/chat/completions";

    let system_role = user.pretend.clone().unwrap_or("You are ChatGPT, a large language model trained by OpenAI. Answer as concisely as possible but clarify what data you base your answers on.".to_string());

    let mut messages: Vec<OpenAIMessage> = user
        .previous_messages
        .iter()
        .map(|m| OpenAIMessage {
            role: "user".to_string(),
            content: m.to_string(),
        })
        .collect();

    messages.push(OpenAIMessage {
        content: message.to_string(),
        role: "user".to_string(),
    });

    messages.insert(
        0,
        OpenAIMessage {
            content: system_role,
            role: "system".to_string(),
        },
    );

    let request_body = json!({
          "model": "gpt-3.5-turbo",
          "messages": messages
    });

    println!("Request body: {:#?}", request_body);

    let client = reqwest::Client::new();
    let response = client
        .post(chatgpt_api_url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                env::var("OPENAI_API_KEY").expect("Missing env variable OPENAI_API_KEY")
            )
            .as_str(),
        )
        .body(request_body.to_string())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // Extract the generated text from the response
    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    if let Some(error) = json["error"]["message"].as_str() {
        Err(error.to_string())
    } else {
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .expect("No response from ChatGPT")
            .to_string())
    }
}
