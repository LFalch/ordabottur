#![warn(clippy::all)]

use std::{
    env,
    collections::HashSet,
    str::FromStr
};

use serenity::{all::{standard::Configuration, ActivityData, CreateAllowedMentions, CreateEmbed, CreateMessage, EditMessage}, async_trait, model::{prelude::Member, user::User}, prelude::*, utils::ContentSafeOptions};
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

use numbers_to_words::to_faroese_words;

const FALCH: UserId = UserId::new(165_877_785_544_491_008);

const FAROESE_SERVER: GuildId = GuildId::new(432837404987228171);
const GENERAL_ALMENT: ChannelId = ChannelId::new(433011994161971230);
const NYGGIR_LIMIR: ChannelId = ChannelId::new(432971266312503301);

const PREFIX: &str = "]";

pub mod dictionary {
    pub mod uio;
    pub mod sprotin;
}
pub mod util;
pub mod wordgame;

use dictionary::uio::{sa_entries, sa_entry, gm_entries, SetelArkivOptions};
use dictionary::sprotin::search as fo_search;
use util::MsgBunchBuilder;
use wordgame::{WordGameState, GuessError};

#[command]
#[description = "Set the status of the bot to be playing the set game"]
#[usage = "<game>"]
#[min_args(1)]
async fn setgame(ctx: &Context, _msg: &Message, args: Args) -> CommandResult {
    ctx.set_activity(Some(ActivityData::playing(args.message())));
    Ok(())
}

