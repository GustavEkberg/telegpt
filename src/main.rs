use dotenvy::dotenv;
use reqwest::Url;
use serde_json::json;
use std::{env, error::Error};
use teloxide::{
    dptree,
    prelude::*,
    types::{InputFile, MediaKind, MessageEntityKind, MessageKind},
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
        // .replace('`', r"\`")
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
        bot.send_message(message.chat.id, "Hmmm.... let me think...")
            .send()
            .await?;

        match message.kind.clone() {
            MessageKind::Common(message_data) => match message_data.media_kind {
                MediaKind::Text(text_data) => {
                    if let Some(entity) = text_data.entities.first() {
                        if entity.kind == MessageEntityKind::BotCommand
                            && text_data.text.starts_with("/imagine")
                        {
                            let response = send_image_prompt_to_openai(
                                text_data.text.to_string().replace("/imagine ", "").as_str(),
                            )
                            .await;
                            bot.send_photo(
                                message.chat.id,
                                InputFile::url(Url::parse(&response).unwrap()),
                            )
                            .await?;
                        }
                    } else {
                        let response = send_text_to_chatgpt(text_data.text.as_str()).await;
                        bot.send_message(message.chat.id, clean_string(response))
                            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                            .send()
                            .await?;
                    }
                }
                _ => (),
            },
            _ => (),
        };

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

async fn send_image_prompt_to_openai(message: &str) -> String {
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
    json["data"][0]["url"]
        .as_str()
        .expect("No response from OpenAI")
        .to_string()
}

async fn send_text_to_chatgpt(message: &str) -> String {
    println!("Sending {message} to ChatGPT");

    let chatgpt_api_url = "https://api.openai.com/v1/chat/completions";

    let request_body = json!({
          "model": "gpt-3.5-turbo",
          "messages": [{
              "role": "user",
              "content": message
      }]
    });

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
