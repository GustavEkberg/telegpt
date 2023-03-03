use dotenvy::dotenv;
use serde_json::json;
use std::{env, error::Error};
use teloxide::{
    dptree,
    prelude::*,
    types::{MediaKind, MessageKind},
    Bot,
};

fn clean_string(s: String) -> String {
    s.replace('_', r"\_")
        .replace('*', r"\*")
        .replace('[', r"\[")
        .replace(']', r"\]")
        .replace('(', r"\(")
        .replace(')', r"\)")
        .replace('~', r"\~")
        .replace('`', r"\`")
        .replace('>', r"\>")
        .replace('#', r"\#")
        .replace('+', r"\+")
        .replace('-', r"\-")
        .replace('=', r"\=")
        .replace('|', r"\|")
        .replace('{', r"\{")
        .replace('}', r"\}")
        .replace('.', r"\.")
        .replace('!', r"\!")
        .replace(r"\`\`\`", r"```")
}

async fn message_handler(bot: Bot, message: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !message.chat.is_private() {
        Ok(())
    } else {
        let response = match message.kind.clone() {
            MessageKind::Common(message_data) => match message_data.media_kind {
                MediaKind::Text(text_data) => Some(send_to_chatgpt(text_data.text.as_str()).await),
                _ => None,
            },
            _ => None,
        };

        println!("{}", clean_string(response.clone().unwrap()));
        bot.send_message(message.chat.id, clean_string(response.unwrap()))
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .send()
            .await?;

        Ok(())
    }
}

pub async fn setup_bot() {
    println!("Starting bot...");

    let bot =
        Bot::new(std::env::var("TELEGRAM_TOKEN").expect("Missing env variable TELEGRAM_TOKEN"));

    let handler = dptree::entry().branch(Update::filter_message().endpoint(message_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn send_to_chatgpt(message: &str) -> String {
    println!("Sending {message} to ChatGPT");

    // We also need to specify the URL for the ChatGPT API
    let chatgpt_api_url = "https://api.openai.com/v1/chat/completions";

    // Construct the request body for the ChatGPT API
    let request_body = json!({
          "model": "gpt-3.5-turbo",
          "messages": [{
              "role": "user",
              "content": message
      }]
    });

    // Send the request to the ChatGPT API and retrieve the response
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
    println!("JSON RESPONSE {:#?}", json);
    json["choices"][0]["message"]["content"]
        .as_str()
        .expect("No response from ChatGPT")
        .to_string()
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    setup_bot().await;
}
