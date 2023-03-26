use dotenvy::dotenv;
use reqwest::Url;
use std::error::Error;
use teloxide::{
    dptree,
    macros::BotCommands,
    prelude::*,
    types::{InputFile, MediaKind, MessageKind},
    Bot,
};
use user::{init_user, set_user};

use crate::openai::{send_image_prompt_to_openai, send_text_to_chatgpt};

mod openai;
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

    #[command(description = "Display your status")]
    Status,
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
        bot.send_message(user_id, "You have no requests left")
            .send()
            .await?;
        return Ok(());
    }
    match cmd {
        BotCommands::Ask => {
            bot.send_message(message.chat.id, "Hmmm.... let me think...")
                .send()
                .await?;

            user.update_requests();
            let response = send_text_to_chatgpt(message.text().unwrap(), &user).await;
            bot.send_message(message.chat.id, clean_string(response.unwrap()))
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .send()
                .await?;
            user.update_last_message(message.text().unwrap().to_string());
        }
        BotCommands::Imagine => {
            bot.send_message(message.chat.id, "Hmmm.... let me think...")
                .send()
                .await?;

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
        BotCommands::Status => {
            bot.send_message(
                message.chat.id,
                format!("You have {} requests left.", user.requests_left),
            )
            .send()
            .await?;
        }
    }

    set_user(user.clone()).await.unwrap();
    println!("User: {:?}", user);
    Ok(())
}

async fn private_message_handler(
    bot: Bot,
    message: Message,
) -> Result<(), Box<dyn Error + Send + Sync>> {
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

        user.update_requests();
        set_user(user.clone()).await.unwrap();

        bot.send_message(message.chat.id, "Hmmm.... let me think...")
            .send()
            .await?;

        match message.kind.clone() {
            MessageKind::Common(message_data) => match message_data.media_kind {
                MediaKind::Text(text_data) => {
                    let response = send_text_to_chatgpt(text_data.text.as_str(), &user).await;
                    bot.send_message(message.chat.id, clean_string(response.unwrap()))
                        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                        .send()
                        .await?;

                    user.update_last_message(message.text().unwrap().to_string());
                }
                _ => (),
            },
            _ => (),
        };

        set_user(user.clone()).await.unwrap();

        println!("User: {:#?}", user);
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
        .branch(dptree::entry().endpoint(private_message_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    setup_bot().await;
}
