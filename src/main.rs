use dotenvy::dotenv;
use reqwest::Url;
use serde_json::json;
use std::{env, error::Error};
use teloxide::{
    dptree,
    macros::BotCommands,
    prelude::*,
    types::{InputFile, MediaKind, MessageKind},
    Bot,
};
use user::{init_user, set_user, User};

mod user;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "OpenAI commands")]
enum BotCommands {
    #[command(description = "Ask ChatHPT a question")]
    Ask,

    #[command(description = "Generate an image")]
    Imagine,

    #[command(description = "Pretend the bot to be something else")]
    Pretend,
}

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

async fn bot_handler(
    message: Message,
    bot: Bot,
    cmd: BotCommands,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let user_id = message.from().unwrap().id;
    let mut user = init_user(
        &user_id.0,
        message.from().unwrap().username.clone().unwrap(),
    )
    .await
    .unwrap();

    if !user.has_requests_left() {
        bot.send_message(
            user_id,
            "You have no requests left, you can purhcase more with /buy",
        )
        .send()
        .await?;
        return Ok(());
    }

    bot.send_message(message.chat.id, "Hmmm.... let me think...")
        .send()
        .await?;

    match cmd {
        BotCommands::Ask => {
            user.update_requests();
            user.previous_messages
                .push(message.text().unwrap().replace("/ask ", ""));
            let response = send_text_to_chatgpt(message.text().unwrap(), &user).await;
            bot.send_message(message.chat.id, clean_string(response))
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .send()
                .await?;
        }
        BotCommands::Imagine => {
            user.update_requests();
            let response = send_image_prompt_to_openai(message.text().unwrap()).await;
            if response.is_err() {
                bot.send_message(message.chat.id, response.err().unwrap())
                    .send()
                    .await?;
            } else {
                bot.send_photo(
                    message.chat.id,
                    InputFile::url(Url::parse(&response.unwrap()).unwrap()),
                )
                .await?;
            }
        }
        BotCommands::Pretend => {
            user.pretend = Some(message.text().unwrap().replace("/pretend ", "").to_string());
            bot.send_message(
                message.chat.id,
                format!(
                    "From now on, I'll pretend \"{}\"",
                    message.text().unwrap().replace("/pretend ", "")
                ),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .send()
            .await?;
        }
    }

    set_user(user.clone()).await.unwrap();
    println!("User: {:?}", user);
    Ok(())
}

async fn message_handler(bot: Bot, message: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !message.chat.is_private() {
        Ok(())
    } else {
        let user_id = message.from().unwrap().id;
        let mut user = init_user(
            &user_id.0,
            message.from().unwrap().username.clone().unwrap(),
        )
        .await
        .unwrap();

        if !user.has_requests_left() {
            bot.send_message(
                user_id,
                "You have no requests left, you can purhcase more with /buy",
            )
            .send()
            .await?;
            return Ok(());
        }

        user.requests_left -= 1;
        set_user(user.clone()).await.unwrap();

        bot.send_message(message.chat.id, "Hmmm.... let me think...")
            .send()
            .await?;

        match message.kind.clone() {
            MessageKind::Common(message_data) => match message_data.media_kind {
                MediaKind::Text(text_data) => {
                    user.previous_messages
                        .push(message.text().unwrap().replace("/ask ", ""));

                    let response = send_text_to_chatgpt(text_data.text.as_str(), &user).await;
                    bot.send_message(message.chat.id, clean_string(response))
                        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                        .send()
                        .await?;
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

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<BotCommands>()
                .endpoint(bot_handler),
        )
        .branch(dptree::entry().endpoint(message_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn send_image_prompt_to_openai(message: &str) -> Result<String, String> {
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

async fn send_text_to_chatgpt(message: &str, user: &User) -> String {
    println!("Sending {message} to ChatGPT");

    let chatgpt_api_url = "https://api.openai.com/v1/chat/completions";

    let role = user.pretend.clone().unwrap_or("You are ChatGPT, a large language model trained by OpenAI. Answer as concisely as possible but clarify what data you base your answers on.".to_string());

    let request_body = json!({
          "model": "gpt-3.5-turbo",
          "messages": [{
              "role": "system",
              "content": role
          },{
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
