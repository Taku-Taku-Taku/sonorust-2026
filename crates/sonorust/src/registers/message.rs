use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
    time::Instant,
};

use either::Either;
use engtokana::EngToKana;
use langrustang::{format_t, lang_t};
use regex::Regex;
use serenity::all::{Context, CreateMessage, EditMessage, Message};
use sonorust_db::GuildData;

use crate::{
    commands,
    crate_extensions::{
        infer_api::InferApiExt, rwlock::RwLockExt, sonorust_setting::SettingJsonExt,
    },
    errors::SonorustError,
    Handler,
};

pub async fn message(handler: &Handler, ctx: &Context, msg: &Message) -> Result<(), SonorustError> {
    if msg.author.bot {
        return Ok(());
    }

    // prefix をキャッシュしておく
    static PREFIX: OnceLock<String> = OnceLock::new();

    let prefix = PREFIX
        .get_or_init(|| handler.setting_json.with_read(|lock| lock.prefix.clone()))
        .as_str();

    // コマンドかそうじゃないかで分岐
    match msg.content.starts_with(prefix) {
        true => command_processing(handler, ctx, msg, prefix).await?,
        false => other_processing(handler, ctx, msg).await?,
    }

    Ok(())
}

/// メッセージの内容がコマンドだった場合の処理
async fn command_processing(
    handler: &Handler,
    ctx: &Context,
    msg: &Message,
    prefix: &str,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();

    // メッセージからプレフィックスを除いたものを取得
    let command_content = &msg.content[prefix.len()..];

    let command_args: Vec<&str> = command_content.split_whitespace().collect();

    let Some(command_name) = command_args.get(0) else {
        return Ok(());
    };
    let command_rest: Vec<_> = command_args
        .get(1..)
        .unwrap_or_else(|| &[])
        .iter()
        .collect();

    let debug_log = || {
        log::debug!(
            "MessageCommand used: /{command_content} {{ Name: {}, ID: {} }}",
            msg.author.name,
            msg.author.id,
        )
    };

    match *command_name {
        "ping" => {
            debug_log();

            let embed = commands::ping::measuring_embed(lang);
            let message = CreateMessage::new().embed(embed);

            let now = Instant::now();
            let mut send_msg = msg.channel_id.send_message(&ctx.http, message).await?;
            let elapsed = now.elapsed();

            // description を計測した時間に書き換え
            let embed = commands::ping::measured_embed(elapsed);
            let edit_msg = EditMessage::new().embed(embed);
            send_msg.edit(&ctx.http, edit_msg).await?;
        }

        "help" => {
            debug_log();

            let embed = commands::help(ctx, lang, prefix).await;
            eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, embed = embed).await?;
        }

        "join" => {
            debug_log();

            let result = commands::join(
                handler,
                ctx,
                lang,
                msg.guild_id,
                msg.channel_id,
                msg.author.id,
            )
            .await;

            match result {
                Ok(s) => {
                    let help_embed = commands::help(ctx, lang, prefix).await;
                    eq_uilibrium::send_msg!(
                        msg.channel_id,
                        &ctx.http,
                        content = s,
                        embed = help_embed
                    )
                    .await?;
                }
                Err(s) => {
                    eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, content = s).await?;
                }
            };

            // すでにボイスチャンネルに参加していた場合などは返す
            if let Err(_) = result {
                return Ok(());
            };

            // 音声再生
            handler
                .infer_client
                .play_on_vc(
                    handler,
                    ctx,
                    msg.guild_id,
                    msg.channel_id,
                    msg.author.id,
                    lang_t!("join.connected", lang),
                )
                .await?;
        }
        "leave" => {
            debug_log();

            let text = commands::leave(handler, ctx, lang, msg.guild_id).await;
            msg.channel_id.say(&ctx.http, text).await?;
        }
        "model" => {
            debug_log();

            let (embed, components) = commands::model(handler, lang).await;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "speaker" => {
            debug_log();

            let (embed, components) = commands::speaker(handler, msg.author.id, lang).await?;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "style" => {
            debug_log();

            let (embed, components) = commands::style(handler, msg.author.id, lang).await?;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "length" => {
            debug_log();

            // 数字部分を取得
            let Some(length) = command_rest.get(0).map(|i| *i) else {
                msg.channel_id
                    .say(&ctx.http, format_t!("length.usage", lang, prefix))
                    .await?;
                return Ok(());
            };

            // 数字に変換できなければ返す
            let Ok(length) = length.parse::<f64>() else {
                msg.channel_id
                    .say(&ctx.http, lang_t!("length.not_num", lang))
                    .await?;
                return Ok(());
            };

            // ユーザーデータを変更してメッセージを送信
            let content = commands::length(msg.author.id, length, lang).await?;
            msg.channel_id.say(&ctx.http, content).await?;
        }
        "wav" => {
            debug_log();

            let mut splitn = msg.content.splitn(2, " ");

            // 生成部分を取得
            let Some(content) = splitn.nth(1) else {
                msg.channel_id
                    .say(&ctx.http, format_t!("wav.usage", lang, prefix))
                    .await?;
                return Ok(());
            };

            match commands::wav(handler, msg.author.id, content).await? {
                Either::Left(attachment) => {
                    eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, add_file = attachment)
                        .await?
                }
                Either::Right(content) => {
                    eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, content = content).await?
                }
            };
        }
        "dict" => {
            debug_log();

            let (embed, components) = commands::dict(ctx, msg.guild_id, lang).await?;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "now" => {
            debug_log();

            let embed = commands::now(handler, &msg.author, lang).await?;
            eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, embed = embed).await?;
        }
        "reload" => {
            debug_log();

            let content = commands::reload(handler, ctx, msg.author.id, lang).await;
            msg.channel_id.say(&ctx.http, content).await?;
        }
        "server" => {
            debug_log();

            let (embed, components) =
                commands::server(ctx, msg.guild_id, msg.author.id, lang).await?;

            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "autojoin" => {
            debug_log();

            let (embed, text) =
                commands::autojoin(ctx, msg.guild_id, msg.author.id, lang, None, None).await?;

            let mut create_message = CreateMessage::new();

            if let Some(embed) = embed {
                create_message = create_message.add_embed(embed);
            }

            if let Some(text) = text {
                create_message = create_message.content(text);
            }

            msg.channel_id
                .send_message(&ctx.http, create_message)
                .await?;
        }
        "read_add" => {
            debug_log();

            let content =
                commands::read_add(handler, ctx, msg.guild_id, msg.channel_id, msg.author.id)?;
            msg.channel_id.say(&ctx.http, content).await?;
        }
        "read_remove" => {
            debug_log();

            let content =
                commands::read_remove(handler, ctx, msg.guild_id, msg.channel_id, msg.author.id)?;
            msg.channel_id.say(&ctx.http, content).await?;
        }

        "clear" => {
            debug_log();

            let content = commands::clear(handler, ctx, msg.guild_id, msg.author.id).await?;
            msg.channel_id.say(&ctx.http, content).await?;
        }

        _ => (),
    }

    Ok(())
}

