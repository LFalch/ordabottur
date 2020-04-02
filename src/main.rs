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

const FALCH: UserId = UserId(165_877_785_544_491_008);
const EILIV: UserId = UserId(234_039_000_036_409_344);

const PREFIX: &str = "]";

mod dictionary;

use dictionary::{gm_entries, MsgBunch};

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

#[command]
#[description = "Say"]
fn say(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    msg.channel_id.say(&ctx, args.message())?;
    msg.delete(&ctx)?;
    Ok(())
}

#[group]
#[commands(gm)]
#[only_in("guilds")]
#[help_available]
struct General;

#[group]
#[only_in("guilds")]
#[required_permissions(ADMINISTRATOR)]
struct ModOnly;

#[group]
#[commands(setgame, say)]
#[owners_only]
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
        .configure(|c| c.dynamic_prefix(|_c, _m| {
            match std::fs::read_to_string(".prefix_override") {
                Ok(s) => Some(s),
                Err(_) => Some(PREFIX.to_owned()),
            }
        }).allow_dm(true).owners(vec![FALCH, EILIV].into_iter().collect()))
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

    fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot {
            return
        }
    }
}