#[command]
#[description = "Søk i grunnmanuskriptet"]
async fn gm(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    match gm_entries(args.message(), 10).await {
        Ok((results_msg, entries)) => {
            let mut mmb = MsgBunchBuilder::new();

            mmb
                .add_string(&results_msg)
                .add_string("\n")
                .entries(entries);

            for msg_body in mmb.build().messages {
                msg.channel_id.say(&ctx, msg_body).await?;
            }
        }
        Err(e) => {
            msg.channel_id.say(&ctx, &format!("Eg fekk tíverri {}", e)).await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Søk i Setelarkivet"]
#[usage = "[-r <registrant>] [-f <forfattar>] [-t <tittel>] [-o <område>] [-s|p <stad>] [oppslagsord]"]
async fn sa(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut options = SetelArkivOptions::default();
    let mut oppslag = "";

    {
        let mut cur_option = None;
    
        for arg in args.raw() {
            if let Some(stripped) = arg.strip_prefix('-') {
                cur_option = Some(stripped);
            } else if let Some(option) = cur_option {
                match option {
                    "r" => options.registrant = arg,
                    "f" => options.author = arg,
                    "t" => options.title = arg,
                    "o" => options.area_code = arg,
                    "s" | "p" => options.place_code = arg,
                    _ => {
                        msg.reply(ctx, "Ukjend søkjeinstilling").await?;

                        return Ok(());
                    }
                }
                cur_option = None;
            } else {
                oppslag = arg;
            }
        }
    }

    match sa_entries(oppslag, 35, options).await {
        Ok((results_msg, entries)) => {
            let mut mmb = MsgBunchBuilder::new();

            mmb
                .add_string(&results_msg)
                .add_string("\n")
                .entries(entries);
            let msg_bunch = mmb.build();

            for msg_body in msg_bunch.messages {
                msg.channel_id.say(&ctx, msg_body).await?;
            }
        }
        Err(e) => {
            msg.channel_id.say(&ctx, &format!("Eg fekk tíverri {}", e)).await?;
        }
    }

    Ok(())
}
#[command]
#[description = "Sjå eit oppslag frå Setelarkivet"]
async fn sai(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = args.single()?;

    match sa_entry(id).await {
        Ok((oppslag, img_src)) => {
            let mut cm = CreateMessage::new().content(oppslag);
            if let Some(img_src) = img_src {
                cm = cm.embed(CreateEmbed::new().image(img_src));
            }
            msg.channel_id.send_message(&ctx, cm).await?;
        }
        Err(e) => {
            msg.channel_id.say(&ctx, &format!("Eg fekk tíverri {}", e)).await?;
        }
    }

    Ok(())
}

#[derive(Debug, Copy, Clone)]
struct DictionaryId(u8);

impl FromStr for DictionaryId {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        if let Ok(id) = s.parse() {
            return Ok(DictionaryId(id));
        }

        fn asciify(c: char) -> char {
            match c {
                'ø' => 'o',
                'å' => 'a',
                'æ' => 'e',
                'í' => 'i',
                'ý' => 'y',
                'á' => 'a',
                'é' => 'e',
                'ó' => 'o',
                'ú' => 'u',
                c => c,
            }
        }

        Ok(DictionaryId(match &*s.chars().filter_map(|c| if c.is_alphanumeric() { Some(c.to_lowercase()) } else { None }).flatten().map(asciify).collect::<String>() {
            "fofo" | "fof" => 1,
            "fon" | "foe" | "foen" => 2,
            "enf" | "enfo" => 3,
            "fod" | "foda" => 4,
            "daf" | "dafo" => 5,
            "daf2" | "dafo2" => 21,
            "fot" | "foty" => 6,
            "tyf" | "tyfo" => 7,
            "fos" | "fosp" => 10,
            "spf" | "spfo" => 20,
            "grf" | "grfo" => 30,
            "frf" | "frfo" => 9,
            "foi" | "foit" => 11,
            "ruf" | "rufo" => 12,
            "fok" | "foki" => 24,
            "kif" | "kifo" => 26,
            
            "sam" => 15,
            "navn" => 25,
            "alfr" => 22,
            "tilt" => 23,
            "yrk" => 13,
            "busk" => 32,
            _ => return Err(()),
        }))
    }
}

#[command]
#[description = "Look up in a Sprotin dictionary. Usage: ]sprotin <dictionary> <word> [word number]"]
#[aliases("fo")]
#[min_args(1)]
async fn sprotin(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let dict = args.single::<DictionaryId>().unwrap_or(DictionaryId(1));

    match fo_search(dict.0, 1, &args.single_quoted::<String>()?, false, false).await {
        Ok(result) => {
            let msg_bunch;

            if let Ok(id) = args.single() {
                msg_bunch = result.word(id).unwrap_or_else(|| result.summary())
            } else {
                msg_bunch = result.summary()
            }

            for msg_body in msg_bunch.messages {
                msg.channel_id.say(&ctx, msg_body).await?;
            }
        }
        Err(e) => {
            msg.channel_id.say(&ctx, &format!("Eg fekk tíverri {}", e)).await?;
        }
    }
    Ok(())
}

macro_rules! short_commands {
    ($(
        $name:ident, ( $($alias:ident),+ ), $description:expr;
    )*) => {
        $(
        #[command]
        #[description = $description]
        #[aliases($($alias),*)]
        async fn $name(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
            sprotin(ctx, msg, serenity::framework::standard::Args::new(&format!(concat!(stringify!($name), " {}"), args.message()), &[' '.into()])).await
        }
        )*
    };
}

short_commands! {
    fof, (føf, fofo, føfø), "Look up a word in the Look up a word in the Faroese-Faroese dictionary";
    foe, (fon, føn, føe, foen, føen), "Look up a word in the Faroese-English dictionary";
    enf, (enfo, enfø), "Look up a word in the English-Faroese dictionary";
    fod, (fød, foda, føda), "Look up a word in the Faroese-Danish dictionary";
    daf, (dafo, dafø), "Look up a word in the Danish-Faroese dictionary";
    daf2, (dafo2, dafø2), "Look up a word in the second Danish-Faroese dictionary";
    fot, (føt, foty, føty), "Look up a word in the Faroese-German dictionary";
    tyf, (tyfo, tyfø), "Look up a word in the German-Faroese dictionary";
    fos, (fosp, føsp), "Look up a word in the Faroese-Spanish dictionary";
    spf, (spfo, spfø), "Look up a word in the Spanish-Faroese dictionary";
    grf, (grfo, grfø), "Look up a word in the Greek-Faroese dictionary";
    frf, (frfo, frfø), "Look up a word in the French-Faroese dictionary";
    foi, (foit, føit), "Look up a word in the Faroese-Italian dictionary";
    ruf, (rufo, rufø), "Look up a word in the Russian-Faroese dictionary";
    fok, (foki, føki), "Look up a word in the Faroese-Chinese dictionary";
    kif, (kifo, kifø), "Look up a word in the Chinese-Faroese dictionary";
    
    sam, (fsam), "Leita eftir einum orði í Samheitaorðabókini";
    navn, (fnavn), "Leita eftir einum orði í Góðkendum fólkanøvnum";
    alfr, (falfr), "Leita eftir einum orði í Alfrøðibókini";
    tilt, (ftilt), "Leita eftir einum orði í Føroyskari tiltaksorðabók";
    yrk, (fyrk), "Leita eftir einum orði í Føroysk-yrkorðabók";
    busk, (fbusk, búsk), "Leita eftir einum orði í Føroysk handils- og búskaparorðum";
}

#[command]
#[description = "Say"]
async fn say(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    msg.channel_id.say(&ctx, args.message()).await?;
    msg.delete(&ctx).await?;
    Ok(())
}

#[command]
#[description = "Pronounce a number in Faroese"]
#[aliases(tal, úttal)]
async fn num(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let n = args.message().replace(<char>::is_whitespace, "");

    if let Some(words) = to_faroese_words(&n) {
        msg.channel_id.say(ctx, words).await?;
    } else {
        msg.channel_id.say(ctx, "Malformed number. Ógilt tal.").await?;
    }

    Ok(())
}

#[command]
#[description = "Start a word game!"]
#[aliases(wordgame, orðaspæl)]
#[max_args(0)]
async fn wg(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    if ctx.data.read().await.get::<wordgame::WordGameState>().is_some() {
        // If a game is already happening, don't replace it
        msg.react(ctx, '🔂').await?;
        return Ok(())
    }

    let table = wordgame::gen_table();
    let msg = msg.channel_id.say(&ctx, wordgame::format_table(&table)).await?;
    let wgs = wordgame::WordGameState::new(table, msg);

    ctx.data.write().await.insert::<wordgame::WordGameState>(wgs);

    Ok(())
}

#[command]
#[description = "Stop current word game!"]
#[aliases(deletewordgame, nýttorðaspæl)]
async fn wgdel(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    ctx.data.write().await.remove::<wordgame::WordGameState>();

    msg.react(ctx, '✅').await?;

    Ok(())
}

#[group]
#[commands(gm, sa, sai, sprotin, fof, foe, enf, fod, daf, daf2, fot, tyf, fos, spf, grf, frf, foi, ruf, fok, kif, sam, navn, alfr, tilt, yrk, busk, num, wg)]
#[only_in("guilds")]
#[help_available]
struct General;

#[group]
#[only_in("guilds")]
#[required_permissions(ADMINISTRATOR)]
struct ModOnly;

#[group]
#[commands(setgame, say, wgdel)]
#[owners_only]
#[only_in("guilds")]
struct Owner;

#[help]
// #[alias("h", "hjelp", "hjælp", "hjálp")]
#[lacking_permissions = "Hide"]
#[max_levenshtein_distance(4)]
async fn help(
   context: &Context,
   msg: &Message,
   args: Args,
   help_options: &'static HelpOptions,
   groups: &[&'static CommandGroup],
   owners: HashSet<UserId>
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = env::var("ORDABOT_TOKEN")
        .expect("Expected a token in the environment");

    let intents = 
        GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_BANS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_PRESENCES
        | GatewayIntents::GUILD_EMOJIS_AND_STICKERS
        ;

    let framework = StandardFramework::new();
    framework.configure(Configuration::new()
        .dynamic_prefix(|_c, _m| Box::pin(async move{
        match std::fs::read_to_string(".prefix_override") {
            Ok(s) => Some(s),
            Err(_) => Some(PREFIX.to_owned()),
        }
    })).allow_dm(true).owners(vec![FALCH].into_iter().collect()));

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework
            .group(&GENERAL_GROUP)
            .group(&OWNER_GROUP)
            .group(&MODONLY_GROUP)
            .help(&HELP)
        ).await.expect("Could not make client");

    {
        // let mut data = client.data.write();
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        println!("Guilds:");
        let ctx = &ctx;

        for name in ready.guilds.iter().map(|g| Box::pin(async move { g.id.to_partial_guild(ctx).await.unwrap().name })) {
            println!("    {}", name.await);
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return
        }

        if let Some(wgs) = ctx.data.write().await.get_mut::<wordgame::WordGameState>() {
            if msg.channel_id == wgs.message.channel_id {
                if let Some(s) = msg.content.strip_prefix([':', '.', ';']) {
                    for guess in s.split_whitespace() {
                        word_guess(&ctx, guess, &msg, wgs).await.unwrap();
                    }
                    // Ignore
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if new_member.guild_id == FAROESE_SERVER {
            GENERAL_ALMENT.say(&ctx, &format!("Bjóðið **{}** vælkomnum/-ari!", new_member.distinct())).await.unwrap();
            NYGGIR_LIMIR.send_message(&ctx, CreateMessage::new().content(
                format!("<@{}> er júst komin upp í servaran!", new_member.user.id)
            ).allowed_mentions(CreateAllowedMentions::new().empty_users())).await.unwrap();
        }
    }
    async fn guild_member_removal(&self, ctx: Context, guild_id: GuildId, user: User, _member_data_if_available: Option<Member>) {
        if guild_id == FAROESE_SERVER {
            NYGGIR_LIMIR.send_message(&ctx, CreateMessage::new().content(
                format!("<@{}> fór júst úr servaranum.", user.id)
            ).allowed_mentions(CreateAllowedMentions::new().empty_users())).await.unwrap();
        }
    }
}

async fn word_guess(ctx: &Context, word: &str, msg: &Message, wgs: &mut WordGameState) -> CommandResult {
    let word = serenity::utils::content_safe(ctx, word, &ContentSafeOptions::default(), &[]);

    match wgs.guess_word(msg.author.id, word).await {
        Ok(()) => {
            msg.react(&ctx, '✅').await?;
            let mut winners = String::new();
            for (user, points) in &wgs.guessers {
                winners.push_str(&format!("<@{}>: {} ({} bókstavir, {} orð)\n", user, points.points, points.letters, points.words));
            }
            let table = wordgame::format_table(&wgs.table);
            let cntnt = format!("Taken words: {}\n\n{winners}\n{table}\nType `.` or `:` followed by your guess(es)", wgs.taken_words.join(", "));

            if wgs.taken_words.len() % 6 == 0 {
                wgs.message = msg.channel_id.say(&ctx, table).await?;
            }
            wgs.message.edit(&ctx, EditMessage::new().content(cntnt)).await?;
        }
        Err(GuessError::AlreadyGuessed) => {
            msg.react(&ctx, ReactionType::Unicode("♻️".to_owned())).await?;
        }
        Err(GuessError::NotFound(word)) => {
            msg.react(&ctx, '❌').await?;
            msg.channel_id.say(&ctx, format!("_{word}_ not found in a dictionary.")).await?;
        }
        Err(GuessError::WrongLetters) => {
            msg.react(&ctx, '❌').await?;
            msg.channel_id.say(&ctx, "You used letters not in the game.").await?;
        }
        Err(GuessError::TooShort) => {
            msg.react(&ctx, '❌').await?;
            msg.channel_id.say(&ctx, "Your guess was too short.").await?;
        }
    }

    Ok(())
}