// コマンド以外だった時の処理 (主にvcで音声再生)
async fn other_processing(
    handler: &Handler,
    ctx: &Context,
    msg: &Message,
) -> Result<(), SonorustError> {
    // セミコロンから始まっていた場合無視
    if msg.content.starts_with(";") {
        return Ok(());
    }

    // ギルド上でない場合無視
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };

    // 読み上げるチャンネルか確認
    let read_target_ch = handler
        .read_channels
        .with_read(|lock| lock.get(&guild_id).cloned());

    let is_read_target = match read_target_ch {
        Some(hashset) => hashset.contains(&msg.channel_id),
        None => return Ok(()),
    };

    if !is_read_target {
        return Ok(());
    }

    let guilddata = GuildData::from(guild_id).await?;

    let lang = handler.setting_json.get_bot_lang();

    // 読み上げ用に文字を置換する
    let mut text_replace = TextReplace::new(&msg.content);

    text_replace.remove_codeblock();
    text_replace.remove_url();
    text_replace.remove_markdown();
    text_replace.remove_discord_obj();

    text_replace.replace_from_guilddict(&guilddata);

    // 日本語の時のみ英語を日本語読みに変換
    {
        use crate::_langrustang_autogen::Lang::*;

        match lang {
            Ja => text_replace.eng_to_kana(),
            _ => (),
        }
    }

    text_replace.remove_emoji();
    text_replace.normalize_whitespace();
    // 空文字になった場合の処理も含むため最後に呼ぶ
    text_replace.remove_err();

    let replaced_text = text_replace.as_string();

    // read_limit よりも長い場合はその長さに制限する
    let read_limit = handler.setting_json.with_read(|lock| lock.read_limit);
    let content = match replaced_text.char_indices().nth(read_limit as _) {
        Some((idx, _)) => format_t!("msg.omitted", lang, &replaced_text[..idx]),
        None => replaced_text,
    };

    // 設定で ON になっていて添付ファイルがあるなら添付ファイルがあることを知らせる
    if !msg.attachments.is_empty() && guilddata.options.is_notice_attachment {
        handler
            .infer_client
            .play_on_vc(
                handler,
                ctx,
                msg.guild_id,
                msg.channel_id,
                msg.author.id,
                lang_t!("msg.attachments", lang),
            )
            .await?;
    }

    handler
        .infer_client
        .play_on_vc(
            handler,
            ctx,
            msg.guild_id,
            msg.channel_id,
            msg.author.id,
            &content,
        )
        .await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct TextReplace {
    text: String,
}

