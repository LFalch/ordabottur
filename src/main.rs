#![warn(clippy::all)]

use std::{
    env,
    collections::{HashSet},
};

use serenity::prelude::*;
use serenity::framework::standard::{
    Args,
    CommandResult,
    StandardFramework,
    help_commands,
    HelpOptions,
    CommandGroup,
    macros::{
        command,
        group,
        help,
    }
};
use serenity::model::{
    channel::*,
    gateway::*,
    id::*,
};

use reqwest::{
    header::CONTENT_TYPE,
    blocking::{Client as ReqClient, RequestBuilder}
};
use scraper::{Html, Selector};
use encoding_rs::mem::convert_utf8_to_latin1_lossy;

const FALCH: UserId = UserId(165_877_785_544_491_008);

const PREFIX: &str = "]";

#[command]
#[description = "Set the status of the bot to be playing the set game"]
#[usage = "<game>"]
#[min_args(1)]
fn setgame(ctx: &mut Context, _msg: &Message, args: Args) -> CommandResult {
    ctx.set_activity(Activity::playing(args.message()));
    Ok(())
}

#[command]
#[description = "Søk i grunnmanuskriptet"]
fn gm(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    match gm_entries(args.message(), 10) {
        Ok((results_msg, entries)) => {
            let mut msg_bunch = MsgBunch::new();
            
            msg_bunch
                .add_string(&results_msg)
                .add_string("\n")
                .entries(entries);

            for msg_body in msg_bunch.messages {
                msg.channel_id.say(&ctx, msg_body)?;
            }
        }
        Err(e) => {
            msg.channel_id.say(&ctx, &format!("Eg fekk tíverri {}", e))?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Entry {
    word: String,
    class: String,
    body: String,
}

impl Entry {
    #[inline(always)]
    fn new_gm(word: String, class: String, mut body: String) -> Self {
        body = body.replace("&amp;", "&");
        body = body.replace("`", "\\`");

        // TODO HACK
        body = body.replace("&emacr;", "ē");
        body = body.replace("&oelig;", "ø");
        body = body.replace("&Oogon;", "Ǫ");
        body = body.replace("&nbsp;", " ");
        body = body.replace("&oogon;", "ǫ");
        body = body.replace("&omacr;", "ō");
        body = body.replace("&ocedil;", "");
        body = body.replace("&kryss;", "♯");
        body = body.replace("&divide;", "÷");
        body = body.replace("&dagger;", "†");

        Entry {
            word, class, body
        }
    }
}

use std::fmt::{self, Display};

impl Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Entry{word, class, body} = self;

        write!(f, "_{}_ ({})\n{}", word, class, body)
    }
}

const MSG_LIMIT: usize = 2000;

struct MsgBunch {
    messages: Vec<String>,
}

impl MsgBunch {
    fn new() -> Self {
        MsgBunch {
            messages: vec![String::with_capacity(MSG_LIMIT)]
        }
    }
    fn add_string(&mut self, mut s: &str) -> &mut Self {
        let mut len = self.messages.last().unwrap().len();

        if len + s.len() > MSG_LIMIT {
            while len + s.len() > MSG_LIMIT {
                let split_index = MSG_LIMIT - len;

                {
                    let last_message = self.messages.last_mut().unwrap();
                    last_message.push_str(&s[..split_index]);
                    debug_assert_eq!(last_message.len(), MSG_LIMIT);
                }
                self.messages.push(String::with_capacity(MSG_LIMIT));

                len = 0;
                s = &s[split_index..];
            }
        }

        let last_message = self.messages.last_mut().unwrap();
        last_message.push_str(s);

        self
    }
    fn entries(&mut self, entries: Vec<Entry>) -> &mut Self {
        self.messages.push(String::with_capacity(MSG_LIMIT));

        for entry in entries {
            let entry_text = format!("{}\n", entry);

            self.add_string(&dbg!(entry_text));
        }

        self
    }
}

fn gm_entries(ord: &str, result_row_amount: u16) -> Result<(String, Vec<Entry>), u16> {
    let client = ReqClient::new();

    let res = gm_search_body(client.post("http://www.edd.uio.no/perl/search/search.cgi"),
        ord, result_row_amount)
        .send()
        .unwrap();

    if res.status().is_success() {
        let html = Html::parse_document(&res.text().unwrap());

        let entry_selector = Selector::parse(".ResRowGray td, .ResRowWhite td").unwrap();
        let result_number_selector = Selector::parse(".BeneathNavigator").unwrap();

        let mut iter = html.select(&entry_selector).map(|tr| tr.inner_html().trim().to_owned());

        let mut entries = Vec::with_capacity(iter.size_hint().0);

        while let Some(word) = iter.next() {
            let class = iter.next().unwrap();
            let body = iter.next().unwrap();

            entries.push(Entry::new_gm(word, class, body));
        }

        // HACK don't look at this

        let results = html.select(&result_number_selector).next().unwrap().text().next().unwrap().to_owned();

        Ok((results, entries))
    } else {
        Err(res.status().as_u16())
    }
}

fn gm_search_body(rb: RequestBuilder, word: &str, result_row_amount: u16) -> RequestBuilder {
    let mut buf_bytes =  vec![0; word.len()];

    convert_utf8_to_latin1_lossy(word.as_bytes(), &mut buf_bytes);

    let mut encoded = String::with_capacity(buf_bytes.len());

    for byte in buf_bytes {
        match byte {
            0x00 => (),
            0x01..=0x1f | 0x21..=0x2c | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0xff => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
            0x20 => encoded.push('+'),
            0x2d..=0x39 | 0x41..=0x5a | 0x61..=0x7a => encoded.push(byte as char),
        }
    }

    rb
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!("tabid=993&appid=59&C%23993.994.545%23994.995.546%23ORD={}&dosearch=++++S%F8k++++&oppsetttid=215&ResultatID=447&ResRowsNum={}",
            encoded, result_row_amount))
}

#[command]
#[description = "Say"]
fn say(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    if msg.author.id == FALCH || msg.author.id == 234_039_000_036_409_344 {
        msg.channel_id.say(&ctx, args.message())?;
        msg.delete(&ctx)?;
    }
    Ok(())
}

#[group]
#[commands(gm)]
#[only_in("guilds")]
#[help_available]
struct General;

#[group]
#[commands(setgame)]
#[only_in("guilds")]
#[required_permissions(ADMINISTRATOR)]
struct ModOnly;

#[group]
#[commands(say)]
#[only_in("guilds")]
struct Owner;

#[help]
#[lacking_permissions = "Hide"]
#[max_levenshtein_distance(4)]
fn help(
   context: &mut Context,
   msg: &Message,
   args: Args,
   help_options: &'static HelpOptions,
   groups: &[&'static CommandGroup],
   owners: HashSet<UserId>
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

fn main() {
    let token = env::var("ORDABOT_TOKEN")
        .expect("Expected a token in the environment");
    let mut client = Client::new(&token, Handler).unwrap();

    client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix(PREFIX).allow_dm(true))
        .group(&GENERAL_GROUP)
        .group(&OWNER_GROUP)
        .group(&MODONLY_GROUP)
        .help(&HELP)
    );

    {
        // let mut data = client.data.write();
    }

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        println!("Guilds:");
        for name in ready.guilds.iter().map(|g| g.id().to_partial_guild(&ctx).unwrap().name) {
            println!("    {}", name);
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot {
            return
        }
    }
}