impl TextReplace {
    pub fn new<S>(text: S) -> Self
    where
        S: Into<String>,
    {
        Self { text: text.into() }
    }

    pub fn as_string(self) -> String {
        self.text
    }

    pub fn remove_codeblock(&mut self) {
        // ``` が含まれていた場合全体をコードブロックと読む
        if self.text.contains("```") {
            self.text = "コードブロック".to_string();
            return;
        }

        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`.*?`").expect("Regex Failed"));
        self.text = RE.replace_all(&self.text, "コード").to_string()
    }

    pub fn remove_url(&mut self) {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"https?://[\w/:%#\$&\?\(\)~\.=\+\-]+").expect("Regex Failed")
        });
        self.text = RE.replace_all(&self.text, "URL").to_string()
    }

    /// Discord の装飾記号を、中の文字は残したまま取り除く
    ///
    /// 記号をそのまま渡すと読み上げられてしまうため
    pub fn remove_markdown(&mut self) {
        // 行頭の引用、見出し、小さい文字
        static RE_LINE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?m)^[ \t]*(?:>>>|>|#{1,3}|-#)[ \t]*").expect("Regex Failed")
        });
        // 太字、斜体、下線、打ち消し線、ネタバレ
        static RE_MARK: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\*\*\*|\*\*|__|~~|\|\||\*|_").expect("Regex Failed"));

        self.text = RE_LINE.replace_all(&self.text, "").to_string();
        self.text = RE_MARK.replace_all(&self.text, "").to_string();
    }

    /// チャンネルやメンション、カスタム絵文字などの置換
    pub fn remove_discord_obj(&mut self) {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<.*?>").expect("Regex Failed"));
        self.text = RE.replace_all(&self.text, "").to_string()
    }

    /// 絵文字や、読み上げられない制御文字などを取り除く
    ///
    /// 句読点や空白は読み上げの区切りに必要なため残す
    pub fn remove_emoji(&mut self) {
        // So: 絵文字などの記号, Sk: 修飾記号 (肌の色など), Cf: 書式制御文字 (ZWJ など),
        // Me: 囲み記号 (キーキャップ), Co: 私用領域, FE00-FE0F: 異体字セレクタ
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"[\p{So}\p{Sk}\p{Cf}\p{Me}\p{Co}\u{FE00}-\u{FE0F}]").expect("Regex Failed")
        });
        self.text = RE.replace_all(&self.text, "").to_string()
    }

    /// 連続する空白や改行をひとつの空白にまとめる
    pub fn normalize_whitespace(&mut self) {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("Regex Failed"));
        self.text = RE.replace_all(&self.text, " ").trim().to_string()
    }

    /// 指定したサーバー辞書をもとに置換する
    pub fn replace_from_guilddict(&mut self, guilddata: &GuildData) {
        let map = &guilddata.dict;

        self.replace_from_hashmap(map);
    }

    /// HashMap をもとに HashMap の Key を Value に置換する
    pub fn replace_from_hashmap(&mut self, map: &HashMap<String, String>) {
        let mut replace_texts = HashMap::new();

        for (i, (before, after)) in map.iter().enumerate() {
            let mark = format!("{{|{}|}}", i);

            self.text = self.text.replace(before, &mark).to_string();
            replace_texts.insert(mark, after);
        }

        for (before, after) in replace_texts {
            self.text = self.text.replace(&before, after)
        }
    }

    /// 英語をカタカナ読みに変換する
    pub fn eng_to_kana(&mut self) {
        self.text = EngToKana::convert_all(&self.text);
    }

    // ~ から始まるとなぜかエラーをはいたりするため修正
    // っーーー や ッーーー を っ に修正
    pub fn remove_err(&mut self) {
        self.text = self.text.replace("~", "-").replace("～", "ー");

        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(ッ|ｯ|っ)ー+").expect("Regex Failed"));
        self.text = RE.replace_all(&self.text, "$1").to_string();

        if self.text.starts_with("ー") {
            if let Some(s) = self.text.strip_prefix("ー") {
                self.text = s.to_string();
            };
        }

        if self.text.is_empty() {
            self.text = String::from("-");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextReplace;

    fn apply<F>(text: &str, f: F) -> String
    where
        F: FnOnce(&mut TextReplace),
    {
        let mut text_replace = TextReplace::new(text);
        f(&mut text_replace);
        text_replace.as_string()
    }

    /// 読み上げ前に通る置換をまとめて適用する (辞書と英語変換を除く)
    fn pipeline(text: &str) -> String {
        apply(text, |t| {
            t.remove_codeblock();
            t.remove_url();
            t.remove_markdown();
            t.remove_discord_obj();
            t.remove_emoji();
            t.normalize_whitespace();
            t.remove_err();
        })
    }

    #[test]
    fn test_remove_codeblock() {
        assert_eq!(
            apply("```rust\nfn main() {}\n```", |t| t.remove_codeblock()),
            "コードブロック"
        );
        assert_eq!(
            apply("これは `let a = 1;` です", |t| t.remove_codeblock()),
            "これは コード です"
        );
    }

    #[test]
    fn test_remove_url() {
        assert_eq!(
            apply("見て https://example.com/a?b=1 これ", |t| t
                .remove_url()),
            "見て URL これ"
        );
    }

    #[test]
    fn test_remove_markdown() {
        assert_eq!(apply("**太字**です", |t| t.remove_markdown()), "太字です");
        assert_eq!(apply("~~打ち消し~~", |t| t.remove_markdown()), "打ち消し");
        assert_eq!(apply("||ネタバレ||", |t| t.remove_markdown()), "ネタバレ");
        assert_eq!(apply("> 引用文", |t| t.remove_markdown()), "引用文");
        assert_eq!(apply("# 見出し", |t| t.remove_markdown()), "見出し");
    }

    #[test]
    fn test_remove_discord_obj() {
        assert_eq!(
            apply("<@123> おはよう", |t| t.remove_discord_obj()),
            " おはよう"
        );
    }

    #[test]
    fn test_remove_emoji() {
        assert_eq!(apply("😀 たのしい 🎉", |t| t.remove_emoji()), " たのしい ");
        // ZWJ や肌の色の指定を含む絵文字も取り除く
        assert_eq!(
            apply("👨‍👩‍👧 かぞく 👍🏽", |t| t
                .remove_emoji()),
            " かぞく "
        );
    }

    /// 句読点や空白は読み上げの区切りに必要なため、消してはいけない
    #[test]
    fn test_remove_emoji_keeps_punctuation() {
        let text = "こんにちは、元気ですか？ はい!";
        assert_eq!(apply(text, |t| t.remove_emoji()), text);
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(
            apply("おはよう\nおやすみ", |t| t.normalize_whitespace()),
            "おはよう おやすみ"
        );
        assert_eq!(apply("  a   b  ", |t| t.normalize_whitespace()), "a b");
    }

    #[test]
    fn test_remove_err() {
        assert_eq!(apply("あっーーー", |t| t.remove_err()), "あっ");
        // ～ は ー に変換され、先頭の ー は取り除かれる
        assert_eq!(apply("～ですね", |t| t.remove_err()), "ですね");
        assert_eq!(apply("それ～ですね", |t| t.remove_err()), "それーですね");
        assert_eq!(apply("", |t| t.remove_err()), "-");
    }

    #[test]
    fn test_pipeline_keeps_punctuation_and_spaces() {
        assert_eq!(
            pipeline("こんにちは、元気ですか？"),
            "こんにちは、元気ですか？"
        );
        assert_eq!(
            pipeline("hello world how are you"),
            "hello world how are you"
        );
        assert_eq!(
            pipeline("えっ、まじで!? やばい。"),
            "えっ、まじで!? やばい。"
        );
    }

    #[test]
    fn test_pipeline_removes_noise() {
        assert_eq!(pipeline("**やった** 🎉"), "やった");
        assert_eq!(pipeline("<@123> おはよう"), "おはよう");
        assert_eq!(pipeline("見て https://example.com これ"), "見て URL これ");
        // 読み上げる中身が無くなった場合
        assert_eq!(pipeline("😀"), "-");
    }
}
